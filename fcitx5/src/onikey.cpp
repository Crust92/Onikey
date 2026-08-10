/*
 * Onikey - addon Fcitx5, lớp mỏng gọi lõi Rust qua C FFI (onikey.h).
 *
 * Mẫu kiến trúc: fcitx5-cskk (addon C++ + lõi Rust libcskk). Mọi logic tiếng
 * Việt nằm ở lõi; file này chỉ dịch sự kiện Fcitx <-> lõi.
 *
 * Bản đầu chạy chế độ Pre-edit — tin cậy nhất, không đụng surrounding text.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.  GPL-3.0-or-later.
 */

#include <fcitx-utils/keysymgen.h>
#include <fcitx-utils/utf8.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx/text.h>

#include <memory>
#include <string>

#include "onikey.h"

namespace {

// Trạng thái cho TỪNG ô nhập: mỗi InputContext một engine lõi riêng, để gõ dở
// ở cửa sổ này không lây sang cửa sổ kia.
class OnikeyState : public fcitx::InputContextProperty {
public:
    OnikeyState() {
        OnikeyUserConfig cfg{};
        onikey_load_user_config(&cfg);
        engine_ = onikey_engine_new(cfg.input_method, cfg.core_flags);
        autoRestore_ = cfg.ib_flags & ONIKEY_IBFLAG_AUTO_NON_VN_RESTORE;
        ddFreeStyle_ = cfg.ib_flags & ONIKEY_IBFLAG_DD_FREE_STYLE;
        charset_ = cfg.output_charset;
        inputMode_ = cfg.default_input_mode;
    }
    ~OnikeyState() override { onikey_engine_free(engine_); }

    std::string display() const {
        char *s = onikey_engine_display_string(engine_, autoRestore_, ddFreeStyle_);
        if (!s) {
            return {};
        }
        std::string out(s);
        onikey_string_free(s);
        return out;
    }

    std::string encoded(const std::string &s) const {
        char *e = onikey_encode(charset_.c_str(), s.c_str());
        if (!e) {
            return s;
        }
        std::string out(e);
        onikey_string_free(e);
        return out;
    }

    OnikeyEngine *engine_ = nullptr;
    bool autoRestore_ = true;
    bool ddFreeStyle_ = true;
    std::string charset_ = "Unicode";
    /// Chế độ gõ người dùng chọn: 1 = Pre-edit, khác = không gạch chân.
    unsigned int inputMode_ = 1;
    /// Chuỗi đã GHI RA ứng dụng ở chế độ không gạch chân — để biết phải xoá
    /// lùi bao nhiêu khi chữ thay đổi. (Cùng vai trò `committed` bên IBus.)
    std::string committed_;
};

// Có gõ được kiểu không gạch chân không: theo chế độ người dùng chọn VÀ ứng
// dụng phải hỗ trợ surrounding text — thiếu thì giữ Pre-edit, thà có gạch chân
// còn hơn nuốt phím (bài học ô địa chỉ Edge bên IBus).
bool noUnderline(fcitx::InputContext *ic, const OnikeyState *state) {
    return state->inputMode_ != 1 &&
           ic->capabilityFlags().test(fcitx::CapabilityFlag::SurroundingText);
}

class OnikeyFcitxEngine final : public fcitx::InputMethodEngineV2 {
public:
    explicit OnikeyFcitxEngine(fcitx::Instance *instance)
        : instance_(instance),
          factory_([](fcitx::InputContext &) { return new OnikeyState(); }) {
        instance_->inputContextManager().registerProperty("onikeyState", &factory_);
    }

    void keyEvent(const fcitx::InputMethodEntry & /*entry*/,
                  fcitx::KeyEvent &keyEvent) override {
        if (keyEvent.isRelease()) {
            return;
        }
        auto *ic = keyEvent.inputContext();
        auto *state = ic->propertyFor(&factory_);
        const auto key = keyEvent.key();
        const auto sym = key.sym();

        // Phím bổ trợ (Ctrl/Alt/Super): chốt chữ đang gõ, nhả phím cho app.
        if (key.states().testAny(fcitx::KeyStates{
                fcitx::KeyState::Ctrl, fcitx::KeyState::Alt, fcitx::KeyState::Super})) {
            commitPending(ic, state);
            return;
        }

        if (sym == FcitxKey_BackSpace) {
            if (state->display().empty()) {
                state->committed_.clear();
                return; // không có gì đang gõ -> app tự xoá
            }
            onikey_engine_remove_last_char(state->engine_, true);
            if (noUnderline(ic, state)) {
                // chữ đã nằm trong app: để app tự xoá 1 ký tự, ta chỉ theo dõi
                state->committed_ = state->display();
                return;
            }
            updatePreedit(ic, state);
            keyEvent.filterAndAccept();
            return;
        }

        if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter || sym == FcitxKey_Escape) {
            commitPending(ic, state);
            return; // Enter/Esc vẫn tới app
        }

        const uint32_t chr = fcitx::Key::keySymToUnicode(sym);
        if (chr == 0) {
            // phím điều hướng (mũi tên, Home...) -> chốt rồi cho qua
            commitPending(ic, state);
            return;
        }
        if (sym == FcitxKey_space || !onikey_engine_can_process_key(state->engine_, chr)) {
            // GỘP ký tự ngắt từ vào chuỗi commit thay vì để fcitx forward phím:
            // hai đường đi (commit qua IM, phím forward thô) không bảo đảm thứ
            // tự tới ứng dụng — dấu cách sẽ chạy lên TRƯỚC chữ. Cùng họ với
            // bài học "passsowrd" bên engine IBus.
            if (noUnderline(ic, state)) {
                // chữ đã nằm trong app, chỉ cần quên từ hiện tại đi
                onikey_engine_reset(state->engine_);
                state->committed_.clear();
                return; // phím ngắt tới app nguyên vẹn
            }
            std::string pending = state->display();
            if (!pending.empty()) {
                onikey_engine_reset(state->engine_);
                clearPanel(ic);
                std::string out = state->encoded(pending);
                out.append(fcitx::utf8::UCS4ToUTF8(chr));
                ic->commitString(out);
                keyEvent.filterAndAccept();
                return;
            }
            return; // không có gì đang gõ -> phím tới app nguyên vẹn
        }

        onikey_engine_process_key(state->engine_, chr, 1 /* VIETNAMESE */);
        if (noUnderline(ic, state)) {
            rewriteCommitted(ic, state);
            keyEvent.filterAndAccept();
            return;
        }
        updatePreedit(ic, state);
        keyEvent.filterAndAccept();
    }

    void reset(const fcitx::InputMethodEntry & /*entry*/,
               fcitx::InputContextEvent &event) override {
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        // Mất focus giữa chừng: chốt phần đang gõ thay vì nuốt mất.
        commitPending(ic, state);
    }

private:
    void updatePreedit(fcitx::InputContext *ic, OnikeyState *state) {
        const std::string s = state->display();
        auto &panel = ic->inputPanel();
        panel.reset();
        if (!s.empty()) {
            fcitx::Text preedit(s, fcitx::TextFormatFlag::Underline);
            preedit.setCursor(static_cast<int>(s.size()));
            if (ic->capabilityFlags().test(fcitx::CapabilityFlag::Preedit)) {
                panel.setClientPreedit(preedit);
            } else {
                panel.setPreedit(preedit);
            }
        }
        ic->updatePreedit();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }

    void clearPanel(fcitx::InputContext *ic) {
        ic->inputPanel().reset();
        ic->updatePreedit();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }

    void commitPending(fcitx::InputContext *ic, OnikeyState *state) {
        const std::string s = state->display();
        onikey_engine_reset(state->engine_);
        clearPanel(ic);
        if (noUnderline(ic, state)) {
            state->committed_.clear();
            return; // chữ đã nằm trong app từ trước
        }
        if (!s.empty()) {
            ic->commitString(state->encoded(s));
        }
    }

    /// Chế độ không gạch chân: so chuỗi cũ/mới, xoá lùi đúng phần khác rồi ghi
    /// đuôi mới. Số ký tự xoá đếm trên chuỗi ĐÃ MÃ HOÁ (VNI Windows dùng 2 ký
    /// tự cho một chữ có dấu). Cùng thuật toán rewrite_to bên engine IBus.
    void rewriteCommitted(fcitx::InputContext *ic, OnikeyState *state) {
        const std::string next = state->display();
        const std::string &prev = state->committed_;
        // Phần đầu chung: so theo byte rồi LÙI về ranh giới ký tự UTF-8 gần
        // nhất (byte tiếp diễn có dạng 10xxxxxx) — cắt giữa một ký tự là xoá
        // lệch nửa chữ.
        size_t common = 0;
        while (common < prev.size() && common < next.size() &&
               prev[common] == next[common]) {
            ++common;
        }
        while (common > 0 &&
               (static_cast<unsigned char>(prev[common - 1]) & 0xC0) == 0x80 &&
               common < prev.size() &&
               (static_cast<unsigned char>(prev[common]) & 0xC0) == 0x80) {
            --common;
        }
        while (common > 0 && common < prev.size() &&
               (static_cast<unsigned char>(prev[common]) & 0xC0) == 0x80) {
            --common;
        }
        const std::string removed = prev.substr(common);
        const std::string tail = next.substr(common);
        const size_t delChars =
            fcitx::utf8::length(state->encoded(removed));
        if (delChars > 0) {
            ic->deleteSurroundingText(-static_cast<int>(delChars),
                                      static_cast<unsigned>(delChars));
        }
        if (!tail.empty()) {
            ic->commitString(state->encoded(tail));
        }
        state->committed_ = next;
    }

    fcitx::Instance *instance_;
    fcitx::FactoryFor<OnikeyState> factory_;
};

class OnikeyFcitxEngineFactory final : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        return new OnikeyFcitxEngine(manager->instance());
    }
};

} // namespace

FCITX_ADDON_FACTORY(OnikeyFcitxEngineFactory)

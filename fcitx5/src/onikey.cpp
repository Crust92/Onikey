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
};

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
                return; // không có gì đang gõ -> app tự xoá
            }
            onikey_engine_remove_last_char(state->engine_, true);
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
        if (!s.empty()) {
            ic->commitString(state->encoded(s));
        }
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

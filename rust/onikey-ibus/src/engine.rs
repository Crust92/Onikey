//! Engine IBus của Onikey (bản Rust).
//!
//! Bản đầu chỉ làm chế độ **Pre-edit** — chế độ mặc định và tin cậy nhất. Các
//! chế độ không gạch chân (Surrounding Text) sẽ thêm sau, khi bản này đã chạy
//! thật ổn định.
//!
//! Bài học đã trả giá bên bản Go, bê thẳng sang đây:
//!
//!   - **Phải hiện thực `org.freedesktop.DBus.Properties.Set`**: từ IBus 1.5,
//!     kiểu ô nhập (`ContentType`) gửi qua THUỘC TÍNH DBus chứ không qua phương
//!     thức `SetContentType`. Thiếu nó thì engine mù, đúng như `goibus`.
//!   - **Không gọi gì đồng bộ ra ngoài trong đường xử lý phím/focus.** Bản Go
//!     từng mất 13ms mỗi lần focus chỉ vì hỏi gnome-shell một câu vô ích.

use std::sync::Mutex;

use onikey_core::{flag, flatten::mode, rules::parse_input_method, Engine as Core};
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zvariant::{OwnedValue, Value};

use crate::config::{ibflag, Config};
use crate::ibus_text::{IBusText, ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, PREEDIT_COMMIT};

/// Bit "ứng dụng cung cấp được surrounding text" trong IBus capabilities.
const IBUS_CAP_SURROUNDING_TEXT: u32 = 1 << 5;

/// Phím IBus cần biết tới.
const IBUS_BACKSPACE: u32 = 0xff08;
const IBUS_RETURN: u32 = 0xff0d;
const IBUS_ESCAPE: u32 = 0xff1b;
const IBUS_SPACE: u32 = 0x020;
const IBUS_RELEASE_MASK: u32 = 1 << 30;
const IBUS_CONTROL_MASK: u32 = 1 << 2;
const IBUS_MOD1_MASK: u32 = 1 << 3;
const IBUS_SUPER_MASK: u32 = 1 << 26;

pub struct OnikeyEngine {
    state: Mutex<State>,
}

struct State {
    core: Core,
    cfg: Config,
    /// Kiểu ô nhập do ứng dụng khai (IBusInputPurpose). Chỉ ghi nhận để gỡ rối.
    content_purpose: u32,
    capabilities: u32,
    /// Chuỗi đã GHI RA ứng dụng ở chế độ không gạch chân — cần nhớ để biết
    /// phải xoá lùi bao nhiêu ký tự khi chữ thay đổi.
    committed: String,
}

impl State {
    /// Kết thúc một tiếng: ở chế độ Pre-edit thì phải CHỐT chuỗi vào ứng dụng;
    /// ở chế độ không gạch chân thì chữ đã nằm sẵn trong đó rồi, chỉ cần quên đi.
    fn finish_word(&mut self) -> Action {
        let s = self.core.get_processed_string(mode::VIETNAMESE);
        self.core.reset();
        if self.no_underline() {
            self.committed.clear();
            return Action::Passthrough(None);
        }
        self.committed.clear();
        Action::Passthrough(if s.is_empty() { None } else { Some(s) })
    }

    /// Có gõ được kiểu không gạch chân không? Chỉ khi người dùng bật cờ VÀ ứng
    /// dụng cung cấp được surrounding text — thiếu thì thà gạch chân còn hơn
    /// nuốt phím (bài học từ ô địa chỉ Edge bên bản Go).
    fn no_underline(&self) -> bool {
        self.cfg.ib_flags & ibflag::NO_UNDERLINE != 0
            && self.capabilities & IBUS_CAP_SURROUNDING_TEXT != 0
    }
}

impl OnikeyEngine {
    pub fn new(cfg: Config) -> OnikeyEngine {
        let core = Core::new(parse_input_method(&cfg.input_method), cfg.flags);
        OnikeyEngine {
            state: Mutex::new(State {
                core,
                cfg,
                content_purpose: 0,
                capabilities: 0,
                committed: String::new(),
            }),
        }
    }
}

/// Số ký tự phải xoá lùi và phần đuôi phải ghi thêm, để biến `old` thành `new`.
/// Giữ lại phần đầu giống nhau — xoá cả rồi ghi lại là nháy chữ và chậm.
fn diff_tail(old: &str, new: &str) -> (u32, String) {
    let o: Vec<char> = old.chars().collect();
    let n: Vec<char> = new.chars().collect();
    let mut k = 0;
    while k < o.len() && k < n.len() && o[k] == n[k] {
        k += 1;
    }
    ((o.len() - k) as u32, n[k..].iter().collect())
}

fn is_modifier_pressed(state: u32) -> bool {
    state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_SUPER_MASK) != 0
}

/// Việc cần làm sau khi đã quyết định xong — TÁCH KHỎI phần giữ khoá, vì giữ
/// mutex qua `await` là lỗi biên dịch (và cũng là lỗi thiết kế: khoá phải nhả
/// trước khi nói chuyện ra ngoài).
enum Action {
    /// Không xử lý, để phím rơi xuống ứng dụng.
    Ignore,
    /// Chốt chuỗi đang gõ (nếu có) rồi vẫn để phím rơi xuống ứng dụng.
    Passthrough(Option<String>),
    /// Cập nhật chuỗi đang gõ, nuốt phím (chế độ Pre-edit, có gạch chân).
    Preedit(String),
    /// Chế độ KHÔNG gạch chân: xoá lùi `n` ký tự đã ghi rồi ghi thêm `tail`.
    /// Chữ nằm thẳng trong ứng dụng nên không có gạch chân nào cả.
    Rewrite { backspaces: u32, tail: String },
}

impl OnikeyEngine {
    /// Phần quyết định: THUẦN ĐỒNG BỘ, không await, không gọi ra ngoài.
    fn decide(&self, keyval: u32, state: u32) -> Action {
        if state & IBUS_RELEASE_MASK != 0 {
            return Action::Ignore;
        }
        let mut st = self.state.lock().unwrap();

        if is_modifier_pressed(state) {
            // Ctrl/Alt/Super + phím là phím tắt của ứng dụng, không phải chữ.
            return st.finish_word();
        }

        match keyval {
            IBUS_BACKSPACE => {
                if st.core.get_processed_string(mode::VIETNAMESE).is_empty() {
                    // Không có gì đang gõ dở -> để ứng dụng tự xoá.
                    st.committed.clear();
                    return Action::Ignore;
                }
                st.core.remove_last_char(true);
                let s = st.core.get_processed_string(mode::VIETNAMESE);
                if st.no_underline() {
                    // Ở chế độ không gạch chân, chữ đã nằm trong ứng dụng: cứ
                    // để ứng dụng tự xoá một ký tự, ta chỉ theo dõi trạng thái.
                    st.committed = s;
                    return Action::Ignore;
                }
                return Action::Preedit(s);
            }
            IBUS_RETURN | IBUS_ESCAPE => return st.finish_word(),
            _ => {}
        }

        let chr = match char::from_u32(keyval) {
            Some(c) => c,
            None => return Action::Ignore,
        };

        // Phím ngắt từ (dấu cách, dấu câu) -> chốt chữ đang gõ rồi cho qua.
        if keyval == IBUS_SPACE || !st.core.can_process_key(chr) {
            return st.finish_word();
        }

        st.core.process_key(chr, mode::VIETNAMESE);
        let s = st.core.get_processed_string(mode::VIETNAMESE);
        if st.no_underline() {
            let (backspaces, tail) = diff_tail(&st.committed, &s);
            st.committed = s;
            return Action::Rewrite { backspaces, tail };
        }
        Action::Preedit(s)
    }

    fn take_pending(&self) -> Option<String> {
        let mut st = self.state.lock().unwrap();
        match st.finish_word() {
            Action::Passthrough(s) => s,
            _ => None,
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl OnikeyEngine {
    async fn process_key_event(
        &self,
        keyval: u32,
        _keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let action = self.decide(keyval, state);
        crate::debug::log(format_args!(
            "phím 0x{keyval:04x} {:?} state=0x{state:x} -> {}",
            char::from_u32(keyval).unwrap_or('?'),
            match &action {
                Action::Ignore => "bỏ qua".to_string(),
                Action::Passthrough(s) => format!("cho qua, chốt {s:?}"),
                Action::Preedit(s) => format!("pre-edit {s:?}"),
                Action::Rewrite { backspaces, tail } => {
                    format!("sửa: xoá {backspaces}, ghi {tail:?}")
                }
            }
        ));
        match action {
            Action::Ignore => false,
            Action::Passthrough(pending) => {
                if let Some(s) = pending {
                    let _ = commit(&emitter, &s).await;
                }
                false
            }
            Action::Preedit(s) => {
                let _ = update_preedit(&emitter, &s).await;
                true
            }
            Action::Rewrite { backspaces, tail } => {
                if backspaces > 0 {
                    let _ = emitter
                        .emit(
                            "org.freedesktop.IBus.Engine",
                            "DeleteSurroundingText",
                            &(-(backspaces as i32), backspaces),
                        )
                        .await;
                }
                if !tail.is_empty() {
                    let _ = commit_raw(&emitter, &tail).await;
                }
                true
            }
        }
    }

    async fn focus_in(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        {
            let mut st = self.state.lock().unwrap();
            st.core.reset();
            st.committed.clear();
        }
        // Báo cho ứng dụng biết ta cần surrounding text; thiếu bước này thì
        // capability không bao giờ có bit tương ứng và chế độ không gạch chân
        // sẽ không bật được.
        let _ = emitter
            .emit("org.freedesktop.IBus.Engine", "RequireSurroundingText", &())
            .await;
        // Đăng ký menu thuộc tính để bấm vào biểu tượng `vi` trên thanh trên
        // là thấy mục cấu hình — như bản Go.
        let props = OwnedValue::try_from(Value::from(crate::ibus_prop::onikey_prop_list()))
            .expect("dựng PropList");
        let _ = emitter
            .emit("org.freedesktop.IBus.Engine", "RegisterProperties", &(props,))
            .await;
    }

    async fn focus_out(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        if let Some(s) = self.take_pending() {
            let _ = commit(&emitter, &s).await;
        }
    }

    async fn reset(&self) {
        self.state.lock().unwrap().core.reset();
    }

    async fn enable(&self) {}
    async fn disable(&self) {}
    async fn destroy(&self) {}

    async fn set_capabilities(&self, caps: u32) {
        self.state.lock().unwrap().capabilities = caps;
    }

    async fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    async fn set_surrounding_text(&self, _text: Value<'_>, _cursor: u32, _anchor: u32) {}

    async fn property_activate(&self, name: String, _state: u32) {
        crate::debug::log(format_args!("PropertyActivate: {name}"));
        match name.as_str() {
            crate::ibus_prop::KEY_CONFIGURATION => {
                // spawn chứ không chờ: hộp thoại sống bao lâu kệ nó, engine
                // không được đứng chờ trong handler DBus.
                let _ = std::process::Command::new("/usr/lib/onikey/onikey-config")
                    .arg("-engine")
                    .arg("onikey")
                    .spawn();
            }
            crate::ibus_prop::KEY_ABOUT => {
                let _ = std::process::Command::new("xdg-open")
                    .arg("https://github.com/xtcrust/Onikey")
                    .spawn();
            }
            _ => {}
        }
    }

    async fn page_up(&self) -> bool {
        false
    }
    async fn page_down(&self) -> bool {
        false
    }
    async fn cursor_up(&self) -> bool {
        false
    }
    async fn cursor_down(&self) -> bool {
        false
    }

    /// CHỖ MÀ goibus THIẾU: IBus 1.5 gửi kiểu ô nhập bằng thuộc tính DBus.
    #[zbus(property, name = "ContentType")]
    fn set_content_type(&self, value: (u32, u32)) {
        let mut st = self.state.lock().unwrap();
        if st.content_purpose != value.0 {
            st.core.reset();
            st.content_purpose = value.0;
        }
    }
}

async fn update_preedit(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()> {
    let len = text.chars().count() as u32;
    if len == 0 {
        return emitter
            .emit("org.freedesktop.IBus.Engine", "HidePreeditText", &())
            .await;
    }
    let t = IBusText::with_attr(text, ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, 0, len);
    let v = OwnedValue::try_from(Value::from(t)).expect("dựng IBusText");
    emitter
        .emit(
            "org.freedesktop.IBus.Engine",
            "UpdatePreeditText",
            &(v, len, true, PREEDIT_COMMIT),
        )
        .await
}

/// Ghi chữ thẳng vào ứng dụng, không đụng tới pre-edit.
async fn commit_raw(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()> {
    let v = OwnedValue::try_from(Value::from(IBusText::new(text))).expect("dựng IBusText");
    emitter
        .emit("org.freedesktop.IBus.Engine", "CommitText", &(v,))
        .await
}

async fn commit(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()> {
    let v = OwnedValue::try_from(Value::from(IBusText::new(text))).expect("dựng IBusText");
    emitter
        .emit("org.freedesktop.IBus.Engine", "HidePreeditText", &())
        .await?;
    emitter
        .emit("org.freedesktop.IBus.Engine", "CommitText", &(v,))
        .await
}

pub fn default_flags() -> u32 {
    flag::STD_FLAGS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chi_sua_phan_duoi_khac_nhau() {
        // "tie" -> "tiê": giữ "ti", xoá 1, ghi "ê"
        assert_eq!(diff_tail("tie", "tiê"), (1, "ê".to_string()));
        // thêm chữ: không phải xoá gì
        assert_eq!(diff_tail("tiê", "tiên"), (0, "n".to_string()));
        // đổi dấu ở giữa: "tiêng" -> "tiếng"
        assert_eq!(diff_tail("tiêng", "tiếng"), (3, "ếng".to_string()));
        // từ rỗng
        assert_eq!(diff_tail("", "t"), (0, "t".to_string()));
    }
}

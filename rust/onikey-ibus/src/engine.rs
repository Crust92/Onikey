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

use crate::ibus_text::{IBusText, ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, PREEDIT_COMMIT};

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
    /// Kiểu ô nhập do ứng dụng khai (IBusInputPurpose). Chỉ ghi nhận để gỡ rối.
    content_purpose: u32,
    capabilities: u32,
}

impl OnikeyEngine {
    pub fn new(input_method: &str, flags: u32) -> OnikeyEngine {
        OnikeyEngine {
            state: Mutex::new(State {
                core: Core::new(parse_input_method(input_method), flags),
                content_purpose: 0,
                capabilities: 0,
            }),
        }
    }
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
    /// Cập nhật chuỗi đang gõ, nuốt phím.
    Preedit(String),
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
            let s = st.core.get_processed_string(mode::VIETNAMESE);
            st.core.reset();
            return Action::Passthrough(if s.is_empty() { None } else { Some(s) });
        }

        match keyval {
            IBUS_BACKSPACE => {
                if st.core.get_processed_string(mode::VIETNAMESE).is_empty() {
                    return Action::Ignore;
                }
                st.core.remove_last_char(true);
                return Action::Preedit(st.core.get_processed_string(mode::VIETNAMESE));
            }
            IBUS_RETURN | IBUS_ESCAPE => {
                let s = st.core.get_processed_string(mode::VIETNAMESE);
                st.core.reset();
                return Action::Passthrough(if s.is_empty() { None } else { Some(s) });
            }
            _ => {}
        }

        let chr = match char::from_u32(keyval) {
            Some(c) => c,
            None => return Action::Ignore,
        };

        // Phím ngắt từ (dấu cách, dấu câu) -> chốt chữ đang gõ rồi cho qua.
        if keyval == IBUS_SPACE || !st.core.can_process_key(chr) {
            let s = st.core.get_processed_string(mode::VIETNAMESE);
            st.core.reset();
            return Action::Passthrough(if s.is_empty() { None } else { Some(s) });
        }

        st.core.process_key(chr, mode::VIETNAMESE);
        Action::Preedit(st.core.get_processed_string(mode::VIETNAMESE))
    }

    fn take_pending(&self) -> Option<String> {
        let mut st = self.state.lock().unwrap();
        let s = st.core.get_processed_string(mode::VIETNAMESE);
        st.core.reset();
        if s.is_empty() {
            None
        } else {
            Some(s)
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
        match self.decide(keyval, state) {
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
        }
    }

    async fn focus_in(&self) {
        self.state.lock().unwrap().core.reset();
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

    async fn property_activate(&self, _name: String, _state: u32) {}

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

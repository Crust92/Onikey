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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use onikey_core::{flag, flatten::mode, rules::parse_input_method, Engine as Core};
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zvariant::{OwnedValue, Value};

use crate::config::{ibflag, shortcut, Config};
use crate::ibus_text::{IBusText, ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, PREEDIT_COMMIT};

/// Bit "ứng dụng cung cấp được surrounding text" trong IBus capabilities.
const IBUS_CAP_SURROUNDING_TEXT: u32 = 1 << 5;
/// IBusInputPurpose: ô địa chỉ / ô URL.
const IBUS_INPUT_PURPOSE_URL: u32 = 5;

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
    /// Đồng bộ theo sự kiện cho chế độ không gạch chân: sau khi xoá lùi phải
    /// CHỜ ứng dụng xác nhận (nó gửi lại SetSurroundingText) rồi mới ghi chữ.
    /// Gửi liền hai lệnh thì lúc gõ nhanh ứng dụng áp lệch thứ tự — chữ trộn
    /// lẫn kiểu "password" -> "passsowrd". Bản Go trả giá rồi mới có cơ chế này.
    st_confirm: tokio::sync::Notify,
    awaiting_confirm: AtomicBool,
}

struct State {
    core: Core,
    cfg: Config,
    macros: crate::macros::MacroTable,
    /// Kiểu ô nhập do ứng dụng khai (IBusInputPurpose). Chỉ ghi nhận để gỡ rối.
    content_purpose: u32,
    capabilities: u32,
    /// Chuỗi đã GHI RA ứng dụng ở chế độ không gạch chân — cần nhớ để biết
    /// phải xoá lùi bao nhiêu ký tự khi chữ thay đổi.
    committed: String,
    /// Tạm tắt tiếng Việt (phím tắt chuyển Anh–Việt).
    english_mode: bool,
    /// Mtime của tệp cấu hình lúc nạp — đổi thì nạp lại ở lần focus sau,
    /// người dùng chỉnh trong hộp thoại là ăn ngay, không cần ibus restart.
    cfg_mtime: Option<std::time::SystemTime>,
}

fn config_mtime() -> Option<std::time::SystemTime> {
    std::fs::metadata(crate::config::config_path())
        .and_then(|m| m.modified())
        .ok()
}

impl State {
    /// Nạp lại cấu hình nếu tệp đã đổi (gọi ở FocusIn — ngoài đường xử lý phím).
    fn reload_config_if_changed(&mut self) {
        let mtime = config_mtime();
        if mtime == self.cfg_mtime {
            return;
        }
        self.cfg_mtime = mtime;
        let cfg = crate::config::load();
        crate::debug::log(format_args!(
            "cấu hình đổi: kiểu gõ {:?}, chế độ {}",
            cfg.input_method, cfg.default_input_mode
        ));
        self.core = Core::new(parse_input_method(&cfg.input_method), cfg.flags);
        self.macros = if cfg.ib_flags & ibflag::MACRO_ENABLED != 0 {
            crate::macros::MacroTable::load(cfg.ib_flags & ibflag::AUTO_CAPITALIZE_MACRO != 0)
        } else {
            crate::macros::MacroTable::default()
        };
        self.cfg = cfg;
        self.committed.clear();
    }

    /// Chuyển sang bảng mã đầu ra (TCVN3, VNI Windows…). Unicode trả nguyên.
    fn encode(&self, s: &str) -> String {
        onikey_core::charsets::encode(&self.cfg.output_charset, s)
    }

    /// Dựng bước sửa chữ cho chế độ không gạch chân, có tính bảng mã đầu ra:
    /// backspaces đếm theo chuỗi ĐÃ MÃ HOÁ (VNI Windows dùng 2 ký tự cho một
    /// chữ có dấu — xoá theo số ký tự Unicode là xoá thiếu).
    fn rewrite_to(&mut self, new_display: String) -> Action {
        let (cut, tail) = diff_tail(&self.committed, &new_display);
        let old_chars: Vec<char> = self.committed.chars().collect();
        let keep = old_chars.len() - cut as usize;
        let removed_encoded: String = self.encode(&old_chars[keep..].iter().collect::<String>());
        let backspaces = removed_encoded.chars().count() as u32;
        let tail_encoded = self.encode(&tail);
        self.committed = new_display;
        Action::Rewrite {
            backspaces,
            tail: tail_encoded,
        }
    }

    /// Chuỗi nên hiển thị cho người gõ: tiếng Việt đã bỏ dấu, hoặc chuỗi phím
    /// gốc nếu từ rõ ràng không phải tiếng Việt.
    fn display_string(&self) -> String {
        onikey_core::display::display_string(
            &self.core,
            self.cfg.ib_flags & ibflag::AUTO_NON_VN_RESTORE != 0,
            self.cfg.ib_flags & ibflag::DD_FREE_STYLE != 0,
        )
    }

    /// Kết thúc một tiếng: ở chế độ Pre-edit thì phải CHỐT chuỗi vào ứng dụng;
    /// ở chế độ không gạch chân thì chữ đã nằm sẵn trong đó rồi, chỉ cần quên đi.
    fn finish_word(&mut self) -> Action {
        let s = self.display_string();
        self.core.reset();
        // Gõ tắt: chuỗi vừa gõ trùng khoá -> thay bằng bản mở rộng.
        if !self.macros.is_empty() {
            if let Some(expanded) = self.macros.expand(&s) {
                let backspaces = if self.no_underline() {
                    self.committed.chars().count() as u32
                } else {
                    0
                };
                self.committed.clear();
                return Action::Expand {
                    backspaces,
                    text: expanded,
                };
            }
        }
        if self.no_underline() {
            self.committed.clear();
            return Action::Passthrough(None);
        }
        self.committed.clear();
        Action::Passthrough(if s.is_empty() {
            None
        } else {
            Some(self.encode(&s))
        })
    }

    /// Có gõ được kiểu không gạch chân không? Điều kiện tiên quyết: ứng dụng
    /// phải cung cấp surrounding text — thiếu thì thà gạch chân còn hơn nuốt
    /// phím (bài học ô địa chỉ Edge). Đủ điều kiện thì:
    ///   - chế độ 2 trở lên: luôn không gạch chân;
    ///   - chế độ 1 (Pre-edit): riêng Ô ĐỊA CHỈ trình duyệt (purpose=URL) nếu
    ///     người dùng bật nút gạt — pre-edit phá gợi ý của thanh địa chỉ.
    fn no_underline(&self) -> bool {
        if self.capabilities & IBUS_CAP_SURROUNDING_TEXT == 0 {
            return false;
        }
        if self.cfg.default_input_mode != 1 {
            return true;
        }
        self.cfg.ib_flags & ibflag::URL_NO_UNDERLINE != 0
            && self.content_purpose == IBUS_INPUT_PURPOSE_URL
    }
}

impl OnikeyEngine {
    pub fn new(cfg: Config) -> OnikeyEngine {
        let core = Core::new(parse_input_method(&cfg.input_method), cfg.flags);
        OnikeyEngine {
            state: Mutex::new(State {
                macros: if cfg.ib_flags & ibflag::MACRO_ENABLED != 0 {
                    crate::macros::MacroTable::load(
                        cfg.ib_flags & ibflag::AUTO_CAPITALIZE_MACRO != 0,
                    )
                } else {
                    crate::macros::MacroTable::default()
                },
                core,
                cfg,
                content_purpose: 0,
                capabilities: 0,
                committed: String::new(),
                english_mode: false,
                cfg_mtime: config_mtime(),
            }),
            st_confirm: tokio::sync::Notify::new(),
            awaiting_confirm: AtomicBool::new(false),
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
    /// Gõ tắt vừa nở: xoá `backspaces` ký tự đã ghi (0 ở chế độ Pre-edit),
    /// ghi bản mở rộng, rồi vẫn để phím ngắt từ rơi xuống ứng dụng.
    Expand { backspaces: u32, text: String },
}

impl OnikeyEngine {
    async fn register_props(&self, emitter: &SignalEmitter<'_>) {
        let list = {
            let st = self.state.lock().unwrap();
            crate::ibus_prop::onikey_prop_list(&st.cfg)
        };
        let props = OwnedValue::try_from(Value::from(list)).expect("dựng PropList");
        let _ = emitter
            .emit("org.freedesktop.IBus.Engine", "RegisterProperties", &(props,))
            .await;
    }

    /// Xoá lùi `n` ký tự đã ghi rồi CHỜ ứng dụng xác nhận — trần chờ thấp:
    /// app chậm quá thì thà ghi sớm còn hơn dồn phím (con số bản Go đã dò).
    async fn delete_committed(&self, emitter: &SignalEmitter<'_>, backspaces: u32) {
        if backspaces == 0 {
            return;
        }
        self.awaiting_confirm.store(true, Ordering::SeqCst);
        let _ = emitter
            .emit(
                "org.freedesktop.IBus.Engine",
                "DeleteSurroundingText",
                &(-(backspaces as i32), backspaces),
            )
            .await;
        let _ = emitter
            .emit("org.freedesktop.IBus.Engine", "RequireSurroundingText", &())
            .await;
        let _ = tokio::time::timeout(Duration::from_millis(60), self.st_confirm.notified()).await;
        self.awaiting_confirm.store(false, Ordering::SeqCst);
    }

    /// Phần quyết định: THUẦN ĐỒNG BỘ, không await, không gọi ra ngoài.
    fn decide(&self, keyval: u32, state: u32) -> Action {
        if state & IBUS_RELEASE_MASK != 0 {
            return Action::Ignore;
        }
        let mut st = self.state.lock().unwrap();

        // Phím tắt chuyển Anh–Việt: chốt từ đang gõ rồi lật công tắc.
        if st.cfg.shortcut_matches(shortcut::VI_EN_SWITCH, keyval, state) {
            let done = st.finish_word();
            st.english_mode = !st.english_mode;
            crate::debug::log(format_args!("chuyển Anh–Việt: english={}", st.english_mode));
            return match done {
                Action::Passthrough(s) => Action::Expand {
                    backspaces: 0,
                    text: s.unwrap_or_default(),
                },
                other => other,
            };
        }
        // Phím tắt khôi phục phím gốc: thay chữ đang gõ bằng đúng chuỗi đã bấm.
        if st.cfg.shortcut_matches(shortcut::RESTORE_KEY_STROKES, keyval, state) {
            if st.core.get_processed_string(mode::VIETNAMESE).is_empty() {
                return Action::Ignore;
            }
            st.core.restore_last_word(false);
            let s = st.display_string();
            if st.no_underline() {
                return st.rewrite_to(s);
            }
            return Action::Preedit(s);
        }
        if st.english_mode {
            return Action::Ignore; // đang tắt tiếng Việt: mọi phím đi thẳng
        }

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
                let s = st.display_string();
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
        let s = st.display_string();
        if st.no_underline() {
            return st.rewrite_to(s);
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
                Action::Expand { backspaces, text } => {
                    format!("gõ tắt: xoá {backspaces}, nở {text:?}")
                }
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
            Action::Expand { backspaces, text } => {
                self.delete_committed(&emitter, backspaces).await;
                let _ = commit_raw(&emitter, &text).await;
                // pre-edit đang treo (nếu có) đã bị thay bằng commit, ẩn đi
                let _ = emitter
                    .emit("org.freedesktop.IBus.Engine", "HidePreeditText", &())
                    .await;
                false // phím ngắt từ vẫn rơi xuống ứng dụng
            }
            Action::Rewrite { backspaces, tail } => {
                self.delete_committed(&emitter, backspaces).await;
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
            st.reload_config_if_changed();
        }
        // Báo cho ứng dụng biết ta cần surrounding text; thiếu bước này thì
        // capability không bao giờ có bit tương ứng và chế độ không gạch chân
        // sẽ không bật được.
        let _ = emitter
            .emit("org.freedesktop.IBus.Engine", "RequireSurroundingText", &())
            .await;
        // Đăng ký menu thuộc tính (dựng theo cấu hình hiện tại để radio đúng).
        self.register_props(&emitter).await;
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

    async fn set_surrounding_text(&self, _text: Value<'_>, _cursor: u32, _anchor: u32) {
        // Ứng dụng báo lại surrounding text — nếu đang chờ xác nhận xoá lùi
        // thì đây chính là tín hiệu "tôi đã áp xong", cho phép ghi tiếp.
        if self.awaiting_confirm.swap(false, Ordering::SeqCst) {
            self.st_confirm.notify_one();
        }
    }

    async fn property_activate(
        &self,
        name: String,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        use crate::ibus_prop as pr;
        crate::debug::log(format_args!("PropertyActivate: {name} state={state}"));

        let mut changed = false;
        if let Some(im) = name.strip_prefix(pr::PREFIX_INPUT_METHOD) {
            let _ = crate::config::save_string("InputMethod", im);
            let mut st = self.state.lock().unwrap();
            st.cfg.input_method = im.to_string();
            st.core = Core::new(parse_input_method(im), st.cfg.flags);
            st.committed.clear();
            changed = true;
        } else if let Some(cs) = name.strip_prefix(pr::PREFIX_CHARSET) {
            let _ = crate::config::save_string("OutputCharset", cs);
            self.state.lock().unwrap().cfg.output_charset = cs.to_string();
            changed = true;
        } else if let Some(m) = name.strip_prefix(pr::PREFIX_INPUT_MODE) {
            if let Ok(m) = m.parse::<u32>() {
                let _ = crate::config::save_number("DefaultInputMode", m);
                let mut st = self.state.lock().unwrap();
                st.cfg.default_input_mode = m;
                st.core.reset();
                st.committed.clear();
                changed = true; // vẽ lại menu: nút gạt ô địa chỉ hiện/ẩn theo chế độ
            }
        } else {
            match name.as_str() {
                pr::KEY_MACRO_ENABLED
                | pr::KEY_AUTO_CAPITALIZE
                | pr::KEY_NON_VN_RESTORE
                | pr::KEY_URL_NO_UNDERLINE => {
                    let bit = match name.as_str() {
                        pr::KEY_MACRO_ENABLED => ibflag::MACRO_ENABLED,
                        pr::KEY_AUTO_CAPITALIZE => ibflag::AUTO_CAPITALIZE_MACRO,
                        pr::KEY_URL_NO_UNDERLINE => ibflag::URL_NO_UNDERLINE,
                        _ => ibflag::AUTO_NON_VN_RESTORE,
                    };
                    let mut st = self.state.lock().unwrap();
                    if state != 0 {
                        st.cfg.ib_flags |= bit;
                    } else {
                        st.cfg.ib_flags &= !bit;
                    }
                    let flags = st.cfg.ib_flags;
                    st.macros = if flags & ibflag::MACRO_ENABLED != 0 {
                        crate::macros::MacroTable::load(flags & ibflag::AUTO_CAPITALIZE_MACRO != 0)
                    } else {
                        crate::macros::MacroTable::default()
                    };
                    drop(st);
                    let _ = crate::config::save_number("IBflags", flags);
                    changed = true;
                }
                pr::KEY_CONFIGURATION | pr::KEY_MACRO_TABLE => {
                    // spawn chứ không chờ: engine không được đứng trong handler DBus.
                    let _ = std::process::Command::new("/usr/lib/onikey/onikey-config")
                        .arg("-engine")
                        .arg("onikey")
                        .spawn();
                }
                pr::KEY_ABOUT => {
                    let _ = std::process::Command::new("xdg-open")
                        .arg("https://github.com/xtcrust/Onikey")
                        .spawn();
                }
                _ => {}
            }
        }
        if changed {
            // Cập nhật mtime đã lưu để FocusIn sau không nạp đè, rồi vẽ lại
            // menu cho radio/toggle đúng trạng thái mới.
            self.state.lock().unwrap().cfg_mtime = config_mtime();
            self.register_props(&emitter).await;
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
            st.committed.clear();
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

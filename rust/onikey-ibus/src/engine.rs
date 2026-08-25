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
//!   - **Chốt chế độ gạch chân một lần cho cả từ.** Bản Go chốt lúc FocusIn
//!     (`updateNoUnderlineMode`) vì capability và kiểu ô nhập tới rải rác từ
//!     nhiều input context. Ở đây chốt muộn hơn một nhịp — lúc BẮT ĐẦU từ,
//!     sau khi ứng dụng đã kịp khai lại capabilities hậu FocusIn — nhưng cùng
//!     một nguyên tắc: trong lúc đang gõ dở, chế độ không được đổi.

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
    /// Đặt ở focus_out; ContentType đến thì xoá. Còn cờ đến focus_in nghĩa
    /// là ô mới KHÔNG khai kiểu ô — lúc đó mới được quên purpose cũ
    /// (ibus-daemon gửi ContentType TRƯỚC FocusIn, quên vô điều kiện từng
    /// xoá nhầm nhãn ô địa chỉ vừa nhận — lỗi "chuyển tab bị gạch chân").
    purpose_stale: bool,
    /// Thời điểm nhận ContentType gần nhất — phân biệt "app vừa khai xong
    /// rồi blur–refocus tức thì" (churn của Chromium quanh Ctrl+L, phải GIỮ
    /// nhãn) với "đổi sang ô khác thật" (nhãn đã cũ, phải quên).
    last_content_type: Option<std::time::Instant>,
    /// Thời điểm phím gõ cuối — churn (blur/refocus/Reset do chính ta ghi/xoá
    /// gây ra) luôn nổ ra NGAY sau một phím; người đổi ô thật thì chậm hơn.
    last_key: Option<std::time::Instant>,
    /// Purpose đến GIỮA TỪ thì treo ở đây, áp khi từ chốt xong — omnibox
    /// Chromium nhấp nháy purpose 5→0→5 quanh mỗi lần ta ghi/xoá, reset
    /// trạng thái theo từng nhịp nháy sẽ cắt vụn từ đang gõ.
    pending_purpose: Option<u32>,
    /// Bằng chứng chờ kiểm của lần sửa-có-xoá gần nhất: (chuỗi đáng lẽ bị
    /// xoá, chuỗi vừa ghi). So với surrounding text app gửi lại — nếu phần
    /// đáng-xoá vẫn đứng ngay trước phần mới thì ô này NUỐT lệnh xoá
    /// (omnibox Chromium/Edge qua text-input-v3 bị vậy).
    delete_evidence: Option<(String, String)>,
    /// Ô hiện tại bị bắt quả tang nuốt lệnh xoá → dùng Pre-edit đến khi đổi ô.
    rewrite_broken: bool,
    capabilities: u32,
    /// Số lần xoá lùi liên tiếp KHÔNG được ứng dụng xác nhận trong hạn chờ.
    /// Ô không xác nhận là ô ta không kiểm chứng được — hai lần liền thì coi
    /// như không đáng tin và lùi về Pre-edit, thà gạch chân còn hơn chồng chữ.
    delete_timeouts: u32,
    /// Chuỗi đã GHI RA ứng dụng ở chế độ không gạch chân — cần nhớ để biết
    /// phải xoá lùi bao nhiêu ký tự khi chữ thay đổi.
    committed: String,
    /// Tạm tắt tiếng Việt (phím tắt chuyển Anh–Việt).
    english_mode: bool,
    /// Mtime của tệp cấu hình lúc nạp — đổi thì nạp lại ở lần focus sau,
    /// người dùng chỉnh trong hộp thoại là ăn ngay, không cần ibus restart.
    cfg_mtime: Option<std::time::SystemTime>,
    /// Chế độ gạch chân đã CHỐT cho từ đang gõ; `None` = chưa gõ từ nào.
    ///
    /// Hai chế độ giữ chữ ở hai chỗ khác nhau — Pre-edit giữ trong engine,
    /// không-gạch-chân giữ thẳng trong ứng dụng (`committed`) — nên đổi chế độ
    /// GIỮA TỪ là bỏ rơi một nửa chữ: đang không gạch chân mà lùi về Pre-edit
    /// thì phần đã ghi nằm lại trong ô rồi pre-edit vẽ lại cả từ (chữ nhân
    /// đôi); ngược lại thì `committed` rỗng nên `rewrite_to` ghi thêm cả từ
    /// trong khi pre-edit vẫn treo. Chốt lúc bắt đầu từ và giữ nguyên tới khi
    /// từ kết thúc, mặc kệ capabilities/purpose nhấp nháy giữa chừng.
    mode_latch: Option<bool>,
}

fn config_mtime() -> Option<std::time::SystemTime> {
    std::fs::metadata(crate::config::config_path())
        .and_then(|m| m.modified())
        .ok()
}

impl State {
    /// Sự kiện (FocusIn/Reset/ContentType) nổ ra ngay sau hoạt động của chính
    /// ta (phím gõ hoặc ContentType vừa đến) là CHURN của ứng dụng, không phải
    /// người dùng đổi ô thật.
    fn is_churn(&self) -> bool {
        let d = std::time::Duration::from_millis(300);
        self.last_key.is_some_and(|t| t.elapsed() < d)
            || self.last_content_type.is_some_and(|t| t.elapsed() < d)
    }

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
        self.unlatch_mode();
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
        if backspaces > 0 {
            self.delete_evidence = Some((removed_encoded.clone(), tail_encoded.clone()));
        }
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
        // Đọc chốt của từ VỪA XONG (chữ đang nằm ở đâu là do nó quyết định),
        // rồi nhả ngay để từ sau chốt lại theo ô nhập lúc đó.
        let no_underline = self.no_underline();
        self.unlatch_mode();
        // Gõ tắt: chuỗi vừa gõ trùng khoá -> thay bằng bản mở rộng. Tra bằng
        // chuỗi HIỂN THỊ: khoá thô ("btw") khớp nhờ khôi phục tiếng Anh, khoá
        // có dấu ("đc" gõ "ddc") khớp nhờ flatten — cả hai đường đều qua đây.
        if !self.macros.is_empty() {
            if let Some(expanded) = self.macros.expand(&s) {
                let backspaces = if no_underline {
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
        if no_underline {
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

    /// Có gõ được kiểu không gạch chân không, theo trạng thái Ô NHẬP LÚC NÀY?
    /// Chỉ được hỏi ở ranh giới từ — trong lúc gõ dở thì dùng `no_underline`.
    ///
    /// Điều kiện tiên quyết: ứng dụng phải cung cấp surrounding text — thiếu
    /// thì thà gạch chân còn hơn nuốt phím (bài học ô địa chỉ Edge). Đủ điều
    /// kiện thì:
    ///   - chế độ 2 trở lên: luôn không gạch chân;
    ///   - chế độ 1 (Pre-edit): riêng Ô ĐỊA CHỈ trình duyệt (purpose=URL) nếu
    ///     người dùng bật nút gạt — pre-edit phá gợi ý của thanh địa chỉ.
    fn compute_no_underline(&self) -> bool {
        if self.capabilities & IBUS_CAP_SURROUNDING_TEXT == 0 {
            return false;
        }
        if self.rewrite_broken {
            return false; // ô này nuốt lệnh xoá — sửa chữ sẽ chồng chữ
        }
        if self.cfg.default_input_mode != 1 {
            return true;
        }
        self.cfg.ib_flags & ibflag::URL_NO_UNDERLINE != 0
            && self.content_purpose == IBUS_INPUT_PURPOSE_URL
    }

    /// Chế độ của TỪ ĐANG GÕ. Chưa gõ từ nào thì chốt theo trạng thái hiện tại
    /// của ô nhập rồi giữ nguyên cho tới hết từ — xem `mode_latch`.
    fn no_underline(&mut self) -> bool {
        match self.mode_latch {
            Some(v) => v,
            None => {
                let v = self.compute_no_underline();
                crate::debug::log(format_args!(
                    "chốt chế độ cho từ mới: {} (cap={:#x} purpose={} broken={})",
                    if v { "không gạch chân" } else { "Pre-edit" },
                    self.capabilities,
                    self.content_purpose,
                    self.rewrite_broken
                ));
                self.mode_latch = Some(v);
                v
            }
        }
    }

    /// Từ đã kết thúc (chốt chữ, đổi ô, Reset, đổi cấu hình) — từ sau chốt lại
    /// chế độ từ đầu.
    fn unlatch_mode(&mut self) {
        self.mode_latch = None;
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
                purpose_stale: true,
                last_content_type: None,
                last_key: None,
                pending_purpose: None,
                delete_evidence: None,
                rewrite_broken: false,
                capabilities: 0,
                delete_timeouts: 0,
                committed: String::new(),
                english_mode: false,
                cfg_mtime: config_mtime(),
                mode_latch: None,
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
            crate::debug::log(format_args!(
                "register_props: mode={} ibflags={:#x}",
                st.cfg.default_input_mode, st.cfg.ib_flags
            ));
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
        let ok = tokio::time::timeout(Duration::from_millis(60), self.st_confirm.notified())
            .await
            .is_ok();
        self.awaiting_confirm.store(false, Ordering::SeqCst);
        let mut st = self.state.lock().unwrap();
        if ok {
            st.delete_timeouts = 0;
            return;
        }
        // Ô im lặng thì ta KHÔNG kiểm chứng được lệnh xoá có ăn hay không —
        // đối chiếu bằng surrounding text (delete_evidence) cũng bó tay vì
        // chẳng có surrounding text nào gửi về. Cửa sổ terminal/Electron kiểu
        // này nuốt lệnh xoá và chữ chồng lên nhau ("gõ" -> "goõ").
        // Một lần im lặng có thể chỉ là máy bận; hai lần liên tiếp thì coi như
        // ô không đáng tin, lùi về Pre-edit tới khi đổi ô — thà gạch chân còn
        // hơn chồng chữ.
        st.delete_timeouts += 1;
        crate::debug::log(format_args!(
            "xoá {backspaces}: KHÔNG có xác nhận trong 60ms (lần {})",
            st.delete_timeouts
        ));
        if st.delete_timeouts >= 2 && !st.rewrite_broken {
            st.rewrite_broken = true;
            st.core.reset();
            st.committed.clear();
            st.unlatch_mode();
            crate::debug::log(format_args!(
                "ô này không xác nhận lệnh xoá hai lần liền — lùi về Pre-edit đến khi đổi ô"
            ));
        }
    }

    /// Phần quyết định: THUẦN ĐỒNG BỘ, không await, không gọi ra ngoài.
    fn decide(&self, keyval: u32, state: u32) -> Action {
        if state & IBUS_RELEASE_MASK != 0 {
            return Action::Ignore;
        }
        let mut st = self.state.lock().unwrap();
        st.last_key = Some(std::time::Instant::now());
        if let Some(p) = st.pending_purpose {
            if st.committed.is_empty() && st.display_string().is_empty() {
                st.pending_purpose = None;
                if st.content_purpose != p {
                    crate::debug::log(format_args!(
                        "ContentType (áp sau khi hết từ): purpose {} -> {p}",
                        st.content_purpose
                    ));
                    st.content_purpose = p;
                    st.delete_evidence = None;
                }
            }
        }

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

        // Phím bổ trợ ĐỨNG MỘT MÌNH (Shift, Ctrl, Alt, Caps, Super...) không
        // phải ký tự — bỏ qua và GIỮ NGUYÊN từ đang gõ. Coi nó như phím
        // thường từng cắt từ mỗi lần nhấn Shift: "BTW" vỡ thành B|T|Ư.
        if (0xffe1..=0xffee).contains(&keyval) {
            return Action::Ignore;
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
                    st.unlatch_mode();
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
            "[{:p}] phím 0x{keyval:04x} {:?} state=0x{state:x} -> {}",
            &self.state,
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
            // Chromium blur–refocus quanh MỖI lần ta ghi/xoá (churn) — nhận
            // ra bằng ContentType vừa đến tức thì. Churn thì giữ nguyên hết:
            // từ đang gõ, nhãn ô, cờ chẩn đoán; reset lúc đó là cắt vụn từ.
            if !st.is_churn() {
                if st.rewrite_broken {
                    crate::debug::log(format_args!("focus_in THẬT: reset trạng thái ô (broken->false)"));
                }
                st.core.reset();
                st.committed.clear();
                // Ô mới, chế độ phải chốt lại: capability của ô trước không
                // nói gì về ô này.
                st.unlatch_mode();
                // Ô không khai ContentType (terminal, GTK cũ, Electron cũ)
                // không gửi gì khi được focus — purpose của ô trước sẽ rò
                // sang nếu cứ giữ. Ô có khai thì ibus-daemon gửi ContentType
                // TRƯỚC FocusIn, nên chỉ quên khi cờ stale còn từ focus_out.
                if st.purpose_stale && st.content_purpose != 0 {
                    crate::debug::log(format_args!(
                        "focus_in: quên purpose {} (ô mới không khai ContentType)",
                        st.content_purpose
                    ));
                    st.content_purpose = 0;
                }
                st.pending_purpose = None;
                st.delete_evidence = None;
                st.rewrite_broken = false;
                st.delete_timeouts = 0;
                // Ô mới phải tự khai lại surrounding text: set_capabilities giữ
                // bit này dính để chống chập chờn, nên phải xoá ở đây, không thì
                // một ô có hỗ trợ sẽ khiến ô sau (terminal, GTK cũ) bị tưởng là
                // có luôn.
                st.capabilities &= !IBUS_CAP_SURROUNDING_TEXT;
            }
            st.purpose_stale = true;
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
        self.state.lock().unwrap().purpose_stale = true;
        if let Some(s) = self.take_pending() {
            let _ = commit(&emitter, &s).await;
        }
    }

    async fn reset(&self) {
        let mut st = self.state.lock().unwrap();
        // Chromium gửi Reset sau MỖI lần ô thay đổi (kể cả do chính ta ghi/
        // xoá bù) — tôn trọng nó giữa từ là vứt từ đang gõ. Churn (ContentType
        // vừa đến tức thì) thì bỏ qua; Reset thật (đổi ô, bấm chuột) hiếm khi
        // rơi vào cửa sổ 300ms sau một keystroke.
        if st.is_churn() && (!st.committed.is_empty() || !st.display_string().is_empty()) {
            crate::debug::log(format_args!("Reset (churn) — giữ từ đang gõ"));
            return;
        }
        st.core.reset();
        st.unlatch_mode();
    }

    async fn enable(&self) {}
    async fn disable(&self) {}
    async fn destroy(&self) {}

    async fn set_capabilities(&self, caps: u32) {
        let mut st = self.state.lock().unwrap();
        // Chromium/GNOME lật capabilities qua lại trên CÙNG một input context:
        // đo trong một phiên gõ thấy 28 lần rơi mất bit surrounding text rồi
        // lấy lại (đi kèm churn blur–refocus quanh mỗi lần ta ghi/xoá). Mỗi lần
        // rơi mất, no_underline() trả false và chữ đang gõ bỗng mọc gạch chân
        // giữa chừng, dù người dùng đang bật chế độ không gạch chân.
        //
        // Giữ bit đó DÍNH cho tới lần focus THẬT (focus_in không phải churn sẽ
        // xoá đi để ô mới tự khai lại). Ô nào thực sự nuốt lệnh xoá thì đã có
        // rewrite_broken lo — đó mới là lá chắn đúng chỗ, chứ không phải cái
        // capability chập chờn này.
        let caps = caps | (st.capabilities & IBUS_CAP_SURROUNDING_TEXT);
        if st.capabilities != caps {
            crate::debug::log(format_args!(
                "capabilities: {:#x} -> {:#x}",
                st.capabilities, caps
            ));
        }
        st.capabilities = caps;
    }

    async fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    async fn set_surrounding_text(
        &self,
        text: Value<'_>,
        cursor: u32,
        _anchor: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        if crate::debug::enabled() {
            // IBusText serialize: struct (sa{sv}sv) — phần tử thứ 3 là chuỗi.
            let s = match &text {
                Value::Structure(st) => st
                    .fields()
                    .get(2)
                    .and_then(|f| match f {
                        Value::Str(s) => Some(s.as_str().to_string()),
                        _ => None,
                    })
                    .unwrap_or_default(),
                _ => String::new(),
            };
            let tail: String = s.chars().take(cursor as usize).collect();
            let tail: String = tail.chars().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect();
            crate::debug::log(format_args!("surrounding: …\"{tail}\" cursor={cursor}"));
        }
        // Ứng dụng báo lại surrounding text — nếu đang chờ xác nhận xoá lùi
        // thì đây chính là tín hiệu "tôi đã áp xong", cho phép ghi tiếp.
        if self.awaiting_confirm.swap(false, Ordering::SeqCst) {
            self.st_confirm.notify_one();
        }
        // Đối chiếu bằng chứng: chỉ xét bản surrounding ĐÃ chứa chuỗi mới
        // (bản trung gian sau-xoá-trước-ghi thì bỏ qua, không kết luận).
        let s = match &text {
            Value::Structure(st) => st
                .fields()
                .get(2)
                .and_then(|f| match f {
                    Value::Str(s) => Some(s.as_str().to_string()),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => return,
        };
        let repair = {
            let mut st = self.state.lock().unwrap();
            if let Some((deleted, new)) = st.delete_evidence.take() {
            let tail: String = s.chars().take(cursor as usize).collect();
            if tail.ends_with(&new) {
                let before = &tail[..tail.len() - new.len()];
                if !deleted.is_empty() && before.ends_with(&deleted) {
                    st.rewrite_broken = true;
                    // Chữ trong ô đang chồng (deleted + new). Ô này nuốt
                    // DeleteSurroundingText nhưng PHÍM BackSpace thật thì phải
                    // nhận — xoá bù bằng ForwardKeyEvent rồi tiếp tục từ đang
                    // gõ dạng pre-edit: từ đầu tiên cũng lành lặn.
                    let n_bs = (deleted.chars().count() + new.chars().count()) as u32;
                    st.committed.clear();
                    // Ép chốt về Pre-edit NGAY GIỮA TỪ — đây là ngoại lệ duy
                    // nhất được đổi chế độ khi đang gõ dở, và đổi được vì ta
                    // vừa xoá sạch phần đã ghi bằng ForwardKeyEvent bên dưới:
                    // chữ không còn nằm hai nơi nữa. Nhả chốt (`unlatch_mode`)
                    // ở đây thì phím sau lại hỏi lại và ra `true` như cũ, sửa
                    // xong lại hỏng ngay.
                    st.mode_latch = Some(false);
                    crate::debug::log(format_args!(
                        "ô này NUỐT lệnh xoá (thấy {deleted:?} vẫn đứng trước {new:?}) — xoá bù {n_bs} phím, về Pre-edit đến khi đổi ô"
                    ));
                    Some((n_bs, st.display_string()))
                } else {
                    None
                }
            } else {
                // chưa thấy chuỗi mới — trạng thái trung gian, trả lại chờ bản sau
                st.delete_evidence = Some((deleted, new));
                None
            }
            } else {
                None
            }
        };
        if let Some((n_bs, word)) = repair {
            self.state.lock().unwrap().last_key = Some(std::time::Instant::now());
            for _ in 0..n_bs {
                let _ = emitter
                    .emit(
                        "org.freedesktop.IBus.Engine",
                        "ForwardKeyEvent",
                        &(0xff08u32, 14u32, 0u32),
                    )
                    .await;
            }
            let _ = update_preedit(&emitter, &word).await;
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

        // GNOME gửi PropertyActivate cho CẢ NHÓM radio khi bấm một mục
        // (mục chọn state=1, mọi mục khác state=0, thứ tự đến không đảm
        // bảo). Radio vì thế chỉ được nhận state=CHECKED — xử lý cả loạt
        // state=0 từng làm config bị ghi thành mục ngẫu nhiên và menu bị
        // vẽ lại dồn dập không đóng được.
        let radio_checked = state == pr::STATE_CHECKED;
        let mut changed = false;
        if let Some(im) = name.strip_prefix(pr::PREFIX_INPUT_METHOD) {
            let mut st = self.state.lock().unwrap();
            if radio_checked && st.cfg.input_method != im {
                st.cfg.input_method = im.to_string();
                st.core = Core::new(parse_input_method(im), st.cfg.flags);
                st.committed.clear();
                st.unlatch_mode();
                drop(st);
                let _ = crate::config::save_string("InputMethod", im);
                changed = true;
            }
        } else if let Some(cs) = name.strip_prefix(pr::PREFIX_CHARSET) {
            let mut st = self.state.lock().unwrap();
            if radio_checked && st.cfg.output_charset != cs {
                st.cfg.output_charset = cs.to_string();
                drop(st);
                let _ = crate::config::save_string("OutputCharset", cs);
                changed = true;
            }
        } else if name == pr::KEY_MODE_NO_UNDERLINE {
            let m = if state != 0 { 2 } else { 1 };
            let mut st = self.state.lock().unwrap();
            if st.cfg.default_input_mode != m {
                st.cfg.default_input_mode = m;
                st.core.reset();
                st.committed.clear();
                // Người dùng vừa lật công tắc chế độ: chốt cũ hết hiệu lực.
                st.unlatch_mode();
                drop(st);
                let _ = crate::config::save_number("DefaultInputMode", m);
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
                    if (state != 0) == (st.cfg.ib_flags & bit != 0) {
                        return; // không đổi gì — sự kiện sync của panel, bỏ qua
                    }
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
                        .arg("https://github.com/Crust92/Onikey")
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
            if name == pr::KEY_MODE_NO_UNDERLINE {
                // RegisterProperties chỉ có tác dụng cho LẦN MỞ MENU SAU;
                // UpdateProperty mới vẽ lại mục lẻ ngay khi menu đang mở —
                // nút ô địa chỉ phải mờ/sáng tức thì theo công tắc chế độ.
                let url_prop = {
                    let st = self.state.lock().unwrap();
                    crate::ibus_prop::url_no_underline_prop(&st.cfg)
                };
                let v = OwnedValue::try_from(Value::from(url_prop)).expect("dựng IBusProperty");
                let _ = emitter
                    .emit("org.freedesktop.IBus.Engine", "UpdateProperty", &(v,))
                    .await;
            }
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
        st.purpose_stale = false;
        st.last_content_type = Some(std::time::Instant::now());
        if st.content_purpose == value.0 {
            st.pending_purpose = None;
            return;
        }
        if !st.committed.is_empty() || !st.display_string().is_empty() {
            // Giữa từ — treo lại, áp khi từ chốt (decide sẽ áp).
            st.pending_purpose = Some(value.0);
            return;
        }
        crate::debug::log(format_args!(
            "ContentType: purpose {} -> {} (5 = ô địa chỉ, broken {} -> false)",
            st.content_purpose, value.0, st.rewrite_broken
        ));
        st.content_purpose = value.0;
        st.pending_purpose = None;
        st.delete_evidence = None;
        // KHÔNG reset rewrite_broken ở đây: purpose nhấp nháy 5↔0 vẫn là
        // CÙNG MỘT Ô (churn của Chromium) — cờ chỉ về false khi đổi ô thật
        // (focus_in ngoài churn).
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

    /// Ô nhập lý tưởng: có surrounding text, chưa gõ gì. `mode` chọn chế độ gõ
    /// người dùng đặt trong hộp thoại (1 = Pre-edit, 2 = không gạch chân).
    fn state_thu(mode: u32) -> State {
        let cfg = Config {
            default_input_mode: mode,
            ..Config::default()
        };
        State {
            core: Core::new(parse_input_method(&cfg.input_method), cfg.flags),
            macros: crate::macros::MacroTable::default(),
            cfg,
            content_purpose: 0,
            purpose_stale: true,
            last_content_type: None,
            last_key: None,
            pending_purpose: None,
            delete_evidence: None,
            rewrite_broken: false,
            delete_timeouts: 0,
            capabilities: IBUS_CAP_SURROUNDING_TEXT,
            committed: String::new(),
            english_mode: false,
            cfg_mtime: None,
            mode_latch: None,
        }
    }

    #[test]
    fn capability_roi_giua_tu_khong_doi_duoc_che_do() {
        let mut st = state_thu(2);
        assert!(st.no_underline(), "đủ điều kiện -> không gạch chân");
        // Churn blur–refocus của Chromium làm rơi bit surrounding text giữa
        // chừng (đo được 28 lần trong một phiên gõ).
        st.capabilities &= !IBUS_CAP_SURROUNDING_TEXT;
        assert!(st.no_underline(), "đã chốt thì giữa từ không được đổi");
        st.finish_word();
        assert!(
            !st.no_underline(),
            "từ sau mới chốt lại: mất capability -> Pre-edit"
        );
    }

    #[test]
    fn o_dia_chi_chot_roi_giu_nguyen_du_purpose_nhap_nhay() {
        // Chế độ 1 + nút gạt URL (bật sẵn trong Config mặc định) = chế độ lai:
        // chỉ ô địa chỉ mới bỏ gạch chân.
        let mut st = state_thu(1);
        st.content_purpose = IBUS_INPUT_PURPOSE_URL;
        assert!(st.no_underline(), "ô địa chỉ -> không gạch chân");
        st.content_purpose = 0; // omnibox nháy 5 -> 0 giữa từ
        assert!(st.no_underline(), "vẫn giữ chốt tới hết từ");
        st.finish_word();
        assert!(!st.no_underline(), "ô thường -> Pre-edit");
    }

    #[test]
    fn ket_tu_theo_chot_cu_chu_khong_theo_trang_thai_moi() {
        let mut st = state_thu(2);
        // Đúng thứ tự của `decide`: gõ phím -> hỏi chế độ (chốt ở đây) -> ghi.
        st.core.process_key('a', mode::VIETNAMESE);
        let s = st.display_string();
        assert!(st.no_underline());
        st.rewrite_to(s); // "a" đã ghi thẳng vào ứng dụng
        assert_eq!(st.committed, "a");
        st.capabilities &= !IBUS_CAP_SURROUNDING_TEXT;
        // Tính lại theo trạng thái mới sẽ ra Pre-edit, và Pre-edit thì kết từ
        // bằng cách CHỐT chuỗi vào ứng dụng — trong khi "a" đã nằm sẵn trong
        // đó. Chốt cũ phải thắng, không thì chữ nhân đôi.
        assert!(
            matches!(st.finish_word(), Action::Passthrough(None)),
            "chữ đã nằm trong ô rồi, không được chốt lại lần nữa"
        );
        assert!(st.committed.is_empty());
    }

    #[test]
    fn duong_sua_chua_ep_ve_preedit_va_giu_toi_het_tu() {
        let mut st = state_thu(2);
        assert!(st.no_underline());
        // set_surrounding_text bắt quả tang ô nuốt lệnh xoá: xoá bù bằng phím
        // BackSpace thật rồi ép phần còn lại của từ đi đường Pre-edit.
        st.rewrite_broken = true;
        st.mode_latch = Some(false);
        assert!(!st.no_underline(), "phần còn lại của từ phải là Pre-edit");
        st.finish_word();
        assert!(
            !st.no_underline(),
            "rewrite_broken còn đó tới khi đổi ô -> từ sau vẫn Pre-edit"
        );
    }

    #[test]
    fn doi_o_thi_chot_lai_tu_dau() {
        let mut st = state_thu(2);
        assert!(st.no_underline());
        // focus_in THẬT: ô mới phải tự khai lại surrounding text.
        st.capabilities &= !IBUS_CAP_SURROUNDING_TEXT;
        st.unlatch_mode();
        assert!(!st.no_underline(), "ô mới chưa khai -> Pre-edit");
    }
}

//! Log gỡ rối ghi ra tệp, giống hệt cách bản Go làm.
//!
//! Engine do ibus-daemon khởi chạy nên stdout/stderr không xem được ở đâu cả —
//! đây là lý do bản Go phải có cơ chế này, và bản Rust cũng cần y như vậy.
//! Bật bằng cách tạo tệp cờ:
//!
//! ```sh
//! touch ~/.config/onikey/onikey-debug && ibus restart
//! ```

use std::fmt::Arguments;
use std::io::Write;
use std::sync::OnceLock;

fn log_file() -> Option<&'static std::path::PathBuf> {
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        let dir = crate::config::config_path().parent()?.to_path_buf();
        if !dir.join("onikey-debug").exists() {
            return None;
        }
        Some(dir.join("onikey-rust-debug.log"))
    })
    .as_ref()
}

/// Tệp log MỞ SẴN. Bản trước mở–ghi–đóng lại MỖI PHÍM; tệp phình lên vài chục
/// MB rồi máy tải nặng là mỗi lần mở phải chờ I/O — người gõ thấy chữ hiện trễ
/// đúng theo tải hệ thống. Giữ một handle: còn đúng một lần write cho mỗi dòng.
fn handle() -> Option<&'static std::sync::Mutex<std::fs::File>> {
    static FILE: OnceLock<Option<std::sync::Mutex<std::fs::File>>> = OnceLock::new();
    FILE.get_or_init(|| {
        let path = log_file()?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(std::sync::Mutex::new)
    })
    .as_ref()
}

pub fn log(args: Arguments<'_>) {
    let Some(f) = handle() else { return };
    if let Ok(mut f) = f.lock() {
        let _ = writeln!(f, "{args}");
    }
}

/// Log đang bật? — cho những chỗ cần chuẩn bị dữ liệu đắt trước khi log.
pub fn enabled() -> bool {
    log_file().is_some()
}

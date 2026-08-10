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

pub fn log(args: Arguments<'_>) {
    let Some(path) = log_file() else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{args}");
    }
}

/// Log đang bật? — cho những chỗ cần chuẩn bị dữ liệu đắt trước khi log.
pub fn enabled() -> bool {
    log_file().is_some()
}

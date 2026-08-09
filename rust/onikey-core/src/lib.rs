//! Lõi xử lý tiếng Việt của Onikey.
//!
//! Đây là bản viết lại bằng Rust của `bamboo-core` (Go, GPLv3). Mốc đúng-sai là
//! bộ ca kiểm `tests/corpus/core.jsonl.gz` sinh từ bản Go: bản này phải cho ra
//! đúng từng ký tự, ở TỪNG PHÍM.
//!
//! Trạng thái: đang port. Mảng đã xong: bảng tra (utils), luật gõ và định nghĩa
//! kiểu gõ (rules). Mảng còn lại: bộ máy biến đổi (composition/transformation),
//! đặt dấu thanh, kiểm tra chính tả.

pub mod rules;
pub mod utils;

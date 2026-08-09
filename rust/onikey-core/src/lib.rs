//! Lõi xử lý tiếng Việt của Onikey.
//!
//! Bản viết lại bằng Rust của `bamboo-core` (Go, GPLv3). Mốc đúng-sai là bộ ca
//! kiểm `tests/corpus/core.jsonl.gz` sinh từ bản Go: bản này phải cho ra đúng
//! từng ký tự, ở TỪNG PHÍM.

pub mod charsets;
pub mod display;
pub mod engine;
pub mod flatten;
pub mod rules;
pub mod spelling;
pub mod utils;

pub use engine::{flag, Engine};
pub use flatten::mode;

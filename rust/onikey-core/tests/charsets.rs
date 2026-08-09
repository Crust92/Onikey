//! Đối chiếu chuyển bảng mã (TCVN3, VNI Windows, VIQR…) với bản Go.
//!
//! Bảng tra do máy sinh từ chính bản Go, nên bài kiểm này nhắm vào phần còn
//! lại: hàm encode và việc bảng có được nạp đúng không.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use onikey_core::charsets::encode;

#[test]
fn chuyen_bang_ma_khop_ban_go() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/charsets.jsonl.gz");
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bỏ qua: chưa có {} ({e})", path.display());
            return;
        }
    };
    let reader = BufReader::new(flate2::read::GzDecoder::new(file));
    let (mut total, mut bad) = (0usize, 0usize);
    for line in reader.lines() {
        let line = line.expect("đọc fixture");
        if line.trim().is_empty() {
            continue;
        }
        let c: serde_json::Value = serde_json::from_str(&line).unwrap();
        let (cs, input, want) = (
            c["charset"].as_str().unwrap(),
            c["in"].as_str().unwrap(),
            c["out"].as_str().unwrap(),
        );
        total += 1;
        let got = encode(cs, input);
        if got != want {
            if bad < 10 {
                eprintln!("[{cs}] {input:?}: mong đợi {want:?}, nhận {got:?}");
            }
            bad += 1;
        }
    }
    eprintln!("đã đối chiếu {total} ca chuyển bảng mã, lệch {bad}");
    assert_eq!(bad, 0);
}

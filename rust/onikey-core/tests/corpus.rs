//! Đối chiếu lõi Rust với bộ ca kiểm sinh từ bản Go.
//!
//! Đây là thước đo tiến độ của cuộc port: in ra tỉ lệ ca kiểm khớp và những
//! chỗ lệch đầu tiên. Ngưỡng `NGUONG_TOI_THIEU` được nâng dần khi lõi hoàn
//! thiện — đặt thấp rồi quên nâng thì bài kiểm này thành vô dụng.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use onikey_core::{flatten::mode, rules::parse_input_method, Engine};

/// Tỉ lệ ca kiểm phải khớp (phần trăm). Nâng dần khi port xong từng mảng.
const NGUONG_TOI_THIEU: f64 = 100.0;

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/core.jsonl.gz")
}

#[derive(Default)]
struct Stats {
    total: usize,
    ok: usize,
    by_field: BTreeMap<String, usize>,
    samples: Vec<String>,
}

#[test]
fn doi_chieu_bo_ca_kiem() {
    let path = corpus_path();
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bỏ qua: chưa có {} ({e})", path.display());
            return;
        }
    };
    let reader = BufReader::new(flate2::read::GzDecoder::new(file));

    let mut st = Stats::default();
    for line in reader.lines() {
        let line = line.expect("đọc corpus");
        if line.trim().is_empty() {
            continue;
        }
        let c: serde_json::Value = serde_json::from_str(&line).expect("dòng corpus hỏng");
        st.total += 1;

        let im_name = c["im"].as_str().unwrap();
        let flags = c["flags"].as_u64().unwrap() as u32;
        let keys = c["keys"].as_str().unwrap();
        let steps: Vec<&str> = c["steps"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_str().unwrap()).collect())
            .unwrap_or_default();

        let mut e = Engine::new(parse_input_method(im_name), flags);
        let mut fail: Option<(String, String, String)> = None;

        for (i, k) in keys.chars().enumerate() {
            e.process_key(k, mode::VIETNAMESE);
            let got = e.get_processed_string(mode::VIETNAMESE);
            if let Some(want) = steps.get(i) {
                if got != *want {
                    fail = Some((format!("bước {}", i + 1), want.to_string(), got));
                    break;
                }
            }
        }
        if fail.is_none() {
            let got = e.get_processed_string(mode::VIETNAMESE);
            let want = c["vi"].as_str().unwrap();
            if got != want {
                fail = Some(("vi".into(), want.into(), got));
            }
        }
        if fail.is_none() {
            let got = e.get_processed_string(mode::ENGLISH | mode::FULL_TEXT);
            let want = c["raw"].as_str().unwrap();
            if got != want {
                fail = Some(("raw".into(), want.into(), got));
            }
        }
        if fail.is_none() {
            let got = e.is_valid(false);
            let want = c["valid"].as_bool().unwrap();
            if got != want {
                fail = Some(("valid".into(), want.to_string(), got.to_string()));
            }
        }
        // Hai thao tác sửa, chỉ có ở bộ ca kiểm tay: xoá lùi một ký tự rồi
        // khôi phục phím gốc (Shift+Space trong bản IBus).
        if fail.is_none() {
            if let Some(want) = c["after_bs"].as_str() {
                e.remove_last_char(true);
                let got = e.get_processed_string(mode::VIETNAMESE);
                if got != want {
                    fail = Some(("after_bs".into(), want.into(), got));
                }
                if fail.is_none() {
                    if let Some(want) = c["after_restore"].as_str() {
                        e.restore_last_word(false);
                        let got = e.get_processed_string(mode::VIETNAMESE);
                        if got != want {
                            fail = Some(("after_restore".into(), want.into(), got));
                        }
                    }
                }
            }
        }

        match fail {
            None => st.ok += 1,
            Some((field, want, got)) => {
                *st.by_field.entry(field.clone()).or_insert(0) += 1;
                if st.samples.len() < 15 {
                    st.samples.push(format!(
                        "[{im_name} cờ={flags}] phím {keys:?}: {field} mong đợi {want:?}, nhận {got:?}"
                    ));
                }
            }
        }
    }

    let pct = st.ok as f64 * 100.0 / st.total as f64;
    eprintln!("\n===== ĐỐI CHIẾU LÕI RUST VỚI BẢN GO =====");
    eprintln!("khớp {}/{} ca kiểm = {:.2}%", st.ok, st.total, pct);
    if !st.by_field.is_empty() {
        eprintln!("lệch theo hạng mục:");
        let mut v: Vec<_> = st.by_field.iter().collect();
        v.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (field, n) in v.iter().take(10) {
            eprintln!("  {field}: {n}");
        }
        eprintln!("ví dụ:");
        for s in &st.samples {
            eprintln!("  {s}");
        }
    }

    assert!(
        pct >= NGUONG_TOI_THIEU,
        "mới khớp {pct:.2}%, dưới ngưỡng {NGUONG_TOI_THIEU}%"
    );
}

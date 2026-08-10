//! Đo hiệu năng lõi: µs cho mỗi phím, trên đúng bộ ca kiểm thật.
//!   cargo run --release -p onikey-core --example bench
use std::io::BufRead;
use std::time::Instant;

use onikey_core::{flatten::mode, rules::parse_input_method, Engine};

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus/core.jsonl.gz");
    let f = std::fs::File::open(path).expect("corpus");
    let reader = std::io::BufReader::new(flate2::read::GzDecoder::new(f));
    let mut cases: Vec<(String, u32, String)> = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap();
        if line.is_empty() { continue; }
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        cases.push((
            v["im"].as_str().unwrap().to_string(),
            v["flags"].as_u64().unwrap() as u32,
            v["keys"].as_str().unwrap().to_string(),
        ));
    }
    let total_keys: usize = cases.iter().map(|c| c.2.chars().count()).sum();
    let ims: std::collections::HashMap<String, _> = cases
        .iter()
        .map(|c| (c.0.clone(), parse_input_method(&c.0)))
        .collect();

    let t = Instant::now();
    let mut sink = 0usize;
    for (im, flags, keys) in &cases {
        let mut e = Engine::new(ims[im].clone(), *flags);
        for k in keys.chars() {
            e.process_key(k, mode::VIETNAMESE);
            sink += onikey_core::display::display_string(&e, true, true).len();
        }
    }
    let dt = t.elapsed();
    println!(
        "{} ca, {} phím trong {:.2?}  ->  {:.1} µs/phím (kèm display từng phím; sink={})",
        cases.len(), total_keys, dt,
        dt.as_micros() as f64 / total_keys as f64, sink
    );
}

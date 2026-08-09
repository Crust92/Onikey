//! Đối chiếu bảng luật gõ do bản Rust phân tích với bảng do bản Go phân tích.
//!
//! `tests/corpus/rules.json` sinh bằng `go run ./tools/dump-rules`. Đây là mốc
//! đầu tiên của cuộc port: phần DSL định nghĩa kiểu gõ ("A_Â", "UOA_ƯƠĂ__Ư",
//! "__ư") rất dễ hiểu sai, sai ở đây thì mọi thứ xây lên trên đều sai theo.

use std::collections::BTreeMap;
use std::path::PathBuf;

use onikey_core::rules::{parse_input_method, EffectType, INPUT_METHOD_DEFINITIONS};

fn effect_type_num(t: EffectType) -> i64 {
    // Khớp hằng số của bản Go: Appending=0, MarkTransformation=1,
    // ToneTransformation=2, Replacing=3.
    match t {
        EffectType::Appending => 0,
        EffectType::MarkTransformation => 1,
        EffectType::ToneTransformation => 2,
        EffectType::Replacing => 3,
    }
}

fn corpus_path() -> PathBuf {
    // tests chạy với thư mục hiện hành là gốc crate (rust/onikey-core)
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/rules.json")
}

#[test]
fn bang_luat_khop_ban_go() {
    let path = corpus_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "bỏ qua: chưa có {} ({e}) — sinh bằng: go run ./tools/dump-rules > tests/corpus/rules.json",
                path.display()
            );
            return;
        }
    };
    let go: serde_json::Value = serde_json::from_str(&data).expect("rules.json hỏng");
    let go = go.as_object().expect("rules.json phải là object");

    assert_eq!(
        go.len(),
        INPUT_METHOD_DEFINITIONS.len(),
        "số kiểu gõ lệch: Go {} vs Rust {}",
        go.len(),
        INPUT_METHOD_DEFINITIONS.len()
    );

    let mut checked_rules = 0usize;
    for (name, expected) in go {
        let im = parse_input_method(name);
        assert_eq!(&im.name, name);

        let sorted = |v: &Vec<char>| {
            let mut s: Vec<String> = v.iter().map(|c| c.to_string()).collect();
            s.sort();
            s
        };
        let want_list = |key: &str| -> Vec<String> {
            expected[key]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect()
        };

        assert_eq!(sorted(&im.keys), want_list("keys"), "[{name}] danh sách phím");
        assert_eq!(sorted(&im.super_keys), want_list("super_keys"), "[{name}] super_keys");
        assert_eq!(sorted(&im.tone_keys), want_list("tone_keys"), "[{name}] tone_keys");
        assert_eq!(
            sorted(&im.appending_keys),
            want_list("appending_keys"),
            "[{name}] appending_keys"
        );

        // Gom luật của bản Rust theo phím rồi so từng phím một.
        let mut ours: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
        for r in &im.rules {
            let mut obj = serde_json::Map::new();
            obj.insert("effect".into(), (r.effect as i64).into());
            obj.insert("effect_type".into(), effect_type_num(r.effect_type).into());
            obj.insert("effect_on".into(), r.effect_on.to_string().into());
            obj.insert("result".into(), r.result.to_string().into());
            if !r.appended_rules.is_empty() {
                let appended: Vec<serde_json::Value> = r
                    .appended_rules
                    .iter()
                    .map(|a| serde_json::Value::from(a.result.to_string()))
                    .collect();
                obj.insert("appended".into(), appended.into());
            }
            ours.entry(r.key.to_string())
                .or_default()
                .push(serde_json::Value::Object(obj));
        }

        let want_rules = expected["rules_by_key"].as_object().unwrap();
        let want_keys: Vec<&String> = want_rules.keys().collect();
        let our_keys: Vec<&String> = ours.keys().collect();
        assert_eq!(
            our_keys.len(),
            want_keys.len(),
            "[{name}] số phím có luật lệch: Rust {:?} vs Go {:?}",
            our_keys,
            want_keys
        );

        for (key, want) in want_rules {
            let got = ours
                .get(key)
                .unwrap_or_else(|| panic!("[{name}] thiếu luật cho phím {key:?}"));
            let want = want.as_array().unwrap();
            assert_eq!(
                got.len(),
                want.len(),
                "[{name}] phím {key:?}: số luật lệch"
            );
            for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
                assert_eq!(g, w, "[{name}] phím {key:?} luật thứ {i}");
            }
            checked_rules += want.len();
        }
    }
    eprintln!("đã đối chiếu {checked_rules} luật gõ, khớp hoàn toàn");
}

// Luật gõ và định nghĩa các kiểu gõ (Telex, VNI, VIQR...).
//
// Port sát từ `rules_parser.go` + `input_method_def.go` của bamboo-core. Định
// nghĩa kiểu gõ là một DSL nhỏ: "A_Â" nghĩa là phím này biến a thành â;
// "DauSac" là phím dấu sắc; "__ư" là phím gõ thẳng ra chữ ư.

use crate::utils::{
    add_tone_to_char, find_mark_from_char, get_mark_family, is_vowel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Appending,
    MarkTransformation,
    ToneTransformation,
    #[allow(dead_code)]
    Replacing,
}

/// Dấu phụ. Giá trị số PHẢI khớp vị trí trong họ dấu (xem utils::MARK_FAMILIES).
pub mod mark {
    pub const NONE: u8 = 0;
    pub const HAT: u8 = 1;
    pub const BREVE: u8 = 2;
    pub const HORN: u8 = 3;
    pub const DASH: u8 = 4;
    #[allow(dead_code)]
    pub const RAW: u8 = 5;
}

/// Dấu thanh. Giá trị số PHẢI khớp vị trí trong nhóm 6 của bảng nguyên âm.
pub mod tone {
    pub const NONE: u8 = 0;
    pub const GRAVE: u8 = 1;
    pub const ACUTE: u8 = 2;
    pub const HOOK: u8 = 3;
    pub const TILDE: u8 = 4;
    pub const DOT: u8 = 5;
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub key: char,
    /// Vừa là dấu thanh vừa là dấu phụ, tuỳ `effect_type` — y như bản Go dùng
    /// chung một ô `Effect uint8`.
    pub effect: u8,
    pub effect_type: EffectType,
    pub effect_on: char,
    pub result: char,
    pub appended_rules: Vec<Rule>,
}

impl Rule {
    pub fn appending(key: char) -> Rule {
        Rule {
            key,
            effect: 0,
            effect_type: EffectType::Appending,
            effect_on: key,
            result: key,
            appended_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputMethod {
    pub name: String,
    pub rules: Vec<Rule>,
    pub super_keys: Vec<char>,
    pub tone_keys: Vec<char>,
    pub appending_keys: Vec<char>,
    pub keys: Vec<char>,
}

fn tone_from_name(name: &str) -> Option<u8> {
    match name {
        "XoaDauThanh" => Some(tone::NONE),
        "DauSac" => Some(tone::ACUTE),
        "DauHuyen" => Some(tone::GRAVE),
        "DauNga" => Some(tone::TILDE),
        "DauNang" => Some(tone::DOT),
        "DauHoi" => Some(tone::HOOK),
        _ => None,
    }
}

/// Định nghĩa các kiểu gõ. Giữ nguyên thứ tự khai báo của bản Go không quan
/// trọng (bản Go duyệt map nên vốn không có thứ tự), nhưng nội dung phải khớp.
pub const INPUT_METHOD_DEFINITIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Telex",
        &[
            ("z", "XoaDauThanh"), ("s", "DauSac"), ("f", "DauHuyen"), ("r", "DauHoi"),
            ("x", "DauNga"), ("j", "DauNang"), ("a", "A_Â"), ("e", "E_Ê"), ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ"), ("d", "D_Đ"),
        ],
    ),
    (
        "VNI",
        &[
            ("0", "XoaDauThanh"), ("1", "DauSac"), ("2", "DauHuyen"), ("3", "DauHoi"),
            ("4", "DauNga"), ("5", "DauNang"), ("6", "AEO_ÂÊÔ"), ("7", "UO_ƯƠ"),
            ("8", "A_Ă"), ("9", "D_Đ"),
        ],
    ),
    (
        "VIQR",
        &[
            ("0", "XoaDauThanh"), ("'", "DauSac"), ("`", "DauHuyen"), ("?", "DauHoi"),
            ("~", "DauNga"), (".", "DauNang"), ("^", "AEO_ÂÊÔ"), ("+", "UO_ƯƠ"),
            ("*", "UO_ƯƠ"), ("(", "A_Ă"), ("d", "D_Đ"),
        ],
    ),
    (
        "Microsoft layout",
        &[
            ("8", "DauSac"), ("5", "DauHuyen"), ("6", "DauHoi"), ("7", "DauNga"),
            ("9", "DauNang"), ("1", "__ă"), ("!", "_Ă"), ("2", "__â"), ("@", "_Â"),
            ("3", "__ê"), ("#", "_Ê"), ("4", "__ô"), ("$", "_Ô"), ("0", "__đ"),
            (")", "_Đ"), ("[", "__ư"), ("{", "_Ư"), ("]", "__ơ"), ("}", "_Ơ"),
        ],
    ),
    (
        "Telex 2",
        &[
            ("z", "XoaDauThanh"), ("s", "DauSac"), ("f", "DauHuyen"), ("r", "DauHoi"),
            ("x", "DauNga"), ("j", "DauNang"), ("a", "A_Â"), ("e", "E_Ê"), ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ__Ư"), ("d", "D_Đ"), ("]", "__ư"), ("[", "__ơ"),
            ("}", "_Ư"), ("{", "_Ơ"),
        ],
    ),
    (
        "Telex + VNI",
        &[
            ("z", "XoaDauThanh"), ("s", "DauSac"), ("f", "DauHuyen"), ("r", "DauHoi"),
            ("x", "DauNga"), ("j", "DauNang"), ("a", "A_Â"), ("e", "E_Ê"), ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ"), ("d", "D_Đ"), ("0", "XoaDauThanh"), ("1", "DauSac"),
            ("2", "DauHuyen"), ("3", "DauHoi"), ("4", "DauNga"), ("5", "DauNang"),
            ("6", "AEO_ÂÊÔ"), ("7", "UO_ƯƠ"), ("8", "A_Ă"), ("9", "D_Đ"),
        ],
    ),
    (
        "Telex + VNI + VIQR",
        &[
            ("z", "XoaDauThanh"), ("s", "DauSac"), ("f", "DauHuyen"), ("r", "DauHoi"),
            ("x", "DauNga"), ("j", "DauNang"), ("a", "A_Â"), ("e", "E_Ê"), ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ"), ("d", "D_Đ"), ("0", "XoaDauThanh"), ("1", "DauSac"),
            ("2", "DauHuyen"), ("3", "DauHoi"), ("4", "DauNga"), ("5", "DauNang"),
            ("6", "AEO_ÂÊÔ"), ("7", "UO_ƯƠ"), ("8", "A_Ă"), ("9", "D_Đ"),
            ("'", "DauSac"), ("`", "DauHuyen"), ("?", "DauHoi"), ("~", "DauNga"),
            (".", "DauNang"), ("^", "AEO_ÂÊÔ"), ("+", "UO_ƯƠ"), ("*", "UO_ƯƠ"),
            ("(", "A_Ă"), ("\\", "D_Đ"),
        ],
    ),
    (
        "VNI Bàn phím tiếng Pháp",
        &[
            ("&", "XoaDauThanh"), ("é", "DauSac"), ("\"", "DauHuyen"), ("'", "DauHoi"),
            ("(", "DauNga"), ("-", "DauNang"), ("è", "AEO_ÂÊÔ"), ("_", "UO_ƯƠ"),
            ("ç", "A_Ă"), ("à", "D_Đ"),
        ],
    ),
    (
        "Telex W",
        &[
            ("z", "XoaDauThanh"), ("s", "DauSac"), ("f", "DauHuyen"), ("r", "DauHoi"),
            ("x", "DauNga"), ("j", "DauNang"), ("a", "A_Â"), ("e", "E_Ê"), ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ__Ư"), ("d", "D_Đ"),
        ],
    ),
];

pub fn parse_input_method(name: &str) -> InputMethod {
    let def = match INPUT_METHOD_DEFINITIONS.iter().find(|(n, _)| *n == name) {
        Some((_, d)) => *d,
        None => return InputMethod::default(),
    };
    let mut im = InputMethod {
        name: name.to_string(),
        ..Default::default()
    };
    for (key_str, line) in def {
        let key = match key_str.chars().next() {
            Some(k) => k,
            None => continue,
        };
        im.rules.extend(parse_rules(key, line));
        if line.to_lowercase().contains("uo") {
            im.super_keys.push(key);
        }
        im.keys.push(key);
    }
    for rule in &im.rules {
        match rule.effect_type {
            EffectType::Appending => im.appending_keys.push(rule.key),
            EffectType::ToneTransformation => im.tone_keys.push(rule.key),
            _ => {}
        }
    }
    im
}

pub fn parse_rules(key: char, line: &str) -> Vec<Rule> {
    if let Some(t) = tone_from_name(line) {
        return vec![Rule {
            key,
            effect: t,
            effect_type: EffectType::ToneTransformation,
            effect_on: '\0',
            result: '\0',
            appended_rules: Vec::new(),
        }];
    }
    parse_toneless_rules(key, line)
}

/// Tách DSL dạng `([a-zA-Z]+)_(\p{L}+)([_\p{L}]*)` mà không cần thư viện regex:
/// phần chữ cái ASCII, dấu `_`, phần chữ kết quả, rồi phần đuôi (gõ thẳng).
fn split_dsl(line: &str) -> Option<(Vec<char>, Vec<char>, String)> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == 0 || i >= chars.len() || chars[i] != '_' {
        return None;
    }
    let effective_ons: Vec<char> = chars[..i].to_vec();
    let mut j = i + 1;
    while j < chars.len() && chars[j].is_alphabetic() {
        j += 1;
    }
    if j == i + 1 {
        return None;
    }
    let results: Vec<char> = chars[i + 1..j].to_vec();
    let tail: String = chars[j..].iter().collect();
    Some((effective_ons, results, tail))
}

pub fn parse_toneless_rules(key: char, line: &str) -> Vec<Rule> {
    // CHÚ Ý: chỉ hạ chữ thường ở nhánh DSL. Nhánh gõ-thẳng phải giữ nguyên hoa
    // thường của định nghĩa, vì "_Ă" (hoa) là luật gõ ra chữ HOA, khác "__ă".
    let lower = line.to_lowercase();
    let mut rules = Vec::new();
    if let Some((effective_ons, results, tail)) = split_dsl(&lower) {
        for (i, effective_on) in effective_ons.iter().enumerate() {
            let result = match results.get(i) {
                Some(r) => *r,
                None => continue,
            };
            let effect = match find_mark_from_char(result) {
                Some(e) => e,
                None => continue,
            };
            rules.extend(parse_toneless_rule(key, *effective_on, result, effect));
        }
        if let Some(rule) = appending_rule(key, &tail) {
            rules.push(rule);
        }
    } else if let Some(rule) = appending_rule(key, line) {
        rules.push(rule);
    }
    rules
}

pub fn parse_toneless_rule(key: char, effective_on: char, result: char, effect: u8) -> Vec<Rule> {
    let mut rules = Vec::new();
    for chr in get_mark_family(effective_on) {
        if chr == result {
            // gõ lại phím dấu -> gỡ dấu, quay về chữ gốc
            rules.push(Rule {
                key,
                effect: 0,
                effect_type: EffectType::MarkTransformation,
                effect_on: result,
                result: effective_on,
                appended_rules: Vec::new(),
            });
        } else if is_vowel(chr) {
            // sinh luật cho cả 6 thanh, để dấu phụ không làm mất dấu thanh
            for t in 0..6u8 {
                rules.push(Rule {
                    key,
                    effect,
                    effect_type: EffectType::MarkTransformation,
                    effect_on: add_tone_to_char(chr, t),
                    result: add_tone_to_char(result, t),
                    appended_rules: Vec::new(),
                });
            }
        } else {
            rules.push(Rule {
                key,
                effect,
                effect_type: EffectType::MarkTransformation,
                effect_on: chr,
                result,
                appended_rules: Vec::new(),
            });
        }
    }
    rules
}

/// Khớp `(_?)_(\p{L}+)`: một hoặc hai gạch dưới rồi tới các chữ gõ thẳng.
fn appending_rule(key: char, value: &str) -> Option<Rule> {
    let chars: Vec<char> = value.chars().collect();
    let pos = chars.iter().position(|c| *c == '_')?;
    let mut i = pos;
    // regex khớp tối đa hai gạch dưới liền nhau
    if i + 1 < chars.len() && chars[i + 1] == '_' {
        i += 1;
    }
    let start = i + 1;
    if start >= chars.len() || !chars[start].is_alphabetic() {
        return None;
    }
    let mut end = start;
    while end < chars.len() && chars[end].is_alphabetic() {
        end += 1;
    }
    let letters = &chars[start..end];
    let mut rule = Rule {
        key,
        effect: 0,
        effect_type: EffectType::Appending,
        effect_on: letters[0],
        result: letters[0],
        appended_rules: Vec::new(),
    };
    for chr in &letters[1..] {
        rule.appended_rules.push(Rule {
            key,
            effect: 0,
            effect_type: EffectType::Appending,
            effect_on: *chr,
            result: *chr,
            appended_rules: Vec::new(),
        });
    }
    Some(rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telex_co_du_phim() {
        let im = parse_input_method("Telex");
        assert_eq!(im.keys.len(), 11);
        assert!(im.tone_keys.contains(&'s'));
        assert!(im.super_keys.contains(&'w')); // "UOA_ƯƠĂ" chứa "uo"
    }

    #[test]
    fn luat_dau_mu_giu_dau_thanh() {
        // phím 'a' của Telex: a -> â, và phải có luật cho cả 6 thanh
        let rules = parse_rules('a', "A_Â");
        assert!(rules
            .iter()
            .any(|r| r.effect_on == 'á' && r.result == 'ấ'));
        // gõ lại 'a' khi đã có 'â' -> gỡ mũ
        assert!(rules
            .iter()
            .any(|r| r.effect_on == 'â' && r.result == 'a' && r.effect == 0));
    }

    #[test]
    fn phim_go_thang() {
        // Telex 2: phím ']' gõ thẳng ra 'ư'
        let rules = parse_rules(']', "__ư");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect_type, EffectType::Appending);
        assert_eq!(rules[0].result, 'ư');
    }
}

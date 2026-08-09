// Bảng tra nguyên âm, dấu phụ và dấu thanh.
//
// Port sát từ `utils.go` của bamboo-core. Cố ý giữ nguyên cách đánh chỉ số
// (vị trí trong chuỗi VOWELS, họ dấu "aâă__") vì toàn bộ thuật toán phía trên
// dựa vào phép tính chỉ số này, đổi cách biểu diễn là lệch hành vi ngay.

/// Nguyên âm xếp theo NHÓM 6: mỗi nhóm là một nguyên âm gốc với 6 thanh
/// (không dấu, huyền, sắc, hỏi, ngã, nặng) — thứ tự này khớp `Tone as u8`.
pub const VOWELS: &str =
    "aàáảãạăằắẳẵặâầấẩẫậeèéẻẽẹêềếểễệiìíỉĩịoòóỏõọôồốổỗộơờớởỡợuùúủũụưừứửữựyỳýỷỹỵ";

pub const PUNCTUATION_MARKS: &[char] = &[
    ',', ';', ':', '.', '"', '\'', '!', '?', ' ', '<', '>', '=', '+', '-', '*', '/', '\\', '_',
    '~', '`', '@', '#', '$', '%', '^', '&', '(', ')', '{', '}', '[', ']', '|',
];

/// Họ dấu của một chữ: vị trí trong chuỗi là loại dấu
/// (0 không dấu, 1 mũ, 2 trăng, 3 móc, 4 gạch ngang của "đ").
const MARK_FAMILIES: &[(char, &str)] = &[
    ('a', "aâă__"),
    ('â', "aâă__"),
    ('ă', "aâă__"),
    ('e', "eê___"),
    ('ê', "eê___"),
    ('o', "oô_ơ_"),
    ('ô', "oô_ơ_"),
    ('ơ', "oô_ơ_"),
    ('u', "u__ư_"),
    ('ư', "u__ư_"),
    ('d', "d___đ"),
    ('đ', "d___đ"),
];

fn mark_family_str(chr: char) -> Option<&'static str> {
    MARK_FAMILIES
        .iter()
        .find(|(c, _)| *c == chr)
        .map(|(_, s)| *s)
}

pub fn is_space(key: char) -> bool {
    key == ' '
}

pub fn is_punctuation_mark(key: char) -> bool {
    PUNCTUATION_MARKS.contains(&key)
}

pub fn is_word_break_symbol(key: char) -> bool {
    is_punctuation_mark(key) || ('0'..='9').contains(&key)
}

pub fn is_vowel(chr: char) -> bool {
    VOWELS.chars().any(|v| v == chr)
}

pub fn find_vowel_position(chr: char) -> Option<usize> {
    VOWELS.chars().position(|v| v == chr)
}

pub fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Các chữ cùng họ dấu với `chr` (bỏ ô trống "_").
pub fn get_mark_family(chr: char) -> Vec<char> {
    match mark_family_str(chr) {
        Some(s) => s.chars().filter(|c| *c != '_').collect(),
        None => Vec::new(),
    }
}

/// Vị trí của `chr` trong chính họ dấu của nó = loại dấu đang mang.
pub fn find_mark_position(chr: char) -> Option<usize> {
    let s = mark_family_str(chr)?;
    s.chars().position(|v| v == chr)
}

pub fn find_mark_from_char(chr: char) -> Option<u8> {
    find_mark_position(chr).map(|p| p as u8)
}

pub fn add_mark_to_toneless_char(chr: char, mark: u8) -> char {
    if let Some(s) = mark_family_str(chr) {
        let marks: Vec<char> = s.chars().collect();
        if let Some(&m) = marks.get(mark as usize) {
            if m != '_' {
                return m;
            }
        }
    }
    chr
}

pub fn add_mark_to_char(chr: char, mark: u8) -> char {
    let tone = find_tone_from_char(chr);
    let chr = add_tone_to_char(chr, 0);
    let chr = add_mark_to_toneless_char(chr, mark);
    add_tone_to_char(chr, tone)
}

/// Thanh điệu của một chữ = vị trí trong nhóm 6 của nó.
pub fn find_tone_from_char(chr: char) -> u8 {
    match find_vowel_position(chr) {
        Some(pos) => (pos % 6) as u8,
        None => 0,
    }
}

pub fn add_tone_to_char(chr: char, tone: u8) -> char {
    match find_vowel_position(chr) {
        Some(pos) => {
            let current = pos % 6;
            let offset = tone as isize - current as isize;
            let idx = (pos as isize + offset) as usize;
            VOWELS.chars().nth(idx).unwrap_or(chr)
        }
        None => chr,
    }
}

pub fn is_vietnamese_char(lower_key: char) -> bool {
    if find_tone_from_char(lower_key) != 0 {
        return true;
    }
    lower_key != add_mark_to_toneless_char(lower_key, 0)
}

pub fn can_process_key(lower_key: char, effect_keys: &[char]) -> bool {
    if is_alpha(lower_key) || effect_keys.contains(&lower_key) {
        return true;
    }
    if is_word_break_symbol(lower_key) {
        return false;
    }
    is_vietnamese_char(lower_key)
}

pub fn has_any_vietnamese_char(word: &str) -> bool {
    word.chars()
        .any(|c| is_vietnamese_char(c.to_lowercase().next().unwrap_or(c)))
}

pub fn has_any_vietnamese_vowel(word: &str) -> bool {
    word.chars()
        .any(|c| is_vowel(c.to_lowercase().next().unwrap_or(c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bang_nguyen_am_xep_theo_nhom_sau() {
        assert_eq!(VOWELS.chars().count() % 6, 0);
        // 'a' không dấu -> thêm thanh sắc (2) ra 'á'
        assert_eq!(add_tone_to_char('a', 2), 'á');
        // 'ắ' đang mang thanh sắc trên chữ 'ă'
        assert_eq!(find_tone_from_char('ắ'), 2);
        assert_eq!(add_tone_to_char('ắ', 0), 'ă');
    }

    #[test]
    fn ho_dau() {
        assert_eq!(get_mark_family('a'), vec!['a', 'â', 'ă']);
        assert_eq!(find_mark_position('â'), Some(1));
        assert_eq!(add_mark_to_char('á', 1), 'ấ'); // giữ thanh khi thêm mũ
        assert_eq!(add_mark_to_char('d', 4), 'đ');
    }
}

// Chuỗi HIỂN THỊ cho người gõ — logic dùng chung cho mọi adapter (IBus,
// Fcitx5, XIM...): tiếng Việt đã bỏ dấu, hoặc chuỗi phím gốc nếu từ rõ ràng
// không phải tiếng Việt ("expression", "password").
//
// Nằm ở lõi vì đây là quyết định NGÔN NGỮ chứ không phải chuyện DBus hay
// Fcitx: mọi adapter phải cho cùng một kết quả, chép mỗi nơi một bản là chúng
// lệch nhau dần.

use crate::engine::Engine;
use crate::flatten::mode;
use crate::utils::{has_any_vietnamese_char, has_any_vietnamese_vowel};

/// Có nên hiển thị CHUỖI PHÍM GỐC thay vì chuỗi đã bỏ dấu không.
/// Port sát shouldFallbackToEnglish của bản Go.
pub fn should_fallback_to_english(
    engine: &Engine,
    auto_restore: bool,
    dd_free_style: bool,
    check_vn_rune: bool,
) -> bool {
    if !auto_restore {
        return false;
    }
    let vn_lower = engine.get_processed_string(mode::VIETNAMESE | mode::LOWER_CASE);
    if vn_lower.is_empty() {
        return false;
    }
    // Viết tắt "dd"/"đ" rất hay dùng -> giữ đ dù không phải tiếng Việt
    if dd_free_style
        && !has_any_vietnamese_vowel(&vn_lower)
        && (vn_lower.chars().last() == Some('d') || vn_lower.contains('đ'))
    {
        return false;
    }
    if check_vn_rune && !has_any_vietnamese_char(&vn_lower) {
        return false;
    }
    !engine.is_valid(false)
}

/// Chuỗi nên đưa lên màn hình / chốt vào ứng dụng.
pub fn display_string(engine: &Engine, auto_restore: bool, dd_free_style: bool) -> String {
    if should_fallback_to_english(engine, auto_restore, dd_free_style, true) {
        return engine.get_processed_string(mode::ENGLISH);
    }
    engine.get_processed_string(mode::VIETNAMESE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::parse_input_method;

    #[test]
    fn khoi_phuc_tieng_anh() {
        let mut e = Engine::new(parse_input_method("Telex"), crate::flag::STD_FLAGS);
        for c in "expr".chars() {
            e.process_key(c, mode::VIETNAMESE);
        }
        assert_eq!(display_string(&e, true, true), "expr");
        assert_ne!(display_string(&e, false, true), "expr"); // tắt thì giữ dấu

        let mut e = Engine::new(parse_input_method("Telex"), crate::flag::STD_FLAGS);
        for c in "tieengs".chars() {
            e.process_key(c, mode::VIETNAMESE);
        }
        assert_eq!(display_string(&e, true, true), "tiếng");
    }
}

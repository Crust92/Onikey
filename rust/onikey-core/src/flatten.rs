// Trải composition thành chuỗi hiển thị. Port sát `flattener.go`.

use crate::engine::{Id, Transformation};
use crate::rules::{mark, EffectType};
use crate::utils::{add_mark_to_char, add_tone_to_char};

/// Cờ chế độ trải chuỗi, khớp bản Go.
pub mod mode {
    pub const VIETNAMESE: u32 = 1 << 0;
    pub const ENGLISH: u32 = 1 << 1;
    pub const TONE_LESS: u32 = 1 << 2;
    pub const MARK_LESS: u32 = 1 << 3;
    pub const LOWER_CASE: u32 = 1 << 4;
    pub const FULL_TEXT: u32 = 1 << 5;
    pub const PUNCTUATION: u32 = 1 << 6;
    pub const IN_REVERSE_ORDER: u32 = 1 << 7;
}

pub fn flatten(composition: &[Transformation], m: u32) -> String {
    get_canvas(composition, m).into_iter().collect()
}

pub fn get_canvas(composition: &[Transformation], m: u32) -> Vec<char> {
    // Danh sách chữ gõ ra, và các biến đổi bám vào từng chữ đó.
    let mut appending_list: Vec<&Transformation> = Vec::new();
    let mut effects: Vec<(Id, &Transformation)> = Vec::new();

    for trans in composition {
        if m & mode::ENGLISH != 0 {
            if trans.rule.key == '\0' {
                continue; // phím ảo không hiện trong chuỗi phím gốc
            }
            appending_list.push(trans);
        } else if trans.rule.effect_type == EffectType::Appending {
            if trans.rule.key == '\0' {
                continue;
            }
            appending_list.push(trans);
        } else if let Some(target) = trans.target {
            effects.push((target, trans));
        }
    }

    let mut canvas = Vec::with_capacity(appending_list.len());
    for app in appending_list {
        let mut chr;
        if m & mode::ENGLISH != 0 {
            chr = app.rule.key;
        } else {
            chr = app.rule.effect_on;
            for (target, trans) in &effects {
                if *target != app.id {
                    continue;
                }
                match trans.rule.effect_type {
                    EffectType::MarkTransformation => {
                        if trans.rule.effect == mark::RAW {
                            chr = app.rule.key;
                        } else {
                            chr = add_mark_to_char(chr, trans.rule.effect);
                        }
                    }
                    EffectType::ToneTransformation => {
                        chr = add_tone_to_char(chr, trans.rule.effect);
                    }
                    _ => {}
                }
            }
        }
        if m & mode::TONE_LESS != 0 {
            chr = add_tone_to_char(chr, 0);
        }
        if m & mode::MARK_LESS != 0 {
            chr = add_mark_to_char(chr, 0);
        }
        if m & mode::LOWER_CASE != 0 {
            chr = chr.to_lowercase().next().unwrap_or(chr);
        } else if app.is_upper_case {
            chr = chr.to_uppercase().next().unwrap_or(chr);
        }
        canvas.push(chr);
    }
    canvas
}

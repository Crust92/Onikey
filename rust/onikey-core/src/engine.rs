// Bộ máy biến đổi: nhận từng phím, dựng "composition" rồi trải ra thành chuỗi.
//
// Port sát `bamboo.go` + `bamboo_utils.go` + `flattener.go` của bamboo-core.
//
// HAI CHỖ PHẢI DỊCH CHỨ KHÔNG BÊ THẲNG ĐƯỢC:
//
//  1. Bản Go dùng `Target *Transformation` và SO SÁNH BẰNG ĐỊA CHỈ. Ở đây mỗi
//     biến đổi mang một `id` bền (bộ đếm tăng dần) và `target: Option<Id>`.
//     Dùng chỉ số mảng là sai ngay khi lọc/cắt mảng vì chỉ số dịch hết.
//
//  2. `refresh_last_tone_target` SỬA TẠI CHỖ target của một biến đổi đang nằm
//     trong composition. Bên Go mọi slice cùng trỏ vào một đối tượng nên ai
//     cũng thấy; bên này phải cố ý gộp composition với các biến đổi mới vào
//     một mảng, sửa trên đó, rồi mới tách ra — xem `generate_transformations`.

use crate::flatten::{flatten, get_canvas, mode};
use crate::rules::{mark, tone, EffectType, InputMethod, Rule};
use crate::spelling::is_valid_cvc;
use crate::utils::{
    add_tone_to_char, find_tone_from_char, is_alpha, is_space, is_vowel, is_word_break_symbol,
};

pub type Id = u32;

#[derive(Debug, Clone)]
pub struct Transformation {
    pub id: Id,
    pub rule: Rule,
    pub target: Option<Id>,
    pub is_upper_case: bool,
}

/// Cờ của lõi, khớp bản Go.
pub mod flag {
    pub const FREE_TONE_MARKING: u32 = 1 << 0;
    pub const STD_TONE_STYLE: u32 = 1 << 1;
    pub const AUTO_CORRECT_ENABLED: u32 = 1 << 2;
    pub const STD_FLAGS: u32 = FREE_TONE_MARKING | STD_TONE_STYLE | AUTO_CORRECT_ENABLED;
}

pub struct Engine {
    pub composition: Vec<Transformation>,
    input_method: InputMethod,
    flags: u32,
    next_id: Id,
}

pub fn find_by_id(comp: &[Transformation], id: Id) -> Option<&Transformation> {
    comp.iter().find(|t| t.id == id)
}

fn find_last_appending(comp: &[Transformation]) -> Option<&Transformation> {
    comp.iter().rev().find(|t| t.rule.effect_type == EffectType::Appending)
}

fn filter_appending(comp: &[Transformation]) -> Vec<Transformation> {
    comp.iter()
        .filter(|t| t.rule.effect_type == EffectType::Appending)
        .cloned()
        .collect()
}

/// Đi ngược chuỗi target về gốc. Target trỏ ra ngoài mảng hiện tại thì coi như
/// đã tới gốc — bên Go con trỏ vẫn còn giá trị, nhưng mọi phép so sánh sau đó
/// đều dựa trên id nên kết quả tương đương.
fn find_root_target(comp: &[Transformation], t: &Transformation) -> Id {
    let mut cur = t.clone();
    loop {
        match cur.target {
            None => return cur.id,
            Some(tid) => match find_by_id(comp, tid) {
                Some(next) => cur = next.clone(),
                None => return tid,
            },
        }
    }
}

fn is_free(comp: &[Transformation], target: Option<Id>, effect_type: EffectType) -> bool {
    !comp
        .iter()
        .any(|t| t.target == target && t.rule.effect_type == effect_type)
}

/// Tách phần đuôi "nguyên âm hay phụ âm" liên tiếp. Đệ quy trong bản Go, ở đây
/// viết thành vòng lặp cho khỏi tràn ngăn xếp.
fn extract_atomic(comp: &[Transformation], last_is_vowel: bool) -> (Vec<Transformation>, Vec<Transformation>) {
    let mut end = comp.len();
    while end > 0 {
        let t = &comp[end - 1];
        if t.target.is_none() && last_is_vowel != is_vowel(t.rule.result) {
            break;
        }
        end -= 1;
    }
    (comp[..end].to_vec(), comp[end..].to_vec())
}

/// Tách một tiếng thành phụ âm đầu / vần / phụ âm cuối.
fn extract_cvc_appending(
    comp: &[Transformation],
) -> (Vec<Transformation>, Vec<Transformation>, Vec<Transformation>) {
    let (head, mut last_consonant) = extract_atomic(comp, false);
    let (mut first_consonant, mut vowel) = extract_atomic(&head, true);
    if !last_consonant.is_empty() && vowel.is_empty() && first_consonant.is_empty() {
        first_consonant = last_consonant;
        vowel = Vec::new();
        last_consonant = Vec::new();
    }

    // 'gi' và 'qu' được tính là phụ âm đầu:
    //   ['g','ia',''] -> ['gi','a','']   ['q','ua',''] -> ['qu','a','']
    // trừ ['g','ie','ng'] thì giữ nguyên.
    if first_consonant.len() == 1 && !vowel.is_empty() {
        let fc0 = first_consonant[0].rule.result;
        let v0 = vowel[0].rule.result;
        let gi = fc0 == 'g'
            && v0 == 'i'
            && vowel.len() > 1
            && !(vowel[1].rule.result == 'e' && !last_consonant.is_empty());
        let qu = fc0 == 'q' && v0 == 'u';
        if gi || qu {
            first_consonant.push(vowel.remove(0));
        }
    }
    (first_consonant, vowel, last_consonant)
}

fn extract_cvc(
    comp: &[Transformation],
) -> (Vec<Transformation>, Vec<Transformation>, Vec<Transformation>) {
    let appending: Vec<Transformation> = comp.iter().filter(|t| t.target.is_none()).cloned().collect();
    let (mut fc, mut vo, mut lc) = extract_cvc_appending(&appending);

    // Gắn thêm các biến đổi nhắm vào từng phần. CHÚ Ý: bản Go duyệt đúng số
    // phần tử BAN ĐẦU (range chốt độ dài trước khi append), nên ở đây cũng chỉ
    // duyệt tới độ dài ban đầu.
    let attach = |group: &mut Vec<Transformation>| {
        let n = group.len();
        for i in 0..n {
            let id = group[i].id;
            let mut extra: Vec<Transformation> =
                comp.iter().filter(|t| t.target == Some(id)).cloned().collect();
            group.append(&mut extra);
        }
    };
    attach(&mut fc);
    attach(&mut vo);
    attach(&mut lc);
    (fc, vo, lc)
}

fn get_right_most_vowels(comp: &[Transformation]) -> Vec<Transformation> {
    extract_cvc(comp).1
}

fn has_valid_tone(comp: &[Transformation], t: u8) -> bool {
    if t == tone::NONE || t == tone::ACUTE || t == tone::DOT {
        return true;
    }
    let (_, _, lc) = extract_cvc(comp);
    if lc.is_empty() {
        return true;
    }
    let last_consonants = flatten(&lc, mode::ENGLISH | mode::LOWER_CASE);
    !matches!(last_consonants.as_str(), "c" | "k" | "p" | "t" | "ch")
}

fn is_valid(comp: &[Transformation], input_is_full_complete: bool) -> bool {
    if comp.len() <= 1 {
        return true;
    }
    for t in comp.iter().rev() {
        if t.rule.effect_type == EffectType::ToneTransformation {
            if !has_valid_tone(comp, t.rule.effect) {
                return false;
            }
            break;
        }
    }
    let (fc, vo, lc) = extract_cvc(comp);
    let m = mode::VIETNAMESE | mode::LOWER_CASE | mode::TONE_LESS;
    is_valid_cvc(
        &flatten(&fc, m),
        &flatten(&vo, m),
        &flatten(&lc, m),
        input_is_full_complete,
    )
}

fn find_tone_target(comp: &[Transformation], std_style: bool) -> Option<Id> {
    if comp.is_empty() {
        return None;
    }
    let (_, vo, lc) = extract_cvc(comp);
    let vowels = filter_appending(&vo);
    if vowels.len() == 1 {
        return Some(vowels[0].id);
    }
    if vowels.len() == 2 && std_style {
        let mut target: Option<Id> = None;
        for t in &vo {
            if t.rule.result == 'ơ' || t.rule.result == 'ê' {
                target = Some(t.target.unwrap_or(t.id));
            }
        }
        if target.is_none() {
            target = Some(if !lc.is_empty() { vowels[1].id } else { vowels[0].id });
        }
        return target;
    }
    if vowels.len() == 2 {
        if !lc.is_empty() {
            return Some(vowels[1].id);
        }
        let s = flatten(
            &vowels,
            mode::ENGLISH | mode::LOWER_CASE | mode::TONE_LESS | mode::MARK_LESS,
        );
        return Some(match s.as_str() {
            "oa" | "oe" | "uy" | "ue" | "uo" => vowels[1].id,
            _ => vowels[0].id,
        });
    }
    if vowels.len() == 3 {
        let s = flatten(
            &vowels,
            mode::ENGLISH | mode::LOWER_CASE | mode::TONE_LESS | mode::MARK_LESS,
        );
        return Some(if s == "uye" { vowels[2].id } else { vowels[1].id });
    }
    None
}

fn get_last_tone_trans_index(comp: &[Transformation]) -> Option<usize> {
    (0..comp.len()).rev().find(|i| {
        comp[*i].rule.effect_type == EffectType::ToneTransformation && comp[*i].target.is_some()
    })
}

/// Chuỗi "uơ"/"ưo" + ít nhất một chữ nữa (thay cho regex `(uơ|ưo)\p{L}+`).
fn matches_uoh_tail(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len().saturating_sub(2) {
        let pair = (chars[i], chars[i + 1]);
        if (pair == ('u', 'ơ') || pair == ('ư', 'o')) && chars[i + 2].is_alphabetic() {
            return true;
        }
    }
    false
}

/// Chứa "ưo" hoặc "ươ" (thay cho regex `(ưo|ươ)`).
fn matches_uh_o(s: &str) -> bool {
    s.contains("ưo") || s.contains("ươ")
}

impl Engine {
    pub fn new(input_method: InputMethod, flags: u32) -> Engine {
        Engine {
            composition: Vec::new(),
            input_method,
            flags,
            next_id: 1,
        }
    }

    pub fn input_method(&self) -> &InputMethod {
        &self.input_method
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }

    fn new_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn new_appending(&mut self, key: char, is_upper_case: bool) -> Transformation {
        Transformation {
            id: self.new_id(),
            rule: Rule::appending(key),
            target: None,
            is_upper_case,
        }
    }

    pub fn can_process_key(&self, key: char) -> bool {
        crate::utils::can_process_key(key, &self.input_method.keys)
    }

    fn applicable_rules(&self, key: char) -> Vec<Rule> {
        let lower = key.to_lowercase().next().unwrap_or(key);
        self.input_method
            .rules
            .iter()
            .filter(|r| r.key == lower)
            .cloned()
            .collect()
    }

    pub fn reset(&mut self) {
        self.composition.clear();
    }

    pub fn get_processed_string(&self, m: u32) -> String {
        if m & mode::FULL_TEXT != 0 {
            return flatten(&self.composition, m);
        }
        if m & mode::PUNCTUATION != 0 {
            let (_, last) = extract_last_word_with_punctuation(&self.composition);
            return flatten(&last, mode::VIETNAMESE);
        }
        let (_, last) = extract_last_word(&self.composition, &self.input_method.keys);
        flatten(&last, m)
    }

    pub fn is_valid(&self, input_is_full_complete: bool) -> bool {
        let (_, last) = extract_last_word(&self.composition, &self.input_method.keys);
        is_valid(&last, input_is_full_complete)
    }

    pub fn process_string(&mut self, s: &str, m: u32) {
        for c in s.chars() {
            self.process_key(c, m);
        }
    }

    pub fn process_key(&mut self, key: char, m: u32) {
        let lower_key = key.to_lowercase().next().unwrap_or(key);
        let is_upper_case = key.is_uppercase();
        if m & mode::ENGLISH != 0 || !self.can_process_key(lower_key) {
            let t = self.new_appending(lower_key, is_upper_case);
            if m & mode::IN_REVERSE_ORDER != 0 {
                self.composition.insert(0, t);
            } else {
                self.composition.push(t);
            }
            return;
        }
        let comp = std::mem::take(&mut self.composition);
        self.composition = self.new_composition(comp, lower_key, is_upper_case);
    }

    fn new_composition(
        &mut self,
        composition: Vec<Transformation>,
        key: char,
        is_upper_case: bool,
    ) -> Vec<Transformation> {
        let (mut previous, mut last_syllable) = extract_last_syllable(&composition);
        self.generate_transformations(&mut last_syllable, key, is_upper_case);
        previous.append(&mut last_syllable);
        previous
    }

    /// Sinh các biến đổi cho phím mới và ĐẨY THẲNG vào `comp`.
    ///
    /// Bản Go trả về mảng rồi caller nối vào; ở đây nối luôn vì bước làm mới
    /// vị trí dấu thanh cần sửa tại chỗ các phần tử đã có trong `comp`.
    fn generate_transformations(&mut self, comp: &mut Vec<Transformation>, lower_key: char, is_upper_case: bool) {
        let rules = self.applicable_rules(lower_key);
        let n = comp.len();

        let mut trans = self.gen_transformations(comp, &rules, lower_key, is_upper_case);
        if trans.is_empty() {
            trans = self.gen_fallback(&rules, lower_key, is_upper_case);
            let mut new_comp = comp.clone();
            new_comp.extend(trans.iter().cloned());
            if let Some(v) = self.apply_uow_shortcut(&new_comp) {
                trans.push(v);
            }
        }

        // Gộp lại thành một mảng để bước làm mới dấu thanh sửa được tại chỗ,
        // rồi tách phần mới ra sau — đúng ngữ nghĩa con trỏ của bản Go.
        comp.extend(trans);
        let refreshed = self.refresh_last_tone_target(comp);
        comp.extend(refreshed);
        let tail = comp.split_off(n);
        comp.extend(tail);
    }

    fn apply_uow_shortcut(&mut self, syllable: &[Transformation]) -> Option<Transformation> {
        let s = flatten(syllable, mode::TONE_LESS | mode::LOWER_CASE);
        if self.input_method.super_keys.is_empty() || !matches_uoh_tail(&s) {
            return None;
        }
        let rules = self.applicable_rules(self.input_method.super_keys[0]);
        let (target, mut rule) = find_target(syllable, &rules, self.flags);
        target.map(|t| {
            rule.key = '\0'; // phím ảo: không được hiện ra trong chuỗi phím gốc
            Transformation {
                id: self.new_id(),
                rule,
                target: Some(t),
                is_upper_case: false,
            }
        })
    }

    fn refresh_last_tone_target(&mut self, comp: &mut Vec<Transformation>) -> Vec<Transformation> {
        if self.flags & flag::FREE_TONE_MARKING == 0 || !is_valid(comp, false) {
            return Vec::new();
        }
        let std_style = self.flags & flag::STD_TONE_STYLE != 0;
        let right_most = get_right_most_vowels(comp);
        let last_tone_idx = match get_last_tone_trans_index(comp) {
            Some(i) => i,
            None => return Vec::new(),
        };
        if right_most.is_empty() {
            return Vec::new();
        }
        let new_target = find_tone_target(comp, std_style);
        if comp[last_tone_idx].target == new_target {
            return Vec::new();
        }
        // SỬA TẠI CHỖ, y như bản Go gán lastToneTrans.Target
        comp[last_tone_idx].target = new_target;
        let override_rule = {
            let mut r = comp[last_tone_idx].rule.clone();
            r.key = '\0';
            r
        };
        let id1 = self.new_id();
        let id2 = self.new_id();
        vec![
            Transformation {
                id: id1,
                rule: Rule {
                    key: '\0',
                    effect: tone::NONE,
                    effect_type: EffectType::ToneTransformation,
                    effect_on: '\0',
                    result: '\0',
                    appended_rules: Vec::new(),
                },
                target: new_target,
                is_upper_case: false,
            },
            Transformation {
                id: id2,
                rule: override_rule,
                target: new_target,
                is_upper_case: false,
            },
        ]
    }

    fn gen_fallback(&mut self, rules: &[Rule], lower_key: char, is_upper_case: bool) -> Vec<Transformation> {
        let mut out = Vec::new();
        let base = self.gen_appending_trans(rules, lower_key, is_upper_case);
        let appended = base.rule.appended_rules.clone();
        out.push(base);
        for ar in appended {
            let up = is_upper_case || ar.effect_on.is_uppercase();
            let lower = ar.effect_on.to_lowercase().next().unwrap_or(ar.effect_on);
            let id = self.new_id();
            out.push(Transformation {
                id,
                rule: Rule {
                    key: '\0', // phím ảo
                    effect: ar.effect,
                    effect_type: ar.effect_type,
                    effect_on: lower,
                    result: lower,
                    appended_rules: Vec::new(),
                },
                target: None,
                is_upper_case: up,
            });
        }
        out
    }

    fn gen_appending_trans(&mut self, rules: &[Rule], lower_key: char, is_upper_case: bool) -> Transformation {
        for rule in rules {
            if rule.key == lower_key && rule.effect_type == EffectType::Appending {
                let up = is_upper_case || rule.effect_on.is_uppercase();
                let lower = rule.effect_on.to_lowercase().next().unwrap_or(rule.effect_on);
                let mut r = rule.clone();
                r.effect_on = lower;
                r.result = lower;
                let id = self.new_id();
                return Transformation {
                    id,
                    rule: r,
                    target: None,
                    is_upper_case: up,
                };
            }
        }
        self.new_appending(lower_key, is_upper_case)
    }

    /// Phần lõi: xem phím mới tạo ra biến đổi gì (đặt dấu, gỡ dấu, hay gõ thẳng).
    fn gen_transformations(
        &mut self,
        comp: &[Transformation],
        rules: &[Rule],
        lower_key: char,
        is_upper_case: bool,
    ) -> Vec<Transformation> {
        // Gõ lặp phím dấu -> gỡ hiệu ứng, trả về chữ gốc (w + w -> w)
        if let Some(last) = comp.last() {
            let r = &last.rule;
            if r.effect_type == EffectType::Appending && r.key == lower_key && r.key != r.result {
                let id = self.new_id();
                return vec![Transformation {
                    id,
                    rule: Rule {
                        key: '\0',
                        effect: mark::RAW,
                        effect_type: EffectType::MarkTransformation,
                        effect_on: '\0',
                        result: '\0',
                        appended_rules: Vec::new(),
                    },
                    target: Some(last.id),
                    is_upper_case: false,
                }];
            }
        }

        let (target, applicable_rule) = find_target(comp, rules, self.flags);
        if let Some(target_id) = target {
            let id = self.new_id();
            let is_mark = applicable_rule.effect_type == EffectType::MarkTransformation;
            let mut out = vec![Transformation {
                id,
                rule: applicable_rule,
                target: Some(target_id),
                is_upper_case,
            }];
            if !is_mark {
                return out;
            }
            let mut new_comp = comp.to_vec();
            new_comp.extend(out.iter().cloned());
            if is_valid(&new_comp, true) {
                return out;
            }
            // lối tắt "uow": dựng một luật móc ảo nhắm vào 'u' hoặc 'o'
            let (t2, mut virtual_rule) = find_target(&new_comp, rules, self.flags);
            if let Some(t2) = t2 {
                virtual_rule.key = '\0';
                let id2 = self.new_id();
                out.push(Transformation {
                    id: id2,
                    rule: virtual_rule,
                    target: Some(t2),
                    is_upper_case: false,
                });
            }
            return out;
        }

        // ươ/ưo(i/c/ng) + o -> uô
        if matches_uh_o(&flatten(comp, mode::VIETNAMESE | mode::TONE_LESS | mode::LOWER_CASE)) {
            let vowels = filter_appending(&get_right_most_vowels(comp));
            if !vowels.is_empty() {
                let id = self.new_id();
                let trans = Transformation {
                    id,
                    rule: Rule {
                        key: '\0',
                        effect: mark::NONE,
                        effect_type: EffectType::MarkTransformation,
                        effect_on: '\0',
                        result: '\0',
                        appended_rules: Vec::new(),
                    },
                    target: Some(vowels[0].id),
                    is_upper_case: false,
                };
                let mut tmp = comp.to_vec();
                tmp.push(trans.clone());
                let (t2, rule2) = find_target(&tmp, rules, self.flags);
                if let Some(t2) = t2 {
                    if t2 != vowels[0].id {
                        let id2 = self.new_id();
                        return vec![
                            trans,
                            Transformation {
                                id: id2,
                                rule: rule2,
                                target: Some(t2),
                                is_upper_case,
                            },
                        ];
                    }
                }
            }
        }

        // Phím dấu không tìm được đích -> gỡ hiệu ứng cũ rồi gõ thẳng (ươ + w -> uow)
        let undo = self.gen_undo_transformations(comp, rules);
        if !undo.is_empty() {
            let mut out = undo;
            out.push(self.new_appending(lower_key, is_upper_case));
            return out;
        }
        Vec::new()
    }

    fn gen_undo_transformations(&mut self, comp: &[Transformation], rules: &[Rule]) -> Vec<Transformation> {
        let mut out: Vec<Transformation> = Vec::new();
        let s = flatten(comp, mode::VIETNAMESE | mode::TONE_LESS | mode::LOWER_CASE);
        for rule in rules {
            match rule.effect_type {
                EffectType::ToneTransformation => {
                    let target = if self.flags & flag::FREE_TONE_MARKING != 0 {
                        if has_valid_tone(comp, rule.effect) {
                            find_tone_target(comp, self.flags & flag::STD_TONE_STYLE != 0)
                        } else {
                            None
                        }
                    } else {
                        match find_last_appending(comp) {
                            Some(la) if is_vowel(la.rule.effect_on) => Some(la.id),
                            _ => None,
                        }
                    };
                    let target = match target {
                        Some(t) => t,
                        None => continue,
                    };
                    let id = self.new_id();
                    out.push(Transformation {
                        id,
                        rule: Rule {
                            key: '\0',
                            effect: 0,
                            effect_type: EffectType::ToneTransformation,
                            effect_on: '\0',
                            result: '\0',
                            appended_rules: Vec::new(),
                        },
                        target: Some(target),
                        is_upper_case: false,
                    });
                }
                EffectType::MarkTransformation => {
                    for i in (0..comp.len()).rev() {
                        if comp[i].rule.result != rule.effect_on {
                            continue;
                        }
                        let target = find_root_target(comp, &comp[i]);
                        let id = self.new_id();
                        let t = Transformation {
                            id,
                            rule: Rule {
                                key: '\0',
                                effect: 0,
                                effect_type: EffectType::MarkTransformation,
                                effect_on: '\0',
                                result: '\0',
                                appended_rules: Vec::new(),
                            },
                            target: Some(target),
                            is_upper_case: false,
                        };
                        let mut tmp = comp.to_vec();
                        tmp.push(t.clone());
                        if s == flatten(&tmp, mode::VIETNAMESE | mode::TONE_LESS | mode::LOWER_CASE) {
                            continue;
                        }
                        out.push(t);
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn remove_last_char(&mut self, refresh_tone: bool) {
        let last_appending = match find_last_appending(&self.composition) {
            Some(t) => t.clone(),
            None => return,
        };
        if !self.can_process_key(last_appending.rule.key) {
            self.composition.pop();
            return;
        }
        let (mut previous, last_comb) = extract_last_word(&self.composition, &self.input_method.keys);
        let mut new_comb: Vec<Transformation> = last_comb
            .into_iter()
            .filter(|t| t.target != Some(last_appending.id) && t.id != last_appending.id)
            .collect();
        if refresh_tone {
            let refreshed = self.refresh_last_tone_target(&mut new_comb);
            new_comb.extend(refreshed);
        }
        previous.append(&mut new_comb);
        self.composition = previous;
    }

    pub fn restore_last_word(&mut self, to_vietnamese: bool) {
        let (mut previous, last_comb) = extract_last_word(&self.composition, &self.input_method.keys);
        if last_comb.is_empty() {
            return;
        }
        if !to_vietnamese {
            let mut broken = Vec::new();
            for t in &last_comb {
                if t.rule.key == '\0' {
                    continue;
                }
                broken.push(self.new_appending(t.rule.key, t.is_upper_case));
            }
            previous.append(&mut broken);
            self.composition = previous;
        } else {
            let mut new_comp: Vec<Transformation> = Vec::new();
            for t in &last_comb {
                new_comp = self.new_composition(new_comp, t.rule.key, t.is_upper_case);
            }
            previous.append(&mut new_comp);
            self.composition = previous;
        }
    }
}

fn find_mark_target(comp: &[Transformation], rules: &[Rule]) -> (Option<Id>, Rule) {
    let s = flatten(comp, mode::VIETNAMESE);
    for i in (0..comp.len()).rev() {
        for rule in rules {
            if rule.effect_type != EffectType::MarkTransformation {
                continue;
            }
            if comp[i].rule.result == rule.effect_on && rule.effect > 0 {
                let target = find_root_target(comp, &comp[i]);
                let probe = Transformation {
                    id: u32::MAX, // id tạm, chỉ dùng để thử trải chuỗi
                    rule: rule.clone(),
                    target: Some(target),
                    is_upper_case: false,
                };
                let mut tmp = comp.to_vec();
                tmp.push(probe);
                if s == flatten(&tmp, mode::VIETNAMESE) {
                    continue;
                }
                if is_valid(&tmp, false) {
                    return (Some(target), rule.clone());
                }
            }
        }
    }
    (None, Rule::appending('\0'))
}

fn find_target(comp: &[Transformation], rules: &[Rule], flags: u32) -> (Option<Id>, Rule) {
    let s = flatten(comp, mode::VIETNAMESE);
    for rule in rules {
        if rule.effect_type != EffectType::ToneTransformation {
            continue;
        }
        let mut target = if flags & flag::FREE_TONE_MARKING != 0 {
            if has_valid_tone(comp, rule.effect) {
                find_tone_target(comp, flags & flag::STD_TONE_STYLE != 0)
            } else {
                None
            }
        } else {
            match find_last_appending(comp) {
                Some(la) if is_vowel(la.rule.effect_on) => Some(la.id),
                _ => None,
            }
        };
        let probe = Transformation {
            id: u32::MAX,
            rule: rule.clone(),
            target,
            is_upper_case: false,
        };
        let mut tmp = comp.to_vec();
        tmp.push(probe);
        if s == flatten(&tmp, mode::VIETNAMESE) {
            continue;
        }
        // Gõ phím xoá dấu khi vốn chưa có dấu -> không nhắm vào đâu cả.
        // Bản Go dựa vào short-circuit để khỏi truy cập con trỏ rỗng; ở đây
        // viết tường minh.
        if rule.effect == tone::NONE && is_free(comp, target, EffectType::ToneTransformation) {
            let tone_of_target = target
                .and_then(|id| find_by_id(comp, id))
                .map(|t| find_tone_from_char(t.rule.result))
                .unwrap_or(tone::NONE);
            if tone_of_target == tone::NONE {
                target = None;
            }
        }
        return (target, rule.clone());
    }
    find_mark_target(comp, rules)
}

fn extract_last_word_with_punctuation(
    comp: &[Transformation],
) -> (Vec<Transformation>, Vec<Transformation>) {
    for i in (0..comp.len()).rev() {
        let canvas = get_canvas(&comp[i..], mode::ENGLISH);
        if canvas.is_empty() {
            continue;
        }
        if is_space(canvas[0]) {
            if i == comp.len() - 1 {
                return (comp.to_vec(), Vec::new());
            }
            return (comp[..i + 1].to_vec(), comp[i + 1..].to_vec());
        }
    }
    (Vec::new(), comp.to_vec())
}

fn extract_last_word(
    comp: &[Transformation],
    effect_keys: &[char],
) -> (Vec<Transformation>, Vec<Transformation>) {
    for i in (0..comp.len()).rev() {
        let canvas = get_canvas(
            &comp[i..],
            mode::VIETNAMESE | mode::LOWER_CASE | mode::TONE_LESS | mode::MARK_LESS,
        );
        if canvas.is_empty() {
            continue;
        }
        let c = canvas[0];
        if !is_alpha(c) && !effect_keys.contains(&c) {
            if i == comp.len() - 1 {
                return (comp.to_vec(), Vec::new());
            }
            return (comp[..i + 1].to_vec(), comp[i + 1..].to_vec());
        }
    }
    (Vec::new(), comp.to_vec())
}

fn extract_last_syllable(comp: &[Transformation]) -> (Vec<Transformation>, Vec<Transformation>) {
    let (mut previous, last) = extract_last_word(comp, &[]);
    let mut anchor = 0usize;
    for i in 0..last.len() {
        if !is_valid(&last[anchor..=i], false) {
            anchor = i;
        }
    }
    if anchor > 0 {
        previous.extend_from_slice(&last[..anchor]);
    }
    (previous, last[anchor..].to_vec())
}

// `is_word_break_symbol` dùng gián tiếp qua can_process_key; giữ import cho rõ.
#[allow(dead_code)]
fn _keep(_: fn(char) -> bool) {}
#[allow(dead_code)]
fn _keep2() {
    let _ = is_word_break_symbol;
    let _ = add_tone_to_char;
}

// Kiểm tra vần tiếng Việt: một tiếng có hợp lệ không (phụ âm đầu + vần + phụ
// âm cuối). Port sát `spelling.go`.
//
// Cách làm của bản gốc: mỗi nhóm phụ âm/nguyên âm là một HÀNG trong bảng, và
// hai ma trận cv/vc cho biết hàng nào ghép được với hàng nào. Giữ nguyên cả
// cách đánh số hàng vì hai ma trận kia tra theo chỉ số hàng.

use crate::utils::add_mark_to_toneless_char;

const FIRST_CONSONANT_SEQS: &[&str] = &[
    "b d đ g gh m n nh p ph r s t tr v z",
    "c h k kh qu th",
    "ch gi l ng ngh x",
    "đ l",
    "h",
];

const VOWEL_SEQS: &[&str] = &[
    "ê i ua uê uy y",
    "a iê oa uyê yê",
    "â ă e o oo ô ơ oe u ư uâ uô ươ",
    "oă",
    "uơ",
    "ai ao au âu ay ây eo êu ia iêu iu oai oao oay oeo oi ôi ơi ưa uây ui ưi uôi ươi ươu ưu uya uyu yêu",
    "ă",
    "i",
];

const LAST_CONSONANT_SEQS: &[&str] = &["ch nh", "c ng", "m n p t", "k", "c"];

const CV_MATRIX: &[&[usize]] = &[
    &[0, 1, 2, 5],
    &[0, 1, 2, 3, 4, 5],
    &[0, 1, 2, 3, 5],
    &[6],
    &[7],
];

const VC_MATRIX: &[&[usize]] = &[
    &[0, 2],
    &[0, 1, 2],
    &[1, 2],
    &[1, 2],
    &[],
    &[],
    &[3],
    &[4],
];

/// Tìm những hàng chứa `input`. `input_is_full` = phải khớp trọn vẹn chứ không
/// chỉ là phần đầu; `input_is_complete` = không chấp nhận chữ thiếu dấu phụ.
fn lookup(seq: &[&str], input: &str, input_is_full: bool, input_is_complete: bool) -> Vec<usize> {
    let input_chars: Vec<char> = input.chars().collect();
    let input_len = input_chars.len();
    let mut ret = Vec::new();

    for (index, row) in seq.iter().enumerate() {
        // thêm dấu cách cuối để ô cuối cùng cũng được xét, y như bản Go
        let rows: Vec<char> = row.chars().chain(std::iter::once(' ')).collect();
        let mut i = 0usize;
        for (j, ch) in rows.iter().enumerate() {
            if *ch != ' ' {
                continue;
            }
            let canvas = &rows[i..j];
            i = j + 1;
            if canvas.len() < input_len || (input_is_full && canvas.len() > input_len) {
                continue;
            }
            let mut is_match = true;
            for (k, ic) in input_chars.iter().enumerate() {
                let c = canvas[k];
                if *ic != c && !(!input_is_complete && add_mark_to_toneless_char(c, 0) == *ic) {
                    is_match = false;
                    break;
                }
            }
            if is_match {
                ret.push(index);
                break;
            }
        }
    }
    ret
}

pub fn is_valid_cvc(fc: &str, vo: &str, lc: &str, input_is_full_complete: bool) -> bool {
    let mut fc_indexes: Option<Vec<usize>> = None;
    let mut vo_indexes: Option<Vec<usize>> = None;
    let mut lc_indexes: Option<Vec<usize>> = None;

    if !fc.is_empty() {
        let r = lookup(
            FIRST_CONSONANT_SEQS,
            fc,
            input_is_full_complete || !vo.is_empty(),
            true,
        );
        if r.is_empty() {
            return false;
        }
        fc_indexes = Some(r);
    }
    if !vo.is_empty() {
        let r = lookup(
            VOWEL_SEQS,
            vo,
            input_is_full_complete || !lc.is_empty(),
            input_is_full_complete,
        );
        if r.is_empty() {
            return false;
        }
        vo_indexes = Some(r);
    }
    if !lc.is_empty() {
        let r = lookup(LAST_CONSONANT_SEQS, lc, input_is_full_complete, true);
        if r.is_empty() {
            return false;
        }
        lc_indexes = Some(r);
    }

    let vo_indexes = match vo_indexes {
        None => return fc_indexes.is_some(), // chỉ có phụ âm đầu
        Some(v) => v,
    };

    if let Some(fc_i) = &fc_indexes {
        let ret = is_valid_cv(fc_i, &vo_indexes);
        if !ret || lc_indexes.is_none() {
            return ret;
        }
    }
    match &lc_indexes {
        Some(lc_i) => is_valid_vc(&vo_indexes, lc_i),
        None => true,
    }
}

fn is_valid_cv(fc_indexes: &[usize], vo_indexes: &[usize]) -> bool {
    fc_indexes
        .iter()
        .any(|fc| CV_MATRIX[*fc].iter().any(|c| vo_indexes.contains(c)))
}

fn is_valid_vc(vo_indexes: &[usize], lc_indexes: &[usize]) -> bool {
    vo_indexes
        .iter()
        .any(|vo| VC_MATRIX[*vo].iter().any(|c| lc_indexes.contains(c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tieng_hop_le() {
        assert!(is_valid_cvc("t", "iê", "ng", true)); // tiếng
        // "người": chữ i thuộc VẦN ("ươi") chứ không phải phụ âm cuối —
        // tách sai thành ("ng","ươ","i") thì đúng ra phải bị coi là không hợp lệ.
        assert!(is_valid_cvc("ng", "ươi", "", true));
        assert!(!is_valid_cvc("ng", "ươ", "i", true));
        assert!(is_valid_cvc("", "a", "", true));
    }

    #[test]
    fn tieng_khong_hop_le() {
        assert!(!is_valid_cvc("z", "z", "", true));
        assert!(!is_valid_cvc("t", "iê", "z", true));
    }
}

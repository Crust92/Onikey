//! Gõ tắt (macro): đọc chung tệp `~/.config/onikey/onikey.macro.text` với bản
//! Go. Mỗi dòng `khoá:văn bản`, dòng bắt đầu `#`/`;` là chú thích.
//!
//! Tự viết hoa theo cách gõ khoá (khi bật cờ): `vn` → "Việt Nam",
//! `VN` → "VIỆT NAM", `Vn` → "Việt Nam" giữ nguyên bản mẫu — theo đúng
//! determineMacroCase của bản Go.

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct MacroTable {
    table: HashMap<String, String>,
    auto_capitalize: bool,
}

pub fn macro_path() -> PathBuf {
    crate::config::config_path()
        .parent()
        .map(|d| d.join("onikey.macro.text"))
        .unwrap_or_else(|| PathBuf::from("onikey.macro.text"))
}

impl MacroTable {
    pub fn load(auto_capitalize: bool) -> MacroTable {
        let mut t = MacroTable {
            table: HashMap::new(),
            auto_capitalize,
        };
        let Ok(data) = std::fs::read_to_string(macro_path()) else {
            return t;
        };
        for line in data.lines() {
            let s = line.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            let Some((key, text)) = s.split_once(':') else {
                continue;
            };
            let (key, text) = (key.trim(), text.trim());
            if key.is_empty() || text.is_empty() {
                continue;
            }
            let key = if auto_capitalize {
                key.to_lowercase()
            } else {
                key.to_string()
            };
            t.table.insert(key, text.to_string());
        }
        t
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Tra khoá theo đúng chuỗi người dùng vừa gõ; trả về bản mở rộng đã chỉnh
    /// hoa thường theo cách gõ khoá.
    pub fn expand(&self, typed: &str) -> Option<String> {
        if typed.is_empty() {
            return None;
        }
        let lookup = if self.auto_capitalize {
            typed.to_lowercase()
        } else {
            typed.to_string()
        };
        let text = self.table.get(&lookup)?;
        if !self.auto_capitalize {
            return Some(text.clone());
        }
        Some(apply_case(typed, text))
    }
}

/// Nhìn cách gõ khoá để chỉnh hoa thường của bản mở rộng — port
/// determineMacroCase: chữ đầu thường → giữ nguyên; TẤT CẢ hoa → HOA HẾT;
/// chỉ chữ đầu hoa → viết hoa chữ đầu.
fn apply_case(typed: &str, text: &str) -> String {
    let chars: Vec<char> = typed.chars().collect();
    let first_upper = chars.first().map(|c| c.is_uppercase()).unwrap_or(false);
    if !first_upper {
        return text.to_string();
    }
    let all_upper = chars.iter().all(|c| !c.is_alphabetic() || c.is_uppercase());
    if all_upper && chars.len() > 1 {
        return text.to_uppercase();
    }
    // Chữ đầu hoa: viết hoa ký tự đầu của bản mở rộng
    let mut out = String::with_capacity(text.len());
    let mut it = text.chars();
    if let Some(c) = it.next() {
        out.extend(c.to_uppercase());
    }
    out.push_str(it.as_str());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bang() -> MacroTable {
        let mut t = MacroTable {
            table: HashMap::new(),
            auto_capitalize: true,
        };
        t.table.insert("vn".into(), "Việt Nam".into());
        t.table.insert("sđt".into(), "số điện thoại".into());
        t
    }

    #[test]
    fn mo_rong_va_chinh_hoa() {
        let t = bang();
        assert_eq!(t.expand("vn").unwrap(), "Việt Nam");
        assert_eq!(t.expand("VN").unwrap(), "VIỆT NAM");
        assert_eq!(t.expand("Sđt").unwrap(), "Số điện thoại");
        assert!(t.expand("xyz").is_none());
        assert!(t.expand("").is_none());
    }
}

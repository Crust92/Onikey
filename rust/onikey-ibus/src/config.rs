//! Đọc tệp cấu hình dùng chung với bản Go: `~/.config/onikey/onikey.config.json`.
//!
//! Bản Rust CỐ Ý đọc đúng tệp đó thay vì tự bịa định dạng riêng — hai engine
//! cùng cài, người dùng đổi qua lại bằng chọn nguồn nhập, nên kiểu gõ và các
//! tuỳ chọn phải giống nhau ở cả hai. (Khi nào bỏ hẳn bản Go mới chuyển TOML,
//! kèm bước chuyển đổi — xem ROADMAP.)

use std::path::PathBuf;

/// Các bit trong `IBflags` mà bản Rust hiện dùng tới.
pub mod ibflag {
    /// Bật gõ tắt (macro).
    pub const MACRO_ENABLED: u32 = 1 << 1;
    /// Tự chỉnh hoa thường của bản gõ tắt theo cách gõ khoá.
    pub const AUTO_CAPITALIZE_MACRO: u32 = 1 << 15;
    /// Tự khôi phục chuỗi phím gốc khi từ không phải tiếng Việt
    /// (gõ "expression" không bị bẻ dấu).
    pub const AUTO_NON_VN_RESTORE: u32 = 1 << 5;
    /// Cho phép "dd" thành "đ" cả trong từ không có nguyên âm tiếng Việt
    /// (viết tắt kiểu "dd" rất hay dùng).
    pub const DD_FREE_STYLE: u32 = 1 << 6;
    /// Gõ không gạch chân ở mọi ô nhập (nếu ứng dụng hỗ trợ).
    pub const NO_UNDERLINE: u32 = 1 << 7;
}

#[derive(Debug, Clone)]
pub struct Config {
    pub input_method: String,
    pub output_charset: String,
    /// Cờ của lõi (bỏ dấu tự do, kiểu dấu chuẩn, tự sửa lỗi).
    pub flags: u32,
    /// Cờ tầng IBus (xem `ibflag`).
    pub ib_flags: u32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            input_method: "Telex".into(),
            output_charset: "Unicode".into(),
            flags: onikey_core::flag::STD_FLAGS,
            ib_flags: ibflag::NO_UNDERLINE,
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into())).join(".config"),
    };
    base.join("onikey/onikey.config.json")
}

/// Đọc cấu hình; thiếu tệp hoặc hỏng thì dùng mặc định — KHÔNG được để lỗi cấu
/// hình làm chết engine, vì hậu quả là mất gõ toàn máy.
pub fn load() -> Config {
    let mut cfg = Config::default();
    let path = config_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return cfg,
    };
    if let Some(v) = json_string(&data, "InputMethod") {
        cfg.input_method = v;
    }
    if let Some(v) = json_string(&data, "OutputCharset") {
        cfg.output_charset = v;
    }
    if let Some(v) = json_number(&data, "Flags") {
        cfg.flags = v as u32;
    }
    if let Some(v) = json_number(&data, "IBflags") {
        cfg.ib_flags = v as u32;
    }
    cfg
}

// Bộ đọc JSON tối giản cho đúng bốn khoá phẳng ở trên. Không kéo cả thư viện
// JSON vào chỉ để đọc bốn giá trị; tệp này do chính engine ghi ra nên định
// dạng biết trước.
fn find_value<'a>(data: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let i = data.find(&pat)? + pat.len();
    let rest = &data[i..];
    let j = rest.find(':')? + 1;
    Some(rest[j..].trim_start())
}

fn json_string(data: &str, key: &str) -> Option<String> {
    let v = find_value(data, key)?;
    let v = v.strip_prefix('"')?;
    let end = v.find('"')?;
    Some(v[..end].to_string())
}

fn json_number(data: &str, key: &str) -> Option<u64> {
    let v = find_value(data, key)?;
    let end = v
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(v.len());
    v[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAU: &str = r#"{
  "InputMethod": "Telex 2",
  "InputMethodDefinitions": { "Telex": { "a": "A_Â" } },
  "OutputCharset": "Unicode",
  "Flags": 7,
  "IBflags": 1081840,
  "DefaultInputMode": 1
}"#;

    #[test]
    fn doc_dung_cac_khoa_can_dung() {
        assert_eq!(json_string(MAU, "InputMethod").unwrap(), "Telex 2");
        assert_eq!(json_string(MAU, "OutputCharset").unwrap(), "Unicode");
        assert_eq!(json_number(MAU, "Flags").unwrap(), 7);
        assert_eq!(json_number(MAU, "IBflags").unwrap(), 1081840);
        // 1081840 có bit 7 -> đang bật gõ không gạch chân
        assert_ne!(1081840u32 & ibflag::NO_UNDERLINE, 0);
    }

    #[test]
    fn thieu_tep_thi_dung_mac_dinh_chu_khong_chet() {
        std::env::set_var("HOME", "/khong/co/that");
        std::env::remove_var("XDG_CONFIG_HOME");
        let c = load();
        assert_eq!(c.input_method, "Telex");
    }
}

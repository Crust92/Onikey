//! Đọc/ghi tệp cấu hình người dùng Onikey — crate dùng chung cho mọi adapter
//! (IBus, Fcitx5...). Trước nằm trong onikey-ibus; tách ra khi làm addon Fcitx5: `~/.config/onikey/onikey.config.json`.
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
    /// Bit "ẩn gạch chân" cũ. KHÔNG còn quyết định chế độ gõ — chế độ lấy từ
    /// DefaultInputMode; giữ tên bit để khỏi ai dùng lại nhầm.
    #[allow(dead_code)]
    pub const NO_UNDERLINE: u32 = 1 << 7;
    /// Ở chế độ Pre-edit: riêng Ô ĐỊA CHỈ TRÌNH DUYỆT (ứng dụng khai
    /// purpose=URL) gõ không gạch chân — pre-edit phá danh sách gợi ý của
    /// thanh địa chỉ. Chỉ có nghĩa khi DefaultInputMode = 1.
    pub const URL_NO_UNDERLINE: u32 = 1 << 21;
}

#[derive(Debug, Clone)]
pub struct Config {
    pub input_method: String,
    pub output_charset: String,
    /// Cờ của lõi (bỏ dấu tự do, kiểu dấu chuẩn, tự sửa lỗi).
    pub flags: u32,
    /// Cờ tầng IBus (xem `ibflag`).
    pub ib_flags: u32,
    /// 5 phím tắt, mỗi cái một cặp (mask, keyval); keyval 0 = tắt. Thứ tự:
    /// chuyển chế độ gõ, khôi phục phím gốc, chuyển Anh–Việt, emoji, hexa.
    pub shortcuts: [u32; 10],
    /// Chế độ gõ do người dùng chọn trong hộp thoại — NGUỒN CHÂN LÝ DUY NHẤT
    /// cho chuyện gạch chân: 1 = Pre-edit (có gạch chân), còn lại = các chế độ
    /// không gạch chân. Từng có thêm một ô tick "không gạch chân" đè lên đây
    /// và hậu quả là người dùng chọn chế độ 1 mà vẫn không thấy gạch chân.
    pub default_input_mode: u32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            input_method: "Telex 2".into(),
            output_charset: "Unicode".into(),
            flags: onikey_core::flag::STD_FLAGS,
            // Máy CHƯA TỪNG cấu hình phải có hành vi chuẩn: tự khôi phục tiếng
            // Anh + dd viết tắt + tự viết hoa macro. ib_flags=0 từng làm addon
            // Fcitx5 trên máy mới gõ "expression" thành "ẽpresion".
            //
            // URL_NO_UNDERLINE bật sẵn: mặc định máy mới là Pre-edit (tin cậy
            // nhất khi máy lag) nhưng RIÊNG ô địa chỉ trình duyệt bỏ gạch chân,
            // vì pre-edit ở đó phá danh sách gợi ý. Đây là cấu hình dùng hằng
            // ngày nên để người dùng phải tự đi bật là thừa một bước.
            ib_flags: ibflag::AUTO_NON_VN_RESTORE
                | ibflag::DD_FREE_STYLE
                | ibflag::AUTO_CAPITALIZE_MACRO
                | ibflag::URL_NO_UNDERLINE,
            shortcuts: [1, 126, 0, 0, 0, 0, 0, 0, 5, 117],
            default_input_mode: 1,
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
    if let Some(v) = json_number(&data, "DefaultInputMode") {
        cfg.default_input_mode = v as u32;
    }
    if let Some(v) = json_u32_array(&data, "Shortcuts") {
        if v.len() == 10 {
            cfg.shortcuts.copy_from_slice(&v);
        }
    }
    cfg
}

/// Chỉ số phím tắt trong `Config::shortcuts` (nhân 2 ra vị trí cặp).
pub mod shortcut {
    #[allow(dead_code)]
    pub const INPUT_MODE_SWITCH: usize = 0;
    pub const RESTORE_KEY_STROKES: usize = 1;
    pub const VI_EN_SWITCH: usize = 2;
    #[allow(dead_code)]
    pub const EMOJI: usize = 3;
    #[allow(dead_code)]
    pub const HEXADECIMAL: usize = 4;
}

impl Config {
    /// Phím tắt thứ `idx` có khớp (mask, keyval) này không? keyval so ở dạng
    /// chữ thường, mask chỉ so các phím bổ trợ chính (Ctrl/Shift/Alt/Super).
    pub fn shortcut_matches(&self, idx: usize, keyval: u32, state: u32) -> bool {
        let mask = self.shortcuts[idx * 2];
        let key = self.shortcuts[idx * 2 + 1];
        if key == 0 {
            return false;
        }
        const RELEVANT: u32 = 1 | (1 << 2) | (1 << 3) | (1 << 26); // Shift|Ctrl|Alt|Super
        let lower = char::from_u32(keyval)
            .map(|c| c.to_lowercase().next().unwrap_or(c) as u32)
            .unwrap_or(keyval);
        state & RELEVANT == mask & RELEVANT && lower == key
    }
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

fn json_u32_array(data: &str, key: &str) -> Option<Vec<u32>> {
    let v = find_value(data, key)?;
    let v = v.strip_prefix('[')?;
    let end = v.find(']')?;
    let mut out = Vec::new();
    for part in v[..end].split(',') {
        out.push(part.trim().parse().ok()?);
    }
    Some(out)
}

fn json_number(data: &str, key: &str) -> Option<u64> {
    let v = find_value(data, key)?;
    let end = v
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(v.len());
    v[..end].parse().ok()
}

/// Ghi đè MỘT khoá chuỗi trong tệp cấu hình, giữ nguyên mọi phần khác (kể cả
/// InputMethodDefinitions mà bản Rust không hiểu). Sửa chuỗi tại chỗ chứ không
/// parse-rồi-ghi-lại: tệp này bản Go cũng đọc, phá cấu trúc là hỏng cả hai.
pub fn save_string(key: &str, value: &str) -> std::io::Result<()> {
    rewrite_value(key, &format!("\"{}\"", value.replace('"', "")))
}

pub fn save_number(key: &str, value: u32) -> std::io::Result<()> {
    rewrite_value(key, &value.to_string())
}

fn rewrite_value(key: &str, new_raw: &str) -> std::io::Result<()> {
    rewrite_value_at(&config_path(), key, new_raw)
}

/// Tạo tệp cấu hình RỖNG nếu chưa có. Bản Rust chỉ SỬA GIÁ TRỊ TẠI CHỖ, nên
/// không có tệp là không lưu được gì: người dùng đổi tuỳ chọn, dùng ngon trong
/// phiên, khởi động lại là mất sạch. Máy mới cài — chưa từng mở hộp thoại cấu
/// hình nên chưa ai tạo tệp — rơi đúng vào cảnh đó.
///
/// CỐ Ý ghi `{}` chứ không đổ sẵn mặc định của bản Rust vào: tệp này bản Go
/// đọc chung mà hai bên có mặc định KHÁC nhau (IBflags của Go còn kèm kiểm tra
/// chính tả, workaround WPS...). Đổ mặc định Rust vào tức là lẳng lặng đổi
/// hành vi bản Go. Khoá nào chưa có thì mỗi engine tự lấy mặc định của mình —
/// cả hai đều khởi tạo từ mặc định rồi mới đè giá trị đọc được.
fn ensure_file_at(path: &std::path::Path) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, "{\n}\n")
}

/// Chèn khoá chưa có vào đầu object, giữ nguyên phần còn lại của tệp. Tệp do
/// bản cũ ghi có thể thiếu khoá mới; báo lỗi ở đây đồng nghĩa tuỳ chọn mới
/// KHÔNG BAO GIỜ lưu được.
fn insert_value_at(
    path: &std::path::Path,
    data: &str,
    key: &str,
    new_raw: &str,
) -> std::io::Result<()> {
    let Some(open) = data.find('{') else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "cấu hình không phải JSON object",
        ));
    };
    let sep = if data[open + 1..].trim_start().starts_with('}') {
        ""
    } else {
        ","
    };
    let mut out = String::with_capacity(data.len() + 64);
    out.push_str(&data[..=open]);
    out.push_str(&format!("\n  \"{key}\": {new_raw}{sep}"));
    out.push_str(&data[open + 1..]);
    std::fs::write(path, out)
}

fn rewrite_value_at(path: &std::path::Path, key: &str, new_raw: &str) -> std::io::Result<()> {
    ensure_file_at(path)?;
    let data = std::fs::read_to_string(path)?;
    let pat = format!("\"{key}\"");
    let Some(i) = data.find(&pat) else {
        return insert_value_at(path, &data, key, new_raw);
    };
    let after_key = i + pat.len();
    let Some(colon) = data[after_key..].find(':') else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "thiếu dấu :"));
    };
    let vstart = after_key + colon + 1;
    let rest = &data[vstart..];
    let skip = rest.len() - rest.trim_start().len();
    let vstart = vstart + skip;
    let rest = &data[vstart..];
    // giá trị là chuỗi "..." hoặc số
    let vlen = if rest.starts_with('"') {
        rest[1..].find('"').map(|e| e + 2)
    } else {
        Some(rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len()))
    }
    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "giá trị hỏng"))?;
    let mut out = String::with_capacity(data.len() + 16);
    out.push_str(&data[..vstart]);
    out.push_str(new_raw);
    out.push_str(&data[vstart + vlen..]);
    std::fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAU: &str = r#"{
  "InputMethod": "Telex 2",
  "Shortcuts": [1, 126, 0, 0, 4, 32, 0, 0, 5, 117],
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
        assert_eq!(json_number(MAU, "DefaultInputMode").unwrap(), 1);
        assert_eq!(
            json_u32_array(MAU, "Shortcuts").unwrap(),
            vec![1, 126, 0, 0, 4, 32, 0, 0, 5, 117]
        );
        let mut c = Config::default();
        c.shortcuts = [1, 126, 0, 0, 4, 32, 0, 0, 5, 117];
        // Ctrl+Space bật/tắt tiếng Việt (mask 4 = Ctrl, key 32 = space)
        assert!(c.shortcut_matches(shortcut::VI_EN_SWITCH, 32, 1 << 2));
        assert!(!c.shortcut_matches(shortcut::VI_EN_SWITCH, 32, 0));
        // keyval 0 = tắt, không bao giờ khớp
        assert!(!c.shortcut_matches(shortcut::RESTORE_KEY_STROKES, 0, 0));
    }

    #[test]
    fn thieu_tep_thi_dung_mac_dinh_chu_khong_chet() {
        std::env::set_var("HOME", "/khong/co/that");
        std::env::remove_var("XDG_CONFIG_HOME");
        let c = load();
        assert_eq!(c.input_method, "Telex 2");
    }

    /// Máy mới cài chưa có tệp cấu hình: đổi tuỳ chọn phải TẠO tệp rồi lưu
    /// được. Trước đây rewrite_value trả lỗi NotFound ở bước đọc tệp, nên tuỳ
    /// chọn chỉ sống trong phiên — khởi động lại là mất.
    #[test]
    fn thieu_tep_thi_van_luu_duoc_tuy_chon() {
        let dir = std::env::temp_dir().join("onikey-test-luu-khi-thieu-tep");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("onikey/onikey.config.json");

        rewrite_value_at(&path, "DefaultInputMode", "2").unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert_eq!(json_number(&data, "DefaultInputMode").unwrap(), 2);
        // Chỉ khoá vừa đổi được ghi: các khoá khác vắng mặt để mỗi engine giữ
        // mặc định của mình (Go và Rust không cùng mặc định IBflags).
        assert!(json_number(&data, "IBflags").is_none());
        assert!(json_string(&data, "InputMethod").is_none());
        // đọc lại qua load() thì khoá vắng vẫn ra mặc định Rust
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Tệp do bản cũ ghi thiếu khoá mới thì chèn thêm, không bỏ qua im lặng.
    #[test]
    fn thieu_khoa_thi_chen_them() {
        let dir = std::env::temp_dir().join("onikey-test-chen-khoa-thieu");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("onikey.config.json");
        std::fs::write(&path, "{\n  \"InputMethod\": \"Telex 2\"\n}\n").unwrap();

        rewrite_value_at(&path, "DefaultInputMode", "2").unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert_eq!(json_number(&data, "DefaultInputMode").unwrap(), 2);
        assert_eq!(json_string(&data, "InputMethod").unwrap(), "Telex 2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Menu thuộc tính của engine trên thanh trên GNOME (biểu tượng `vi`).
//!
//! Dựng theo đúng bố cục bản Go: Bảng mã › / Kiểu gõ › / Gõ tắt › / Chính tả ›
//! / Phím tắt / Cấu hình khác — nhưng CHỈ những mục bản Rust làm được thật.
//! Radio phản ánh cấu hình hiện tại; bấm là ghi vào tệp cấu hình chung.
//!
//! Khuôn dây (khớp goibus, có test khoá):
//!   IBusProperty = (sa{sv}suvsvbbuvv)
//!   IBusPropList = (sa{sv}av)

use std::collections::HashMap;

use zvariant::{OwnedValue, Type, Value};

use crate::config::{ibflag, Config};
use crate::ibus_text::IBusText;

pub const PROP_TYPE_NORMAL: u32 = 0;
pub const PROP_TYPE_TOGGLE: u32 = 1;
pub const PROP_TYPE_RADIO: u32 = 2;
pub const PROP_TYPE_MENU: u32 = 3;
pub const PROP_TYPE_SEPARATOR: u32 = 4;

pub const STATE_UNCHECKED: u32 = 0;
pub const STATE_CHECKED: u32 = 1;

/// Khoá các mục menu — `property_activate` nhận lại đúng chuỗi này.
pub const KEY_ABOUT: &str = "about";
pub const KEY_CONFIGURATION: &str = "configuration";
pub const KEY_MACRO_TABLE: &str = "open_macro_table";
pub const KEY_MACRO_ENABLED: &str = "macro_enabled";
pub const KEY_AUTO_CAPITALIZE: &str = "auto_capitalize_macro";
pub const KEY_NON_VN_RESTORE: &str = "auto_non_vn_restore";
pub const KEY_URL_NO_UNDERLINE: &str = "url_no_underline";
/// Công tắc chế độ gõ: bật = không gạch chân (mode 2), tắt = Pre-edit (mode 1).
/// Dùng TOGGLE chứ không RADIO vì GNOME giữ menu MỞ khi bấm switch — người
/// dùng chỉnh nhiều cài đặt liền nhau không phải mở lại menu.
pub const KEY_MODE_NO_UNDERLINE: &str = "mode_no_underline";
/// Tiền tố cho radio: "InputMethod::Telex 2", "OutputCharset::TCVN3 (ABC)".
pub const PREFIX_INPUT_METHOD: &str = "InputMethod::";
pub const PREFIX_CHARSET: &str = "OutputCharset::";

#[derive(Debug, Clone, Type, Value, OwnedValue)]
pub struct IBusProperty {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub key: String,
    pub prop_type: u32,
    pub label: OwnedValue,
    pub icon: String,
    pub tooltip: OwnedValue,
    pub sensitive: bool,
    pub visible: bool,
    pub state: u32,
    pub sub_props: OwnedValue,
    pub symbol: OwnedValue,
}

#[derive(Debug, Clone, Type, Value, OwnedValue)]
pub struct IBusPropList {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub properties: Vec<OwnedValue>,
}

fn text_value(s: &str) -> OwnedValue {
    OwnedValue::try_from(Value::from(IBusText::new(s))).expect("dựng IBusText")
}

fn prop_list(props: Vec<IBusProperty>) -> IBusPropList {
    IBusPropList {
        name: "IBusPropList".into(),
        attachments: HashMap::new(),
        properties: props
            .into_iter()
            .map(|p| OwnedValue::try_from(Value::from(p)).expect("property"))
            .collect(),
    }
}

fn prop(key: &str, prop_type: u32, label: &str, state: u32, sub: Option<IBusPropList>) -> IBusProperty {
    IBusProperty {
        name: "IBusProperty".into(),
        attachments: HashMap::new(),
        key: key.into(),
        prop_type,
        label: text_value(label),
        icon: String::new(),
        tooltip: text_value(label),
        sensitive: true,
        visible: true,
        state,
        sub_props: OwnedValue::try_from(Value::from(sub.unwrap_or_else(|| prop_list(Vec::new()))))
            .expect("subprops"),
        symbol: text_value(""),
    }
}

fn separator() -> IBusProperty {
    prop("-", PROP_TYPE_SEPARATOR, "", STATE_UNCHECKED, None)
}

fn checked(b: bool) -> u32 {
    if b {
        STATE_CHECKED
    } else {
        STATE_UNCHECKED
    }
}

/// Nút gạt ô địa chỉ — tách riêng để engine gửi UpdateProperty cập nhật
/// TẠI CHỖ khi menu đang mở (GNOME chỉ vẽ lại mục lẻ qua tín hiệu này).
pub fn url_no_underline_prop(cfg: &Config) -> IBusProperty {
    let mut p = prop(
        KEY_URL_NO_UNDERLINE,
        PROP_TYPE_TOGGLE,
        "Bỏ gạch chân trình duyệt",
        checked(cfg.ib_flags & ibflag::URL_NO_UNDERLINE != 0),
        None,
    );
    p.sensitive = cfg.default_input_mode == 1;
    p
}

/// Menu đầy đủ, dựng theo cấu hình hiện tại.
pub fn onikey_prop_list(cfg: &Config) -> IBusPropList {
    // Kiểu gõ › — radio theo danh sách kiểu gõ của lõi
    let im_items: Vec<IBusProperty> = onikey_core::rules::INPUT_METHOD_DEFINITIONS
        .iter()
        .map(|(name, _)| {
            prop(
                &format!("{PREFIX_INPUT_METHOD}{name}"),
                PROP_TYPE_RADIO,
                name,
                checked(*name == cfg.input_method),
                None,
            )
        })
        .collect();

    // Bảng mã › — radio theo danh sách bảng mã của lõi
    let cs_items: Vec<IBusProperty> = onikey_core::charsets::charset_names()
        .into_iter()
        .map(|name| {
            prop(
                &format!("{PREFIX_CHARSET}{name}"),
                PROP_TYPE_RADIO,
                name,
                checked(name == cfg.output_charset),
                None,
            )
        })
        .collect();

    // Gõ tắt › — hai công tắc + mở bảng gõ tắt
    let macro_items = vec![
        prop(
            KEY_MACRO_ENABLED,
            PROP_TYPE_TOGGLE,
            "Bật gõ tắt",
            checked(cfg.ib_flags & ibflag::MACRO_ENABLED != 0),
            None,
        ),
        prop(
            KEY_AUTO_CAPITALIZE,
            PROP_TYPE_TOGGLE,
            "Tự động viết hoa",
            checked(cfg.ib_flags & ibflag::AUTO_CAPITALIZE_MACRO != 0),
            None,
        ),
        separator(),
        prop(
            KEY_MACRO_TABLE,
            PROP_TYPE_NORMAL,
            "Sửa bảng gõ tắt",
            STATE_UNCHECKED,
            None,
        ),
    ];

    // Chính tả › — chỉ mục bản Rust thật sự dùng
    let spell_items = vec![prop(
        KEY_NON_VN_RESTORE,
        PROP_TYPE_TOGGLE,
        "Khôi phục từ ngoại ngữ",
        checked(cfg.ib_flags & ibflag::AUTO_NON_VN_RESTORE != 0),
        None,
    )];

    // Cài đặt khác › — chế độ gõ (chuyển từ hộp thoại ra đây) + nút gạt ô địa chỉ
    let mut other_items = vec![prop(
        KEY_MODE_NO_UNDERLINE,
        PROP_TYPE_TOGGLE,
        "Bỏ gạch chân",
        checked(cfg.default_input_mode != 1),
        None,
    )];
    // Nút gạt ô địa chỉ LUÔN CÓ MẶT, chỉ MỜ ĐI khi ở chế độ không gạch
    // chân (mọi ô đã không gạch chân sẵn). Không được thêm/bớt mục theo
    // trạng thái: GNOME đóng menu khi re-register làm menu NGẮN đi —
    // cấu trúc bất biến thì menu ở lại, đúng ý người dùng.
    other_items.push(separator());
    other_items.push(url_no_underline_prop(cfg));

    prop_list(vec![
        prop(
            KEY_ABOUT,
            PROP_TYPE_NORMAL,
            &format!("Onikey {}", env!("CARGO_PKG_VERSION")),
            STATE_UNCHECKED,
            None,
        ),
        separator(),
        prop("-", PROP_TYPE_MENU, "Bảng mã", STATE_UNCHECKED, Some(prop_list(cs_items))),
        prop("-", PROP_TYPE_MENU, "Kiểu gõ", STATE_UNCHECKED, Some(prop_list(im_items))),
        prop("-", PROP_TYPE_MENU, "Gõ tắt", STATE_UNCHECKED, Some(prop_list(macro_items))),
        prop(
            "-",
            PROP_TYPE_MENU,
            "Kiểm tra chính tả",
            STATE_UNCHECKED,
            Some(prop_list(spell_items)),
        ),
        prop(
            "-",
            PROP_TYPE_MENU,
            "Cài đặt khác",
            STATE_UNCHECKED,
            Some(prop_list(other_items)),
        ),
        separator(),
        prop(
            KEY_CONFIGURATION,
            PROP_TYPE_NORMAL,
            "Hộp thoại cấu hình (phím tắt, gõ tắt…)",
            STATE_UNCHECKED,
            None,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvariant::Type;

    #[test]
    fn chu_ky_kieu_khop_goibus() {
        assert_eq!(IBusProperty::SIGNATURE.to_string(), "(sa{sv}suvsvbbuvv)");
        assert_eq!(IBusPropList::SIGNATURE.to_string(), "(sa{sv}av)");
    }

    #[test]
    fn menu_theo_cau_hinh() {
        let mut cfg = Config::default();
        cfg.input_method = "Telex 2".into();
        let l = onikey_prop_list(&cfg);
        // about + sep + 5 menu + sep + hộp thoại = 9 mục
        assert_eq!(l.properties.len(), 9);
    }
}

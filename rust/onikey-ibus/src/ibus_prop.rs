//! Menu thuộc tính của engine trên thanh trên GNOME (biểu tượng `vi`).
//!
//! GNOME vẽ menu này từ danh sách property mà engine đăng ký qua tín hiệu
//! `RegisterProperties`. Không đăng ký thì bấm vào biểu tượng chỉ thấy mỗi
//! danh sách nguồn nhập — đúng cái Hoang báo thiếu ở bản Rust.
//!
//! Khuôn dây (khớp goibus, và có test khoá lại):
//!   IBusProperty = (sa{sv}suvsvbbuvv)
//!   IBusPropList = (sa{sv}av)

use std::collections::HashMap;

use zvariant::{OwnedValue, Type, Value};

use crate::ibus_text::IBusText;

/// Kiểu property của IBus.
pub const PROP_TYPE_NORMAL: u32 = 0;
#[allow(dead_code)]
pub const PROP_TYPE_SEPARATOR: u32 = 4;

/// Khoá của các mục menu — phải khớp chuỗi mà `property_activate` nhận lại.
pub const KEY_CONFIGURATION: &str = "configuration";
pub const KEY_ABOUT: &str = "about";

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

fn empty_prop_list() -> OwnedValue {
    OwnedValue::try_from(Value::from(IBusPropList {
        name: "IBusPropList".into(),
        attachments: HashMap::new(),
        properties: Vec::new(),
    }))
    .expect("dựng IBusPropList rỗng")
}

fn property(key: &str, label: &str, tooltip: &str) -> IBusProperty {
    IBusProperty {
        name: "IBusProperty".into(),
        attachments: HashMap::new(),
        key: key.into(),
        prop_type: PROP_TYPE_NORMAL,
        label: text_value(label),
        icon: String::new(),
        tooltip: text_value(tooltip),
        sensitive: true,
        visible: true,
        state: 0,
        sub_props: empty_prop_list(),
        symbol: text_value(""),
    }
}

/// Menu của bản Rust: gọn — mở hộp thoại cấu hình (nơi chỉnh mọi thứ) và
/// trang giới thiệu. Các mục bật/tắt lẻ tẻ của bản Go sẽ cân nhắc sau;
/// menu dài mà nửa số mục chưa hoạt động thì tệ hơn menu ngắn chạy đúng.
pub fn onikey_prop_list() -> IBusPropList {
    IBusPropList {
        name: "IBusPropList".into(),
        attachments: HashMap::new(),
        properties: vec![
            OwnedValue::try_from(Value::from(property(
                KEY_CONFIGURATION,
                "Cấu hình bộ gõ (bản Rust)",
                "Mở hộp thoại cấu hình Onikey",
            )))
            .expect("mục cấu hình"),
            OwnedValue::try_from(Value::from(property(
                KEY_ABOUT,
                "Giới thiệu Onikey",
                "Trang dự án Onikey trên GitHub",
            )))
            .expect("mục giới thiệu"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvariant::Type;

    #[test]
    fn chu_ky_kieu_khop_goibus() {
        // Sai khuôn là GNOME lặng lẽ bỏ menu, không có lỗi nào để lần.
        assert_eq!(IBusProperty::SIGNATURE.to_string(), "(sa{sv}suvsvbbuvv)");
        assert_eq!(IBusPropList::SIGNATURE.to_string(), "(sa{sv}av)");
    }

    #[test]
    fn menu_co_muc_cau_hinh() {
        let l = onikey_prop_list();
        assert_eq!(l.properties.len(), 2);
    }
}

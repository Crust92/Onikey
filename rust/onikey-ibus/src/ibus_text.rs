//! Kiểu dữ liệu IBusText trên dây DBus.
//!
//! IBus tuần tự hoá đối tượng của nó theo khuôn `(sa{sv}...)`: tên lớp, bảng
//! đính kèm, rồi mới tới nội dung. Sai khuôn này thì chữ không hiện mà cũng
//! chẳng có lỗi nào rõ ràng — nên có test khoá chữ ký kiểu ở cuối tệp.

use std::collections::HashMap;

use zvariant::{OwnedValue, Type, Value};

/// Kiểu gạch chân của IBus.
pub const ATTR_TYPE_UNDERLINE: u32 = 1;
pub const ATTR_UNDERLINE_SINGLE: u32 = 1;

/// Chế độ pre-edit khi mất focus: 0 = xoá, 1 = commit.
pub const PREEDIT_CLEAR: u32 = 0;
pub const PREEDIT_COMMIT: u32 = 1;

#[derive(Debug, Clone, Type, Value, OwnedValue)]
pub struct IBusAttribute {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub attr_type: u32,
    pub value: u32,
    pub start_index: u32,
    pub end_index: u32,
}

#[derive(Debug, Clone, Type, Value, OwnedValue)]
pub struct IBusAttrList {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub attributes: Vec<OwnedValue>,
}

#[derive(Debug, Clone, Type, Value, OwnedValue)]
pub struct IBusText {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub text: String,
    pub attrs: OwnedValue,
}

impl IBusAttrList {
    pub fn empty() -> IBusAttrList {
        IBusAttrList {
            name: "IBusAttrList".into(),
            attachments: HashMap::new(),
            attributes: Vec::new(),
        }
    }
}

impl IBusText {
    pub fn new(text: &str) -> IBusText {
        IBusText {
            name: "IBusText".into(),
            attachments: HashMap::new(),
            text: text.to_string(),
            attrs: OwnedValue::try_from(Value::from(IBusAttrList::empty()))
                .expect("dựng IBusAttrList rỗng"),
        }
    }

    /// Gắn một thuộc tính (ví dụ gạch chân) lên toàn bộ chuỗi.
    pub fn with_attr(text: &str, attr_type: u32, value: u32, start: u32, end: u32) -> IBusText {
        let attr = IBusAttribute {
            name: "IBusAttribute".into(),
            attachments: HashMap::new(),
            attr_type,
            value,
            start_index: start,
            end_index: end,
        };
        let list = IBusAttrList {
            name: "IBusAttrList".into(),
            attachments: HashMap::new(),
            attributes: vec![OwnedValue::try_from(Value::from(attr)).expect("thuộc tính")],
        };
        IBusText {
            name: "IBusText".into(),
            attachments: HashMap::new(),
            text: text.to_string(),
            attrs: OwnedValue::try_from(Value::from(list)).expect("danh sách thuộc tính"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvariant::Type;

    #[test]
    fn chu_ky_kieu_dung_khuon_ibus() {
        // IBus đòi đúng khuôn này; lệch là chữ không hiện mà không báo lỗi.
        assert_eq!(IBusText::SIGNATURE.to_string(), "(sa{sv}sv)");
        assert_eq!(IBusAttrList::SIGNATURE.to_string(), "(sa{sv}av)");
        assert_eq!(IBusAttribute::SIGNATURE.to_string(), "(sa{sv}uuuu)");
    }

    #[test]
    fn dung_duoc_text_co_gach_chan() {
        let t = IBusText::with_attr("tiếng", ATTR_TYPE_UNDERLINE, ATTR_UNDERLINE_SINGLE, 0, 5);
        assert_eq!(t.text, "tiếng");
        assert_eq!(t.name, "IBusText");
    }
}

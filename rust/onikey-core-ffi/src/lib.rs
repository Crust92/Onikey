//! Vỏ C cho lõi Onikey.
//!
//! Đây là mặt cắt để các adapter gọi vào: addon C++ của Fcitx5, engine IBus,
//! XIM… Nguyên tắc:
//!
//!   - **Không giữ trạng thái toàn cục.** Mọi thứ nằm trong con trỏ engine mà
//!     bên gọi tự tạo và tự huỷ, nên nhiều ô nhập chạy song song vẫn được.
//!   - **Chuỗi trả về do Rust cấp phát**, bên gọi phải trả lại bằng
//!     `onikey_string_free`. Đừng gọi `free()` của libc.
//!   - Mọi hàm chịu được con trỏ NULL, vì lỗi ở tầng này là mất bộ gõ toàn máy.

use std::ffi::{c_char, c_uint, CStr, CString};

use onikey_core::{charsets, rules::parse_input_method, Engine};

/// Đối tượng engine mờ với bên C.
pub struct OnikeyEngine {
    inner: Engine,
}

fn to_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe fn str_from_ptr<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

/// Tạo engine cho một kiểu gõ ("Telex", "VNI"...). Tên lạ thì vẫn trả về engine
/// nhưng không có luật nào — gõ ra y như gõ tiếng Anh.
#[no_mangle]
pub extern "C" fn onikey_engine_new(input_method: *const c_char, flags: c_uint) -> *mut OnikeyEngine {
    let name = unsafe { str_from_ptr(input_method) }.unwrap_or("Telex");
    let e = Engine::new(parse_input_method(name), flags as u32);
    Box::into_raw(Box::new(OnikeyEngine { inner: e }))
}

#[no_mangle]
pub extern "C" fn onikey_engine_free(engine: *mut OnikeyEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(engine));
    }
}

#[no_mangle]
pub extern "C" fn onikey_engine_reset(engine: *mut OnikeyEngine) {
    if let Some(e) = unsafe { engine.as_mut() } {
        e.inner.reset();
    }
}

/// Nạp một phím. `key` là mã Unicode (không phải keycode bàn phím).
#[no_mangle]
pub extern "C" fn onikey_engine_process_key(engine: *mut OnikeyEngine, key: u32, mode: c_uint) {
    if let (Some(e), Some(c)) = (unsafe { engine.as_mut() }, char::from_u32(key)) {
        e.inner.process_key(c, mode as u32);
    }
}

/// Lấy chuỗi đang gõ. Bên gọi PHẢI trả lại bằng `onikey_string_free`.
#[no_mangle]
pub extern "C" fn onikey_engine_get_string(engine: *const OnikeyEngine, mode: c_uint) -> *mut c_char {
    match unsafe { engine.as_ref() } {
        Some(e) => to_c_string(e.inner.get_processed_string(mode as u32)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn onikey_engine_is_valid(engine: *const OnikeyEngine, full: bool) -> bool {
    unsafe { engine.as_ref() }
        .map(|e| e.inner.is_valid(full))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn onikey_engine_can_process_key(engine: *const OnikeyEngine, key: u32) -> bool {
    match (unsafe { engine.as_ref() }, char::from_u32(key)) {
        (Some(e), Some(c)) => e.inner.can_process_key(c),
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn onikey_engine_remove_last_char(engine: *mut OnikeyEngine, refresh_tone: bool) {
    if let Some(e) = unsafe { engine.as_mut() } {
        e.inner.remove_last_char(refresh_tone);
    }
}

#[no_mangle]
pub extern "C" fn onikey_engine_restore_last_word(engine: *mut OnikeyEngine, to_vietnamese: bool) {
    if let Some(e) = unsafe { engine.as_mut() } {
        e.inner.restore_last_word(to_vietnamese);
    }
}

/// Chuyển chuỗi Unicode sang bảng mã cũ. Trả về chuỗi phải giải phóng.
#[no_mangle]
pub extern "C" fn onikey_encode(charset: *const c_char, input: *const c_char) -> *mut c_char {
    let cs = unsafe { str_from_ptr(charset) }.unwrap_or(charsets::UNICODE);
    let inp = unsafe { str_from_ptr(input) }.unwrap_or("");
    to_c_string(charsets::encode(cs, inp))
}

/// Trả lại chuỗi do các hàm trên cấp phát.
#[no_mangle]
pub extern "C" fn onikey_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn go_qua_ffi_ra_dung_tieng_viet() {
        let im = CString::new("Telex").unwrap();
        let e = onikey_engine_new(im.as_ptr(), 7);
        for c in "tieengs".chars() {
            onikey_engine_process_key(e, c as u32, 1);
        }
        let s = onikey_engine_get_string(e, 1);
        let got = unsafe { CStr::from_ptr(s) }.to_str().unwrap().to_string();
        onikey_string_free(s);
        onikey_engine_free(e);
        assert_eq!(got, "tiếng");
    }

    #[test]
    fn con_tro_rong_khong_lam_sap() {
        onikey_engine_reset(std::ptr::null_mut());
        onikey_engine_process_key(std::ptr::null_mut(), 'a' as u32, 1);
        onikey_engine_free(std::ptr::null_mut());
        onikey_string_free(std::ptr::null_mut());
        assert!(onikey_engine_get_string(std::ptr::null(), 1).is_null());
    }
}

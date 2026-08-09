//! Tìm và kết nối tới ibus-daemon.
//!
//! IBus KHÔNG dùng bus phiên thông thường mà có bus riêng, địa chỉ nằm trong
//! biến `IBUS_ADDRESS` hoặc trong một tệp dưới `~/.config/ibus/bus/`. Tên tệp
//! ghép từ machine-id + tên máy hiển thị, và khác nhau giữa X11 với Wayland —
//! sai chỗ này thì engine không kết nối được mà chẳng có thông báo gì rõ ràng.

use std::path::PathBuf;

pub fn machine_id() -> std::io::Result<String> {
    let raw = std::fs::read_to_string("/var/lib/dbus/machine-id")
        .or_else(|_| std::fs::read_to_string("/etc/machine-id"))?;
    Ok(raw.trim().to_string())
}

fn config_dir() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into())).join(".config"),
    }
}

/// Đường dẫn tệp chứa địa chỉ bus của ibus-daemon.
pub fn socket_path() -> std::io::Result<PathBuf> {
    if let Ok(p) = std::env::var("IBUS_ADDRESS_FILE") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let (hostname, display_number) = if !wayland.is_empty() {
        ("unix".to_string(), wayland)
    } else {
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".into());
        // dạng {hostname}:{số hiển thị}.{số màn hình}
        let (host, rest) = match display.split_once(':') {
            Some((h, r)) => (h.to_string(), r.to_string()),
            None => (String::new(), display.clone()),
        };
        let num = rest.split('.').next().unwrap_or("0").to_string();
        (if host.is_empty() { "unix".into() } else { host }, num)
    };
    let name = format!("{}-{}-{}", machine_id()?, hostname, display_number);
    Ok(config_dir().join("ibus/bus").join(name))
}

/// Địa chỉ DBus của ibus-daemon.
pub fn address() -> std::io::Result<String> {
    if let Ok(a) = std::env::var("IBUS_ADDRESS") {
        if !a.is_empty() {
            return Ok(a);
        }
    }
    let path = socket_path()?;
    let data = std::fs::read_to_string(&path)?;
    for line in data.lines() {
        if let Some(rest) = line.strip_prefix("IBUS_ADDRESS=") {
            return Ok(rest.trim().to_string());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("không thấy IBUS_ADDRESS trong {}", path.display()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duong_dan_socket_khac_nhau_giua_x11_va_wayland() {
        std::env::set_var("HOME", "/home/x");
        std::env::remove_var("IBUS_ADDRESS_FILE");
        std::env::remove_var("XDG_CONFIG_HOME");

        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        let p = socket_path().unwrap();
        assert!(p.to_string_lossy().ends_with("-unix-wayland-0"), "{p:?}");

        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::set_var("DISPLAY", ":0.0");
        let p = socket_path().unwrap();
        assert!(p.to_string_lossy().ends_with("-unix-0"), "{p:?}");
    }
}

# Onikey

Bộ gõ tiếng Việt cho Linux, tinh chỉnh để **gõ không gạch chân dưới từ đang gõ** trên **GNOME Wayland**.

Đây là một **fork của [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)** (BambooEngine), giữ nguyên giấy phép **GPLv3**. Mọi công lao cốt lõi thuộc về các tác giả gốc; kho này chỉ thêm một số bản vá và tinh chỉnh cho môi trường GNOME Wayland.

## Vì sao có fork này

Trên GNOME Wayland, các bộ gõ IBus hiển thị **preedit có gạch chân** dưới từ đang soạn. Fork này mặc định dùng chế độ commit-trực-tiếp (Surrounding Text) để **không còn gạch chân**, đồng thời vá vài lỗi và giảm lỗi gõ khi máy tải nặng.

## Thay đổi so với ibus-bamboo gốc

1. **Vá crash khi chuyển ứng dụng** (`x11_introspector.c`, `engine_utils.go`)
   `x11GetFocusWindowClass()` thiếu null-check con trỏ `Display` → segfault khi focus vào app native-Wayland (Edge/Electron) làm chết engine, mất gõ toàn hệ thống. Đã thêm null-check và bỏ gọi X11 introspection trên phiên Wayland (vốn vô dụng vì GNOME khóa `Shell.Eval`).

2. **Vá panic của hộp thoại cấu hình** (`ui/ui.go`)
   `OpenGUI` panic khi thiếu file macro (chạy chế độ standalone). Đã xử lý mềm, không làm chết tiến trình.

3. **Đồng bộ theo sự kiện, chống lỗi dấu khi lag** (`engine.go`, `engine_backspace.go`)
   Thay việc chờ cứng giữa `DeleteSurroundingText` và `CommitText` bằng cơ chế hỏi–chờ app xác nhận (`waitForSurroundingTextSync`), có timeout dự phòng và tự rút gọn khi app không phản hồi. Giúp thời điểm ghi lại dấu tự thích nghi với độ trễ của app/hệ thống.

## Giới hạn đã biết trên GNOME Wayland

Những điều này là **hạn chế của nền tảng**, không phải lỗi của bộ gõ:

- **Không nhận diện được cửa sổ app đang focus** — GNOME 50 khóa `org.gnome.Shell.Eval`, và app native-Wayland không có WM_CLASS qua X11. Do đó không đặt được chế độ riêng theo từng app.
- **Không dùng được XTest** — cách bơm phím giả (như UniKey/Windows dùng `SendInput`) bị Wayland chặn: GNOME hiện hộp thoại *Remote Desktop / Allow Remote Interaction*. Đây là tính năng bảo mật.
- **App chạy bằng Wine** không hỗ trợ giao thức surrounding text → chữ bị nhân đôi. Không có cách sửa tin cậy từ phía bộ gõ trên GNOME Wayland; giải pháp thực tế là chạy UniKey bản Windows *trong chính prefix Wine* của app đó.
- Đánh đổi cố hữu: **không gạch chân** (commit-rồi-sửa) thì khi máy lag đôi lúc lỗi dấu / mất ký tự đầu; **tin cậy tuyệt đối** thì phải dùng chế độ Pre-edit (có gạch chân). Không có ô "vừa không gạch chân vừa miễn nhiễm lag" vì Wayland đã khóa cơ chế injection.

## Build & cài đặt

Yêu cầu: Go, `libgtk-3-dev`, `libxtst-dev`, `libx11-dev`.

```sh
make
sudo make install PREFIX=/usr
ibus restart
```

Chọn engine **Bamboo** trong *Settings → Keyboard → Input Sources*. Mặc định chế độ không gạch chân (Telex, Unicode).

## Giấy phép

GPLv3 — như dự án gốc ibus-bamboo. Xem [LICENSE](LICENSE).

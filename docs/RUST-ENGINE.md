# Engine Rust — cách dựng và thử

Bản Rust chạy **song song** bản Go: khác tên thành phần
(`org.freedesktop.IBus.OnikeyRust`), khác tên engine (`OnikeyRust`), cùng đọc
một tệp cấu hình. Trong *Settings → Keyboard → Input Sources* nó hiện là
**"Onikey (Rust, thử nghiệm)"**.

Hỏng thì chọn lại "Onikey" là gõ tiếp được — đó là điều kiện để dám thay dần.

## Dựng và cài

```sh
cd rust && cargo build --release -p onikey-ibus
sudo install -Dm755 target/release/onikey-engine-rs /usr/lib/onikey/onikey-engine-rs
sudo install -Dm644 ../data/onikey-rust.xml /usr/share/ibus/component/onikey-rust.xml
ibus restart
```

Đổi qua lại bằng dòng lệnh:

```sh
ibus engine OnikeyRust    # sang bản Rust
ibus engine Onikey        # về bản Go
```

## Gỡ rối

Engine do ibus-daemon khởi chạy nên stdout không xem được ở đâu. Bật log:

```sh
touch ~/.config/onikey/onikey-debug && ibus restart
tail -f ~/.config/onikey/onikey-rust-debug.log
```

Log ghi địa chỉ ibus nối tới, mỗi lần tạo engine, và cấu hình đọc được — đủ để
biết ngay nó có đọc đúng kiểu gõ của bạn không.

## Đã có gì

| Mảng | Trạng thái |
|---|---|
| Lõi tiếng Việt | Khớp 100% bản Go trên 126.831 ca kiểm |
| Tầng DBus (zbus) | Tự tìm địa chỉ ibus (X11 và Wayland khác nhau), Factory, Engine |
| `ContentType` qua `Properties.Set` | Có ngay từ đầu — đúng chỗ `goibus` thiếu |
| Chế độ Pre-edit (có gạch chân) | Xong, đã gõ thật ra `tiếng Việt` |
| Chế độ không gạch chân | Xong phần cơ chế: xoá lùi phần khác nhau rồi ghi đuôi mới |
| Đọc cấu hình chung với bản Go | Xong (kiểu gõ, cờ lõi, cờ IBus) |

## Đã thêm sau lần gõ thật đầu tiên

| Mảng | Ghi chú |
|---|---|
| Tự khôi phục tiếng Anh | `expression`/`password` không bị bẻ dấu; test tích hợp với lõi thật |
| Đồng bộ xoá–ghi | Chờ ứng dụng xác nhận xoá lùi rồi mới ghi (trần 60ms) — sửa lỗi `password` → `passsowrd` khi gõ nhanh |
| Menu thuộc tính | Bấm biểu tượng `vi` có mục "Cấu hình bộ gõ (bản Rust)" |
| Gõ tắt (macro) | Đọc chung `onikey.macro.text`, tự chỉnh hoa theo cách gõ khoá (`vn`→Việt Nam, `VN`→VIỆT NAM) |

## Đã thêm tiếp

| Mảng | Ghi chú |
|---|---|
| Phím tắt chuyển Anh–Việt | Đọc từ `Shortcuts` trong cấu hình (đang tắt trên máy này) |
| Phím tắt khôi phục phím gốc | Thay chữ đang gõ bằng đúng chuỗi đã bấm (đang tắt trên máy này) |
| Bảng mã đầu ra | `encode` theo `OutputCharset` ở mọi chỗ chốt chữ; chế độ không gạch chân đếm xoá lùi theo chuỗi **đã mã hoá** (VNI Windows 2 ký tự/chữ có dấu) |
| Tự nạp lại cấu hình | So mtime ở FocusIn — đổi kiểu gõ trong hộp thoại xong bấm sang ô khác là ăn, không cần `ibus restart` |

## Chưa có (nên chưa thay được bản Go)

- Bảng emoji, gõ hexadecimal (cấu hình máy này đang tắt emoji; hexa ít dùng).
- Phím tắt chuyển chế độ gõ Shift+~ (bản Go cũng chỉ chạy khi nhận diện được
  ứng dụng — trên Wayland vốn không nhận diện được nên gần như vô dụng).
- Danh sách loại trừ theo ứng dụng (cùng lý do Wayland).
- Hộp thoại cấu hình vẫn là bản GTK3 của engine Go.
- Macro dạng tiền tố (gõ tiếp sau khoá).

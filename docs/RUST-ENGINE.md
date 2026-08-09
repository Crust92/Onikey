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

## Chưa có (nên chưa thay được bản Go)

- Gõ tắt (macro), bảng emoji, gõ hexadecimal.
- Các phím tắt: chuyển chế độ gõ (Shift+~), tạm tắt bộ gõ, khôi phục phím gốc.
- Bảng mã đầu ra (lõi có sẵn hàm `encode`, tầng IBus chưa gọi tới).
- Danh sách loại trừ theo ứng dụng.
- Hộp thoại cấu hình vẫn là bản GTK3 của engine Go.

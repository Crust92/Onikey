Onikey — Bộ gõ tiếng Việt cho Linux, engine viết bằng Rust
==========================================================
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://opensource.org/licenses/GPL-3.0)

**Onikey 1.0** là bộ gõ tiếng Việt cho IBus, khởi đầu là fork của
[ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo) và nay chạy bằng
**engine viết lại hoàn toàn bằng Rust** — giữ đúng từng ký tự hành vi gõ của
bản gốc, sửa những lỗi nền tảng mà bản gốc không sửa được, và thêm những thứ
người gõ hằng ngày cảm nhận rõ. Toàn bộ công lao về cách gõ tiếng Việt thuộc
về các tác giả BambooEngine; kho này giữ nguyên giấy phép **GPLv3**.

## Mục lục
- [Điểm nổi bật kỹ thuật](#điểm-nổi-bật-kỹ-thuật)
- [Kiến trúc](#kiến-trúc)
- [Cài đặt từ mã nguồn](#cài-đặt-từ-mã-nguồn)
- [Sử dụng](#sử-dụng)
- [Gỡ rối](#gỡ-rối)
- [Giới hạn trên GNOME Wayland](#giới-hạn-trên-gnome-wayland)
- [Giấy phép](#giấy-phép)

## Điểm nổi bật kỹ thuật

**Lõi Rust khớp 100% hành vi bản gốc — có chứng minh, không phải lời hứa.**
Trước khi viết dòng Rust nào, bộ ca kiểm **126.831 ca** được sinh từ chính
engine Go: mỗi ca ghi chuỗi tiếng Việt **sau từng phím** (không chỉ kết quả
cuối), cộng chuỗi phím gốc, tính hợp lệ của vần, và trạng thái sau xoá
lùi/khôi phục — trên 9 kiểu gõ (Telex/Telex 2/VNI/VIQR/MS layout…) và 4 tổ
hợp cờ. Lõi Rust phải khớp **từng ký tự, từng bước** mới được nhận
(`make rust-test`, chạy trong CI mỗi lần push). Bảng mã (TCVN3, VNI Windows,
VISCII…) do máy sinh từ bảng gốc, đối chiếu 134.521 ca, lệch 0.

**Sửa chỗ mù của tầng IBus mà bản gốc mang theo nhiều năm.** Từ IBus 1.5,
kiểu ô nhập (URL/email/mật khẩu) gửi bằng **thuộc tính DBus `ContentType`**,
không qua phương thức `SetContentType` — thư viện `goibus` của bản gốc không
xử lý thuộc tính DBus nên engine chưa bao giờ nhận được thông tin này. Engine
Rust (dùng `zbus`) hiện thực `Properties.Set` ngay từ đầu.

**Gõ nhanh không lỗi dấu ở chế độ không gạch chân.** Kiểu "sửa chữ đã ghi"
(xoá lùi rồi ghi đè) vốn dễ trộn chữ khi ứng dụng xử lý không kịp
(`password` → `passsowrd`). Onikey đồng bộ **theo sự kiện**: xoá xong chờ ứng
dụng xác nhận (nó gửi lại surrounding text) rồi mới ghi, trần chờ 60ms — app
nhanh thì ghi ngay, app chậm thì không dồn phím.

**Tự khôi phục tiếng Anh.** Gõ `expression`, `password` giữa câu tiếng Việt
không bị bẻ dấu: khi chuỗi không còn là vần tiếng Việt hợp lệ, engine trả về
đúng chuỗi phím đã bấm. Giữ ngoại lệ `dd` → `đ` cho viết tắt.

**Menu áp tức thì, cấu hình nóng.** Đổi kiểu gõ/bảng mã từ menu trên thanh
hệ thống là lõi dựng lại ngay — không cần `ibus restart`. Sửa cấu hình bằng
hộp thoại hay tay đều được: engine so mtime tệp và tự nạp lại. Menu chỉ chứa
mục **có tác dụng thật**.

**Engine không thể chết vì GUI.** Hộp thoại cấu hình là tiến trình riêng
(`onikey-config`); engine chỉ liên kết libc — GUI lỗi cỡ nào cũng không mất
gõ toàn hệ thống (bản gốc từng panic cả engine vì thiếu một tệp macro).

**Chọn chế độ gõ an toàn theo khả năng thật của ứng dụng.** Chế độ không
gạch chân chỉ bật khi ứng dụng cung cấp được surrounding text; thiếu (Zalo,
Electron cũ) thì giữ Pre-edit — thà có gạch chân còn hơn nuốt phím. Khả năng
của từng ứng dụng đo thật và ghi ở [docs/APP-COMPAT.md](docs/APP-COMPAT.md).

**Đóng gói đa distro, kiểm thật.** `PREFIX`/`LIBEXECDIR` chuẩn (Fedora dùng
`/usr/libexec`), component XML sinh theo nơi cài. Build + cài kiểm trong
container Fedora/Arch/Debian; gõ thật kiểm trong máy ảo (GNOME Wayland, GNOME
X11, KDE Plasma) bằng phím bơm ở tầng nhân — cách làm ở
[docs/VM-TEST.md](docs/VM-TEST.md).

**Sẵn đường đi tiếp.** Lõi xuất **C FFI** (`libonikey_core` + `onikey.h`,
kiểm bằng chương trình C thật trong CI) — nền cho addon Fcitx5/XIM theo lộ
trình [docs/ROADMAP.md](docs/ROADMAP.md).

## Kiến trúc

```
rust/onikey-core      lõi tiếng Việt thuần (không I/O, không phụ thuộc crate nào)
rust/onikey-core-ffi  vỏ C ABI: libonikey_core.{a,so} + include/onikey.h
rust/onikey-ibus      engine IBus (zbus) — binary onikey-engine-rs, TÊN ENGINE "Onikey"
*.go                  engine Go dự phòng — tên engine "OnikeyGo", sẽ gỡ khi hết vai trò
```

## Cài đặt

**Cách khuyến nghị (Ubuntu/Debian) — cài một lần, về sau `apt upgrade` tự lên bản mới:**
```sh
curl -fsSL https://crust92.github.io/Onikey/onikey-archive-keyring.gpg \
  | sudo tee /usr/share/keyrings/onikey-archive-keyring.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/onikey-archive-keyring.gpg] https://crust92.github.io/Onikey stable main" \
  | sudo tee /etc/apt/sources.list.d/onikey.list
sudo apt update
sudo apt install onikey          # bộ gõ cho IBus (GNOME mặc định)
sudo apt install fcitx5-onikey   # addon cho Fcitx5 (KDE...)
```
Xong. **Bấm <kbd>Super</kbd>+<kbd>Space</kbd> là gõ được tiếng Việt** — gói
tự đánh thức IBus và tự thêm nguồn nhập, không cần đăng xuất, không cần vào
Settings. (Nguồn nhập được THÊM vào cuối danh sách, bàn phím hiện tại giữ
nguyên; muốn gỡ thì xoá trong Settings như thường.)

**Đã cài trước 2026-08-10 (địa chỉ kho cũ `xtcrust.github.io`)?** Kho đã
chuyển sang `crust92.github.io` và địa chỉ cũ KHÔNG chuyển hướng (GitHub
Pages không làm việc đó khi đổi tên tài khoản) — `apt update` sẽ báo 404 cho
tới khi trỏ lại:
```sh
curl -fsSL https://crust92.github.io/Onikey/onikey-archive-keyring.gpg \
  | sudo tee /usr/share/keyrings/onikey-archive-keyring.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/onikey-archive-keyring.gpg] https://crust92.github.io/Onikey stable main" \
  | sudo tee /etc/apt/sources.list.d/onikey.list
sudo apt update && sudo apt upgrade
```
Phải tải lại **cả tệp khoá**: bản tải từ địa chỉ cũ nay chỉ là trang 404, dùng
tiếp thì `apt update` báo `NO_PUBKEY ... is not signed`.

## Cài đặt từ mã nguồn

**Yêu cầu (Ubuntu/Debian và tương tự):**
```sh
sudo apt install -y golang cargo gcc make pkg-config libgtk-3-dev libxtst-dev libx11-dev
```

**Build & cài đặt** (build bằng user thường, chỉ bước cài mới cần sudo):
```sh
git clone https://github.com/Crust92/Onikey.git
cd Onikey
make
sudo make install PREFIX=/usr
ibus restart
```

**Chọn bộ gõ:** *Settings → Keyboard → Input Sources → +* → *Vietnamese* →
**Onikey**. Hoặc:
```sh
gsettings set org.gnome.desktop.input-sources sources "[('ibus', 'Onikey'), ('xkb', 'us')]"
```

Gỡ cài đặt: `sudo make uninstall`. Cài nơi khác `/usr`: truyền `PREFIX=`,
Fedora thêm `LIBEXECDIR=/usr/libexec/onikey`.

## Sử dụng

- Mặc định **Telex, Unicode, Pre-edit** (gạch chân dưới từ đang gõ — tin cậy
  tuyệt đối kể cả khi máy lag). Muốn không gạch chân: chọn chế độ 2 trong hộp
  thoại cấu hình.
- Bấm biểu tượng `vi` trên thanh hệ thống: chọn **kiểu gõ**, **bảng mã**, bật
  **gõ tắt**, mở hộp thoại cấu hình — mọi thay đổi áp ngay.
- Gõ tắt dùng tệp `~/.config/onikey/onikey.macro.text` (`khoá:văn bản` mỗi
  dòng), tự viết hoa theo cách gõ khoá (`vn`→Việt Nam, `VN`→VIỆT NAM).
- Engine dự phòng: `ibus engine OnikeyGo` (bản Go cũ) — quay về bằng
  `ibus engine Onikey`.

## Gỡ rối

Engine do ibus-daemon khởi chạy nên stdout không xem được ở đâu. Bật log:
```sh
touch ~/.config/onikey/onikey-debug && ibus restart
```
Log ghi vào `~/.config/onikey/onikey-rust-debug.log`: cấu hình nạp được, từng
phím và hành động engine chọn (pre-edit gì, xoá mấy ký tự, ghi gì) — đủ để
chẩn đoán từ xa. Xoá tệp cờ rồi `ibus restart` để tắt.

## Giới hạn trên GNOME Wayland

Hạn chế của nền tảng, không phải lỗi bộ gõ:
- **Không nhận diện được ứng dụng đang focus** (GNOME khoá `Shell.Eval`,
  không có WM_CLASS) → không có cấu hình riêng theo từng app; thay vào đó
  Onikey quyết định theo **khả năng ứng dụng tự khai báo**.
- **Không bơm được phím giả** (Wayland chặn XTest) → không có chế độ
  XTestFakeKeyEvent như trên X11.
- App Wine không hỗ trợ surrounding text → dùng Pre-edit ở đó.

## Giấy phép

GPLv3 — như dự án gốc [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)
và lõi [bamboo-core](https://github.com/BambooEngine/bamboo-core) mà Onikey
kế thừa cách gõ. Xem [LICENSE](LICENSE).

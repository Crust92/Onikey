Onikey — Bộ gõ tiếng Việt không gạch chân cho GNOME Wayland
===========================================================
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://opensource.org/licenses/GPL-3.0)

**Onikey** là bản **fork của [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)** (BambooEngine), tinh chỉnh để **gõ tiếng Việt không có gạch chân dưới từ đang gõ** trên **GNOME Wayland**, kèm một số bản vá ổn định. Toàn bộ công lao cốt lõi thuộc về các tác giả gốc; kho này giữ nguyên giấy phép **GPLv3**.

> Từ bản 0.9.0, engine mang tên **Onikey** (trước đây là *Bamboo*) — trong *Settings → Keyboard → Input Sources* bạn chọn engine tên **Onikey**. Cấu hình cũ ở `~/.config/ibus-bamboo` được tự chuyển sang `~/.config/onikey` ở lần chạy đầu.

## Mục lục
- [Điểm khác biệt của Onikey](#điểm-khác-biệt-của-onikey)
- [Tính năng](#tính-năng)
- [Cài đặt từ mã nguồn](#cài-đặt-từ-mã-nguồn)
- [Hướng dẫn sử dụng](#hướng-dẫn-sử-dụng)
- [Giới hạn trên GNOME Wayland](#giới-hạn-trên-gnome-wayland)
- [Báo lỗi](#báo-lỗi)
- [Giấy phép](#giấy-phép)

## Điểm khác biệt của Onikey

So với ibus-bamboo gốc, fork này:

1. **"Không gạch chân" là thật.** Bật ô tick *"Gõ không gạch chân ở mọi ô nhập"* thì engine chuyển hẳn sang chế độ Surrounding Text ở những ứng dụng hỗ trợ, thay vì chỉ bỏ thuộc tính gạch chân của pre-edit như bản gốc (bỏ thuộc tính xong ứng dụng vẫn tự vẽ gạch chân). Ứng dụng không cung cấp surrounding text (Zalo/Electron cũ) thì giữ Pre-edit — thà gạch chân còn hơn nuốt phím.
2. **Nhận được kiểu ô nhập từ ứng dụng.** Từ IBus 1.5, kiểu ô nhập (URL/email/mật khẩu…) gửi qua **thuộc tính DBus `ContentType`** chứ không qua phương thức `SetContentType`; thư viện `goibus` không xử lý thuộc tính DBus nên ibus-bamboo gốc chưa bao giờ nhận được. Onikey thêm handler `org.freedesktop.DBus.Properties.Set` — đây là cơ sở cho chế độ lai ở trên.
3. **Vá crash khi chuyển ứng dụng.** `x11GetFocusWindowClass()` thiếu null-check con trỏ `Display` → segfault khi focus vào app native-Wayland (Edge/Electron), làm chết engine và mất gõ toàn hệ thống. Đã thêm null-check và bỏ gọi X11 introspection trên phiên Wayland.
4. **Vá panic của hộp thoại cấu hình** khi thiếu file macro (chạy standalone).
5. **Đồng bộ theo sự kiện, giảm lỗi dấu khi máy lag.** Thay việc chờ cứng giữa `DeleteSurroundingText` và `CommitText` bằng cơ chế hỏi–chờ app xác nhận (có timeout dự phòng và tự rút gọn khi app không phản hồi).

## Tính năng
Kế thừa đầy đủ từ ibus-bamboo:
* Bảng mã: Unicode, TCVN (ABC), VIQR, VNI, VPS, VISCII, BK HCM1/2, Unicode UTF-8, Unicode NCR…
* Kiểu gõ: Telex, Telex W, Telex 2, VNI, VIQR, Microsoft layout…
* Kiểm tra chính tả (từ điển/luật ghép vần), dấu thanh chuẩn & kiểu mới, bỏ dấu tự do, gõ tắt, 2666 emoji.
* Nhiều **chế độ gõ**: Pre-edit (có gạch chân) và các chế độ không gạch chân (Surrounding Text, ForwardKeyEvent…). Chuyển nhanh bằng <kbd>Shift</kbd>+<kbd>~</kbd>.

## Cài đặt từ mã nguồn

**Yêu cầu (Ubuntu/Debian và tương tự):**
```sh
sudo apt install -y golang git make gcc libgtk-3-dev libxtst-dev libx11-dev
```

**Build & cài đặt:**
```sh
git clone https://github.com/XTCRust/Onikey.git
cd Onikey
make
sudo make install PREFIX=/usr
ibus restart
```

**Chọn bộ gõ:** vào *Settings → Keyboard → Input Sources → +* → *Vietnamese* → **Onikey**. Hoặc đặt nhanh bằng lệnh:
```sh
gsettings set org.gnome.desktop.input-sources sources "[('xkb', 'us'), ('ibus', 'Onikey')]"
```
Chuyển giữa tiếng Anh (`us`) và tiếng Việt (`Onikey`) bằng <kbd>Super</kbd>+<kbd>Space</kbd>.

**Gỡ cài đặt:** trong thư mục mã nguồn chạy `sudo make uninstall`.

**Cài vào nơi khác `/usr`** (Fedora, FreeBSD, hoặc cài cho riêng một người dùng):
```sh
make build PREFIX=/usr/local
sudo make install PREFIX=/usr/local            # Fedora: thêm LIBEXECDIR=/usr/libexec/onikey
```
Đường dẫn dữ liệu được nhúng lúc build theo `PREFIX`, còn component XML và
desktop file thì `scripts/install` sinh ra theo đúng nơi đang cài. Chạy thử bản
build tại chỗ mà không cài thì đặt `ONIKEY_DATA_DIR=$PWD`.

**Hai binary, có chủ đích:** `onikey-engine` (bộ gõ, chỉ cần libc + X11) và
`onikey-config` (hộp thoại cấu hình, cần GTK3). Engine gọi hộp thoại bằng
`exec`, nên hộp thoại lỗi cũng không làm mất gõ toàn hệ thống.

## Cập nhật bằng gói (.deb)
Onikey đóng gói được thành `.deb` để cài/cập nhật gọn qua dpkg/apt:
```sh
sh scripts/build-deb              # tạo onikey_<phiên-bản>_<arch>.deb
sudo apt install ./onikey_*.deb   # cài (dpkg quản lý)
ibus restart
```
Sau lần cài đầu, gói kèm sẵn lệnh **`onikey-update`** — chạy bất cứ lúc nào để tự lấy bản mới nhất từ GitHub, đóng gói lại và cài đè:
```sh
onikey-update
```
Phiên bản gói tăng dần theo giờ commit nên `apt` luôn nhận ra bản mới hơn. Gỡ: `sudo apt remove onikey`.

## Hướng dẫn sử dụng
- Mặc định là **Telex, Unicode, không gạch chân**. Gõ ngay được, ví dụ `Tieengs Vieejt` → *Tiếng Việt*.
- Onikey có nhiều **chế độ gõ** (đừng nhầm với **kiểu gõ** như telex/vni). Nhấn vào một khung nhập rồi bấm <kbd>Shift</kbd>+<kbd>~</kbd> để chọn chế độ khác.
- Một app có thể hợp với chế độ này mà không hợp chế độ khác; dùng *Thêm vào danh sách loại trừ* để tắt tiếng Việt cho một app.
- Để gõ ký tự `~`, bấm <kbd>Shift</kbd>+<kbd>~</kbd> hai lần.

## Giới hạn trên GNOME Wayland
Đây là **hạn chế của nền tảng**, không phải lỗi bộ gõ:
- **Không nhận diện được cửa sổ app đang focus** (GNOME khóa `org.gnome.Shell.Eval`; app native-Wayland không có WM_CLASS qua X11) → không đặt được chế độ riêng theo từng app.
- **Không dùng được cơ chế bơm phím giả (XTest)** như UniKey/Windows dùng `SendInput` — Wayland chặn vì lý do bảo mật (hiện hộp thoại *Remote Desktop*).
- **App chạy bằng Wine** không hỗ trợ surrounding text → chữ bị nhân đôi. Giải pháp thực tế: chạy UniKey bản Windows *trong chính prefix Wine* của app đó.
- Đánh đổi cố hữu: **không gạch chân** thì khi máy lag đôi lúc lỗi dấu/mất ký tự đầu; muốn **tin cậy tuyệt đối** thì dùng chế độ Pre-edit (có gạch chân). Không có ô "vừa không gạch chân vừa miễn nhiễm lag" vì Wayland đã khóa cơ chế injection.

## Gỡ rối
Engine do ibus-daemon khởi chạy nên không xem được stdout. Bật log:
```sh
touch ~/.config/onikey/onikey-debug && ibus restart
```
Log ghi vào `~/.config/onikey/onikey-debug.log` (focus, capabilities, kiểu ô nhập, chế độ gõ đang dùng). Tắt bằng cách xóa file cờ rồi `ibus restart`.

## Báo lỗi
Mở issue tại [github.com/XTCRust/Onikey/issues](https://github.com/XTCRust/Onikey/issues). Với các vấn đề chung của engine, có thể tham khảo thêm [wiki của ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo/wiki).

## Giấy phép
GPLv3 — như dự án gốc [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo). Xem [LICENSE](LICENSE).

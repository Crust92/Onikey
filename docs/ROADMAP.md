# Onikey — kế hoạch: đa nền tảng Linux & chuyển sang Rust

> Trạng thái: bản nháp làm việc, 2026-08. Mục tiêu của tài liệu là **thứ tự làm**
> và **ranh giới kiến trúc**, không phải lịch trình theo ngày.

## 0. Hiện trạng (điểm xuất phát)

| Mảng | Hôm nay |
|---|---|
| Ngôn ngữ | Go (engine) + C (GTK3 GUI cấu hình, X11 helper) |
| Lõi tiếng Việt | `github.com/BambooEngine/bamboo-core` (Go, GPLv3) |
| Giao tiếp | IBus qua DBus, dùng `goibus` (Go, tự duy trì bởi dự án gốc) |
| Chạy được | GNOME Wayland/X11 với IBus. Chưa hỗ trợ Fcitx5, chưa hỗ trợ XIM thuần, chưa hỗ trợ Wayland input-method-v2 |
| Đóng gói | `.deb` tự dựng (`scripts/build-deb`), spec RPM và PKGBUILD kế thừa từ dự án gốc — **chưa ai kiểm thử** |
| Điểm yếu đã biết | `goibus` không xử lý thuộc tính DBus (đã tự vá `ContentType`); phụ thuộc đường dẫn `/usr` cứng; dò cửa sổ bằng `Shell.Eval` vô dụng trên GNOME đời mới |

## 1. Ràng buộc phải nhớ (quyết định kiến trúc dựa trên đây)

1. **GNOME bắt buộc đi qua IBus.** Mutter không cài đặt `zwp_input_method_v2`
   (lẫn v1), nên bộ gõ Wayland "thuần" không chạy trên GNOME — kime đã đâm vào
   đúng bức tường này. Với máy chính (Ubuntu GNOME) thì **IBus là đường duy nhất**.
2. **Fcitx5 mở rộng bằng addon C++** nạp động, không có API DBus để cắm engine từ
   ngoài. Muốn hỗ trợ Fcitx5 thì phải có một lớp addon C++ mỏng gọi vào lõi.
   `fcitx5-cskk` làm đúng vậy: addon C++ + lõi Rust `libcskk` qua C FFI.
3. **Lõi tiếng Việt là phần đáng giá nhất và ít phụ thuộc nền tảng nhất** — nó
   phải là thư viện thuần, không I/O, không DBus, để mọi frontend dùng chung.
4. **Giấy phép**: `bamboo-core` là GPLv3. Bản port sang Rust là tác phẩm phái
   sinh → Onikey giữ GPLv3. Không được "mượn" code rồi đổi giấy phép.

## 2. Kiến trúc đích

```
                       ┌──────────────────────────┐
                       │  onikey-core  (Rust lib) │  thuần, không I/O
                       │  telex/vni/viqr, dấu,    │  test bằng bảng ca kiểm
                       │  chính tả, bảng mã, macro│
                       └───────────┬──────────────┘
                                   │ C FFI (staticlib + onikey.h)
        ┌──────────────────┬───────┴────────┬───────────────────┐
        │                  │                │                   │
┌───────▼───────┐  ┌───────▼───────┐ ┌──────▼──────┐   ┌────────▼────────┐
│ onikey-ibus   │  │ onikey-fcitx5 │ │ onikey-xim  │   │ onikey-wayland  │
│ Rust + zbus   │  │ addon C++     │ │ crate xim   │   │ input-method-v2 │
│ GNOME, mọi    │  │ KDE, đa số    │ │ X11 cổ điển │   │ sway/hyprland   │
│ distro có IBus│  │ distro        │ │ (Wine, Java)│   │ (KDE có hỗ trợ) │
└───────────────┘  └───────────────┘ └─────────────┘   └─────────────────┘
```

Phần **GUI cấu hình** tách hẳn thành tiến trình riêng (`onikey-config`), không
nằm trong engine — hiện GUI GTK3 viết bằng C đang bị nhét chung vào binary
engine và từng gây panic khi thiếu file macro.

## 3. Lộ trình

### Giai đoạn A — dọn nền trên bản Go hiện tại (làm trước, rủi ro thấp)

Mục tiêu: bản Go vẫn là bản dùng hằng ngày, nhưng hết các giả định "chỉ Ubuntu".

- [x] **Bỏ đường dẫn cứng**: `DataDir` nay là biến, đặt lúc build bằng
      `-X main.DataDir=$PREFIX/share/onikey`, đè lúc chạy bằng `ONIKEY_DATA_DIR`.
      `scripts/install` sinh component XML + desktop file theo đúng prefix đang
      cài, và nhận `LIBEXECDIR` riêng (Fedora dùng `/usr/libexec`).
- [x] **Tách GUI khỏi engine**: `onikey-config` là binary riêng, engine chỉ
      `exec` rồi chờ. Engine **không còn liên kết GTK** — chỉ còn libc + X11/Xtst.
- [x] **Bỏ hẳn `Shell.Eval`**: xoá `gnome_introspector.go`. Nhận diện ô nhập nay
      dựa vào content type ứng dụng khai báo (`isURLBar`).
- [x] **Kiểm thử đóng gói thật** trong container: build + install sạch trên
      Fedora 42 (layout `/usr/libexec`) và Arch; `.deb` cài–gỡ sạch trên
      Ubuntu 24.04 (cũ hơn máy dựng gói).
- [ ] **Ma trận kiểm thử tay** (mỗi bản: gõ ở ô thường, ô địa chỉ, xóa lùi,
      chuyển app, đăng nhập lại): Ubuntu GNOME Wayland ✔ (máy chính) ·
      Ubuntu GNOME X11 · Fedora GNOME · KDE Plasma. Cần máy ảo, chưa làm.

### Giai đoạn B — `onikey-core` bằng Rust, chạy song song

Mục tiêu: có lõi Rust **đạt ngang tính năng**, kiểm chứng bằng chính bộ ca kiểm
sinh ra từ bản Go, mà chưa đụng gì tới bản đang dùng.

- [ ] Dựng bộ **ca kiểm đối chiếu**: chạy bản Go, sinh ra bảng
      `(kiểu gõ, chuỗi phím) -> chuỗi ra` cho vài chục nghìn trường hợp
      (gồm cả tiếng Việt sai chính tả, phím lặp, khôi phục phím gốc).
      Đây là lưới an toàn cho toàn bộ cuộc chuyển đổi.
- [ ] Cài đặt `onikey-core`: bộ chuyển đổi Telex/Telex2/VNI/VIQR/MS layout, đặt
      dấu chuẩn & kiểu mới, bỏ dấu tự do, kiểm tra chính tả (luật ghép vần +
      từ điển), bảng mã ra (Unicode, TCVN3, VNI-Win, VIQR, NCR…), gõ tắt.
      Tham khảo `vi-rs` (Telex/VNI, MIT) để đối chiếu cách đặt dấu, nhưng **không
      dựa vào nó làm lõi**: thiếu bảng mã, chính tả, macro — đúng những thứ
      ibus-bamboo hơn người.
- [ ] `cargo test` phải xanh trên toàn bộ bảng ca kiểm của bước 1.
- [ ] Xuất **C FFI** (`onikey-core-ffi`: staticlib + `onikey.h`), API dạng
      "nạp phím vào, hỏi trạng thái ra", không giữ trạng thái toàn cục.

### Giai đoạn C — `onikey-ibus` bằng Rust thay thế engine Go

- [ ] Cài đặt `org.freedesktop.IBus.Engine` bằng `zbus` (sinh khung bằng
      `zbus_xmlgen` từ chính XML của IBus). **Nhớ làm cả `Properties.Set`** —
      đây là chỗ `goibus` thiếu khiến `ContentType` không bao giờ tới engine.
- [ ] Bê nguyên các bài học đã trả giá của bản Go: chế độ lai theo ô nhập, chốt
      chế độ lúc focus, đồng bộ theo sự kiện cho Surrounding Text, không chặn
      luồng xử lý phím bằng lời gọi đồng bộ ra ngoài.
- [ ] Chạy song song: gói `.deb` cài cả hai, đổi qua lại bằng biến môi trường
      hoặc hai component XML khác tên, để lùi về bản Go được ngay khi hỏng.
- [ ] Khi bản Rust chạy ổn định 2–3 tuần dùng thật → xoá mã Go.

### Giai đoạn D — mở rộng nền tảng

- [ ] `onikey-fcitx5`: addon C++ mỏng gọi C FFI (mẫu: `fcitx5-cskk`) → KDE và
      phần lớn distro không dùng IBus.
- [ ] `onikey-xim`: dùng crate `xim` (của kime) → app X11 cổ điển, Wine, Java.
- [ ] `onikey-wayland`: `zwp_input_method_v2` → sway/hyprland (và KDE).
      **Không** dùng được trên GNOME, xem mục 1.
- [ ] `onikey-config`: GUI cấu hình viết lại (gtk4-rs hoặc Slint), dùng chung
      file cấu hình với mọi frontend.

## 4. Rủi ro và cách phòng

| Rủi ro | Phòng |
|---|---|
| Lõi Rust sai dấu ở ca hiếm → gõ sai mà không biết | Bảng ca kiểm đối chiếu sinh từ bản Go **trước khi** viết dòng Rust nào |
| Viết lại xong mất tính năng lặt vặt (macro, bảng mã cũ, phím tắt) | Liệt kê tính năng thành danh sách kiểm, đánh dấu từng mục khi đạt |
| Mất bộ gõ giữa chừng trên máy chính | Luôn giữ được đường lùi: hai engine cùng cài, đổi bằng chọn nguồn nhập |
| Đổi tên/đường dẫn làm mất cấu hình người dùng | Đã có `MigrateLegacyConfig`; mọi lần đổi bố cục phải kèm bước chuyển đổi |
| Cắm đầu vào Wayland thuần rồi phát hiện GNOME không hỗ trợ | Đã kiểm chứng trước: GNOME = IBus, ghi trong mục 1 |

## 5. Đã chốt (2026-08)

- **Cách chuyển đổi: từng bước, giữ bản Go chạy song song.** Không viết lại từ
  đầu. Máy chính phải luôn gõ được; mỗi giai đoạn đều có đường lùi.
- **Thứ tự nền tảng sau IBus: Fcitx5 trước, rồi XIM.** Fcitx5 mở ra KDE và phần
  lớn distro; XIM đánh vào đúng chỗ đang bất lực (app Wine nhân đôi chữ, Java).
  Wayland `input-method-v2` **hoãn** — không dùng được trên GNOME nên lợi ích
  thấp cho tới khi có máy chạy wlroots/KDE.
- **Cấu hình chuyển sang TOML** khi sang Rust. Kèm bước chuyển đổi tự động từ
  `onikey.config.json` sang `onikey.toml` (giữ nguyên tinh thần
  `MigrateLegacyConfig`: chỉ đọc bản cũ, ghi bản mới, không xoá).

## 6. Việc chưa quyết

- GUI cấu hình: gtk4-rs (nặng, hợp GNOME) hay Slint/iced (nhẹ, đồng nhất mọi DE).
- Có giữ chế độ `XTestFakeKeyEvent` không — Wayland đã khoá, chỉ còn ý nghĩa trên X11
  (sẽ rõ hơn sau khi có adapter XIM).

## Nguồn đã tra

- kime (IME Rust, XIM/Wayland/GTK/Qt) và ghi chú "Mutter không cài đặt
  `zwp_input_method_v2`": <https://github.com/Riey/kime>
- `fcitx5-cskk` — addon C++ Fcitx5 + lõi Rust qua C FFI:
  <https://github.com/fcitx/fcitx5-cskk>
- vnkey — bộ gõ tiếng Việt lõi Rust + adapter Fcitx5/IBus/Windows/macOS:
  <https://github.com/marixdev/vnkey>
- `vi-rs` — thư viện gõ tiếng Việt bằng Rust (Telex/VNI):
  <https://github.com/ZeroX-DG/vi-rs>
- `zbus` + `zbus_xmlgen` (DBus cho Rust): <https://lib.rs/crates/zbus_xmlgen>
- Hướng dẫn viết input method cho Fcitx 5:
  <https://fcitx-im.org/wiki/Develop_an_simple_input_method>

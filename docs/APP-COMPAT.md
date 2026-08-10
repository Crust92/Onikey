# Ứng dụng nào khai báo gì cho bộ gõ

Bảng này ghi những gì **đo được thật** (bật log gỡ rối rồi focus vào ô nhập của
từng ứng dụng), không phải phỏng đoán. Nó quyết định Onikey chọn chế độ gõ nào.

Bật log để tự kiểm chứng:

```sh
touch ~/.config/onikey/onikey-debug && ibus restart
tail -f ~/.config/onikey/onikey-debug.log
```

## Ý nghĩa các con số

- `cap` — khả năng của ứng dụng (IBus capabilities). Bit đáng kể:
  `0x01` hiện được pre-edit, `0x20` **cung cấp được surrounding text**.
  - `cap=0x29` = pre-edit + focus + surrounding text → chạy được mọi chế độ.
  - `cap=0x9`  = **không** có surrounding text → chỉ nên dùng Pre-edit.
- `purpose` — kiểu ô nhập (IBusInputPurpose): `0` thường, `5` URL, `6` email,
  `8` mật khẩu. Onikey chỉ ghi nhận để gỡ rối; **không** còn đổi chế độ theo nó.

## Đã đo (GNOME 50 Wayland, Ubuntu 26.04)

| Ứng dụng | cap | purpose | Chế độ Onikey dùng | Ghi chú |
|---|---|---|---|---|
| GTK (gedit/zenity/Text Editor) | `0x29` | `0` | Pre-edit | Gõ `tieengs Vieejt` ra đúng `tiếng Việt` (kiểm thử bơm phím thật qua uinput) |
| Microsoft Edge — ô địa chỉ | `0x29` (lúc đầu `0x9`) | **`5`** | theo cấu hình | Edge báo capability theo **hai nhịp** — chốt chế độ ở nhịp đầu là hỏng, xem `updateNoUnderlineMode` |
| Microsoft Edge — ô trong trang | `0x29` | `0` | Pre-edit | |
| Firefox — ô địa chỉ | `0x29` | `0` | Pre-edit (vẫn gạch chân) | Firefox **không** khai báo kiểu URL cho thanh địa chỉ của chính nó |
| Firefox — ô trong trang web | `0x29` | theo HTML | Pre-edit / Surrounding Text | `type=url` và `inputmode=url` đều cho `purpose=5`; `type=email` cho `6` |
| Zalo Desktop (Electron 22) | **`0x9`** | `0` | Pre-edit | **Không hỗ trợ surrounding text** → chế độ không gạch chân sẽ gõ lỗi ở đây. Gõ tiếng Việt ở chế độ Pre-edit: **đã kiểm, chạy tốt** |

## Rút ra

1. **Mặc định Pre-edit là lựa chọn đúng.** Ứng dụng Electron cũ (Zalo) không có
   surrounding text; ép chế độ không gạch chân ở đó là hỏng chữ.
2. **Chỉ bỏ gạch chân khi ứng dụng thật sự hỗ trợ.** Onikey kiểm tra bit
   surrounding text trước khi đổi chế độ; thiếu thì giữ Pre-edit (xem
   `updateNoUnderlineMode`). Từng có bản lùi về *Forward as commit* khi thiếu
   bit này và hậu quả là ô địa chỉ Edge nuốt phím.
3. **Không suy đoán theo tên ứng dụng.** Trên GNOME Wayland không lấy được
   WM_CLASS; mọi quyết định phải dựa vào cái ứng dụng tự khai báo.

## Đo trên VM (2026-08-10): ô địa chỉ trình duyệt và text-input-v3

Thí nghiệm trên VM Fedora 42 GNOME Wayland + Ubuntu 24.04 GNOME X11, bơm phím
uinput, đọc log engine từng sự kiện:

| Đường vào | ContentType (purpose=URL)? | DeleteSurroundingText? |
|---|---|---|
| Chromium **Wayland** `--enable-wayland-ime --wayland-text-input-version=3` | **Có** | **NUỐT** — xoá không ăn, không xác nhận |
| Chromium Xwayland/X11 (GTK IM) | Không | Có (GTK áp chuẩn) |
| Firefox (GTK, mọi đường) | Không | Có |
| GTK app (zenity, gnome-text-editor) | Có (editor khai purpose=0) | Có, xác nhận qua surrounding |

Hệ quả và cách Onikey xử lý (đều đã kiểm chứng bằng log + ảnh màn hình):

1. **ibus-daemon gửi ContentType TRƯỚC FocusIn** → không được quên purpose vô
   điều kiện ở FocusIn; chỉ quên khi không có ContentType nào đến quanh đó
   (cờ stale + cửa sổ thời gian).
2. **Chromium churn**: quanh MỖI lần engine ghi/xoá, Chromium bắn chuỗi
   blur–refocus + Reset + purpose nhấp nháy 5→0→5. Engine nhận biết churn
   (phím gõ hoặc ContentType vừa đến < 300ms) và giữ nguyên trạng thái;
   purpose đến giữa từ thì treo lại, áp khi hết từ.
3. **Ô nuốt lệnh xoá**: bằng chứng lấy từ surrounding text — nếu chuỗi
   đáng-lẽ-bị-xoá vẫn đứng trước chuỗi vừa ghi thì ô bị đánh dấu hỏng, engine
   xoá bù bằng ForwardKeyEvent BackSpace và về Pre-edit đến khi đổi ô. Cờ này
   sống qua churn, chỉ reset khi đổi ô thật.

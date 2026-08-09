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
| Zalo Desktop (Electron 22) | **`0x9`** | `0` | Pre-edit | **Không hỗ trợ surrounding text** → các chế độ không gạch chân sẽ gõ lỗi trong Zalo |

## Rút ra

1. **Mặc định Pre-edit là lựa chọn đúng.** Ứng dụng Electron cũ (Zalo) không có
   surrounding text; ép chế độ không gạch chân ở đó là hỏng chữ.
2. **Chỉ bỏ gạch chân khi ứng dụng thật sự hỗ trợ.** Onikey kiểm tra bit
   surrounding text trước khi đổi chế độ; thiếu thì giữ Pre-edit (xem
   `updateNoUnderlineMode`). Từng có bản lùi về *Forward as commit* khi thiếu
   bit này và hậu quả là ô địa chỉ Edge nuốt phím.
3. **Không suy đoán theo tên ứng dụng.** Trên GNOME Wayland không lấy được
   WM_CLASS; mọi quyết định phải dựa vào cái ứng dụng tự khai báo.

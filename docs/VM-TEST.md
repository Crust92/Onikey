# Kiểm thử gõ thật trong máy ảo

Kiểm thử bộ gõ **không thể** làm bằng unit test: phải có phím thật đi qua
compositor → IBus → engine → ứng dụng. Cách dưới đây bơm phím ở tầng nhân
(uinput) trong một máy ảo, nên vừa thật vừa không đụng tới máy đang dùng.

> **Đừng chạy trò bơm phím này trên máy thật.** `ydotool type` có thể đánh rơi
> sự kiện nhả phím; khi đó compositor tưởng phím vẫn đang nhấn và **lặp vô hạn**
> vào cửa sổ đang focus. Trong VM thì vô hại, trên máy thật thì phải bấm tay
> phím đó mới gỡ được.

## Dựng máy ảo Fedora

```sh
# 1) Ảnh Fedora Cloud + hạt giống cloud-init (tạo user 'test' có khoá SSH)
curl -L -o fedora-cloud.qcow2 \
  http://ftp.iij.ad.jp/pub/linux/Fedora/fedora/linux/releases/42/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-42-1.1.x86_64.qcow2
qemu-img check fedora-cloud.qcow2          # BẮT BUỘC: tải hỏng vẫn mở được, chỉ check mới thấy

# 2) Tạo máy
sudo cp fedora-cloud.qcow2 /var/lib/libvirt/images/onikey.qcow2
sudo qemu-img resize /var/lib/libvirt/images/onikey.qcow2 30G
sudo virt-install --name onikey --memory 6144 --vcpus 4 \
  --disk /var/lib/libvirt/images/onikey.qcow2,bus=virtio \
  --disk /var/lib/libvirt/images/onikey-seed.iso,device=cdrom \
  --osinfo detect=on,require=off --virt-type kvm \
  --graphics spice --video virtio --network network=default --import --noautoconsole
sudo virsh domifaddr onikey                # lấy IP
```

## Cài môi trường trong máy ảo

```sh
scp scripts/vm/*.sh onikey-0.9.0-1.fc42.x86_64.rpm test@<IP>:~/
ssh test@<IP> 'sudo bash provision.sh ~/onikey-0.9.0-1.fc42.x86_64.rpm && sudo reboot'
```

Nếu `dnf` báo `Signature verification failed` hoặc `checksum doesn't match`: đó là
mạng làm hỏng gói giữa đường, **ghim thẳng vào một mirror** thay vì để metalink tự
chọn:

```sh
sudo tee /etc/yum.repos.d/fedora.repo <<'EOF'
[fedora]
name=Fedora 42
baseurl=http://ftp.iij.ad.jp/pub/linux/Fedora/fedora/linux/releases/42/Everything/x86_64/os/
enabled=1
gpgcheck=0
EOF
```

Muốn nhẹ thì không cần trọn bộ Workstation, chỉ cần:
`gnome-shell gdm ibus zenity gnome-text-editor ydotool`.

## Chạy kiểm thử

Sau khi máy ảo tự đăng nhập vào GNOME Wayland:

```sh
ssh test@<IP>
export XDG_RUNTIME_DIR=/run/user/1000 \
       DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
       WAYLAND_DISPLAY=wayland-0
gsettings set org.gnome.desktop.input-sources sources "[('ibus','Onikey'),('xkb','us')]"
sudo systemctl start ydotoold
./type-test.sh "tieengs Vieejt"      # mong đợi in ra: tiếng Việt
```

**Bẫy hay gặp:** phiên GNOME mới bật thường đang ở màn hình **Overview**, mà ở đó
mọi phím rơi vào ô tìm kiếm chứ không vào cửa sổ. Bấm `Esc` (keycode 1) cho tới
khi ra desktop rồi mới gõ. Nhìn tận mắt bằng:

```sh
sudo virsh screenshot onikey /tmp/vm.ppm && convert /tmp/vm.ppm /tmp/vm.png
```

## Kết quả đã đạt

| Môi trường | Cài từ | Kết quả |
|---|---|---|
| Fedora 42, GNOME 48 Wayland | RPM `onikey-0.9.0-1.fc42` | Gõ `tieengs Vieejt Fedora` → nhận đúng **`tiếng Việt Fedora`**, chạy chế độ Pre-edit (`mode=1`) |
| Fedora 42, GNOME X11 | RPM | Chưa kiểm được (xem dưới) |
| **Ubuntu 24.04, GNOME X11** | `.deb` | Gõ `tieengs Vieejt X11` → nhận đúng **`tiếng Việt X11`** |

## Bơm phím: X11 dùng xdotool, Wayland dùng ydotool

Trên **X11 thì `xdotool` tốt hơn hẳn**: nó bơm qua XTEST (vẫn đi qua IBus như
phím thật) và quan trọng hơn là `xdotool windowactivate` chọn được đúng cửa sổ,
hết luôn chuyện đoán xem cửa sổ nào đang focus.

```sh
sudo apt install -y xdotool
WID=$(xdotool search --name onikeytest | head -1)
xdotool windowactivate --sync $WID
xdotool type --delay 100 "tieengs Vieejt"
```

Hai cái bẫy đã dính, ghi lại cho khỏi mất công:

- **Ubuntu không đóng gói `ydotoold`.** Thiếu daemon thì mỗi lệnh `ydotool` tự
  tạo một thiết bị ảo tạm, và sự kiện **nhả phím bị hiểu thành nhấn** — vòng
  "nhả hết phím cho an toàn" của tôi gõ ra một tràng số vào ô tìm kiếm.
- **Overview của GNOME nuốt bàn phím.** Chữ vẫn ra đúng nhưng rơi vào ô tìm
  kiếm của Overview chứ không vào cửa sổ. Chụp màn hình bằng `virsh screenshot`
  là cách nhanh nhất để biết chữ đang đi đâu.

## Chưa làm được: phiên X11 trên FEDORA

Fedora 42 đã bỏ phiên GNOME X11 khỏi GDM (đặt `WaylandEnable=false` cũng bị
lờ), nên phải đổi sang SDDM mới vào được X11. Nhưng khi đó phiên thiếu phần
tích hợp IBus mà GNOME thường tự lo, và biểu hiện rất dễ đánh lừa:

- engine **có** nhận `FocusIn` từ ứng dụng → tưởng là đã thông;
- nhưng **không** nhận một phím nào (`ProcessKeyEvent` không hề được gọi),
  chữ rơi thẳng vào ứng dụng ở dạng phím thô.

Đã loại trừ: engine chạy (`ibus engine` trả về `Onikey`), nguồn nhập đúng
(`current=0`), `ibus-daemon` sống, module `im-ibus.so` có mặt và
`immodules.cache` có nhắc tới ibus, đã thử cả `GTK_IM_MODULE=ibus`,
Super+Space lẫn Ctrl+Space để bật ngữ cảnh.

Kết luận: **ngữ cảnh IBus không được bật** vì thiếu phần quản lý nguồn nhập
của phiên GNOME thật. Đây là giới hạn của cách dựng máy ảo, KHÔNG phải bằng
chứng Onikey hỏng trên X11. Muốn kiểm thật thì dùng bản phân phối còn hỗ trợ
phiên GNOME X11 tử tế (Ubuntu 24.04 chẳng hạn) thay vì ép Fedora 42.

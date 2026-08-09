#!/bin/bash
# Chạy TRONG VM Fedora (qua SSH, quyền sudo). Dựng phiên GNOME Wayland tự đăng
# nhập + cài Onikey từ RPM + bộ công cụ để bơm phím kiểm thử.
set -euo pipefail

RPM="${1:-/home/test/onikey.rpm}"

echo "==> Cài môi trường GNOME (mất vài phút)"
sudo dnf -y -q group install workstation-product-environment >/dev/null

echo "==> Cài ibus + công cụ kiểm thử"
sudo dnf -y -q install ibus ydotool zenity gnome-text-editor >/dev/null

echo "==> Cài Onikey"
sudo dnf -y -q install "$RPM"
rpm -q onikey
ls -l /usr/libexec/onikey/ /usr/share/ibus/component/onikey.xml

echo "==> Bật GDM tự đăng nhập (Wayland)"
sudo mkdir -p /etc/gdm
sudo tee /etc/gdm/custom.conf >/dev/null <<'EOF'
[daemon]
WaylandEnable=true
AutomaticLoginEnable=true
AutomaticLogin=test
EOF
sudo systemctl set-default graphical.target

echo "==> Cho phép ydotool chạy nền như dịch vụ hệ thống"
sudo tee /etc/systemd/system/ydotoold.service >/dev/null <<'EOF'
[Unit]
Description=ydotool daemon
[Service]
ExecStart=/usr/bin/ydotoold
Restart=always
[Install]
WantedBy=multi-user.target
EOF
sudo systemctl enable ydotoold >/dev/null

echo "==> Xong. Khởi động lại vào phiên đồ hoạ."

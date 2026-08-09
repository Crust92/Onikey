#!/bin/bash
# Chạy TRONG VM. Gõ một chuỗi phím THẬT (uinput) vào một hộp nhập GTK rồi in ra
# thứ ứng dụng nhận được — kiểm thử bộ gõ đầu-cuối.
#
#   ./type-test.sh "tieengs Vieejt"   ->  in ra "tiếng Việt" nếu đúng
#
# Luôn NHẢ HẾT phím ở cuối: sự cố kẹt phím lặp vô hạn từng xảy ra khi ydotool
# đánh rơi sự kiện nhả.
set -uo pipefail

KEYS="${1:-tieengs Vieejt}"
DELAY="${2:-80}"

UID_=$(id -u)
export XDG_RUNTIME_DIR="/run/user/${UID_}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export DISPLAY="${DISPLAY:-:0}"

OUT=$(mktemp)
setsid zenity --entry --title="onikey-test" --text="gõ thử" > "$OUT" 2>/dev/null &
ZPID=$!
sleep 3

sudo ydotool type --key-delay "$DELAY" "$KEYS"
sleep 1
sudo ydotool key 28:1 28:0     # Enter -> zenity in ra nội dung
sleep 1

# nhả hết mọi phím có thể còn kẹt (chữ, số, modifier)
for k in $(seq 1 58) 96 97 100 125 126; do sudo ydotool key ${k}:0 >/dev/null 2>&1; done

kill "$ZPID" 2>/dev/null
printf 'NHẬN ĐƯỢC: %s\n' "$(cat "$OUT")"
rm -f "$OUT"

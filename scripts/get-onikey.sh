#!/bin/sh
# get-onikey.sh — cài Onikey nhanh nhất: tải bản binary mới nhất từ GitHub
# Releases, kiểm tra checksum rồi cài.
#
#   curl -fsSL https://raw.githubusercontent.com/Crust92/Onikey/master/scripts/get-onikey.sh | sh
#
# Không cần trình biên dịch, không cần thêm kho phần mềm. Cần: curl, tar, sudo
# (hoặc dùng ONIKEY_PREFIX=~/.local để cài riêng cho mình, khỏi root).
#
# Biến môi trường:
#   ONIKEY_VERSION=v1.0.1     cài đúng bản đó thay vì bản mới nhất
#   ONIKEY_PREFIX=~/.local    cài cho riêng người dùng, không cần quyền root
set -eu

REPO=Crust92/Onikey
PREFIX="${ONIKEY_PREFIX:-/usr}"

say() { printf '%s\n' "$*"; }
die() { printf 'Lỗi: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null || die "cần curl"
command -v tar  >/dev/null || die "cần tar"

ARCH=$(uname -m)
case "$ARCH" in
  x86_64|aarch64) ;;
  *) die "chưa có bản dựng sẵn cho kiến trúc $ARCH — cài từ mã nguồn theo README" ;;
esac

VERSION="${ONIKEY_VERSION:-}"
if [ -z "$VERSION" ]; then
  say "==> Hỏi GitHub bản mới nhất"
  VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
            sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)
  [ -n "$VERSION" ] || die "không đọc được bản mới nhất (GitHub chặn hay hết hạn mức?)"
fi
NUM=${VERSION#v}
FILE="onikey-$NUM-linux-$ARCH.tar.gz"
BASE="https://github.com/$REPO/releases/download/$VERSION"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

say "==> Tải $FILE ($VERSION)"
curl -fsSL "$BASE/$FILE"        -o "$TMP/$FILE" || die "không tải được $BASE/$FILE"
curl -fsSL "$BASE/$FILE.sha256" -o "$TMP/$FILE.sha256" ||
  die "không tải được checksum — dừng, không cài thứ chưa kiểm chứng"

say "==> Kiểm checksum"
( cd "$TMP" && sha256sum -c "$FILE.sha256" >/dev/null 2>&1 ) ||
  die "checksum KHÔNG khớp — tệp tải về hỏng hoặc bị can thiệp, không cài"

tar -xzf "$TMP/$FILE" -C "$TMP"
DIR="$TMP/onikey-$NUM-linux-$ARCH"

if [ "$PREFIX" = "/usr" ] && [ "$(id -u)" != 0 ]; then
  command -v sudo >/dev/null || die "cần sudo, hoặc đặt ONIKEY_PREFIX=~/.local"
  say "==> Cài vào /usr (cần sudo)"
  sudo sh "$DIR/install.sh"
else
  say "==> Cài vào $PREFIX"
  sh "$DIR/install.sh" --prefix "$PREFIX"
fi

Onikey — Vietnamese input method for Linux, Rust engine
=======================================================
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://opensource.org/licenses/GPL-3.0)

**Onikey 1.0** is a Vietnamese IME for IBus. It started as a fork of
[ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo) and now runs on a
**completely rewritten Rust engine** that preserves the original typing
behavior character-for-character. All credit for the Vietnamese input logic
belongs to the BambooEngine authors; this repo stays **GPLv3**.

## Technical highlights

- **Proven behavioral parity.** A corpus of **126,831 test cases** generated
  from the original Go engine records the Vietnamese string **after every
  keystroke** (not just the final result), plus raw-key restore, syllable
  validity, and backspace/restore state — across 9 input methods and 4 flag
  combinations. The Rust core must match exactly; CI runs it on every push.
  Charset tables (TCVN3, VNI Windows, VISCII…) are machine-generated from the
  original and verified against 134,521 cases.
- **Fixes a long-standing IBus blind spot.** Since IBus 1.5 the input-field
  type (URL/email/password) arrives as the DBus **property** `ContentType`,
  not the `SetContentType` method. The original's DBus layer never implemented
  properties, so the engine never saw it. The Rust engine (zbus) implements
  `Properties.Set` from day one.
- **Fast typing without tone glitches** in underline-free mode: after
  deleting committed text the engine **waits for the app to acknowledge**
  (via surrounding-text) before writing the replacement — no more
  `password` → `passsowrd` under load.
- **English auto-restore**: `expression` typed mid-sentence comes out intact,
  with the `dd` → `đ` abbreviation exception preserved.
- **Instant-apply menu & hot config reload**: switching input method or
  charset from the panel menu rebuilds the core immediately; config file
  changes are picked up by mtime — no `ibus restart`.
- **The engine cannot die with the GUI**: the settings dialog is a separate
  process; the engine links against libc only.
- **Capability-aware mode selection**: underline-free mode engages only when
  the app actually provides surrounding text (measured per-app in
  [docs/APP-COMPAT.md](docs/APP-COMPAT.md)); otherwise Pre-edit — an
  underline beats swallowed keys.
- **Multi-distro packaging, actually tested**: proper `PREFIX`/`LIBEXECDIR`,
  component XML generated for the install location; build+install verified in
  Fedora/Arch/Debian containers and real typing verified in VMs (GNOME
  Wayland, GNOME X11, KDE Plasma) with kernel-level key injection
  ([docs/VM-TEST.md](docs/VM-TEST.md)).
- **A path forward**: the core exports a C ABI (`libonikey_core` +
  `onikey.h`, exercised by a real C program in CI) as the foundation for
  Fcitx5/XIM adapters ([docs/ROADMAP.md](docs/ROADMAP.md)).

## Architecture

```
rust/onikey-core      pure Vietnamese core (no I/O, zero dependencies)
rust/onikey-core-ffi  C ABI: libonikey_core.{a,so} + include/onikey.h
rust/onikey-ibus      IBus engine (zbus) — binary onikey-engine-rs, engine name "Onikey"
*.go                  legacy Go engine — engine name "OnikeyGo", kept as fallback
```

## Install (recommended — future `apt upgrade` keeps you current)

```sh
curl -fsSL https://xtcrust.github.io/Onikey/onikey-archive-keyring.gpg \
  | sudo tee /usr/share/keyrings/onikey-archive-keyring.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/onikey-archive-keyring.gpg] https://xtcrust.github.io/Onikey stable main" \
  | sudo tee /etc/apt/sources.list.d/onikey.list
sudo apt update && sudo apt install onikey     # or fcitx5-onikey
```

## Build & install

Requirements (Ubuntu/Debian): `golang cargo gcc make pkg-config libgtk-3-dev libxtst-dev libx11-dev`.

```sh
git clone https://github.com/xtcrust/Onikey.git
cd Onikey
make                       # build as regular user
sudo make install PREFIX=/usr
ibus restart
```

Pick **Onikey** under *Settings → Keyboard → Input Sources*. Uninstall with
`sudo make uninstall`. Non-`/usr` installs: pass `PREFIX=`, on Fedora add
`LIBEXECDIR=/usr/libexec/onikey`.

## Debugging

`touch ~/.config/onikey/onikey-debug && ibus restart` — the engine logs its
loaded config and every key decision to `~/.config/onikey/onikey-rust-debug.log`.

## License

GPLv3, same as upstream [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)
and [bamboo-core](https://github.com/BambooEngine/bamboo-core). See [LICENSE](LICENSE).

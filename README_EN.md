Onikey — Underline-free Vietnamese input for GNOME Wayland
==========================================================
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://opensource.org/licenses/GPL-3.0)

**Onikey** is a **fork of [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)** (BambooEngine), tuned to type Vietnamese **without the preedit underline** on **GNOME Wayland**, plus reliability fixes. All core credit goes to the original authors; this repo keeps the **GPLv3** license.

> Onikey keeps the internal engine name **Bamboo** — in *Settings → Keyboard → Input Sources* you select the engine named **Bamboo**.

## What this fork changes
1. **No underline by default** — defaults to *Surrounding Text* (direct commit) instead of *Pre-edit*.
2. **Crash fix on app switch** — `x11GetFocusWindowClass()` lacked a NULL `Display` check, segfaulting when focusing a native-Wayland app and killing input system-wide. Added the null-check and skip X11 introspection on Wayland.
3. **Setup-GUI panic fix** when the macro file is missing (standalone launch).
4. **Event-based surrounding-text sync** — after `DeleteSurroundingText`, wait for the app to confirm before committing (capped timeout + adaptive fallback), so tone correction adapts to app/system lag.

## Build & install from source
Requirements (Ubuntu/Debian): `golang git make gcc libgtk-3-dev libxtst-dev libx11-dev`.
```sh
git clone https://github.com/xtcnet/Onikey.git
cd Onikey
make
sudo make install PREFIX=/usr
ibus restart
```
Then add the **Bamboo** engine under *Settings → Keyboard → Input Sources*. Switch English/Vietnamese with <kbd>Super</kbd>+<kbd>Space</kbd>. Cycle input modes with <kbd>Shift</kbd>+<kbd>~</kbd>. Uninstall with `sudo make uninstall`.

## Known GNOME Wayland limits
Platform limitations, not engine bugs: no focused-window detection (`Shell.Eval` is locked; native-Wayland apps have no X WM_CLASS); no synthetic key injection (XTest is blocked by Wayland — triggers the *Remote Desktop* prompt); Wine apps don't support surrounding text (characters get doubled). The underline-free approach trades perfect lag-immunity for no underline; use Pre-edit mode if you need the former.

## License
GPLv3, same as the upstream [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo). See [LICENSE](LICENSE).

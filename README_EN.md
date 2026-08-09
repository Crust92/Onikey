Onikey — Underline-free Vietnamese input for GNOME Wayland
==========================================================
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://opensource.org/licenses/GPL-3.0)

**Onikey** is a **fork of [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)** (BambooEngine), tuned to type Vietnamese **without the preedit underline** on **GNOME Wayland**, plus reliability fixes. All core credit goes to the original authors; this repo keeps the **GPLv3** license.

> Since 0.9.0 the engine is named **Onikey** (it used to be *Bamboo*) — pick **Onikey** in *Settings → Keyboard → Input Sources*. An existing `~/.config/ibus-bamboo` config is migrated to `~/.config/onikey` on first run.

## What this fork changes
1. **Per-field hybrid mode** — regular fields use *Pre-edit* (underlined, most reliable under lag), while the **browser address bar automatically switches to an underline-free mode** so URL suggestions keep working. Toggle it in the setup dialog.
2. **Reads the client's content type** — since IBus 1.5 the input-field type (URL/email/password…) is delivered as the **DBus property `ContentType`**, not via the `SetContentType` method; `goibus` implements no DBus properties, so upstream ibus-bamboo never received it. Onikey adds an `org.freedesktop.DBus.Properties.Set` handler, which is what makes the hybrid mode possible.
3. **Crash fix on app switch** — `x11GetFocusWindowClass()` lacked a NULL `Display` check, segfaulting when focusing a native-Wayland app and killing input system-wide. Added the null-check and skip X11 introspection on Wayland.
4. **Setup-GUI panic fix** when the macro file is missing (standalone launch).
5. **Event-based surrounding-text sync** — after `DeleteSurroundingText`, wait for the app to confirm before committing (capped timeout + adaptive fallback), so tone correction adapts to app/system lag.

## Build & install from source
Requirements (Ubuntu/Debian): `golang git make gcc libgtk-3-dev libxtst-dev libx11-dev`.
```sh
git clone https://github.com/XTCRust/Onikey.git
cd Onikey
make
sudo make install PREFIX=/usr
ibus restart
```
Then add the **Onikey** engine under *Settings → Keyboard → Input Sources*. Switch English/Vietnamese with <kbd>Super</kbd>+<kbd>Space</kbd>. Cycle input modes with <kbd>Shift</kbd>+<kbd>~</kbd>. Uninstall with `sudo make uninstall`.

## Known GNOME Wayland limits
Platform limitations, not engine bugs: no focused-window detection (`Shell.Eval` is locked; native-Wayland apps have no X WM_CLASS); no synthetic key injection (XTest is blocked by Wayland — triggers the *Remote Desktop* prompt); Wine apps don't support surrounding text (characters get doubled). The underline-free approach trades perfect lag-immunity for no underline; use Pre-edit mode if you need the former. The hybrid mode also can't help Firefox's own address bar: Chromium reports it as a URL field, Firefox only reports content types for in-page inputs.

## Debugging
The engine is spawned by ibus-daemon, so its stdout is invisible. Enable a log file with `touch ~/.config/onikey/onikey-debug && ibus restart`; it writes focus, capability, content-type and input-mode events to `~/.config/onikey/onikey-debug.log`. Remove the flag file and restart ibus to turn it off.

## License
GPLv3, same as the upstream [ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo). See [LICENSE](LICENSE).

# mic-toggle

[![CI](https://github.com/mnismi/mic-toggle/actions/workflows/ci.yml/badge.svg)](https://github.com/mnismi/mic-toggle/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Press **F8** (configurable, see below) to mute/unmute the default microphone. A tray icon shows the
state (green = live, red = muted); a low beep means muted, a higher beep
means unmuted. Right-click the tray icon → **Exit** to quit.

Single native exe — no .NET runtime, no AutoHotkey. Windows 10/11.

## Install

Download [`mic-toggle.exe`](https://github.com/mnismi/mic-toggle/releases/download/latest/mic-toggle.exe)
from the [latest release](https://github.com/mnismi/mic-toggle/releases/tag/latest)
and put it anywhere you like, then run it. No installer, no dependencies.

> **Note:** the exe is not code-signed, so Windows SmartScreen may warn on
> first run. Click *More info* → *Run anyway*, or build from source below.

### Start with Windows

Enabled by default: the first launch registers the app to start when you
log in (a per-user registry entry — no admin rights needed). Toggle it
anytime via right-click on the tray icon → **Start with Windows**; your
choice is remembered.

## Custom hotkey (optional)

Create `mic-toggle.toml` next to `mic-toggle.exe`:

    hotkey = "Ctrl+Shift+F8"

Format: optional modifiers (`Ctrl`, `Alt`, `Shift`, `Win`) joined with `+`,
then one key — `A`–`Z`, `0`–`9`, `F1`–`F24`, or `Space`, `Tab`, `Pause`,
`ScrollLock`, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`,
`Backspace`. Case doesn't matter. Restart the app after editing.

No file (or no `hotkey` line that parses) means the default **F8**; an
invalid value shows a warning and falls back to F8.

## Build from source

Requires stable [Rust](https://rustup.rs/) on Windows (MSVC toolchain).

    cargo build --release

The exe lands at `target\release\mic-toggle.exe`. Copy it anywhere you like.

## Notes

- Only the *default* capture device is toggled.
- The tray icon syncs with the real device state at startup and on every
  hotkey press; if you mute the mic in another app, the icon catches up on
  the next press.
- Launching a second copy does nothing (single-instance).
- If you move `mic-toggle.exe`, the start-with-Windows entry updates to
  the new location the next time you run it.

## Contributing

Bug reports and pull requests are welcome — see
[CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)

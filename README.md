# mic-toggle

Press **F8** (configurable, see below) to mute/unmute the default microphone. A tray icon shows the
state (green = live, red = muted); a low beep means muted, a higher beep
means unmuted. Right-click the tray icon → **Exit** to quit.

Single native exe — no .NET runtime, no AutoHotkey.

## Custom hotkey (optional)

Create `mic-toggle.toml` next to `mic-toggle.exe`:

    hotkey = "Ctrl+Shift+F8"

Format: optional modifiers (`Ctrl`, `Alt`, `Shift`, `Win`) joined with `+`,
then one key — `A`–`Z`, `0`–`9`, `F1`–`F24`, or `Space`, `Tab`, `Pause`,
`ScrollLock`, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`,
`Backspace`. Case doesn't matter. Restart the app after editing.

No file (or no `hotkey` line that parses) means the default **F8**; an
invalid value shows a warning and falls back to F8.

## Build

    cargo build --release

The exe lands at `target\release\mic-toggle.exe`. Copy it anywhere you like.

## Start with Windows (optional)

Press `Win+R`, run `shell:startup`, and drop a shortcut to
`mic-toggle.exe` in the folder that opens.

## Notes

- Only the *default* capture device is toggled.
- The tray icon syncs with the real device state at startup and on every
  F8 press; if you mute the mic in another app, the icon catches up on the
  next press.
- Launching a second copy does nothing (single-instance).

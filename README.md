# mic-toggle

Press **F8** to mute/unmute the default microphone. A tray icon shows the
state (green = live, red = muted); a low beep means muted, a higher beep
means unmuted. Right-click the tray icon → **Exit** to quit.

Single native exe — no .NET runtime, no AutoHotkey.

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

# mic-toggle — Design

**Date:** 2026-07-26
**Status:** Approved

## Purpose

A small background Rust app for Windows that toggles mute on the default
microphone when F8 is pressed. Replaces the previous setup of an AutoHotkey
script launching a .NET `MicMute.exe` (which required the .NET runtime and
NAudio DLLs). The result is one native `.exe` with no runtime dependencies.

## Requirements

- Global **F8** hotkey: press to mute the default capture device, press again
  to unmute.
- Runs in the background with no console window.
- Feedback on each toggle:
  - **Sound cue:** low beep (400 Hz, ~150 ms) on mute, higher beep (800 Hz,
    ~150 ms) on unmute.
  - **Tray icon:** shows current state (normal vs. muted-red), right-click
    menu with **Exit**.
- Default microphone only (not all capture devices).
- Single instance: launching the exe twice must not register F8 twice.

## Approach

Plain Win32 via the official `windows` crate (single dependency). A classic
Win32 message loop drives everything:

- `RegisterHotKey` for F8 (`WM_HOTKEY` in the message loop).
- Core Audio: `IMMDeviceEnumerator` → default capture endpoint →
  `IAudioEndpointVolume::SetMute` / `GetMute`.
- `Shell_NotifyIcon` for the tray icon; `TrackPopupMenu` for the
  right-click Exit menu.
- `Beep()` for sound cues (called off the toggle path or with short duration
  so the message loop isn't blocked noticeably).

The alternative (ecosystem crates: `global-hotkey`, `tray-icon`, `tao`) was
rejected: much larger dependency tree and a Tauri-oriented event loop for
what is ~300 lines of Win32 calls.

## Behavior details

- **Startup:** read the mic's current mute state and set the tray icon to
  match, so the icon is truthful even if the mic was muted elsewhere.
- **Toggle:** acquire the default capture endpoint fresh on every F8 press,
  so a changed default device just works. Flip mute, update tray icon, beep.
- **External changes:** the icon syncs on startup and on every toggle; it
  does not poll for changes made in other apps between presses. (Accepted
  limitation — next F8 press re-syncs from the real state by reading before
  flipping.)
- **Icons:** two embedded icons (normal / muted-red), compiled into the exe
  as resources.
- **Single instance:** named mutex (`CreateMutexW`); if it already exists,
  exit silently.
- **Errors:**
  - Startup: if hotkey registration or the audio device fails, show a
    `MessageBoxW` with the error and exit.
  - Runtime: if a toggle fails (e.g., no capture device present), skip the
    beep and leave the icon unchanged; no crash.

## Structure

- `main.rs` — entry point, single-instance guard, message loop.
- `audio.rs` — Core Audio wrapper: get default capture device, get/set mute.
- `tray.rs` — tray icon add/update/remove, context menu.
- `hotkey.rs` — register/unregister F8.

Unsafe Win32 plumbing stays isolated inside these modules; `main.rs`
orchestrates through safe functions.

## Build & deployment

- `#![windows_subsystem = "windows"]` — no console window.
- `cargo build --release` → single exe.
- Optional autostart: user drops a shortcut in `shell:startup`.

## Testing

- Unit-testable surface is thin (thin wrappers over OS APIs); primary
  verification is manual:
  - F8 toggles mute (verify in Windows Sound settings).
  - Beeps and tray icon match state.
  - Second launch does nothing (single instance).
  - Exit via tray menu unregisters hotkey and removes icon.

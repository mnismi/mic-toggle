# mic-toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A background Windows exe that toggles mute on the default microphone when F8 is pressed, with a beep + tray-icon state feedback.

**Architecture:** Single-threaded Win32 app driven by a classic message loop. A hidden window receives `WM_HOTKEY` (F8) and tray callbacks. Core Audio (`IAudioEndpointVolume`) flips mute; `Shell_NotifyIcon` shows state; `Beep()` gives audio cues. Unsafe Win32 code is isolated in `audio.rs`, `hotkey.rs`, `tray.rs`; `main.rs` orchestrates.

**Tech Stack:** Rust (edition 2021), single dependency: `windows` crate v0.58 (official Microsoft Win32 bindings).

## Global Constraints

- Project root: `F:\Work\Playground\mic-toggle` (repo already initialized, spec committed).
- Spec: `docs/superpowers/specs/2026-07-26-mic-toggle-design.md`.
- Only dependency: `windows = "0.58"`. No other crates.
- Hotkey: F8, no modifiers, registered with `MOD_NOREPEAT` (holding the key must not auto-repeat toggles).
- Audio target: default capture device, `eCapture` / `eConsole` role. Acquired **fresh on every toggle** (never cached).
- Beeps: 400 Hz on mute, 800 Hz on unmute, 150 ms, played on a spawned thread (never block the message loop).
- Single instance via named mutex `"mic-toggle-single-instance"`; second launch exits silently.
- No console window in normal builds: `#![cfg_attr(not(test), windows_subsystem = "windows")]` (the `cfg_attr` guard keeps test output visible).
- Startup failure (hotkey/audio/tray) → `MessageBoxW` with the error, then exit.
- Tray icons are generated at runtime (filled circle: green `0x3FB950` = live, red `0xD33F3F` = muted); every `HICON` passed to the shell is destroyed after the `Shell_NotifyIcon` call.
- Commit after every task. Expect `dead_code` warnings until Task 5 wires everything together — warnings are OK, errors are not.
- The `windows` crate occasionally shifts signatures between minors. The pin to 0.58 should make the code below compile as-is; if `cargo check` reports a signature mismatch, adapt to the compiler's suggestion without changing behavior.

---

### Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (placeholder)
- Create: `.gitignore`

**Interfaces:**
- Consumes: nothing
- Produces: a compiling cargo project with the `windows` dependency and release profile that later tasks build on

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "mic-toggle"
version = "0.1.0"
edition = "2021"

[dependencies.windows]
version = "0.58"
features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_Media_Audio",
    "Win32_Media_Audio_Endpoints",
    "Win32_System_Com",
    "Win32_System_Diagnostics_Debug",
    "Win32_System_LibraryLoader",
    "Win32_System_Threading",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
]

[profile.release]
opt-level = "z"
lto = true
strip = true
panic = "abort"
```

- [ ] **Step 2: Create placeholder `src/main.rs`**

```rust
#![cfg_attr(not(test), windows_subsystem = "windows")]

fn main() {}
```

- [ ] **Step 3: Create `.gitignore`**

```
/target
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check` (in `F:\Work\Playground\mic-toggle`)
Expected: `Finished` with no errors (first run downloads/compiles the `windows` crate — can take a few minutes).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/main.rs
git commit -m "feat: scaffold mic-toggle cargo project"
```

---

### Task 2: Audio module — get/toggle mute on default mic

**Files:**
- Create: `src/audio.rs`
- Modify: `src/main.rs` (add `mod audio;`)

**Interfaces:**
- Consumes: nothing from other tasks
- Produces:
  - `audio::is_muted() -> windows::core::Result<bool>` — current mute state of the default capture device
  - `audio::toggle_mute() -> windows::core::Result<bool>` — flips mute, returns the NEW state (`true` = now muted)
  - Both require COM initialized on the calling thread (`CoInitializeEx`), which `main.rs` (Task 5) and the tests do themselves.

- [ ] **Step 1: Write the failing test inside `src/audio.rs` with stub implementations**

Create `src/audio.rs`:

```rust
use windows::core::Result;

/// Current mute state of the default capture device.
pub fn is_muted() -> Result<bool> {
    todo!()
}

/// Flip mute on the default capture device; returns the NEW state (true = muted).
pub fn toggle_mute() -> Result<bool> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    // NOTE: this test touches the real default microphone (brief mute blip)
    // and requires a capture device to be present.
    #[test]
    fn toggle_flips_and_restores_mute_state() {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };

        let before = is_muted().expect("read initial state");

        let flipped = toggle_mute().expect("first toggle");
        assert_eq!(flipped, !before, "toggle must flip the state");
        assert_eq!(
            is_muted().unwrap(),
            flipped,
            "device state must match returned state"
        );

        let restored = toggle_mute().expect("second toggle");
        assert_eq!(restored, before, "second toggle must restore original state");
    }
}
```

Add to `src/main.rs` (below the `#![cfg_attr...]` line):

```rust
mod audio;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test toggle_flips -- --nocapture`
Expected: FAIL — panics with `not yet implemented` (the `todo!()`).

- [ ] **Step 3: Implement the audio functions**

Replace the two stubs in `src/audio.rs` with:

```rust
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eCapture, eConsole, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

/// Acquired fresh on every call so a changed default device just works.
fn endpoint_volume() -> Result<IAudioEndpointVolume> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
    }
}

/// Current mute state of the default capture device.
pub fn is_muted() -> Result<bool> {
    unsafe { Ok(endpoint_volume()?.GetMute()?.as_bool()) }
}

/// Flip mute on the default capture device; returns the NEW state (true = muted).
pub fn toggle_mute() -> Result<bool> {
    unsafe {
        let vol = endpoint_volume()?;
        let new_state = !vol.GetMute()?.as_bool();
        vol.SetMute(new_state, std::ptr::null())?;
        Ok(new_state)
    }
}
```

(keep the existing `use windows::core::Result;` line and the `#[cfg(test)]` module)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test toggle_flips -- --nocapture`
Expected: PASS (`test audio::tests::toggle_flips_and_restores_mute_state ... ok`).
Also spot-check: open Windows Settings → Sound → the mic should end in the same state it started.

- [ ] **Step 5: Commit**

```bash
git add src/audio.rs src/main.rs
git commit -m "feat: audio module toggles mute on default capture device"
```

---

### Task 3: Hotkey module — register/unregister F8

**Files:**
- Create: `src/hotkey.rs`
- Modify: `src/main.rs` (add `mod hotkey;`)

**Interfaces:**
- Consumes: nothing from other tasks
- Produces:
  - `hotkey::HOTKEY_ID: i32` — the id arriving in `WM_HOTKEY`'s `wParam`
  - `hotkey::register(hwnd: HWND) -> windows::core::Result<()>`
  - `hotkey::unregister(hwnd: HWND)`

- [ ] **Step 1: Write the failing test with stub implementations**

Create `src/hotkey.rs`:

```rust
use windows::core::Result;
use windows::Win32::Foundation::HWND;

/// Identifier delivered in WM_HOTKEY's wParam.
pub const HOTKEY_ID: i32 = 1;

/// Register F8 (no modifiers, no key-repeat) as a global hotkey for `hwnd`.
pub fn register(hwnd: HWND) -> Result<()> {
    let _ = hwnd;
    todo!()
}

pub fn unregister(hwnd: HWND) {
    let _ = hwnd;
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f8_can_be_registered_and_unregistered() {
        // A null HWND registers a thread-level hotkey — enough to prove
        // registration works without creating a window.
        register(HWND::default()).expect("F8 should be free to register");
        unregister(HWND::default());
        // Re-registering only succeeds if unregister actually released it.
        register(HWND::default()).expect("re-register after unregister");
        unregister(HWND::default());
    }
}
```

Add to `src/main.rs`:

```rust
mod hotkey;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test f8_can_be -- --nocapture`
Expected: FAIL — panics with `not yet implemented`.

(If it fails with "hotkey already registered" instead, another app owns F8 — stop and tell the user.)

- [ ] **Step 3: Implement register/unregister**

Replace the two stubs in `src/hotkey.rs` with:

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VK_F8,
};

/// Register F8 (no modifiers, no key-repeat) as a global hotkey for `hwnd`.
pub fn register(hwnd: HWND) -> Result<()> {
    unsafe { RegisterHotKey(hwnd, HOTKEY_ID, MOD_NOREPEAT, VK_F8.0 as u32) }
}

pub fn unregister(hwnd: HWND) {
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
    }
}
```

(keep the existing `use` lines, `HOTKEY_ID`, and the test module)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test f8_can_be -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/hotkey.rs src/main.rs
git commit -m "feat: hotkey module registers F8 with MOD_NOREPEAT"
```

---

### Task 4: Tray module — state icon and Exit menu

**Files:**
- Create: `src/tray.rs`
- Modify: `src/main.rs` (add `mod tray;`)

**Interfaces:**
- Consumes: nothing from other tasks
- Produces:
  - `tray::WM_TRAYICON: u32` — window message posted for tray events (`WM_APP + 1`)
  - `tray::IDM_EXIT: u32` — command id returned by `show_menu` when Exit is picked
  - `tray::add(hwnd: HWND, muted: bool) -> windows::core::Result<()>`
  - `tray::update(hwnd: HWND, muted: bool)` — best-effort, never fails the caller
  - `tray::remove(hwnd: HWND)`
  - `tray::show_menu(hwnd: HWND) -> u32` — blocks while the menu is open; returns chosen command id or 0

No unit test: every function needs a live window/tray session. Verified by `cargo check` here and manually in Tasks 5–6.

- [ ] **Step 1: Write `src/tray.rs`**

```rust
use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    SetForegroundWindow, TrackPopupMenu, HICON, ICONINFO, MF_STRING, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP,
};

/// Message posted to the window for tray icon events.
pub const WM_TRAYICON: u32 = WM_APP + 1;
/// Command id returned by `show_menu` when Exit is chosen.
pub const IDM_EXIT: u32 = 100;

const TRAY_ID: u32 = 1;
const LIVE_RGB: u32 = 0x003F_B950; // green
const MUTED_RGB: u32 = 0x00D3_3F3F; // red

/// 32x32 icon: filled circle of `rgb`, transparent background.
fn make_icon(rgb: u32) -> Result<HICON> {
    const S: i32 = 32;
    let mut pixels = [0u32; (S * S) as usize]; // BGRA, zero = transparent
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x - 16, y - 16);
            if dx * dx + dy * dy <= 14 * 14 {
                pixels[(y * S + x) as usize] = 0xFF00_0000 | rgb;
            }
        }
    }
    let mask = [0u8; (S * S / 8) as usize];
    unsafe {
        let color = CreateBitmap(S, S, 1, 32, Some(pixels.as_ptr().cast()));
        let mono = CreateBitmap(S, S, 1, 1, Some(mask.as_ptr().cast()));
        let info = ICONINFO {
            fIcon: true.into(),
            hbmMask: mono,
            hbmColor: color,
            ..Default::default()
        };
        let icon = CreateIconIndirect(&info);
        let _ = DeleteObject(color);
        let _ = DeleteObject(mono);
        icon
    }
}

fn notify_data(hwnd: HWND, muted: bool) -> Result<NOTIFYICONDATAW> {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: make_icon(if muted { MUTED_RGB } else { LIVE_RGB })?,
        ..Default::default()
    };
    let tip = if muted {
        "Mic: muted (F8 to unmute)"
    } else {
        "Mic: live (F8 to mute)"
    };
    for (i, u) in tip.encode_utf16().enumerate() {
        nid.szTip[i] = u;
    }
    Ok(nid)
}

pub fn add(hwnd: HWND, muted: bool) -> Result<()> {
    let nid = notify_data(hwnd, muted)?;
    let result = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).ok() };
    unsafe { let _ = DestroyIcon(nid.hIcon); } // shell keeps its own copy
    result
}

/// Best-effort icon/tooltip refresh; a failure just leaves the old icon.
pub fn update(hwnd: HWND, muted: bool) {
    if let Ok(nid) = notify_data(hwnd, muted) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
            let _ = DestroyIcon(nid.hIcon);
        }
    }
}

pub fn remove(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// Show the right-click menu at the cursor; returns the chosen command id
/// (IDM_EXIT) or 0 if dismissed.
pub fn show_menu(hwnd: HWND) -> u32 {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return 0 };
        let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT as usize, w!("Exit"));
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        // Required so the menu closes when clicking elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        cmd.0 as u32
    }
}
```

Add to `src/main.rs`:

```rust
mod tray;
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: success. `dead_code` warnings for the unused tray functions are expected until Task 5.

- [ ] **Step 3: Commit**

```bash
git add src/tray.rs src/main.rs
git commit -m "feat: tray module with runtime-generated state icons and exit menu"
```

---

### Task 5: Main — window, message loop, wiring

**Files:**
- Modify: `src/main.rs` (replace entirely)

**Interfaces:**
- Consumes:
  - `audio::is_muted() -> Result<bool>`, `audio::toggle_mute() -> Result<bool>` (Task 2)
  - `hotkey::HOTKEY_ID`, `hotkey::register(hwnd)`, `hotkey::unregister(hwnd)` (Task 3)
  - `tray::WM_TRAYICON`, `tray::IDM_EXIT`, `tray::add/update/remove/show_menu` (Task 4)
- Produces: the complete application.

- [ ] **Step 1: Replace `src/main.rs` with the full wiring**

```rust
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod audio;
mod hotkey;
mod tray;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Diagnostics::Debug::Beep;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MessageBoxW,
    PostQuitMessage, RegisterClassW, TranslateMessage, CW_USEDEFAULT, MB_ICONERROR, MB_OK,
    MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WM_HOTKEY, WM_RBUTTONUP, WNDCLASSW,
};

const WINDOW_CLASS: PCWSTR = w!("mic-toggle-window");

fn main() {
    unsafe {
        // Single instance: second launch exits silently.
        let _mutex = CreateMutexW(None, true, w!("mic-toggle-single-instance"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        if let Err(e) = run() {
            let text: Vec<u16> = format!("mic-toggle failed to start:\n{e}\0")
                .encode_utf16()
                .collect();
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                w!("mic-toggle"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

unsafe fn run() -> windows::core::Result<()> {
    CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

    let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance,
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    RegisterClassW(&wc);

    // Hidden window: receives WM_HOTKEY and tray callbacks, never shown.
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        WINDOW_CLASS,
        w!("mic-toggle"),
        WINDOW_STYLE(0),
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        0,
        0,
        None,
        None,
        hinstance,
        None,
    )?;

    hotkey::register(hwnd)?;
    tray::add(hwnd, audio::is_muted()?)?;

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    tray::remove(hwnd);
    hotkey::unregister(hwnd);
    Ok(())
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY if wparam.0 as i32 == hotkey::HOTKEY_ID => {
            on_hotkey(hwnd);
            LRESULT(0)
        }
        tray::WM_TRAYICON => {
            if lparam.0 as u32 == WM_RBUTTONUP && tray::show_menu(hwnd) == tray::IDM_EXIT {
                unsafe { PostQuitMessage(0) };
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Toggle failure (e.g. no capture device): skip beep, leave icon unchanged.
fn on_hotkey(hwnd: HWND) {
    if let Ok(muted) = audio::toggle_mute() {
        tray::update(hwnd, muted);
        std::thread::spawn(move || {
            let freq = if muted { 400 } else { 800 };
            unsafe { let _ = Beep(freq, 150); }
        });
    }
}
```

- [ ] **Step 2: Verify everything still compiles and tests pass**

Run: `cargo test`
Expected: both tests pass, no `dead_code` warnings remain.

- [ ] **Step 3: Manual smoke test**

Run: `cargo run` (returns immediately to a detached background app — that's expected with the windows subsystem; if it stays attached, that's fine too).

Verify:
1. A green circle icon appears in the system tray (may be in the overflow flyout).
2. Press F8 → low beep, icon turns red, Windows Settings → Sound shows the mic muted.
3. Press F8 again → higher beep, icon turns green, mic unmuted.
4. Hold F8 down → toggles once, does not machine-gun toggle.
5. Right-click the tray icon → Exit → icon disappears, process ends (check Task Manager for `mic-toggle.exe`).
6. If the mic ends up muted, press F8 once more before exiting to leave it live.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire hotkey, audio, and tray into message loop"
```

---

### Task 6: Release build, single-instance check, README

**Files:**
- Create: `README.md`

**Interfaces:**
- Consumes: the complete app (Task 5)
- Produces: `target/release/mic-toggle.exe` and user docs

- [ ] **Step 1: Build release**

Run: `cargo build --release`
Expected: success; `target\release\mic-toggle.exe` exists (roughly 200–400 KB).

- [ ] **Step 2: Manual release checklist**

1. Double-click (or `start`) `target\release\mic-toggle.exe` → **no console window flashes**, tray icon appears.
2. Launch the exe a second time → nothing happens (no duplicate icon, second process exits — verify in Task Manager).
3. F8 toggles with beeps and icon change, as in Task 5.
4. Tray → Exit cleans up.

- [ ] **Step 3: Write `README.md`**

```markdown
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
```

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage and autostart instructions"
```

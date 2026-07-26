use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    SetForegroundWindow, TrackPopupMenu, HICON, ICONINFO, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_APP,
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
    // The shell keeps its own copy of the icon.
    unsafe {
        let _ = DestroyIcon(nid.hIcon);
    }
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
        let Ok(menu) = CreatePopupMenu() else {
            return 0;
        };
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

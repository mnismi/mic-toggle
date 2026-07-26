#![cfg_attr(not(test), windows_subsystem = "windows")]

mod audio;
mod config;
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
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MessageBoxW, PostQuitMessage,
    RegisterClassW, TranslateMessage, CW_USEDEFAULT, MB_ICONERROR, MB_ICONWARNING, MB_OK, MSG,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WM_HOTKEY, WM_RBUTTONUP, WNDCLASSW,
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

    let hk = match config::load() {
        Ok(Some(hk)) => hk,
        Ok(None) => hotkey::DEFAULT,
        Err(msg) => {
            let text: Vec<u16> = format!("mic-toggle.toml: {msg}\nFalling back to F8.\0")
                .encode_utf16()
                .collect();
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                w!("mic-toggle"),
                MB_OK | MB_ICONWARNING,
            );
            hotkey::DEFAULT
        }
    };
    hotkey::register(hwnd, hk)?;
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
            unsafe {
                let _ = Beep(freq, 150);
            }
        });
    }
}

use crate::config::Hotkey;
use windows::core::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT, VK_F8,
};

/// Identifier delivered in WM_HOTKEY's wParam.
pub const HOTKEY_ID: i32 = 1;

/// Used when there is no config file: plain F8.
pub const DEFAULT: Hotkey = Hotkey {
    modifiers: HOT_KEY_MODIFIERS(0),
    vk: VK_F8.0 as u32,
};

/// Register `hk` (plus no-key-repeat) as a global hotkey for `hwnd`.
pub fn register(hwnd: HWND, hk: Hotkey) -> Result<()> {
    unsafe {
        RegisterHotKey(
            hwnd,
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(hk.modifiers.0 | MOD_NOREPEAT.0),
            hk.vk,
        )
    }
}

pub fn unregister(hwnd: HWND) {
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_hotkey;

    #[test]
    fn default_and_combo_can_be_registered_and_unregistered() {
        // A null HWND registers a thread-level hotkey — enough to prove
        // registration works without creating a window.
        register(HWND::default(), DEFAULT).expect("F8 should be free to register");
        unregister(HWND::default());
        // Re-registering only succeeds if unregister actually released it.
        register(HWND::default(), DEFAULT).expect("re-register after unregister");
        unregister(HWND::default());

        let combo = parse_hotkey("Ctrl+Shift+F8").expect("valid combo");
        register(HWND::default(), combo).expect("Ctrl+Shift+F8 should register");
        unregister(HWND::default());
    }
}

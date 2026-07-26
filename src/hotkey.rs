use windows::core::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VK_F8,
};

/// Identifier delivered in WM_HOTKEY's wParam.
pub const HOTKEY_ID: i32 = 1;

/// Register F8 (no modifiers, no key-repeat) as a global hotkey for `hwnd`.
pub fn register(hwnd: HWND) -> Result<()> {
    unsafe { RegisterHotKey(hwnd, HOTKEY_ID, MOD_NOREPEAT, VK_F8.0 as u32) }
}

pub fn unregister(hwnd: HWND) {
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
    }
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

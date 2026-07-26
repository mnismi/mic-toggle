//! Start-with-Windows via the per-user `Run` registry key (no admin needed).
//!
//! Autostart is on by default: the first launch writes the `Run` entry and a
//! one-time marker under `HKCU\Software\mic-toggle`. Later launches respect
//! whatever the user chose from the tray menu; the marker is what tells a
//! fresh install apart from "the user turned it off".

use std::path::Path;
use windows::core::{w, Result, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_ROUTINE_FLAGS,
    REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_KEY: PCWSTR = w!("Software\\mic-toggle");
const RUN_VALUE: PCWSTR = w!("mic-toggle");
const CONFIGURED_VALUE: PCWSTR = w!("AutostartConfigured");

/// Whether the `Run` entry currently exists.
pub fn is_enabled() -> bool {
    value_exists(RUN_KEY, RUN_VALUE, RRF_RT_REG_SZ)
}

/// Write (or refresh) the `Run` entry pointing at the current exe.
pub fn enable() -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| {
        windows::core::Error::new(windows::core::HRESULT(0), format!("current_exe: {e}"))
    })?;
    set_string_value(RUN_KEY, RUN_VALUE, &run_command(&exe))
}

/// Remove the `Run` entry. Best-effort: a missing value is already "disabled".
pub fn disable() {
    delete_value(RUN_KEY, RUN_VALUE);
}

/// Called once at startup. First launch: turn autostart on and set the
/// marker. Later launches: leave the user's choice alone, but if autostart
/// is on, rewrite the entry so it tracks the exe if it was moved.
pub fn ensure_default() {
    if !value_exists(APP_KEY, CONFIGURED_VALUE, RRF_RT_REG_DWORD) {
        if enable().is_ok() {
            let _ = set_dword_value(APP_KEY, CONFIGURED_VALUE, 1);
        }
    } else if is_enabled() {
        let _ = enable();
    }
}

/// The command stored in the `Run` entry: the exe path, quoted so paths with
/// spaces survive.
fn run_command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

fn value_exists(subkey: PCWSTR, name: PCWSTR, kind: REG_ROUTINE_FLAGS) -> bool {
    unsafe { RegGetValueW(HKEY_CURRENT_USER, subkey, name, kind, None, None, None).is_ok() }
}

fn set_string_value(subkey: PCWSTR, name: PCWSTR, value: &str) -> Result<()> {
    let data: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * 2) };
    with_key(subkey, |key| unsafe {
        RegSetValueExW(key, name, 0, REG_SZ, Some(bytes)).ok()
    })
}

fn set_dword_value(subkey: PCWSTR, name: PCWSTR, value: u32) -> Result<()> {
    with_key(subkey, |key| unsafe {
        RegSetValueExW(key, name, 0, REG_DWORD, Some(&value.to_le_bytes())).ok()
    })
}

fn delete_value(subkey: PCWSTR, name: PCWSTR) {
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_SET_VALUE, &mut key).is_ok() {
            let _ = RegDeleteValueW(key, name);
            let _ = RegCloseKey(key);
        }
    }
}

/// Open-or-create `subkey` with write access, run `f`, close the key.
fn with_key(subkey: PCWSTR, f: impl FnOnce(HKEY) -> Result<()>) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey,
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let result = f(key);
        let _ = RegCloseKey(key);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Registry::RegDeleteKeyW;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Read a string value back; None if missing.
    fn read_string(subkey: PCWSTR, name: PCWSTR) -> Option<String> {
        unsafe {
            let mut buf = [0u16; 512];
            let mut size = (buf.len() * 2) as u32;
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey,
                name,
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
            .is_ok()
            .then(|| {
                let len = (size as usize / 2).saturating_sub(1); // drop the NUL
                String::from_utf16_lossy(&buf[..len])
            })
        }
    }

    fn delete_key(subkey: PCWSTR) {
        unsafe {
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, subkey);
        }
    }

    #[test]
    fn run_command_quotes_path() {
        let cmd = run_command(Path::new(r"C:\Program Files\mic toggle\mic-toggle.exe"));
        assert_eq!(cmd, r#""C:\Program Files\mic toggle\mic-toggle.exe""#);
    }

    #[test]
    fn string_value_roundtrip() {
        let key_name = wide(&format!(
            r"Software\mic-toggle-test-str-{}",
            std::process::id()
        ));
        let key = PCWSTR(key_name.as_ptr());
        let name = w!("TestValue");

        assert!(!value_exists(key, name, RRF_RT_REG_SZ));
        set_string_value(key, name, r#""C:\some path\app.exe""#).unwrap();
        assert!(value_exists(key, name, RRF_RT_REG_SZ));
        assert_eq!(
            read_string(key, name).as_deref(),
            Some(r#""C:\some path\app.exe""#)
        );

        delete_value(key, name);
        assert!(!value_exists(key, name, RRF_RT_REG_SZ));
        delete_key(key);
    }

    #[test]
    fn dword_value_roundtrip() {
        let key_name = wide(&format!(
            r"Software\mic-toggle-test-dword-{}",
            std::process::id()
        ));
        let key = PCWSTR(key_name.as_ptr());
        let name = w!("Configured");

        assert!(!value_exists(key, name, RRF_RT_REG_DWORD));
        set_dword_value(key, name, 1).unwrap();
        assert!(value_exists(key, name, RRF_RT_REG_DWORD));

        delete_value(key, name);
        delete_key(key);
    }

    #[test]
    fn deleting_missing_value_is_harmless() {
        let key_name = wide(&format!(
            r"Software\mic-toggle-test-missing-{}",
            std::process::id()
        ));
        delete_value(PCWSTR(key_name.as_ptr()), w!("Nope"));
    }
}

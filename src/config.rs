//! Optional `mic-toggle.toml` next to the exe: `hotkey = "Ctrl+Shift+F8"`.

use std::path::Path;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: HOT_KEY_MODIFIERS,
    pub vk: u32,
}

/// `Ok(None)`: no config file — use the default. `Err`: file exists but is invalid.
pub fn load() -> Result<Option<Hotkey>, String> {
    let path = match std::env::current_exe() {
        Ok(exe) => exe.with_file_name("mic-toggle.toml"),
        Err(_) => return Ok(None),
    };
    match read_config(&path)? {
        Some(text) => parse_config(&text).map(Some),
        None => Ok(None),
    }
}

/// Read the config file. `Ok(None)`: file does not exist. `Err`: it exists but
/// couldn't be read (e.g. invalid UTF-8, permission denied) — distinct from
/// "missing", since silently falling back to the default would hide a real problem.
fn read_config(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot read config: {e}")),
    }
}

/// Parse the whole config text: find the `hotkey = "..."` line.
fn parse_config(text: &str) -> Result<Hotkey, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("hotkey") {
            if let Some(value) = rest.trim_start().strip_prefix('=') {
                return parse_hotkey(extract_value(value));
            }
        }
    }
    Err("no `hotkey = \"...\"` line found".to_string())
}

/// Strip surrounding quotes; a quoted value may be followed by a comment.
fn extract_value(raw: &str) -> &str {
    let raw = raw.trim();
    if let Some(stripped) = raw.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            return &stripped[..end];
        }
    }
    raw
}

/// Parse a combo like `Ctrl+Shift+F8` (case-insensitive, spaces allowed).
pub fn parse_hotkey(s: &str) -> Result<Hotkey, String> {
    let tokens: Vec<&str> = s.split('+').map(str::trim).collect();
    let (mod_tokens, key_token) = tokens.split_at(tokens.len() - 1);
    let mut modifiers = HOT_KEY_MODIFIERS(0);
    for t in mod_tokens {
        let flag = match t.to_ascii_lowercase().as_str() {
            "ctrl" => MOD_CONTROL,
            "alt" => MOD_ALT,
            "shift" => MOD_SHIFT,
            "win" => MOD_WIN,
            _ => return Err(format!("unknown modifier '{t}'")),
        };
        if modifiers.0 & flag.0 != 0 {
            return Err(format!("duplicate modifier '{t}'"));
        }
        modifiers = HOT_KEY_MODIFIERS(modifiers.0 | flag.0);
    }
    Ok(Hotkey {
        modifiers,
        vk: parse_key(key_token[0])?,
    })
}

fn parse_key(t: &str) -> Result<u32, String> {
    let u = t.to_ascii_uppercase();
    // A-Z and 0-9: virtual-key code equals the ASCII code.
    if u.len() == 1 {
        let c = u.as_bytes()[0];
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            return Ok(c as u32);
        }
    }
    // F1..F24 (VK_F1 = 0x70).
    if let Some(n) = u.strip_prefix('F') {
        if let Ok(n) = n.parse::<u32>() {
            return if (1..=24).contains(&n) {
                Ok(0x6F + n)
            } else {
                Err(format!("unknown key '{t}'"))
            };
        }
    }
    match u.as_str() {
        "SPACE" => Ok(0x20),
        "TAB" => Ok(0x09),
        "PAUSE" => Ok(0x13),
        "SCROLLLOCK" => Ok(0x91),
        "INSERT" => Ok(0x2D),
        "DELETE" => Ok(0x2E),
        "HOME" => Ok(0x24),
        "END" => Ok(0x23),
        "PAGEUP" => Ok(0x21),
        "PAGEDOWN" => Ok(0x22),
        "BACKSPACE" => Ok(0x08),
        "" => Err("missing key".to_string()),
        _ => Err(format!("unknown key '{t}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_f8() {
        let hk = parse_hotkey("F8").unwrap();
        assert_eq!(
            hk,
            Hotkey {
                modifiers: HOT_KEY_MODIFIERS(0),
                vk: 0x77
            }
        );
    }

    #[test]
    fn modifiers_any_order_case_insensitive() {
        let a = parse_hotkey("ctrl+shift+m").unwrap();
        let b = parse_hotkey("SHIFT+CTRL+M").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.modifiers, HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0));
        assert_eq!(a.vk, 'M' as u32);
    }

    #[test]
    fn whitespace_tolerated() {
        let hk = parse_hotkey(" Alt + F12 ").unwrap();
        assert_eq!(hk.modifiers, HOT_KEY_MODIFIERS(MOD_ALT.0));
        assert_eq!(hk.vk, 0x7B);
    }

    #[test]
    fn win_modifier_and_digits() {
        let hk = parse_hotkey("Win+5").unwrap();
        assert_eq!(hk.modifiers, HOT_KEY_MODIFIERS(MOD_WIN.0));
        assert_eq!(hk.vk, '5' as u32);
    }

    #[test]
    fn named_keys() {
        assert_eq!(parse_hotkey("Space").unwrap().vk, 0x20);
        assert_eq!(parse_hotkey("pagedown").unwrap().vk, 0x22);
        assert_eq!(parse_hotkey("ScrollLock").unwrap().vk, 0x91);
    }

    #[test]
    fn f_keys_span_1_to_24() {
        assert_eq!(parse_hotkey("F1").unwrap().vk, 0x70);
        assert_eq!(parse_hotkey("F24").unwrap().vk, 0x87);
        assert!(parse_hotkey("F25").is_err());
        assert!(parse_hotkey("F0").is_err());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_hotkey("").is_err());
        assert!(parse_hotkey("Ctrl+").is_err());
        assert!(parse_hotkey("Ctrl+Ctrl+A").is_err());
        assert!(parse_hotkey("Meta+A").is_err());
        assert!(parse_hotkey("Foo").is_err());
        assert!(parse_hotkey("Ctrl").is_err()); // modifier alone is not a key
    }

    #[test]
    fn config_text_finds_hotkey_line() {
        let hk = parse_config("# comment\nhotkey = \"Ctrl+F8\"\n").unwrap();
        assert_eq!(hk.modifiers, HOT_KEY_MODIFIERS(MOD_CONTROL.0));
        assert_eq!(hk.vk, 0x77);
    }

    #[test]
    fn config_quoted_value_ignores_trailing_comment() {
        let hk = parse_config("hotkey = \"F8\" # my key\n").unwrap();
        assert_eq!(hk.vk, 0x77);
    }

    #[test]
    fn config_without_hotkey_line_is_error() {
        assert!(parse_config("# nothing here\n").is_err());
        assert!(parse_config("").is_err());
    }

    #[test]
    fn config_bom_prefix_is_stripped() {
        let hk = parse_config("\u{feff}hotkey = \"F8\"\n").unwrap();
        assert_eq!(hk.vk, 0x77);
    }

    #[test]
    fn read_config_missing_file_returns_none() {
        let path = std::env::temp_dir().join("mic-toggle-test-missing-config.toml");
        // Guard against a stray leftover from a previous failed run.
        let _ = std::fs::remove_file(&path);
        assert!(read_config(&path).unwrap().is_none());
    }

    #[test]
    fn read_config_invalid_utf8_is_distinct_error() {
        let path = std::env::temp_dir().join(format!(
            "mic-toggle-test-invalid-utf8-{}.toml",
            std::process::id()
        ));
        // 0xFF is never valid at the start of a UTF-8 byte sequence.
        std::fs::write(&path, [0xFF, 0xFE, b'h', b'i']).unwrap();
        let result = read_config(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }
}

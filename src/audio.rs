use windows::core::Result;
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

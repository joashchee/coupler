//! Whether a screen reader is running, so Coupler's own voice can take
//! turns with it rather than talk over it (`lib/screenReader.ts`).
//!
//! Each platform says so its own way (docs/platform-parity.md):
//! - macOS: VoiceOver runs as its own process, `VoiceOver`, only while
//!   it's on.
//! - Windows: `SystemParametersInfoW(SPI_GETSCREENREADER)`, the flag
//!   NVDA, JAWS and Narrator set while they run.
//! - Linux: GNOME's `screen-reader-enabled` setting, or Orca's process.
//!
//! Nothing here is sent anywhere; it's asked again every few seconds.

#[cfg(unix)]
use std::process::{Command, Stdio};

/// Whether a process of exactly this name is running (`pgrep -x`).
#[cfg(unix)]
fn process_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
pub fn running() -> bool {
    process_running("VoiceOver")
}

#[cfg(target_os = "windows")]
pub fn running() -> bool {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, value: *mut c_void, ini: u32) -> i32;
    }
    const SPI_GETSCREENREADER: u32 = 0x0046;
    let mut on: i32 = 0;
    // SAFETY: SPI_GETSCREENREADER writes one BOOL to the pointer given.
    let ok = unsafe { SystemParametersInfoW(SPI_GETSCREENREADER, 0, (&mut on as *mut i32).cast(), 0) };
    ok != 0 && on != 0
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn running() -> bool {
    let gnome = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.a11y.applications", "screen-reader-enabled"])
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false);
    gnome || process_running("orca")
}

#[cfg(not(any(unix, windows)))]
pub fn running() -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn asking_never_fails() {
        // Whatever the answer on this machine, asking returns one.
        let _ = super::running();
    }
}

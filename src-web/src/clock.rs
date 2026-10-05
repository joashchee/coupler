//! The clock the shared modules read (src-tauri/src/clock.rs has the
//! desktop's). `std::time::Instant` panics in a browser, so in WebAssembly
//! it's the page's `performance.now()`, imported (`coupler_now_ms`,
//! src/web/core.ts); in the host's tests, the standard one.

use std::ops::Add;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "coupler")]
extern "C" {
    fn coupler_now_ms() -> f64;
    fn coupler_unix_ms() -> f64;
}

/// Milliseconds since the page started.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Instant(f64);

#[cfg(not(target_arch = "wasm32"))]
fn start() -> std::time::Instant {
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    *START.get_or_init(std::time::Instant::now)
}

impl Instant {
    pub fn now() -> Instant {
        #[cfg(target_arch = "wasm32")]
        // SAFETY: a plain function the page provides, with no arguments.
        return Instant(unsafe { coupler_now_ms() });
        #[cfg(not(target_arch = "wasm32"))]
        Instant(start().elapsed().as_secs_f64() * 1000.0)
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        Duration::from_secs_f64(((self.0 - earlier.0) / 1000.0).max(0.0))
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.saturating_duration_since(earlier)
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(*self)
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, d: Duration) -> Instant {
        Instant(self.0 + d.as_secs_f64() * 1000.0)
    }
}

/// Seconds since 1970, by the computer's clock.
pub fn unix_now() -> u64 {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: as above.
    return (unsafe { coupler_unix_ms() } / 1000.0) as u64;
    #[cfg(not(target_arch = "wasm32"))]
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

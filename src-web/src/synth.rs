//! The web build has no bundled speech engines:
//! characters speak in the browser's own voices. Only the desktop's
//! engine names are here, so `cast.rs` reads a voice that names one the
//! same way on both (src-tauri/src/synth.rs has the engines); the page
//! lists no bundled voices, and `lib/voice.ts` falls back to the
//! browser's for a voice whose engine it doesn't have.

pub struct BundledVoice {
    pub engine: &'static str,
}

pub const VOICES: &[BundledVoice] = &[BundledVoice { engine: "flite" }, BundledVoice { engine: "pocket" }];

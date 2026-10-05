//! Pocket TTS (Kyutai), the Candle port by the Pocket TTS contributors,
//! vendored and trimmed by Coupler: see `README.md` beside this crate.

pub mod conditioners;
pub mod config;
pub mod models;
pub mod modules;
pub mod pause;
pub mod tts_model;
pub mod voice_state;

pub use pause::{ParsedText, PauseMarker, parse_text_with_pauses};
pub use tts_model::TTSModel;
pub use voice_state::ModelState;

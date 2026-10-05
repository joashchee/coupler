//! The clock the pure modules read (`speech.rs`, `daytime.rs`). The
//! desktop's is the standard one; the web build's (`src-web/src/clock.rs`)
//! is `web-time`'s, since `std::time::Instant` panics in a browser.

pub use std::time::Instant;

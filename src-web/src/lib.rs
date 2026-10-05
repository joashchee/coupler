//! **Coupler on the web** (coupler.ansiapps.com):
//! the desktop's own protocol and game logic, compiled to WebAssembly.
//! Every module but `ffi`, `session`, `hooks`, `clock` and `synth` is the
//! desktop's file, included as it is, so the two can't drift; the four
//! here stand in for what a browser does differently.
//!
//! The page (`src/web/`) owns the WebSocket and the storage and calls
//! `Coupler`'s methods through `ffi.rs`; `session.rs` is the desktop's `session.rs`
//! without the socket, the threads, the journal, the hooks database or
//! the pictures.

// The shared modules carry what only the desktop calls.
#![allow(dead_code)]

#[path = "../../src-tauri/src/ambient.rs"]
mod ambient;
#[path = "../../src-tauri/src/ansi.rs"]
mod ansi;
#[path = "../../src-tauri/src/ansi_art.rs"]
mod ansi_art;
#[path = "../../src-tauri/src/cast.rs"]
mod cast;
#[path = "../../src-tauri/src/character.rs"]
mod character;
mod clock;
#[path = "../../src-tauri/src/combat.rs"]
mod combat;
#[path = "../../src-tauri/src/creation.rs"]
mod creation;
#[path = "../../src-tauri/src/daytime.rs"]
mod daytime;
#[path = "../../src-tauri/src/echo.rs"]
mod echo;
mod ffi;
#[path = "../../src-tauri/src/hidden.rs"]
mod hidden;
mod hooks;
#[path = "../../src-tauri/src/mapper.rs"]
mod mapper;
#[path = "../../src-tauri/src/paint.rs"]
mod paint;
#[path = "../../src-tauri/src/painter.rs"]
mod painter;
#[path = "../../src-tauri/src/portrait.rs"]
mod portrait;
#[path = "../../src-tauri/src/ports.rs"]
mod ports;
#[path = "../../src-tauri/src/senses.rs"]
mod senses;
mod session;
#[path = "../../src-tauri/src/speech.rs"]
mod speech;
mod synth;
#[path = "../../src-tauri/src/telnet.rs"]
mod telnet;
#[path = "../../src-tauri/src/trigger.rs"]
mod trigger;
#[path = "../../src-tauri/src/who.rs"]
mod who;

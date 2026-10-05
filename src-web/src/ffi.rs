//! The module's whole interface to the page (src/web/core.ts): a few
//! C-style exports over linear memory, so there's no generated glue.
//!
//! - `coupler_alloc`/`coupler_free`: the page writes its input here.
//! - `coupler_call(ptr, len)`: a JSON command, `{"cmd": "...", ...}`
//!   with the desktop command's arguments; 0 and the JSON result, or 1
//!   and the error's words, in the output buffer.
//! - `coupler_feed(ptr, len)`: a binary frame from the game; 1 and why
//!   when the connection should end.
//! - `coupler_take_outgoing()`: what to send the game, as bytes.
//! - `coupler_out_ptr`/`coupler_out_len`: the output buffer, valid
//!   until the next call.

use std::cell::RefCell;

use serde_json::Value;

use crate::session::Coupler;

thread_local! {
    static APP: RefCell<Option<Coupler>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn put(bytes: Vec<u8>) {
    OUT.with(|o| *o.borrow_mut() = bytes);
}

#[no_mangle]
pub extern "C" fn coupler_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `ptr` and `len` are what `coupler_alloc` gave and was asked for.
#[no_mangle]
pub unsafe extern "C" fn coupler_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn coupler_out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn coupler_out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

/// # Safety
/// `ptr` points at `len` bytes the page wrote.
#[no_mangle]
pub unsafe extern "C" fn coupler_call(ptr: *const u8, len: usize) -> u32 {
    let input = std::slice::from_raw_parts(ptr, len);
    match call(input) {
        Ok(v) => {
            put(v.to_string().into_bytes());
            0
        }
        Err(e) => {
            put(e.into_bytes());
            1
        }
    }
}

/// # Safety
/// As `coupler_call`.
#[no_mangle]
pub unsafe extern "C" fn coupler_feed(ptr: *const u8, len: usize) -> u32 {
    let bytes = std::slice::from_raw_parts(ptr, len);
    match with(|app| app.feed(bytes)) {
        Ok(()) => 0,
        Err(e) => {
            put(e.into_bytes());
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn coupler_take_outgoing() {
    let bytes = with(|app| Ok(app.take_outgoing())).unwrap_or_default();
    put(bytes);
}

fn with<T>(f: impl FnOnce(&mut Coupler) -> Result<T, String>) -> Result<T, String> {
    APP.with(|a| match a.borrow_mut().as_mut() {
        Some(app) => f(app),
        None => Err("Coupler hasn't started.".into()),
    })
}

fn json(text: String) -> Result<Value, String> {
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn call(input: &[u8]) -> Result<Value, String> {
    let args: Value = serde_json::from_slice(input).map_err(|e| e.to_string())?;
    let s = |name: &str| args.get(name).and_then(Value::as_str).unwrap_or("").to_string();
    let n = |name: &str| args.get(name).and_then(Value::as_u64).unwrap_or(0).min(u64::from(u16::MAX)) as u16;
    let cmd = s("cmd");
    if cmd == "init" {
        let app = Coupler::new(&s("version"), &s("mirror"))?;
        APP.with(|a| *a.borrow_mut() = Some(app));
        return Ok(Value::Null);
    }
    if cmd == "world_of" {
        return Ok(Coupler::world_of(&s("portId")).map_or(Value::Null, Value::String));
    }
    with(|app| match cmd.as_str() {
        "server_info" => json(app.server_info()),
        "mud_connect" => app.connect(&s("portId"), &s("map"), &s("cast")).map(Value::String),
        "mud_closed" => {
            app.closed(args.get("reason").and_then(Value::as_str).map(str::to_string));
            Ok(Value::Null)
        }
        "mud_send" => app.send_line(&s("line")).map(|()| Value::Null),
        "mud_resize" => {
            app.resize(n("columns"), n("rows"));
            Ok(Value::Null)
        }
        "connected" => Ok(Value::Bool(app.connected())),
        "take_events" => json(app.take_events()),
        "map_snapshot" => json(app.map_snapshot()),
        "map_find" => json(app.map_find(&s("query"))),
        "map_directions" => app.map_directions(&s("to")).map(Value::String),
        "map_walk" => app.map_walk(&s("to")).map(Value::String),
        "map_stop" => {
            app.map_stop();
            Ok(Value::Null)
        }
        "map_set_landmark" => app.map_set_landmark(&s("id"), &s("name")).map(|()| Value::Null),
        "map_clear" => {
            app.map_clear();
            Ok(Value::Null)
        }
        "cast_list" => json(app.cast_list()),
        "hooks_count" => Ok(Value::from(app.hooks_count())),
        "ambience_now" => json(app.ambience_now()),
        "who_now" => json(app.who_now()),
        "portrait_paint" => {
            let size = |name: &str| args.get(name).and_then(Value::as_u64).unwrap_or(0) as usize;
            serde_json::to_value(crate::portrait::portrait(&s("kind"), &s("name"), size("columns"), size("rows"))).map_err(|e| e.to_string())
        }
        "echo_list" => serde_json::to_value(crate::echo::listing()).map_err(|e| e.to_string()),
        "who_tick" => {
            app.who_tick();
            Ok(Value::Null)
        }
        other => Err(format!("The web build has no {other}.")),
    })
}

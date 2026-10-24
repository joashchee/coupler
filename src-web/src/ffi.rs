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
//! - `coupler_backup_write(ptr, len)`: a Coupler Backup of the page's
//!   files (`{"app", "made", "settings", "files": [[path, text]]}`), its
//!   bytes in the output buffer; `coupler_backup_read(ptr, len)`: a
//!   backup's bytes, what the web keeps of it as JSON. 0, or 1 and why.
//! - `coupler_out_ptr`/`coupler_out_len`: the output buffer, valid
//!   until the next call.

use std::cell::RefCell;

use serde_json::Value;

use crate::backup_format;
use crate::mssp::{self, Probe};
use crate::session::Coupler;

thread_local! {
    static APP: RefCell<Option<Coupler>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// The greeting's MSSP probe (mssp.rs), on a socket of its own.
    static PROBE: RefCell<Probe> = RefCell::new(Probe::default());
}

/// What the web keeps of a backup: the files it has a place for (each
/// world's map, cast and looks, the times played, and its own journal
/// and Bestiary); the desktop's databases and Assets aren't for it.
fn kept_on_the_web(path: &str) -> bool {
    let json = path.ends_with(".json");
    json && (path.starts_with("maps/") || path.starts_with("cast/") || path.starts_with("pictures/") || path.starts_with("web/") || path == "played.json")
}

/// # Safety
/// `ptr` points at `len` bytes the page wrote.
#[no_mangle]
pub unsafe extern "C" fn coupler_backup_write(ptr: *const u8, len: usize) -> u32 {
    let input = std::slice::from_raw_parts(ptr, len);
    let made = (|| {
        let args: Value = serde_json::from_slice(input).map_err(|e| e.to_string())?;
        let files: Vec<(String, String)> = serde_json::from_value(args.get("files").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
        let files: Vec<(String, Vec<u8>)> = files.into_iter().filter(|(p, _)| kept_on_the_web(p)).map(|(p, t)| (p, t.into_bytes())).collect();
        let s = |name: &str| args.get(name).and_then(Value::as_str).unwrap_or("").to_string();
        backup_format::write_all(&s("app"), args.get("made").and_then(Value::as_u64).unwrap_or(0), &s("settings"), &files)
    })();
    match made {
        Ok(bytes) => {
            put(bytes);
            0
        }
        Err(e) => {
            put(e.into_bytes());
            1
        }
    }
}

/// # Safety
/// `ptr` points at `len` bytes the page wrote.
#[no_mangle]
pub unsafe extern "C" fn coupler_backup_read(ptr: *const u8, len: usize) -> u32 {
    let mut input = std::slice::from_raw_parts(ptr, len);
    let read = backup_format::read_all(&mut input).map(|(manifest, settings, files)| {
        let mut kept = Vec::new();
        let mut left = 0;
        for (path, bytes) in files {
            match String::from_utf8(bytes) {
                Ok(text) if kept_on_the_web(&path) => kept.push((path, text)),
                _ => left += 1,
            }
        }
        serde_json::json!({ "manifest": manifest, "settings": settings, "files": kept, "left": left }).to_string()
    });
    match read {
        Ok(json) => {
            put(json.into_bytes());
            0
        }
        Err(e) => {
            put(e.into_bytes());
            1
        }
    }
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
        let played = args.get("played").and_then(Value::as_u64).unwrap_or(0);
        let app = Coupler::new(&s("version"), &s("mirror"), &s("journal"), &s("bestiary"), played)?;
        APP.with(|a| *a.borrow_mut() = Some(app));
        return Ok(Value::Null);
    }
    if cmd == "world_of" {
        return Ok(Coupler::world_of(&s("portId")).map_or(Value::Null, Value::String));
    }
    // The greeting at launch: MSSP on a socket of the page's own.
    if cmd == "probe_start" {
        let port = crate::ports::PORTS.iter().find(|p| p.id == s("portId")).ok_or("Coupler doesn't know that way to play.")?;
        PROBE.with(|p| *p.borrow_mut() = Probe::default());
        return Ok(Value::String(crate::ports::web_socket_url(port)));
    }
    if cmd == "probe_feed" {
        let bytes: Vec<u8> = serde_json::from_value(args.get("bytes").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
        return PROBE.with(|p| {
            let mut p = p.borrow_mut();
            let reply = p.feed(&bytes);
            Ok(serde_json::json!({ "reply": reply, "done": p.done() }))
        });
    }
    if cmd == "probe_result" {
        let played = with(|app| Ok(app.played())).unwrap_or(0);
        return PROBE.with(|p| {
            let p = p.borrow();
            let version = p.version().map(str::to_string);
            Ok(serde_json::json!({ "played": played, "online": p.players(), "fit": version.as_deref().map(mssp::fit), "version": version, "builtFor": mssp::BUILT_FOR }))
        });
    }
    with(|app| match cmd.as_str() {
        "server_info" => json(app.server_info()),
        "mud_connect" => app.connect(&s("portId"), &s("map"), &s("cast"), &s("looks")).map(Value::String),
        "save_now" => {
            app.save_now();
            Ok(Value::Null)
        }
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
        "cast_set_voice" => {
            let voice: crate::cast::Voice = serde_json::from_value(args.get("voice").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
            app.cast_edit(|c| c.set_voice(&s("key"), voice)).and_then(json)
        }
        "cast_reset" => app.cast_edit(|c| c.reset(&s("key"))).and_then(json),
        "cast_forget" => app.cast_edit(|c| Ok(c.forget(&s("key")))).and_then(json),
        "journal_list" => app.journal_list(&s("book"), &s("query"), args.get("who").and_then(Value::as_str)).and_then(json),
        "journal_last" => app.journal_last(&s("line")).and_then(json),
        "journal_unheard" => app.journal_unheard().and_then(json),
        "journal_unheard_entries" => app.journal_unheard_entries(&s("book")).and_then(json),
        "journal_set_heard" => {
            let ids: Vec<i64> = serde_json::from_value(args.get("ids").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
            app.journal_set_heard(&ids, args.get("heard").and_then(Value::as_bool).unwrap_or(true)).map(|()| Value::Null)
        }
        "journal_heard_all" => app.journal_heard_all(&s("book")).map(|()| Value::Null),
        "bestiary_mobs" => app.bestiary_mobs(&s("query")).and_then(json),
        "bestiary_mob" => serde_json::to_value(app.bestiary_mob(&s("key"))?).map_err(|e| e.to_string()),
        "bestiary_items" => app.bestiary_items(&s("query")).and_then(json),
        "bestiary_item" => serde_json::to_value(app.bestiary_item(&s("key"))?).map_err(|e| e.to_string()),
        "bestiary_face" => {
            let moment: crate::face::Moment = args.get("moment").cloned().filter(|m| !m.is_null()).map(serde_json::from_value).transpose().map_err(|e| e.to_string())?.unwrap_or_default();
            let face = app.bestiary_sitter(&s("key"))?.map(|sitter| crate::face::face_in(&sitter, &moment));
            serde_json::to_value(face).map_err(|e| e.to_string())
        }
        "fight_paint" => {
            let moment = |name: &str| -> Result<crate::face::Moment, String> { args.get(name).cloned().filter(|m| !m.is_null()).map(serde_json::from_value).transpose().map_err(|e| e.to_string()).map(Option::unwrap_or_default) };
            let (you_now, foe_now) = (moment("youNow")?, moment("foeNow")?);
            let style: crate::paint::Style = serde_json::from_value(args.get("style").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
            let columns = args.get("columns").and_then(Value::as_u64).unwrap_or(0) as usize;
            let (you, foe, scene) = app.fighters(&s("foe"))?;
            let fighters = crate::fight_art::Fighters { you: (&you, &you_now), foe: (&foe, &foe_now) };
            serde_json::to_value(crate::fight_art::paint(scene.as_ref(), style, &fighters, columns)).map_err(|e| e.to_string())
        }
        "bestiary_item_art" => {
            let art = app.bestiary_thing(&s("key"))?.map(|t| crate::item_art::painted(&t));
            serde_json::to_value(art).map_err(|e| e.to_string())
        }
        "automation_set" => {
            app.automation_set(serde_json::from_value(args.get("rules").cloned().unwrap_or_default()).map_err(|e| e.to_string())?);
            Ok(Value::Null)
        }
        "channels_ask" => app.channel_commands(|c, now| c.ask(now)).map(|_| Value::Null),
        "channel_set" => {
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            let name = s("name");
            app.channel_commands(|c, now| c.turn(&name, on, now)).map(|_| Value::Null)
        }
        "channels_all" => {
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            app.channel_commands(|c, now| c.all(on, now)).map(|_| Value::Null)
        }
        "automation_recent" => json(app.automation_recent()),
        "automation_suggest" => json(app.automation_suggest(&s("line"))),
        "automation_match" => Ok(serde_json::json!(crate::automation::Pattern::new(&s("pattern")).captures(&s("line")))),
        "bestiary_auto_look" => {
            app.bestiary_auto_look(args.get("on").and_then(Value::as_bool).unwrap_or(false));
            Ok(Value::Null)
        }
        "picture_now" => json(app.picture_now()),
        "picture_again" => app.picture_again().and_then(json),
        "picture_forget_looks" => {
            app.picture_forget_looks();
            Ok(Value::Null)
        }
        "picture_paint" => {
            let scene: crate::paint::Scene = serde_json::from_value(args.get("scene").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
            let style: crate::paint::Style = serde_json::from_value(args.get("style").cloned().unwrap_or_default()).map_err(|e| e.to_string())?;
            let size = |name: &str| args.get(name).and_then(Value::as_u64).unwrap_or(0) as usize;
            let again = args.get("again").and_then(Value::as_u64).unwrap_or(0) as u32;
            serde_json::to_value(crate::painter::paint(&scene, style, size("columns"), size("rows"), again)).map_err(|e| e.to_string())
        }
        "hooks_count" => Ok(Value::from(app.hooks_count())),
        "ambience_now" => json(app.ambience_now()),
        "who_now" => json(app.who_now()),
        "who_ask" => Ok(Value::from(app.who_ask())),
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

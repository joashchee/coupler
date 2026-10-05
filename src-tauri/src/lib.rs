//! Coupler's desktop shell: the Tauri commands over the CoffeeMUD session.
//! The protocol lives in `telnet` and `ansi` (pure, unit-tested) and the
//! socket in `session`; this file only wires them to the window.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};

mod ambient;
// Public for the web mirror tool (examples/web_mirror.rs), not an API.
#[doc(hidden)]
pub mod ansi;
#[doc(hidden)]
pub mod ansi_art;
#[doc(hidden)]
pub mod assets;
mod backup;
mod cast;
#[doc(hidden)]
pub mod codecs;
mod character;
mod clock;
mod combat;
mod compose;
mod create;
mod daytime;
mod creation;
mod echo;
mod first_run;
mod hidden;
#[doc(hidden)]
pub mod hooks;
mod journal;
mod mapper;
mod mssp;
mod music;
mod paint;
mod ports;
mod painter;
mod portrait;
mod senses;
mod session;
mod speech;
mod stretch;
mod synth;
mod telnet;
mod update;
#[doc(hidden)]
pub mod trigger;
mod voicecache;
mod who;

use session::Session;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    host: &'static str,
    /// Every way to play: CoffeeMUD's ports, fixed in session.rs.
    ports: &'static [session::Port],
    connected: bool,
    /// The ID of the port the live connection is on.
    port_id: Option<&'static str>,
}

/// Where Coupler connects, for the UI's labels. Fixed: see session.rs.
#[tauri::command]
fn server_info(session: State<'_, Session>) -> ServerInfo {
    ServerInfo { host: session::HOST, ports: session::PORTS, connected: session.connected(), port_id: session.port().map(|p| p.id) }
}

/// `port_id` names an entry of `session::PORTS`. There is no way to pass
/// a host or a port number (CLAUDE.md rule 2).
#[tauri::command]
async fn mud_connect(app: AppHandle, port_id: String) -> Result<(), String> {
    // DNS and the TCP handshake block, so they run off the main thread.
    tauri::async_runtime::spawn_blocking(move || app.state::<Session>().connect(&app, &port_id))
        .await
        .map_err(|e| e.to_string())?
}

/// For the greeting at launch: the times this Mac has played, how many
/// are online now in the game `port_id` names, and its CoffeeMUD version
/// against the one Coupler was made for (each None when the server didn't
/// say). Only asks CoffeeMUD (CLAUDE.md rules 1 and 2).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchCounts {
    played: u64,
    online: Option<u32>,
    version: Option<String>,
    built_for: &'static str,
    fit: Option<mssp::Fit>,
}

#[tauri::command]
async fn launch_counts(app: AppHandle, port_id: String) -> Result<LaunchCounts, String> {
    let played = session::played(&app);
    // The socket blocks, so it runs off the main thread.
    let (online, version) = tauri::async_runtime::spawn_blocking(move || session::game_status(&port_id))
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|e| {
            log::info!("game status: {e}");
            (None, None)
        });
    let fit = version.as_deref().map(mssp::fit);
    Ok(LaunchCounts { played, online, version, built_for: mssp::BUILT_FOR, fit })
}

/// Asks GitHub whether a newer Coupler is out (update.rs). Only when the
/// player asks: the check button, or automatic checks they turned on.
#[tauri::command]
async fn update_check(app: AppHandle) -> Result<update::Check, String> {
    update::check(&app.package_info().version.to_string()).await
}

/// Shows Coupler's latest release page in the player's browser.
#[tauri::command]
fn update_open() -> Result<(), String> {
    update::open()
}

#[tauri::command]
fn mud_send(app: AppHandle, line: String, session: State<'_, Session>) -> Result<(), String> {
    session.send_line(&app, &line)
}

/// The map around the player, and the current room in words.
#[tauri::command]
fn map_snapshot(session: State<'_, Session>) -> mapper::Snapshot {
    session.map_snapshot()
}

/// Visited rooms matching `query`; the landmarks when it's empty.
#[tauri::command]
fn map_find(query: String, session: State<'_, Session>) -> Vec<mapper::RoomRef> {
    session.map_find(&query)
}

/// The way to a room, in words.
#[tauri::command]
fn map_directions(to: String, session: State<'_, Session>) -> Result<String, String> {
    session.map_directions(&to)
}

/// Walks to a room, one confirmed step at a time. Returns the way in words.
#[tauri::command]
fn map_walk(app: AppHandle, to: String, session: State<'_, Session>) -> Result<String, String> {
    session.walk_to(&app, &to)
}

#[tauri::command]
fn map_stop(app: AppHandle, session: State<'_, Session>) {
    session.stop_walk(&app, "Stopped walking.");
}

/// Names a room as a landmark; an empty name removes it.
#[tauri::command]
fn map_set_landmark(app: AppHandle, id: String, name: String, session: State<'_, Session>) -> Result<(), String> {
    session.map_set_landmark(&app, &id, &name)
}

#[tauri::command]
fn map_clear(app: AppHandle, session: State<'_, Session>) {
    session.map_clear(&app);
}

/// A book's lines (`journal` or `log`) for the character last played:
/// the newest that match `query`, narrowed to a speaker or channel.
#[tauri::command]
fn journal_list(app: AppHandle, book: String, query: String, who: Option<String>, session: State<'_, Session>) -> Result<journal::Listing, String> {
    session.journal_list(&app, &book, &query, who.as_deref())
}

/// What's still unheard in the journal and the log.
#[tauri::command]
fn journal_unheard(app: AppHandle, session: State<'_, Session>) -> Result<journal::Unheard, String> {
    session.journal_unheard(&app)
}

/// A book's unheard lines, oldest first.
#[tauri::command]
fn journal_unheard_entries(app: AppHandle, book: String, session: State<'_, Session>) -> Result<Vec<journal::Entry>, String> {
    session.journal_unheard_entries(&app, &book)
}

/// Marks lines heard (played to the end) or unheard.
#[tauri::command]
fn journal_set_heard(app: AppHandle, ids: Vec<i64>, heard: bool, session: State<'_, Session>) -> Result<(), String> {
    session.journal_set_heard(&app, &ids, heard)
}

/// Marks a whole book heard.
#[tauri::command]
fn journal_heard_all(app: AppHandle, book: String, session: State<'_, Session>) -> Result<(), String> {
    session.journal_heard_all(&app, &book)
}

/// Everyone met in the world last connected to, with their voices.
#[tauri::command]
fn cast_list(session: State<'_, Session>) -> session::CastListing {
    session.cast_list()
}

/// The player's own voice for a character; kept from then on.
#[tauri::command]
fn cast_set_voice(app: AppHandle, key: String, voice: cast::Voice, session: State<'_, Session>) -> Result<cast::Character, String> {
    session.cast_edit(&app, |c| c.set_voice(&key, voice))
}

/// A character's voice made from their name again.
#[tauri::command]
fn cast_reset(app: AppHandle, key: String, session: State<'_, Session>) -> Result<cast::Character, String> {
    session.cast_edit(&app, |c| c.reset(&key))
}

/// Forgets a character: met again, they get a new voice.
#[tauri::command]
fn cast_forget(app: AppHandle, key: String, session: State<'_, Session>) -> Result<bool, String> {
    session.cast_edit(&app, |c| Ok(c.forget(&key)))
}

/// The voices of the speech engines bundled in Coupler (synth.rs) that
/// this copy has: Pocket TTS's only when its files came with it.
#[tauri::command]
fn voice_engines(engines: State<'_, synth::Engines>) -> Vec<synth::BundledVoice> {
    engines.available()
}

/// The voice cache (voicecache.rs) and the lines waiting to go in it.
#[derive(Default)]
struct Renders {
    /// None until launch has opened it, or if it couldn't be.
    cache: Mutex<Option<voicecache::RenderCache>>,
    queue: voicecache::Queue,
}

/// How often the free space is looked at, besides after each render.
const CACHE_CHECK: std::time::Duration = std::time::Duration::from_secs(300);

fn cache_key(engine: &str, voice: &str, text: &str, gender: cast::Gender, pitch: u16, rate: u16) -> voicecache::Key {
    voicecache::Key { engine: engine.into(), voice: voice.into(), text: text.trim().into(), gender: format!("{gender:?}"), pitch, rate }
}

impl Renders {
    fn keep(&self, key: &voicecache::Key, bytes: &[u8], origin: voicecache::Origin, played: bool) {
        if let Some(cache) = self.cache.lock().unwrap().as_mut() {
            if let Err(e) = cache.put(key, bytes, origin, played) {
                log::warn!("{e}");
            }
            cache.fit();
        }
    }
}

/// Opens the voice cache, then renders what's queued, one line at a
/// time, and looks at the free space now and then.
fn start_renders(app: &AppHandle, dir: PathBuf) {
    let renders = app.state::<Renders>();
    match voicecache::RenderCache::open(&dir) {
        Ok(mut cache) => {
            cache.fit();
            *renders.cache.lock().unwrap() = Some(cache);
        }
        Err(e) => log::warn!("{e}"),
    }
    let worker = app.clone();
    std::thread::spawn(move || {
        let renders = worker.state::<Renders>();
        loop {
            let job = renders.queue.next();
            let cached = renders.cache.lock().unwrap().as_ref().is_none_or(|c| c.has(&job.key));
            if !cached {
                let k = &job.key;
                match worker.state::<synth::Engines>().say(&k.engine, &k.voice, &k.text, job.gender, k.pitch, k.rate) {
                    Ok(bytes) => renders.keep(k, &bytes, job.origin, false),
                    Err(e) => log::warn!("{e}"),
                }
            }
            renders.queue.done();
        }
    });
    let watcher = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(CACHE_CHECK);
        if let Some(cache) = watcher.state::<Renders>().cache.lock().unwrap().as_mut() {
            cache.fit();
        }
    });
}

/// A line in a bundled voice, as raw audio (synth.rs says how it's laid
/// out). Off the main thread: a long line takes a moment. A line of
/// talk (`book` is `journal` or `log`) comes from the voice cache when
/// it was rendered before, and goes in it when it wasn't; a line to try
/// a voice (no `book`) is made each time.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn voice_synth(app: AppHandle, engine: String, voice: String, text: String, gender: cast::Gender, pitch: u16, rate: u16, book: Option<String>) -> Result<tauri::ipc::Response, String> {
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        let Some(origin) = book.as_deref().and_then(voicecache::Origin::parse) else {
            return app.state::<synth::Engines>().say(&engine, &voice, &text, gender, pitch, rate);
        };
        let renders = app.state::<Renders>();
        let key = cache_key(&engine, &voice, &text, gender, pitch, rate);
        renders.queue.claim(&key);
        if let Some(bytes) = renders.cache.lock().unwrap().as_mut().and_then(|c| c.get(&key, true)) {
            return Ok(bytes);
        }
        let bytes = app.state::<synth::Engines>().say(&engine, &voice, &text, gender, pitch, rate)?;
        renders.keep(&key, &bytes, origin, true);
        Ok(bytes)
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Renders a line of talk in a bundled voice before it's wanted, into
/// the voice cache: the journal's before the log's, in the order they
/// come. Returns at once.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn voice_prerender(engine: String, voice: String, text: String, gender: cast::Gender, pitch: u16, rate: u16, book: String, renders: State<'_, Renders>, engines: State<'_, synth::Engines>) {
    let Some(origin) = voicecache::Origin::parse(&book) else { return };
    if text.trim().is_empty() || !engines.available().iter().any(|v| v.engine == engine && v.id == voice) {
        return;
    }
    renders.queue.push(voicecache::Job { key: cache_key(&engine, &voice, &text, gender, pitch, rate), gender, origin });
}

/// How many unique GMCP key and value pairs the hooks list holds
/// (filtered keys aren't counted).
#[tauri::command]
fn hooks_count(app: AppHandle, session: State<'_, Session>) -> usize {
    session.hooks_count(&app)
}

/// The hooks whose key or value contains `query`; all of them when it's
/// empty. A long list is cut short, with the full count beside it.
#[tauri::command]
fn hooks_list(app: AppHandle, query: String, session: State<'_, Session>) -> Result<hooks::Listing, String> {
    session.hooks_list(&app, &query)
}

/// The keys moved out of the hooks list, each with how many values it has.
#[tauri::command]
fn hooks_filtered(app: AppHandle, query: String, session: State<'_, Session>) -> Result<hooks::FilteredListing, String> {
    session.hooks_filtered(&app, &query)
}

/// Moves a key to the Filtered list (true) or back to the hooks list.
#[tauri::command]
fn hooks_set_filtered(app: AppHandle, key: String, filtered: bool, session: State<'_, Session>) -> Result<(), String> {
    session.hooks_set_filtered(&app, &key, filtered)
}

/// Sets what a pair sets off when the game sends it (sound, music, a
/// picture); an empty trigger removes it.
#[tauri::command]
fn hooks_set_trigger(app: AppHandle, key: String, value: String, trigger: hooks::Trigger, session: State<'_, Session>) -> Result<(), String> {
    session.hooks_set_trigger(&app, &key, &value, trigger)
}

/// The commands that answer in one line and what the narrator says for
/// each answer (echo.rs), for Workshop's Narrator's Answers.
#[tauri::command]
fn echo_list() -> echo::Listing {
    echo::listing()
}

/// Who's online now (who.rs), for the report key.
#[tauri::command]
fn who_now(session: State<'_, Session>) -> who::Report {
    session.who_now()
}

/// What background noise and weather should be looping now (ambient.rs).
#[tauri::command]
fn ambience_now(session: State<'_, Session>) -> ambient::Ambience {
    session.ambience()
}

// ---- The room's picture (paint.rs, painter.rs) ----

/// The room the picture shows now, or None when the player is nowhere.
#[tauri::command]
fn picture_now(session: State<'_, Session>) -> Option<paint::Scene> {
    session.picture_now()
}

/// A new look for the room the player is in (Cmd+Shift+P), kept as its look.
#[tauri::command]
fn picture_again(app: AppHandle, session: State<'_, Session>) -> Result<Option<paint::Scene>, String> {
    session.picture_again(&app)
}

/// Every room of this world back to its first look.
#[tauri::command]
fn picture_forget_looks(app: AppHandle, session: State<'_, Session>) -> Result<(), String> {
    session.picture_forget_looks(&app)
}

/// Coupler's painter's picture of a room, at a size in cells. `again`
/// counts the times the player asked for a new look.
#[tauri::command]
fn picture_paint(scene: paint::Scene, style: paint::Style, columns: usize, rows: usize, again: u32) -> ansi_art::Art {
    painter::paint(&scene, style, columns, rows, again)
}

/// A race's or a class's portrait by Coupler's painter (portrait.rs),
/// for the guide to making a character. `kind` is `race` or `class`.
#[tauri::command]
fn portrait_paint(kind: String, name: String, columns: usize, rows: usize) -> ansi_art::Art {
    portrait::portrait(&kind, &name, columns, rows)
}

// ---- The Assets folder (assets.rs) ----

/// The Assets folder, in app data (CLAUDE.md rule 8).
fn assets_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("Assets"))
}

/// The Assets folder's files, each with how many hooks use it, and the
/// assets cleared from the hooks because they've left the folder.
#[derive(serde::Serialize)]
struct AssetsListing {
    assets: Vec<assets::Asset>,
    cleared: Vec<hooks::Cleared>,
    /// Each MIDI file played through a SoundFont the player chose, and
    /// the SoundFont; every other plays through Neumetik.
    fonts: std::collections::BTreeMap<String, String>,
}

/// Every file in the Assets folder, after clearing any the hooks name
/// that aren't there any more.
#[tauri::command]
fn assets_list(app: AppHandle, session: State<'_, Session>) -> Result<AssetsListing, String> {
    let root = assets_dir(&app)?;
    // A folder that can't be read at all isn't a reason to forget every asset.
    let cleared = if root.is_dir() { session.hooks_clear_missing(&app, |path| assets::resolve(&root, path).is_ok())? } else { Vec::new() };
    let uses = session.hooks_asset_uses(&app);
    let mut all = assets::list(&root);
    for asset in &mut all {
        asset.uses = uses.get(&asset.path).copied().unwrap_or(0);
    }
    Ok(AssetsListing { assets: all, cleared, fonts: assets::fonts(&root) })
}

/// Chooses the SoundFont a MIDI asset plays through, or Neumetik (None).
#[tauri::command]
fn asset_set_font(app: AppHandle, path: String, font: Option<String>) -> Result<(), String> {
    assets::set_font(&assets_dir(&app)?, &path, font.as_deref())
}

/// An asset Coupler made from the player's words, and what it is in words.
#[derive(serde::Serialize)]
struct Made {
    #[serde(flatten)]
    asset: assets::Asset,
    about: String,
    /// How music was written, a sentence for each decision (none for a picture).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rules: Vec<String>,
}

/// Paints a picture from the player's words with Coupler's painter and
/// saves it in the Assets folder as ANSI art.
#[tauri::command]
async fn asset_create_art(app: AppHandle, prompt: String, columns: usize, rows: usize) -> Result<Made, String> {
    let root = assets_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut about = String::new();
        let asset = assets::add_made(&root, "ans", &create::file_stem(&prompt, "picture"), |take| {
            let (scene, style, said) = create::scene_from(&prompt, take);
            about = said;
            create::to_ansi(&painter::paint(&scene, style, columns, rows, take), &prompt)
        })?;
        Ok(Made { asset, about, rules: Vec::new() })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Composes a short piece from the player's words and Create Asset's
/// options for Neumetik and saves it in the Assets folder as MIDI.
#[tauri::command]
async fn asset_create_music(app: AppHandle, prompt: String, options: Option<compose::Options>) -> Result<Made, String> {
    let root = assets_dir(&app)?;
    let options = options.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        let mut about = String::new();
        let mut rules = Vec::new();
        let asset = assets::add_made(&root, "mid", &create::file_stem(&prompt, "music"), |take| {
            let piece = compose::compose(&prompt, take, &options);
            about = piece.about;
            rules = piece.rules;
            piece.midi
        })?;
        Ok(Made { asset, about, rules })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Copies files the player dropped or chose into the Assets folder,
/// sending `progress` as it goes.
#[tauri::command]
async fn assets_import(app: AppHandle, paths: Vec<PathBuf>, progress: tauri::ipc::Channel<assets::Progress>) -> Result<assets::Imported, String> {
    let root = assets_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        assets::import(&root, &paths, |p| {
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| e.to_string())
}

/// Puts the smaller copies the player took in place of their WAVs.
#[tauri::command]
async fn assets_compress(app: AppHandle, paths: Vec<String>) -> Result<assets::Compressed, String> {
    let root = assets_dir(&app)?;
    let uses = app.state::<Session>().hooks_asset_uses(&app);
    tauri::async_runtime::spawn_blocking(move || assets::compress(&root, &paths, |p| uses.contains_key(p))).await.map_err(|e| e.to_string())
}

/// Forgets the smaller copies the player didn't take.
#[tauri::command]
fn assets_keep(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    assets::keep(&assets_dir(&app)?, &paths);
    Ok(())
}

/// A sound or music asset as something the WebView plays (WAV or MP3),
/// sent as raw bytes. Rendering music can take a moment, so it's off
/// the main thread.
#[tauri::command]
async fn asset_audio(app: AppHandle, path: String, looping: bool) -> Result<tauri::ipc::Response, String> {
    let root = assets_dir(&app)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || assets::audio(&root, &path, looping)).await.map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
}

/// An ANSI art asset, drawn into rows of styled characters.
#[tauri::command]
fn asset_ansi(app: AppHandle, path: String) -> Result<ansi_art::Art, String> {
    assets::ansi(&assets_dir(&app)?, &path)
}

/// An image asset's bytes.
#[tauri::command]
fn asset_picture(app: AppHandle, path: String) -> Result<tauri::ipc::Response, String> {
    Ok(tauri::ipc::Response::new(assets::picture(&assets_dir(&app)?, &path)?))
}

#[tauri::command]
fn mud_disconnect(app: AppHandle, session: State<'_, Session>) {
    session.disconnect(&app);
}

/// The output pane's size in character cells, for NAWS.
#[tauri::command]
fn mud_resize(columns: u16, rows: u16, session: State<'_, Session>) -> Result<(), String> {
    session.resize(columns.max(1), rows.max(1))
}

/// Full screen on or off. The window is one fixed size otherwise
/// (tauri.conf.json: 1280 by 720, not resizable), and in full screen the
/// frontend scales that same screen up to fit (src/lib/stage.ts).
#[tauri::command]
fn window_set_fullscreen(window: tauri::WebviewWindow, on: bool) -> Result<(), String> {
    window.set_fullscreen(on).map_err(|e| e.to_string())
}

#[tauri::command]
fn window_is_fullscreen(window: tauri::WebviewWindow) -> Result<bool, String> {
    window.is_fullscreen().map_err(|e| e.to_string())
}

// ---- The Music Editor (music.rs) ----

/// A MIDI asset as the Music Editor's song.
#[tauri::command]
fn music_open(app: AppHandle, path: String) -> Result<music::Song, String> {
    let file = assets::resolve(&assets_dir(&app)?, &path)?;
    let bytes = std::fs::read(&file).map_err(|e| format!("{path} couldn't be read. ({e})"))?;
    music::read(&bytes).map_err(|e| format!("{path} couldn't be opened: {e}."))
}

/// A new, empty song.
#[tauri::command]
fn music_new() -> music::Song {
    music::Song::new()
}

/// Saves the song over the MIDI asset it came from.
#[tauri::command]
fn music_save(app: AppHandle, path: String, song: music::Song) -> Result<(), String> {
    assets::replace_midi(&assets_dir(&app)?, &path, &music::write(&song))
}

/// Saves the song as a new MIDI asset named from `name` (another take
/// if that's taken: nothing is written over).
#[tauri::command]
fn music_save_as(app: AppHandle, name: String, song: music::Song) -> Result<assets::Asset, String> {
    let stem = create::file_stem(name.trim().trim_end_matches(".mid").trim_end_matches(".midi"), "music");
    assets::add_made(&assets_dir(&app)?, "mid", &stem, |_| music::write(&song))
}

/// The bars `from` to `to` of the song as a WAV to play, through
/// Neumetik, or the SoundFont `font` when the player chose one in the
/// editor (never the one the file was given in the Assets tab).
#[tauri::command]
async fn music_render(app: AppHandle, font: Option<String>, song: music::Song, from: u32, to: u32, looping: bool) -> Result<tauri::ipc::Response, String> {
    let root = assets_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || assets::midi_audio(&root, font.as_deref(), &music::preview(&song, from, to), looping).map(tauri::ipc::Response::new))
        .await
        .map_err(|e| e.to_string())?
}

/// Neumetik's instruments, bank by bank, and its drum kits.
#[derive(serde::Serialize)]
struct Instruments {
    banks: Vec<InstrumentBank>,
    kits: Vec<(u8, &'static str)>,
}

#[derive(serde::Serialize)]
struct InstrumentBank {
    msb: u8,
    names: Vec<&'static str>,
}

#[tauri::command]
fn music_instruments() -> Instruments {
    Instruments {
        banks: neumetik::BANKS.iter().map(|b| InstrumentBank { msb: b.msb, names: b.patches.iter().map(|p| p.name).collect() }).collect(),
        kits: neumetik::drums::KITS.to_vec(),
    }
}

// ---- Coupler Backup (backup.rs) ----

/// Writes a Coupler Backup to `path`: app data's kept files and
/// `settings` (the frontend's, as JSON), sending `progress` as it goes.
#[tauri::command]
async fn backup_export(app: AppHandle, path: PathBuf, settings: String, progress: tauri::ipc::Channel<backup::Progress>) -> Result<backup::Made, String> {
    let root = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    // What's waiting to be saved goes in too.
    app.state::<Session>().save_now(&app);
    let version = app.package_info().version.to_string();
    let made = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    tauri::async_runtime::spawn_blocking(move || {
        backup::export(&root, &path, &version, made, &settings, |p| {
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// What's in a backup, before it's restored.
#[tauri::command]
fn backup_inspect(path: PathBuf) -> Result<backup::Manifest, String> {
    backup::inspect(&mut backup::open(&path)?)
}

/// Unpacks a backup, checked, to take the place of what's here at the
/// next launch (`backup::apply`). Not while connected.
#[tauri::command]
async fn backup_restore(app: AppHandle, path: PathBuf, progress: tauri::ipc::Channel<backup::Progress>) -> Result<backup::Manifest, String> {
    if app.state::<Session>().connected() {
        return Err("Disconnect from CoffeeMUD first, then restore the backup.".into());
    }
    let root = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    tauri::async_runtime::spawn_blocking(move || {
        backup::unpack(&mut backup::open(&path)?, &root, |p| {
            let _ = progress.send(p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Starts Coupler again, to apply a restored backup.
#[tauri::command]
fn app_restart(app: AppHandle) {
    app.restart();
}

/// Dev-only App Testing report (the overlay is compiled out of release
/// builds; this plain file write stays, as in Diskette).
#[tauri::command]
fn export_app_testing_report(path: String, report: String) -> Result<(), String> {
    std::fs::write(&path, report).map_err(|e| e.to_string())
}

/// Creates the main window, once the first-run warning is accepted.
fn start(app: &AppHandle) {
    let config = &app.config().app.windows[0];
    // A backup just restored: its settings go in before the page's scripts run.
    let restored = app.path().app_data_dir().ok().and_then(|dir| backup::take_settings(&dir));
    tauri::WebviewWindowBuilder::from_config(app, config)
        .map(|builder| match restored {
            Some(settings) => {
                let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1);
                builder.initialization_script(backup::settings_script(&settings, nonce))
            }
            None => builder,
        })
        .and_then(|builder| builder.build())
        .expect("failed to create the main window");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .manage(Session::default())
        .manage(synth::Engines::default())
        .manage(Renders::default())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to resolve app data dir");
            // A backup restored last run takes its place before anything's opened.
            match backup::apply(&app_data_dir) {
                Ok(true) => log::info!("A Coupler Backup was restored."),
                Ok(false) => {}
                Err(e) => log::warn!("{e}"),
            }
            // The Assets folder is there from the start, for the player to find.
            if let Err(e) = std::fs::create_dir_all(app_data_dir.join("Assets")) {
                log::warn!("The Assets folder couldn't be made. ({e})");
            }
            // Smaller copies offered last time, never answered.
            assets::forget_waiting(&app_data_dir.join("Assets"));
            // Pocket TTS's weights and voices ship in the app's Resources.
            match app.path().resource_dir() {
                Ok(dir) => app.state::<synth::Engines>().set_pocket_dir(dir.join("pocket-tts")),
                Err(e) => log::warn!("Coupler's Resources folder couldn't be found, so Pocket TTS's voices are off. ({e})"),
            }
            // Lines of talk rendered before they're wanted (voicecache.rs).
            start_renders(app.handle(), app_data_dir.join("voice-cache"));
            // The main window has `"create": false` in tauri.conf.json, so
            // nothing is drawn until the first-run warning is answered.
            if first_run::accepted(&app_data_dir) {
                start(app.handle());
            } else {
                first_run::ask(app.handle(), &app_data_dir, start);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            server_info,
            mud_connect,
            launch_counts,
            update_check,
            update_open,
            mud_send,
            mud_disconnect,
            mud_resize,
            map_snapshot,
            map_find,
            map_directions,
            map_walk,
            map_stop,
            map_set_landmark,
            map_clear,
            cast_list,
            cast_set_voice,
            cast_reset,
            cast_forget,
            journal_list,
            journal_unheard,
            journal_unheard_entries,
            journal_set_heard,
            journal_heard_all,
            voice_engines,
            voice_synth,
            voice_prerender,
            hooks_count,
            hooks_list,
            hooks_filtered,
            hooks_set_filtered,
            hooks_set_trigger,
            ambience_now,
            who_now,
            echo_list,
            picture_now,
            picture_again,
            picture_forget_looks,
            picture_paint,
            portrait_paint,
            assets_list,
            assets_import,
            assets_compress,
            assets_keep,
            asset_set_font,
            asset_create_art,
            asset_create_music,
            asset_audio,
            asset_picture,
            asset_ansi,
            window_set_fullscreen,
            window_is_fullscreen,
            music_open,
            music_new,
            music_save,
            music_save_as,
            music_render,
            music_instruments,
            backup_export,
            backup_inspect,
            backup_restore,
            app_restart,
            export_app_testing_report
        ])
        .build(tauri::generate_context!())
        .expect("error while building Coupler")
        .run(|app, event| {
            // Quitting hangs up first, as Disconnect does: the game sees
            // the line close, the map is saved, and the walk stops.
            if let tauri::RunEvent::Exit = event {
                app.state::<Session>().disconnect(app);
            }
        });
}

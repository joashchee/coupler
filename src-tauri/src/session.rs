//! The connection to CoffeeMUD: one TCP socket, a reader thread that runs
//! everything through `telnet` and `ansi` and emits it to the window, and
//! a writer the commands share. It also feeds the auto-mapper (`mapper`)
//! from GMCP, saves the map, and walks routes one confirmed room at a time,
//! and records every GMCP key and value in the hooks database (`hooks`).
//! It notices a logout or a switch of character while still connected
//! (`character`) and emits `character-left`. It reads the time of day
//! from MSDP's hour and the game's text (`daytime`) and records it as a
//! pair too. It reads the character's vitals and the talk addressed to
//! them (`senses`) and emits `vitals` and `talk` for Immersive mode,
//! each line of talk with its speaker's voice from the cast of everyone
//! met (`cast`), kept per world like the map. Each line of talk is
//! kept in the journal or the log (`journal`), per world and character,
//! and `journal-changed` says what's still unheard. It gathers what a
//! picture of the room is made from (`paint`) and emits `picture` when
//! that changes. It counts each time a character comes into the game
//! (`played.json`, for the greeting at launch), and asks the server how
//! many are online, and the game's version, by MSSP (`game_status`, `mssp`). It keeps who's
//! online (`who`): it asks WHO once a minute and hides the reply, and
//! hides the game's login and logout announcements, emitting `who` with
//! each change instead. It runs the player's triggers, gags, aliases and
//! tickers (`automation`): gagged lines aren't shown, a trigger's or a
//! ticker's commands are sent, and `automation-fired` says what to say
//! and play.
//!
//! **Coupler connects to one place only** (CLAUDE.md rule 2): the host
//! and the list of its ports are constants here, and no command takes an
//! address: `connect` takes the ID of an entry in `PORTS`, never a
//! number. This is Coupler's only network code.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::ambient::{self, Ambience, Ambient};
use crate::songbook::{self, Finder, Poster};
use crate::music;
use crate::ansi::{Line, Screen};
use crate::automation::{self, Automation};
use crate::bestiary::{self, Bestiary, Creature, Cry, Fact, Reader};
use crate::cast::{self, Cast};
use crate::character::{Character, Left};
use crate::combat::{Combat, Opponent};
use crate::daytime::{self, Daytime, Phase};
use crate::foley::{self, Comings, Happening, Movement, Place};
use crate::hooks::{self, Hooks, Trigger};
use crate::soundbank;
use crate::journal::{self, Book, Journal};
use crate::mapper::{self, Map, Step};
use crate::msp;
use crate::mssp::Probe;
use crate::paint::{Looks, Scene};
pub use crate::ports::{Port, HOST, PORTS};
use crate::senses::{Senses, Talk, Vitals};
use crate::creation::{self, Creation};
use crate::echo::{Echoed, Echoes};
use crate::hidden::{Hidden, LongLook};
use crate::speech::{Kinds, LineKind};
use crate::telnet::{self, Event, Telnet};
use crate::account::{self, Account};
use crate::who::{self, Seen, Who};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// The MSSP probe at launch gives up after this long, all told.
const PROBE_TIMEOUT: Duration = Duration::from_secs(6);
/// The map is written at most this often while exploring, and always
/// when the connection ends.
const MAP_SAVE_EVERY: Duration = Duration::from_secs(20);
/// How far the map picture reaches from the current room, in rooms. The
/// picture is a fixed size (MapPanel.tsx's VIEW_RX and VIEW_RY match).
const MAP_RADIUS: (i32, i32) = (5, 3);
/// How many hooks the list shows at once; its filter narrows the rest down.
const HOOKS_SHOWN: usize = 500;
/// How many lines the journal and the log show at once; the search
/// finds the rest.
const JOURNAL_SHOWN: usize = 1000;
/// How often the connection's clock checks whether WHO is due (`who.rs`).
const WHO_TICK: Duration = Duration::from_secs(1);

/// While a character's being made or the account menu's open, the
/// game starts the login over once it's read nothing for 3 minutes
/// (`DefaultSession.HSIDLETIMEOUT`): a telnet NOP (it does nothing, but
/// it's read) after this long without sending keeps the line alive.
/// Any line would be an answer, so nothing else is sent.
const KEEP_ALIVE: Duration = Duration::from_secs(60);

/// Lines the reader finished, and the unfinished one (a prompt), which
/// replaces the last one sent.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutputEvent {
    pub lines: Vec<Line>,
    /// What kind each line is (`speech.rs`), for speech by kind.
    pub kinds: Vec<LineKind>,
    /// The commands' one-line answers among them (`echo.rs`).
    pub echoes: Vec<Echoed>,
    /// A long look's room and its hidden details (`hidden.rs`).
    pub hidden: Option<Hidden>,
    pub partial: Option<Line>,
}

#[derive(Serialize, Clone)]
pub struct GmcpEvent {
    pub package: String,
    /// The message's JSON, unparsed; empty when it has none.
    pub data: String,
}

#[derive(Serialize, Clone)]
pub struct ClosedEvent {
    /// Why, in words for the status line. None when the user disconnected.
    pub reason: Option<String>,
}

/// The player left their character but not the game: a logout or a
/// switch (`character.rs`). The frontend fades out what was playing and
/// closes the pictures.
#[derive(Serialize, Clone)]
pub struct LeftEvent {
    /// Words for the status line.
    pub message: String,
}

/// The walk in progress: started, stopped or finished, with words for
/// the status line (and for a screen reader).
#[derive(Serialize, Clone)]
pub struct WalkEvent {
    pub walking: bool,
    pub message: String,
}

struct Live {
    stream: TcpStream,
    telnet: Arc<Mutex<Telnet>>,
    /// Bumped per connection, so an old reader's "closed" is ignored.
    id: u64,
    port: &'static Port,
    /// When Coupler last sent the game anything, for the keep-alive.
    wrote: Instant,
}

/// The map of the world the player is connected to, and the walk along it.
#[derive(Default)]
struct Atlas {
    map: Map,
    /// Which world's map is loaded (`Port::world`); None before the first connection.
    world: Option<&'static str>,
    unsaved: bool,
    saved_at: Option<Instant>,
    /// The rest of a walk: the next step is first.
    walk: VecDeque<Step>,
    /// Asked the game where the player is since the map last knew.
    asked_where: bool,
}

/// The hooks database (`hooks.rs`), opened the first time it's wanted.
/// None inside means it couldn't be opened at all, even in memory.
#[derive(Default)]
struct HookStore {
    hooks: Option<Hooks>,
    opened: bool,
}

/// The ambience's facts (`ambient.rs`) and what was last sent.
#[derive(Default)]
struct AmbientStore {
    ambient: Ambient,
    sent: Ambience,
}

/// Everyone met in a world and their voices (`cast.rs`).
#[derive(Default)]
struct CastStore {
    cast: Cast,
    /// Which world's cast is loaded; None before the first connection.
    world: Option<&'static str>,
    unsaved: bool,
    saved_at: Option<Instant>,
}

/// What `talk` sends: the line, its speaker's voice, and its place in
/// the journal or the log (None for a repeat, which isn't kept).
#[derive(Serialize, Clone)]
struct TalkEvent {
    #[serde(flatten)]
    talk: Talk,
    voice: Option<cast::Voice>,
    entry: Option<journal::Entry>,
    /// A line this speaker has said before: not kept, and not spoken.
    repeat: bool,
}

/// The journal and the log (`journal.rs`), opened the first time
/// they're wanted, and whose they are: the world and character last
/// played, so they can be read after hanging up.
#[derive(Default)]
struct JournalStore {
    journal: Option<Journal>,
    opened: bool,
    world: Option<&'static str>,
    character: String,
}

/// The Bestiary and the Items (`bestiary.rs`), opened the first time
/// they're wanted, the world last played (so they can be read after
/// hanging up), what's being read from the game, and the fight's
/// opponent and hit points last kept.
#[derive(Default)]
struct BestiaryStore {
    bestiary: Option<Bestiary>,
    opened: bool,
    world: Option<&'static str>,
    reader: Reader,
    opponent: Option<String>,
    /// When the fight turned to this opponent: MSDP's figures follow
    /// within a second, so until then they're the last one's.
    since: Option<Instant>,
    health: Option<i64>,
    /// Coupler looks at each creature never looked at, unobtrusively
    /// (the player's choice, `bestiary_auto_look`).
    auto: bool,
    /// Creatures seen that want a look, the first met first.
    to_look: VecDeque<String>,
    /// When Coupler last looked by itself.
    looked_at: Option<Instant>,
}

/// How often Coupler looks at a creature by itself, at most.
const AUTO_LOOK_EVERY: Duration = Duration::from_secs(4);

/// What `cast_list` answers.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastListing {
    /// The world's name; None before the first connection.
    pub world: Option<&'static str>,
    pub characters: Vec<cast::Character>,
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The room's picture: its name from `room.info` (the rest of the
/// scene is the ambience's and the time's), and the scene last sent.
#[derive(Default)]
struct PictureStore {
    name: String,
    sent: Option<Scene>,
    /// The looks the player chose for rooms (`paint::Looks`), and whose
    /// world they are; None before the first scene.
    looks: Looks,
    world: Option<&'static str>,
}

/// Music in the journals (`songbook.rs`): TMF read out of the game's
/// lines, and a song being written into a journal.
#[derive(Default)]
struct Songbook {
    finder: Finder,
    poster: Option<Poster>,
}

/// What `music-found` sends: music read in the game, a journal entry's
/// TMF, as a song for the Music Editor, or why it couldn't be read.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MusicFound {
    title: String,
    song: Option<music::Song>,
    characters: usize,
    problem: Option<String>,
}

/// What `music-written` sends: whether the journal entry was added, in words.
#[derive(Serialize, Clone)]
struct MusicWritten {
    ok: bool,
    message: String,
}

/// The fight (`combat.rs`) and what was last sent.
#[derive(Default)]
struct CombatStore {
    combat: Combat,
    sent: Option<Opponent>,
}

#[derive(Default)]
pub struct Session {
    live: Mutex<Option<Live>>,
    next_id: Mutex<u64>,
    /// The last window size, applied to each new connection.
    size: Mutex<(u16, u16)>,
    atlas: Mutex<Atlas>,
    hooks: Mutex<HookStore>,
    ambient: Mutex<AmbientStore>,
    /// The time of day (`daytime.rs`).
    daytime: Mutex<Daytime>,
    /// Who's playing, for telling a logout or a switch; new each connection.
    character: Mutex<Character>,
    /// Who the player is fighting; new each connection.
    combat: Mutex<CombatStore>,
    /// Music in the journals (`songbook.rs`); new each connection.
    songbook: Mutex<Songbook>,
    /// The vitals and talk (`senses.rs`); new each connection.
    senses: Mutex<SensesStore>,
    /// Everyone met and their voices (`cast.rs`), the world's.
    cast: Mutex<CastStore>,
    /// Every line of talk, in the journal or the log (`journal.rs`).
    journal: Mutex<JournalStore>,
    /// Every creature and item come across (`bestiary.rs`).
    bestiary: Mutex<BestiaryStore>,
    /// What the room's picture shows (`paint.rs`).
    picture: Mutex<PictureStore>,
    /// This character's coming into the game was counted (`played.json`);
    /// cleared on each connection, logout and switch.
    counted: Mutex<bool>,
    /// Who's online (`who.rs`); new each connection.
    who: Mutex<Who>,
    /// The account menu (`account.rs`); new each connection.
    account: Mutex<Account>,
    /// The commands sent, waiting for their one-line answers (`echo.rs`).
    echoes: Mutex<Echoes>,
    /// A long look sent, waiting for its room (`hidden.rs`).
    long_look: Mutex<LongLook>,
    /// A character being made, a question at a time (`creation.rs`).
    creation: Mutex<Creation>,
    /// The lines that say how someone came or went, until GMCP says who
    /// (`foley.rs`).
    comings: Mutex<Comings>,
    /// The player's triggers, gags, aliases and tickers (`automation.rs`).
    automation: Mutex<Automation>,
}

/// Something Coupler held back of the player's rules: too much at once.
const HELD: &str = "Coupler held back commands of your triggers or tickers: too many at once. A trigger may be answering its own line.";

/// What `who` sends: who logged on or off.
#[derive(Serialize, Clone)]
struct WhoEvent {
    changes: Vec<who::Change>,
}

/// How many times a character came into the game with Coupler on this
/// Mac, kept in app data as `played.json`.
#[derive(serde::Deserialize, Serialize, Default)]
struct Played {
    played: u64,
}

fn played_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("played.json"))
}

/// The times played so far.
pub fn played(app: &AppHandle) -> u64 {
    played_path(app)
        .and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string()))
        .ok()
        .and_then(|json| serde_json::from_str::<Played>(&json).ok())
        .map_or(0, |p| p.played)
}

/// How many are online in a game now, and its CoffeeMUD version, by
/// MSSP (`mssp.rs`): a short connection that never logs in and hangs up
/// once the table comes. Each None when the server didn't say in time.
/// `port_id` names an entry of `PORTS`, like `connect`'s.
pub fn game_status(port_id: &str) -> Result<(Option<u32>, Option<String>), String> {
    let port = PORTS.iter().find(|p| p.id == port_id).ok_or("Coupler doesn't know that way to play.")?;
    let addr = (HOST, port.port)
        .to_socket_addrs()
        .map_err(|e| format!("Couldn't find {HOST}. ({e})"))?
        .next()
        .ok_or_else(|| format!("Couldn't find {HOST}."))?;
    let started = Instant::now();
    let mut stream = TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).map_err(|e| format!("Couldn't reach CoffeeMUD. ({e})"))?;
    let mut probe = Probe::default();
    let mut buf = [0u8; 4096];
    while !probe.done() {
        let left = PROBE_TIMEOUT.saturating_sub(started.elapsed());
        if left.is_zero() {
            break;
        }
        stream.set_read_timeout(Some(left)).map_err(|e| e.to_string())?;
        let n = match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let replies = probe.feed(&buf[..n]);
        if !replies.is_empty() && stream.write_all(&replies).is_err() {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok((probe.players(), probe.version().map(str::to_string)))
}

/// The vitals and talk (`senses.rs`) and the vitals last sent.
#[derive(Default)]
struct SensesStore {
    senses: Senses,
    sent: Option<Vitals>,
}

/// Where a world's map is kept: under the app-data dir (CLAUDE.md rule 8).
fn map_path(app: &AppHandle, world: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("maps").join(format!("{world}.json")))
}

/// Where a world's cast is kept, beside its map.
/// Where a world's chosen picture looks are kept: under the app-data dir.
fn looks_path(app: &AppHandle, world: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("pictures").join(format!("{world}.json")))
}

fn cast_path(app: &AppHandle, world: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("cast").join(format!("{world}.json")))
}

/// Where the journal and the log are kept: one database for every world
/// and character, each line marked with whose it is.
fn journal_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("journal.sqlite"))
}

/// Where the Bestiary and the Items are kept: one database for every
/// world, each entry marked with its world.
fn bestiary_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("bestiary.sqlite"))
}

/// Where the hooks database is kept: one for every world, since the
/// games share one GMCP.
fn hooks_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("Couldn't find where Coupler keeps its data. ({e})"))?;
    Ok(dir.join("hooks.sqlite"))
}

/// Writes a JSON file beside the old one and renames it over, so a crash
/// mid-write can't lose what was there.
fn write_replacing(path: &Path, json: String) -> Result<(), String> {
    let dir = path.parent().expect("the file is inside the app-data dir");
    let temp = path.with_extension("json.saving");
    std::fs::create_dir_all(dir)
        .and_then(|_| std::fs::write(&temp, json))
        .and_then(|_| std::fs::rename(&temp, path))
        .map_err(|e| e.to_string())
}

impl Session {
    pub fn connect(&self, app: &AppHandle, port_id: &str) -> Result<(), String> {
        if self.live.lock().unwrap().is_some() {
            return Ok(());
        }
        let port = PORTS.iter().find(|p| p.id == port_id).ok_or("Coupler doesn't know that way to play.")?;
        let (name, number) = (port.name, port.port);
        let addr = (HOST, number)
            .to_socket_addrs()
            .map_err(|e| format!("Couldn't find {HOST}. Check the internet connection. ({e})"))?
            .next()
            .ok_or_else(|| format!("Couldn't find {HOST}."))?;
        let stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
            .map_err(|e| format!("Couldn't reach CoffeeMUD {name} at {HOST}, port {number}. ({e})"))?;
        let _ = stream.set_nodelay(true);
        let reader = stream.try_clone().map_err(|e| e.to_string())?;

        let mut telnet = Telnet::new(&app.package_info().version.to_string());
        let (w, h) = *self.size.lock().unwrap();
        if w > 0 {
            telnet.resize(w, h);
        }
        let telnet = Arc::new(Mutex::new(telnet));
        let id = {
            let mut n = self.next_id.lock().unwrap();
            *n += 1;
            *n
        };
        self.load_map(app, port.world);
        self.load_cast(app, port.world);
        {
            let mut journal = self.journal.lock().unwrap();
            if journal.world != Some(port.world) {
                journal.world = Some(port.world);
                journal.character.clear();
            }
        }
        {
            let mut store = self.bestiary.lock().unwrap();
            store.world = Some(port.world);
            store.reader.clear();
            store.opponent = None;
            store.health = None;
            store.to_look.clear();
        }
        *self.character.lock().unwrap() = Character::default();
        *self.combat.lock().unwrap() = CombatStore::default();
        *self.songbook.lock().unwrap() = Songbook::default();
        *self.senses.lock().unwrap() = SensesStore::default();
        *self.counted.lock().unwrap() = false;
        *self.who.lock().unwrap() = Who::default();
        *self.account.lock().unwrap() = Account::default();
        self.echoes.lock().unwrap().clear();
        self.long_look.lock().unwrap().clear();
        self.creation.lock().unwrap().clear();
        self.automation.lock().unwrap().closed();
        *self.live.lock().unwrap() = Some(Live { stream, telnet: telnet.clone(), id, port, wrote: Instant::now() });

        // WHO's clock, for as long as this connection lasts.
        let clock = app.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(WHO_TICK);
            let session = clock.state::<Session>();
            if session.live.lock().unwrap().as_ref().is_none_or(|l| l.id != id) {
                break;
            }
            session.who_tick(&clock);
        });

        let app = app.clone();
        std::thread::spawn(move || {
            let reason = read_loop(&app, reader, &telnet);
            let session = app.state::<Session>();
            let mut live = session.live.lock().unwrap();
            if live.as_ref().is_some_and(|l| l.id == id) {
                *live = None;
                drop(live);
                session.left_the_game(&app, "Stopped walking: the connection ended.");
                let _ = app.emit("mud-closed", ClosedEvent { reason });
            }
        });
        log::info!("connected to {HOST}:{number}");
        Ok(())
    }

    /// A line the player typed: an alias's commands if it's one
    /// (`automation.rs`; only in the game, never into a password). Typing
    /// takes over from a walk in progress.
    pub fn send_line(&self, app: &AppHandle, line: &str) -> Result<(), String> {
        self.stop_walk(app, "Stopped walking: you typed a command.");
        if self.songbook.lock().unwrap().poster.take().is_some() {
            let _ = app.emit("music-written", MusicWritten { ok: false, message: "You typed a command, so Coupler stopped writing the music: answer the game's question yourself.".into() });
        }
        let acting = self.who.lock().unwrap().acting();
        let expanded = if acting { self.automation.lock().unwrap().expand(line) } else { None };
        for line in expanded.unwrap_or_else(|| vec![line.to_string()]) {
            self.who.lock().unwrap().typed(&line, Instant::now());
            self.send_command(app, &line)?;
        }
        Ok(())
    }

    /// A command of the player's rules (a trigger's, a ticker's): sent as
    /// typed, but it isn't the player at the keyboard (`Who::typed`), so
    /// it never keeps an away player "here".
    fn send_auto(&self, app: &AppHandle, line: &str) {
        // Not into a WRITE's questions (songbook.rs).
        if self.songbook.lock().unwrap().poster.is_some() {
            log::info!("a rule's command waits out a WRITE: not sent");
            return;
        }
        if let Err(e) = self.send_command(app, line) {
            log::warn!("{e}");
        }
    }

    /// Sends a command and tells everything that waits for its answer.
    fn send_command(&self, app: &AppHandle, line: &str) -> Result<(), String> {
        self.write(&telnet::encode_line(line))?;
        self.echoes.lock().unwrap().typed(line, Instant::now());
        self.long_look.lock().unwrap().typed(line, Instant::now());
        self.bestiary.lock().unwrap().reader.typed(line, Instant::now());
        self.creation.lock().unwrap().typed(line);
        let left = self.character.lock().unwrap().sent(line);
        if let Some(left) = left {
            self.character_left(app, left);
        }
        Ok(())
    }

    pub fn disconnect(&self, app: &AppHandle) {
        if let Some(live) = self.live.lock().unwrap().take() {
            let _ = live.stream.shutdown(Shutdown::Both);
            self.left_the_game(app, "Stopped walking: the connection ended.");
            let _ = app.emit("mud-closed", ClosedEvent { reason: None });
        }
    }

    /// The port the live connection is on.
    pub fn port(&self) -> Option<&'static Port> {
        self.live.lock().unwrap().as_ref().map(|l| l.port)
    }

    pub fn resize(&self, width: u16, height: u16) -> Result<(), String> {
        *self.size.lock().unwrap() = (width, height);
        let update = match self.live.lock().unwrap().as_ref() {
            Some(live) => live.telnet.lock().unwrap().resize(width, height),
            None => None,
        };
        match update {
            Some(bytes) => self.write(&bytes),
            None => Ok(()),
        }
    }

    /// Writes the map, the cast and the rooms' looks now if they've
    /// changed, for a backup.
    pub fn save_now(&self, app: &AppHandle) {
        self.save_map(app, true);
        self.save_cast(app, true);
        self.save_looks(app);
    }

    pub fn connected(&self) -> bool {
        self.live.lock().unwrap().is_some()
    }

    // ---- The player's rules (automation.rs) ----

    pub fn automation_set(&self, rules: automation::Rules) {
        self.automation.lock().unwrap().set(rules);
    }

    pub fn automation_recent(&self) -> Vec<automation::Recent> {
        self.automation.lock().unwrap().recent()
    }

    /// Patterns for a line, anyone met in this world a name it may be about.
    pub fn automation_suggest(&self, line: &str) -> Vec<automation::Suggestion> {
        let names: Vec<String> = self.cast.lock().unwrap().cast.list().into_iter().map(|c| c.name).collect();
        automation::suggest(line, &names)
    }

    /// Sends the tickers that are due (`who_tick`, each second).
    fn automation_tick(&self, app: &AppHandle, now: Instant) {
        let present = self.who.lock().unwrap().present(now);
        let due = self.automation.lock().unwrap().tick(now, present);
        for line in due {
            self.send_auto(app, &line);
        }
        self.automation_held(app);
    }

    /// Tells the player once when their rules wanted more than the allowance.
    fn automation_held(&self, app: &AppHandle) {
        if self.automation.lock().unwrap().take_held() {
            let _ = app.emit("automation-held", HELD);
        }
    }

    // ---- Who's online (who.rs) ----

    /// Sends WHO when it's due, and WEATHER when the painter's sky needs
    /// it (`ambient.rs`'s `weather_wanted`), each only when it's gentle to.
    fn who_tick(&self, app: &AppHandle) {
        let now = Instant::now();
        self.keep_alive(now);
        self.songbook_tick(app, now);
        // A WRITE under way takes the next lines as its answers: nothing
        // of Coupler's own (WHO, WEATHER, a look, a ticker) goes till it's done.
        if self.songbook.lock().unwrap().poster.is_some() {
            return;
        }
        self.automation_tick(app, now);
        if self.who.lock().unwrap().due(now) {
            if let Err(e) = self.write(&telnet::encode_line("who")) {
                log::warn!("{e}");
            }
            return;
        }
        let gentle = self.who.lock().unwrap().gentle(now);
        let mut ambient = self.ambient.lock().unwrap();
        if gentle && ambient.ambient.weather_wanted(now) {
            ambient.ambient.weather_asked(now);
            drop(ambient);
            if let Err(e) = self.write(&telnet::encode_line("weather")) {
                log::warn!("{e}");
            }
            return;
        }
        drop(ambient);
        if gentle {
            self.bestiary_tick(app, now);
        }
    }

    /// Sends a telnet NOP when the login's been quiet `KEEP_ALIVE` while
    /// a character's being made or the account menu waits.
    fn keep_alive(&self, now: Instant) {
        let waiting = self.creation.lock().unwrap().active() || self.account.lock().unwrap().open();
        let quiet = self.live.lock().unwrap().as_ref().is_some_and(|l| now.duration_since(l.wrote) >= KEEP_ALIVE);
        if waiting && quiet {
            if let Err(e) = self.write(&telnet::NOP) {
                log::warn!("{e}");
            }
        }
    }

    pub fn who_now(&self) -> who::Report {
        self.who.lock().unwrap().report(Instant::now())
    }

    /// The say key before any WHO was read: asks now (`Who::ask_now`).
    /// Returns whether it was sent.
    pub fn who_ask(&self) -> bool {
        let asked = self.who.lock().unwrap().ask_now(Instant::now());
        if asked {
            if let Err(e) = self.write(&telnet::encode_line("who")) {
                log::warn!("{e}");
                return false;
            }
        }
        asked
    }

    /// Emits `who` when someone logged on or off.
    fn who_changed(&self, app: &AppHandle) {
        let changes = self.who.lock().unwrap().take_changes();
        if !changes.is_empty() {
            let _ = app.emit("who", WhoEvent { changes });
        }
    }

    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        let mut live = self.live.lock().unwrap();
        let live = live.as_mut().ok_or("Not connected to CoffeeMUD.")?;
        live.wrote = Instant::now();
        live.stream.write_all(bytes).map_err(|e| format!("Couldn't send to CoffeeMUD. ({e})"))
    }

    // ---- The cast ----

    /// Loads `world`'s cast, unless it's the one already in memory.
    fn load_cast(&self, app: &AppHandle, world: &'static str) {
        let mut store = self.cast.lock().unwrap();
        if store.world == Some(world) {
            return;
        }
        let cast = match cast_path(app, world).and_then(|p| match std::fs::read_to_string(&p) {
            Ok(json) => Cast::from_json(&json),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Cast::default()),
            Err(e) => Err(format!("The saved voices couldn't be opened. ({e})")),
        }) {
            Ok(cast) => cast,
            Err(e) => {
                // As with the map: start afresh, the old file kept aside.
                log::warn!("{e}");
                if let Ok(p) = cast_path(app, world) {
                    let _ = std::fs::rename(&p, p.with_extension("json.unreadable"));
                }
                Cast::default()
            }
        };
        *store = CastStore { cast, world: Some(world), ..CastStore::default() };
    }

    /// Writes the cast if it changed: every time when `now`, otherwise no
    /// more often than the map.
    fn save_cast(&self, app: &AppHandle, now: bool) {
        let (json, world) = {
            let mut store = self.cast.lock().unwrap();
            let Some(world) = store.world else { return };
            if !store.unsaved || (!now && store.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
                return;
            }
            store.unsaved = false;
            store.saved_at = Some(Instant::now());
            (store.cast.to_json(), world)
        };
        if let Err(e) = cast_path(app, world).and_then(|path| write_replacing(&path, json)) {
            log::warn!("couldn't save the voices: {e}");
            self.cast.lock().unwrap().unsaved = true;
        }
    }

    /// A GMCP message for the cast. Returns whether anyone new was met.
    fn cast_gmcp(&self, package: &str, data: &str) -> bool {
        let mut store = self.cast.lock().unwrap();
        let new = store.cast.gmcp(package, data, unix_now());
        // Seeing someone again changes when they were last met too.
        store.unsaved |= new || package.eq_ignore_ascii_case("room.mobiles") || package.eq_ignore_ascii_case("room.players");
        new
    }

    /// A line of talk with its speaker's voice, kept in the journal or
    /// the log; the second is whether they're new.
    fn cast_talk(&self, app: &AppHandle, talk: Talk) -> (TalkEvent, bool) {
        let (voice, new, name, person) = {
            let mut store = self.cast.lock().unwrap();
            let (voice, new) = store.cast.talk(&talk, unix_now());
            store.unsaved |= voice.is_some();
            let met = store.cast.character(&crate::senses::letters_and_digits(&talk.from));
            let name = met.map(|c| c.name.clone()).unwrap_or_default();
            // Only players use the channels; anyone met in a room is known.
            let person = talk.kind == crate::senses::TalkKind::Channel || met.is_some_and(|c| c.who == cast::Who::Pc);
            (voice, new, name, person)
        };
        let (entry, repeat) = self.journal_record(app, &talk, &name, person);
        (TalkEvent { talk, voice, entry, repeat }, new)
    }

    // ---- The journal and the log (journal.rs) ----

    /// The journal, opened first if it hasn't been.
    fn journal_store(&self, app: &AppHandle) -> MutexGuard<'_, JournalStore> {
        let mut store = self.journal.lock().unwrap();
        if !store.opened {
            store.opened = true;
            let open = |path: &Path| {
                std::fs::create_dir_all(path.parent().expect("the file is inside the app-data dir")).map_err(|e| e.to_string())?;
                Journal::open(path)
            };
            let opened = journal_path(app).and_then(|path| {
                open(&path).or_else(|e| {
                    // Start fresh, as with the hooks; the old file is kept aside.
                    log::warn!("{e}");
                    let _ = std::fs::rename(&path, path.with_extension("sqlite.unreadable"));
                    open(&path)
                })
            });
            // Rather than refuse to play: a journal for this run only.
            store.journal = opened.or_else(|e| {
                log::warn!("{e}");
                Journal::in_memory()
            })
            .map_err(|e| log::warn!("{e}"))
            .ok();
        }
        store
    }

    /// Keeps a line of talk, and says what's unheard now. The entry is
    /// None for a repeat (the second is true) or when the journal can't
    /// be used.
    fn journal_record(&self, app: &AppHandle, talk: &Talk, speaker: &str, person: bool) -> (Option<journal::Entry>, bool) {
        let character = self.senses.lock().unwrap().senses.name().to_string();
        let at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
        let entry = {
            let mut store = self.journal_store(app);
            if !character.is_empty() {
                store.character = character;
            }
            let (Some(world), Some(journal)) = (store.world, store.journal.as_ref()) else { return (None, false) };
            let scope = journal::Scope { world, character: &store.character };
            match journal.record(scope, talk, speaker, person, at) {
                Ok(entry) => entry,
                Err(e) => {
                    log::warn!("{e}");
                    return (None, false);
                }
            }
        };
        let repeat = entry.is_none();
        if !repeat {
            self.journal_changed(app);
        }
        (entry, repeat)
    }

    /// Runs `f` on the journal of the character last played.
    fn with_journal<T>(&self, app: &AppHandle, f: impl FnOnce(&Journal, journal::Scope) -> Result<T, String>) -> Result<T, String> {
        let store = self.journal_store(app);
        let journal = store.journal.as_ref().ok_or("The journal couldn't be opened.")?;
        let world = store.world.ok_or("Nothing's in the journal until you've played.")?;
        f(journal, journal::Scope { world, character: &store.character })
    }

    /// Emits `journal-changed`: what's still unheard.
    fn journal_changed(&self, app: &AppHandle) {
        match self.journal_unheard(app) {
            Ok(unheard) => {
                let _ = app.emit("journal-changed", unheard);
            }
            Err(e) => log::warn!("{e}"),
        }
    }

    /// What's still unheard in the journal and the log.
    pub fn journal_unheard(&self, app: &AppHandle) -> Result<journal::Unheard, String> {
        let store = self.journal_store(app);
        let (Some(journal), Some(world)) = (store.journal.as_ref(), store.world) else { return Ok(journal::Unheard::default()) };
        journal.unheard(journal::Scope { world, character: &store.character })
    }

    /// A book's lines matching `query`, narrowed to `who` when given.
    pub fn journal_list(&self, app: &AppHandle, book: &str, query: &str, who: Option<&str>) -> Result<journal::Listing, String> {
        let book = Book::parse(book)?;
        self.with_journal(app, |j, scope| j.list(scope, book, query, who, JOURNAL_SHOWN))
    }

    /// A typed channel LAST (`OOC LAST 10`), answered from the journal
    /// rather than sent: the channel and its newest lines, or None when
    /// the line isn't one (the game's to answer).
    pub fn journal_last(&self, app: &AppHandle, line: &str) -> Result<Option<journal::LastAnswer>, String> {
        let Some((word, count)) = journal::last_asked(line) else { return Ok(None) };
        let found = self.with_journal(app, |j, scope| j.last(scope, &word, count))?;
        Ok(found.map(|(channel, entries)| journal::LastAnswer { channel, count, entries }))
    }

    /// A book's unheard lines, oldest first.
    pub fn journal_unheard_entries(&self, app: &AppHandle, book: &str) -> Result<Vec<journal::Entry>, String> {
        let book = Book::parse(book)?;
        self.with_journal(app, |j, scope| j.unheard_entries(scope, book, JOURNAL_SHOWN))
    }

    /// Marks lines heard or not.
    pub fn journal_set_heard(&self, app: &AppHandle, ids: &[i64], heard: bool) -> Result<(), String> {
        let changed = self.journal_store(app).journal.as_ref().ok_or("The journal couldn't be opened.")?.set_heard(ids, heard)?;
        if changed > 0 {
            self.journal_changed(app);
        }
        Ok(())
    }

    /// Marks a whole book heard.
    pub fn journal_heard_all(&self, app: &AppHandle, book: &str) -> Result<(), String> {
        let book = Book::parse(book)?;
        if self.with_journal(app, |j, scope| j.heard_all(scope, book))? > 0 {
            self.journal_changed(app);
        }
        Ok(())
    }

    // ---- The Bestiary and the Items (bestiary.rs) ----

    /// The Bestiary, opened first if it hasn't been.
    fn bestiary_store(&self, app: &AppHandle) -> MutexGuard<'_, BestiaryStore> {
        let mut store = self.bestiary.lock().unwrap();
        if !store.opened {
            store.opened = true;
            let open = |path: &Path| {
                std::fs::create_dir_all(path.parent().expect("the file is inside the app-data dir")).map_err(|e| e.to_string())?;
                Bestiary::open(path)
            };
            let opened = bestiary_path(app).and_then(|path| {
                open(&path).or_else(|e| {
                    // Start fresh rather than lose play; the old file is kept aside.
                    log::warn!("{e}");
                    let _ = std::fs::rename(&path, path.with_extension("sqlite.unreadable"));
                    open(&path)
                })
            });
            // Rather than refuse to play: a Bestiary for this run only.
            store.bestiary = opened
                .or_else(|e| {
                    log::warn!("{e}");
                    Bestiary::in_memory()
                })
                .map_err(|e| log::warn!("{e}"))
                .ok();
        }
        store
    }

    /// Keeps facts in the world's Bestiary. Returns whether there were any.
    fn bestiary_keep(store: &BestiaryStore, facts: &[Fact]) -> bool {
        let (Some(world), Some(bestiary)) = (store.world, store.bestiary.as_ref()) else { return false };
        let at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
        for fact in facts {
            if let Err(e) = bestiary.record(world, store.reader.area(), fact, at) {
                log::warn!("{e}");
            }
        }
        !facts.is_empty()
    }

    /// A creature's cry, by what the Bestiary knows of its race.
    fn bestiary_cry(store: &BestiaryStore, name: &str, cry: Cry) -> Option<(Creature, Cry)> {
        let world = store.world?;
        store.bestiary.as_ref()?.creature(world, name).map(|c| (c, cry))
    }

    /// A GMCP message for the Bestiary. Returns whether it learned
    /// anything, and the cry of a creature that came into the room (not
    /// someone the cast knows for a player).
    fn bestiary_gmcp(&self, app: &AppHandle, package: &str, data: &str) -> (bool, Option<(Creature, Cry)>) {
        let (facts, entered) = self.bestiary_store(app).reader.gmcp(package, data);
        self.bestiary_queue(app, &facts);
        let entered = entered.filter(|name| {
            let cast = self.cast.lock().unwrap();
            cast.cast.character(&crate::senses::letters_and_digits(name)).is_none_or(|c| c.who != cast::Who::Pc)
        });
        let store = self.bestiary_store(app);
        let changed = Self::bestiary_keep(&store, &facts);
        (changed, entered.and_then(|name| Self::bestiary_cry(&store, &name, Cry::Comes)))
    }

    /// Creatures just seen that have never been looked at, waiting for
    /// Coupler's own look, when the player has asked for it.
    fn bestiary_queue(&self, app: &AppHandle, facts: &[Fact]) {
        let mut store = self.bestiary_store(app);
        if !store.auto {
            return;
        }
        let Some(world) = store.world else { return };
        let wanted: Vec<String> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Seen { name } => Some(name.clone()),
                _ => None,
            })
            .filter(|name| store.bestiary.as_ref().is_some_and(|b| b.wants_look(world, name)) && !store.to_look.contains(name))
            .collect();
        store.to_look.extend(wanted);
    }

    /// The player's choice: Coupler looks at each creature never looked
    /// at, once, unobtrusively.
    pub fn bestiary_auto_look(&self, app: &AppHandle, on: bool) {
        let mut store = self.bestiary_store(app);
        store.auto = on;
        if !on {
            store.to_look.clear();
            return;
        }
        // Those in the room already.
        let here: Vec<Fact> = store.reader.here().map(|name| Fact::Seen { name: name.to_string() }).collect();
        drop(store);
        self.bestiary_queue(app, &here);
    }

    /// On WHO's clock, when a command of Coupler's own would go unnoticed
    /// (`Who::gentle`) and no fight is on: a look at the next creature
    /// waiting, if it's still here and still unlooked at. Returns whether
    /// one was sent.
    fn bestiary_tick(&self, app: &AppHandle, now: Instant) -> bool {
        let fighting = self.combat.lock().unwrap().combat.opponent().is_some();
        let mut store = self.bestiary_store(app);
        if !store.auto || fighting || store.reader.busy() || store.looked_at.is_some_and(|t| now.duration_since(t) < AUTO_LOOK_EVERY) {
            return false;
        }
        let Some(world) = store.world else { return false };
        while let Some(name) = store.to_look.pop_front() {
            if !store.reader.here().any(|n| n == name) || !store.bestiary.as_ref().is_some_and(|b| b.wants_look(world, &name)) {
                continue;
            }
            let Some(line) = store.reader.auto_look(&name, now) else { continue };
            store.looked_at = Some(now);
            let at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
            if let Some(Err(e)) = store.bestiary.as_ref().map(|b| b.looked(world, &name, at)) {
                log::warn!("{e}");
            }
            drop(store);
            if let Err(e) = self.write(&telnet::encode_line(&line)) {
                log::warn!("{e}");
            }
            return true;
        }
        false
    }

    /// A read's lines for the Bestiary, as printed. Returns whether it
    /// learned anything, the cries of creatures that died, and the lines
    /// that answered Coupler's own look (not to be shown).
    /// `prompted`: the read ended on the game's prompt.
    fn bestiary_lines(&self, app: &AppHandle, texts: &[String], prompted: bool) -> (bool, Vec<(Creature, Cry)>, Vec<usize>) {
        let me = self.senses.lock().unwrap().senses.name().to_string();
        let mut store = self.bestiary_store(app);
        store.reader.character(&me);
        store.reader.examined(texts);
        let facts = store.reader.lines(texts, Instant::now());
        let hidden = store.reader.hidden().to_vec();
        if prompted {
            store.reader.prompted();
        }
        let changed = Self::bestiary_keep(&store, &facts);
        let cries = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Died { name } => Self::bestiary_cry(&store, name, Cry::Dies),
                _ => None,
            })
            .collect();
        (changed, cries, hidden)
    }

    /// The fight, for the Bestiary: the opponent's hit points in all
    /// (MSDP's), and its cry as a fight with someone new starts.
    fn bestiary_fight(&self, app: &AppHandle) -> (bool, Option<(Creature, Cry)>) {
        let opponent = self.combat.lock().unwrap().combat.opponent();
        let name = opponent.as_ref().and_then(|o| o.name.clone());
        let health = opponent.as_ref().and_then(|o| o.health_max);
        let mut store = self.bestiary_store(app);
        store.reader.opponent(name.as_deref());
        let mut changed = false;
        let mut cry = None;
        if name != store.opponent {
            store.opponent = name.clone();
            store.since = Some(Instant::now());
            store.health = None;
            cry = name.as_deref().and_then(|n| Self::bestiary_cry(&store, n, Cry::Fights));
        }
        let settled = store.since.is_some_and(|t| t.elapsed() > Duration::from_millis(1500));
        if let (Some(name), Some(max), true) = (name, health.filter(|h| *h > 0), settled) {
            if store.health != Some(max) {
                store.health = Some(max);
                changed = Self::bestiary_keep(&store, &[Fact::Health { name, max }]);
            }
        }
        (changed, cry)
    }

    /// Runs `f` on the Bestiary of the world last played.
    fn with_bestiary<T: Default>(&self, app: &AppHandle, f: impl FnOnce(&Bestiary, &str) -> Result<T, String>) -> Result<T, String> {
        let store = self.bestiary_store(app);
        let bestiary = store.bestiary.as_ref().ok_or("The Bestiary couldn't be opened.")?;
        match store.world {
            Some(world) => f(bestiary, world),
            None => Ok(T::default()),
        }
    }

    pub fn bestiary_mobs(&self, app: &AppHandle, query: &str) -> Result<Vec<bestiary::MobRow>, String> {
        self.with_bestiary(app, |b, world| b.mobs(world, query))
    }

    pub fn bestiary_mob(&self, app: &AppHandle, key: &str) -> Result<Option<bestiary::MobEntry>, String> {
        self.with_bestiary(app, |b, world| b.mob_entry(world, key))
    }

    pub fn bestiary_items(&self, app: &AppHandle, query: &str) -> Result<Vec<bestiary::ItemRow>, String> {
        self.with_bestiary(app, |b, world| b.items(world, query))
    }

    pub fn bestiary_item(&self, app: &AppHandle, key: &str) -> Result<Option<bestiary::ItemEntry>, String> {
        self.with_bestiary(app, |b, world| b.item_entry(world, key))
    }

    /// Creatures' cries, each recorded as its `coupler.creature` pair and
    /// played as the player's trigger on it, else the bank's.
    fn cry_sounds(&self, app: &AppHandle, cries: &[(Creature, Cry, Option<Place>)]) -> (bool, Vec<hooks::Fired>) {
        let bank = app.state::<soundbank::Sounds>().available();
        let values: Vec<String> = cries.iter().map(|(c, cry, _)| bestiary::cry_value(*c, *cry)).collect();
        let heard: Vec<_> = values
            .iter()
            .zip(cries)
            .map(|(value, (_, _, place))| Heard { key: bestiary::CREATURE_KEY, value, default: soundbank::creature_default(value).filter(|_| bank).map(|p| (p, 100)), under: None, place: place.clone() })
            .collect();
        self.own_sounds(app, &heard)
    }

    pub fn cast_list(&self) -> CastListing {
        let store = self.cast.lock().unwrap();
        CastListing { world: store.world, characters: store.cast.list() }
    }

    /// Changes the cast as the player asked, and saves it at once.
    pub fn cast_edit<T>(&self, app: &AppHandle, edit: impl FnOnce(&mut Cast) -> Result<T, String>) -> Result<T, String> {
        let done = {
            let mut store = self.cast.lock().unwrap();
            let done = edit(&mut store.cast)?;
            store.unsaved = true;
            done
        };
        self.save_cast(app, true);
        Ok(done)
    }

    // ---- The map ----

    /// Loads `world`'s saved map, unless it's the one already in memory.
    fn load_map(&self, app: &AppHandle, world: &'static str) {
        let mut atlas = self.atlas.lock().unwrap();
        if atlas.world == Some(world) {
            return;
        }
        let map = match map_path(app, world).and_then(|p| match std::fs::read_to_string(&p) {
            Ok(json) => Map::from_json(&json),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::default()),
            Err(e) => Err(format!("The saved map couldn't be opened. ({e})")),
        }) {
            Ok(map) => map,
            Err(e) => {
                // Start fresh rather than refuse to play; the old file is
                // kept aside so nothing explored is thrown away.
                log::warn!("{e}");
                if let Ok(p) = map_path(app, world) {
                    let _ = std::fs::rename(&p, p.with_extension("json.unreadable"));
                }
                Map::default()
            }
        };
        *atlas = Atlas { map, world: Some(world), ..Atlas::default() };
    }

    /// Writes the map if it changed: every time when `now`, otherwise no
    /// more often than `MAP_SAVE_EVERY`.
    fn save_map(&self, app: &AppHandle, now: bool) {
        let (json, world) = {
            let mut atlas = self.atlas.lock().unwrap();
            let Some(world) = atlas.world else { return };
            if !atlas.unsaved || (!now && atlas.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
                return;
            }
            atlas.unsaved = false;
            atlas.saved_at = Some(Instant::now());
            (atlas.map.to_json(), world)
        };
        let saved = map_path(app, world).and_then(|path| write_replacing(&path, json));
        if let Err(e) = saved {
            log::warn!("couldn't save the map: {e}");
            self.atlas.lock().unwrap().unsaved = true;
        }
    }

    fn emit_map(&self, app: &AppHandle) {
        let snapshot = self.map_snapshot();
        let _ = app.emit("map-changed", snapshot);
    }

    pub fn map_snapshot(&self) -> mapper::Snapshot {
        self.atlas.lock().unwrap().map.snapshot(MAP_RADIUS.0, MAP_RADIUS.1)
    }

    pub fn map_find(&self, query: &str) -> Vec<mapper::RoomRef> {
        self.atlas.lock().unwrap().map.find(query, 50)
    }

    /// The way to a room in words ("3 north, east"), without walking it.
    pub fn map_directions(&self, to: &str) -> Result<String, String> {
        let atlas = self.atlas.lock().unwrap();
        if atlas.map.current().is_none() {
            return Err("The map doesn't know where you are yet. Move or look once in the game.".into());
        }
        let route = atlas.map.route(to).ok_or("The map has no known way there from here.")?;
        Ok(if route.is_empty() { "You're already there.".into() } else { mapper::directions(&route) })
    }

    pub fn map_set_landmark(&self, app: &AppHandle, id: &str, name: &str) -> Result<(), String> {
        {
            let mut atlas = self.atlas.lock().unwrap();
            if !atlas.map.set_landmark(id, name) {
                return Err("The map doesn't know that room.".into());
            }
            atlas.unsaved = true;
        }
        self.save_map(app, true);
        self.emit_map(app);
        Ok(())
    }

    /// Forgets the loaded world's map.
    pub fn map_clear(&self, app: &AppHandle) {
        {
            let mut atlas = self.atlas.lock().unwrap();
            atlas.map.clear();
            atlas.walk.clear();
            atlas.unsaved = true;
        }
        self.save_map(app, true);
        self.emit_map(app);
    }

    /// Starts walking to a room: sends the first step, and `room_changed`
    /// sends each next one only once the game confirms the last, so the
    /// game is never sent a burst of commands. Returns the way in words.
    pub fn walk_to(&self, app: &AppHandle, to: &str) -> Result<String, String> {
        let (words, name) = {
            let mut atlas = self.atlas.lock().unwrap();
            if atlas.map.current().is_none() {
                return Err("The map doesn't know where you are yet. Move or look once in the game.".into());
            }
            let route = atlas.map.route(to).ok_or("The map has no known way there from here.")?;
            if route.is_empty() {
                return Err("You're already there.".into());
            }
            let words = mapper::directions(&route);
            atlas.walk = route.into();
            (words, atlas.map.room_name(to))
        };
        let _ = app.emit("map-walk", WalkEvent { walking: true, message: format!("Walking to {name}: {words}.") });
        self.take_step(app);
        Ok(words)
    }

    /// Ends a walk in progress, saying why. Does nothing when there isn't one.
    pub fn stop_walk(&self, app: &AppHandle, why: &str) {
        let was_walking = {
            let mut atlas = self.atlas.lock().unwrap();
            let was = !atlas.walk.is_empty();
            atlas.walk.clear();
            was
        };
        if was_walking {
            let _ = app.emit("map-walk", WalkEvent { walking: false, message: why.to_string() });
        }
    }

    /// Sends the walk's next step, opening a closed door first.
    fn take_step(&self, app: &AppHandle) {
        let lines = {
            let atlas = self.atlas.lock().unwrap();
            let Some(step) = atlas.walk.front() else { return };
            let way = mapper::dir_name(&step.dir);
            let mut lines = Vec::new();
            if atlas.map.exit(&step.dir).is_some_and(|e| e.door && !e.open) {
                lines.push(format!("open {way}"));
            }
            lines.push(way.to_string());
            lines
        };
        for line in lines {
            if let Err(e) = self.write(&telnet::encode_line(&line)) {
                self.stop_walk(app, &format!("Stopped walking. {e}"));
                return;
            }
        }
    }

    /// The game reported the room the player is in (after a batch of GMCP
    /// was applied): show it, save now and then, and carry a walk on.
    fn room_changed(&self, app: &AppHandle) {
        self.emit_map(app);
        self.save_map(app, false);
        enum Next {
            Nothing,
            Step,
            Arrived(String),
            Lost,
        }
        let next = {
            let mut atlas = self.atlas.lock().unwrap();
            let here = atlas.map.current().map(str::to_string);
            match atlas.walk.front() {
                None => Next::Nothing,
                Some(step) if here.as_deref() == Some(step.to.as_str()) => {
                    atlas.walk.pop_front();
                    if atlas.walk.is_empty() {
                        Next::Arrived(atlas.map.room_name(here.as_deref().unwrap_or("")))
                    } else {
                        Next::Step
                    }
                }
                // Still in the room the step leaves from (a look, or a
                // door opening): keep waiting for the move itself.
                Some(step) if atlas.map.exit(&step.dir).is_some_and(|e| e.to == step.to) => Next::Nothing,
                Some(_) => Next::Lost,
            }
        };
        match next {
            Next::Nothing => {}
            Next::Step => self.take_step(app),
            Next::Arrived(name) => {
                let _ = app.emit("map-walk", WalkEvent { walking: false, message: format!("Arrived: {name}.") });
            }
            Next::Lost => self.stop_walk(app, "Stopped walking: you ended up somewhere off the route."),
        }
    }

    /// The connection ended: the map no longer knows where the player is.
    fn left_the_game(&self, app: &AppHandle, why: &str) {
        self.stop_walk(app, why);
        self.atlas.lock().unwrap().map.leave();
        self.save_map(app, true);
        self.emit_map(app);
        self.ambient.lock().unwrap().ambient.leave();
        self.update_ambience(app);
        self.daytime.lock().unwrap().leave();
        self.update_picture(app);
        self.combat.lock().unwrap().combat.leave();
        self.update_combat(app);
        self.senses.lock().unwrap().senses.leave();
        self.update_vitals(app);
        self.cast.lock().unwrap().cast.leave();
        self.save_cast(app, true);
        self.who.lock().unwrap().leave();
        let mut songbook = self.songbook.lock().unwrap();
        songbook.finder.leave();
        if songbook.poster.take().is_some() {
            let _ = app.emit("music-written", MusicWritten { ok: false, message: "The connection ended before the music was written.".into() });
        }
    }

    /// Emits `vitals` when they changed: null when nothing is known.
    fn update_vitals(&self, app: &AppHandle) {
        let mut store = self.senses.lock().unwrap();
        let now = store.senses.vitals();
        if now != store.sent {
            store.sent = now.clone();
            let _ = app.emit("vitals", now);
        }
    }

    /// Emits `combat` when the fight changed: the opponent, or null when
    /// it's over.
    fn update_combat(&self, app: &AppHandle) {
        let mut store = self.combat.lock().unwrap();
        let now = store.combat.opponent();
        if now != store.sent {
            store.sent = now.clone();
            // Dev builds only, to check against the live game until
            // Combat mode shows it: a release never logs a fight.
            #[cfg(debug_assertions)]
            log::info!("combat {now:?}");
            let _ = app.emit("combat", now);
        }
    }

    /// The player logged out or switched character, still connected.
    /// A logout is like leaving the game: nowhere, no walk, no ambience.
    /// A switch stops the walk and sends the ambience again once the
    /// frontend has faded everything out: the new character's room has
    /// its own (its room.info has come by the time `char.base` names
    /// them), and its loops fade back in.
    fn character_left(&self, app: &AppHandle, left: Left) {
        *self.counted.lock().unwrap() = false;
        self.echoes.lock().unwrap().clear();
        self.long_look.lock().unwrap().clear();
        self.bestiary.lock().unwrap().reader.clear();
        let message = match &left {
            Left::Logout => {
                self.left_the_game(app, "Stopped walking: you logged out.");
                "Logged out: the sounds faded and the pictures closed.".to_string()
            }
            Left::Switch(name) => {
                self.stop_walk(app, "Stopped walking: you switched character.");
                self.combat.lock().unwrap().combat.leave();
                self.update_combat(app);
                self.senses.lock().unwrap().senses.leave();
                self.update_vitals(app);
                self.who.lock().unwrap().leave();
                format!("Now playing {name}: the sounds faded and the pictures closed.")
            }
        };
        let _ = app.emit("character-left", LeftEvent { message });
        if matches!(left, Left::Switch(_)) {
            self.ambient.lock().unwrap().sent = Ambience::default();
            self.update_ambience(app);
        }
    }

    /// A character is in the game: counted once (`played.json`) until the
    /// next connection, logout or switch.
    fn count_play(&self, app: &AppHandle) {
        {
            let mut counted = self.counted.lock().unwrap();
            if *counted {
                return;
            }
            *counted = true;
        }
        let json = serde_json::to_string(&Played { played: played(app) + 1 }).unwrap_or_default();
        if let Err(e) = played_path(app).and_then(|path| write_replacing(&path, json)) {
            log::warn!("couldn't count the play: {e}");
        }
    }

    /// Asks the game where the player is (GMCP `Room.Info`, answered at
    /// once) when the map doesn't know: after logging in, a reconnect,
    /// or a switch, since the game sends `room.info` by itself only when
    /// the room changes. Called on each `char.*` message, which comes
    /// only once a character is in the game; asks once until the map
    /// knows the room again, so it's gentle on the shared server.
    fn ask_where(&self) {
        let mut atlas = self.atlas.lock().unwrap();
        if atlas.map.current().is_some() {
            atlas.asked_where = false;
            return;
        }
        if atlas.asked_where {
            return;
        }
        atlas.asked_where = true;
        drop(atlas);
        if let Err(e) = self.write(&Telnet::gmcp("Room.Info", "")) {
            log::warn!("{e}");
        }
    }

    /// One GMCP message, for the map. Returns whether the room changed.
    fn map_gmcp(&self, app: &AppHandle, package: &str, data: &str) -> bool {
        let applied = match package.to_ascii_lowercase().as_str() {
            "room.info" => {
                let mut atlas = self.atlas.lock().unwrap();
                atlas.map.apply_room_info(data).map(|changed| {
                    atlas.unsaved |= changed;
                    true
                })
            }
            "room.exits" => {
                let mut atlas = self.atlas.lock().unwrap();
                atlas.map.apply_room_exits(data).map(|changed| {
                    atlas.unsaved |= changed;
                    true
                })
            }
            "room.wrongdir" => {
                self.stop_walk(app, "Stopped walking: the game says you can't go that way.");
                Ok(false)
            }
            _ => Ok(false),
        };
        applied.unwrap_or_else(|e| {
            log::warn!("{e}");
            false
        })
    }

    // ---- The hooks ----

    /// The hooks database, opened first if it hasn't been.
    fn hook_store(&self, app: &AppHandle) -> MutexGuard<'_, HookStore> {
        let mut store = self.hooks.lock().unwrap();
        if !store.opened {
            store.opened = true;
            let open = |path: &Path| {
                std::fs::create_dir_all(path.parent().expect("the file is inside the app-data dir")).map_err(|e| e.to_string())?;
                Hooks::open(path)
            };
            let opened = hooks_path(app).and_then(|path| {
                open(&path).or_else(|e| {
                    // Start fresh, as with the map; the old file is kept aside.
                    log::warn!("{e}");
                    let _ = std::fs::rename(&path, path.with_extension("sqlite.unreadable"));
                    open(&path)
                })
            });
            // Rather than refuse to play: hooks for this run only.
            store.hooks = opened
                .or_else(|e| {
                    log::warn!("{e}");
                    Hooks::in_memory()
                })
                .map_err(|e| log::warn!("{e}"))
                .ok();
            // The time of day is listed from the start, so a trigger can be
            // set on each part before the game has come round to it.
            if let Some(hooks) = store.hooks.as_mut() {
                let times = Phase::ALL.map(Phase::name);
                if let Err(e) = hooks.add_own(daytime::TIME_KEY, &times) {
                    log::warn!("{e}");
                }
                // So are the game's sounds, every one CoffeeMUD sends, and
                // Coupler's own events (foley.rs).
                let names = soundbank::msp_names();
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                if let Err(e) = hooks.add_own(msp::SOUND_KEY, &names) {
                    log::warn!("{e}");
                }
                if let Err(e) = hooks.add_own(foley::EVENT_KEY, &Happening::ALL.map(Happening::name)) {
                    log::warn!("{e}");
                }
                // And every creature's cry (bestiary.rs).
                let cries = bestiary::cry_values();
                if let Err(e) = hooks.add_own(bestiary::CREATURE_KEY, &cries.iter().map(String::as_str).collect::<Vec<_>>()) {
                    log::warn!("{e}");
                }
            }
        }
        store
    }

    /// How many unique GMCP pairs the hooks list holds.
    pub fn hooks_count(&self, app: &AppHandle) -> usize {
        self.hook_store(app).hooks.as_ref().map_or(0, Hooks::len)
    }

    /// The hooks matching `query` (every one when it's empty), the first
    /// `HOOKS_SHOWN` of them.
    pub fn hooks_list(&self, app: &AppHandle, query: &str) -> Result<hooks::Listing, String> {
        let store = self.hook_store(app);
        let hooks = store.hooks.as_ref().ok_or("The hooks database couldn't be opened.")?;
        hooks.list(query, HOOKS_SHOWN)
    }

    /// The bank's defaults, each with the player's own sound that takes
    /// its place (`soundbank::defaults`).
    pub fn soundbank_defaults(&self, app: &AppHandle) -> Result<soundbank::Defaults, String> {
        let store = self.hook_store(app);
        let hooks = store.hooks.as_ref().ok_or("The hooks database couldn't be opened.")?;
        let rooms = hooks.triggers().iter().filter(|(key, _, t)| *key == "room.info.id" && t.bgn.is_some()).count();
        Ok(soundbank::defaults(|key, value| hooks.trigger(key, value).cloned(), rooms))
    }

    /// The filtered keys matching `query`, the first `HOOKS_SHOWN` of them.
    pub fn hooks_filtered(&self, app: &AppHandle, query: &str) -> Result<hooks::FilteredListing, String> {
        let store = self.hook_store(app);
        let hooks = store.hooks.as_ref().ok_or("The hooks database couldn't be opened.")?;
        hooks.filtered_list(query, HOOKS_SHOWN)
    }

    /// Moves a key out of the hooks list (filtered), or back into it.
    pub fn hooks_set_filtered(&self, app: &AppHandle, key: &str, filtered: bool) -> Result<(), String> {
        {
            let mut store = self.hook_store(app);
            let hooks = store.hooks.as_mut().ok_or("The hooks database couldn't be opened.")?;
            hooks.set_filtered(key, filtered)?;
        }
        self.hooks_changed(app);
        Ok(())
    }

    /// Sets what a pair sets off when the game sends it; an empty
    /// trigger removes it.
    pub fn hooks_set_trigger(&self, app: &AppHandle, key: &str, value: &str, trigger: hooks::Trigger) -> Result<(), String> {
        {
            let mut store = self.hook_store(app);
            let hooks = store.hooks.as_mut().ok_or("The hooks database couldn't be opened.")?;
            hooks.set_trigger(key, value, trigger)?;
        }
        // A BGN or BGW set for where the player is plays now, and an ART for the room shows.
        self.update_ambience(app);
        self.update_picture(app);
        Ok(())
    }

    /// How many triggers use each asset.
    pub fn hooks_asset_uses(&self, app: &AppHandle) -> HashMap<String, usize> {
        self.hook_store(app).hooks.as_ref().map(Hooks::asset_uses).unwrap_or_default()
    }

    /// Clears assets `exists` says are gone from every trigger naming them.
    pub fn hooks_clear_missing(&self, app: &AppHandle, exists: impl Fn(&str) -> bool) -> Result<Vec<hooks::Cleared>, String> {
        let cleared = {
            let mut store = self.hook_store(app);
            let hooks = store.hooks.as_mut().ok_or("The hooks database couldn't be opened.")?;
            hooks.clear_missing(exists)?
        };
        if !cleared.is_empty() {
            self.update_ambience(app);
            self.update_picture(app);
        }
        Ok(cleared)
    }

    // ---- Music in the journals (songbook.rs) ----

    /// Starts writing a song into a journal as TMF: WRITE is sent, and
    /// each of its questions answered as it comes (`songbook_read`).
    /// Returns how many characters the music is as text.
    pub fn music_write(&self, journal: &str, song: &music::Song, title: &str) -> Result<usize, String> {
        if !self.who.lock().unwrap().acting() {
            return Err("Music is written into a journal from the game: come into it first.".into());
        }
        let mut songbook = self.songbook.lock().unwrap();
        if songbook.poster.is_some() {
            return Err("Coupler is writing music into a journal already.".into());
        }
        let text = music::to_tmf(song, title);
        let (poster, first) = Poster::new(journal, title, text, Instant::now())?;
        let characters = poster.characters();
        self.write(&telnet::encode_line(&first))?;
        songbook.poster = Some(poster);
        Ok(characters)
    }

    /// A read's lines and prompt, for the WRITE under way: its next
    /// answer sent, unechoed, or how it ended.
    fn songbook_read(&self, app: &AppHandle, texts: &[String], prompt: Option<&str>) {
        let mut songbook = self.songbook.lock().unwrap();
        let Some(poster) = songbook.poster.as_mut() else { return };
        match poster.read(texts, prompt, Instant::now()) {
            Some(songbook::Step::Send(line)) => {
                drop(songbook);
                if let Err(e) = self.write(&telnet::encode_line(&line)) {
                    self.songbook.lock().unwrap().poster = None;
                    let _ = app.emit("music-written", MusicWritten { ok: false, message: e });
                }
            }
            Some(songbook::Step::Done(done)) => {
                songbook.poster = None;
                let (ok, message) = match done {
                    Ok(()) => (true, "The music is in the journal: anyone who reads that entry with Coupler gets the song.".to_string()),
                    Err(e) => (false, e),
                };
                let _ = app.emit("music-written", MusicWritten { ok, message });
            }
            None => {}
        }
    }

    /// Gives up a WRITE the game stopped answering.
    fn songbook_tick(&self, app: &AppHandle, now: Instant) {
        let mut songbook = self.songbook.lock().unwrap();
        if let Some(why) = songbook.poster.as_ref().and_then(|p| p.timed_out(now)) {
            songbook.poster = None;
            let _ = app.emit("music-written", MusicWritten { ok: false, message: why });
        }
    }

    /// Music found in the game's lines, for the Music Editor.
    fn music_found(app: &AppHandle, text: &str, subject: Option<String>) {
        let found = match music::from_tmf(text) {
            Ok((song, title)) => {
                let title = if title.trim().is_empty() { subject.unwrap_or_else(|| "Music from a journal".into()) } else { title };
                MusicFound { title, song: Some(song), characters: text.len(), problem: None }
            }
            Err(e) => MusicFound { title: subject.unwrap_or_else(|| "Music from a journal".into()), song: None, characters: text.len(), problem: Some(e) },
        };
        let _ = app.emit("music-found", found);
    }

    /// Hooks music (an asset's path) to where the player is, the room or
    /// its area, as the place's music (`ambient.rs`); None unhooks it.
    /// Returns the place in words.
    pub fn music_hook(&self, app: &AppHandle, scope: &str, path: Option<&str>) -> Result<String, String> {
        let (key, value, words) = self.ambient.lock().unwrap().ambient.place(scope).ok_or_else(|| match scope {
            "room" | "area" => "Coupler doesn't know where you are yet: LOOK, then try again.".to_string(),
            _ => format!("Music is hooked to a room or an area, not \"{scope}\"."),
        })?;
        let mut trigger = self.hook_store(app).hooks.as_ref().ok_or("The hooks database couldn't be opened.")?.trigger(key, &value).cloned().unwrap_or_default();
        if path.is_none() && trigger.bgm.is_none() {
            return Err(format!("No music is hooked to {words}."));
        }
        trigger.bgm = path.map(str::to_string);
        trigger.bgm_loop = true;
        self.hooks_set_trigger(app, key, &value, trigger)?;
        self.hooks_changed(app);
        Ok(words)
    }

    // ---- Ambience: BGN and BGW (ambient.rs) ----

    /// A `room.info`: where the player is now, for the ambience, and its
    /// room type recorded as Coupler's own pair (with what that sets off).
    fn ambient_room(&self, app: &AppHandle, data: &str) -> (bool, Vec<hooks::Fired>) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else { return (false, Vec::new()) };
        let field = |name: &str| v.get(name).and_then(serde_json::Value::as_str).unwrap_or("").to_string();
        let room = ambient::Room { id: field("id"), zone: field("zone"), terrain: field("terrain").to_ascii_lowercase() };
        let kind = ambient::room_type(&room.terrain);
        self.picture.lock().unwrap().name = field("name");
        self.ambient.lock().unwrap().ambient.enter(room);
        match kind {
            Some(kind) => self.hooks_gmcp(app, ambient::ROOM_TYPE_KEY, &ambient::json_text(kind)),
            None => (false, Vec::new()),
        }
    }

    /// A finished line of the game's text: if it says what the weather
    /// is, that's kept for the area and recorded as Coupler's own pair.
    /// A line of the game's, read for the weather: whether the hooks
    /// gained a pair, what it set off, and whether it was the answer to
    /// Coupler's own WEATHER (not to be shown).
    fn ambient_line(&self, app: &AppHandle, text: &str) -> (bool, Vec<hooks::Fired>, bool) {
        let (kind, asked) = {
            let mut store = self.ambient.lock().unwrap();
            match store.ambient.reader.line(text) {
                Some(seen) => {
                    let asked = store.ambient.answered(Instant::now());
                    (store.ambient.saw(seen), asked)
                }
                None => (None, false),
            }
        };
        let (added, fired) = match kind {
            Some(kind) => self.hooks_gmcp(app, ambient::WEATHER_KEY, &ambient::json_text(kind)),
            None => (false, Vec::new()),
        };
        (added, fired, asked)
    }

    // ---- The time of day (daytime.rs) ----

    /// A finished line of the game's text, or MSDP's `WORLD_TIME`: a new
    /// time of day is recorded as Coupler's own pair, with what that
    /// sets off.
    fn daytime(&self, app: &AppHandle, said: Said) -> (bool, Vec<hooks::Fired>) {
        let now = Instant::now();
        let changed = {
            let mut daytime = self.daytime.lock().unwrap();
            match said {
                Said::Line(text) => daytime::read(text).and_then(|told| daytime.told(told, now)),
                Said::WorldTime(value) => daytime::world_time_hour(value).and_then(|hour| daytime.hour(hour, now)),
            }
        };
        match changed {
            Some(phase) => self.hooks_gmcp(app, daytime::TIME_KEY, &ambient::json_text(phase.name())),
            None => (false, Vec::new()),
        }
    }

    /// Works out what should loop now and tells the frontend if that
    /// changed. What the player's triggers leave silent plays the bank's
    /// loop for the terrain (the battle's in a fight) and the weather
    /// (`soundbank::fill_ambience`).
    fn update_ambience(&self, app: &AppHandle) {
        let night = self.daytime.lock().unwrap().now() == Some(Phase::Night);
        let fighting = self.combat.lock().unwrap().combat.opponent().is_some();
        let mut ambient = self.ambient.lock().unwrap();
        let mut now = {
            let store = self.hook_store(app);
            store.hooks.as_ref().map(|h| ambient.ambient.ambience_from(h)).unwrap_or_default()
        };
        if app.state::<soundbank::Sounds>().available() {
            let a = &ambient.ambient;
            soundbank::fill_ambience(&mut now, a.room.as_ref().map(|r| r.terrain.as_str()), a.weather_overhead(), night, fighting);
        }
        // The place's music goes quiet in a fight, and starts again after.
        if fighting {
            now.bgm = None;
        }
        if now != ambient.sent {
            ambient.sent = now.clone();
            let _ = app.emit("ambient", now);
        }
    }

    // ---- The room's picture (paint.rs, painter.rs) ----

    /// The room as a picture sees it, or None when the player is nowhere:
    /// with the look the player chose for it and their own ART for it.
    fn scene(&self, app: &AppHandle) -> Option<Scene> {
        let world = self.atlas.lock().unwrap().world?;
        self.load_looks(app, world);
        let (time, arc) = {
            let daytime = self.daytime.lock().unwrap();
            (daytime.now().map(|p| p.name().to_string()), daytime.arc(Instant::now()))
        };
        let mut scene = {
            let ambient = self.ambient.lock().unwrap();
            let room = ambient.ambient.room.as_ref()?;
            Scene {
                world: world.to_string(),
                room: room.id.clone(),
                zone: room.zone.clone(),
                terrain: room.terrain.clone(),
                weather: ambient.ambient.weather().map(str::to_string),
                time,
                arc,
                ..Scene::default()
            }
        };
        {
            let picture = self.picture.lock().unwrap();
            scene.name = picture.name.clone();
            scene.look = picture.looks.of(&scene.room);
        }
        let store = self.hook_store(app);
        scene.art = store.hooks.as_ref().and_then(|h| h.trigger("room.info.id", &ambient::json_text(&scene.room))?.art.clone());
        Some(scene)
    }

    /// Reads a world's chosen looks, once per world.
    fn load_looks(&self, app: &AppHandle, world: &'static str) {
        let mut store = self.picture.lock().unwrap();
        if store.world == Some(world) {
            return;
        }
        store.world = Some(world);
        store.looks = match looks_path(app, world).and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string())) {
            Ok(json) => Looks::from_json(&json).unwrap_or_else(|e| {
                log::warn!("{e}");
                Looks::default()
            }),
            Err(_) => Looks::default(),
        };
    }

    fn save_looks(&self, app: &AppHandle) {
        let (json, world) = {
            let store = self.picture.lock().unwrap();
            let Some(world) = store.world else { return };
            (store.looks.to_json(), world)
        };
        if let Err(e) = looks_path(app, world).and_then(|path| write_replacing(&path, json)) {
            log::warn!("couldn't save the rooms' looks: {e}");
        }
    }

    /// Tells the frontend when the room's picture should change: a new
    /// room, weather, time of day, look or ART, or null when the player's nowhere.
    fn update_picture(&self, app: &AppHandle) {
        let now = self.scene(app);
        let mut store = self.picture.lock().unwrap();
        if now != store.sent {
            store.sent = now.clone();
            let _ = app.emit("picture", now);
        }
    }

    /// A new look for the room the player is in, kept as its look.
    pub fn picture_again(&self, app: &AppHandle) -> Result<Option<Scene>, String> {
        let room = self.picture.lock().unwrap().sent.as_ref().map(|s| s.room.clone()).ok_or("You're not in a room Coupler knows yet.")?;
        self.picture.lock().unwrap().looks.again(&room);
        self.save_looks(app);
        self.update_picture(app);
        Ok(self.picture_now())
    }

    /// Every room back to its first look.
    pub fn picture_forget_looks(&self, app: &AppHandle) -> Result<(), String> {
        if let Some(world) = self.atlas.lock().unwrap().world {
            self.load_looks(app, world);
        }
        self.picture.lock().unwrap().looks.forget();
        self.save_looks(app);
        self.update_picture(app);
        Ok(())
    }

    /// The room the picture shows now, for a frontend that has just started.
    pub fn picture_now(&self) -> Option<Scene> {
        self.picture.lock().unwrap().sent.clone()
    }

    /// What's looping now, for a frontend that has just started.
    pub fn ambience(&self) -> Ambience {
        self.ambient.lock().unwrap().sent.clone()
    }

    /// The game's sounds (`msp.rs`), for the hooks: each recorded as its
    /// pair, setting off the player's trigger on it, and one the player
    /// gave no sound plays the bank's for its name (`soundbank::msp_default`).
    /// Returns whether a pair was new to the list, and what to play.
    fn msp_sounds(&self, app: &AppHandle, sounds: &[msp::Sound]) -> (bool, Vec<hooks::Fired>) {
        let bank = app.state::<soundbank::Sounds>().available();
        let heard: Vec<_> = sounds
            .iter()
            .map(|sound| Heard { key: sound.key(), value: &sound.name, default: soundbank::msp_default(&sound.name).filter(|_| bank && !sound.music).map(|p| (p, 100)), under: None, place: None })
            .collect();
        self.own_sounds(app, &heard)
    }

    /// Coupler's own events (`foley.rs`), the same way: each recorded as
    /// its `coupler.event` pair, and one the player gave no sound plays
    /// the bank's (`soundbank::event_default`), heard from where someone
    /// came or went.
    fn event_sounds(&self, app: &AppHandle, happenings: &[Movement]) -> (bool, Vec<hooks::Fired>) {
        let bank = app.state::<soundbank::Sounds>().available();
        let heard: Vec<_> = happenings
            .iter()
            .map(|m| Heard {
                key: foley::EVENT_KEY,
                value: m.happening.name(),
                default: soundbank::event_default(m.happening).filter(|_| bank),
                under: soundbank::event_under(m.happening).filter(|_| bank),
                place: m.place.clone(),
            })
            .collect();
        self.own_sounds(app, &heard)
    }

    /// Pairs of Coupler's own, recorded, each setting off the player's
    /// trigger on it, or where that gives no sound effect, the bank's.
    fn own_sounds(&self, app: &AppHandle, heard: &[Heard]) -> (bool, Vec<hooks::Fired>) {
        let mut store = self.hook_store(app);
        let mut added = false;
        let mut fired = Vec::new();
        for Heard { key, value: name, default, under, place } in heard {
            let value = ambient::json_text(name);
            let mut set_off = match store.hooks.as_mut().map(|h| h.record(key, &value)) {
                Some(Ok(r)) => {
                    added |= r.added > 0;
                    r.fired
                }
                Some(Err(e)) => {
                    log::warn!("{e}");
                    Vec::new()
                }
                None => Vec::new(),
            };
            let silent = set_off.iter().all(|f| f.trigger.sfx.is_none());
            if let Some((path, volume)) = default.clone().filter(|_| silent) {
                let bank = |path| Trigger { sfx: Some(path), sfx_volume: volume, ..Trigger::default() };
                match set_off.first_mut() {
                    Some(f) => {
                        f.trigger.sfx = Some(path);
                        f.trigger.sfx_volume = volume;
                    }
                    None => set_off.push(hooks::Fired { key: (*key).into(), value: value.clone(), trigger: bank(path), place: None }),
                }
                if let Some(under) = under.clone() {
                    set_off.push(hooks::Fired { key: (*key).into(), value, trigger: bank(under), place: None });
                }
            }
            for f in &mut set_off {
                f.place = place.clone();
            }
            fired.extend(set_off);
        }
        (added, fired)
    }

    /// One GMCP message, for the hooks. Returns whether it held a pair
    /// new to the list, and the triggers it set off.
    fn hooks_gmcp(&self, app: &AppHandle, package: &str, data: &str) -> (bool, Vec<hooks::Fired>) {
        let mut store = self.hook_store(app);
        let Some(hooks) = store.hooks.as_mut() else { return (false, Vec::new()) };
        hooks.record(package, data).map(|r| (r.added > 0, r.fired)).unwrap_or_else(|e| {
            log::warn!("{e}");
            (false, Vec::new())
        })
    }

    /// The hooks list changed (after a batch of GMCP, or a key was
    /// filtered): show the new total.
    fn hooks_changed(&self, app: &AppHandle) {
        let count = self.hooks_count(app);
        let _ = app.emit("hooks-changed", count);
    }
}

/// One of Coupler's own pairs heard (`Session::own_sounds`): its key and
/// value, the bank's sound for it and how loud (a share of SFX), what the
/// bank plays under it, and where it's heard from.
struct Heard<'a> {
    key: &'a str,
    value: &'a str,
    default: Option<(String, u8)>,
    under: Option<String>,
    place: Option<Place>,
}

/// What the game said that can tell the time of day (`daytime.rs`).
enum Said<'a> {
    Line(&'a str),
    WorldTime(&'a str),
}

/// What's said when the game hangs up (a QUIT).
const CLOSED: &str = "CoffeeMUD closed the connection.";

/// Reads until the socket closes. Returns why, for the status line.
fn read_loop(app: &AppHandle, mut stream: TcpStream, telnet: &Mutex<Telnet>) -> Option<String> {
    let mut writer = stream.try_clone().ok();
    let mut screen = Screen::default();
    let mut kinds = Kinds::default();
    // The unfinished line last sent, kept while a hidden WHO is cut off mid-row.
    let mut shown_partial: Option<Line> = None;
    // The tip about SOUNDS is given once a connection.
    let mut told_sounds_off = false;
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => return Some(CLOSED.into()),
            Ok(n) => n,
            // A QUIT the game ends with a reset rather than a close: it's
            // still the game hanging up, not the line dropping.
            Err(e) if matches!(e.kind(), std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted) => return Some(CLOSED.into()),
            // Our own shutdown lands here too; `disconnect` has already
            // reported it, and the id check drops this one.
            Err(e) => return Some(format!("The connection to CoffeeMUD dropped. ({e})")),
        };
        let out = match telnet.lock().unwrap().feed(&buf[..n]) {
            Ok(out) => out,
            Err(e) => return Some(e),
        };
        if !out.replies.is_empty() {
            if let Some(w) = writer.as_mut() {
                if w.write_all(&out.replies).is_err() {
                    writer = None;
                }
            }
        }
        let mut lines = Vec::new();
        let mut room_changed = false;
        // A `char.*` message came: a character is in the game.
        let mut in_game = false;
        let mut hooks_changed = false;
        // Someone new was met: an open Characters' Voices list reads again.
        let mut met = false;
        let mut fired = Vec::new();
        // The game's sounds, cut out of its text.
        let mut sounds = Vec::new();
        // Coupler's own events, from the text and GMCP (foley.rs).
        let mut happenings = Vec::new();
        // Creatures' cries, and whether the Bestiary learned anything.
        let mut cries: Vec<(Creature, Cry, Option<Place>)> = Vec::new();
        let mut bestiary_changed = false;
        // Journal entries' music read whole (songbook.rs): its text and the entry's subject.
        let mut music_found: Vec<(String, Option<String>)> = Vec::new();
        let session = app.state::<Session>();
        for event in out.events {
            match event {
                Event::Text(bytes) => {
                    let mut done = screen.push(&bytes);
                    // The game's sounds come out of the text (msp.rs); a
                    // line that held nothing else isn't shown.
                    done.retain_mut(|line| {
                        let blank = line.is_empty();
                        sounds.extend(msp::take(line));
                        blank || !line.is_empty()
                    });
                    // Coupler's own WHO and the login announcements don't show.
                    {
                        let mut who = session.who.lock().unwrap();
                        let now = Instant::now();
                        done.retain(|line| who.line(&line.iter().map(|span| span.text.as_str()).collect::<String>(), now) == Seen::Show);
                    }
                    // Music in a journal entry (songbook.rs): its TMF isn't shown or said.
                    {
                        let mut songbook = session.songbook.lock().unwrap();
                        done.retain(|line| match songbook.finder.line(&line.iter().map(|span| span.text.as_str()).collect::<String>()) {
                            songbook::Seen::Show | songbook::Seen::Broken => true,
                            songbook::Seen::Hide => false,
                            songbook::Seen::Done { text, subject } => {
                                music_found.push((text, subject));
                                false
                            }
                        });
                    }
                    let mut answer = Vec::new();
                    for (i, line) in done.iter().enumerate() {
                        let text: String = line.iter().map(|span| span.text.as_str()).collect();
                        let (added, set_off, asked) = session.ambient_line(app, &text);
                        hooks_changed |= added;
                        fired.extend(set_off);
                        if asked {
                            answer.push(i);
                        }
                        let (added, set_off) = session.daytime(app, Said::Line(&text));
                        hooks_changed |= added;
                        fired.extend(set_off);
                        happenings.extend(foley::line(&text).map(|happening| Movement { happening, place: None }));
                        session.comings.lock().unwrap().line(&text, Instant::now());
                    }
                    // Coupler's own WEATHER's answer doesn't show either.
                    let mut i = 0;
                    done.retain(|_| {
                        i += 1;
                        !answer.contains(&(i - 1))
                    });
                    lines.extend(done);
                }
                // The unfinished line is sent as `partial` after every read
                // anyway, so a prompt shows whether or not GA marks it.
                Event::Prompt => {}
                Event::ServerEcho(on) => {
                    session.who.lock().unwrap().echo(on);
                    let _ = app.emit("mud-echo", on);
                }
                // On: the bank's sounds are made ready. Off at login: the
                // character's SOUNDS is off, which only the player turns on.
                Event::Msp(true) => {
                    let app = app.clone();
                    std::thread::spawn(move || app.state::<soundbank::Sounds>().warm());
                }
                Event::Msp(false) => {
                    if !std::mem::replace(&mut told_sounds_off, true) {
                        let _ = app.emit("sounds-off", msp::SOUNDS_OFF);
                    }
                }
                Event::Msdp(name, value) => {
                    if session.combat.lock().unwrap().combat.msdp(&name, &value) {
                        continue;
                    }
                    if name == "WORLD_TIME" {
                        let (added, set_off) = session.daytime(app, Said::WorldTime(&value));
                        hooks_changed |= added;
                        fired.extend(set_off);
                    }
                }
                Event::Gmcp(package, data) => {
                    log::debug!("GMCP {package}");
                    let left = session.character.lock().unwrap().gmcp(&package, &data);
                    // Emitted now, so what this read sets off (the new
                    // character's room, which comes before their char.base)
                    // plays after the fade, not under it.
                    if let Some(left) = left {
                        session.character_left(app, left);
                    }
                    session.combat.lock().unwrap().combat.gmcp(&package, &data);
                    let moved = session.comings.lock().unwrap().gmcp(&package, &data, Instant::now());
                    let (learned, cry) = session.bestiary_gmcp(app, &package, &data);
                    bestiary_changed |= learned;
                    if let Some((creature, cry)) = cry {
                        // Heard from where its footsteps are.
                        cries.push((creature, cry, moved.as_ref().and_then(|m| m.place.clone())));
                    }
                    happenings.extend(moved);
                    let talk: Option<Talk> = session.senses.lock().unwrap().senses.gmcp(&package, &data);
                    // A login or logout on a channel is a sound, not talk.
                    let talk = talk.filter(|t| !session.who.lock().unwrap().heard(&t.text, Instant::now()));
                    // Kept once its lines are read, as they were printed (speech.rs `heard`).
                    if let Some(talk) = talk {
                        kinds.talk(talk, Instant::now());
                    }
                    met |= session.cast_gmcp(&package, &data);
                    room_changed |= session.map_gmcp(app, &package, &data);
                    in_game |= package.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("char."));
                    if package.eq_ignore_ascii_case("room.info") {
                        // The new character's in the game: the guide closes.
                        if session.account.lock().unwrap().entered() {
                            let _ = app.emit("account-menu", None::<account::Menu>);
                        }
                        if session.creation.lock().unwrap().entered() {
                            let _ = app.emit("creation", None::<creation::Step>);
                        }
                        let (added, set_off) = session.ambient_room(app, &data);
                        hooks_changed |= added;
                        fired.extend(set_off);
                    }
                    let (added, set_off) = session.hooks_gmcp(app, &package, &data);
                    hooks_changed |= added;
                    fired.extend(set_off);
                    let _ = app.emit("mud-gmcp", GmcpEvent { package, data });
                }
            }
        }
        // Talk, as the player saw it: a mood's words, a language
        // scrambled or translated, a whisper (speech.rs `heard`).
        let mut texts: Vec<String> = lines.iter().map(|line| line.iter().map(|span| span.text.as_str()).collect()).collect();
        // The Bestiary reads them (bestiary.rs); the answer to Coupler's
        // own look isn't shown, nor the prompt it was asked at, written
        // again as a line when the answer began.
        let prompted = screen.partial().is_some_and(|line| line.iter().any(|span| !span.text.trim().is_empty()));
        let (learned, died, mut hidden_lines) = session.bestiary_lines(app, &texts, prompted);
        bestiary_changed |= learned;
        cries.extend(died.into_iter().map(|(c, cry)| (c, cry, None)));
        if let Some(&first) = hidden_lines.first() {
            let before = (0..first).rev().find(|&i| !texts[i].trim().is_empty() || i == 0);
            let prompt = shown_partial.as_ref().map(|line| line.iter().map(|span| span.text.as_str()).collect::<String>());
            if let (Some(i), Some(prompt)) = (before, prompt) {
                if texts[i].trim() == prompt.trim() {
                    hidden_lines.extend(i..first);
                }
            }
            let mut i = 0;
            lines.retain(|_| {
                i += 1;
                !hidden_lines.contains(&(i - 1))
            });
            let mut i = 0;
            texts.retain(|_| {
                i += 1;
                !hidden_lines.contains(&(i - 1))
            });
        }
        // The player's gags, and what their triggers set off (automation.rs).
        let acting = session.who.lock().unwrap().acting();
        let (gagged, actions) = session.automation.lock().unwrap().lines(&texts, acting, Instant::now());
        if !gagged.is_empty() {
            let mut i = 0;
            lines.retain(|_| {
                i += 1;
                !gagged.contains(&(i - 1))
            });
            let mut i = 0;
            texts.retain(|_| {
                i += 1;
                !gagged.contains(&(i - 1))
            });
        }
        for line in &actions.send {
            session.send_auto(app, line);
        }
        session.automation_held(app);
        if !actions.say.is_empty() || !actions.sounds.is_empty() {
            let _ = app.emit("automation-fired", &actions);
        }
        if session.automation.lock().unwrap().take_recent_changed() {
            let _ = app.emit("automation-recent", session.automation_recent());
        }
        for talk in kinds.heard(&texts, Instant::now()) {
            let (event, new) = session.cast_talk(app, talk);
            met |= new;
            let _ = app.emit("talk", event);
        }
        // After the whole read, so a room.exits that came with the
        // room.info (doors, locks) is in before a walk takes its next step.
        if room_changed {
            session.room_changed(app);
        }
        // After the whole read too, so a room.info that came with the char.* isn't asked for again.
        if in_game {
            session.count_play(app);
            session.ask_where();
        }
        if !sounds.is_empty() {
            let (added, set_off) = session.msp_sounds(app, &sounds);
            hooks_changed |= added;
            fired.extend(set_off);
        }
        if !happenings.is_empty() {
            let (added, set_off) = session.event_sounds(app, &happenings);
            hooks_changed |= added;
            fired.extend(set_off);
        }
        let (learned, fights) = session.bestiary_fight(app);
        bestiary_changed |= learned;
        cries.extend(fights.map(|(c, cry)| (c, cry, None)));
        if !cries.is_empty() {
            let (added, set_off) = session.cry_sounds(app, &cries);
            hooks_changed |= added;
            fired.extend(set_off);
        }
        if bestiary_changed {
            let _ = app.emit("bestiary-changed", ());
        }
        if hooks_changed {
            session.hooks_changed(app);
        }
        if met {
            let _ = app.emit("cast-changed", ());
        }
        session.save_cast(app, false);
        session.update_ambience(app);
        // After the sounds: a picture never goes before them.
        session.update_picture(app);
        session.update_combat(app);
        session.update_vitals(app);
        // The frontend plays and shows them (src/lib/assets.ts); the
        // place's music plays as the ambience says.
        ambient::without_place_music(&mut fired);
        if !fired.is_empty() {
            let _ = app.emit("hook-fired", fired);
        }
        let mut partial = screen.partial().and_then(|mut line| {
            msp::tidy(&mut line);
            (!line.is_empty()).then_some(line)
        });
        let mut prompt = partial.as_ref().map(|line| line.iter().map(|span| span.text.as_str()).collect::<String>());
        {
            let mut who = session.who.lock().unwrap();
            let now = Instant::now();
            who.character(session.senses.lock().unwrap().senses.name(), now);
            if who.partial(prompt.as_deref(), now) {
                partial = shown_partial.clone();
                prompt = partial.as_ref().map(|line| line.iter().map(|span| span.text.as_str()).collect::<String>());
            }
        }
        shown_partial = partial.clone();
        session.who_changed(app);
        session.character.lock().unwrap().partial(prompt.as_deref());
        for (text, subject) in music_found {
            Session::music_found(app, &text, subject);
        }
        session.songbook_read(app, &texts, prompt.as_deref());
        let fighting = session.combat.lock().unwrap().combat.opponent().is_some();
        let mut line_kinds = kinds.lines(&texts, prompt.as_deref(), fighting, Instant::now());
        let playing = !session.senses.lock().unwrap().senses.name().is_empty();
        // Before the output, so it's known to be a question of the guide's.
        match session.creation.lock().unwrap().lines(&lines, &line_kinds, partial.as_ref(), playing) {
            Some(creation::Update::Step(step)) => {
                let _ = app.emit("creation", Some(step));
            }
            Some(creation::Update::Ended) => {
                let _ = app.emit("creation", None::<creation::Step>);
            }
            None => {}
        }
        // The account menu, for its dialog (account.rs); its list asked once.
        let menu = session.account.lock().unwrap().lines(&texts, prompt.as_deref());
        match menu {
            Some(account::Update::Open(menu)) => {
                let _ = app.emit("account-menu", Some(menu));
            }
            Some(account::Update::Closed) => {
                let _ = app.emit("account-menu", None::<account::Menu>);
            }
            None => {}
        }
        if session.account.lock().unwrap().wants_list() {
            if let Err(e) = session.write(&telnet::encode_line("L")) {
                log::warn!("{e}");
            }
        }
        // WHO waits while a character's being made (who.rs).
        let creating = session.creation.lock().unwrap().active();
        session.who.lock().unwrap().creating(creating);
        let echoes = session.echoes.lock().unwrap().lines(&texts, &mut line_kinds, prompt.as_deref(), playing, Instant::now());
        let hidden = {
            let atlas = session.atlas.lock().unwrap();
            session.long_look.lock().unwrap().lines(&lines, &line_kinds, atlas.map.here_name(), Instant::now())
        };
        let _ = app.emit("mud-output", OutputEvent { lines, kinds: line_kinds, echoes, hidden, partial });
    }
}

//! The web build's session: what the desktop's `session.rs` does with
//! each read, without the socket. The page (through `ffi.rs`) opens the game's WebSocket
//! (`connect` gives its address: `ports::web_socket_url`, the only one),
//! hands every binary frame to `feed`, sends what `take_outgoing` gives
//! back, and plays what `take_events` says, by the same event names and
//! payloads the desktop emits, so the UI can't tell them apart.
//!
//! The same but for what a browser can't have: the hooks database (the
//! triggers are the mirror's, `hooks.rs`; the bank's sounds are the
//! mirror's renders) and the voice cache. What the desktop keeps in app
//! data the page keeps in IndexedDB, from the `coupler-save` event: each
//! world's map, cast and rooms' looks under the desktop's own file names
//! (`maps/<world>.json`), and the journal and the Bestiary as the web's
//! own JSON (`journal_store.rs`, `bestiary_store.rs`), all of them read
//! back by `new` and `connect`.

use std::collections::VecDeque;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use crate::ambient::{self, Ambience, Ambient};
use crate::ansi::{Line, Screen};
use crate::automation::{self, Automation};
use crate::channels::Channels;
use crate::bestiary::{self, Bestiary, Creature, Cry, Fact, Reader};
use crate::cast::{self, Cast};
use crate::character::{Character, Left};
use crate::clock::{unix_ms, unix_now, Instant};
use crate::combat::{Combat, Opponent};
use crate::daytime::{self, Daytime, Phase};
use crate::creation::{self, Creation};
use crate::echo::{Echoed, Echoes};
use crate::foley::{self, Comings, Movement, Place};
use crate::hidden::{Hidden, LongLook};
use crate::hooks::{Fired, Hooks, Trigger};
use crate::journal::{self, Book, Journal};
use crate::mapper::{self, Map, Step};
use crate::msp;
use crate::paint::{Looks, Scene};
use crate::ports::{self, Port, HOST, PORTS};
use crate::senses::{letters_and_digits, Senses, Talk, TalkKind, Vitals};
use crate::soundbank;
use crate::speech::{Kinds, LineKind};
use crate::telnet::{self, Event, Telnet};
use crate::who::{Seen, Who};
use crate::ways::{Way, Ways};

/// As the desktop's.
const MAP_SAVE_EVERY: Duration = Duration::from_secs(20);
const MAP_RADIUS: (i32, i32) = (5, 3);
/// As the desktop's: how many lines the journal and the log show at once.
const JOURNAL_SHOWN: usize = 1000;
/// As the desktop's: how often Coupler looks at a creature by itself, at most.
const AUTO_LOOK_EVERY: Duration = Duration::from_secs(4);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputEvent {
    lines: Vec<Line>,
    kinds: Vec<LineKind>,
    echoes: Vec<Echoed>,
    hidden: Option<Hidden>,
    partial: Option<Line>,
}

#[derive(Serialize)]
struct TalkEvent {
    #[serde(flatten)]
    talk: Talk,
    voice: Option<cast::Voice>,
    entry: Option<journal::Entry>,
    /// A line this speaker has said before: not kept, and not spoken.
    repeat: bool,
}

/// What the Bestiary is reading from the game, as the desktop's
/// `BestiaryStore` but for the store itself.
#[derive(Default)]
struct Beasts {
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
    looked_at: Option<Instant>,
}

/// One of Coupler's own pairs heard, as the desktop's `Heard`.
struct Heard<'a> {
    key: &'a str,
    value: &'a str,
    default: Option<(String, u8)>,
    under: Option<String>,
    place: Option<Place>,
}

#[derive(Default)]
struct Saved {
    unsaved: bool,
    saved_at: Option<Instant>,
}

enum Said<'a> {
    Line(&'a str),
    WorldTime(&'a str),
}

pub struct Coupler {
    version: String,
    hooks: Hooks,
    telnet: Option<Telnet>,
    port: Option<&'static Port>,
    world: Option<&'static str>,
    size: (u16, u16),
    screen: Screen,
    kinds: Kinds,
    map: Map,
    map_saved: Saved,
    walk: VecDeque<Step>,
    asked_where: bool,
    cast: Cast,
    cast_saved: Saved,
    /// The journal and the log, every world's, and whose they are: the
    /// world and character last played, so they can be read after hanging up.
    journal: Journal,
    journal_world: Option<&'static str>,
    journal_character: String,
    journal_saved: Saved,
    /// The Bestiary and the Items, every world's; the world last played.
    bestiary: Bestiary,
    bestiary_world: Option<&'static str>,
    beasts: Beasts,
    bestiary_saved: Saved,
    /// The room's name from `room.info`, the picture last sent, and the
    /// world's chosen looks.
    picture_name: String,
    picture_sent: Option<Scene>,
    looks: Looks,
    /// Times a character came into the game here (the desktop's
    /// `played.json`), and whether this one was counted.
    played: u64,
    counted: bool,
    ambient: Ambient,
    ambience_sent: Ambience,
    daytime: Daytime,
    character: Character,
    combat: Combat,
    combat_sent: Option<Opponent>,
    senses: Senses,
    vitals_sent: Option<Vitals>,
    who: Who,
    account: crate::account::Account,
    echoes: Echoes,
    long_look: LongLook,
    creation: Creation,
    /// Portals, ships and caravans, read from the exits text (`ways.rs`).
    ways: Ways,
    /// The game's channels, turned on and off by its own commands (`channels.rs`).
    channels: Channels,
    /// The unfinished line last sent, kept while a hidden WHO is cut off mid-row.
    shown_partial: Option<Line>,
    /// The tip about SOUNDS was given this connection.
    told_sounds_off: bool,
    /// The lines that say how someone came or went (`foley.rs`).
    comings: Comings,
    /// The player's triggers, gags, aliases and tickers (`automation.rs`).
    automation: Automation,
    outgoing: Vec<u8>,
    events: Vec<Value>,
}

/// As the desktop's: something held back of the player's rules.
const HELD: &str = "Coupler held back commands of your triggers or tickers: too many at once. A trigger may be answering its own line.";

impl Coupler {
    /// `mirror` is the mirror's `hooks.json`, or empty. A mirror that
    /// can't be read means no hooks, never a Coupler that can't start
    /// (and so no ports to play); nor does a journal or Bestiary that
    /// can't be read (each starts afresh). `played` is the times played.
    pub fn new(version: &str, mirror: &str, journal: &str, bestiary: &str, played: u64) -> Result<Coupler, String> {
        Ok(Coupler {
            version: version.to_string(),
            hooks: Hooks::from_mirror(mirror).unwrap_or_default(),
            journal: Journal::from_json(journal).unwrap_or_default(),
            journal_world: None,
            journal_character: String::new(),
            journal_saved: Saved::default(),
            bestiary: Bestiary::from_json(bestiary).unwrap_or_default(),
            bestiary_world: None,
            beasts: Beasts::default(),
            bestiary_saved: Saved::default(),
            picture_name: String::new(),
            picture_sent: None,
            looks: Looks::default(),
            played,
            counted: false,
            telnet: None,
            port: None,
            world: None,
            size: (0, 0),
            screen: Screen::default(),
            kinds: Kinds::default(),
            map: Map::default(),
            map_saved: Saved::default(),
            walk: VecDeque::new(),
            asked_where: false,
            cast: Cast::default(),
            cast_saved: Saved::default(),
            ambient: Ambient::default(),
            ambience_sent: Ambience::default(),
            daytime: Daytime::default(),
            character: Character::default(),
            combat: Combat::default(),
            combat_sent: None,
            senses: Senses::default(),
            vitals_sent: None,
            who: Who::default(),
            account: crate::account::Account::default(),
            echoes: Echoes::default(),
            long_look: LongLook::default(),
            creation: Creation::default(),
            ways: Ways::default(),
            channels: Channels::default(),
            shown_partial: None,
            told_sounds_off: false,
            comings: Comings::default(),
            automation: Automation::default(),
            outgoing: Vec::new(),
            events: Vec::new(),
        })
    }

    /// As the desktop's `server_info`, as JSON.
    pub fn server_info(&self) -> String {
        json!({ "host": HOST, "ports": PORTS, "connected": self.telnet.is_some(), "portId": self.port.map(|p| p.id) }).to_string()
    }

    /// The world a port plays in (its map and cast are kept by it).
    pub fn world_of(port_id: &str) -> Option<String> {
        PORTS.iter().find(|p| p.id == port_id).map(|p| p.world.to_string())
    }

    /// Gets ready to play on a port and returns the WebSocket to open.
    /// `map`, `cast` and `looks` are the world's saved JSON, or empty.
    pub fn connect(&mut self, port_id: &str, map: &str, cast: &str, looks: &str) -> Result<String, String> {
        let port = PORTS.iter().find(|p| p.id == port_id).ok_or("Coupler doesn't know that way to play.")?;
        let mut telnet = Telnet::new(&self.version);
        if self.size.0 > 0 {
            telnet.resize(self.size.0, self.size.1);
        }
        if self.world != Some(port.world) {
            self.map = if map.is_empty() { Map::default() } else { Map::from_json(map).unwrap_or_default() };
            self.cast = if cast.is_empty() { Cast::default() } else { Cast::from_json(cast).unwrap_or_default() };
            self.looks = if looks.is_empty() { Looks::default() } else { Looks::from_json(looks).unwrap_or_default() };
            self.map_saved = Saved::default();
            self.cast_saved = Saved::default();
            self.walk.clear();
            self.world = Some(port.world);
        }
        if self.journal_world != Some(port.world) {
            self.journal_world = Some(port.world);
            self.journal_character.clear();
        }
        self.bestiary_world = Some(port.world);
        let auto = self.beasts.auto;
        self.beasts = Beasts { auto, ..Beasts::default() };
        self.counted = false;
        self.telnet = Some(telnet);
        self.port = Some(port);
        self.screen = Screen::default();
        self.kinds = Kinds::default();
        self.asked_where = false;
        self.character = Character::default();
        self.combat = Combat::default();
        self.combat_sent = None;
        self.senses = Senses::default();
        self.vitals_sent = None;
        self.who = Who::default();
        self.account = crate::account::Account::default();
        self.echoes.clear();
        self.long_look.clear();
        self.creation.clear();
        self.ways.clear();
        self.shown_partial = None;
        self.told_sounds_off = false;
        self.automation.closed();
        self.outgoing.clear();
        Ok(ports::web_socket_url(port))
    }

    /// The socket closed: `reason` is None when the player hung up.
    pub fn closed(&mut self, reason: Option<String>) {
        if self.telnet.take().is_none() {
            return;
        }
        self.port = None;
        self.outgoing.clear();
        self.left_the_game("Stopped walking: the connection ended.");
        self.emit("mud-closed", json!({ "reason": reason }));
    }

    pub fn connected(&self) -> bool {
        self.telnet.is_some()
    }

    /// A line the player typed: an alias's commands if it's one, as the desktop's.
    pub fn send_line(&mut self, line: &str) -> Result<(), String> {
        if self.telnet.is_none() {
            return Err("Not connected to CoffeeMUD.".into());
        }
        self.stop_walk("Stopped walking: you typed a command.");
        let expanded = if self.who.acting() { self.automation.expand(line) } else { None };
        for line in expanded.unwrap_or_else(|| vec![line.to_string()]) {
            self.who.typed(&line, Instant::now());
            self.send_command(&line);
        }
        Ok(())
    }

    /// Sends a command and tells everything that waits for its answer;
    /// a trigger's or a ticker's isn't the player typing (`Who::typed`).
    fn send_command(&mut self, line: &str) {
        self.outgoing.extend(telnet::encode_line(line));
        self.echoes.typed(line, Instant::now());
        self.long_look.typed(line, Instant::now());
        self.beasts.reader.typed(line, Instant::now());
        self.creation.typed(line);
        if let Some(left) = self.character.sent(line) {
            self.character_left(left);
        }
    }

    // ---- The game's channels (channels.rs), as the desktop's ----

    /// Sends what `make` gives, as the player's own commands, only with a
    /// character in the game; the list comes as `channels`.
    pub fn channel_commands(&mut self, make: impl FnOnce(&mut Channels, Instant) -> Vec<String>) -> Result<(), String> {
        if self.telnet.is_none() || !self.who.acting() {
            return Err("The channels are your character's: enter the game first.".into());
        }
        let now = Instant::now();
        let lines = make(&mut self.channels, now);
        if lines.is_empty() {
            return Err("That isn't a channel's name.".into());
        }
        for line in lines {
            self.send_command(&line);
            self.who.typed(&line, now);
        }
        Ok(())
    }

    // ---- The player's rules (automation.rs), as the desktop's ----

    pub fn automation_set(&mut self, rules: automation::Rules) {
        self.automation.set(rules);
    }

    pub fn automation_recent(&self) -> String {
        json!(self.automation.recent()).to_string()
    }

    pub fn automation_suggest(&self, line: &str) -> String {
        let names: Vec<String> = self.cast.list().into_iter().map(|c| c.name).collect();
        json!(automation::suggest(line, &names)).to_string()
    }

    fn automation_held(&mut self) {
        if self.automation.take_held() {
            self.emit("automation-held", json!(HELD));
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.size = (width, height);
        if let Some(bytes) = self.telnet.as_mut().and_then(|t| t.resize(width, height)) {
            self.outgoing.extend(bytes);
        }
    }

    /// The page's clock, each second while connected: WHO when it's due,
    /// WEATHER when the sky's unknown, each only when it's gentle to.
    pub fn who_tick(&mut self) {
        if self.telnet.is_none() {
            return;
        }
        let now = Instant::now();
        // Nothing of Coupler's own while a channel command's answer is coming (channels.rs).
        if self.channels.awaiting(now) {
            return;
        }
        let due = self.automation.tick(now, self.who.present(now), self.who.awaiting());
        for line in due {
            self.send_command(&line);
        }
        self.automation_held();
        if self.who.due(now) {
            self.outgoing.extend(telnet::encode_line("who"));
        } else if self.who.gentle(now) && self.ambient.weather_wanted(now) {
            self.ambient.weather_asked(now);
            self.outgoing.extend(telnet::encode_line("weather"));
        } else if self.who.gentle(now) {
            self.bestiary_tick(now);
        }
    }

    /// The say key before any WHO was read, as the desktop's `who_ask`.
    pub fn who_ask(&mut self) -> bool {
        if self.telnet.is_none() || !self.who.ask_now(Instant::now()) {
            return false;
        }
        self.outgoing.extend(telnet::encode_line("who"));
        true
    }

    /// Who's online now, as the desktop's `who_now`.
    pub fn who_now(&self) -> String {
        json!(self.who.report(Instant::now())).to_string()
    }

    /// What to send the game now.
    pub fn take_outgoing(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.outgoing)
    }

    /// The events since last asked, as a JSON array of `[name, payload]`.
    pub fn take_events(&mut self) -> String {
        Value::Array(std::mem::take(&mut self.events)).to_string()
    }

    /// A binary frame from the game: the same work as the desktop's
    /// `read_loop` does with each read. An error means hang up.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), String> {
        let Some(telnet) = self.telnet.as_mut() else { return Ok(()) };
        let out = telnet.feed(bytes)?;
        self.outgoing.extend(out.replies);
        let mut lines = Vec::new();
        let mut room_changed = false;
        let mut in_game = false;
        let mut met = false;
        let mut fired = Vec::new();
        // The game's sounds, cut out of its text.
        let mut sounds = Vec::new();
        // Coupler's own events, from the text and GMCP (foley.rs).
        let mut happenings = Vec::new();
        // Creatures' cries, and whether the Bestiary learned anything.
        let mut cries: Vec<(Creature, Cry, Option<Place>)> = Vec::new();
        let mut bestiary_changed = false;
        for event in out.events {
            match event {
                Event::Text(bytes) => {
                    let mut done = self.screen.push(&bytes);
                    // The game's sounds come out of the text (msp.rs); a
                    // line that held nothing else isn't shown.
                    done.retain_mut(|line| {
                        let blank = line.is_empty();
                        sounds.extend(msp::take(line));
                        blank || !line.is_empty()
                    });
                    let now = Instant::now();
                    done.retain(|line| self.who.line(&line.iter().map(|span| span.text.as_str()).collect::<String>(), now) == Seen::Show);
                    let mut answer = Vec::new();
                    for (i, line) in done.iter().enumerate() {
                        let text: String = line.iter().map(|span| span.text.as_str()).collect();
                        let (set_off, asked) = self.ambient_line(&text);
                        fired.extend(set_off);
                        if asked {
                            answer.push(i);
                        }
                        fired.extend(self.daytime(Said::Line(&text)));
                        happenings.extend(foley::line(&text).map(|happening| Movement { happening, place: None }));
                        self.comings.line(&text, Instant::now());
                    }
                    // Coupler's own WEATHER's answer doesn't show.
                    let mut i = 0;
                    done.retain(|_| {
                        i += 1;
                        !answer.contains(&(i - 1))
                    });
                    lines.extend(done);
                }
                Event::Prompt => {}
                Event::ServerEcho(on) => {
                    self.who.echo(on);
                    self.emit("mud-echo", json!(on));
                }
                Event::Msp(on) => {
                    if !on && !std::mem::replace(&mut self.told_sounds_off, true) {
                        self.emit("sounds-off", json!(msp::SOUNDS_OFF));
                    }
                }
                Event::Msdp(name, value) => {
                    if self.combat.msdp(&name, &value) {
                        continue;
                    }
                    if name == "WORLD_TIME" {
                        fired.extend(self.daytime(Said::WorldTime(&value)));
                    }
                }
                Event::Gmcp(package, data) => {
                    if let Some(left) = self.character.gmcp(&package, &data) {
                        self.character_left(left);
                    }
                    self.combat.gmcp(&package, &data);
                    self.ways.gmcp(&package, &data);
                    let moved = self.comings.gmcp(&package, &data, Instant::now());
                    let (learned, cry) = self.bestiary_gmcp(&package, &data);
                    bestiary_changed |= learned;
                    if let Some((creature, cry)) = cry {
                        // Heard from where its footsteps are.
                        cries.push((creature, cry, moved.as_ref().and_then(|m| m.place.clone())));
                    }
                    happenings.extend(moved);
                    let talk = self.senses.gmcp(&package, &data).filter(|t| !self.who.heard(&t.text, Instant::now()));
                    // Told once its lines are read, as printed (speech.rs `heard`).
                    if let Some(talk) = talk {
                        self.kinds.talk(talk, Instant::now());
                    }
                    let new = self.cast.gmcp(&package, &data, unix_now());
                    self.cast_saved.unsaved |= new || package.eq_ignore_ascii_case("room.mobiles") || package.eq_ignore_ascii_case("room.players");
                    met |= new;
                    room_changed |= self.map_gmcp(&package, &data);
                    in_game |= package.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("char."));
                    if package.eq_ignore_ascii_case("room.info") {
                        if self.account.entered() {
                            self.emit("account-menu", Value::Null);
                        }
                        if self.creation.entered() {
                            self.emit("creation", Value::Null);
                        }
                        fired.extend(self.ambient_room(&data));
                    }
                    fired.extend(self.hooks.record(&package, &data));
                    self.emit("mud-gmcp", json!({ "package": package, "data": data }));
                }
            }
        }
        // Talk, as the player saw it (speech.rs `heard`).
        let mut texts: Vec<String> = lines.iter().map(|line| line.iter().map(|span| span.text.as_str()).collect()).collect();
        // The Bestiary reads them (bestiary.rs); the answer to Coupler's
        // own look isn't shown, nor the prompt it was asked at, written
        // again as a line when the answer began.
        let prompted = self.screen.partial().is_some_and(|line| line.iter().any(|span| !span.text.trim().is_empty()));
        // Before anything's hidden: the player's gags are for the screen.
        let ways_found = self.ways_lines(&texts, prompted);
        let (learned, died, mut hidden_lines) = self.bestiary_lines(&texts, prompted);
        bestiary_changed |= learned;
        cries.extend(died.into_iter().map(|(c, cry)| (c, cry, None)));
        // The answers to Coupler's channel commands aren't shown either (channels.rs).
        let now = Instant::now();
        hidden_lines.extend(texts.iter().enumerate().filter(|(_, text)| self.channels.line(text, now)).map(|(i, _)| i));
        if let Some(listing) = self.channels.take_listing() {
            self.emit("channels", json!(listing));
        }
        hidden_lines.sort_unstable();
        hidden_lines.dedup();
        if let Some(&first) = hidden_lines.first() {
            let before = (0..first).rev().find(|&i| !texts[i].trim().is_empty() || i == 0);
            let prompt = self.shown_partial.as_ref().map(|line| line.iter().map(|span| span.text.as_str()).collect::<String>());
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
        let (gagged, actions) = self.automation.lines(&texts, self.who.acting(), Instant::now());
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
            self.send_command(line);
        }
        self.automation_held();
        if !actions.say.is_empty() || !actions.sounds.is_empty() {
            self.emit("automation-fired", json!(actions));
        }
        if self.automation.take_recent_changed() {
            self.emit("automation-recent", json!(self.automation.recent()));
        }
        for talk in self.kinds.heard(&texts, Instant::now()) {
            let (event, new) = self.cast_talk(talk);
            met |= new;
            self.emit("talk", json!(event));
        }
        if room_changed {
            self.room_changed();
        }
        // After the room.info, so they're the new room's.
        if let Some(ways) = ways_found {
            self.ways_found(ways);
        }
        if in_game {
            self.count_play();
            self.ask_where();
        }
        if !sounds.is_empty() {
            fired.extend(self.msp_sounds(&sounds));
        }
        if !happenings.is_empty() {
            fired.extend(self.event_sounds(&happenings));
        }
        let (learned, fights) = self.bestiary_fight();
        bestiary_changed |= learned;
        cries.extend(fights.map(|(c, cry)| (c, cry, None)));
        if !cries.is_empty() {
            fired.extend(self.cry_sounds(&cries));
        }
        if bestiary_changed {
            self.emit("bestiary-changed", Value::Null);
        }
        if met {
            self.emit("cast-changed", Value::Null);
        }
        self.save_cast(false);
        self.save_journal(false);
        self.save_bestiary(false);
        self.update_ambience();
        // After the sounds: a picture never goes before them.
        self.update_picture();
        self.update_combat();
        self.update_vitals();
        ambient::without_place_music(&mut fired);
        if !fired.is_empty() {
            self.emit("hook-fired", json!(fired));
        }
        let mut partial = self.screen.partial().and_then(|mut line| {
            msp::tidy(&mut line);
            (!line.is_empty()).then_some(line)
        });
        let mut prompt = partial.as_ref().map(|line| line.iter().map(|span| span.text.as_str()).collect::<String>());
        let now = Instant::now();
        self.who.character(self.senses.name(), now);
        if self.who.partial(prompt.as_deref(), now) {
            partial = self.shown_partial.clone();
            prompt = partial.as_ref().map(|line| line.iter().map(|span| span.text.as_str()).collect::<String>());
        }
        self.shown_partial = partial.clone();
        let changes = self.who.take_changes();
        if !changes.is_empty() {
            self.emit("who", json!({ "changes": changes }));
        }
        self.character.partial(prompt.as_deref());
        let fighting = self.combat.opponent().is_some();
        let mut line_kinds = self.kinds.lines(&texts, prompt.as_deref(), fighting, Instant::now());
        let playing = !self.senses.name().is_empty();
        match self.creation.lines(&lines, &line_kinds, partial.as_ref(), playing) {
            Some(creation::Update::Step(step)) => self.emit("creation", json!(step)),
            Some(creation::Update::Ended) => self.emit("creation", Value::Null),
            None => {}
        }
        // The account menu, for its dialog (account.rs); its list asked once.
        match self.account.lines(&texts, prompt.as_deref()) {
            Some(crate::account::Update::Open(menu)) => self.emit("account-menu", json!(menu)),
            Some(crate::account::Update::Closed) => self.emit("account-menu", Value::Null),
            None => {}
        }
        if self.account.wants_list() {
            self.outgoing.extend(telnet::encode_line("L"));
        }
        // WHO waits while a character's being made (who.rs).
        self.who.creating(self.creation.active());
        let echoes = self.echoes.lines(&texts, &mut line_kinds, prompt.as_deref(), playing, Instant::now());
        let hidden = self.long_look.lines(&lines, &line_kinds, self.map.here_name(), Instant::now());
        self.emit("mud-output", json!(OutputEvent { lines, kinds: line_kinds, echoes, hidden, partial }));
        Ok(())
    }

    // ---- The map, as the desktop's commands ----

    pub fn map_snapshot(&self) -> String {
        json!(self.map.snapshot(MAP_RADIUS.0, MAP_RADIUS.1)).to_string()
    }

    pub fn map_find(&self, query: &str) -> String {
        json!(self.map.find(query, 50)).to_string()
    }

    pub fn map_directions(&self, to: &str) -> Result<String, String> {
        if self.map.current().is_none() {
            return Err("The map doesn't know where you are yet. Move or look once in the game.".into());
        }
        let route = self.map.route(to).ok_or("The map has no known way there from here.")?;
        Ok(if route.is_empty() { "You're already there.".into() } else { mapper::directions(&route) })
    }

    pub fn map_set_landmark(&mut self, id: &str, name: &str) -> Result<(), String> {
        if !self.map.set_landmark(id, name) {
            return Err("The map doesn't know that room.".into());
        }
        self.map_saved.unsaved = true;
        self.save_map(true);
        self.emit_map();
        Ok(())
    }

    pub fn map_clear(&mut self) {
        self.map.clear();
        self.walk.clear();
        self.map_saved.unsaved = true;
        self.save_map(true);
        self.emit_map();
    }

    pub fn map_walk(&mut self, to: &str) -> Result<String, String> {
        if self.map.current().is_none() {
            return Err("The map doesn't know where you are yet. Move or look once in the game.".into());
        }
        let route = self.map.route(to).ok_or("The map has no known way there from here.")?;
        if route.is_empty() {
            return Err("You're already there.".into());
        }
        let words = mapper::directions(&route);
        self.walk = route.into();
        let name = self.map.room_name(to);
        self.emit("map-walk", json!({ "walking": true, "message": format!("Walking to {name}: {words}.") }));
        self.take_step();
        Ok(words)
    }

    pub fn map_stop(&mut self) {
        self.stop_walk("Stopped walking.");
    }

    // ---- The rest the UI asks for ----

    pub fn cast_list(&self) -> String {
        json!({ "world": self.world, "characters": self.cast.list() }).to_string()
    }

    /// How many triggers the mirror holds (the status bar's total).
    pub fn hooks_count(&self) -> usize {
        self.hooks.len()
    }

    pub fn ambience_now(&self) -> String {
        json!(self.ambience_sent).to_string()
    }
}

impl Coupler {
    fn emit(&mut self, name: &str, payload: Value) {
        self.events.push(json!([name, payload]));
    }

    /// The read's finished lines, for the exits text: as the desktop's
    /// `ways_lines`.
    fn ways_lines(&mut self, texts: &[String], prompted: bool) -> Option<Vec<Way>> {
        let mut found = None;
        for text in texts {
            if let Some(w) = self.ways.line(text) {
                found = Some(w);
            }
        }
        if prompted {
            if let Some(w) = self.ways.prompt() {
                found = Some(w);
            }
        }
        found
    }

    /// An exits block's ways: on the map's room, and `ways` for the narrator.
    fn ways_found(&mut self, ways: Vec<Way>) {
        if self.map.set_ways(ways.clone()).is_some() {
            self.map_saved.unsaved = true;
            self.emit_map();
            self.save_map(false);
        }
        if let Some(here) = self.map.current().map(str::to_string) {
            if self.ways.announce(&here, &ways) {
                self.emit("ways", json!(ways));
            }
        }
    }

    fn emit_map(&mut self) {
        let snapshot = json!(self.map.snapshot(MAP_RADIUS.0, MAP_RADIUS.1));
        self.emit("map-changed", snapshot);
    }

    /// Asks the page to keep the map: every time when `now`, otherwise no
    /// more often than the desktop writes it.
    fn save_map(&mut self, now: bool) {
        let Some(world) = self.world else { return };
        if !self.map_saved.unsaved || (!now && self.map_saved.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
            return;
        }
        self.map_saved = Saved { unsaved: false, saved_at: Some(Instant::now()) };
        let json = self.map.to_json();
        self.emit("coupler-save", json!({ "key": format!("maps/{world}.json"), "json": json }));
    }

    fn save_cast(&mut self, now: bool) {
        let Some(world) = self.world else { return };
        if !self.cast_saved.unsaved || (!now && self.cast_saved.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
            return;
        }
        self.cast_saved = Saved { unsaved: false, saved_at: Some(Instant::now()) };
        let json = self.cast.to_json();
        self.emit("coupler-save", json!({ "key": format!("cast/{world}.json"), "json": json }));
    }

    fn stop_walk(&mut self, why: &str) {
        if !self.walk.is_empty() {
            self.walk.clear();
            self.emit("map-walk", json!({ "walking": false, "message": why }));
        }
    }

    fn take_step(&mut self) {
        let Some(step) = self.walk.front() else { return };
        let way = mapper::dir_name(&step.dir);
        if self.map.exit(&step.dir).is_some_and(|e| e.door && !e.open) {
            self.outgoing.extend(telnet::encode_line(&format!("open {way}")));
        }
        self.outgoing.extend(telnet::encode_line(way));
    }

    fn room_changed(&mut self) {
        self.emit_map();
        self.save_map(false);
        let here = self.map.current().map(str::to_string);
        let Some(step) = self.walk.front() else { return };
        if here.as_deref() == Some(step.to.as_str()) {
            self.walk.pop_front();
            if self.walk.is_empty() {
                let name = self.map.room_name(here.as_deref().unwrap_or(""));
                self.emit("map-walk", json!({ "walking": false, "message": format!("Arrived: {name}.") }));
            } else {
                self.take_step();
            }
        } else if !self.map.exit(&step.dir).is_some_and(|e| e.to == step.to) {
            self.stop_walk("Stopped walking: you ended up somewhere off the route.");
        }
    }

    fn left_the_game(&mut self, why: &str) {
        self.stop_walk(why);
        self.map.leave();
        self.map_saved.unsaved = true;
        self.save_map(true);
        self.emit_map();
        self.ambient.leave();
        self.update_ambience();
        self.daytime.leave();
        self.update_picture();
        self.combat.leave();
        self.update_combat();
        self.senses.leave();
        self.update_vitals();
        self.cast.leave();
        self.cast_saved.unsaved = true;
        self.save_cast(true);
        self.save_journal(true);
        self.save_bestiary(true);
        self.who.leave();
    }

    fn update_vitals(&mut self) {
        let now = self.senses.vitals();
        if now != self.vitals_sent {
            self.vitals_sent = now.clone();
            self.emit("vitals", json!(now));
        }
    }

    fn update_combat(&mut self) {
        let now = self.combat.opponent();
        if now != self.combat_sent {
            self.combat_sent = now.clone();
            self.emit("combat", json!(now));
        }
    }

    fn character_left(&mut self, left: Left) {
        self.counted = false;
        self.echoes.clear();
        self.long_look.clear();
        self.ways.clear();
        self.channels.closed();
        self.beasts.reader.clear();
        let message = match &left {
            Left::Logout => {
                self.left_the_game("Stopped walking: you logged out.");
                "Logged out: the sounds faded and the pictures closed.".to_string()
            }
            Left::Switch(name) => {
                self.stop_walk("Stopped walking: you switched character.");
                self.combat.leave();
                self.update_combat();
                self.senses.leave();
                self.update_vitals();
                self.who.leave();
                format!("Now playing {name}: the sounds faded and the pictures closed.")
            }
        };
        self.emit("character-left", json!({ "message": message }));
        if matches!(left, Left::Switch(_)) {
            self.ambience_sent = Ambience::default();
            self.update_ambience();
        }
    }

    fn ask_where(&mut self) {
        if self.map.current().is_some() {
            self.asked_where = false;
            return;
        }
        if !self.asked_where {
            self.asked_where = true;
            self.outgoing.extend(Telnet::gmcp("Room.Info", ""));
        }
    }

    fn map_gmcp(&mut self, package: &str, data: &str) -> bool {
        let applied = match package.to_ascii_lowercase().as_str() {
            "room.info" => self.map.apply_room_info(data),
            "room.exits" => self.map.apply_room_exits(data),
            "room.wrongdir" => {
                self.stop_walk("Stopped walking: the game says you can't go that way.");
                return false;
            }
            _ => return false,
        };
        match applied {
            Ok(changed) => {
                self.map_saved.unsaved |= changed;
                true
            }
            Err(_) => false,
        }
    }

    fn ambient_room(&mut self, data: &str) -> Vec<Fired> {
        let Ok(v) = serde_json::from_str::<Value>(data) else { return Vec::new() };
        let field = |name: &str| v.get(name).and_then(Value::as_str).unwrap_or("").to_string();
        let room = ambient::Room { id: field("id"), zone: field("zone"), terrain: field("terrain").to_ascii_lowercase() };
        let kind = ambient::room_type(&room.terrain);
        self.picture_name = field("name");
        self.ambient.enter(room);
        match kind {
            Some(kind) => self.hooks.record(ambient::ROOM_TYPE_KEY, &ambient::json_text(kind)),
            None => Vec::new(),
        }
    }

    /// What a line's weather set off, and whether it answered Coupler's own WEATHER.
    fn ambient_line(&mut self, text: &str) -> (Vec<Fired>, bool) {
        let Some(seen) = self.ambient.reader.line(text) else { return (Vec::new(), false) };
        let asked = self.ambient.answered(Instant::now());
        let fired = match self.ambient.saw(seen) {
            Some(kind) => self.hooks.record(ambient::WEATHER_KEY, &ambient::json_text(kind)),
            None => Vec::new(),
        };
        (fired, asked)
    }

    fn daytime(&mut self, said: Said) -> Vec<Fired> {
        let now = Instant::now();
        let changed = match said {
            Said::Line(text) => daytime::read(text).and_then(|told| self.daytime.told(told, now)),
            Said::WorldTime(value) => daytime::world_time_hour(value).and_then(|hour| self.daytime.hour(hour, now)),
        };
        match changed {
            Some(phase) => self.hooks.record(daytime::TIME_KEY, &ambient::json_text(phase.name())),
            None => Vec::new(),
        }
    }

    /// As the desktop's: what the mirror's triggers leave silent plays
    /// the bank's loop for the terrain (the battle's in a fight) and the
    /// weather (`soundbank::fill_ambience`).
    fn update_ambience(&mut self) {
        let mut now = self.ambient.ambience_from(&self.hooks);
        if self.hooks.bank() {
            let night = self.daytime.now() == Some(Phase::Night);
            let fighting = self.combat.opponent().is_some();
            let a = &self.ambient;
            soundbank::fill_ambience(&mut now, a.room.as_ref().map(|r| r.terrain.as_str()), a.weather_overhead(), night, fighting);
        }
        // The place's music goes quiet in a fight, and starts again after.
        if self.combat.opponent().is_some() {
            now.bgm = None;
        }
        if now != self.ambience_sent {
            self.ambience_sent = now.clone();
            self.emit("ambient", json!(now));
        }
    }
}

// ---- What the desktop keeps in app data, kept by the page ----

impl Coupler {
    /// Asks the page to keep everything waiting to be kept: when the page
    /// is hidden or closed, and before a backup.
    pub fn save_now(&mut self) {
        self.save_map(true);
        self.save_cast(true);
        self.save_journal(true);
        self.save_bestiary(true);
    }

    fn save_journal(&mut self, now: bool) {
        self.journal_saved.unsaved |= self.journal.changed();
        if !self.journal_saved.unsaved || (!now && self.journal_saved.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
            return;
        }
        self.journal_saved = Saved { unsaved: false, saved_at: Some(Instant::now()) };
        let json = self.journal.to_json();
        self.emit("coupler-save", json!({ "key": "web/journal.json", "json": json }));
    }

    fn save_bestiary(&mut self, now: bool) {
        self.bestiary_saved.unsaved |= self.bestiary.changed();
        if !self.bestiary_saved.unsaved || (!now && self.bestiary_saved.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
            return;
        }
        self.bestiary_saved = Saved { unsaved: false, saved_at: Some(Instant::now()) };
        let json = self.bestiary.to_json();
        self.emit("coupler-save", json!({ "key": "web/bestiary.json", "json": json }));
    }

    fn save_looks(&mut self) {
        let Some(world) = self.world else { return };
        let json = self.looks.to_json();
        self.emit("coupler-save", json!({ "key": format!("pictures/{world}.json"), "json": json }));
    }

    /// A character is in the game: counted once until the next
    /// connection, logout or switch, as the desktop's `played.json`.
    fn count_play(&mut self) {
        if std::mem::replace(&mut self.counted, true) {
            return;
        }
        self.played += 1;
        self.emit("coupler-save", json!({ "key": "played.json", "json": json!({ "played": self.played }).to_string() }));
    }

    /// The times played, for the greeting at launch.
    pub fn played(&self) -> u64 {
        self.played
    }

    // ---- The cast's voices ----

    /// Changes the cast as the player asked, and keeps it at once.
    pub fn cast_edit<T: Serialize>(&mut self, edit: impl FnOnce(&mut Cast) -> Result<T, String>) -> Result<String, String> {
        let done = edit(&mut self.cast)?;
        self.cast_saved.unsaved = true;
        self.save_cast(true);
        Ok(json!(done).to_string())
    }

    /// A line of talk with its speaker's voice, kept in the journal or
    /// the log; the second is whether they're new.
    fn cast_talk(&mut self, talk: Talk) -> (TalkEvent, bool) {
        let (voice, new) = self.cast.talk(&talk, unix_now());
        self.cast_saved.unsaved |= voice.is_some();
        let met = self.cast.character(&letters_and_digits(&talk.from));
        let name = met.map(|c| c.name.clone()).unwrap_or_default();
        // Only players use the channels; anyone met in a room is known.
        let person = talk.kind == TalkKind::Channel || met.is_some_and(|c| c.who == cast::Who::Pc);
        let (entry, repeat) = self.journal_record(&talk, &name, person);
        (TalkEvent { talk, voice, entry, repeat }, new)
    }

    // ---- The journal and the log (journal.rs) ----

    fn journal_record(&mut self, talk: &Talk, speaker: &str, person: bool) -> (Option<journal::Entry>, bool) {
        let character = self.senses.name().to_string();
        if !character.is_empty() {
            self.journal_character = character;
        }
        let Some(world) = self.journal_world else { return (None, false) };
        let scope = journal::Scope { world, character: &self.journal_character };
        let entry = match self.journal.record(scope, talk, speaker, person, unix_ms()) {
            Ok(entry) => entry,
            Err(_) => return (None, false),
        };
        let repeat = entry.is_none();
        if !repeat {
            self.journal_changed();
        }
        (entry, repeat)
    }

    fn with_journal<T>(&self, f: impl FnOnce(&Journal, journal::Scope) -> Result<T, String>) -> Result<T, String> {
        let world = self.journal_world.ok_or("Nothing's in the journal until you've played.")?;
        f(&self.journal, journal::Scope { world, character: &self.journal_character })
    }

    fn journal_changed(&mut self) {
        if let Ok(unheard) = self.journal_unheard_now() {
            self.emit("journal-changed", json!(unheard));
        }
    }

    fn journal_unheard_now(&self) -> Result<journal::Unheard, String> {
        match self.journal_world {
            Some(world) => self.journal.unheard(journal::Scope { world, character: &self.journal_character }),
            None => Ok(journal::Unheard::default()),
        }
    }

    pub fn journal_unheard(&self) -> Result<String, String> {
        Ok(json!(self.journal_unheard_now()?).to_string())
    }

    pub fn journal_list(&self, book: &str, query: &str, who: Option<&str>) -> Result<String, String> {
        let book = Book::parse(book)?;
        Ok(json!(self.with_journal(|j, scope| j.list(scope, book, query, who, JOURNAL_SHOWN))?).to_string())
    }

    /// A typed channel LAST (`OOC LAST 10`), answered from the log, as the desktop's.
    pub fn journal_last(&self, line: &str) -> Result<String, String> {
        let Some((word, count)) = journal::last_asked(line) else { return Ok("null".into()) };
        let found = self.with_journal(|j, scope| j.last(scope, &word, count))?;
        Ok(json!(found.map(|(channel, entries)| journal::LastAnswer { channel, count, entries })).to_string())
    }

    pub fn journal_unheard_entries(&self, book: &str) -> Result<String, String> {
        let book = Book::parse(book)?;
        Ok(json!(self.with_journal(|j, scope| j.unheard_entries(scope, book, JOURNAL_SHOWN))?).to_string())
    }

    pub fn journal_set_heard(&mut self, ids: &[i64], heard: bool) -> Result<(), String> {
        if self.journal.set_heard(ids, heard)? > 0 {
            self.journal_changed();
            self.save_journal(false);
        }
        Ok(())
    }

    pub fn journal_heard_all(&mut self, book: &str) -> Result<(), String> {
        let book = Book::parse(book)?;
        if self.with_journal(|j, scope| j.heard_all(scope, book))? > 0 {
            self.journal_changed();
            self.save_journal(false);
        }
        Ok(())
    }

    // ---- The Bestiary and the Items (bestiary.rs) ----

    /// Keeps facts in the world's Bestiary. Returns whether there were any.
    fn bestiary_keep(&self, facts: &[Fact]) -> bool {
        let Some(world) = self.bestiary_world else { return false };
        let at = unix_ms();
        for fact in facts {
            let _ = self.bestiary.record(world, self.beasts.reader.area(), fact, at);
        }
        !facts.is_empty()
    }

    fn bestiary_cry(&self, name: &str, cry: Cry) -> Option<(Creature, Cry)> {
        self.bestiary.creature(self.bestiary_world?, name).map(|c| (c, cry))
    }

    fn bestiary_gmcp(&mut self, package: &str, data: &str) -> (bool, Option<(Creature, Cry)>) {
        let (facts, entered) = self.beasts.reader.gmcp(package, data);
        self.bestiary_queue(&facts);
        let entered = entered.filter(|name| self.cast.character(&letters_and_digits(name)).is_none_or(|c| c.who != cast::Who::Pc));
        let changed = self.bestiary_keep(&facts);
        (changed, entered.and_then(|name| self.bestiary_cry(&name, Cry::Comes)))
    }

    fn bestiary_queue(&mut self, facts: &[Fact]) {
        if !self.beasts.auto {
            return;
        }
        let Some(world) = self.bestiary_world else { return };
        let wanted: Vec<String> = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Seen { name } => Some(name.clone()),
                _ => None,
            })
            .filter(|name| self.bestiary.wants_look(world, name) && !self.beasts.to_look.contains(name))
            .collect();
        self.beasts.to_look.extend(wanted);
    }

    /// The player's choice, as the desktop's `bestiary_auto_look`.
    pub fn bestiary_auto_look(&mut self, on: bool) {
        self.beasts.auto = on;
        if !on {
            self.beasts.to_look.clear();
            return;
        }
        let here: Vec<Fact> = self.beasts.reader.here().map(|name| Fact::Seen { name: name.to_string() }).collect();
        self.bestiary_queue(&here);
    }

    /// On WHO's clock, as the desktop's: a look at the next creature waiting.
    fn bestiary_tick(&mut self, now: Instant) {
        let fighting = self.combat.opponent().is_some();
        let b = &self.beasts;
        if !b.auto || fighting || b.reader.busy() || b.looked_at.is_some_and(|t| now.duration_since(t) < AUTO_LOOK_EVERY) {
            return;
        }
        let Some(world) = self.bestiary_world else { return };
        while let Some(name) = self.beasts.to_look.pop_front() {
            if !self.beasts.reader.here().any(|n| n == name) || !self.bestiary.wants_look(world, &name) {
                continue;
            }
            let Some(line) = self.beasts.reader.auto_look(&name, now) else { continue };
            self.beasts.looked_at = Some(now);
            let _ = self.bestiary.looked(world, &name, unix_ms());
            self.outgoing.extend(telnet::encode_line(&line));
            return;
        }
    }

    fn bestiary_lines(&mut self, texts: &[String], prompted: bool) -> (bool, Vec<(Creature, Cry)>, Vec<usize>) {
        let me = self.senses.name().to_string();
        self.beasts.reader.character(&me);
        self.beasts.reader.examined(texts);
        let facts = self.beasts.reader.lines(texts, Instant::now());
        let hidden = self.beasts.reader.hidden().to_vec();
        if prompted {
            self.beasts.reader.prompted();
        }
        let changed = self.bestiary_keep(&facts);
        let cries = facts
            .iter()
            .filter_map(|f| match f {
                Fact::Died { name } => self.bestiary_cry(name, Cry::Dies),
                _ => None,
            })
            .collect();
        (changed, cries, hidden)
    }

    fn bestiary_fight(&mut self) -> (bool, Option<(Creature, Cry)>) {
        let opponent = self.combat.opponent();
        let name = opponent.as_ref().and_then(|o| o.name.clone());
        let health = opponent.as_ref().and_then(|o| o.health_max);
        self.beasts.reader.opponent(name.as_deref());
        let mut changed = false;
        let mut cry = None;
        if name != self.beasts.opponent {
            self.beasts.opponent = name.clone();
            self.beasts.since = Some(Instant::now());
            self.beasts.health = None;
            cry = name.as_deref().and_then(|n| self.bestiary_cry(n, Cry::Fights));
        }
        let settled = self.beasts.since.is_some_and(|t| t.elapsed() > Duration::from_millis(1500));
        if let (Some(name), Some(max), true) = (name, health.filter(|h| *h > 0), settled) {
            if self.beasts.health != Some(max) {
                self.beasts.health = Some(max);
                changed = self.bestiary_keep(&[Fact::Health { name, max }]);
            }
        }
        (changed, cry)
    }

    /// Runs `f` on the Bestiary of the world last played; nothing before any.
    fn with_bestiary<T: Default>(&self, f: impl FnOnce(&Bestiary, &str) -> Result<T, String>) -> Result<T, String> {
        match self.bestiary_world {
            Some(world) => f(&self.bestiary, world),
            None => Ok(T::default()),
        }
    }

    pub fn bestiary_mobs(&self, query: &str) -> Result<String, String> {
        Ok(json!(self.with_bestiary(|b, world| b.mobs(world, query).map(|rows| rows.into_iter().map(bestiary::MobRow::shown).collect::<Vec<_>>()))?).to_string())
    }

    pub fn bestiary_mob(&self, key: &str) -> Result<Option<bestiary::MobEntry>, String> {
        self.with_bestiary(|b, world| b.mob_entry(world, key).map(|e| e.map(bestiary::MobEntry::shown)))
    }

    pub fn bestiary_items(&self, query: &str) -> Result<String, String> {
        Ok(json!(self.with_bestiary(|b, world| b.items(world, query).map(|rows| rows.into_iter().map(bestiary::ItemRow::shown).collect::<Vec<_>>()))?).to_string())
    }

    pub fn bestiary_item(&self, key: &str) -> Result<Option<bestiary::ItemEntry>, String> {
        self.with_bestiary(|b, world| b.item_entry(world, key).map(|e| e.map(bestiary::ItemEntry::shown)))
    }

    // ---- The game's sounds, Coupler's own and the creatures' (soundbank.rs) ----

    fn msp_sounds(&self, sounds: &[msp::Sound]) -> Vec<Fired> {
        let bank = self.hooks.bank();
        let heard: Vec<_> = sounds
            .iter()
            .map(|sound| Heard { key: sound.key(), value: &sound.name, default: soundbank::msp_default(&sound.name).filter(|_| bank && !sound.music).map(|p| (p, 100)), under: None, place: None })
            .collect();
        self.own_sounds(&heard)
    }

    fn event_sounds(&self, happenings: &[Movement]) -> Vec<Fired> {
        let bank = self.hooks.bank();
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
        self.own_sounds(&heard)
    }

    fn cry_sounds(&self, cries: &[(Creature, Cry, Option<Place>)]) -> Vec<Fired> {
        let bank = self.hooks.bank();
        let values: Vec<String> = cries.iter().map(|(c, cry, _)| bestiary::cry_value(*c, *cry)).collect();
        let heard: Vec<_> = values
            .iter()
            .zip(cries)
            .map(|(value, (_, _, place))| Heard { key: bestiary::CREATURE_KEY, value, default: soundbank::creature_default(value).filter(|_| bank).map(|p| (p, 100)), under: None, place: place.clone() })
            .collect();
        self.own_sounds(&heard)
    }

    /// As the desktop's `own_sounds`: each pair sets off the mirror's
    /// trigger on it, or where that gives no sound effect, the bank's.
    fn own_sounds(&self, heard: &[Heard]) -> Vec<Fired> {
        let mut fired = Vec::new();
        for Heard { key, value: name, default, under, place } in heard {
            let value = ambient::json_text(name);
            let mut set_off = self.hooks.record(key, &value);
            let silent = set_off.iter().all(|f| f.trigger.sfx.is_none());
            if let Some((path, volume)) = default.clone().filter(|_| silent) {
                let bank = |path| Trigger { sfx: Some(path), sfx_volume: volume, ..Trigger::default() };
                match set_off.first_mut() {
                    Some(f) => {
                        f.trigger.sfx = Some(path);
                        f.trigger.sfx_volume = volume;
                    }
                    None => set_off.push(Fired { key: (*key).into(), value: value.clone(), trigger: bank(path), place: None }),
                }
                if let Some(under) = under.clone() {
                    set_off.push(Fired { key: (*key).into(), value, trigger: bank(under), place: None });
                }
            }
            for f in &mut set_off {
                f.place = place.clone();
            }
            fired.extend(set_off);
        }
        fired
    }

    // ---- The room's picture (paint.rs, painter.rs) ----

    /// The room as a picture sees it, as the desktop's `scene`.
    fn scene(&self) -> Option<Scene> {
        let world = self.world?;
        let room = self.ambient.room.as_ref()?;
        let mut scene = Scene {
            world: world.to_string(),
            room: room.id.clone(),
            zone: room.zone.clone(),
            terrain: room.terrain.clone(),
            weather: self.ambient.weather().map(str::to_string),
            time: self.daytime.now().map(|p| p.name().to_string()),
            arc: self.daytime.arc(Instant::now()),
            ..Scene::default()
        };
        scene.name = self.picture_name.clone();
        scene.look = self.looks.of(&scene.room);
        scene.art = self.hooks.trigger("room.info.id", &ambient::json_text(&scene.room)).and_then(|t| t.art.clone());
        Some(scene)
    }

    fn update_picture(&mut self) {
        let now = self.scene();
        if now != self.picture_sent {
            self.picture_sent = now.clone();
            self.emit("picture", json!(now));
        }
    }

    pub fn picture_now(&self) -> String {
        json!(self.picture_sent).to_string()
    }

    pub fn picture_again(&mut self) -> Result<String, String> {
        let room = self.picture_sent.as_ref().map(|s| s.room.clone()).ok_or("You're not in a room Coupler knows yet.")?;
        self.looks.again(&room);
        self.save_looks();
        self.update_picture();
        Ok(self.picture_now())
    }

    pub fn picture_forget_looks(&mut self) {
        self.looks.forget();
        self.save_looks();
        self.update_picture();
    }
}

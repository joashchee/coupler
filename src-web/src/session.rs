//! The web build's session: what the desktop's `session.rs` does with
//! each read, without the socket. The page (through `ffi.rs`) opens the game's WebSocket
//! (`connect` gives its address: `ports::web_socket_url`, the only one),
//! hands every binary frame to `feed`, sends what `take_outgoing` gives
//! back, and plays what `take_events` says, by the same event names and
//! payloads the desktop emits, so the UI can't tell them apart.
//!
//! Left out, for a slim build: the journal and the
//! log (talk is still said as it comes), the hooks database (the
//! triggers are the mirror's, `hooks.rs`), room pictures, and the voice
//! cache. The map and the cast are kept per world by the page
//! (IndexedDB), from the `coupler-save` event.

use std::collections::VecDeque;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use crate::ambient::{self, Ambience, Ambient};
use crate::ansi::{Line, Screen};
use crate::cast::{self, Cast};
use crate::character::{Character, Left};
use crate::clock::{unix_now, Instant};
use crate::combat::{Combat, Opponent};
use crate::daytime::{self, Daytime};
use crate::creation::{self, Creation};
use crate::echo::{Echoed, Echoes};
use crate::hidden::{Hidden, LongLook};
use crate::hooks::{Fired, Hooks};
use crate::mapper::{self, Map, Step};
use crate::ports::{self, Port, HOST, PORTS};
use crate::senses::{Senses, Talk, Vitals};
use crate::speech::{Kinds, LineKind};
use crate::telnet::{self, Event, Telnet};
use crate::who::{Seen, Who};

/// As the desktop's.
const MAP_SAVE_EVERY: Duration = Duration::from_secs(20);
const MAP_RADIUS: (i32, i32) = (5, 3);

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
    /// No journal on the web: never kept, never a repeat.
    entry: Option<()>,
    repeat: bool,
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
    ambient: Ambient,
    ambience_sent: Ambience,
    daytime: Daytime,
    character: Character,
    combat: Combat,
    combat_sent: Option<Opponent>,
    senses: Senses,
    vitals_sent: Option<Vitals>,
    who: Who,
    echoes: Echoes,
    long_look: LongLook,
    creation: Creation,
    /// The unfinished line last sent, kept while a hidden WHO is cut off mid-row.
    shown_partial: Option<Line>,
    outgoing: Vec<u8>,
    events: Vec<Value>,
}

impl Coupler {
    /// `mirror` is the mirror's `hooks.json`, or empty. A mirror that
    /// can't be read means no hooks, never a Coupler that can't start
    /// (and so no ports to play).
    pub fn new(version: &str, mirror: &str) -> Result<Coupler, String> {
        Ok(Coupler {
            version: version.to_string(),
            hooks: Hooks::from_mirror(mirror).unwrap_or_default(),
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
            echoes: Echoes::default(),
            long_look: LongLook::default(),
            creation: Creation::default(),
            shown_partial: None,
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
    /// `map` and `cast` are the world's saved JSON, or empty.
    pub fn connect(&mut self, port_id: &str, map: &str, cast: &str) -> Result<String, String> {
        let port = PORTS.iter().find(|p| p.id == port_id).ok_or("Coupler doesn't know that way to play.")?;
        let mut telnet = Telnet::new(&self.version);
        if self.size.0 > 0 {
            telnet.resize(self.size.0, self.size.1);
        }
        if self.world != Some(port.world) {
            self.map = if map.is_empty() { Map::default() } else { Map::from_json(map).unwrap_or_default() };
            self.cast = if cast.is_empty() { Cast::default() } else { Cast::from_json(cast).unwrap_or_default() };
            self.map_saved = Saved::default();
            self.cast_saved = Saved::default();
            self.walk.clear();
            self.world = Some(port.world);
        }
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
        self.echoes.clear();
        self.long_look.clear();
        self.creation.clear();
        self.shown_partial = None;
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

    /// A line the player typed.
    pub fn send_line(&mut self, line: &str) -> Result<(), String> {
        if self.telnet.is_none() {
            return Err("Not connected to CoffeeMUD.".into());
        }
        self.stop_walk("Stopped walking: you typed a command.");
        self.outgoing.extend(telnet::encode_line(line));
        self.who.typed(line, Instant::now());
        self.echoes.typed(line, Instant::now());
        self.long_look.typed(line, Instant::now());
        self.creation.typed(line);
        if let Some(left) = self.character.sent(line) {
            self.character_left(left);
        }
        Ok(())
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
        if self.who.due(now) {
            self.outgoing.extend(telnet::encode_line("who"));
        } else if self.who.gentle(now) && self.ambient.weather_wanted(now) {
            self.ambient.weather_asked(now);
            self.outgoing.extend(telnet::encode_line("weather"));
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
        for event in out.events {
            match event {
                Event::Text(bytes) => {
                    let mut done = self.screen.push(&bytes);
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
                    let talk = self.senses.gmcp(&package, &data).filter(|t| !self.who.heard(&t.text, Instant::now()));
                    if let Some(talk) = talk {
                        self.kinds.talk(&talk.text, Instant::now());
                        let (voice, new) = self.cast.talk(&talk, unix_now());
                        self.cast_saved.unsaved |= voice.is_some();
                        met |= new;
                        self.emit("talk", json!(TalkEvent { talk, voice, entry: None, repeat: false }));
                    }
                    let new = self.cast.gmcp(&package, &data, unix_now());
                    self.cast_saved.unsaved |= new || package.eq_ignore_ascii_case("room.mobiles") || package.eq_ignore_ascii_case("room.players");
                    met |= new;
                    room_changed |= self.map_gmcp(&package, &data);
                    in_game |= package.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("char."));
                    if package.eq_ignore_ascii_case("room.info") {
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
        if room_changed {
            self.room_changed();
        }
        if in_game {
            self.ask_where();
        }
        if met {
            self.emit("cast-changed", Value::Null);
        }
        self.save_cast(false);
        self.update_ambience();
        self.update_combat();
        self.update_vitals();
        if !fired.is_empty() {
            self.emit("hook-fired", json!(fired));
        }
        let mut partial = self.screen.partial();
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
        let texts: Vec<String> = lines.iter().map(|line| line.iter().map(|span| span.text.as_str()).collect()).collect();
        let fighting = self.combat.opponent().is_some();
        let mut line_kinds = self.kinds.lines(&texts, prompt.as_deref(), fighting, Instant::now());
        let playing = !self.senses.name().is_empty();
        match self.creation.lines(&lines, &line_kinds, partial.as_ref(), playing) {
            Some(creation::Update::Step(step)) => self.emit("creation", json!(step)),
            Some(creation::Update::Ended) => self.emit("creation", Value::Null),
            None => {}
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
        self.emit("coupler-save", json!({ "kind": "map", "world": world, "json": json }));
    }

    fn save_cast(&mut self, now: bool) {
        let Some(world) = self.world else { return };
        if !self.cast_saved.unsaved || (!now && self.cast_saved.saved_at.is_some_and(|t| t.elapsed() < MAP_SAVE_EVERY)) {
            return;
        }
        self.cast_saved = Saved { unsaved: false, saved_at: Some(Instant::now()) };
        let json = self.cast.to_json();
        self.emit("coupler-save", json!({ "kind": "cast", "world": world, "json": json }));
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
        self.combat.leave();
        self.update_combat();
        self.senses.leave();
        self.update_vitals();
        self.cast.leave();
        self.cast_saved.unsaved = true;
        self.save_cast(true);
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
        self.echoes.clear();
        self.long_look.clear();
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

    fn update_ambience(&mut self) {
        let now = self.ambient.ambience_from(&self.hooks);
        if now != self.ambience_sent {
            self.ambience_sent = now.clone();
            self.emit("ambient", json!(now));
        }
    }
}

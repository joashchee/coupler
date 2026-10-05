//! Ambience: the background noise (BGN) and background weather (BGW)
//! the hooks' triggers loop (`hooks.rs`), and the two facts Coupler
//! works out for them that the game doesn't send by GMCP. Pure and
//! unit-tested; `session.rs` feeds it.
//!
//! **Room type.** `room.info.terrain` is one of CoffeeMUD's locale
//! types, split into an outdoor and an indoor set
//! (`Locales/interfaces/Room.java`, `DOMAIN_OUTDOOR_DESCS` and
//! `DOMAIN_INDOORS_DESCS`). Coupler records which as its own pair,
//! `coupler.room.type`, `"indoors"` or `"outdoors"`.
//!
//! **Weather.** CoffeeMUD's GMCP has none (`Libraries/CMProtocols.java`
//! sends no climate package), so it's read from the game's text. Every
//! weather line comes from `resources/lists.ini`: for each of the 14
//! kinds (`Climate.WEATHER_DESCS`), five openings (by climate: normal,
//! wet, cold or winter, hot or summer, dry) and two endings (windy or
//! not), joined by a space (`DefaultClimate.theWeatherDescription`); a
//! line for each kind's end (`WEATHER_ENDS`); and one for no sky
//! (`WEATHER_NONE`). A change prints the end of the old weather, the
//! new one, or the end and then the new one on one line
//! (`DefaultClimate.weatherTick`'s `sayMap`). The same descriptions come
//! from the `weather` command, stepping outdoors with AUTOWEATHER on
//! (`MUDTracker`), the `time` command at night (`DefaultTimeClock`, with
//! "The clouds obscure the moon." and "It is very foggy.") and storms
//! (`Behaviors/WeatherAffects`). Coupler records what it read as its own
//! pair, `coupler.weather`, the kind's name in lower case (`"rain"`).
//!
//! A line counts only where the game starts one: at the start of the
//! line, or just after a prompt's closing `>`. A player saying the words
//! (`Bob says 'A light rain falls from the sky.'`) doesn't change the
//! weather. A description the server wrapped onto a second line is
//! joined back up before it's matched.
//!
//! Known gaps, from the game's own words: "A slushy snow" opens both
//! snow and sleet in a wet climate (read as snow); a line that only says
//! the old weather ended is read as clear, though a few changes (snow to
//! a cold snap) say only that; the weather is kept per area and is
//! unknown in an area until a line about it comes.

use std::collections::HashMap;

use serde::Serialize;

use crate::clock::Instant;
use crate::hooks::{Hooks, Trigger};

/// The pair Coupler records for the room type.
pub const ROOM_TYPE_KEY: &str = "coupler.room.type";
/// The pair Coupler records for the weather.
pub const WEATHER_KEY: &str = "coupler.weather";

const OUTDOOR: [&str; 14] = ["city", "woods", "rocky", "plains", "underwater", "air", "watersurface", "jungle", "swamp", "desert", "hills", "mountains", "spaceport", "seaport"];
const INDOOR: [&str; 10] = ["stone", "wooden", "cave", "magic", "in_underwater", "gap", "cavelakesurface", "metal", "innerseaport", "caveseaport"];

/// `"indoors"` or `"outdoors"` for a terrain, or None for one CoffeeMUD doesn't list.
pub fn room_type(terrain: &str) -> Option<&'static str> {
    let terrain = terrain.to_ascii_lowercase();
    if INDOOR.contains(&terrain.as_str()) {
        Some("indoors")
    } else if OUTDOOR.contains(&terrain.as_str()) {
        Some("outdoors")
    } else {
        None
    }
}

/// Whether a room has weather: outdoors and not underwater (`CMMap.hasASky`).
pub fn has_sky(terrain: &str) -> bool {
    room_type(terrain) == Some("outdoors") && !terrain.eq_ignore_ascii_case("underwater")
}

/// Each kind, by the name Coupler records it under, in
/// `Climate.WEATHER_DESCS`'s order, with its openings and endings from
/// `lists.ini` (the endings may be empty).
const DESCRIPTIONS: [(&str, &[&str], &[&str]); 14] = [
    (
        "clear",
        &["The weather is clear.", "The weather is clear and humid.", "The weather is cool and clear.", "The weather is warm and clear.", "The weather is dry and clear."],
        &[],
    ),
    ("cloudy", &["Fluffy cloudbanks", "Dark and looming stormclouds", "Gloomy cloudbanks", "Light clouds", "Light wisps of cloud"], &["move across the sky.", "obscure the sky."]),
    ("windy", &["A light wind", "A foreboding gust of wind", "A cold wind", "A hot wind", "A light dry wind"], &["gusts through here.", "blows through here."]),
    ("rain", &["A light rain", "A cool soaking rain", "A cold light rain", "A warm rain", "A light drizzling rain"], &["swirls down from the sky.", "falls from the sky."]),
    ("thunderstorm", &["A heavy and blusterous rainstorm"], &["swirls all around you.", "pours down from above."]),
    ("snow", &["An unseasonable snow", "A slushy snow", "A light snow", "A freakish snow", "A fluffy snow"], &["swirls down from the sky.", "falls from the sky."]),
    ("hail", &["Light streams of hail", "Hard and slushy clumps of ice", "Golfball sized clumps of ice", "Strange clumps of ice", "Hard clumps of ice"], &["swirl down from above", "fall from the sky."]),
    ("heat", &["It is very hot.", "It is very hot and muggy.", "It is rather warm.", "It is extremely hot.", "It is very hot and dry."], &[]),
    ("sleet", &["An unseasonable sleet storm", "A slushy snow", "A sleet storm", "A freakish sleet storm", "A light sleet"], &["swirls down from the sky.", "falls from the sky."]),
    ("blizzard", &["A thunderous blizzard"], &["swirls all around you.", "pours down from above."]),
    ("dust", &["An eye-stinging dust storm"], &["swirls all around you.", "blows through the area."]),
    ("drought", &["There are horrible drought conditions.", "There are drought conditions."], &[]),
    ("cold", &["It is cold.", "It is cold and muggy.", "It is very cold.", "It is strangely cold.", "It is very cold and dry."], &[]),
    ("fog", &["A soupy fog", "A bank of fog", "An obscuring fog", "A thick fog", "A rolling fog"], &["fills the air.", "descends."]),
];

/// The end of each kind (`WEATHER_ENDS`; clear has none).
const ENDS: [(&str, &str); 13] = [
    ("cloudy", "The clouds dissipate."),
    ("windy", "The wind gusts stop."),
    ("rain", "It stops raining."),
    ("thunderstorm", "The thunderstorm stops."),
    ("snow", "It stops snowing."),
    ("hail", "The hailstorm stops."),
    ("heat", "The heat wave eases."),
    ("sleet", "The sleet stops pouring down."),
    ("blizzard", "The blizzard lets up."),
    ("dust", "The dust storm ends."),
    ("drought", "The drought is finally over."),
    ("cold", "The cold snap is over."),
    ("fog", "The fog lifts."),
];

/// What the `time` command says at night instead, for two kinds.
const AT_NIGHT: [(&str, &str); 2] = [("cloudy", "The clouds obscure the moon."), ("fog", "It is very foggy.")];

const NO_SKY: &str = "You can't tell much about the weather from here.";

/// An area's weather is asked for (WEATHER, `Commands/Weather.java`, one
/// line) at most this often: the game says when it changes.
const ASK_AGAIN: std::time::Duration = std::time::Duration::from_secs(15 * 60);
/// How long the asked weather's answer is waited for, to hide it.
const ANSWER_WAITS: std::time::Duration = std::time::Duration::from_secs(4);

/// What a line said about the weather.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    /// The weather is now this kind.
    Weather(&'static str),
    /// This kind ended (and nothing new was said on the line).
    Ended(&'static str),
    /// No sky here.
    NoSky,
}

/// Each full sentence a description can be, with its kind, longest first
/// (so "It is very hot and muggy." wins over "It is very hot.").
fn sentences() -> Vec<(&'static str, String)> {
    let mut all = Vec::new();
    for (kind, openings, endings) in DESCRIPTIONS {
        for opening in openings {
            if endings.is_empty() {
                all.push((kind, (*opening).to_string()));
            }
            for ending in endings {
                all.push((kind, format!("{opening} {ending}")));
            }
        }
    }
    for (kind, said) in AT_NIGHT {
        all.push((kind, said.to_string()));
    }
    // Stable: of two the same ("A slushy snow"), snow (listed first) wins.
    all.sort_by_key(|(_, s)| std::cmp::Reverse(s.len()));
    all
}

/// Spaces run together, as the server's wrapping and padding may leave them.
fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

enum Described {
    Full(&'static str),
    /// The start of a description, the rest presumably on the next line.
    Partial,
    None,
}

/// Reads the weather out of the game's lines.
pub struct WeatherReader {
    sentences: Vec<(&'static str, String)>,
    /// The start of a description wrapped onto the next line.
    pending: Option<String>,
}

impl Default for WeatherReader {
    fn default() -> Self {
        WeatherReader { sentences: sentences(), pending: None }
    }
}

impl WeatherReader {
    fn describe(&self, text: &str) -> Described {
        if let Some((kind, _)) = self.sentences.iter().find(|(_, s)| text.starts_with(s.as_str())) {
            return Described::Full(kind);
        }
        // A wrapped start: at least a word and a half, so a line that's
        // only "A" isn't held for the next.
        if text.len() >= 8 && self.sentences.iter().any(|(_, s)| s.starts_with(text)) {
            return Described::Partial;
        }
        Described::None
    }

    /// What a line (its text, without color) says about the weather, if
    /// anything. Call it with every finished line, in order.
    pub fn line(&mut self, line: &str) -> Option<Seen> {
        let line = squeeze(line);
        if let Some(start) = self.pending.take() {
            let joined = format!("{start} {line}");
            if let Described::Full(kind) = self.describe(&joined) {
                return Some(Seen::Weather(kind));
            }
            // Not the rest of it after all: the line on its own.
        }
        // Where the game can start a line: its start, or after a prompt.
        let starts = std::iter::once(0).chain(line.match_indices('>').map(|(i, _)| i + 1));
        for start in starts {
            let text = line[start..].trim_start();
            if let Some(seen) = self.read(text) {
                return Some(seen);
            }
        }
        None
    }

    fn read(&mut self, text: &str) -> Option<Seen> {
        if text.starts_with(NO_SKY) {
            return Some(Seen::NoSky);
        }
        if let Some((kind, end)) = ENDS.iter().find(|(_, end)| text.starts_with(end)) {
            let rest = text[end.len()..].trim_start();
            return Some(match self.describe(rest) {
                Described::Full(new) => Seen::Weather(new),
                Described::Partial => {
                    self.pending = Some(rest.to_string());
                    Seen::Ended(kind)
                }
                Described::None => Seen::Ended(kind),
            });
        }
        match self.describe(text) {
            Described::Full(kind) => Some(Seen::Weather(kind)),
            Described::Partial => {
                self.pending = Some(text.to_string());
                None
            }
            Described::None => None,
        }
    }
}

/// A loop to play: an asset and its volume (a share of its bus).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Loop {
    pub path: String,
    pub volume: u8,
}

/// What should be looping now; sent to the frontend when it changes.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Ambience {
    pub bgn: Option<Loop>,
    pub bgw: Option<Loop>,
}

/// Where the player is, from `room.info`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Room {
    pub id: String,
    pub zone: String,
    pub terrain: String,
}

/// The facts ambience depends on.
#[derive(Default)]
pub struct Ambient {
    pub room: Option<Room>,
    /// The weather last read in each area, by its name.
    weather: HashMap<String, &'static str>,
    pub reader: WeatherReader,
    /// When each area's weather was last asked for, by its name.
    asked: HashMap<String, Instant>,
    /// The weather asked for, its answer not yet come.
    asking: Option<Instant>,
}

/// A pair's value as the hooks keep it: its JSON text.
pub fn json_text(text: &str) -> String {
    serde_json::Value::String(text.to_string()).to_string()
}

impl Ambient {
    /// The player moved (or the room's facts changed).
    pub fn enter(&mut self, room: Room) {
        self.room = Some(room);
    }

    /// The connection ended: nowhere, and the weather forgotten.
    pub fn leave(&mut self) {
        self.room = None;
        self.weather.clear();
        self.reader.pending = None;
        self.asked.clear();
        self.asking = None;
    }

    /// Whether to ask the game for the weather now: the player's under
    /// the sky in an area whose weather isn't known, and it hasn't been
    /// asked lately. The game says the weather only when it changes, or
    /// on stepping outside when it isn't clear, so without asking the
    /// painter's sky would stay clear (CoffeeMUD has no weather GMCP).
    /// The caller asks only when it's gentle to (`who.rs`'s `gentle`).
    pub fn weather_wanted(&self, now: Instant) -> bool {
        let Some(room) = &self.room else { return false };
        has_sky(&room.terrain)
            && self.asking.is_none()
            && !self.weather.contains_key(&room.zone)
            && self.asked.get(&room.zone).is_none_or(|at| now.saturating_duration_since(*at) >= ASK_AGAIN)
    }

    /// WEATHER was sent for the player's area.
    pub fn weather_asked(&mut self, now: Instant) {
        if let Some(room) = &self.room {
            self.asked.insert(room.zone.clone(), now);
        }
        self.asking = Some(now);
    }

    /// Whether a line the reader made something of is the asked
    /// weather's answer, to be hidden: the first one after asking.
    pub fn answered(&mut self, now: Instant) -> bool {
        self.asking.take().is_some_and(|at| now.saturating_duration_since(at) <= ANSWER_WAITS)
    }

    /// What a line said, applied to the weather of the player's area.
    /// Returns the kind to record, if it named one.
    pub fn saw(&mut self, seen: Seen) -> Option<&'static str> {
        let zone = self.room.as_ref().map(|r| r.zone.clone()).unwrap_or_default();
        match seen {
            Seen::Weather(kind) => {
                self.weather.insert(zone, kind);
                Some(kind)
            }
            // The old weather ended: clear, unless something newer was read.
            Seen::Ended(kind) => {
                if self.weather.get(&zone).is_none_or(|now| *now == kind) {
                    self.weather.insert(zone, "clear");
                    return Some("clear");
                }
                None
            }
            Seen::NoSky => None,
        }
    }

    /// The weather in the player's area, if it's been read.
    pub fn weather(&self) -> Option<&'static str> {
        self.weather.get(&self.room.as_ref()?.zone).copied()
    }

    /// What should loop now, from the triggers. BGN: the room's own, else
    /// its terrain's, else its room type's. BGW: the weather's, under the sky.
    pub fn ambience(&self, trigger: impl Fn(&str, &str) -> Option<Trigger>) -> Ambience {
        let Some(room) = &self.room else { return Ambience::default() };
        let mut places = vec![("room.info.id", json_text(&room.id)), ("room.info.terrain", json_text(&room.terrain))];
        if let Some(kind) = room_type(&room.terrain) {
            places.push((ROOM_TYPE_KEY, json_text(kind)));
        }
        let bgn = places.iter().find_map(|(key, value)| {
            let t = trigger(key, value)?;
            Some(Loop { path: t.bgn?, volume: t.bgn_volume })
        });
        let bgw = self.weather().filter(|_| has_sky(&room.terrain)).and_then(|kind| {
            let t = trigger(WEATHER_KEY, &json_text(kind))?;
            Some(Loop { path: t.bgw?, volume: t.bgw_volume })
        });
        Ambience { bgn, bgw }
    }

    /// `ambience`, from the hooks database.
    pub fn ambience_from(&self, hooks: &Hooks) -> Ambience {
        self.ambience(|key, value| hooks.trigger(key, value).cloned())
    }
}

#[cfg(test)]
mod ask_tests {
    use super::*;
    use std::time::Duration;

    fn room(zone: &str, terrain: &str) -> Room {
        Room { id: format!("{zone}#1"), zone: zone.into(), terrain: terrain.into() }
    }

    #[test]
    fn the_weather_is_asked_under_an_unknown_sky_once_in_a_while() {
        let t = Instant::now();
        let mut a = Ambient::default();
        assert!(!a.weather_wanted(t), "nowhere yet");
        a.enter(room("Midgaard", "stone"));
        assert!(!a.weather_wanted(t), "indoors");
        a.enter(room("Midgaard", "underwater"));
        assert!(!a.weather_wanted(t), "underwater");
        a.enter(room("Midgaard", "city"));
        assert!(a.weather_wanted(t));
        a.weather_asked(t);
        assert!(!a.weather_wanted(t), "waiting for the answer");
        let seen = a.reader.line("A light rain falls from the sky.").unwrap();
        assert!(a.answered(t + Duration::from_secs(1)), "the answer is hidden");
        a.saw(seen);
        assert!(!a.answered(t + Duration::from_secs(1)), "only the first");
        assert!(!a.weather_wanted(t + ASK_AGAIN), "known now");
        // Another area: asked; not again soon if it never said.
        a.enter(room("Woods", "woods"));
        assert!(a.weather_wanted(t));
        a.weather_asked(t);
        assert!(!a.answered(t + ANSWER_WAITS + Duration::from_secs(1)), "too late to be the answer");
        assert!(!a.weather_wanted(t + Duration::from_secs(60)));
        assert!(a.weather_wanted(t + ASK_AGAIN));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrains_are_indoors_or_outdoors() {
        assert_eq!(room_type("woods"), Some("outdoors"));
        assert_eq!(room_type("STONE"), Some("indoors"));
        assert_eq!(room_type("in_underwater"), Some("indoors"));
        assert_eq!(room_type("somewhere"), None);
        assert!(has_sky("plains"));
        assert!(!has_sky("underwater"));
        assert!(!has_sky("cave"));
    }

    #[test]
    fn every_description_is_read_as_its_kind() {
        let mut reader = WeatherReader::default();
        for (kind, openings, endings) in DESCRIPTIONS {
            for opening in openings {
                let lines: Vec<String> = if endings.is_empty() { vec![opening.to_string()] } else { endings.iter().map(|e| format!("{opening} {e}")).collect() };
                for line in lines {
                    // The one opening snow and sleet share reads as snow.
                    let expect = if *opening == "A slushy snow" { "snow" } else { kind };
                    assert_eq!(reader.line(&line), Some(Seen::Weather(expect)), "{line}");
                }
            }
        }
    }

    #[test]
    fn the_longer_sentence_wins() {
        let mut reader = WeatherReader::default();
        assert_eq!(reader.line("It is very hot and muggy."), Some(Seen::Weather("heat")));
        assert_eq!(reader.line("It is very cold and dry."), Some(Seen::Weather("cold")));
    }

    #[test]
    fn an_end_alone_or_with_the_new_weather() {
        let mut reader = WeatherReader::default();
        assert_eq!(reader.line("It stops raining."), Some(Seen::Ended("rain")));
        assert_eq!(reader.line("The clouds dissipate. A light rain falls from the sky."), Some(Seen::Weather("rain")));
        assert_eq!(reader.line("You can't tell much about the weather from here."), Some(Seen::NoSky));
    }

    #[test]
    fn after_a_prompt_but_not_inside_speech() {
        let mut reader = WeatherReader::default();
        assert_eq!(reader.line("<100Hp 50m 80mv> A cold wind blows through here."), Some(Seen::Weather("windy")));
        assert_eq!(reader.line("Bob says 'A light rain falls from the sky.'"), None);
        assert_eq!(reader.line("You gossip 'It is cold.'"), None);
    }

    #[test]
    fn at_night_with_the_moon() {
        let mut reader = WeatherReader::default();
        assert_eq!(reader.line("A light snow falls from the sky. You can't see the moon."), Some(Seen::Weather("snow")));
        assert_eq!(reader.line("The clouds obscure the moon."), Some(Seen::Weather("cloudy")));
    }

    #[test]
    fn a_wrapped_description_is_joined_up() {
        let mut reader = WeatherReader::default();
        assert_eq!(reader.line("The thunderstorm stops. A heavy and blusterous"), Some(Seen::Ended("thunderstorm")));
        assert_eq!(reader.line("rainstorm pours down from above."), Some(Seen::Weather("thunderstorm")));
        // A start that isn't followed by the rest is let go, and the next line read for itself.
        assert_eq!(reader.line("A light drizzling"), None);
        assert_eq!(reader.line("A thick fog descends."), Some(Seen::Weather("fog")));
    }

    #[test]
    fn ordinary_lines_say_nothing() {
        let mut reader = WeatherReader::default();
        for line in ["The Temple of Mota", "A", "You are in the southern end of the temple hall.", "It is cold and dark in here, but you see a door."] {
            assert_eq!(reader.line(line), None, "{line}");
        }
    }

    fn with(bgn: Option<&str>, bgw: Option<&str>) -> Trigger {
        Trigger { bgn: bgn.map(Into::into), bgn_volume: 50, bgw: bgw.map(Into::into), bgw_volume: 60, ..Trigger::default() }
    }

    fn town() -> Room {
        Room { id: "Midgaard#3001".into(), zone: "Midgaard".into(), terrain: "city".into() }
    }

    #[test]
    fn the_room_wins_over_its_terrain_over_its_type() {
        let mut ambient = Ambient::default();
        ambient.enter(town());
        let triggers = |room: bool, terrain: bool| {
            move |key: &str, value: &str| match (key, value) {
                ("room.info.id", r#""Midgaard#3001""#) if room => Some(with(Some("wav/temple.wav"), None)),
                ("room.info.terrain", r#""city""#) if terrain => Some(with(Some("ogg/city.ogg"), None)),
                ("coupler.room.type", r#""outdoors""#) => Some(with(Some("wav/outside.wav"), None)),
                _ => None,
            }
        };
        assert_eq!(ambient.ambience(triggers(true, true)).bgn.unwrap().path, "wav/temple.wav");
        assert_eq!(ambient.ambience(triggers(false, true)).bgn.unwrap().path, "ogg/city.ogg");
        assert_eq!(ambient.ambience(triggers(false, false)).bgn, Some(Loop { path: "wav/outside.wav".into(), volume: 50 }));
    }

    #[test]
    fn the_weather_plays_under_the_sky_in_its_own_area() {
        let mut ambient = Ambient::default();
        let rain = |key: &str, value: &str| (key == WEATHER_KEY && value == r#""rain""#).then(|| with(None, Some("wav/rain.wav")));
        ambient.enter(town());
        assert_eq!(ambient.ambience(rain).bgw, None, "not read yet");
        assert_eq!(ambient.saw(Seen::Weather("rain")), Some("rain"));
        assert_eq!(ambient.ambience(rain).bgw, Some(Loop { path: "wav/rain.wav".into(), volume: 60 }));
        // Indoors in the same area: quiet, and back outside it plays again.
        ambient.enter(Room { terrain: "stone".into(), ..town() });
        assert_eq!(ambient.ambience(rain).bgw, None);
        ambient.enter(town());
        assert!(ambient.ambience(rain).bgw.is_some());
        // Another area's weather hasn't been read.
        ambient.enter(Room { zone: "Elsewhere".into(), ..town() });
        assert_eq!(ambient.ambience(rain).bgw, None);
        ambient.enter(town());
        // It stops: clear, which has no BGW here.
        assert_eq!(ambient.saw(Seen::Ended("rain")), Some("clear"));
        assert_eq!(ambient.ambience(rain).bgw, None);
        // An end of weather that isn't this area's changes nothing.
        ambient.saw(Seen::Weather("fog"));
        assert_eq!(ambient.saw(Seen::Ended("rain")), None);
        assert_eq!(ambient.weather(), Some("fog"));
    }

    #[test]
    fn nothing_plays_out_of_the_game() {
        let mut ambient = Ambient::default();
        ambient.enter(town());
        ambient.saw(Seen::Weather("rain"));
        ambient.leave();
        assert_eq!(ambient.ambience(|_, _| Some(with(Some("a.wav"), Some("b.wav")))), Ambience::default());
    }
}

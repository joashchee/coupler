//! **The other ways out**: portals, ships, caravans and the like, pure
//! and unit-tested. CoffeeMUD's GMCP `room.info` lists only the
//! directions; an item that's an exit (`StdPortal`, every `Boardable`:
//! ships, caravans, castles) shows only in the room's exits text
//! (`CommonMsgs.lookAtExitsShort` and `lookAtExits`), after the
//! directions, by its door name:
//!
//! - `[Exits: north east gangplank portal]` (AUTOEXITS, the usual), the
//!   names space-separated, wrapped across lines to the `]`;
//! - `Obvious exits:` and a row each, `North: The town square` for a
//!   direction and `gangplank: a sailing ship Foo` for an item, its door
//!   name, a colon and what it is (`: a shimmering portal` when the
//!   builder gave it no door name).
//!
//! A boardable's door name is `gangplank` unless its builder named it
//! (`StdBoardable`); a space ship's `hatch`, a castle's `portcullis`; a
//! portal's is the builder's. The game's own MXP link for one is
//! `ENTER <door name>` (`resources/text/mxp.txt`, `MEX`), so that's the
//! way to go through it, and the item's name says what it is: told from
//! the long list's row, or else from the room's items (GMCP
//! `room.items.list`). On a ship or a caravan the directions are its
//! own (`Directions.java`: foreward, aft, starboard, portside…), never
//! a way.
//!
//! `session.rs` feeds it every finished line and the room's items, puts
//! what an exits block names on the map's room (`Map::set_ways`), and
//! emits `ways` when the player comes to a room that has some
//! (`announce`), for Immersive's narrator. Immersive leaves the exits'
//! lines out of the game output (`src/lib/output.ts`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What kind of way out it is.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Portal,
    Ship,
    Caravan,
    /// A castle's portcullis, or anything else that's an exit.
    Other,
}

/// One way out that isn't a direction.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Way {
    pub kind: Kind,
    /// What to type to go through it: `ENTER gangplank`.
    pub command: String,
    /// What it is, as the game names it ("a sailing ship Windrunner"),
    /// or its door name when the game said no more.
    pub name: String,
}

/// Every direction's word, compass, shipboard and caravan
/// (`Directions.java`'s `DEFAULT_DIRECTIONS_LIST_*`), a word at a time:
/// the two-word ones (`Fore Starboard`) are two of these.
const DIRECTIONS: &[&str] = &[
    "north", "south", "east", "west", "up", "down", "there", "northeast", "northwest", "southeast", "southwest", "foreward", "forward", "fore", "aft", "starboard",
    "portside", "port", "above", "below", "back", "backward", "right", "left",
];

/// The door names the game's own items have, each one way by itself in
/// the bracketed list: a name after one of these is another way.
const DOORS: &[&str] = &["gangplank", "hatch", "portcullis"];

/// Words in a name that tell a ship or a caravan.
const SHIPS: &[&str] = &[
    "ship", "boat", "galleon", "sloop", "schooner", "frigate", "barge", "raft", "canoe", "junk", "longship", "airship", "skiff", "yacht", "vessel", "carrack",
    "caravel", "cog", "brig", "keelboat", "dinghy", "gondola", "spaceship", "shuttle", "cutter", "ketch", "dhow", "trireme", "longboat", "ferry",
];
const CARAVANS: &[&str] = &["caravan", "wagon", "cart", "carriage", "coach", "buggy", "stagecoach", "chariot", "sled", "sledge", "wain", "caboose"];

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase)
}

fn is_direction(word: &str) -> bool {
    DIRECTIONS.contains(&word.to_lowercase().as_str())
}

/// A ship or a caravan, by a word in its name; None when it says neither.
fn vehicle(name: &str) -> Option<Kind> {
    words(name).find_map(|w| {
        if SHIPS.contains(&w.as_str()) {
            Some(Kind::Ship)
        } else if CARAVANS.contains(&w.as_str()) {
            Some(Kind::Caravan)
        } else {
            None
        }
    })
}

/// The room's items, the colors and blurbs aside.
fn item_names(data: &str) -> Option<Vec<String>> {
    let value: Value = serde_json::from_str(data).ok()?;
    if value.get("location").and_then(Value::as_str) != Some("room") {
        return None;
    }
    Some(value.get("items")?.as_array()?.iter().filter_map(|i| i.get("name").and_then(Value::as_str)).map(str::to_string).collect())
}

/// The exits block being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Open {
    /// `[Exits: …`, not yet at its `]`.
    Bracket,
    /// `Obvious exits:`'s rows.
    Rows,
}

#[derive(Default)]
pub struct Ways {
    open: Option<Open>,
    /// The door names (and what the long list says each is) found so far.
    found: Vec<(String, Option<String>)>,
    /// The bracketed list's text so far, across its lines.
    bracket: String,
    /// The last row ran to the wrap: the next line goes on with it.
    wrapped: bool,
    /// The room's items, by name.
    items: Vec<String>,
    /// The room and the ways last told (`announce`).
    told: Option<(String, Vec<Way>)>,
}

/// The start of the room's exits, either form.
fn exits_start(text: &str) -> Option<(Open, &str)> {
    let t = text.trim_start();
    let lower = t.to_lowercase();
    for (lead, open) in [("[exits:", Open::Bracket), ("[exit:", Open::Bracket), ("obvious exits:", Open::Rows), ("obvious exit:", Open::Rows)] {
        if lower.starts_with(lead) {
            return Some((open, &t[lead.len()..]));
        }
    }
    None
}

/// A row this long was wrapped by the game (src/lib/output.ts's `WRAPPED`).
const WRAPPED: usize = 70;

impl Ways {
    /// A GMCP message: the room's items, to tell what a way is.
    pub fn gmcp(&mut self, package: &str, data: &str) {
        if package.eq_ignore_ascii_case("room.items.list") {
            if let Some(names) = item_names(data) {
                self.items = names;
            }
        } else if package.eq_ignore_ascii_case("room.info") {
            self.items.clear();
        }
    }

    /// A finished line. Returns the ways an exits block named once it's
    /// over (an empty list for one with none).
    pub fn line(&mut self, text: &str) -> Option<Vec<Way>> {
        match self.open {
            Some(Open::Bracket) => {
                self.bracket.push(' ');
                self.bracket.push_str(text);
                return text.contains(']').then(|| self.close());
            }
            Some(Open::Rows) => {
                if self.wrapped {
                    if let Some((_, Some(what))) = self.found.last_mut() {
                        what.push(' ');
                        what.push_str(text.trim());
                    }
                    self.wrapped = text.trim_end().len() >= WRAPPED;
                    return None;
                }
                if let Some((name, what)) = row(text) {
                    if !name.split_whitespace().all(is_direction) || name.is_empty() {
                        self.found.push((name, Some(what)));
                    }
                    self.wrapped = text.trim_end().len() >= WRAPPED;
                    return None;
                }
                let ways = self.close();
                // A block can follow another at once.
                return match self.line(text) {
                    Some(next) => Some(next),
                    None => Some(ways),
                };
            }
            None => {}
        }
        let (open, rest) = exits_start(text)?;
        self.found.clear();
        self.wrapped = false;
        match open {
            Open::Bracket => {
                self.bracket = rest.to_string();
                self.open = Some(Open::Bracket);
                rest.contains(']').then(|| self.close())
            }
            Open::Rows => {
                self.open = Some(Open::Rows);
                None
            }
        }
    }

    /// The read ended at a prompt: a list of rows is over.
    pub fn prompt(&mut self) -> Option<Vec<Way>> {
        (self.open == Some(Open::Rows)).then(|| self.close())
    }

    /// The connection's over.
    pub fn clear(&mut self) {
        *self = Ways::default();
    }

    fn close(&mut self) -> Vec<Way> {
        if self.open.take() == Some(Open::Bracket) {
            let inside = self.bracket.split(']').next().unwrap_or("").to_string();
            self.found = bracketed(&inside).into_iter().map(|n| (n, None)).collect();
        }
        self.bracket.clear();
        self.wrapped = false;
        let found = std::mem::take(&mut self.found);
        found.into_iter().map(|(door, what)| self.way(&door, what.as_deref())).collect()
    }

    /// One way: its kind and name from what the long list says it is, or
    /// else from the room's items.
    fn way(&self, door: &str, what: Option<&str>) -> Way {
        let door = door.trim();
        let what = what.map(|w| w.trim().trim_end_matches('.').trim()).filter(|w| !w.is_empty());
        let command = format!("ENTER {}", if door.is_empty() { what.and_then(|w| words(w).last()).unwrap_or_default() } else { door.to_lowercase() });
        let lower = door.to_lowercase();
        // A boardable's door: the room's ship or caravan says which.
        let boarded = lower == "gangplank" || lower == "hatch";
        let item = |kind: Kind| self.items.iter().find(|i| vehicle(i) == Some(kind)).cloned();
        let (kind, name) = match what {
            Some(what) => (vehicle(what).or_else(|| vehicle(door)).unwrap_or(if lower == "portcullis" { Kind::Other } else if boarded { Kind::Ship } else { Kind::Portal }), what.to_string()),
            None if boarded => match (item(Kind::Ship), item(Kind::Caravan)) {
                (Some(ship), None) => (Kind::Ship, ship),
                (None, Some(caravan)) => (Kind::Caravan, caravan),
                (Some(_), Some(_)) | (None, None) => (if lower == "hatch" { Kind::Ship } else { vehicle(door).unwrap_or(Kind::Ship) }, door.to_string()),
            },
            None if lower == "portcullis" => (Kind::Other, door.to_string()),
            None => {
                // A portal's door name is often a word of its item's name.
                let named = self.items.iter().find(|i| words(i).any(|w| w == lower)).cloned();
                let kind = vehicle(door).or_else(|| named.as_deref().and_then(vehicle)).unwrap_or(Kind::Portal);
                (kind, named.unwrap_or_else(|| door.to_string()))
            }
        };
        Way { kind, command, name }
    }

    /// Whether the narrator tells the ways in `room` now: when they're
    /// new there, or the player came from elsewhere. A room with none
    /// is told nothing, but a return to it tells them again.
    pub fn announce(&mut self, room: &str, ways: &[Way]) -> bool {
        let new = self.told.as_ref().is_none_or(|(r, w)| r != room || w != ways);
        self.told = Some((room.to_string(), ways.to_vec()));
        new && !ways.is_empty()
    }
}

/// The names after the directions in `[Exits: …]`: the words that
/// aren't a direction, run together, but for a game door name, which is
/// one by itself.
fn bracketed(inside: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    let flush = |run: &mut Vec<&str>, names: &mut Vec<String>| {
        if !run.is_empty() {
            names.push(run.join(" "));
            run.clear();
        }
    };
    for word in inside.split_whitespace() {
        let w = word.trim_matches(|c: char| c == ',' || c == '.');
        if w.is_empty() {
            continue;
        }
        if is_direction(w) && run.is_empty() {
            continue;
        }
        if is_direction(w) || DOORS.contains(&w.to_lowercase().as_str()) {
            flush(&mut run, &mut names);
            if !is_direction(w) {
                names.push(w.to_string());
            }
            continue;
        }
        run.push(w);
    }
    flush(&mut run, &mut names);
    names
}

/// A row of `Obvious exits:`: its name (a direction, a door name, or
/// nothing) and what's behind it. `None.` and anything else isn't one.
fn row(text: &str) -> Option<(String, String)> {
    let t = text.trim();
    let colon = t.find(':')?;
    let (name, rest) = (t[..colon].trim(), &t[colon + 1..]);
    if name.len() > 40 || !rest.starts_with(' ') || rest.trim().is_empty() {
        return None;
    }
    if !name.chars().all(|c| c.is_alphanumeric() || " ',-".contains(c)) {
        return None;
    }
    Some((name.to_string(), rest.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(ways: &mut Ways, lines: &[&str]) -> Option<Vec<Way>> {
        let mut out = None;
        for line in lines {
            if let Some(w) = ways.line(line) {
                out = Some(w);
            }
        }
        ways.prompt().or(out)
    }

    #[test]
    fn the_bracketed_list_names_the_ways_after_the_directions() {
        let mut ways = Ways::default();
        assert_eq!(ways.line("[Exits: north east]"), Some(vec![]));
        ways.gmcp("room.items.list", r#"{"location":"room","items":[{"id":1,"name":"a sailing ship Windrunner","attrib":""},{"id":2,"name":"a sword","attrib":""}]}"#);
        let found = ways.line("[Exits: north gangplank shimmering portal]").unwrap();
        assert_eq!(
            found,
            vec![
                Way { kind: Kind::Ship, command: "ENTER gangplank".into(), name: "a sailing ship Windrunner".into() },
                Way { kind: Kind::Portal, command: "ENTER shimmering portal".into(), name: "shimmering portal".into() },
            ]
        );
    }

    #[test]
    fn a_list_wrapped_across_lines_is_read_to_its_end() {
        let mut ways = Ways::default();
        ways.gmcp("room.items.list", r#"{"location":"room","items":[{"id":1,"name":"a caravan Dusty","attrib":""}]}"#);
        assert_eq!(ways.line("[Exits: north south east west northeast northwest southeast southwest up down"), None);
        let found = ways.line("gangplank]").unwrap();
        assert_eq!(found, vec![Way { kind: Kind::Caravan, command: "ENTER gangplank".into(), name: "a caravan Dusty".into() }]);
    }

    #[test]
    fn a_portal_by_its_door_name_takes_its_items_name() {
        let mut ways = Ways::default();
        ways.gmcp("room.items.list", r#"{"location":"room","items":[{"id":1,"name":"a glowing archway","attrib":""}]}"#);
        let found = ways.line("[Exits: west archway]").unwrap();
        assert_eq!(found, vec![Way { kind: Kind::Portal, command: "ENTER archway".into(), name: "a glowing archway".into() }]);
        // A new room forgets the last one's items.
        ways.gmcp("room.info", "{}");
        assert_eq!(ways.line("[Exits: west archway]").unwrap()[0].name, "archway");
    }

    #[test]
    fn the_long_list_says_what_each_is() {
        let mut ways = Ways::default();
        let found = read(
            &mut ways,
            &["Obvious exits:", "North: The town square", "Up   : A loft", "gangplank: a caravan Dusty", ": a swirling vortex", "portcullis: Castle Grey", "A rat is here."],
        )
        .unwrap();
        assert_eq!(
            found,
            vec![
                Way { kind: Kind::Caravan, command: "ENTER gangplank".into(), name: "a caravan Dusty".into() },
                Way { kind: Kind::Portal, command: "ENTER vortex".into(), name: "a swirling vortex".into() },
                Way { kind: Kind::Other, command: "ENTER portcullis".into(), name: "Castle Grey".into() },
            ]
        );
        // Ended by the prompt as well.
        assert_eq!(read(&mut ways, &["Obvious exits:", "gangplank: a sailing ship Swift"]).unwrap()[0].kind, Kind::Ship);
        assert_eq!(read(&mut ways, &["Obvious exits:", "None."]), Some(vec![]));
    }

    #[test]
    fn a_ships_own_directions_are_not_ways() {
        let mut ways = Ways::default();
        assert_eq!(ways.line("[Exits: foreward aft starboard portside below]"), Some(vec![]));
        assert_eq!(read(&mut ways, &["Obvious exits:", "Fore Starboard: The bow", "Below: The hold"]), Some(vec![]));
    }

    #[test]
    fn told_once_a_room() {
        let mut ways = Ways::default();
        let portal = vec![Way { kind: Kind::Portal, command: "ENTER portal".into(), name: "a portal".into() }];
        assert!(ways.announce("A#1", &portal));
        assert!(!ways.announce("A#1", &portal));
        assert!(!ways.announce("A#2", &[]));
        assert!(ways.announce("A#1", &portal));
    }

    #[test]
    fn other_lines_are_not_exits() {
        let mut ways = Ways::default();
        assert_eq!(ways.line("You say: hello"), None);
        assert_eq!(ways.prompt(), None);
    }
}

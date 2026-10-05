//! The auto-mapper: builds a map of CoffeeMUD from the GMCP `room.info`
//! and `room.exits` messages the game sends on every room change
//! (docs/coffeemud-gmcp.md has the field-by-field study of the server).
//! Pure and unit-tested like `telnet` and `ansi`: no I/O here; `session`
//! feeds it and saves what it builds.
//!
//! What the server gives makes this simpler than a text-scraping mapper:
//! every room has a stable ID (`Midgaard#3001`, or `Area#12#(3,4)` inside
//! a grid), and `idexits` names the room behind each exit even before the
//! player walks there. So rooms are keyed by ID, exits are recorded with
//! their destination, and unvisited neighbours are kept as stubs.
//!
//! The map has no stored coordinates. CoffeeMUD areas aren't laid out on
//! a grid (exits loop, overlap and skip), so the picture is laid out
//! fresh around the current room each time (`view`), and everything a
//! picture says is also available as words (`detail`, `route`,
//! `directions`), for people who can't see it.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The direction letters CoffeeMUD uses in GMCP (`Directions.java`'s
/// `DIRECTION_CHARS`), in the order exits are listed. `V` is a gate or
/// vortex ("there"): it has no place on a grid and isn't walked.
const DIRS: &[(&str, &str, (i32, i32, i32))] = &[
    ("N", "north", (0, -1, 0)),
    ("NE", "northeast", (1, -1, 0)),
    ("E", "east", (1, 0, 0)),
    ("SE", "southeast", (1, 1, 0)),
    ("S", "south", (0, 1, 0)),
    ("SW", "southwest", (-1, 1, 0)),
    ("W", "west", (-1, 0, 0)),
    ("NW", "northwest", (-1, -1, 0)),
    ("U", "up", (0, 0, 1)),
    ("D", "down", (0, 0, -1)),
    ("V", "there", (0, 0, 0)),
];

fn dir_rank(dir: &str) -> usize {
    DIRS.iter().position(|d| d.0 == dir).unwrap_or(DIRS.len())
}

/// The word for a direction letter: what the player types, and what's read out.
pub fn dir_name(dir: &str) -> &str {
    DIRS.iter().find(|d| d.0 == dir).map_or(dir, |d| d.1)
}

fn dir_delta(dir: &str) -> Option<(i32, i32, i32)> {
    DIRS.iter().find(|d| d.0 == dir && d.0 != "V").map(|d| d.2)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Exit {
    /// The room behind the exit (its ID).
    pub to: String,
    #[serde(default)]
    pub door: bool,
    /// Known only for the room the player is standing in, as of the last visit.
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Room {
    #[serde(default)]
    pub name: String,
    /// The area's name. Empty for a stub (a room seen only as an exit).
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub terrain: String,
    /// "normal", "swim", "fly" or "crawl".
    #[serde(default)]
    pub travel: String,
    /// Keyed by direction letter.
    #[serde(default)]
    pub exits: BTreeMap<String, Exit>,
    /// The player has stood here.
    #[serde(default)]
    pub visited: bool,
    /// The player's own name for the room (a landmark to find and walk to).
    #[serde(default)]
    pub landmark: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Map {
    /// Bumped if the saved shape ever changes.
    #[serde(default)]
    version: u32,
    #[serde(default)]
    rooms: HashMap<String, Room>,
    /// Where the player is, when the game has said. Not saved.
    #[serde(skip)]
    current: Option<String>,
}

/// One exit of the current room, in words.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExitDetail {
    pub dir: String,
    /// "north", "up", …: also the command that walks it.
    pub dir_name: String,
    pub to: String,
    /// The destination's name, empty when it hasn't been visited.
    pub to_name: String,
    pub visited: bool,
    pub door: bool,
    pub open: bool,
    pub locked: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoomDetail {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub terrain: String,
    pub travel: String,
    pub landmark: String,
    pub exits: Vec<ExitDetail>,
}

/// A room placed on the picture, relative to the current room at (0, 0).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub visited: bool,
    pub current: bool,
    pub up: bool,
    pub down: bool,
    pub landmark: bool,
    /// In a different area from the current room: drawn, not explored from.
    pub other_zone: bool,
    /// Compass exits this room has, as direction letters.
    pub exits: Vec<String>,
    /// The game's terrain, lower case (`woods`, `stone`...): the picture
    /// colors the room by it. Empty for a room not visited.
    pub terrain: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub room: Option<RoomDetail>,
    pub cells: Vec<Cell>,
    /// Rooms the player has visited, and areas they're in.
    pub rooms_known: usize,
    pub zones_known: usize,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoomRef {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub landmark: String,
    /// So the rooms found say in words what the picture says in color.
    pub terrain: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub dir: String,
    pub to: String,
}

/// CoffeeMUD's own color codes (`^w`, `^#123`, `^|123`, `^^`) and ANSI
/// escapes can ride along in names. The map keeps plain words.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '^' => match chars.next() {
                Some('^') => out.push('^'),
                Some('#') | Some('|') => {
                    for _ in 0..3 {
                        if chars.peek().is_some_and(|d| d.is_ascii_digit()) {
                            chars.next();
                        }
                    }
                }
                _ => {}
            },
            '\u{1b}' => {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for d in chars.by_ref() {
                        if d.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_field(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).map(plain).unwrap_or_default()
}

impl Map {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("The saved map couldn't be read. ({e})"))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a map is plain data")
    }

    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// The current room's name as the game gives it (never a landmark's).
    pub fn here_name(&self) -> Option<&str> {
        self.rooms.get(self.current.as_deref()?).map(|r| r.name.as_str()).filter(|n| !n.is_empty())
    }

    /// The player is somewhere the map can't follow (disconnected, or a
    /// room with no ID).
    pub fn leave(&mut self) {
        self.current = None;
    }

    /// A `room.info` message. Records the room, its exits and stubs for
    /// the rooms behind them, and moves the player there. Returns whether
    /// anything worth saving changed.
    pub fn apply_room_info(&mut self, json: &str) -> Result<bool, String> {
        let v: Value = serde_json::from_str(json).map_err(|e| format!("room.info wasn't valid JSON. ({e})"))?;
        let id = v.get("id").and_then(Value::as_str).unwrap_or("").trim().to_string();
        if id.is_empty() {
            // Rooms without an ID (some temporary ones) can't be told apart.
            self.current = None;
            return Ok(false);
        }
        let mut exits = BTreeMap::new();
        if let Some(list) = v.get("idexits").and_then(Value::as_object) {
            for (dir, to) in list {
                if let Some(to) = to.as_str().filter(|t| !t.is_empty()) {
                    exits.insert(dir.to_uppercase(), to.to_string());
                }
            }
        }
        let mut changed = false;
        for to in exits.values() {
            if !self.rooms.contains_key(to) {
                self.rooms.insert(to.clone(), Room::default());
                changed = true;
            }
        }
        let room = self.rooms.entry(id.clone()).or_default();
        let (name, zone, terrain, travel) = (text_field(&v, "name"), text_field(&v, "zone"), text_field(&v, "terrain"), text_field(&v, "move"));
        if !room.visited || room.name != name || room.zone != zone || room.terrain != terrain || room.travel != travel {
            changed = true;
        }
        room.visited = true;
        room.name = name;
        room.zone = zone;
        room.terrain = terrain;
        room.travel = travel;
        // The game lists only the exits this character can see right now.
        // A hidden exit seen before is kept (it's still there); an exit
        // whose destination changed is replaced.
        for (dir, to) in exits {
            match room.exits.get_mut(&dir) {
                Some(exit) if exit.to == to => {}
                Some(exit) => {
                    *exit = Exit { to, ..Exit::default() };
                    changed = true;
                }
                None => {
                    room.exits.insert(dir, Exit { to, open: true, ..Exit::default() });
                    changed = true;
                }
            }
        }
        self.current = Some(id);
        Ok(changed)
    }

    /// A `room.exits` message: doors and locks for the current room's exits.
    pub fn apply_room_exits(&mut self, json: &str) -> Result<bool, String> {
        let v: Value = serde_json::from_str(json).map_err(|e| format!("room.exits wasn't valid JSON. ({e})"))?;
        let Some(room) = self.current.as_ref().and_then(|id| self.rooms.get_mut(id)) else {
            return Ok(false);
        };
        let mut changed = false;
        if let Some(list) = v.get("exits").and_then(Value::as_object) {
            for (dir, info) in list {
                let Some(exit) = room.exits.get_mut(&dir.to_uppercase()) else { continue };
                let door = info.get("door").and_then(Value::as_str).is_some_and(|d| !d.is_empty());
                let open = info.get("open").and_then(Value::as_bool).unwrap_or(true);
                let locked = info.get("locked").and_then(Value::as_bool).unwrap_or(false);
                if (exit.door, exit.open, exit.locked) != (door, open, locked) {
                    (exit.door, exit.open, exit.locked) = (door, open, locked);
                    changed = true;
                }
            }
        }
        Ok(changed)
    }

    /// The current room in words.
    pub fn detail(&self) -> Option<RoomDetail> {
        let id = self.current.as_ref()?;
        let room = self.rooms.get(id)?;
        let mut exits: Vec<ExitDetail> = room
            .exits
            .iter()
            .map(|(dir, exit)| {
                let to = self.rooms.get(&exit.to);
                ExitDetail {
                    dir: dir.clone(),
                    dir_name: dir_name(dir).to_string(),
                    to: exit.to.clone(),
                    to_name: to.map(|r| r.name.clone()).unwrap_or_default(),
                    visited: to.is_some_and(|r| r.visited),
                    door: exit.door,
                    open: exit.open,
                    locked: exit.locked,
                }
            })
            .collect();
        exits.sort_by_key(|e| dir_rank(&e.dir));
        Some(RoomDetail {
            id: id.clone(),
            name: room.name.clone(),
            zone: room.zone.clone(),
            terrain: room.terrain.clone(),
            travel: room.travel.clone(),
            landmark: room.landmark.clone(),
            exits,
        })
    }

    /// The picture: rooms laid out around the current one, on its level,
    /// out to `rx` cells sideways and `ry` up and down. Laid out breadth
    /// first, so the nearest rooms are placed truest; a room whose cell
    /// is already taken is left out rather than drawn in the wrong place.
    pub fn view(&self, rx: i32, ry: i32) -> Vec<Cell> {
        let Some(start) = self.current.as_ref() else { return Vec::new() };
        let Some(here) = self.rooms.get(start) else { return Vec::new() };
        let mut at: HashMap<&str, (i32, i32, i32)> = HashMap::new();
        let mut taken: HashSet<(i32, i32, i32)> = HashSet::new();
        let mut queue = VecDeque::new();
        let mut order: Vec<&str> = Vec::new();
        at.insert(start, (0, 0, 0));
        taken.insert((0, 0, 0));
        queue.push_back(start.as_str());
        order.push(start);
        while let Some(id) = queue.pop_front() {
            let (room, (x, y, z)) = (&self.rooms[id], at[id]);
            let mut exits: Vec<(&String, &Exit)> = room.exits.iter().collect();
            exits.sort_by_key(|(dir, _)| dir_rank(dir));
            for (dir, exit) in exits {
                let Some((dx, dy, dz)) = dir_delta(dir) else { continue };
                let spot = (x + dx, y + dy, z + dz);
                let Some(next) = self.rooms.get(&exit.to) else { continue };
                if at.contains_key(exit.to.as_str()) || taken.contains(&spot) || spot.0.abs() > rx || spot.1.abs() > ry || spot.2.abs() > 3 {
                    continue;
                }
                at.insert(&exit.to, spot);
                taken.insert(spot);
                order.push(&exit.to);
                // Stubs and other areas are drawn but not explored from.
                if next.visited && next.zone == here.zone {
                    queue.push_back(&exit.to);
                }
            }
        }
        order
            .into_iter()
            .filter(|id| at[id].2 == 0)
            .map(|id| {
                let room = &self.rooms[id];
                let (x, y, _) = at[id];
                let mut exits: Vec<String> = room.exits.keys().filter(|d| !matches!(d.as_str(), "U" | "D" | "V")).cloned().collect();
                exits.sort_by_key(|d| dir_rank(d));
                Cell {
                    id: id.to_string(),
                    name: room.name.clone(),
                    x,
                    y,
                    visited: room.visited,
                    current: id == start,
                    up: room.exits.contains_key("U"),
                    down: room.exits.contains_key("D"),
                    landmark: !room.landmark.is_empty(),
                    other_zone: room.visited && room.zone != here.zone,
                    exits,
                    terrain: room.terrain.to_ascii_lowercase(),
                }
            })
            .collect()
    }

    pub fn snapshot(&self, rx: i32, ry: i32) -> Snapshot {
        let zones: HashSet<&str> = self.rooms.values().filter(|r| r.visited).map(|r| r.zone.as_str()).collect();
        Snapshot {
            room: self.detail(),
            cells: self.view(rx, ry),
            rooms_known: self.rooms.values().filter(|r| r.visited).count(),
            zones_known: zones.len(),
        }
    }

    /// Visited rooms whose name, landmark or area contains `query`
    /// (any case), landmarks first, then the current area, then by name.
    pub fn find(&self, query: &str, limit: usize) -> Vec<RoomRef> {
        let q = query.trim().to_lowercase();
        let zone = self.current.as_ref().and_then(|id| self.rooms.get(id)).map(|r| r.zone.as_str()).unwrap_or("");
        let mut found: Vec<(&String, &Room)> = self
            .rooms
            .iter()
            .filter(|(_, r)| r.visited)
            .filter(|(_, r)| q.is_empty() && !r.landmark.is_empty() || !q.is_empty() && [&r.name, &r.landmark, &r.zone].iter().any(|s| s.to_lowercase().contains(&q)))
            .collect();
        found.sort_by(|a, b| {
            (a.1.landmark.is_empty(), a.1.zone != zone, &a.1.zone, &a.1.name, a.0).cmp(&(b.1.landmark.is_empty(), b.1.zone != zone, &b.1.zone, &b.1.name, b.0))
        });
        found
            .into_iter()
            .take(limit)
            .map(|(id, r)| RoomRef { id: id.clone(), name: r.name.clone(), zone: r.zone.clone(), landmark: r.landmark.clone(), terrain: r.terrain.to_ascii_lowercase() })
            .collect()
    }

    /// The shortest known way from the current room to `to`: only through
    /// rooms the player has visited, never through a door last seen
    /// locked or a gate. None when the map doesn't connect them.
    pub fn route(&self, to: &str) -> Option<Vec<Step>> {
        let from = self.current.as_deref()?;
        if !self.rooms.contains_key(to) {
            return None;
        }
        let mut came: HashMap<&str, (&str, &str)> = HashMap::new();
        let mut queue = VecDeque::from([from]);
        let mut seen: HashSet<&str> = HashSet::from([from]);
        while let Some(id) = queue.pop_front() {
            if id == to {
                break;
            }
            let room = &self.rooms[id];
            if !room.visited {
                continue;
            }
            let mut exits: Vec<(&String, &Exit)> = room.exits.iter().collect();
            exits.sort_by_key(|(dir, _)| dir_rank(dir));
            for (dir, exit) in exits {
                if dir == "V" || exit.locked || !self.rooms.contains_key(&exit.to) || !seen.insert(&exit.to) {
                    continue;
                }
                came.insert(&exit.to, (id, dir));
                queue.push_back(&exit.to);
            }
        }
        let mut steps = Vec::new();
        let mut at = to;
        while at != from {
            let (prev, dir) = *came.get(at)?;
            steps.push(Step { dir: dir.to_string(), to: at.to_string() });
            at = prev;
        }
        steps.reverse();
        Some(steps)
    }

    /// The exit leading from the current room in `dir`.
    pub fn exit(&self, dir: &str) -> Option<&Exit> {
        self.rooms.get(self.current.as_ref()?)?.exits.get(dir)
    }

    /// Names (or clears, with an empty name) a landmark. False if the room is unknown.
    pub fn set_landmark(&mut self, id: &str, name: &str) -> bool {
        match self.rooms.get_mut(id) {
            Some(room) => {
                room.landmark = plain(name);
                true
            }
            None => false,
        }
    }

    pub fn room_name(&self, id: &str) -> String {
        self.rooms.get(id).map(|r| if r.landmark.is_empty() { r.name.clone() } else { r.landmark.clone() }).unwrap_or_default()
    }

    /// Forgets everything (the player asked to start the map over).
    pub fn clear(&mut self) {
        self.rooms.clear();
        self.current = None;
    }
}

/// A route in words, runs of the same direction counted: "3 north, east, 2 up".
pub fn directions(steps: &[Step]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < steps.len() {
        let mut n = 1;
        while i + n < steps.len() && steps[i + n].dir == steps[i].dir {
            n += 1;
        }
        let name = dir_name(&steps[i].dir);
        parts.push(if n == 1 { name.to_string() } else { format!("{n} {name}") });
        i += n;
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `room.info` as CMProtocols.java builds it.
    fn info(id: &str, name: &str, zone: &str, exits: &[(&str, &str)]) -> String {
        let idexits: serde_json::Map<String, Value> = exits.iter().map(|(d, to)| (d.to_string(), Value::from(*to))).collect();
        serde_json::json!({
            "num": 1, "id": id, "name": name, "zone": zone, "desc": "A place.", "terrain": "city", "move": "normal",
            "details": "", "extradata": {}, "exits": {}, "idexits": idexits, "coord": {"id": 0, "x": -1, "y": -1, "cont": 0}
        })
        .to_string()
    }

    /// A square: A -E- B, B -S- C, C -W- D, D -N- A, with a cellar under A.
    fn square() -> Map {
        let mut m = Map::default();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("S", "Z#4"), ("D", "Z#5")])).unwrap();
        m.apply_room_info(&info("Z#2", "B", "Zone", &[("W", "Z#1"), ("S", "Z#3")])).unwrap();
        m.apply_room_info(&info("Z#3", "C", "Zone", &[("N", "Z#2"), ("W", "Z#4")])).unwrap();
        m.apply_room_info(&info("Z#4", "D", "Zone", &[("N", "Z#1"), ("E", "Z#3")])).unwrap();
        m
    }

    #[test]
    fn records_rooms_exits_and_stubs() {
        let mut m = Map::default();
        assert!(m.apply_room_info(&info("Midgaard#3001", "The Temple", "Midgaard", &[("N", "Midgaard#3054"), ("d", "Midgaard#3005")])).unwrap());
        let d = m.detail().unwrap();
        assert_eq!((d.name.as_str(), d.zone.as_str()), ("The Temple", "Midgaard"));
        assert_eq!(d.exits.iter().map(|e| e.dir_name.as_str()).collect::<Vec<_>>(), ["north", "down"]);
        assert!(d.exits.iter().all(|e| !e.visited && e.to_name.is_empty()));
        let s = m.snapshot(5, 5);
        assert_eq!((s.rooms_known, s.zones_known), (1, 1));
        // The same room again changes nothing worth saving.
        assert!(!m.apply_room_info(&info("Midgaard#3001", "The Temple", "Midgaard", &[("N", "Midgaard#3054"), ("D", "Midgaard#3005")])).unwrap());
    }

    #[test]
    fn a_room_without_an_id_takes_the_player_off_the_map() {
        let mut m = square();
        assert!(!m.apply_room_info(&info("", "Limbo", "", &[])).unwrap());
        assert!(m.current().is_none());
        assert!(m.detail().is_none());
        assert!(m.view(5, 5).is_empty());
    }

    #[test]
    fn rejects_broken_json() {
        let mut m = Map::default();
        assert!(m.apply_room_info("{not json").is_err());
        assert!(m.apply_room_exits("[").is_err());
    }

    #[test]
    fn strips_color_codes_from_names() {
        assert_eq!(plain("^wThe ^#123Temple^N of ^^Mota\u{1b}[0m"), "The Temple of ^Mota");
        assert_eq!(plain("two\r\nlines"), "two lines");
    }

    #[test]
    fn a_hidden_exit_seen_before_is_kept() {
        let mut m = Map::default();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("N", "Z#9")])).unwrap();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2")])).unwrap();
        assert_eq!(m.detail().unwrap().exits.len(), 2);
    }

    #[test]
    fn room_exits_records_doors_and_locks() {
        let mut m = square();
        assert!(m.apply_room_exits(r#"{"exits":{"N":{"id":5,"door":"door","open":false,"locked":true,"move":"normal"},"E":{"id":6,"door":"","open":true,"locked":false,"move":"normal"}},"idexits":{}}"#).unwrap());
        let n = m.exit("N").unwrap();
        assert!(n.door && !n.open && n.locked);
        assert!(!m.exit("E").unwrap().door);
    }

    #[test]
    fn lays_out_the_current_level() {
        let mut m = square();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("S", "Z#4"), ("D", "Z#5")])).unwrap();
        let cells = m.view(5, 5);
        let at = |name: &str| cells.iter().find(|c| c.name == name).map(|c| (c.x, c.y));
        assert_eq!(at("A"), Some((0, 0)));
        assert_eq!(at("B"), Some((1, 0)));
        assert_eq!(at("C"), Some((1, 1)));
        assert_eq!(at("D"), Some((0, 1)));
        // The cellar is on another level: not drawn, but A says it goes down.
        assert_eq!(cells.len(), 4);
        let a = cells.iter().find(|c| c.current).unwrap();
        assert!(a.down && !a.up);
        assert_eq!(a.exits, ["E", "S"]);
        // Each room carries its terrain, for its color.
        assert!(cells.iter().all(|c| c.terrain == "city"));
    }

    #[test]
    fn a_taken_cell_is_not_reused() {
        // Two rooms both claim the cell east of A (B directly, X by going
        // north then southeast). The nearer one wins; the other is left out.
        let mut m = Map::default();
        m.apply_room_info(&info("Z#3", "N", "Zone", &[("S", "Z#1"), ("SE", "Z#9")])).unwrap();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("N", "Z#3")])).unwrap();
        let cells = m.view(5, 5);
        assert_eq!(cells.iter().filter(|c| (c.x, c.y) == (1, 0)).count(), 1);
        assert_eq!(cells.iter().find(|c| (c.x, c.y) == (1, 0)).unwrap().id, "Z#2");
    }

    #[test]
    fn stays_inside_the_radius_and_does_not_explore_other_areas() {
        let mut m = Map::default();
        m.apply_room_info(&info("Y#1", "Far", "Other", &[("W", "Z#2"), ("E", "Y#2")])).unwrap();
        m.apply_room_info(&info("Z#2", "B", "Zone", &[("W", "Z#1"), ("E", "Y#1")])).unwrap();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2")])).unwrap();
        let cells = m.view(5, 5);
        let far = cells.iter().find(|c| c.name == "Far").unwrap();
        assert!(far.other_zone);
        assert!(!cells.iter().any(|c| c.id == "Y#2"));
        assert_eq!(m.view(1, 1).len(), 2);
    }

    #[test]
    fn routes_the_short_way_and_says_it_in_words() {
        let mut m = square();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("S", "Z#4"), ("D", "Z#5")])).unwrap();
        let route = m.route("Z#3").unwrap();
        assert_eq!(route.len(), 2);
        assert_eq!(directions(&route), "east, south");
        assert_eq!(m.route("Z#1").unwrap(), Vec::<Step>::new());
        assert!(m.route("Nowhere#1").is_none());
        let steps: Vec<Step> = ["N", "N", "N", "E", "U", "U"].iter().map(|d| Step { dir: d.to_string(), to: String::new() }).collect();
        assert_eq!(directions(&steps), "3 north, east, 2 up");
    }

    #[test]
    fn routes_avoid_locked_doors_and_unvisited_rooms() {
        let mut m = square();
        m.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("S", "Z#4"), ("D", "Z#5")])).unwrap();
        m.apply_room_exits(r#"{"exits":{"E":{"id":1,"door":"door","open":false,"locked":true,"move":"normal"}}}"#).unwrap();
        assert_eq!(directions(&m.route("Z#2").unwrap()), "south, east, north");
        // The cellar is a stub: it can be walked to, but not through.
        assert_eq!(directions(&m.route("Z#5").unwrap()), "down");
    }

    #[test]
    fn finds_rooms_and_landmarks() {
        let mut m = square();
        assert!(m.set_landmark("Z#3", "Bank"));
        assert!(!m.set_landmark("Nope#1", "x"));
        assert_eq!(m.find("", 10).iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["Z#3"]);
        assert_eq!(m.find("bank", 10)[0].id, "Z#3");
        assert_eq!(m.find("bank", 10)[0].terrain, "city");
        assert_eq!(m.find("zone", 10).len(), 4);
        assert_eq!(m.find("zone", 2).len(), 2);
        assert_eq!(m.room_name("Z#3"), "Bank");
    }

    #[test]
    fn survives_a_save_and_load() {
        let mut m = square();
        m.set_landmark("Z#2", "Inn");
        let mut back = Map::from_json(&m.to_json()).unwrap();
        assert!(back.current().is_none());
        back.apply_room_info(&info("Z#1", "A", "Zone", &[("E", "Z#2"), ("S", "Z#4"), ("D", "Z#5")])).unwrap();
        assert_eq!(back.view(5, 5).len(), 4);
        assert_eq!(back.find("inn", 5).len(), 1);
        assert!(Map::from_json("nonsense").is_err());
        back.clear();
        assert_eq!(back.snapshot(5, 5).rooms_known, 0);
    }
}

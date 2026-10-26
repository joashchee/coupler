//! **The Bestiary and the Items**: every creature and every item the
//! player comes across in a world, and what's been learned about each,
//! kept for the player to look up and for Coupler to use (a creature's
//! cry from coffeemud.sf2). `session.rs` feeds it GMCP, each line sent
//! and each read's lines, and opens it at `bestiary.sqlite` in app data.
//!
//! Shared with the web build. What's kept is `Bestiary`, from each
//! build's own `bestiary_store.rs`: SQLite on the desktop, memory kept as
//! JSON in the browser on the web; the tests here run against both.
//!
//! CoffeeMUD tells a player little about an NPC outright: no GMCP for
//! one but its name (`room.mobiles`), and LOOK says race and level for
//! players only (`CommonMsgs.handleBeingMobLookedAt`). So each fact is
//! gleaned, silently, from what the game shows anyway:
//!
//! - **Seen**: `room.mobiles` names the NPCs in the room, `room.info`'s
//!   `zone` the area; `room.items.list` the items lying there.
//! - **Race**: read from the name ("an orc" is an Orc, the longest race
//!   name in it, `race_in_name`) until a long look says it: a creature
//!   stronger or weaker than most of its kind "appears stronger than the
//!   average Orc" (`relativeCharStatTest`). The race's category picks
//!   its cry from coffeemud.sf2 (`Creature`).
//! - **A look** (LOOK or a long look at one, typed by the player): its
//!   height, weight and sex, its body (a long look: legs, arms, wings),
//!   its description and what it wears, each worn item an item seen.
//! - **Level**: CONSIDER (and a long look, which considers too) compares
//!   it with the player's level from `char.status`, by stock
//!   coffeemud.ini's EXPRATE of 6 (`Consider.doConsider`): "is your
//!   equal" is the same level, "is almost your equal" within 2, and so
//!   on. Each consider narrows the range in that area.
//! - **Hit points**: MSDP's `OPPONENT_HEALTH_MAX` in a fight
//!   (`combat.rs`), exact, in that area.
//! - **Alignment**: "(glowing red)" or "(glowing blue)" after its line in
//!   the room, with the player's DETECT EVIL or DETECT GOOD on.
//! - **Abilities**: a line that starts with its name and is one an
//!   ability prints when it's used (`abilities.tsv`, made from
//!   CoffeeMUD's source by `scripts/coffeemud-bestiary.py`). The game
//!   names no NPC's class; the kind of ability it uses most (spells,
//!   prayers, chants) says what it seems to be.
//! - **Deaths and drops**: "An orc is DEAD!!!", and "You get a dagger
//!   from the body of an orc." (`Get.java`, `StdRace.getCorpseContainer`)
//!   is a drop of that creature's.
//! - **Butchering**: "You manage to skin and chop up the body of a
//!   wolf." (`Butchering.unInvoke`), then the room's list sent again
//!   with what it gave, each named by the race's own resources
//!   (`myResources`: "a pound of wolf meat", "some wolf hair"), so the
//!   race is read from those, as the game says it.
//! - **Bodies**: a creature's body has no race to show (`Corpse.java`),
//!   but EXAMINE at one says the creature's own level (exact at 18
//!   intelligence, a range below) and weight, and any look at one lists
//!   what it carried ("the body of an orc contains:").
//! - **Looking unobtrusively** (the player's choice, off at first):
//!   Coupler looks at each creature it has never seen looked at, once,
//!   with `LOOK <it> UNOBTRUSIVELY` (at a creature, `Look.java` still
//!   tells the room; only a look around is quiet), and hides the
//!   answer, every line to the prompt (`auto_look`, `hidden`).
//! - **Items**: EXAMINE at one says its level, its weight and what it's
//!   made of ("A dagger is a level 3 item, and weighs 2 pounds.  It is
//!   mostly made of iron.", `getExamineItemString`), and the lines
//!   before that, its description.
//!
//! Creatures and items are keyed by their names (letters and digits,
//! lowercased, as the cast keys them): two creatures of one name are one
//! entry, told apart by area. One per world, for every character played
//! there. It's the game's world and the player's play, so like the
//! journal it stays on this machine (CLAUDE.md rule 1).

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::ansi::{caret_line, Line};
use crate::clock::Instant;
pub use crate::bestiary_store::Bestiary;

/// Coupler's own pair a creature's cry plays on: `"canine fights"`.
pub const CREATURE_KEY: &str = "coupler.creature";

/// How long a look or an examine waits for its answer.
const WAITS: Duration = Duration::from_secs(4);

/// Stock coffeemud.ini's EXPRATE: CONSIDER's words for a level apart.
const EXPRATE: i64 = 6;

// ---- Races ----

/// A race CoffeeMUD has, as `races.tsv` lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Race {
    pub name: &'static str,
    pub category: &'static str,
}

/// Race names too much like everyday words to be read from a creature's
/// name ("the stone carver", "a shadow thief"). A long look still says them.
const NOT_IN_NAMES: &[&str] = &[
    "Animal", "Humanoid", "Insect", "Rodent", "Equine", "Mustelid", "Pachyderm", "Stone", "Solar", "Plant", "Flower", "Doll", "Dummy", "Spirit",
    "Shadow", "PlayerRace", "School Monster", "Clockwork", "Vine",
];

fn races() -> &'static [Race] {
    static RACES: OnceLock<Vec<Race>> = OnceLock::new();
    RACES.get_or_init(|| {
        include_str!("races.tsv")
            .lines()
            .skip(1)
            .filter_map(|line| {
                let (name, category) = line.split_once('\t')?;
                Some(Race { name, category })
            })
            .collect()
    })
}

/// A name's words, lowercased, a hyphen a space: "Man-Scorpion" is "man scorpion".
fn words(name: &str) -> Vec<String> {
    name.to_lowercase()
        .replace('-', " ")
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

/// The race a creature's name says: the longest race name in its words,
/// the first of those.
pub fn race_in_name(name: &str) -> Option<Race> {
    race_among(name, |r| !NOT_IN_NAMES.contains(&r.name))
}

/// The race a butchered body's resource names ("a pound of wolf meat",
/// `myResources`, the race's own name in lower case): every race counts,
/// since the game put it there.
fn race_in_material(name: &str) -> Option<Race> {
    race_among(name, |_| true)
}

fn race_among(name: &str, eligible: impl Fn(&Race) -> bool) -> Option<Race> {
    let said = words(name);
    let mut best: Option<(usize, usize, Race)> = None;
    for race in races().iter().filter(|r| eligible(r)) {
        let own = words(race.name);
        let Some(at) = said.windows(own.len()).position(|w| w == own.as_slice()) else { continue };
        let better = match best {
            None => true,
            Some((len, start, _)) => own.len() > len || (own.len() == len && at < start),
        };
        if better {
            best = Some((own.len(), at, *race));
        }
    }
    best.map(|(_, _, race)| race)
}

/// A race by its name, any case.
pub fn race_named(name: &str) -> Option<Race> {
    races().iter().find(|r| r.name.eq_ignore_ascii_case(name.trim())).copied()
}

// ---- Cries ----

/// The creatures coffeemud.sf2 has a voice for (program 0, keys 48 to
/// 59), each with three: idle, aggressive and pain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Creature {
    Humanoid,
    Goblinoid,
    GiantKin,
    Canine,
    Beast,
    Avian,
    Reptile,
    Insect,
    Undead,
    Dragon,
    Elemental,
    Construct,
}

impl Creature {
    pub const ALL: [Creature; 12] = [
        Creature::Humanoid,
        Creature::Goblinoid,
        Creature::GiantKin,
        Creature::Canine,
        Creature::Beast,
        Creature::Avian,
        Creature::Reptile,
        Creature::Insect,
        Creature::Undead,
        Creature::Dragon,
        Creature::Elemental,
        Creature::Construct,
    ];

    /// The cry for a racial category, if the bank has one that suits.
    /// Fish, plants, rodents and the rest make no sound of their own.
    pub fn of(category: &str) -> Option<Creature> {
        Some(match category {
            "Humanoid" | "Elf" | "Dwarf" | "Halfling" | "Gnome" | "Illithid" | "Celestial" | "Fairy-kin" | "Primate" => Creature::Humanoid,
            "Goblinoid" => Creature::Goblinoid,
            "Giant-kin" | "Demon" | "Devil" => Creature::GiantKin,
            "Canine" => Creature::Canine,
            "Feline" | "Ursine" | "Bovine" | "Equine" | "Porcine" | "Ovine" | "Pachyderm" | "Sea Mammal" => Creature::Beast,
            "Avian" | "Pteropine" => Creature::Avian,
            "Reptile" | "Ophidian" | "Amphibian" => Creature::Reptile,
            "Insect" | "Arachnid" | "Worm" | "Slime" => Creature::Insect,
            "Undead" => Creature::Undead,
            "Dragon" => Creature::Dragon,
            "Elemental" | "Mephit" => Creature::Elemental,
            "Stone Golem" | "Wood Golem" | "Metal Golem" => Creature::Construct,
            _ => return None,
        })
    }

    /// The cry for a race: its category's, but where CoffeeMUD files a
    /// race by its looks rather than its voice (the Orc is "Porcine").
    pub fn of_race(race: Race) -> Option<Creature> {
        match race.name {
            "Orc" | "Kobold" => Some(Creature::Goblinoid),
            "Tiefling" | "Merfolk" | "Draconian" | "Selkie" => Some(Creature::Humanoid),
            "Minotaur" => Some(Creature::GiantKin),
            "Treant" => Some(Creature::Construct),
            _ => Creature::of(race.category),
        }
    }

    /// Its word in the pair's value.
    pub fn name(self) -> &'static str {
        match self {
            Creature::Humanoid => "humanoid",
            Creature::Goblinoid => "goblinoid",
            Creature::GiantKin => "giant-kin",
            Creature::Canine => "canine",
            Creature::Beast => "beast",
            Creature::Avian => "avian",
            Creature::Reptile => "reptile",
            Creature::Insect => "insect",
            Creature::Undead => "undead",
            Creature::Dragon => "dragon",
            Creature::Elemental => "elemental",
            Creature::Construct => "construct",
        }
    }

    /// Its sound's name in coffeemud.sf2 (`soundbank.tsv`).
    pub fn slot(self) -> &'static str {
        match self {
            Creature::Humanoid => "Humanoid",
            Creature::Goblinoid => "Goblinoid",
            Creature::GiantKin => "Giant-kin",
            Creature::Canine => "Canine",
            Creature::Beast => "Feline",
            Creature::Avian => "Avian",
            Creature::Reptile => "Reptile",
            Creature::Insect => "Insect",
            Creature::Undead => "Undead",
            Creature::Dragon => "Dragon",
            Creature::Elemental => "Elemental",
            Creature::Construct => "Construct",
        }
    }
}

/// When a creature cries: coming into the room (idle), a fight with the
/// player starting (aggressive), dying (pain).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cry {
    Comes,
    Fights,
    Dies,
}

impl Cry {
    pub const ALL: [Cry; 3] = [Cry::Comes, Cry::Fights, Cry::Dies];

    fn word(self) -> &'static str {
        match self {
            Cry::Comes => "comes",
            Cry::Fights => "fights",
            Cry::Dies => "dies",
        }
    }
}

/// The pair's value for a cry: `"canine fights"`.
pub fn cry_value(creature: Creature, cry: Cry) -> String {
    format!("{} {}", creature.name(), cry.word())
}

/// The other way: the creature and the cry a value names.
pub fn cry_of(value: &str) -> Option<(Creature, Cry)> {
    let (name, word) = value.rsplit_once(' ')?;
    let creature = Creature::ALL.into_iter().find(|c| c.name() == name)?;
    let cry = Cry::ALL.into_iter().find(|c| c.word() == word)?;
    Some((creature, cry))
}

/// Every cry's value, listed in the hooks from the start.
pub fn cry_values() -> Vec<String> {
    Creature::ALL.iter().flat_map(|c| Cry::ALL.map(|cry| cry_value(*c, cry))).collect()
}

// ---- Abilities ----

/// A line an ability prints when it's used, the one using it left off:
/// `*` stands for a name, a pronoun or a value.
struct AbilityLine {
    ability: &'static str,
    kind: &'static str,
    /// Lowercased, split at each `*`.
    parts: Vec<String>,
}

/// The lines, and which to try for a line's first word.
type AbilityLines = (Vec<AbilityLine>, HashMap<String, Vec<usize>>);

fn ability_lines() -> &'static AbilityLines {
    static LINES: OnceLock<AbilityLines> = OnceLock::new();
    LINES.get_or_init(|| {
        let lines: Vec<AbilityLine> = include_str!("abilities.tsv")
            .lines()
            .skip(1)
            .filter_map(|row| {
                let mut cols = row.splitn(3, '\t');
                let (ability, kind, line) = (cols.next()?, cols.next()?, cols.next()?);
                Some(AbilityLine { ability, kind, parts: line.to_lowercase().split('*').map(str::to_string).collect() })
            })
            .collect();
        // By the first word, so a line is tried against a few; those
        // starting with a wildcard are under "".
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, line) in lines.iter().enumerate() {
            let first = line.parts[0].split_whitespace().next().unwrap_or("").to_string();
            index.entry(first).or_default().push(i);
        }
        (lines, index)
    })
}

/// Whether `text` is `parts` with something (at least one character) in
/// each gap.
fn glob(parts: &[String], text: &str) -> bool {
    if let [only] = parts {
        return text == only;
    }
    let [first, middle @ .., last] = parts else { return false };
    let Some(mut rest) = text.strip_prefix(first.as_str()) else { return false };
    let one = |s: &str| s.chars().next().map(char::len_utf8);
    for part in middle {
        let Some(skip) = one(rest) else { return false };
        let Some(at) = rest[skip..].find(part.as_str()) else { return false };
        rest = &rest[skip + at + part.len()..];
    }
    rest.len() > last.len() && rest.ends_with(last.as_str()) && rest.is_char_boundary(rest.len() - last.len())
}

/// The ability a line shows being used, from what follows the one using it.
pub fn ability_in(rest: &str) -> Option<(&'static str, &'static str)> {
    let rest = squeeze(rest).to_lowercase();
    let rest = if rest.starts_with('\'') { rest } else { format!(" {rest}") };
    let (lines, index) = ability_lines();
    let first = rest.split_whitespace().next().unwrap_or("");
    let tried = index.get(first).into_iter().chain(index.get("")).flatten();
    for &i in tried {
        let line = &lines[i];
        if glob(&line.parts, &rest) {
            return Some((line.ability, line.kind));
        }
    }
    None
}

/// What a creature seems to be, from the kind of ability it uses most.
pub fn seems(kinds: &[(String, u32)]) -> Option<&'static str> {
    let mut best: Option<(&'static str, u32)> = None;
    for (kind, count) in kinds {
        let class = match kind.as_str() {
            "spell" => "Mage",
            "prayer" => "Cleric",
            "chant" => "Druid",
            "song" => "Bard",
            "thief skill" => "Thief",
            "fighter skill" => "Fighter",
            "paladin skill" => "Paladin",
            "ranger skill" => "Ranger",
            _ => continue,
        };
        if best.is_none_or(|(_, n)| *count > n) {
            best = Some((class, *count));
        }
    }
    best.map(|(class, _)| class)
}

// ---- Reading the game ----

/// Spaces collapsed, ends trimmed.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Something learned, about a creature (by its name as shown) or an item.
#[derive(Debug, Clone, PartialEq)]
pub enum Fact {
    /// Come into the room, or the player came to it.
    Seen { name: String },
    /// An item lying in the room.
    ItemSeen { name: String, container: bool },
    /// Its race, as the game says it.
    Race { name: String, race: String },
    /// A look: "He is 70 inches tall and weighs 180 pounds."
    Size { name: String, sex: String, height: u32, weight: u32 },
    /// A long look: "It has 4 leg(s), no arms and a tail."
    Body { name: String, body: String },
    /// A long look: "He appears stronger than the average Orc."
    Stat { name: String, stat: String },
    Description { name: String, text: String },
    /// Worn or held, where: "(worn on torso) a leather jerkin".
    Wears { name: String, place: String, item: String },
    /// Its level, between these.
    Level { name: String, low: i64, high: i64 },
    Health { name: String, max: i64 },
    /// Evil or good.
    Alignment { name: String, alignment: &'static str },
    Ability { name: String, ability: String, kind: String },
    Died { name: String },
    /// The player took it from the creature's body.
    Dropped { name: String, item: String },
    /// Seen in its body, so many of it.
    Carried { name: String, item: String, count: u32 },
    /// What butchering its body gave: "a pound of wolf meat".
    Butchered { name: String, item: String },
    /// Its weight, exactly: its body's.
    Weight { name: String, weight: u32 },
    /// What the viewer's senses or the item's own state add after its
    /// name: "glowing", "magical aura", "invisible", "floating".
    ItemAura { name: String, aura: String },
    /// An item examined.
    Item { name: String, level: Option<(i64, i64)>, weight: Option<(i64, i64)>, material: Option<String>, description: Option<String> },
}

/// What a look at a creature is waiting for next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    /// Its size, body and stats, then the health line.
    Top,
    /// The blank line after the health line.
    BeforeDescription,
    Description,
    /// The blank line after it, then "He is wearing:".
    AfterDescription,
    Wearing,
}

#[derive(Debug, Clone)]
struct Looking {
    name: String,
    asked: Instant,
    part: Part,
    description: Vec<String>,
    /// Coupler's own look: its answer isn't shown.
    hidden: bool,
}

#[derive(Debug, Clone)]
struct Examining {
    asked: Instant,
    lines: Vec<String>,
}

/// Reads GMCP and the game's lines for facts. Pure.
#[derive(Default)]
pub struct Reader {
    /// The area the player's in, from `room.info`'s `zone`.
    area: Option<String>,
    room: Option<String>,
    /// The NPCs in the room: name and the context name that targets it.
    /// Kept across a move: the game sends the list again only when it's
    /// changed (`CMProtocols`' room hashes), and a follower's is the same.
    mobiles: Vec<(String, String)>,
    /// The items lying in the room, the same way.
    items: Vec<String>,
    /// The player moved: everything in the next lists is newly seen.
    moved_mobiles: bool,
    moved_items: bool,
    players: Vec<String>,
    /// The player's level, from `char.status`.
    level: Option<i64>,
    me: String,
    /// The opponent's name, while fighting.
    opponent: Option<String>,
    looking: Option<Looking>,
    examining: Option<Examining>,
    /// A body's contents being listed: whose.
    contents: Option<String>,
    /// A body just butchered: whose, till the room's list brings what it gave.
    butchered: Option<String>,
    /// The last read's lines that were Coupler's own look's answer.
    hidden: Vec<usize>,
    /// Coupler's own look's answer began (when): every line is hidden
    /// till the game's prompt comes back (`prompted`), so nothing of it
    /// shows, read or not.
    gag: Option<Instant>,
}

/// The words that look (`Look.java`) and long-look (`Examine.java`).
const LOOK: &[&str] = &["LOOK", "LOO", "LO", "L"];
const EXAMINE: &[&str] = &["EXAMINE", "EXAM", "EXA", "LONGLOOK", "LLOOK", "LL"];

/// The pronoun a line about a looked-at creature starts with.
fn pronoun(word: &str) -> Option<&'static str> {
    match word {
        "He" => Some("male"),
        "She" => Some("female"),
        "It" => Some("neuter"),
        _ => None,
    }
}

/// "12" or "10-14", as EXAMINE says a level or a weight.
fn range(s: &str) -> Option<(i64, i64)> {
    match s.split_once('-') {
        Some((a, b)) => Some((a.trim().parse().ok()?, b.trim().parse().ok()?)),
        None => s.trim().parse().ok().map(|n| (n, n)),
    }
}

/// CONSIDER's words, and how many levels from the player's each means
/// at stock EXPRATE (`Consider.doConsider`).
fn considered(words: &str) -> Option<(i64, i64)> {
    let x = EXPRATE;
    // "almost" is under half of EXPRATE, "somewhat superior" under 0.8 of it.
    let half = (x - 1) / 2;
    let most = (x * 4 - 1) / 5;
    Some(match words {
        "is your equal" => (0, 0),
        "is almost your equal" => (-half, half),
        "is somewhat inferior to you" => (-x, -(half + 1)),
        "is inferior to you" => (-x, -(half + 1)),
        "is vastly inferior to you" => (i64::MIN / 4, -(x + 1)),
        "is somewhat superior to you" => (half + 1, most),
        "is superior to you" => (most + 1, x),
        "is far superior to you" => (x + 1, i64::MAX / 4),
        _ => return None,
    })
}

const CONSIDERS: &[&str] = &[
    "is your equal",
    "is almost your equal",
    "is somewhat inferior to you",
    "is vastly inferior to you",
    "is inferior to you",
    "is somewhat superior to you",
    "is far superior to you",
    "is superior to you",
];

impl Reader {
    /// The connection or the character changed: nowhere, nothing waiting.
    pub fn clear(&mut self) {
        *self = Reader { level: self.level, ..Reader::default() };
    }

    pub fn area(&self) -> Option<&str> {
        self.area.as_deref()
    }

    /// The character's name, so they're never taken for a creature.
    pub fn character(&mut self, name: &str) {
        self.me = name.to_lowercase();
    }

    /// The fight's opponent, or None.
    pub fn opponent(&mut self, name: Option<&str>) {
        self.opponent = name.map(str::to_string);
    }

    /// A GMCP message. Facts it gives, and the name of someone who came
    /// into the room (`room.enter`, sent before the room's list has them),
    /// unless it's a player here or the player: for their cry, if the
    /// Bestiary knows them for a creature.
    pub fn gmcp(&mut self, package: &str, data: &str) -> (Vec<Fact>, Option<String>) {
        let package = package.to_ascii_lowercase();
        let value: Value = serde_json::from_str(data).unwrap_or(Value::Null);
        let mut facts = Vec::new();
        match package.as_str() {
            "room.info" => {
                self.area = value.get("zone").and_then(Value::as_str).filter(|z| !z.is_empty()).map(str::to_string);
                let room = value.get("id").and_then(Value::as_str).map(str::to_string);
                if room != self.room {
                    self.room = room;
                    self.moved_mobiles = true;
                    self.moved_items = true;
                    self.butchered = None;
                }
            }
            "room.mobiles" => {
                let now: Vec<(String, String)> = pairs(&value, "npcs");
                let moved = std::mem::take(&mut self.moved_mobiles);
                for (name, _) in &now {
                    if moved || !self.mobiles.iter().any(|(n, _)| n == name) {
                        facts.push(Fact::Seen { name: name.clone() });
                    }
                }
                self.mobiles = now;
            }
            "room.players" => self.players = pairs(&value, "pcs").into_iter().map(|(name, _)| name.to_lowercase()).collect(),
            "room.items.list" if value.get("location").and_then(Value::as_str) == Some("room") => {
                let moved = std::mem::take(&mut self.moved_items);
                let mut now = Vec::new();
                let mut fresh = Vec::new();
                for item in value.get("items").and_then(Value::as_array).into_iter().flatten() {
                    let Some(name) = item.get("name").and_then(Value::as_str) else { continue };
                    now.push(name.to_string());
                    if !self.items.iter().any(|i| i == name) {
                        fresh.push(name.to_string());
                    }
                    if corpse(name).is_some() || (!moved && self.items.iter().any(|i| i == name)) {
                        continue;
                    }
                    let container = item.get("attrib").and_then(Value::as_str).is_some_and(|a| a.contains('c'));
                    facts.push(Fact::ItemSeen { name: name.to_string(), container });
                }
                self.items = now;
                if let Some(body) = self.butchered.clone() {
                    let found = butchered(&body, &fresh);
                    if !found.is_empty() {
                        self.butchered = None;
                        facts.extend(found);
                    }
                }
            }
            "char.status" => {
                if let Some(level) = value.get("level").and_then(Value::as_i64) {
                    self.level = Some(level);
                }
            }
            "room.enter" => {
                let name = value.as_str().map(str::to_string);
                return (facts, name.filter(|n| n.to_lowercase() != self.me && !self.players.contains(&n.to_lowercase())));
            }
            _ => {}
        }
        (facts, None)
    }

    /// A line the player sent: a look or an examine at something.
    pub fn typed(&mut self, line: &str, now: Instant) {
        // Coupler's own look still waiting: the game answers it first, so
        // it stays hidden (the player's line is answered after its prompt).
        if self.looking.as_ref().is_some_and(|l| l.hidden && now.duration_since(l.asked) <= WAITS) {
            return;
        }
        self.looking = None;
        self.examining = None;
        self.gag = None;
        let mut words = line.split_whitespace();
        let Some(command) = words.next().map(str::to_uppercase) else { return };
        let mut rest: Vec<&str> = words.collect();
        if rest.first().is_some_and(|w| w.eq_ignore_ascii_case("at")) {
            rest.remove(0);
        }
        // Looking without the room seeing (`Look.java`).
        if rest.last().is_some_and(|w| w.eq_ignore_ascii_case("unobtrusively")) {
            rest.pop();
        }
        if rest.is_empty() || rest.first().is_some_and(|w| w.eq_ignore_ascii_case("in")) {
            return;
        }
        let long = EXAMINE.contains(&command.as_str());
        if !long && !LOOK.contains(&command.as_str()) {
            return;
        }
        match self.target(&rest.join(" ")) {
            Some(name) => self.looking = Some(Looking { name, asked: now, part: Part::Top, description: Vec::new(), hidden: false }),
            None if long => self.examining = Some(Examining { asked: now, lines: Vec::new() }),
            None => {}
        }
    }

    /// Coupler's own look at a creature here, unseen by the room: the line
    /// to send, its answer kept and hidden. None if it's gone, or a look
    /// is already waiting.
    pub fn auto_look(&mut self, name: &str, now: Instant) -> Option<String> {
        if self.looking.is_some() || self.examining.is_some() {
            return None;
        }
        let (_, context) = self.mobiles.iter().find(|(n, _)| n == name)?;
        // "an orc.2" targets as "orc.2": one word, nothing to quote.
        let target = context.split_whitespace().last()?.to_string();
        self.looking = Some(Looking { name: name.to_string(), asked: now, part: Part::Top, description: Vec::new(), hidden: true });
        Some(format!("look {target} unobtrusively"))
    }

    /// Whether a look or an examine is waiting for its answer.
    pub fn busy(&self) -> bool {
        self.looking.is_some() || self.examining.is_some()
    }

    /// The creatures in the room now.
    pub fn here(&self) -> impl Iterator<Item = &str> {
        self.mobiles.iter().map(|(n, _)| n.as_str())
    }

    /// The last read's lines (by their place in it) that were Coupler's
    /// own look's answer, not to be shown.
    pub fn hidden(&self) -> &[usize] {
        &self.hidden
    }

    /// The creature in the room a typed target means, as CoffeeMUD finds
    /// one: its context name ("orc.2"), else the nth whose name's words
    /// start with the target's.
    fn target(&self, target: &str) -> Option<String> {
        let target = target.to_lowercase();
        if let Some((name, _)) = self.mobiles.iter().find(|(_, context)| context.eq_ignore_ascii_case(&target)) {
            return Some(name.clone());
        }
        let (target, nth) = match target.rsplit_once('.') {
            Some((t, n)) if n.parse::<usize>().is_ok() => (t.to_string(), n.parse::<usize>().unwrap_or(1).max(1)),
            _ => (target.clone(), 1),
        };
        let wanted = words(&target);
        if wanted.is_empty() {
            return None;
        }
        self.mobiles
            .iter()
            .filter(|(name, _)| {
                let own = words(name);
                wanted.iter().all(|w| own.iter().any(|o| o.starts_with(w.as_str())))
            })
            .nth(nth - 1)
            .map(|(name, _)| name.clone())
    }

    /// Whether a name is a creature here (not a player, not the player).
    fn creature(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        lower != self.me && !self.players.contains(&lower) && (self.mobiles.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) || self.opponent.as_deref().is_some_and(|o| o.eq_ignore_ascii_case(name)))
    }

    /// The creature here a line begins with, and the rest of the line.
    fn begins_with<'a>(&self, line: &'a str) -> Option<(String, &'a str)> {
        let names = self.mobiles.iter().map(|(n, _)| n.as_str()).chain(self.opponent.as_deref());
        let mut best: Option<(&str, &str)> = None;
        for name in names {
            if name.is_empty() || line.len() <= name.len() || !line.is_char_boundary(name.len()) || !line[..name.len()].eq_ignore_ascii_case(name) {
                continue;
            }
            let rest = &line[name.len()..];
            if (rest.starts_with(' ') || rest.starts_with('\'')) && best.is_none_or(|(b, _)| name.len() > b.len()) {
                best = Some((name, rest));
            }
        }
        best.map(|(name, rest)| (name.to_string(), rest))
    }

    /// One read's lines, as printed (colors gone). Facts they give, and
    /// the creatures that died, for their cry.
    pub fn lines(&mut self, texts: &[String], now: Instant) -> Vec<Fact> {
        if self.looking.as_ref().is_some_and(|l| now.duration_since(l.asked) > WAITS) {
            self.end_look();
        }
        if self.examining.as_ref().is_some_and(|e| now.duration_since(e.asked) > WAITS) {
            self.examining = None;
        }
        if self.gag.is_some_and(|t| now.duration_since(t) > WAITS) {
            self.gag = None;
        }
        let mut facts = Vec::new();
        self.hidden.clear();
        for (i, text) in texts.iter().enumerate() {
            let line = squeeze(text);
            // Coupler's own look is sent only at the prompt with nothing
            // going on (`Who::gentle`), so the game's next lines are its
            // answer: all hidden from the first, read or not. Waiting for
            // a line the reader knows let the first ones through: the
            // health line is the race's (`healthText`), and its name
            // `name(viewer)`, not always room.mobiles' `Name()`.
            if self.looking.as_ref().is_some_and(|l| l.hidden) && self.gag.is_none() {
                self.gag = Some(now);
            }
            self.line(&line, text, &mut facts);
            if self.gag.is_some() {
                self.hidden.push(i);
            }
        }
        if let Some(look) = self.looking.as_mut() {
            if look.part == Part::Wearing && texts.last().is_some_and(|t| t.trim().is_empty()) {
                self.looking = None;
            }
        }
        // A body's list is over with the read.
        self.contents = None;
        facts
    }

    fn end_look(&mut self) {
        self.looking = None;
    }

    /// The game's prompt came back (the read ended on it): Coupler's own
    /// look was answered, so nothing more is hidden.
    pub fn prompted(&mut self) {
        if self.gag.take().is_some() && self.looking.as_ref().is_some_and(|l| l.hidden) {
            self.looking = None;
        }
    }

    /// One line. Returns whether it was part of a look's answer.
    fn line(&mut self, line: &str, raw: &str, facts: &mut Vec<Fact>) -> bool {
        if let Some(fact) = self.look_line(line, raw) {
            facts.extend(fact);
            return true;
        }
        // Coupler's own look at one who's gone: the game's no is hidden too.
        if self.looking.as_ref().is_some_and(|l| l.hidden) && line.starts_with("You don't see") {
            self.looking = None;
            return true;
        }
        if let Some(found) = self.examine_line(line) {
            facts.extend(found);
            return false;
        }
        // A body's contents: "the body of an orc contains:", then an item a
        // line ("(2) a dagger"), or all on one line with COMPRESS.
        if let Some(name) = self.contents.clone() {
            if line.is_empty() {
                self.contents = None;
            } else {
                facts.extend(line.split(", ").filter_map(listed).map(|(item, count)| Fact::Carried { name: name.clone(), item, count }));
                facts.extend(line.split(", ").flat_map(listed_auras));
            }
            return false;
        }
        if let Some((body, rest)) = line.split_once(" contains:") {
            if let Some(name) = corpse(body) {
                facts.extend(rest.split(", ").filter_map(listed).map(|(item, count)| Fact::Carried { name: name.clone(), item, count }));
                facts.extend(rest.split(", ").flat_map(listed_auras));
                self.contents = Some(name);
                return false;
            }
        }
        // CONSIDER, typed or a long look's.
        for words in CONSIDERS {
            let Some(at) = line.find(&format!(" {words}")) else { continue };
            let name = &line[..at];
            let name = match (pronoun(name), &self.looking) {
                (Some(_), Some(look)) => look.name.clone(),
                _ => name.to_string(),
            };
            if let (Some((low, high)), Some(level), true) = (considered(words), self.level, self.creature(&name)) {
                facts.push(Fact::Level { name, low: (level + low).max(1), high: level + high });
            }
            return false;
        }
        // A body butchered: what it gave comes in the room's next list.
        if let Some(body) = line.strip_prefix("You manage to skin and chop up ").and_then(|l| l.strip_suffix('.')) {
            if let Some(name) = corpse(body) {
                self.butchered = Some(name);
            }
            return false;
        }
        if let Some(name) = line.strip_suffix(" is DEAD!!!") {
            if self.creature(name) {
                facts.push(Fact::Died { name: name.to_string() });
            }
            return false;
        }
        if let Some(got) = line.strip_prefix("You get ").and_then(|l| l.strip_suffix('.')) {
            if let Some((item, from)) = got.rsplit_once(" from ") {
                if let Some(name) = corpse(from) {
                    facts.push(Fact::Dropped { name, item: coins(item) });
                }
            }
            return false;
        }
        // An item lying here, its auras after its line: "A sword lies here. (glowing)".
        if line.ends_with(')') {
            if let Some(item) = self.item_in(line) {
                let (_, auras) = blurbs(line);
                facts.extend(auras.into_iter().map(|aura| Fact::ItemAura { name: item.clone(), aura }));
                return false;
            }
        }
        for (aura, alignment) in [("(glowing red)", "evil"), ("(glowing blue)", "good")] {
            if line.contains(aura) {
                if let Some(name) = self.named_in(line) {
                    facts.push(Fact::Alignment { name, alignment });
                }
            }
        }
        if let Some((name, rest)) = self.begins_with(line) {
            if let Some((ability, kind)) = ability_in(rest) {
                facts.push(Fact::Ability { name, ability: ability.into(), kind: kind.into() });
            }
        }
        false
    }

    /// The item lying here a room line names (its name less the article:
    /// "rusty scimitar" in "A rusty scimitar lies here."), the longest such.
    fn item_in(&self, line: &str) -> Option<String> {
        let lower = line.to_lowercase();
        self.items
            .iter()
            .filter(|name| {
                let bare = name.to_lowercase();
                let bare = ["a ", "an ", "the ", "some "].iter().find_map(|a| bare.strip_prefix(a)).unwrap_or(&bare).to_string();
                bare.len() >= 3 && lower.contains(&bare)
            })
            .max_by_key(|name| name.len())
            .cloned()
    }

    /// The one creature here a room line is about: its name's last word
    /// (the noun: "orc" for "a big orc") in it, and no other's.
    fn named_in(&self, line: &str) -> Option<String> {
        let said = words(line);
        let mut found = self.mobiles.iter().filter(|(name, _)| words(name).last().is_some_and(|noun| said.contains(noun)));
        let (name, _) = found.next()?;
        found.next().is_none().then(|| name.clone())
    }

    /// A line of the answer to a look at a creature.
    fn look_line(&mut self, line: &str, raw: &str) -> Option<Vec<Fact>> {
        let look = self.looking.as_mut()?;
        let name = look.name.clone();
        let mut facts = Vec::new();
        let first = line.split(' ').next().unwrap_or("");
        match look.part {
            Part::Top => {
                if let (Some(sex), Some(rest)) = (pronoun(first), line.split_once(' ').map(|(_, r)| r)) {
                    if let Some(size) = rest.strip_prefix("is ").and_then(|r| r.strip_suffix(" pounds.")) {
                        let (height, weight) = size.split_once(" inches tall and weighs ")?;
                        facts.push(Fact::Size { name, sex: sex.into(), height: height.parse().ok()?, weight: weight.parse().ok()? });
                    } else if let Some(body) = rest.strip_prefix("has ").and_then(|r| r.strip_suffix('.')) {
                        facts.push(Fact::Body { name, body: body.to_string() });
                    } else {
                        let stat = rest.strip_prefix("appears ").and_then(|r| r.strip_suffix('.'))?;
                        let (how, race) = stat.split_once(" than the average ")?;
                        facts.push(Fact::Stat { name: name.clone(), stat: how.to_string() });
                        facts.push(Fact::Race { name, race: race.to_string() });
                    }
                    return Some(facts);
                }
                // The health line: "An orc is in perfect health."
                if line.len() > name.len() && line.is_char_boundary(name.len()) && line[..name.len()].eq_ignore_ascii_case(&name) {
                    look.part = Part::BeforeDescription;
                    return Some(facts);
                }
                None
            }
            Part::BeforeDescription => {
                if !line.is_empty() {
                    look.part = Part::Description;
                    look.description.push(line.to_string());
                }
                Some(facts)
            }
            Part::Description => {
                if line.is_empty() {
                    look.part = Part::AfterDescription;
                    let text = look.description.join(" ");
                    facts.push(Fact::Description { name, text });
                } else {
                    look.description.push(line.to_string());
                }
                Some(facts)
            }
            Part::AfterDescription => {
                if line.is_empty() {
                    return Some(facts);
                }
                if pronoun(first).is_some() && line.ends_with("is wearing:") {
                    look.part = Part::Wearing;
                    return Some(facts);
                }
                self.looking = None;
                None
            }
            Part::Wearing => {
                let worn = raw.trim_start().strip_prefix('(').and_then(|r| r.split_once(')'));
                match worn {
                    Some((place, item)) if !item.trim().is_empty() => {
                        let (item, auras) = blurbs(&squeeze(item));
                        facts.extend(auras.into_iter().map(|aura| Fact::ItemAura { name: item.clone(), aura }));
                        facts.push(Fact::Wears { name, place: squeeze(place), item });
                        Some(facts)
                    }
                    _ => {
                        self.looking = None;
                        None
                    }
                }
            }
        }
    }

    /// A line of the answer to an examine at an item: its description,
    /// then the line that says its level. A creature's body says the
    /// creature's level and weight (`getCorpseContainer` gives it both).
    fn examine_line(&mut self, line: &str) -> Option<Vec<Fact>> {
        let at = line.find(" is a level ")?;
        let name = &line[..at];
        let rest = &line[at + " is a level ".len()..];
        let (level, rest) = rest.split_once(" item, and weighs ")?;
        let (weight, rest) = rest.split_once(" pounds.")?;
        let description = self.examining.take().map(|e| squeeze(&e.lines.join(" "))).filter(|d| !d.is_empty() && !d.starts_with("You don't see anything special"));
        if let Some(creature) = corpse(name) {
            let mut facts = Vec::new();
            if let Some((low, high)) = range(level) {
                facts.push(Fact::Level { name: creature.clone(), low, high });
            }
            if let Some((weight, _)) = range(weight).filter(|(l, h)| l == h && *l > 0) {
                facts.push(Fact::Weight { name: creature, weight: weight as u32 });
            }
            return Some(facts);
        }
        let material = rest.split_once("It is mostly made of ").and_then(|(_, m)| m.split_once('.')).map(|(m, _)| m.trim().to_string());
        Some(vec![Fact::Item { name: name.to_string(), level: range(level), weight: range(weight), material, description }])
    }

    /// The lines of an examine's answer before its level line.
    pub fn examined(&mut self, texts: &[String]) {
        if let Some(e) = self.examining.as_mut() {
            for t in texts {
                let t = squeeze(t);
                if t.contains(" is a level ") {
                    break;
                }
                if !t.is_empty() {
                    e.lines.push(t);
                }
            }
        }
    }
}

/// `{"npcs":[{"an orc":"an orc.2"}]}` as (name, context name) pairs.
fn pairs(value: &Value, key: &str) -> Vec<(String, String)> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .flat_map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or(k).to_string())))
        .collect()
}

/// The creature a body is of: "the body of an orc" is an orc's. The game
/// writes a name's apostrophes as backquotes in it (`getCorpseContainer`).
/// What butchering `name`'s body gave, among the items new in the room:
/// those named for one race, the race most of them name, and that race.
fn butchered(name: &str, fresh: &[String]) -> Vec<Fact> {
    let mut counts: Vec<(Race, usize)> = Vec::new();
    for item in fresh {
        let Some(race) = race_in_material(item) else { continue };
        match counts.iter_mut().find(|(r, _)| r.name == race.name) {
            Some((_, n)) => *n += 1,
            None => counts.push((race, 1)),
        }
    }
    // The first named wins a tie: the list is in the order they fell.
    let Some((race, _)) = counts.iter().fold(None, |best: Option<&(Race, usize)>, c| if best.is_none_or(|b| c.1 > b.1) { Some(c) } else { best }) else { return Vec::new() };
    let mut facts = vec![Fact::Race { name: name.to_string(), race: race.name.to_string() }];
    facts.extend(fresh.iter().filter(|item| race_in_material(item).is_some_and(|r| r.name == race.name)).map(|item| Fact::Butchered { name: name.to_string(), item: item.clone() }));
    facts
}

fn corpse(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    ["the body of ", "the corpse of "].iter().find_map(|p| lower.starts_with(p).then(|| name[p.len()..].replace('`', "'")))
}

/// An item as a container lists it: "(2) a dagger", "( 12) an arrow",
/// "a sword (glowing)." as its name and how many.
fn listed(line: &str) -> Option<(String, u32)> {
    let line = line.trim().trim_end_matches('.');
    let (count, rest) = match line.strip_prefix('(').and_then(|r| r.split_once(')')) {
        Some((n, rest)) if n.trim().parse::<u32>().is_ok() => (n.trim().parse().unwrap_or(1), rest.trim()),
        _ => (1, line),
    };
    // What the viewer's senses add after it: "(glowing)", "(magical aura)".
    let (name, _) = blurbs(rest);
    (!name.is_empty()).then(|| (coins(&name), count))
}

/// The auras of an item a container lists, by its name.
fn listed_auras(line: &str) -> Vec<Fact> {
    let line = line.trim().trim_end_matches('.');
    let rest = match line.strip_prefix('(').and_then(|r| r.split_once(')')) {
        Some((n, rest)) if n.trim().parse::<u32>().is_ok() => rest.trim(),
        _ => line,
    };
    let (name, auras) = blurbs(rest);
    let name = coins(&name);
    auras.into_iter().map(|aura| Fact::ItemAura { name: name.clone(), aura }).collect()
}

/// A name and what's added after it in brackets (`Sense.getDispositionBlurbs`):
/// "a sword (glowing) (magical aura)" is "a sword", ["glowing", "magical aura"].
fn blurbs(text: &str) -> (String, Vec<String>) {
    let mut name = text.trim().trim_end_matches('.').trim_end();
    let mut found = Vec::new();
    while let Some(open) = name.rfind(" (").filter(|_| name.ends_with(')')) {
        let aura = squeeze(&name[open + 2..name.len() - 1]).to_lowercase();
        if !aura.is_empty() && !aura.chars().all(|c| c.is_ascii_digit()) && aura.len() <= 30 {
            found.insert(0, aura);
        }
        name = name[..open].trim_end();
    }
    (name.to_string(), found)
}

/// Coins by their kind, not how many: "12 gold coins" is "gold coins",
/// "5 Copper Crowns" "Copper Crowns" (every currency: `loot::is_money`).
fn coins(item: &str) -> String {
    let lower = item.to_lowercase();
    if (lower.contains("coin") || crate::loot::is_money(item)) && item.chars().any(|c| c.is_ascii_digit()) {
        squeeze(&item.chars().filter(|c| !c.is_ascii_digit()).collect::<String>().replace(" of  ", " of "))
    } else {
        item.to_string()
    }
}

/// What an item is kept as: a bundle by what it holds, not its weight
/// ("a 25# oak bundle" is "an oak bundle", "a 12# pool of water" "a pool
/// of water": `RawCMaterial.adjustResourceName`), the rest as the game
/// names it, colors and all.
pub fn item_name(name: &str) -> String {
    let plain = crate::mapper::plain(name);
    let mut words = plain.split(' ');
    let (Some(article), Some(weight)) = (words.next(), words.next()) else { return name.trim().to_string() };
    let rest: Vec<&str> = words.collect();
    let weighed = weight.strip_suffix('#').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    let bundle = rest.last() == Some(&"bundle") || rest.first() == Some(&"pool");
    if !matches!(article.to_lowercase().as_str(), "a" | "an") || !weighed || !bundle || rest.len() < 2 {
        return name.trim().to_string();
    }
    let rest = rest.join(" ");
    let an = rest.starts_with(|c: char| "aeiouAEIOU".contains(c));
    format!("{} {rest}", if an { "an" } else { "a" })
}

/// An item's key: its kept name's letters and digits, its colors left out,
/// so the name GMCP sends (`^ga green potion^?`) and the one a line
/// prints are the same item.
pub fn item_key(name: &str) -> String {
    crate::senses::letters_and_digits(&crate::mapper::plain(&item_name(name)))
}

// ---- What's kept (bestiary_store.rs) ----

/// A creature in the list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobRow {
    pub key: String,
    /// The name in plain words; `styled` is it in the game's colors.
    pub name: String,
    pub styled: Line,
    pub race: Option<String>,
    /// The areas it's been seen in.
    pub areas: Vec<String>,
    pub deaths: u32,
    pub last_seen: i64,
}

/// What's known of a creature in one area.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobArea {
    pub area: String,
    pub seen: u32,
    pub last_seen: i64,
    pub level_low: Option<i64>,
    pub level_high: Option<i64>,
    pub max_hp: Option<i64>,
    pub alignment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counted {
    pub name: String,
    /// A kind (an ability's), or a place (where an item's worn).
    pub what: Option<String>,
    pub count: u32,
}

/// Everything known of a creature.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobEntry {
    pub key: String,
    pub name: String,
    pub race: Option<String>,
    /// The game said it (a long look), rather than its name.
    pub race_seen: bool,
    pub category: Option<String>,
    /// Its cry in coffeemud.sf2, by `Creature::name`.
    pub creature: Option<&'static str>,
    pub sex: Option<String>,
    pub height: Option<u32>,
    pub weight: Option<u32>,
    pub body: Option<String>,
    pub stats: Vec<String>,
    pub description: Option<String>,
    /// What it seems to be from its abilities: "Mage".
    pub seems: Option<&'static str>,
    pub first_seen: i64,
    pub last_seen: i64,
    pub deaths: u32,
    pub areas: Vec<MobArea>,
    pub abilities: Vec<Counted>,
    pub drops: Vec<Counted>,
    pub wears: Vec<Counted>,
    /// Seen in its body, the most at once.
    pub carried: Vec<Counted>,
    /// What butchering its body gave.
    pub butchered: Vec<Counted>,
}

/// An item in the list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRow {
    pub key: String,
    /// The name in plain words; `styled` is it in the game's colors.
    pub name: String,
    pub styled: Line,
    pub level_low: Option<i64>,
    pub level_high: Option<i64>,
    pub areas: Vec<String>,
    pub last_seen: i64,
}

/// Everything known of an item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEntry {
    pub key: String,
    pub name: String,
    pub level_low: Option<i64>,
    pub level_high: Option<i64>,
    pub weight_low: Option<i64>,
    pub weight_high: Option<i64>,
    pub material: Option<String>,
    pub description: Option<String>,
    pub container: bool,
    pub first_seen: i64,
    pub last_seen: i64,
    pub areas: Vec<Counted>,
    /// The creatures it was taken from, and how often.
    pub dropped_by: Vec<Counted>,
    /// The creatures seen wearing it, and where.
    pub worn_by: Vec<Counted>,
    /// The creatures whose bodies held it.
    pub carried_by: Vec<Counted>,
    /// The creatures whose bodies gave it when butchered.
    pub butchered_from: Vec<Counted>,
    /// What's been seen after its name: "glowing", "magical aura", "floating".
    pub auras: Vec<String>,
}

/// A name as it's kept (colors and all) in plain words, for the rows'
/// and entries' names.
fn plain(name: &mut String) {
    if name.contains(['^', '\u{1b}']) {
        *name = crate::mapper::plain(name);
    }
}

fn plain_all(rows: &mut [Counted]) {
    for row in rows {
        plain(&mut row.name);
    }
}

impl MobRow {
    /// As the list shows it: the name in its colors and in plain words.
    pub fn shown(mut self) -> Self {
        self.styled = caret_line(&self.name);
        plain(&mut self.name);
        self
    }
}

impl ItemRow {
    /// As the list shows it: the name in its colors and in plain words.
    pub fn shown(mut self) -> Self {
        self.styled = caret_line(&self.name);
        plain(&mut self.name);
        self
    }
}

impl MobEntry {
    /// Every name in it in plain words, for its facts and its picture.
    pub fn shown(mut self) -> Self {
        plain(&mut self.name);
        for list in [&mut self.drops, &mut self.wears, &mut self.carried, &mut self.butchered] {
            plain_all(list);
        }
        self
    }
}

impl ItemEntry {
    /// Every name in it in plain words, for its facts and its picture.
    pub fn shown(mut self) -> Self {
        plain(&mut self.name);
        for list in [&mut self.dropped_by, &mut self.worn_by, &mut self.carried_by, &mut self.butchered_from] {
            plain_all(list);
        }
        self
    }
}

/// Narrows a range by another: where they overlap, else the newer (a
/// creature whose level varies, or a level gained).
pub fn narrow(old: (Option<i64>, Option<i64>), new: (i64, i64)) -> (i64, i64) {
    match old {
        (Some(lo), Some(hi)) if lo.max(new.0) <= hi.min(new.1) => (lo.max(new.0), hi.min(new.1)),
        _ => new,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(reader: &mut Reader, lines: &[&str], now: Instant) -> Vec<Fact> {
        let texts: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        reader.examined(&texts);
        reader.lines(&texts, now)
    }

    fn in_room(reader: &mut Reader) {
        reader.gmcp("room.info", r#"{"id":"Midgaard#3001","zone":"Midgaard"}"#);
        reader.gmcp("char.status", r#"{"level":10}"#);
        let (facts, _) = reader.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc"},{"a large black dog":"dog"}]}"#);
        assert_eq!(facts, [Fact::Seen { name: "an orc".into() }, Fact::Seen { name: "a large black dog".into() }]);
    }

    #[test]
    fn races_from_names() {
        assert_eq!(race_in_name("an orc").map(|r| r.name), Some("Orc"));
        assert_eq!(race_in_name("the giant rat").map(|r| r.name), Some("Giant Rat"));
        assert_eq!(race_in_name("a hulking man-scorpion").map(|r| r.name), Some("Man-Scorpion"));
        assert_eq!(race_in_name("a human guard with a dog").map(|r| r.name), Some("Human"));
        assert_eq!(race_in_name("the stone carver"), None);
        assert_eq!(race_in_name("a city guard"), None);
        assert_eq!(race_named("black dragon"), None);
        assert_eq!(race_named("orc").map(|r| r.category), Some("Porcine"));
    }

    #[test]
    fn cries() {
        assert_eq!(Creature::of("Goblinoid"), Some(Creature::Goblinoid));
        assert_eq!(Creature::of("Fish"), None);
        assert_eq!(race_named("orc").and_then(Creature::of_race), Some(Creature::Goblinoid));
        assert_eq!(cry_value(Creature::Canine, Cry::Fights), "canine fights");
        assert_eq!(cry_of("giant-kin dies"), Some((Creature::GiantKin, Cry::Dies)));
        assert_eq!(cry_values().len(), 36);
        // Every category CoffeeMUD's races have is either a cry or meant to be none.
        for race in races() {
            let silent = ["Fish", "Vegetation", "Rodent", "Animal", "Mustelid"];
            assert!(Creature::of(race.category).is_some() || silent.contains(&race.category), "{} ({})", race.name, race.category);
        }
    }

    #[test]
    fn globs() {
        let parts = |p: &str| p.split('*').map(str::to_string).collect::<Vec<_>>();
        assert!(glob(&parts(" points at *, shooting forth a magic missile!"), " points at you, shooting forth a magic missile!"));
        assert!(!glob(&parts(" points at *, shooting forth a magic missile!"), " points at , shooting forth a magic missile!"));
        assert!(glob(&parts(" * for flaming healing."), " prays for flaming healing."));
        assert!(!glob(&parts(" chants."), " chants loudly."));
    }

    #[test]
    fn abilities_from_lines() {
        assert_eq!(ability_in(" points at you, shooting forth a magic missile!"), Some(("Magic Missile", "spell")));
        assert_eq!(ability_in(" screams a mighty RALLYING CRY!!"), Some(("Rally Cry", "fighter skill")));
        assert_eq!(ability_in(" hits you."), None);
        assert_eq!(seems(&[("spell".into(), 3), ("fighter skill".into(), 1)]), Some("Mage"));
        assert_eq!(seems(&[("power".into(), 3)]), None);
    }

    #[test]
    fn a_look_at_a_creature() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        let t = Instant::now();
        reader.typed("ll orc", t);
        let facts = read(
            &mut reader,
            &[
                "He is 70 inches tall and weighs 180 pounds.",
                "He has 2 leg(s), 2 arm(s).",
                "He appears much stronger than the average Orc.",
                "An orc is in perfect health.",
                "",
                "A big, ugly orc, with a nasty",
                "  look in his eye.",
                "",
                "He is wearing:",
                "(worn on torso)      a leather jerkin",
                "(wielded)            a rusty scimitar",
                "",
                "An orc is almost your equal and is the perfect match!",
            ],
            t,
        );
        let orc = || "an orc".to_string();
        assert_eq!(
            facts,
            [
                Fact::Size { name: orc(), sex: "male".into(), height: 70, weight: 180 },
                Fact::Body { name: orc(), body: "2 leg(s), 2 arm(s)".into() },
                Fact::Stat { name: orc(), stat: "much stronger".into() },
                Fact::Race { name: orc(), race: "Orc".into() },
                Fact::Description { name: orc(), text: "A big, ugly orc, with a nasty look in his eye.".into() },
                Fact::Wears { name: orc(), place: "worn on torso".into(), item: "a leather jerkin".into() },
                Fact::Wears { name: orc(), place: "wielded".into(), item: "a rusty scimitar".into() },
                Fact::Level { name: "An orc".into(), low: 8, high: 12 },
            ]
        );
        // The look waits a few seconds, then nothing more is taken for it.
        reader.typed("look dog", t);
        let later = t + Duration::from_secs(10);
        assert_eq!(read(&mut reader, &["She is 20 inches tall and weighs 30 pounds."], later), []);
    }

    #[test]
    fn levels_from_consider() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        let t = Instant::now();
        assert_eq!(read(&mut reader, &["An orc is your equal and is the perfect match!"], t), [Fact::Level { name: "An orc".into(), low: 10, high: 10 }]);
        assert_eq!(read(&mut reader, &["A large black dog is vastly inferior to you and is an easy kill."], t), [Fact::Level { name: "A large black dog".into(), low: 1, high: 3 }]);
        assert_eq!(read(&mut reader, &["Bob is your equal and is the perfect match!"], t), []);
    }

    #[test]
    fn deaths_drops_auras_and_abilities() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        let t = Instant::now();
        let facts = read(
            &mut reader,
            &[
                "An orc points at you, shooting forth a magic missile!",
                "A large black dog is here, wagging its tail. (glowing blue)",
                "An orc is DEAD!!!",
                "You get a rusty scimitar from the body of an orc.",
                "You get 12 gold coins from the body of an orc.",
                "Bob is DEAD!!!",
            ],
            t,
        );
        assert_eq!(
            facts,
            [
                Fact::Ability { name: "an orc".into(), ability: "Magic Missile".into(), kind: "spell".into() },
                Fact::Alignment { name: "a large black dog".into(), alignment: "good" },
                Fact::Died { name: "An orc".into() },
                Fact::Dropped { name: "an orc".into(), item: "a rusty scimitar".into() },
                Fact::Dropped { name: "an orc".into(), item: "gold coins".into() },
            ]
        );
    }

    #[test]
    fn butchering_shows_the_race_in_what_it_gives() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        let t = Instant::now();
        reader.gmcp("room.items.list", r#"{"location":"room","items":[{"id":"1","name":"the body of a large beast","attrib":"c"},{"id":"2","name":"a sword","attrib":""}]}"#);
        assert_eq!(read(&mut reader, &["You manage to skin and chop up the body of a large beast."], t), []);
        let after = r#"{"location":"room","items":[{"id":"2","name":"a sword","attrib":""},{"id":"3","name":"a pound of wolf meat","attrib":""},{"id":"4","name":"some wolf hair","attrib":""},{"id":"5","name":"a dagger","attrib":""}]}"#;
        let facts = reader.gmcp("room.items.list", after).0;
        assert!(facts.contains(&Fact::Race { name: "a large beast".into(), race: "Wolf".into() }), "{facts:?}");
        assert!(facts.contains(&Fact::Butchered { name: "a large beast".into(), item: "a pound of wolf meat".into() }));
        assert!(facts.contains(&Fact::Butchered { name: "a large beast".into(), item: "some wolf hair".into() }));
        assert!(!facts.contains(&Fact::Butchered { name: "a large beast".into(), item: "a dagger".into() }));
        // Once only: the next list is the room's again.
        let facts = reader.gmcp("room.items.list", r#"{"location":"room","items":[{"id":"6","name":"a pile of wolf bones","attrib":""}]}"#).0;
        assert!(!facts.iter().any(|f| matches!(f, Fact::Race { .. } | Fact::Butchered { .. })));
    }

    #[test]
    fn examining_an_item() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        let t = Instant::now();
        reader.typed("examine dagger", t);
        let facts = read(&mut reader, &["A sharp little blade.", "A dagger is a level 3 item, and weighs 2 pounds.  It is mostly made of iron."], t);
        assert_eq!(
            facts,
            [Fact::Item { name: "A dagger".into(), level: Some((3, 3)), weight: Some((2, 2)), material: Some("iron".into()), description: Some("A sharp little blade.".into()) }]
        );
        let facts = read(&mut reader, &["A shield is a level 10-14 item, and weighs 10-14 pounds.  "], t);
        assert_eq!(facts, [Fact::Item { name: "A shield".into(), level: Some((10, 14)), weight: Some((10, 14)), material: None, description: None }]);
    }

    #[test]
    fn seen_once_a_visit() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        // The list again with one more: only the new one's seen.
        let (facts, _) = reader.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc"},{"a large black dog":"dog"},{"a rat":"rat"}]}"#);
        assert_eq!(facts, [Fact::Seen { name: "a rat".into() }]);
        let items = r#"{"location":"room","items":[{"id":1,"name":"a sword"},{"id":2,"name":"the body of an orc","attrib":"c"}]}"#;
        assert_eq!(reader.gmcp("room.items.list", items).0, [Fact::ItemSeen { name: "a sword".into(), container: false }]);
        assert_eq!(reader.gmcp("room.items.list", items).0, []);
        // Moving with a follower: the game doesn't send the same list
        // again; it's still known, and seen in the next room once it is.
        reader.gmcp("room.info", r#"{"id":"Midgaard#3002","zone":"Midgaard"}"#);
        assert!(reader.creature("a rat"));
        assert_eq!(reader.gmcp("room.items.list", items).0, [Fact::ItemSeen { name: "a sword".into(), container: false }]);
        let (facts, _) = reader.gmcp("room.mobiles", r#"{"npcs":[{"a rat":"rat"}]}"#);
        assert_eq!(facts, [Fact::Seen { name: "a rat".into() }]);
    }

    #[test]
    fn item_auras() {
        assert_eq!(blurbs("a sword (glowing) (magical aura)."), ("a sword".to_string(), vec!["glowing".to_string(), "magical aura".to_string()]));
        let mut reader = Reader::default();
        in_room(&mut reader);
        reader.gmcp("room.items.list", r#"{"location":"room","items":[{"id":"1","name":"a rusty scimitar","attrib":""}]}"#);
        let t = Instant::now();
        let facts = read(&mut reader, &["A rusty scimitar lies here. (glowing) (floating)"], t);
        assert_eq!(facts, [Fact::ItemAura { name: "a rusty scimitar".into(), aura: "glowing".into() }, Fact::ItemAura { name: "a rusty scimitar".into(), aura: "floating".into() }]);
        // Worn: the item's name without them.
        reader.typed("look orc", t);
        let facts = read(&mut reader, &["An orc is in perfect health.", "", "Ugly.", "", "He is wearing:", "(worn on head)  an iron helm (invisible)"], t);
        assert!(facts.contains(&Fact::ItemAura { name: "an iron helm".into(), aura: "invisible".into() }));
        assert!(facts.contains(&Fact::Wears { name: "an orc".into(), place: "worn on head".into(), item: "an iron helm".into() }));
        let b = Bestiary::in_memory().unwrap();
        b.record("w", None, &Fact::ItemAura { name: "an iron helm".into(), aura: "invisible".into() }, 1).unwrap();
        b.record("w", None, &Fact::ItemAura { name: "an iron helm".into(), aura: "invisible".into() }, 2).unwrap();
        assert_eq!(b.item_entry("w", "anironhelm").unwrap().unwrap().auras, ["invisible"]);
    }

    #[test]
    fn a_body() {
        let mut reader = Reader::default();
        in_room(&mut reader);
        let t = Instant::now();
        reader.typed("examine body", t);
        let facts = read(
            &mut reader,
            &[
                "Bloody and bruised, obviously mistreated.",
                "The body of Bob`s pet is a level 12 item, and weighs 180 pounds.  It is mostly made of flesh.",
                "The body of Bob`s pet contains:",
                "( 2) a rusty dagger",
                "a leather jerkin (glowing).",
                "",
                "The body of a rat is a level 10-14 item, and weighs 0-3 pounds.  ",
            ],
            t,
        );
        let pet = || "Bob's pet".to_string();
        assert_eq!(
            facts,
            [
                Fact::Level { name: pet(), low: 12, high: 12 },
                Fact::Weight { name: pet(), weight: 180 },
                Fact::Carried { name: pet(), item: "a rusty dagger".into(), count: 2 },
                Fact::Carried { name: pet(), item: "a leather jerkin".into(), count: 1 },
                Fact::ItemAura { name: "a leather jerkin".into(), aura: "glowing".into() },
                Fact::Level { name: "a rat".into(), low: 10, high: 14 },
            ]
        );
        // COMPRESS: the list on the line.
        let facts = read(&mut reader, &["the body of an orc contains: a sword, 12 gold coins"], t);
        assert_eq!(facts, [Fact::Carried { name: "an orc".into(), item: "a sword".into(), count: 1 }, Fact::Carried { name: "an orc".into(), item: "gold coins".into(), count: 1 }]);
        // Every currency by its kind, not how many.
        assert_eq!(coins("5 Copper Crowns"), "Copper Crowns");
        assert_eq!(coins("3 conch(es)"), "conch(es)");
        assert_eq!(coins("a 2 handed sword"), "a 2 handed sword");
    }

    #[test]
    fn looking_unobtrusively() {
        let mut reader = Reader::default();
        reader.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc.2"}]}"#);
        let t = Instant::now();
        assert_eq!(reader.auto_look("an orc", t).as_deref(), Some("look orc.2 unobtrusively"));
        assert!(reader.busy());
        assert_eq!(reader.auto_look("an orc", t), None);
        let facts = read(&mut reader, &["", "He is 70 inches tall and weighs 180 pounds.", "An orc is in perfect health.", "", "Ugly.", "", "You see a door."], t);
        assert_eq!(facts.len(), 2);
        // The answer's lines are hidden, and all till the prompt.
        assert_eq!(reader.hidden(), [0, 1, 2, 3, 4, 5, 6]);
        assert!(!reader.busy());
        reader.prompted();
        // Gone before the look: the game's no is hidden too.
        reader.gmcp("room.mobiles", r#"{"npcs":[{"a rat":"rat"}]}"#);
        assert_eq!(reader.auto_look("a rat", t).as_deref(), Some("look rat unobtrusively"));
        read(&mut reader, &["You don't see 'rat' here."], t);
        assert_eq!(reader.hidden(), [0]);
        assert_eq!(reader.auto_look("a ghost", t), None);
    }

    #[test]
    fn coupler_s_own_look_is_gagged_till_the_prompt() {
        let mut reader = Reader::default();
        reader.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"orc"}]}"#);
        let t = Instant::now();
        reader.auto_look("an orc", t);
        // Equipment on one line (EQVIEW paragraph), which the reader
        // doesn't follow: hidden all the same, over two reads.
        read(&mut reader, &["", "An orc is in perfect health.", "", "Ugly."], t);
        assert_eq!(reader.hidden(), [0, 1, 2, 3]);
        read(&mut reader, &["", "He is wearing a helm, a club."], t);
        assert_eq!(reader.hidden(), [0, 1]);
        reader.prompted();
        read(&mut reader, &["A rat arrives."], t);
        assert!(reader.hidden().is_empty());
        assert!(!reader.busy());
    }

    #[test]
    fn coupler_s_own_look_is_hidden_from_its_first_line() {
        let mut reader = Reader::default();
        // GMCP's `Name()` in colors; the race's health line its own words.
        reader.gmcp("room.mobiles", r#"{"npcs":[{"^ga green slime^?":"slime"}]}"#);
        let t = Instant::now();
        reader.auto_look("^ga green slime^?", t);
        read(&mut reader, &["The slime quivers, unharmed.", "", "A puddle of goo."], t);
        assert_eq!(reader.hidden(), [0, 1, 2]);
        reader.prompted();
        read(&mut reader, &["A rat arrives."], t);
        assert!(reader.hidden().is_empty());
    }

    #[test]
    fn looked_at_once() {
        let b = Bestiary::in_memory().unwrap();
        assert!(b.wants_look("w", "an orc"));
        b.record("w", None, &Fact::Seen { name: "an orc".into() }, 1).unwrap();
        assert!(b.wants_look("w", "an orc"));
        b.looked("w", "an orc", 2).unwrap();
        assert!(!b.wants_look("w", "an orc"));
        b.record("w", None, &Fact::Description { name: "a rat".into(), text: "A rat.".into() }, 3).unwrap();
        assert!(!b.wants_look("w", "a rat"));
        b.record("w", None, &Fact::Carried { name: "a rat".into(), item: "a cheese".into(), count: 2 }, 4).unwrap();
        b.record("w", None, &Fact::Carried { name: "a rat".into(), item: "a cheese".into(), count: 1 }, 5).unwrap();
        let rat = b.mob_entry("w", "arat").unwrap().unwrap();
        assert_eq!(rat.carried, [Counted { name: "a cheese".into(), what: None, count: 2 }]);
        let cheese = b.item_entry("w", "acheese").unwrap().unwrap();
        assert_eq!(cheese.carried_by, [Counted { name: "a rat".into(), what: None, count: 2 }]);
        b.record("w", None, &Fact::Butchered { name: "a rat".into(), item: "some rat hair".into() }, 6).unwrap();
        b.record("w", None, &Fact::Butchered { name: "a rat".into(), item: "some rat hair".into() }, 7).unwrap();
        let rat = b.mob_entry("w", "arat").unwrap().unwrap();
        assert_eq!(rat.butchered, [Counted { name: "some rat hair".into(), what: None, count: 2 }]);
        let hair = b.item_entry("w", "somerathair").unwrap().unwrap();
        assert_eq!(hair.butchered_from, [Counted { name: "a rat".into(), what: None, count: 2 }]);
    }

    #[test]
    fn targets() {
        let mut reader = Reader::default();
        reader.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc"},{"an orc":"an orc.2"},{"a city guard":"guard"}]}"#);
        assert_eq!(reader.target("guard").as_deref(), Some("a city guard"));
        assert_eq!(reader.target("or").as_deref(), Some("an orc"));
        assert_eq!(reader.target("sword"), None);
        reader.typed("look at guard unobtrusively", Instant::now());
        assert_eq!(reader.looking.map(|l| l.name).as_deref(), Some("a city guard"));
    }

    #[test]
    fn bundles_by_what_they_hold() {
        assert_eq!(item_name("a 25# oak bundle"), "an oak bundle");
        assert_eq!(item_name("an 8# iron ore bundle"), "an iron ore bundle");
        assert_eq!(item_name("a 12# pool of water"), "a pool of water");
        assert_eq!(item_name("a bundle of oak"), "a bundle of oak");
        assert_eq!(item_name("a 25# sword"), "a 25# sword");
        assert_eq!(item_name("^ga green potion^?"), "^ga green potion^?");
        assert_eq!(item_key("^ga green potion^?"), item_key("a green potion"));
    }

    #[test]
    fn one_entry_for_a_bundle_and_for_a_name_in_colors() {
        let b = Bestiary::in_memory().unwrap();
        let w = "coffeemud";
        for (n, name) in ["a 25# oak bundle", "a 3# oak bundle", "a green potion", "^ga green potion^?", "a green potion"].iter().enumerate() {
            b.record(w, Some("Midgaard"), &Fact::ItemSeen { name: name.to_string(), container: false }, 1_000 * n as i64).unwrap();
        }
        let rows: Vec<ItemRow> = b.items(w, "").unwrap().into_iter().map(ItemRow::shown).collect();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["a green potion", "an oak bundle"]);
        // The colors the game gave, once it gave them.
        assert_eq!(rows[0].styled[0].style.fg, Some(crate::ansi::Color::Index { index: 10 }));
        let entry = b.item_entry(w, &rows[1].key).unwrap().unwrap();
        assert_eq!(entry.areas[0].count, 2);
    }

    #[test]
    fn kept_and_listed() {
        let b = Bestiary::in_memory().unwrap();
        let w = "coffeemud";
        let at = |n: i64| 1_000 * n;
        let area = Some("Midgaard");
        b.record(w, area, &Fact::Seen { name: "an orc".into() }, at(1)).unwrap();
        b.record(w, area, &Fact::Seen { name: "an orc".into() }, at(2)).unwrap();
        b.record(w, Some("Moria"), &Fact::Seen { name: "an orc".into() }, at(3)).unwrap();
        b.record(w, area, &Fact::Level { name: "An orc".into(), low: 8, high: 12 }, at(4)).unwrap();
        b.record(w, area, &Fact::Level { name: "An orc".into(), low: 10, high: 14 }, at(5)).unwrap();
        b.record(w, area, &Fact::Health { name: "an orc".into(), max: 55 }, at(5)).unwrap();
        b.record(w, area, &Fact::Ability { name: "an orc".into(), ability: "Magic Missile".into(), kind: "spell".into() }, at(6)).unwrap();
        b.record(w, area, &Fact::Died { name: "An orc".into() }, at(7)).unwrap();
        b.record(w, area, &Fact::Dropped { name: "an orc".into(), item: "a rusty scimitar".into() }, at(7)).unwrap();
        b.record(w, area, &Fact::Dropped { name: "an orc".into(), item: "a rusty scimitar".into() }, at(8)).unwrap();
        b.record(w, area, &Fact::Wears { name: "an orc".into(), place: "worn on torso".into(), item: "a leather jerkin".into() }, at(8)).unwrap();
        b.record(w, area, &Fact::Item { name: "a rusty scimitar".into(), level: Some((3, 3)), weight: None, material: Some("iron".into()), description: None }, at(9)).unwrap();
        b.record(w, area, &Fact::ItemSeen { name: "a rusty scimitar".into(), container: false }, at(9)).unwrap();

        let rows = b.mobs(w, "").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].race.as_deref(), Some("Orc"));
        assert_eq!(rows[0].areas, ["Midgaard", "Moria"]);
        assert_eq!(b.mobs(w, "moria").unwrap().len(), 1);
        assert_eq!(b.mobs(w, "dragon").unwrap().len(), 0);
        assert_eq!(b.mobs("another world", "").unwrap().len(), 0);

        let orc = b.mob_entry(w, "anorc").unwrap().unwrap();
        assert_eq!(orc.category.as_deref(), Some("Porcine"));
        assert_eq!(orc.creature, Some("goblinoid"));
        assert_eq!(orc.deaths, 1);
        assert_eq!(orc.seems, Some("Mage"));
        let midgaard = orc.areas.iter().find(|a| a.area == "Midgaard").unwrap();
        assert_eq!((midgaard.seen, midgaard.level_low, midgaard.level_high, midgaard.max_hp), (2, Some(10), Some(12), Some(55)));
        assert_eq!(orc.drops, [Counted { name: "a rusty scimitar".into(), what: None, count: 2 }]);
        assert_eq!(orc.wears, [Counted { name: "a leather jerkin".into(), what: Some("worn on torso".into()), count: 1 }]);
        assert_eq!(b.creature(w, "An orc"), Some(Creature::Goblinoid));
        // Never met, the name still says.
        assert_eq!(b.creature(w, "a black dog"), Some(Creature::Canine));

        let items = b.items(w, "iron").unwrap();
        assert_eq!(items.len(), 1);
        let sword = b.item_entry(w, &items[0].key).unwrap().unwrap();
        assert_eq!((sword.level_low, sword.material.as_deref()), (Some(3), Some("iron")));
        assert_eq!(sword.dropped_by, [Counted { name: "an orc".into(), what: None, count: 2 }]);
        assert_eq!(sword.areas, [Counted { name: "Midgaard".into(), what: None, count: 1 }]);
        assert_eq!(b.item_entry(w, "nothing").unwrap(), None);
    }

    #[test]
    fn a_long_look_corrects_the_name() {
        let b = Bestiary::in_memory().unwrap();
        b.record("w", None, &Fact::Seen { name: "a dog-faced warrior".into() }, 1).unwrap();
        assert_eq!(b.creature("w", "a dog-faced warrior"), Some(Creature::Canine));
        b.record("w", None, &Fact::Race { name: "a dog-faced warrior".into(), race: "gnoll".into() }, 2).unwrap();
        let entry = b.mob_entry("w", "adogfacedwarrior").unwrap().unwrap();
        assert_eq!((entry.race.as_deref(), entry.race_seen), (Some("Gnoll"), true));
    }
}

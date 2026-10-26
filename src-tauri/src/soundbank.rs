//! coffeemud.sf2, CoffeeMUD's sounds as a SoundFont: each of the game's
//! events on a key of its own, made by ansiapps' Neumetik and dedicated
//! to the public domain (CC0). Shipped in the app's Resources
//! (`coffeemud/coffeemud.sf2`, fetched by scripts/coffeemud-sf2-fetch.py),
//! with `CREDITS.txt` beside it.
//!
//! The slots are `soundbank.tsv`, Neumetik's `coffeemud/map.tsv` at the
//! bank's release (scripts/coffeemud-sf2-sync.py): a program and a key
//! each, what triggers it, the MSP names CoffeeMUD sends for it, and
//! whether it plays once or loops while held. Every sound plays at its
//! own pitch on its key, so the key names the sound. On some keys the
//! velocity also picks the sound (`Tier`).
//!
//! Each sound is an asset by its name in the `bank` folder
//! (`bank/Flesh hit (light)`), so triggers, captions and the buses take
//! it like a file in Assets. `Sounds` makes it a WAV the first time it's
//! asked for and keeps it (`soundbank-cache/<VERSION>/` in app data): a
//! one-shot played through Neumetik MIDI from its note-on to its end, a
//! loop taken straight from the bank at its loop points. Pure but for
//! `Sounds`, and unit-tested.
//!
//! Its loops are also the ambience's defaults: where the player's
//! triggers leave BGN or BGW silent, the terrain's loop and the weather
//! overhead's play, and in a fight its battle loop (`fill_ambience`).
//! Its one-shots are the defaults for the game's MSP sounds
//! (`msp_default`, with the names CoffeeMUD rolls dice for, `ROLLED`)
//! and for Coupler's own events (`event_default`, `foley.rs`). A sound
//! the bank lays a sub-bass or an accent under is made with them in it
//! (`LAYERS`), so it plays as one wherever it's chosen.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use neumetik_midi::soundfont::{Link, Sample, SoundFont};
use neumetik_midi::SoundBank;
use serde::Serialize;

use crate::foley::{Happening, Manner};

/// Where the bank is, in the app's Resources.
pub const FILE: &str = "coffeemud/coffeemud.sf2";
/// The bank's version, as scripts/coffeemud-sf2-fetch.py's `TAG` names
/// it. Its rendered sounds are kept per version.
pub const VERSION: &str = "0.1.0";
/// The folder its sounds are named in, as assets: `bank/Tavern`.
pub const FOLDER: &str = "bank";

const MAP: &str = include_str!("soundbank.tsv");
/// One-shots are rendered at this rate, for at most `LONGEST_SECONDS`.
const RATE: u32 = 44_100;
const LONGEST_SECONDS: usize = 30;
/// Quieter than this (-80 dB) is silence, trimmed from a one-shot's end.
const SILENCE: f32 = 1e-4;
/// The loudest a one-shot is made (-1 dBFS), as the bank mixes its own.
const PEAK: f32 = 0.891;
/// How the one-shots are made, beside the bank's version in the cache's
/// folder name: a new way (2: with their layers) makes them again.
const MADE: u32 = 2;

/// A sound in the bank.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// 0 effects, 1 ambience and weather, 2 interface and socials, 3
    /// crafting and foley; 4 and 5 are 0's sub-bass and high detail alone.
    pub program: u8,
    pub key: u8,
    /// Its name, as a caption says it ("Fire impact").
    pub name: &'static str,
    /// What sets it off, in words.
    pub trigger: &'static str,
    /// The MSP sounds CoffeeMUD sends for it (`fireball.wav`).
    pub msp: Vec<&'static str>,
    /// Loops while its key is held, rather than playing once to its end.
    pub looped: bool,
    /// Velocity picks one of three sounds (`Tier`).
    pub tiered: bool,
}

/// On a tiered key, which of its three sounds: combat's light, normal
/// and severe, a creature's idle, aggressive and pain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Light,
    Normal,
    Severe,
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Light, Tier::Normal, Tier::Severe];

    /// The loudest velocity in the tier (of 1–63, 64–107, 108–127). The
    /// bank sets each sound's level in its samples, so at 127 a sound is
    /// as its mix made it; a lighter tier plays quieter, as it was meant to.
    pub fn velocity(self) -> u8 {
        match self {
            Tier::Light => 63,
            Tier::Normal => 107,
            Tier::Severe => 127,
        }
    }
}

impl Slot {
    /// The tiers it has: three, or the one played at every velocity.
    pub fn tiers(&self) -> &'static [Tier] {
        if self.tiered {
            &Tier::ALL
        } else {
            &[Tier::Severe]
        }
    }

    /// A tier's word in its name: light and severe for a blow, idle and
    /// pain for a creature. The middle one, and an untiered sound, has none.
    pub fn tier_word(&self, tier: Tier) -> Option<&'static str> {
        let creature = self.program == 0 && (48..=59).contains(&self.key);
        match (self.tiered, tier, creature) {
            (false, _, _) | (true, Tier::Normal, _) => None,
            (true, Tier::Light, false) => Some("light"),
            (true, Tier::Severe, false) => Some("severe"),
            (true, Tier::Light, true) => Some("idle"),
            (true, Tier::Severe, true) => Some("pain"),
        }
    }

    /// A tier of it as an asset: `bank/Flesh hit (light)`.
    pub fn asset_path(&self, tier: Tier) -> String {
        match self.tier_word(tier) {
            Some(word) => format!("{FOLDER}/{} ({word})", self.name),
            None => format!("{FOLDER}/{}", self.name),
        }
    }
}

/// Every slot in the bank, in the map's order.
pub fn slots() -> &'static [Slot] {
    static SLOTS: OnceLock<Vec<Slot>> = OnceLock::new();
    SLOTS.get_or_init(|| parse(MAP))
}

fn parse(map: &'static str) -> Vec<Slot> {
    let mut lines = map.lines();
    let head: Vec<&str> = lines.next().unwrap_or_default().split('\t').collect();
    let col = |name: &str| head.iter().position(|h| *h == name).unwrap_or_else(|| panic!("soundbank.tsv has no {name} column"));
    let (program, key, name, trigger, msp, play, source) = (col("prog"), col("key"), col("name"), col("trigger"), col("msp"), col("play"), col("source"));
    lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&'static str> = l.split('\t').collect();
            let get = |i: usize| f.get(i).copied().unwrap_or_default();
            Slot {
                program: get(program).parse().expect("a program number"),
                key: get(key).parse().expect("a key number"),
                name: get(name),
                trigger: get(trigger),
                msp: get(msp).split_whitespace().collect(),
                looped: get(play) == "loop",
                tiered: get(source).split(" | ").count() == 3,
            }
        })
        .collect()
}

/// The slot on a program's key.
pub fn slot(program: u8, key: u8) -> Option<&'static Slot> {
    slots().iter().find(|s| s.program == program && s.key == key)
}

/// The slot for an MSP sound, by name, any case. A few names are on two
/// slots (CoffeeMUD sends `dooropen.wav` for closing too): the first, as
/// the map orders them, is the one meant, but for one laid under the
/// other (`explode.wav`'s sub-bass is in the fire impact's making).
pub fn by_msp(name: &str) -> Option<&'static Slot> {
    let named: Vec<&'static Slot> = slots().iter().filter(|s| s.msp.iter().any(|m| m.eq_ignore_ascii_case(name))).collect();
    let under = |s: &Slot| named.iter().any(|o| layers(o, Tier::Severe).contains(&s));
    named.iter().find(|s| !under(s)).or(named.first()).copied().or_else(|| rolled(name))
}

/// The sounds CoffeeMUD names by a roll of the dice, which the map's list
/// (every name in the code as written) can't hold: each blow that lands
/// is `punch1.wav` to `punch7.wav` (`MUDFight.standardHitString`, every
/// weapon but a bow or a thrown one), each death `death1.wav` to
/// `death7.wav` (`MUDFight.postDeath`; a feigned one rolls 1 to 4). By the
/// name's start, the slot, and the numbers rolled.
const ROLLED: [(&str, &str, u8); 2] = [("punch", "Flesh hit", 7), ("death", "Monster collapse", 7)];

fn rolled(name: &str) -> Option<&'static Slot> {
    let name = name.to_ascii_lowercase();
    let stem = name.strip_suffix(".wav")?;
    let (start, slot, most) = ROLLED.iter().find(|(start, _, _)| stem.starts_with(start))?;
    let roll: u8 = stem[start.len()..].parse().ok()?;
    (1..=*most).contains(&roll).then(|| slots().iter().find(|s| s.name == *slot)).flatten()
}

/// Every MSP sound CoffeeMUD sends, as the hooks list holds them from
/// the start: the map's, then the rolled ones.
pub fn msp_names() -> Vec<String> {
    let mut names: Vec<String> = slots().iter().flat_map(|s| s.msp.iter().map(|m| m.to_string())).collect();
    names.extend(ROLLED.iter().flat_map(|(start, _, most)| (1..=*most).map(move |n| format!("{start}{n}.wav"))));
    names
}

/// What sounds under or over another in the bank, as its map says
/// ("Critical hit, under A#1", "High detail over C#4 or C#8"): the
/// sound, the ones laid with it, and from which of its tiers (a pain
/// grunt's heavy thud only when it's severe; a roar's foundation not
/// under an idle breath).
const LAYERS: [(&str, &[&str], Tier); 28] = [
    ("Critical hit", &["Critical thump"], Tier::Light),
    ("Pain grunt", &["Heavy damage thud"], Tier::Severe),
    ("Monster collapse", &["Creature death drop"], Tier::Light),
    ("Body fall", &["Humanoid fall sub"], Tier::Light),
    ("Dragon", &["Roar foundation"], Tier::Normal),
    ("Giant-kin", &["Roar foundation"], Tier::Normal),
    ("Arcane impact", &["Arcane shimmer"], Tier::Light),
    ("Fire impact", &["Explosion sub", "Flame crackle"], Tier::Light),
    ("Cold impact", &["Frost sparkle"], Tier::Light),
    ("Water impact", &["Droplets"], Tier::Light),
    ("Acid impact", &["Acid fizz"], Tier::Light),
    ("Gas impact", &["Gas hiss"], Tier::Light),
    ("Electric impact", &["Electric sub crack", "Voltage crackle"], Tier::Light),
    ("Mind impact", &["Eerie ring"], Tier::Light),
    ("Poison impact", &["Venom buzz"], Tier::Light),
    ("Holy impact", &["Bell air"], Tier::Light),
    ("Necromantic impact", &["Whispering air"], Tier::Light),
    ("Arcane cast", &["Arcane shimmer"], Tier::Light),
    ("Fire cast", &["Flame crackle"], Tier::Light),
    ("Cold cast", &["Frost sparkle"], Tier::Light),
    ("Water cast", &["Droplets"], Tier::Light),
    ("Acid cast", &["Acid fizz"], Tier::Light),
    ("Gas cast", &["Gas hiss"], Tier::Light),
    ("Electric cast", &["Voltage crackle"], Tier::Light),
    ("Mind cast", &["Eerie ring"], Tier::Light),
    ("Poison cast", &["Venom buzz"], Tier::Light),
    ("Holy cast", &["Bell air"], Tier::Light),
    ("Necromantic cast", &["Whispering air"], Tier::Light),
];

/// The sounds laid with a slot's tier, as it's made.
pub fn layers(slot: &Slot, tier: Tier) -> Vec<&'static Slot> {
    LAYERS
        .iter()
        .filter(|(name, _, from)| *name == slot.name && tier >= *from)
        .flat_map(|(_, with, _)| with.iter().filter_map(|w| slots().iter().find(|s| s.name == *w && !s.looped)))
        .collect()
}

/// What an MSP sound plays when the player has set no sound on it: the
/// bank's sound for that name, once. Not a weather loop (BGW loops the
/// weather while it lasts, `ambient.rs`). A blow or a creature plays its
/// middle sound, but a scream its pain.
pub fn msp_default(name: &str) -> Option<String> {
    let slot = by_msp(name).filter(|s| !s.looped)?;
    let tier = match (slot.tiered, name.eq_ignore_ascii_case("scream.wav")) {
        (true, false) => Tier::Normal,
        _ => Tier::Severe,
    };
    Some(slot.asset_path(tier))
}

/// The bank's loop for each of CoffeeMUD's terrains
/// (`ambient.rs`'s `OUTDOOR` and `INDOOR`), by its name. Woods have a
/// second, by night (`bgn_default`). Open sea isn't told from a lake by
/// the terrain, so `watersurface` is the river or lake.
const TERRAIN_LOOPS: [(&str, &str); 24] = [
    ("city", "Town"),
    ("woods", "Forest by day"),
    ("rocky", "Rocky ground"),
    ("plains", "Plains"),
    ("underwater", "Underwater"),
    ("air", "Sky"),
    ("watersurface", "River or lake"),
    ("jungle", "Jungle"),
    ("swamp", "Swamp"),
    ("desert", "Desert"),
    ("hills", "Hills"),
    ("mountains", "Mountains"),
    ("spaceport", "Metal hull"),
    ("seaport", "Harbor"),
    ("stone", "Castle halls"),
    ("wooden", "Tavern"),
    ("cave", "Cave"),
    ("magic", "Wizard's tower"),
    ("in_underwater", "Underwater"),
    ("gap", "Sky"),
    ("cavelakesurface", "Cave lake"),
    ("metal", "Metal hull"),
    ("innerseaport", "Harbor"),
    ("caveseaport", "Cave lake"),
];

/// The bank's loop for each kind of weather `ambient.rs` reads, by its
/// name, as the map's triggers say. Clear has none.
const WEATHER_LOOPS: [(&str, &str); 13] = [
    ("cloudy", "Light wind"),
    ("windy", "Strong wind"),
    ("rain", "Light rain"),
    ("thunderstorm", "Thunderstorm"),
    ("snow", "Snow"),
    ("hail", "Hail"),
    ("heat", "Heat"),
    ("sleet", "Light rain"),
    ("blizzard", "Snow"),
    ("dust", "Dust storm"),
    ("drought", "Heat"),
    ("cold", "Bitter cold"),
    ("fog", "Fog"),
];

/// A loop the bank plays by default, as a share of its bus: under the
/// talk and the game's own sounds, since it never stops.
pub const LOOP_VOLUME: u8 = 60;

/// A loop's asset path, by its name in the bank.
fn loop_path(name: &str) -> Option<String> {
    slots().iter().find(|s| s.looped && s.name == name).map(|s| s.asset_path(Tier::Severe))
}

/// What BGN plays where the player has set no sound of their own: the
/// bank's loop for the terrain, the woods' by night at night.
pub fn bgn_default(terrain: &str, night: bool) -> Option<String> {
    let terrain = terrain.to_ascii_lowercase();
    let name = TERRAIN_LOOPS.iter().find(|(t, _)| *t == terrain)?.1;
    loop_path(if night && name == "Forest by day" { "Forest by night" } else { name })
}

/// What BGW plays for the weather overhead where the player has set no
/// sound of their own.
pub fn bgw_default(weather: &str) -> Option<String> {
    loop_path(WEATHER_LOOPS.iter().find(|(w, _)| *w == weather)?.1)
}

/// The loop that takes the place's in a fight.
const FIGHT_LOOP: &str = "Battle tension";

/// Fills what the player's triggers left silent with the bank's loops:
/// the terrain's for BGN (the battle's while `fighting`), the weather
/// overhead's for BGW.
pub fn fill_ambience(ambience: &mut crate::ambient::Ambience, terrain: Option<&str>, weather: Option<&str>, night: bool, fighting: bool) {
    use crate::ambient::Loop;
    let as_loop = |path| Loop { path, volume: LOOP_VOLUME };
    if ambience.bgn.is_none() {
        let place = if fighting && terrain.is_some() { loop_path(FIGHT_LOOP) } else { terrain.and_then(|t| bgn_default(t, night)) };
        ambience.bgn = place.map(as_loop);
    }
    if ambience.bgw.is_none() {
        ambience.bgw = weather.and_then(bgw_default).map(as_loop);
    }
}

/// The bank's sound for each of Coupler's own events (`foley.rs`), by its
/// name in the bank, and how loud, as a share of the SFX volume. Someone
/// leaving is heard softer than someone coming (going away), and someone
/// creeping softer still. Stomping is the footsteps with a heavy thud
/// under them (`event_under`).
const EVENT_SOUNDS: [(Happening, &str, u8); 26] = [
    (Happening::Arrives(Manner::Walks), "Footsteps in", 100),
    (Happening::Arrives(Manner::Creeps), "Footsteps in", 40),
    (Happening::Arrives(Manner::Stomps), "Footsteps in", 100),
    (Happening::Arrives(Manner::Flies), "Miss whoosh", 100),
    (Happening::Arrives(Manner::Swims), "Water impact", 100),
    (Happening::Arrives(Manner::Falls), "Fall", 100),
    (Happening::Leaves(Manner::Walks), "Footsteps out", 70),
    (Happening::Leaves(Manner::Creeps), "Footsteps out", 28),
    (Happening::Leaves(Manner::Stomps), "Footsteps out", 70),
    (Happening::Leaves(Manner::Flies), "Miss whoosh", 70),
    (Happening::Leaves(Manner::Swims), "Water impact", 70),
    (Happening::Leaves(Manner::Falls), "Fall", 70),
    // Running off: every step heard.
    (Happening::Flees, "Footsteps out", 100),
    (Happening::Sit, "Posture", 100),
    (Happening::Stand, "Posture", 100),
    (Happening::Sleep, "Sleep and wake", 100),
    (Happening::Wake, "Sleep and wake", 100),
    (Happening::Get, "Item handling", 100),
    (Happening::Drop, "Item handling", 100),
    (Happening::Give, "Item handling", 100),
    (Happening::Put, "Item handling", 100),
    (Happening::Wear, "Equip", 100),
    (Happening::Remove, "Unequip", 100),
    (Happening::Buy, "Coins", 100),
    (Happening::Sell, "Coins", 100),
    (Happening::Coins, "Coins", 100),
];

/// What one of Coupler's own events plays when the player has set no
/// sound on it, and at what share of the SFX volume.
pub fn event_default(happening: Happening) -> Option<(String, u8)> {
    let (_, name, volume) = EVENT_SOUNDS.iter().find(|(h, _, _)| *h == happening)?;
    slots().iter().find(|s| !s.looped && s.name == *name).map(|s| (s.asset_path(Tier::Severe), *volume))
}

/// The bank's sound for a creature's cry (`bestiary.rs`, `coupler.creature`):
/// its kind's key, idle as it comes, aggressive as it fights, pain as it dies.
pub fn creature_default(value: &str) -> Option<String> {
    use crate::bestiary::{cry_of, Cry};
    let (creature, cry) = cry_of(value)?;
    let tier = match cry {
        Cry::Comes => Tier::Light,
        Cry::Fights => Tier::Normal,
        Cry::Dies => Tier::Severe,
    };
    slots().iter().find(|s| !s.looped && s.tiered && s.name == creature.slot()).map(|s| s.asset_path(tier))
}

/// What plays with an event's own sound when the player has set none: a
/// stomp's heavy thud.
pub fn event_under(happening: Happening) -> Option<String> {
    let stomps = matches!(happening, Happening::Arrives(Manner::Stomps) | Happening::Leaves(Manner::Stomps));
    stomps.then(|| slots().iter().find(|s| s.name == "Heavy impact").map(|s| s.asset_path(Tier::Severe))).flatten()
}

/// Whether an asset path is in the bank's folder (named one of its sounds
/// or not).
pub fn is_bank_path(path: &str) -> bool {
    path.strip_prefix(FOLDER).is_some_and(|rest| rest.starts_with('/'))
}

/// The sound an asset path names, any case: `bank/Flesh hit (light)`.
pub fn find(path: &str) -> Option<(&'static Slot, Tier)> {
    let name = path.strip_prefix(FOLDER)?.strip_prefix('/')?;
    slots().iter().find_map(|s| s.tiers().iter().find(|t| s.asset_path(**t)[FOLDER.len() + 1..].eq_ignore_ascii_case(name)).map(|t| (s, *t)))
}

/// One of the bank's sounds, as the hook editor lists it.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub path: String,
    /// A loop (for BGN and BGW), rather than a sound played once (SFX).
    pub looped: bool,
    /// The MSP sounds that play it when the player has set none
    /// (`msp_default`).
    pub msp: Vec<String>,
    /// Coupler's own events that play it when the player has set none
    /// (`event_default`).
    pub events: Vec<&'static str>,
    /// Creatures' cries that play it when the player has set none
    /// (`creature_default`): `"canine fights"`.
    pub cries: Vec<String>,
    /// The terrains it loops in when the player has set no BGN, by day
    /// or by night (`bgn_default`).
    pub terrains: Vec<&'static str>,
    /// The weathers it loops for when the player has set no BGW
    /// (`bgw_default`).
    pub weathers: Vec<&'static str>,
}

/// Every sound in the bank, each tier its own, in the map's order.
pub fn list() -> Vec<Sound> {
    let names = msp_names();
    slots()
        .iter()
        .flat_map(|s| {
            let names = &names;
            s.tiers().iter().map(move |t| {
                let path = s.asset_path(*t);
                let msp = names.iter().filter(|name| by_msp(name) == Some(s) && msp_default(name).as_deref() == Some(path.as_str())).cloned().collect();
                let events = Happening::ALL.iter().filter(|h| event_default(**h).is_some_and(|(p, _)| p == path)).map(|h| h.name()).collect();
                let cries = crate::bestiary::cry_values().into_iter().filter(|v| creature_default(v).as_deref() == Some(path.as_str())).collect();
                let terrains = TERRAIN_LOOPS.iter().map(|(t, _)| *t).filter(|t| [false, true].iter().any(|night| bgn_default(t, *night).as_deref() == Some(path.as_str()))).collect();
                let weathers = WEATHER_LOOPS.iter().map(|(w, _)| *w).filter(|w| bgw_default(w).as_deref() == Some(path.as_str())).collect();
                Sound { path, looped: s.looped, msp, events, cries, terrains, weathers }
            })
        })
        .collect()
}

/// The groups the defaults are listed in (`defaults`), in this order.
pub const GROUPS: [&str; 12] =
    ["Places", "Weather", "Combat", "Magic", "Creatures", "Nature", "Doors and locks", "Comings and goings", "Everyday", "Crafting", "Socials", "Your character"];

/// A one-shot's group, by where the map puts it.
fn group_of(slot: &Slot) -> &'static str {
    match (slot.program, slot.key) {
        (0, 19) | (3, 89..=90) => "Nature",
        (0, 26..=38) => "Combat",
        (0, 39..=47) => "Doors and locks",
        (0, 48..=59) => "Creatures",
        (0, _) => "Magic",
        (2, 62) | (2, 71) => "Your character",
        (2, _) => "Socials",
        (3, 60..=81) => "Crafting",
        _ => "Everyday",
    }
}

/// One of the bank's defaults: what plays for a pair the player has set
/// no sound on, and whether they have (`yours`).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankDefault {
    /// One of `GROUPS`.
    pub group: &'static str,
    /// The pair it plays for (its value as text, not JSON); no key for the
    /// battle loop, which plays for a fight rather than a pair.
    pub key: &'static str,
    pub value: String,
    /// When, beside the pair: "by night".
    pub when: Option<&'static str>,
    /// `SFX`, `BGN` or `BGW`.
    pub bus: &'static str,
    /// The bank's sound, and one played with it (a stomp's thud).
    pub sound: String,
    pub under: Option<String>,
    pub volume: u8,
    /// The player's own sound for it, which plays instead.
    pub yours: Option<Yours>,
    /// What the pair sets off now, all of it, for the hook editor.
    pub trigger: Option<crate::hooks::Trigger>,
}

/// The player's trigger that takes a default's place: on its own pair,
/// or for a terrain on its room type's.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Yours {
    pub key: &'static str,
    pub value: String,
    pub sound: String,
    pub volume: u8,
}

/// Every default, and how many rooms have background noise of their own
/// (`room.info.id`), which plays there in place of the terrain's.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Defaults {
    pub sounds: Vec<BankDefault>,
    pub rooms: usize,
}

/// Every pair the bank plays a sound for by default, in `GROUPS`' order,
/// each with the player's own sound on it if `trigger` (a pair, its
/// value as JSON) gives one.
pub fn defaults(trigger: impl Fn(&str, &str) -> Option<crate::hooks::Trigger>, rooms: usize) -> Defaults {
    use crate::ambient::{json_text, room_type, ROOM_TYPE_KEY, WEATHER_KEY};
    let mut sounds = Vec::new();
    let sfx = |key: &'static str, value: &str| {
        let t = trigger(key, &json_text(value))?;
        Some(Yours { key, value: value.to_string(), sound: t.sfx?, volume: t.sfx_volume })
    };
    let bgn = |key: &'static str, value: &str| {
        let t = trigger(key, &json_text(value))?;
        Some(Yours { key, value: value.to_string(), sound: t.bgn?, volume: t.bgn_volume })
    };
    for (terrain, _) in TERRAIN_LOOPS {
        let yours = bgn("room.info.terrain", terrain).or_else(|| room_type(terrain).and_then(|kind| bgn(ROOM_TYPE_KEY, kind)));
        let nights: &[(bool, Option<&'static str>)] = if bgn_default(terrain, true) != bgn_default(terrain, false) { &[(false, Some("by day")), (true, Some("by night"))] } else { &[(false, None)] };
        for (night, when) in nights {
            let Some(sound) = bgn_default(terrain, *night) else { continue };
            let own = trigger("room.info.terrain", &json_text(terrain));
            sounds.push(BankDefault { group: "Places", key: "room.info.terrain", value: terrain.to_string(), when: *when, bus: "BGN", sound, under: None, volume: LOOP_VOLUME, yours: yours.clone(), trigger: own });
        }
    }
    for (weather, _) in WEATHER_LOOPS {
        let Some(sound) = bgw_default(weather) else { continue };
        let own = trigger(WEATHER_KEY, &json_text(weather));
        let yours = own.clone().and_then(|t| Some(Yours { key: WEATHER_KEY, value: weather.to_string(), sound: t.bgw?, volume: t.bgw_volume }));
        sounds.push(BankDefault { group: "Weather", key: WEATHER_KEY, value: weather.to_string(), when: None, bus: "BGW", sound, under: None, volume: LOOP_VOLUME, yours, trigger: own });
    }
    if let Some(sound) = loop_path(FIGHT_LOOP) {
        sounds.push(BankDefault { group: "Combat", key: "", value: "in a fight".into(), when: None, bus: "BGN", sound, under: None, volume: LOOP_VOLUME, yours: None, trigger: None });
    }
    let mut seen = std::collections::HashSet::new();
    for name in msp_names() {
        let Some(sound) = msp_default(&name) else { continue };
        if !seen.insert(name.to_ascii_lowercase()) {
            continue;
        }
        let Some(slot) = by_msp(&name) else { continue };
        let yours = sfx(crate::msp::SOUND_KEY, &name);
        let own = trigger(crate::msp::SOUND_KEY, &json_text(&name));
        sounds.push(BankDefault { group: group_of(slot), key: crate::msp::SOUND_KEY, value: name, when: None, bus: "SFX", sound, under: None, volume: 100, yours, trigger: own });
    }
    for happening in Happening::ALL {
        let Some((sound, volume)) = event_default(happening) else { continue };
        let group = if matches!(happening, Happening::Arrives(_) | Happening::Leaves(_) | Happening::Flees) { "Comings and goings" } else { "Everyday" };
        let value = happening.name();
        let yours = sfx(crate::foley::EVENT_KEY, value);
        let own = trigger(crate::foley::EVENT_KEY, &json_text(value));
        sounds.push(BankDefault { group, key: crate::foley::EVENT_KEY, value: value.into(), when: None, bus: "SFX", sound, under: event_under(happening), volume, yours, trigger: own });
    }
    for value in crate::bestiary::cry_values() {
        let Some(sound) = creature_default(&value) else { continue };
        let yours = sfx(crate::bestiary::CREATURE_KEY, &value);
        let own = trigger(crate::bestiary::CREATURE_KEY, &json_text(&value));
        sounds.push(BankDefault { group: "Creatures", key: crate::bestiary::CREATURE_KEY, value, when: None, bus: "SFX", sound, under: None, volume: 100, yours, trigger: own });
    }
    // Stable: each group keeps the order it was listed in.
    sounds.sort_by_key(|d| GROUPS.iter().position(|g| *g == d.group));
    Defaults { sounds, rooms }
}

/// The bank's path, in the app's Resources folder.
pub fn path(resources: &Path) -> PathBuf {
    resources.join(FILE)
}

/// Reads the bank, for a synthesizer to play through.
pub fn open(resources: &Path) -> Result<Arc<SoundBank>, String> {
    let bytes = read(resources)?;
    SoundBank::from_bytes(&bytes).map(Arc::new).map_err(|e| format!("CoffeeMUD's sounds couldn't be used. ({e})"))
}

fn read(resources: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path(resources)).map_err(|e| format!("CoffeeMUD's sounds couldn't be read. ({e})"))
}

/// A one-shot played through Neumetik MIDI, dry, from its note-on to the
/// end of its sample, with what the bank lays under or over it
/// (`layers`) at the same velocity, as a WAV (mono if both sides are the
/// same). Its level is the bank's own: nothing's made louder, so the
/// bank's mix holds, and where the layers together would peak over
/// -1 dBFS, all of it is brought down to that.
pub fn render_once(bank: &Arc<SoundBank>, slot: &Slot, tier: Tier) -> Vec<u8> {
    let mut synth = neumetik_midi::Synth::new(RATE);
    synth.set_sound_bank(Some(bank.clone()));
    // The bank was mixed without the synthesizer's reverb and chorus.
    synth.control_change(0, 91, 0);
    synth.control_change(0, 93, 0);
    synth.program_change(0, slot.program);
    // Never let go: a one-shot's voice ends with its sample, and a note-off
    // before the first render silences it at once, long release or not.
    synth.note_on(0, slot.key, tier.velocity());
    for layer in layers(slot, tier) {
        let channel = if layer.program == slot.program { 0 } else { 1 };
        if channel == 1 {
            synth.control_change(1, 91, 0);
            synth.control_change(1, 93, 0);
            synth.program_change(1, layer.program);
        }
        synth.note_on(channel, layer.key, tier.velocity());
    }
    const BLOCK: usize = 1024;
    let (mut left, mut right) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    let mut out: Vec<f32> = Vec::new();
    while out.len() < LONGEST_SECONDS * RATE as usize * 2 {
        synth.render(&mut left, &mut right);
        out.extend(left.iter().zip(&right).flat_map(|(l, r)| [*l, *r]));
        if synth.active_voices() == 0 {
            break;
        }
    }
    out.truncate(out.iter().rposition(|x| x.abs() > SILENCE).map_or(0, |i| i / 2 * 2 + 2));
    let peak = out.iter().fold(0.0f32, |p, x| p.max(x.abs()));
    if peak > PEAK {
        out.iter_mut().for_each(|x| *x *= PEAK / peak);
    }
    let samples: Vec<i16> = out.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
    if samples.as_chunks::<2>().0.iter().all(|[l, r]| l == r) {
        crate::wav::wav(1, RATE, &samples.iter().step_by(2).copied().collect::<Vec<_>>())
    } else {
        crate::wav::wav(2, RATE, &samples)
    }
}

/// The samples a slot plays at a velocity, by their place in the bank.
fn samples_for(font: &SoundFont, slot: &Slot, vel: u8) -> Vec<usize> {
    let Some(preset) = font.presets.iter().find(|p| p.bank == 0 && p.program == u16::from(slot.program)) else {
        return Vec::new();
    };
    preset
        .zones
        .iter()
        .filter(|z| z.covers(slot.key, vel))
        .filter_map(|z| z.target)
        .flat_map(|i| font.instruments[i].zones.iter().filter(|z| z.covers(slot.key, vel)).filter_map(|z| z.target))
        .collect()
}

/// A loop as a WAV, straight from the bank: its loop and nothing else, so
/// it goes round without a seam, at its own rate, a stereo pair as two
/// channels.
pub fn loop_take(font: &SoundFont, slot: &Slot) -> Result<Vec<u8>, String> {
    let missing = || format!("{} isn't in CoffeeMUD's sounds.", slot.name);
    let first = &font.samples[*samples_for(font, slot, 127).first().ok_or_else(missing)?];
    let (left, right) = match first.link {
        Link::Left(other) => (first, Some(&font.samples[other])),
        Link::Right(other) => (&font.samples[other], Some(first)),
        Link::Mono => (first, None),
    };
    let take = |s: &Sample| s.data.get(s.loop_start as usize..s.loop_end as usize).filter(|t| !t.is_empty()).map(<[i16]>::to_vec);
    let left_take = take(left).ok_or_else(|| format!("{} doesn't loop.", slot.name))?;
    match right.map(take) {
        None => Ok(crate::wav::wav(1, left.rate, &left_take)),
        Some(Some(right_take)) if right_take.len() == left_take.len() => {
            Ok(crate::wav::wav(2, left.rate, &left_take.iter().zip(&right_take).flat_map(|(l, r)| [*l, *r]).collect::<Vec<_>>()))
        }
        Some(_) => Err(format!("{}'s two sides don't loop together.", slot.name)),
    }
}

/// The bank's sounds as WAVs, each made once and kept. The bank, once
/// read, stays read: the game's sounds come often.
#[derive(Default)]
pub struct Sounds {
    inner: Mutex<Inner>,
    /// The game's sounds have been made ready this run (`warm`).
    warmed: AtomicBool,
}

#[derive(Default)]
struct Inner {
    resources: Option<PathBuf>,
    cache: Option<PathBuf>,
    bank: Option<Arc<SoundBank>>,
}

impl Sounds {
    /// Where the bank ships (the app's Resources), and the folder its
    /// sounds are kept in (`soundbank-cache`, a folder per version and
    /// way of making them: an older one's are deleted).
    pub fn set_dirs(&self, resources: PathBuf, cache_root: &Path) {
        let cache = cache_root.join(format!("{VERSION}-{MADE}"));
        for old in std::fs::read_dir(cache_root).into_iter().flatten().flatten() {
            if old.path() != cache {
                let _ = std::fs::remove_dir_all(old.path());
            }
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.resources = Some(resources);
        inner.cache = Some(cache);
    }

    /// Whether this copy of Coupler has the bank.
    pub fn available(&self) -> bool {
        self.dirs().is_some_and(|(resources, _)| path(&resources).is_file())
    }

    fn dirs(&self) -> Option<(PathBuf, PathBuf)> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Some((inner.resources.clone()?, inner.cache.clone()?))
    }

    /// One of the bank's sounds as a WAV, by its asset path.
    pub fn audio(&self, asset: &str) -> Result<Vec<u8>, String> {
        let (slot, tier) = find(asset).ok_or_else(|| format!("{asset} isn't one of CoffeeMUD's sounds."))?;
        let (resources, cache) = self.dirs().ok_or("CoffeeMUD's sounds aren't ready yet.")?;
        let file = cache.join(file_name(slot, tier));
        if let Ok(bytes) = std::fs::read(&file) {
            return Ok(bytes);
        }
        if !slot.looped {
            let wav = render_once(&self.bank(&resources)?, slot, tier);
            keep(&cache, &file, &wav);
            return Ok(wav);
        }
        // Reading the bank for its loops is the slow part, so the first
        // loop asked for takes them all out.
        let font = SoundFont::from_bytes(&read(&resources)?).map_err(|e| format!("CoffeeMUD's sounds couldn't be used. ({e})"))?;
        let mut asked = None;
        for s in slots().iter().filter(|s| s.looped) {
            let Ok(wav) = loop_take(&font, s) else { continue };
            keep(&cache, &cache.join(file_name(s, Tier::Severe)), &wav);
            if s == slot {
                asked = Some(wav);
            }
        }
        asked.map_or_else(|| loop_take(&font, slot), Ok)
    }

    /// Makes every sound the game's MSP names and Coupler's own events
    /// play ready, once a run, so the first fireball isn't late: each made
    /// now that isn't kept from before. Slow (the bank's one-shots are
    /// rendered), so on a thread of its own.
    pub fn warm(&self) {
        if !self.available() || self.warmed.swap(true, Ordering::Relaxed) {
            return;
        }
        let mut paths: Vec<String> = msp_names().iter().filter_map(|name| msp_default(name)).collect();
        paths.extend(Happening::ALL.into_iter().filter_map(event_default).map(|(path, _)| path));
        paths.extend(Happening::ALL.into_iter().filter_map(event_under));
        paths.sort();
        paths.dedup();
        for path in paths {
            if let Err(e) = self.audio(&path) {
                log::warn!("{e}");
            }
        }
    }

    fn bank(&self, resources: &Path) -> Result<Arc<SoundBank>, String> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(bank) = &inner.bank {
            return Ok(bank.clone());
        }
        let bank = open(resources)?;
        inner.bank = Some(bank.clone());
        Ok(bank)
    }
}

/// A sound's file in the cache: its program, key and velocity.
fn file_name(slot: &Slot, tier: Tier) -> String {
    format!("{}-{}-{}.wav", slot.program, slot.key, tier.velocity())
}

/// Keeps a made sound for next time, through a file renamed into place.
/// One that can't be kept costs only another render.
fn keep(cache: &Path, file: &Path, wav: &[u8]) {
    let part = file.with_extension("part");
    if std::fs::create_dir_all(cache).is_ok() && std::fs::write(&part, wav).is_ok() {
        let _ = std::fs::rename(&part, file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_map_reads() {
        let all = slots();
        assert_eq!(all.len(), 210);
        let keys: HashSet<(u8, u8)> = all.iter().map(|s| (s.program, s.key)).collect();
        assert_eq!(keys.len(), all.len(), "a key booked twice");
        assert!(all.iter().all(|s| s.program <= 3 && s.key <= 127 && !s.name.is_empty()));
        // The ambience and weather loops, and the dread drone under them.
        assert_eq!(all.iter().filter(|s| s.looped).count(), 53);
        assert!(all.iter().filter(|s| s.looped).all(|s| s.program == 1 || (s.program, s.key) == (0, 23)));
        // Combat's and the creatures' keys but arrow release, bow draw,
        // miss and critical hit, which have one sound at every velocity.
        assert_eq!(all.iter().filter(|s| s.tiered).count(), 20);
    }

    #[test]
    fn finds_by_msp_name() {
        let names: HashSet<&str> = slots().iter().flat_map(|s| s.msp.iter().copied()).collect();
        assert_eq!(names.len(), 104);
        assert!(names.iter().all(|n| n.ends_with(".wav")));
        assert_eq!(by_msp("fireball.wav").map(|s| s.name), Some("Fire impact"));
        assert_eq!(by_msp("FIREBALL.WAV").map(|s| s.name), Some("Fire impact"));
        assert_eq!(by_msp("dooropen.wav").map(|s| s.name), Some("Door open"));
        assert_eq!(by_msp("rain.wav").map(|s| (s.program, s.key, s.looped)), Some((1, 85, true)));
        assert_eq!(by_msp("nothing.wav"), None);
    }

    #[test]
    fn defaults_are_grouped_and_show_the_players_own() {
        use crate::hooks::Trigger;
        let none = defaults(|_, _| None, 0);
        let groups: Vec<&str> = none.sounds.iter().map(|d| d.group).collect();
        let mut sorted = groups.clone();
        sorted.sort_by_key(|g| GROUPS.iter().position(|o| o == g));
        assert_eq!(groups, sorted);
        assert!(GROUPS.iter().all(|g| groups.contains(g)), "an empty group");
        assert!(none.sounds.iter().all(|d| d.yours.is_none()));
        let find = |all: &Defaults, key: &str, value: &str, when: Option<&str>| all.sounds.iter().find(|d| d.key == key && d.value == value && d.when == when).cloned().unwrap();
        // Every terrain, the woods twice; every weather; one row a name.
        assert_eq!(none.sounds.iter().filter(|d| d.group == "Places").count(), 25);
        assert_eq!(find(&none, "room.info.terrain", "woods", Some("by night")).sound, "bank/Forest by night");
        assert_eq!(none.sounds.iter().filter(|d| d.group == "Weather").count(), 13);
        assert_eq!(none.sounds.iter().filter(|d| d.value == "dooropen.wav").count(), 1);
        assert_eq!(find(&none, "msp.sound", "fireball.wav", None).group, "Magic");
        assert_eq!(find(&none, "msp.sound", "punch3.wav", None).group, "Combat");
        assert_eq!(find(&none, "msp.sound", "laugh.wav", None).group, "Socials");
        assert_eq!(find(&none, "msp.sound", "hammer.wav", None).group, "Crafting");
        assert_eq!(find(&none, "coupler.event", "someone stomps in", None).under.as_deref(), Some("bank/Heavy impact"));
        assert_eq!(find(&none, "coupler.event", "you sit", None).group, "Everyday");
        assert!(none.sounds.iter().any(|d| d.key.is_empty() && d.sound == "bank/Battle tension"));

        let mine = |sfx: Option<&str>, bgn: Option<&str>, bgw: Option<&str>| Trigger { sfx: sfx.map(Into::into), bgn: bgn.map(Into::into), bgw: bgw.map(Into::into), ..Trigger::default() };
        let set = defaults(
            |key, value| match (key, value) {
                ("msp.sound", r#""fireball.wav""#) => Some(mine(Some("wav/boom.wav"), None, None)),
                // Music only: the bank's sound still plays.
                ("msp.sound", r#""laugh.wav""#) => Some(Trigger { bgm: Some("mid/ha.mid".into()), ..Trigger::default() }),
                ("coupler.event", r#""you sit""#) => Some(mine(Some("wav/chair.wav"), None, None)),
                ("coupler.room.type", r#""indoors""#) => Some(mine(None, Some("wav/inside.wav"), None)),
                ("room.info.terrain", r#""cave""#) => Some(mine(None, Some("wav/drip.wav"), None)),
                ("coupler.weather", r#""rain""#) => Some(mine(None, None, Some("wav/rain.wav"))),
                _ => None,
            },
            2,
        );
        assert_eq!(set.rooms, 2);
        let yours = |key, value| find(&set, key, value, None).yours.map(|y| (y.key, y.sound));
        assert_eq!(yours("msp.sound", "fireball.wav"), Some(("msp.sound", "wav/boom.wav".into())));
        assert_eq!(yours("msp.sound", "laugh.wav"), None);
        assert_eq!(find(&set, "msp.sound", "laugh.wav", None).trigger.and_then(|t| t.bgm).as_deref(), Some("mid/ha.mid"));
        assert_eq!(yours("coupler.event", "you sit"), Some(("coupler.event", "wav/chair.wav".into())));
        assert_eq!(yours("room.info.terrain", "cave"), Some(("room.info.terrain", "wav/drip.wav".into())));
        assert_eq!(yours("room.info.terrain", "stone"), Some(("coupler.room.type", "wav/inside.wav".into())));
        assert_eq!(yours("room.info.terrain", "city"), None);
        assert_eq!(yours("coupler.weather", "rain"), Some(("coupler.weather", "wav/rain.wav".into())));
    }

    #[test]
    fn msp_defaults_are_one_shots() {
        assert_eq!(msp_default("fireball.wav").as_deref(), Some("bank/Fire impact"));
        assert_eq!(msp_default("bashed1.wav").as_deref(), Some("bank/Flesh hit"));
        assert_eq!(msp_default("scream.wav").as_deref(), Some("bank/Humanoid (pain)"));
        assert_eq!(msp_default("rain.wav"), None, "the weather loops on BGW");
        assert_eq!(msp_default("nothing.wav"), None);
        // Each names a sound the bank plays.
        for name in slots().iter().flat_map(|s| s.msp.iter()) {
            if let Some(path) = msp_default(name) {
                let (slot, _) = find(&path).unwrap_or_else(|| panic!("{path} isn't a sound"));
                assert!(!slot.looped);
            }
        }
        assert_eq!(slot(0, 29).map(|s| (s.name, s.tiered)), Some(("Flesh hit", true)));
        assert_eq!(slot(4, 12), None);
        // The explosion's sub-bass is in the fire impact's making.
        assert_eq!(msp_default("explode.wav").as_deref(), Some("bank/Fire impact"));
        assert_eq!(msp_default("earthquake.wav").as_deref(), Some("bank/Earthquake rumble"));
    }

    #[test]
    fn rolled_names_play_too() {
        for n in 1..=7 {
            assert_eq!(msp_default(&format!("punch{n}.wav")).as_deref(), Some("bank/Flesh hit"));
            assert_eq!(msp_default(&format!("DEATH{n}.WAV")).as_deref(), Some("bank/Monster collapse"));
        }
        for name in ["punch0.wav", "punch8.wav", "punch.wav", "death.wav", "punchy.wav", "death12.wav", "punch1.mid"] {
            assert_eq!(rolled(name), None, "{name}");
        }
        // punch.wav itself is in the map (never sent; the stock sounds have it).
        assert_eq!(msp_default("punch.wav").as_deref(), Some("bank/Natural attack"));
        let names = msp_names();
        assert_eq!(names.len(), slots().iter().map(|s| s.msp.len()).sum::<usize>() + 14);
        assert!(names.iter().all(|n| msp_default(n).is_some() || by_msp(n).is_some_and(|s| s.looped)), "every name plays something");
        // The hook editor's list says which each plays.
        let all = list();
        let hit = all.iter().find(|s| s.path == "bank/Flesh hit").unwrap();
        assert!(hit.msp.iter().any(|n| n == "punch4.wav") && hit.msp.iter().any(|n| n == "bashed1.wav"));
    }

    #[test]
    fn layers_are_the_maps() {
        for (name, with, _) in LAYERS {
            let s = slots().iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{name} isn't in the bank"));
            assert!(!s.looped, "{name}");
            assert_eq!(layers(s, Tier::Severe).len(), with.len(), "{name} lays a sound that isn't in the bank");
            // Each laid sound's own trigger names the sound it goes with
            // ("under A#1", "over C#4 or C#8"), by the key's note.
            for layer in layers(s, Tier::Severe) {
                let note = {
                    const NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
                    format!("{}{}", NOTES[usize::from(s.key % 12)], s.key / 12 - 1)
                };
                assert!(layer.trigger.contains(&note), "{} doesn't say it goes with {name} ({note})", layer.name);
            }
        }
        let tier_of = |name: &str| find(name).unwrap();
        let (grunt, _) = tier_of("bank/Pain grunt");
        assert!(layers(grunt, Tier::Normal).is_empty());
        assert_eq!(layers(grunt, Tier::Severe).iter().map(|s| s.name).collect::<Vec<_>>(), ["Heavy damage thud"]);
        let (dragon, _) = tier_of("bank/Dragon");
        assert!(layers(dragon, Tier::Light).is_empty() && layers(dragon, Tier::Normal).len() == 1);
        let (fire, _) = tier_of("bank/Fire impact");
        assert_eq!(layers(fire, Tier::Severe).iter().map(|s| s.name).collect::<Vec<_>>(), ["Explosion sub", "Flame crackle"]);
        assert!(layers(tier_of("bank/Door open").0, Tier::Severe).is_empty());
    }

    #[test]
    fn events_have_their_sounds() {
        for h in Happening::ALL {
            let (path, _) = event_default(h).unwrap_or_else(|| panic!("{} plays nothing", h.name()));
            assert!(find(&path).is_some_and(|(s, _)| !s.looped), "{path}");
        }
        assert_eq!(event_default(Happening::Arrives(Manner::Walks)), Some(("bank/Footsteps in".into(), 100)));
        assert_eq!(event_default(Happening::Buy).map(|(p, _)| p).as_deref(), Some("bank/Coins"));
        // Coming is louder than going, and creeping quieter than walking.
        for m in Manner::ALL {
            let (_, coming) = event_default(Happening::Arrives(m)).unwrap();
            let (_, going) = event_default(Happening::Leaves(m)).unwrap();
            assert!(going < coming, "{m:?}");
        }
        let walk = event_default(Happening::Arrives(Manner::Walks)).unwrap().1;
        assert!(event_default(Happening::Arrives(Manner::Creeps)).unwrap().1 < walk);
        assert_eq!(EVENT_SOUNDS.len(), Happening::ALL.len());
        assert_eq!(event_under(Happening::Leaves(Manner::Stomps)).as_deref(), Some("bank/Heavy impact"));
        assert_eq!(event_under(Happening::Leaves(Manner::Walks)), None);
        // Every creature's cry is in the bank, each of its three.
        for value in crate::bestiary::cry_values() {
            assert!(creature_default(&value).is_some(), "{value}");
        }
        assert_eq!(creature_default("beast comes").as_deref(), Some("bank/Feline (idle)"));
        assert_eq!(creature_default("canine fights").as_deref(), Some("bank/Canine"));
        assert_eq!(creature_default("undead dies").as_deref(), Some("bank/Undead (pain)"));
        let all = list();
        let posture = all.iter().find(|s| s.path == "bank/Posture").unwrap();
        assert_eq!(posture.events, ["you sit", "you stand"]);
    }

    #[test]
    fn ambience_defaults_are_loops() {
        use crate::ambient::{room_type, Ambience, Loop};
        // Every terrain CoffeeMUD lists has one, and every weather but clear.
        for (terrain, _) in TERRAIN_LOOPS {
            assert!(room_type(terrain).is_some(), "{terrain} isn't a terrain");
            let path = bgn_default(terrain, false).unwrap_or_else(|| panic!("{terrain} has no loop"));
            assert!(find(&path).is_some_and(|(s, _)| s.looped), "{path}");
        }
        for (weather, _) in WEATHER_LOOPS {
            let path = bgw_default(weather).unwrap_or_else(|| panic!("{weather} has no loop"));
            assert!(find(&path).is_some_and(|(s, _)| s.looped), "{path}");
        }
        assert_eq!(bgn_default("City", false).as_deref(), Some("bank/Town"));
        assert_eq!(bgn_default("woods", true).as_deref(), Some("bank/Forest by night"));
        assert_eq!(bgn_default("cave", true).as_deref(), Some("bank/Cave"));
        assert_eq!(bgn_default("nowhere", false), None);
        assert_eq!(bgw_default("clear"), None);
        assert_eq!(bgw_default("rain").as_deref(), Some("bank/Light rain"));

        // The player's own loops stay; only what's silent is filled.
        let own = Loop { path: "BGN/brook.ogg".into(), volume: 40 };
        let mut a = Ambience { bgn: Some(own.clone()), bgw: None, bgm: None };
        fill_ambience(&mut a, Some("woods"), Some("fog"), false, true);
        assert_eq!(a.bgn, Some(own), "the player's own, even in a fight");
        assert_eq!(a.bgw, Some(Loop { path: "bank/Fog".into(), volume: LOOP_VOLUME }));
        let mut a = Ambience::default();
        fill_ambience(&mut a, None, None, false, true);
        assert_eq!(a, Ambience::default(), "nowhere, nothing");
        // A fight takes the place's loop over, and gives it back.
        let mut a = Ambience::default();
        fill_ambience(&mut a, Some("city"), Some("rain"), false, true);
        assert_eq!(a.bgn.map(|l| l.path).as_deref(), Some("bank/Battle tension"));
        assert_eq!(a.bgw.map(|l| l.path).as_deref(), Some("bank/Light rain"));
        let mut a = Ambience::default();
        fill_ambience(&mut a, Some("city"), None, false, false);
        assert_eq!(a.bgn.map(|l| l.path).as_deref(), Some("bank/Town"));

        // The hook editor's list says where each plays.
        let all = list();
        let sound = |name: &str| all.iter().find(|s| s.path == format!("bank/{name}")).unwrap();
        assert_eq!(sound("Forest by night").terrains, ["woods"]);
        assert_eq!(sound("Underwater").terrains, ["underwater", "in_underwater"]);
        assert_eq!(sound("Snow").weathers, ["snow", "blizzard"]);
        assert!(sound("Battle tension").terrains.is_empty() && sound("Fire impact").weathers.is_empty());
    }

    #[test]
    fn tiers_sit_in_their_ranges() {
        let v: Vec<u8> = Tier::ALL.iter().map(|t| t.velocity()).collect();
        assert!((1..=63).contains(&v[0]) && (64..=107).contains(&v[1]) && (108..=127).contains(&v[2]));
    }

    /// The bank scripts/coffeemud-sf2-fetch.py puts in Resources, if it's here.
    fn bank_bytes() -> Option<Vec<u8>> {
        let file = path(&Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"));
        let bytes = std::fs::read(file).ok();
        if bytes.is_none() {
            eprintln!("coffeemud.sf2 isn't here (scripts/coffeemud-sf2-fetch.py): its tests are skipped.");
        }
        bytes
    }

    fn velocities(slot: &Slot) -> Vec<u8> {
        slot.tiers().iter().map(|t| t.velocity()).collect()
    }

    /// Every slot in the map has a sound of its own in the bank, at every
    /// velocity it's played at, looping just when the map says.
    #[test]
    fn every_slot_is_in_the_bank() {
        let Some(bytes) = bank_bytes() else { return };
        let font = SoundFont::from_bytes(&bytes).unwrap();
        for s in slots() {
            for vel in velocities(s) {
                let samples = samples_for(&font, s, vel);
                assert!(!samples.is_empty(), "{} ({} {}) has no sound at velocity {vel}", s.name, s.program, s.key);
                assert!(samples.iter().all(|i| font.samples[*i].has_loop() == s.looped), "{} loops when it shouldn't, or doesn't", s.name);
            }
        }
    }

    /// And each one sounds, played through Neumetik MIDI as Coupler will.
    #[test]
    fn every_slot_plays() {
        let Some(bytes) = bank_bytes() else { return };
        let bank = Arc::new(SoundBank::from_bytes(&bytes).unwrap());
        const RATE: u32 = 22_050;
        for s in slots() {
            for vel in velocities(s) {
                let mut synth = neumetik_midi::Synth::new(RATE);
                synth.set_sound_bank(Some(bank.clone()));
                synth.program_change(0, s.program);
                synth.note_on(0, s.key, vel);
                // Some build from nothing (the implosion): up to 4 s.
                let (mut left, mut right) = (vec![0.0; RATE as usize / 4], vec![0.0; RATE as usize / 4]);
                let sounds = (0..16).any(|_| {
                    synth.render(&mut left, &mut right);
                    left.iter().chain(&right).any(|x| x.abs() > 0.001)
                });
                assert!(sounds, "{} ({} {}) is silent at velocity {vel}", s.name, s.program, s.key);
            }
        }
    }

    #[test]
    fn the_version_is_the_fetched_one() {
        let fetch = include_str!("../../scripts/coffeemud-sf2-fetch.py");
        assert!(fetch.contains(&format!("TAG = \"coffeemud-sf2-v{VERSION}\"")), "VERSION isn't the fetch script's TAG");
    }

    #[test]
    fn every_sound_has_a_path_of_its_own() {
        let all = list();
        assert_eq!(all.len(), 210 + 2 * 20);
        assert_eq!(all.iter().filter(|s| s.looped).count(), 53);
        let paths: HashSet<String> = all.iter().map(|s| s.path.to_lowercase()).collect();
        assert_eq!(paths.len(), all.len());
        for sound in &all {
            let (slot, tier) = find(&sound.path).unwrap_or_else(|| panic!("{} isn't found", sound.path));
            assert_eq!(slot.asset_path(tier), sound.path);
            assert!(is_bank_path(&sound.path));
            // The frontend's title is all after the folder, its extension cut.
            let title = &sound.path[FOLDER.len() + 1..];
            assert!(!title.contains('.') && !title.contains('/'), "{title}");
        }
        assert_eq!(find("bank/flesh hit (LIGHT)").map(|(s, t)| (s.name, t)), Some(("Flesh hit", Tier::Light)));
        assert_eq!(find("bank/Flesh hit").map(|(_, t)| t), Some(Tier::Normal));
        assert_eq!(find("bank/Canine (idle)").map(|(s, t)| (s.name, t)), Some(("Canine", Tier::Light)));
        assert_eq!(find("bank/Canine (pain)").map(|(_, t)| t), Some(Tier::Severe));
        // Only a tiered sound has tiers, each of its own kind.
        assert_eq!(find("bank/Flesh hit (pain)"), None);
        assert_eq!(find("bank/Tavern (light)"), None);
        assert_eq!(find("bank/Tavern").map(|(s, _)| s.looped), Some(true));
        assert_eq!(find("bank/Arrow release").map(|(_, t)| t.velocity()), Some(127));
        assert_eq!(find("wav/Tavern"), None);
        assert!(!is_bank_path("bankrobber/x.wav") && !is_bank_path("wav/bank") && !is_bank_path("bank"));
    }

    /// A WAV's channels, rate and samples.
    fn read_wav(wav: &[u8]) -> (u16, u32, Vec<i16>) {
        assert_eq!(&wav[..4], b"RIFF");
        let channels = u16::from_le_bytes([wav[22], wav[23]]);
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        (channels, rate, wav[44..].as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect())
    }

    #[test]
    fn a_one_shot_plays_to_its_end_at_the_banks_level() {
        let Some(bytes) = bank_bytes() else { return };
        let bank = Arc::new(SoundBank::from_bytes(&bytes).unwrap());
        for name in ["bank/Flesh hit (light)", "bank/Flesh hit (severe)", "bank/Door open", "bank/Implosion", "bank/Level up", "bank/Fire impact", "bank/Electric impact", "bank/Critical hit", "bank/Dragon (pain)"] {
            let (slot, tier) = find(name).unwrap();
            let (channels, rate, samples) = read_wav(&render_once(&bank, slot, tier));
            assert_eq!(rate, RATE);
            let seconds = samples.len() as f32 / f32::from(channels) / rate as f32;
            assert!(seconds > 0.05 && seconds < LONGEST_SECONDS as f32, "{name}: {seconds} s");
            // Mixed to peak at -1 dBFS at most, and nothing made louder here.
            let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0) as f32 / 32768.0;
            assert!(peak > 0.02 && peak < 0.95, "{name}: peak {peak}");
            // Trimmed: it ends on sound, not seconds of silence.
            let tail = &samples[samples.len().saturating_sub(usize::from(channels))..];
            assert!(tail.iter().any(|s| s.unsigned_abs() > 0), "{name} ends in silence");
        }
    }

    #[test]
    fn a_loop_is_its_loop_and_nothing_else() {
        let Some(bytes) = bank_bytes() else { return };
        let font = SoundFont::from_bytes(&bytes).unwrap();
        for name in ["bank/Tavern", "bank/Heavy rain", "bank/Dread drone", "bank/Cave"] {
            let (slot, _) = find(name).unwrap();
            let (channels, rate, samples) = read_wav(&loop_take(&font, slot).unwrap());
            assert!(matches!(channels, 1 | 2) && matches!(rate, 22_050 | 32_000), "{name}: {channels} channels at {rate}");
            let seconds = samples.len() as f32 / f32::from(channels) / rate as f32;
            assert!((5.0..=12.0).contains(&seconds), "{name}: {seconds} s");
        }
        assert!(loop_take(&font, find("bank/Door open").unwrap().0).is_err());
    }

    #[test]
    fn sounds_are_made_once_and_kept() {
        if bank_bytes().is_none() {
            return;
        }
        let root = std::env::temp_dir().join(format!("coupler-soundbank-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("0.0.1")).unwrap();
        let sounds = Sounds::default();
        assert!(!sounds.available());
        assert!(sounds.audio("bank/Tavern").is_err());
        sounds.set_dirs(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources"), &root);
        assert!(sounds.available());
        // An older bank's sounds are gone.
        assert!(!root.join("0.0.1").exists());
        let cache = root.join(format!("{VERSION}-{MADE}"));
        let tavern = sounds.audio("bank/Tavern").unwrap();
        // The first loop takes out every loop.
        let kept = std::fs::read_dir(&cache).unwrap().count();
        assert_eq!(kept, 53);
        assert_eq!(sounds.audio("bank/tavern").unwrap(), tavern);
        let hit = sounds.audio("bank/Flesh hit (light)").unwrap();
        assert_eq!(std::fs::read(cache.join("0-29-63.wav")).unwrap(), hit);
        assert_eq!(sounds.audio("bank/Flesh hit (light)").unwrap(), hit);
        assert!(sounds.audio("bank/Nothing").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}

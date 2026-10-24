//! Every fight, recorded for its **Combat Summary**: what the Fight panel
//! sees, kept by the second, then put in words to study afterwards. Pure
//! and unit-tested; shared with the web build. Each build feeds a
//! `Recorder` every read (the fight, the vitals, the lines), every
//! command sent and a clock tick, and keeps what it hands back in a
//! `Fights` book, saved as `fights/<world>.json` in app data.
//!
//! - **Blows** are read from the game's lines (`read_line`): CoffeeMUD's
//!   standard hit and miss strings (`lists.ini` `WEAPON_HIT_DESCS`,
//!   `WEAPON_MISS_DESCS`, `MISS_DESCS`), the dodges and parries
//!   (`Fighter_Dodge`, `Fighter_Parry` and kin), a death ("is DEAD!!!",
//!   `MUDFight.java`), a flight ("You flee north.", `MUDTracker.java`),
//!   experience ("You gain 25 experience points.", `CoffeeLevels.java`)
//!   and loot ("You get …").
//! - **Damage** is exact when the server shows it (`SHOWDAMAGE`: "slice
//!   (12)", `MUDFight.replaceDamageTag`; coffeemud.ini's default is NO).
//!   Otherwise it's measured: each fall of the foe's health (MSDP's hit
//!   points, every second) is shared among the blows landed on it since,
//!   by their hit words' ranges (`DAMAGE_WORDS` by
//!   `DAMAGE_WORDS_THRESHOLDS`: "cut" is 11 to 15); the player's own
//!   health (`char.vitals`) the same for the blows on them. Estimates are
//!   marked "about". A fall no blow explains is the player's last command
//!   (a spell, a kick) when one was sent just before, else something
//!   unseen. With no hit points to measure by (GMCP's percentage only), a
//!   blow is its word's middle.
//! - **Mana and moves** spent, and healing, are the vitals' changes,
//!   each put to the command just sent.
//! - A fight is kept from Combat mode's start to `AFTER_MS` past its end,
//!   for the death line, the experience and the loot that follow.
//!
//! - **Effects** gained and lost (GMCP `char.effects.add`, `.remove`) and
//!   the tank's turns (GMCP `group`), from `effects.rs`; what was on the
//!   player when it began, from its last AFFECTS.
//!
//! Spells' and skills' own lines aren't read yet: their damage is the
//! measured kind. Other players' names are in it, so it stays on this
//! machine (CLAUDE.md rule 1).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::combat::Opponent;
use crate::effects::Effect;
use crate::senses::{letters_and_digits, Vitals};

/// How long after a fight's end its lines still count: the death line
/// can follow MSDP's end, and the experience and loot follow the death.
pub const AFTER_MS: i64 = 10_000;
/// A command sent this long before a fall of health, a heal or mana
/// spent is what did it.
const USED_MS: i64 = 4_000;
/// A blow waits this long for a fall of health to share, then is its word's middle.
const PENDING_MS: i64 = 2_500;
/// Fights kept in full per character; older ones keep their overview.
pub const FULL: usize = 200;
/// Overviews kept per character.
const ALL: usize = 2_000;
/// Events kept per fight: a long siege is cut short, not the memory.
const MOST_EVENTS: usize = 5_000;
/// The health chart's size, in cells.
const CHART_COLUMNS: usize = 60;
const CHART_ROWS: usize = 4;

/// CoffeeMUD's `lists.ini`: the damage each hit word stands for.
const DAMAGE_WORDS_THRESHOLDS: [i64; 17] = [0, 3, 6, 10, 15, 25, 35, 50, 70, 100, 130, 165, 215, 295, 495, 795, i64::MAX];
const DAMAGE_WORDS: &str = "ALL,annoy(s), scratch(es),graze(s),wound(s),cut(s),damage(s),decimate(s),murder(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
NATURAL, annoy(s), scratch(es),graze(s),hit(s),cut(s),hurt(s),rip(s),crunch(es),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
SLASHING, annoy(s), scratch(es),graze(s),wound(s),cut(s),slice(s),gut(s),murder(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
PIERCING, annoy(s), scratch(es),graze(s),prick(s),cut(s),stab(s),pierce(s),murder(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
BASHING, annoy(s), scratch(es),graze(s),hit(s),smash(es),bash(es),crush(es),crunch(es),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
BURNING, annoy(s), warm(s),heat(s),singe(s),burn(s),flame(s),scorch(es),incinerate(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
SHOOTING, annoy(s), scratch(es),graze(s),hit(s),pierce(s),pierce(s),decimate(s),murder(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
MELTING, annoy(s), sting(s),sizzle(s),burn(s),scorch(es),dissolve(s),melt(s),melt(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
STRIKING, annoy(s), sting(s),charge(s),singe(s),burn(s),scorch(es),blast(s),incinerate(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
BURSTING, annoy(s), scratch(es),graze(s),wound(s),cut(s),damage(s),decimate(s),murder(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--;\
LASERING, light(s), poke(s),graze(s),puncture(s),drill(s),pierce(s),spear(s),murder(s),massacre(s), MASSACRE(S), destroy(s), DESTROY(S), obliterate(s), OBLITERATE(S), **OBLITERATE(S)**, --==::OBLITERATE(S)::==--";
/// The top word's damage has no end; an estimate stops here.
const TOP_DAMAGE: i64 = 999;

/// The verbs of the miss strings, both persons.
const MISS_VERBS: &[&str] = &["fire", "fires", "shoot", "shoots", "throw", "throws", "swing", "swings", "attack", "attacks", "lunge", "lunges", "thrust", "thrusts"];

/// The player, as an event names them.
pub const YOU: &str = "you";

/// Each hit word, as the game prints it to either person, and the least
/// and most damage it stands for (over every damage type that uses it).
fn hit_words() -> &'static BTreeMap<String, (i64, i64)> {
    static WORDS: OnceLock<BTreeMap<String, (i64, i64)>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let mut words: BTreeMap<String, (i64, i64)> = BTreeMap::new();
        for list in DAMAGE_WORDS.split(';') {
            for (k, word) in list.split(',').map(str::trim).enumerate().skip(1) {
                let least = if k == 1 { 0 } else { DAMAGE_WORDS_THRESHOLDS[k - 2] + 1 };
                let most = DAMAGE_WORDS_THRESHOLDS[k - 1].min(TOP_DAMAGE);
                let most = if k == DAMAGE_WORDS_THRESHOLDS.len() - 1 { TOP_DAMAGE } else { most };
                let (one, many) = match (word.find('('), word.find(')')) {
                    (Some(open), Some(close)) if open < close => (format!("{}{}", &word[..open], &word[close + 1..]), format!("{}{}{}", &word[..open], &word[open + 1..close], &word[close + 1..])),
                    _ => (word.to_string(), word.to_string()),
                };
                for form in [one, many] {
                    let range = words.entry(form).or_insert((least, most));
                    *range = (range.0.min(least), range.1.max(most));
                }
            }
        }
        words
    })
}

/// The damage a hit word stands for, least and most.
pub fn word_range(word: &str) -> Option<(i64, i64)> {
    hit_words().get(word).copied()
}

/// What a line of the fight says.
#[derive(Debug, Clone, PartialEq)]
pub enum Said {
    Hit { who: String, whom: String, word: String, damage: Option<i64>, with: String },
    Miss { who: String, whom: String, with: String },
    /// `who` dodged, parried or blocked `whom`'s attack.
    Defend { who: String, whom: String },
    Died { who: String },
    Fled { who: String },
    Xp(i64),
    Loot(String),
}

/// A name as the game prints it, as an event keeps it: "you" for the
/// player, an article lowercased ("A rat" is "a rat").
fn person(name: &str) -> String {
    let name = name.trim().trim_end_matches(['.', '!']).trim();
    let lower = name.to_lowercase();
    if lower == "you" || lower == "yourself" || lower == "your" {
        return YOU.to_string();
    }
    for article in ["A ", "An ", "The ", "Some "] {
        if let Some(rest) = name.strip_prefix(article) {
            return format!("{}{rest}", article.to_lowercase());
        }
    }
    name.to_string()
}

/// "Your swing" or "a rat's attack": whose it is.
fn owner(words: &str) -> Option<String> {
    let words = words.trim();
    if words.eq_ignore_ascii_case("your") || words.starts_with("Your ") || words.starts_with("your ") {
        return Some(YOU.to_string());
    }
    let at = words.rfind("'s ").or_else(|| words.strip_suffix("'s").map(str::len))?;
    Some(person(&words[..at]))
}

/// The line without its closing mark, if it has one.
fn closed(line: &str) -> Option<&str> {
    line.strip_suffix('.').or_else(|| line.strip_suffix('!')).map(str::trim_end)
}

/// A standard hit: "You slice a rat with a dagger.", "A rat bites (3)
/// you with its teeth.", "You fire an arrow at a rat and pierce it."
fn hit(line: &str) -> Option<Said> {
    let body = closed(line)?;
    let words: Vec<&str> = body.split(' ').collect();
    for (i, word) in words.iter().enumerate().skip(1) {
        if word_range(word).is_none() {
            continue;
        }
        let before = words[..i].join(" ");
        let mut rest = &words[i + 1..];
        let mut damage = None;
        if let Some(n) = rest.first().and_then(|w| w.strip_prefix('(')).and_then(|w| w.strip_suffix(')')).and_then(|n| n.parse().ok()) {
            damage = Some(n);
            rest = &rest[1..];
        }
        let after = rest.join(" ");
        // "<S> fire(s) <TOOL> at <T> and <DAMAGE> <T-HIM-HER>."
        if let Some(lead) = before.strip_suffix(" and") {
            let Some(verb) = lead.find(" fire ").or_else(|| lead.find(" fires ")).or_else(|| lead.find(" throw ")).or_else(|| lead.find(" throws ")) else { continue };
            let shot = &lead[verb + 1..];
            let shot = &shot[shot.find(' ')? + 1..];
            let Some(at) = shot.rfind(" at ") else { continue };
            return Some(Said::Hit { who: person(&lead[..verb]), whom: person(&shot[at + 4..]), word: word.to_string(), damage, with: shot[..at].trim().to_string() });
        }
        if after.is_empty() {
            continue;
        }
        // "<S> <DAMAGE> <T> with <TOOL>.", or with no tool at all.
        let (whom, with) = match after.find(" with ") {
            Some(at) => (&after[..at], after[at + 6..].trim()),
            None => (after.as_str(), ""),
        };
        return Some(Said::Hit { who: person(&before), whom: person(whom), word: word.to_string(), damage, with: with.to_string() });
    }
    None
}

/// A standard miss: "You swing at a rat with a dagger and miss.", "A rat
/// attacks you and misses.", "Your swing misses a rat."
fn miss(line: &str) -> Option<Said> {
    let body = closed(line)?;
    if let Some(body) = body.strip_suffix(" and miss").or_else(|| body.strip_suffix(" and misses")) {
        let words: Vec<&str> = body.split(' ').collect();
        let verb = words.iter().skip(1).position(|w| MISS_VERBS.contains(w))? + 1;
        let who = person(&words[..verb].join(" "));
        let after = words[verb + 1..].join(" ");
        let after = after.strip_prefix("at ").unwrap_or(&after);
        let (whom, with) = if let Some(at) = after.find(" with ") {
            (&after[..at], &after[at + 6..])
        } else if let Some(at) = after.find(" at ") {
            (&after[at + 4..], &after[..at])
        } else {
            (after, "")
        };
        return Some(Said::Miss { who, whom: person(whom), with: with.trim().to_string() });
    }
    for verb in [" misses ", " miss "] {
        if let Some(at) = body.find(verb) {
            let who = owner(&body[..at])?;
            return Some(Said::Miss { who, whom: person(&body[at + verb.len()..]), with: String::new() });
        }
    }
    None
}

/// A dodge, a parry or a block: "You dodge a rat's bite!", "A guard
/// parries your attack with a sword!", "You block the attack by a rat!"
fn defend(line: &str) -> Option<Said> {
    let body = closed(line)?;
    let words: Vec<&str> = body.split(' ').collect();
    let verb = words.iter().skip(1).position(|w| {
        let w = w.to_ascii_lowercase();
        w.starts_with("dodge") || w.starts_with("parr") || w.starts_with("circle-parr") || w.starts_with("block")
    })? + 1;
    let who = person(&words[..verb].join(" "));
    let rest = words[verb + 1..].join(" ");
    let whom = if let Some(at) = rest.find("attack by ").or_else(|| rest.find("attack from ")) {
        let by = &rest[at..];
        let by = &by[by.find(' ')? + 1..];
        let by = &by[by.find(' ')? + 1..];
        person(by.split(" with ").next()?)
    } else {
        let rest = rest.split(" with ").next()?;
        owner(rest).or_else(|| owner(&rest[..rest.rfind(' ')?]))?
    };
    Some(Said::Defend { who, whom })
}

/// What a line of the fight says, if it's one Coupler reads.
pub fn read_line(line: &str) -> Option<Said> {
    let line = line.trim();
    if line.is_empty() || line.len() > 240 {
        return None;
    }
    if line.contains("YOU ARE DEAD") || line.starts_with("You have been KILLED") {
        return Some(Said::Died { who: YOU.into() });
    }
    if let Some(at) = line.find(" is DEAD") {
        return Some(Said::Died { who: person(&line[..at]) });
    }
    if line.starts_with("You flee") {
        return Some(Said::Fled { who: YOU.into() });
    }
    if let Some(at) = line.find(" flees") {
        return Some(Said::Fled { who: person(&line[..at]) });
    }
    if let Some(rest) = line.strip_prefix("You gain ") {
        if rest.contains("experience point") {
            return rest.split(' ').next()?.replace(',', "").parse().ok().map(Said::Xp);
        }
    }
    if let Some(rest) = line.strip_prefix("You get ") {
        return Some(Said::Loot(closed(rest).unwrap_or(rest).to_string()));
    }
    miss(line).or_else(|| defend(line)).or_else(|| hit(line))
}

/// How a fight ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Ending {
    Killed,
    Fled,
    YouFled,
    YouDied,
    #[default]
    Over,
}

/// What happened, in an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum What {
    Hit,
    Miss,
    Defend,
    /// A command the player sent.
    Used,
    /// Health fell and no blow said why.
    Damage,
    Heal,
    Mana,
    Moves,
    /// The foe changed.
    Switch,
    Died,
    Fled,
    Xp,
    Loot,
    /// An effect came on the player, or wore off (`with` is its name).
    Gained,
    Lost,
    /// The foe turned on someone (`whom`; "you" for the player).
    Tank,
}

/// One moment of a fight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Since the fight began, in milliseconds.
    pub at: i64,
    pub what: What,
    /// Who did it ("you" for the player); empty when no one's known.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub who: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub whom: String,
    /// The weapon, the command used, or the loot.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub with: String,
    /// The game's hit word.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub word: String,
    /// Damage, healing, mana, moves or experience.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    /// The amount is the game's own figure, not an estimate.
    #[serde(default)]
    pub exact: bool,
    /// The game's line, when it was one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub line: String,
}

impl Event {
    fn new(at: i64, what: What) -> Event {
        Event { at, what, who: String::new(), whom: String::new(), with: String::new(), word: String::new(), amount: None, exact: false, line: String::new() }
    }
}

/// The fight's figures at one second.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    /// Since the fight began, in milliseconds.
    pub at: i64,
    pub hp: Option<i64>,
    pub max_hp: Option<i64>,
    pub mana: Option<i64>,
    pub max_mana: Option<i64>,
    pub moves: Option<i64>,
    pub max_moves: Option<i64>,
    /// The foe's health out of 100, and its hit points when MSDP sends them.
    pub foe: Option<i64>,
    pub foe_hp: Option<i64>,
    pub foe_max: Option<i64>,
}

/// A fight, as recorded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fight {
    /// When it began (Unix milliseconds) and how long it lasted.
    pub started: i64,
    pub length: i64,
    /// Every foe, in the order the player turned on them.
    pub foes: Vec<String>,
    pub ending: Ending,
    /// The room it began in.
    pub place: Option<String>,
    /// What was on the player when it began.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wearing: Vec<String>,
    pub events: Vec<Event>,
    pub samples: Vec<Sample>,
}

/// A fight being recorded.
struct Under {
    fight: Fight,
    foe: Option<String>,
    foe_hp: Option<i64>,
    /// When Combat mode ended; the fight is kept `AFTER_MS` later.
    ended: Option<i64>,
    /// Blows on the foe, and on the player, waiting for a fall of health.
    on_foe: Vec<usize>,
    on_you: Vec<usize>,
}

/// Records one fight after another.
#[derive(Default)]
pub struct Recorder {
    under: Option<Under>,
    vitals: Vitals,
    /// The player's last command, and when it was sent.
    used: Option<(String, i64)>,
}

/// One read of the game, as the recorder takes it.
pub struct Reading<'a> {
    pub opponent: Option<&'a Opponent>,
    pub vitals: Option<&'a Vitals>,
    pub lines: &'a [String],
    /// The room the player is in.
    pub place: Option<&'a str>,
    /// What's on the player (`effects.rs`).
    pub effects: &'a [Effect],
}

/// Shares a fall of health among the blows that caused it, by their
/// words; returns what's left that no blow explains.
fn share(events: &mut [Event], blows: &[usize], fall: i64) -> i64 {
    let exact: i64 = blows.iter().filter(|&&i| events[i].exact).filter_map(|&i| events[i].amount).sum();
    let guessed: Vec<usize> = blows.iter().copied().filter(|&i| !events[i].exact).collect();
    let rest = fall - exact;
    if guessed.is_empty() {
        return rest.max(0);
    }
    let ranges: Vec<(i64, i64)> = guessed.iter().map(|&i| word_range(&events[i].word).unwrap_or((0, TOP_DAMAGE))).collect();
    let middle = |(least, most): (i64, i64)| (least + most) as f64 / 2.0;
    let total: f64 = ranges.iter().map(|&r| middle(r)).sum();
    let mut given = 0;
    for (&i, &(least, most)) in guessed.iter().zip(&ranges) {
        let part = if rest <= 0 || total <= 0.0 { least } else { ((rest as f64 * middle((least, most)) / total).round() as i64).clamp(least, most) };
        events[i].amount = Some(part);
        given += part;
    }
    (rest - given).max(0)
}

/// A blow with no fall of health to share: its word's middle.
fn guess(event: &mut Event) {
    if event.amount.is_none() {
        let (least, most) = word_range(&event.word).unwrap_or((0, 0));
        event.amount = Some((least + most + 1) / 2);
    }
}

fn same(a: &str, b: &str) -> bool {
    !a.is_empty() && letters_and_digits(a) == letters_and_digits(b)
}

impl Under {
    fn at(&self, now: i64) -> i64 {
        (now - self.fight.started).max(0)
    }

    fn push(&mut self, event: Event) -> Option<usize> {
        if self.fight.events.len() >= MOST_EVENTS {
            return None;
        }
        self.fight.events.push(event);
        Some(self.fight.events.len() - 1)
    }

    fn is_foe(&self, name: &str) -> bool {
        self.fight.foes.iter().any(|f| same(f, name))
    }

    /// The blows still waiting, each its word's middle.
    fn settle(&mut self, now: Option<i64>) {
        let at = now.map(|n| self.at(n));
        for list in [&mut self.on_foe, &mut self.on_you] {
            list.retain(|&i| {
                let event = &mut self.fight.events[i];
                if at.is_none_or(|at| at - event.at >= PENDING_MS) {
                    guess(event);
                    false
                } else {
                    true
                }
            });
        }
    }

    /// Notes the figures at this second.
    fn sample(&mut self, now: i64, vitals: &Vitals, opponent: Option<&Opponent>) {
        let at = self.at(now);
        let mut sample = Sample { at, hp: vitals.hp, max_hp: vitals.max_hp, mana: vitals.mana, max_mana: vitals.max_mana, moves: vitals.moves, max_moves: vitals.max_moves, ..Sample::default() };
        let last = self.fight.samples.last();
        match opponent {
            Some(o) => {
                sample.foe = o.percent;
                sample.foe_hp = o.health;
                sample.foe_max = o.health_max;
            }
            None => {
                sample.foe = last.and_then(|s| s.foe);
                sample.foe_hp = last.and_then(|s| s.foe_hp);
                sample.foe_max = last.and_then(|s| s.foe_max);
            }
        }
        match self.fight.samples.last_mut() {
            Some(last) if last.at / 1000 == at / 1000 => *last = Sample { at: last.at, ..sample },
            Some(last) if *last == Sample { at: last.at, ..sample.clone() } => {}
            _ => self.fight.samples.push(sample),
        }
    }
}

impl Recorder {
    /// The player sent a command.
    pub fn sent(&mut self, command: &str, now: i64) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        self.used = Some((command.to_string(), now));
        if let Some(under) = self.under.as_mut().filter(|u| u.ended.is_none()) {
            let mut event = Event::new(under.at(now), What::Used);
            event.who = YOU.into();
            event.with = command.to_string();
            under.push(event);
        }
    }

    /// An effect came on the player (`gained`) or wore off, during a fight.
    pub fn effect(&mut self, gained: bool, name: &str, now: i64) {
        if let Some(under) = self.under.as_mut().filter(|u| u.ended.is_none()) {
            let mut event = Event::new(under.at(now), if gained { What::Gained } else { What::Lost });
            event.who = YOU.into();
            event.with = name.to_string();
            under.push(event);
        }
    }

    /// The foe turned on someone in the group ("you" for the player).
    pub fn tank(&mut self, whom: &str, now: i64) {
        if let Some(under) = self.under.as_mut().filter(|u| u.ended.is_none()) {
            let mut event = Event::new(under.at(now), What::Tank);
            event.whom = person(whom);
            under.push(event);
        }
    }

    /// The command just sent, if one was.
    fn used(&self, now: i64) -> String {
        self.used.as_ref().filter(|(_, at)| now - at <= USED_MS).map(|(c, _)| c.clone()).unwrap_or_default()
    }

    /// One read of the game. Returns a fight that's over and done.
    pub fn read(&mut self, reading: Reading, now: i64) -> Option<Fight> {
        let mut done = None;
        let ongoing = self.under.as_ref().is_some_and(|u| u.ended.is_none());
        if let Some(o) = reading.opponent {
            if !ongoing {
                done = self.finish();
                self.start(o, reading.vitals, reading.place, now);
                if let Some(under) = self.under.as_mut() {
                    under.fight.wearing = reading.effects.iter().map(|e| e.name.clone()).collect();
                }
            } else {
                self.turn_to(o, now);
            }
        }
        for line in reading.lines {
            self.line(line, now);
        }
        if let Some(o) = reading.opponent {
            self.foe_health(o, now);
        }
        if let Some(v) = reading.vitals {
            self.vitals(v, now);
        }
        let vitals = self.vitals.clone();
        if let Some(under) = self.under.as_mut() {
            if reading.opponent.is_none() && under.ended.is_none() {
                under.ended = Some(now);
                under.fight.length = under.at(now);
            }
            if under.ended.is_none() {
                under.sample(now, &vitals, reading.opponent);
            }
            under.settle(Some(now));
        }
        done.or_else(|| self.tick(now))
    }

    /// The fight is kept once `AFTER_MS` have passed since its end.
    pub fn tick(&mut self, now: i64) -> Option<Fight> {
        match self.under.as_ref()?.ended {
            Some(ended) if now - ended >= AFTER_MS => self.finish(),
            _ => None,
        }
    }

    /// The player left the game or their character: the fight's over and kept.
    pub fn leave(&mut self, now: i64) -> Option<Fight> {
        if let Some(under) = self.under.as_mut().filter(|u| u.ended.is_none()) {
            under.ended = Some(now);
            under.fight.length = under.at(now);
        }
        self.vitals = Vitals::default();
        self.used = None;
        self.finish()
    }

    fn start(&mut self, o: &Opponent, vitals: Option<&Vitals>, place: Option<&str>, now: i64) {
        if let Some(v) = vitals {
            self.vitals = v.clone();
        }
        let foe = o.name.clone();
        let fight = Fight { started: now, length: 0, foes: foe.iter().cloned().collect(), ending: Ending::Over, place: place.map(str::to_string), wearing: Vec::new(), events: Vec::new(), samples: Vec::new() };
        let mut under = Under { fight, foe, foe_hp: o.health, ended: None, on_foe: Vec::new(), on_you: Vec::new() };
        // The command that began it: KILL, or a spell.
        let used = self.used(now);
        if !used.is_empty() {
            let mut event = Event::new(0, What::Used);
            event.who = YOU.into();
            event.with = used;
            under.push(event);
        }
        under.sample(now, &self.vitals, Some(o));
        self.under = Some(under);
    }

    /// The fight goes on: a new foe, perhaps.
    fn turn_to(&mut self, o: &Opponent, now: i64) {
        let Some(under) = self.under.as_mut() else { return };
        let Some(name) = o.name.as_ref() else { return };
        if under.foe.as_deref() == Some(name.as_str()) {
            return;
        }
        let first = under.foe.is_none();
        under.foe = Some(name.clone());
        if !under.is_foe(name) {
            under.fight.foes.push(name.clone());
        }
        // MSDP's figures are the last foe's for a second.
        for &i in &under.on_foe {
            guess(&mut under.fight.events[i]);
        }
        under.on_foe.clear();
        under.foe_hp = None;
        if !first {
            let mut event = Event::new(under.at(now), What::Switch);
            event.whom = name.clone();
            under.push(event);
        }
    }

    fn foe_health(&mut self, o: &Opponent, now: i64) {
        let used = self.used(now);
        let Some(under) = self.under.as_mut().filter(|u| u.ended.is_none()) else { return };
        let Some(hp) = o.health else { return };
        let before = under.foe_hp.replace(hp);
        let Some(before) = before else { return };
        let at = under.at(now);
        let foe = under.foe.clone().unwrap_or_default();
        if hp < before {
            let blows = std::mem::take(&mut under.on_foe);
            let left = share(&mut under.fight.events, &blows, before - hp);
            if left > 0 {
                let mut event = Event::new(at, What::Damage);
                event.who = if used.is_empty() { String::new() } else { YOU.into() };
                event.whom = foe;
                event.with = used;
                event.amount = Some(left);
                under.push(event);
            }
        } else if hp > before {
            let mut event = Event::new(at, What::Heal);
            event.whom = foe;
            event.amount = Some(hp - before);
            under.push(event);
        }
    }

    fn vitals(&mut self, v: &Vitals, now: i64) {
        let before = std::mem::replace(&mut self.vitals, v.clone());
        let used = self.used(now);
        let Some(under) = self.under.as_mut().filter(|u| u.ended.is_none()) else { return };
        let at = under.at(now);
        if let (Some(was), Some(hp)) = (before.hp, v.hp) {
            if hp < was {
                let blows = std::mem::take(&mut under.on_you);
                let left = share(&mut under.fight.events, &blows, was - hp);
                if left > 0 {
                    let mut event = Event::new(at, What::Damage);
                    event.whom = YOU.into();
                    event.amount = Some(left);
                    under.push(event);
                }
            } else if hp > was {
                let mut event = Event::new(at, What::Heal);
                event.whom = YOU.into();
                event.with = used.clone();
                event.amount = Some(hp - was);
                under.push(event);
            }
        }
        for (what, was, now) in [(What::Mana, before.mana, v.mana), (What::Moves, before.moves, v.moves)] {
            if let (Some(was), Some(now)) = (was, now) {
                if now < was {
                    let mut event = Event::new(at, what);
                    event.who = YOU.into();
                    event.with = used.clone();
                    event.amount = Some(was - now);
                    under.push(event);
                }
            }
        }
    }

    fn line(&mut self, line: &str, now: i64) {
        let Some(under) = self.under.as_mut() else { return };
        let Some(said) = read_line(line) else { return };
        let after = under.ended.is_some();
        let at = under.at(now);
        let mut event = Event::new(at, What::Hit);
        event.line = line.trim().to_string();
        match said {
            // "You warm yourself": not a blow.
            Said::Hit { who, whom, word, damage, with } if !after && who != whom => {
                let on_foe = under.foe.as_deref().is_some_and(|f| same(f, &whom));
                let on_you = whom == YOU;
                event.exact = damage.is_some();
                event.amount = damage;
                (event.who, event.whom, event.word, event.with) = (who, whom, word, with);
                let Some(i) = under.push(event) else { return };
                if on_foe && under.foe_hp.is_some() {
                    under.on_foe.push(i);
                } else if on_you {
                    under.on_you.push(i);
                } else {
                    guess(&mut under.fight.events[i]);
                }
            }
            Said::Miss { who, whom, with } if !after => {
                event.what = What::Miss;
                (event.who, event.whom, event.with) = (who, whom, with);
                under.push(event);
            }
            Said::Defend { who, whom } if !after => {
                event.what = What::Defend;
                (event.who, event.whom) = (who, whom);
                under.push(event);
            }
            Said::Died { who } => {
                if who == YOU {
                    under.fight.ending = Ending::YouDied;
                } else if under.is_foe(&who) {
                    under.fight.ending = Ending::Killed;
                    // The killing blows took what health was left.
                    if under.foe.as_deref().is_some_and(|f| same(f, &who)) {
                        if let Some(left) = under.foe_hp.take() {
                            let blows = std::mem::take(&mut under.on_foe);
                            share(&mut under.fight.events, &blows, left);
                        }
                    }
                }
                event.what = What::Died;
                event.who = who;
                under.push(event);
            }
            Said::Fled { who } => {
                if who == YOU && under.fight.ending != Ending::YouDied {
                    under.fight.ending = Ending::YouFled;
                } else if under.is_foe(&who) && under.fight.ending == Ending::Over {
                    under.fight.ending = Ending::Fled;
                }
                event.what = What::Fled;
                event.who = who;
                under.push(event);
            }
            Said::Xp(n) => {
                event.what = What::Xp;
                event.who = YOU.into();
                event.amount = Some(n);
                event.exact = true;
                under.push(event);
            }
            // Loot is only what follows a death.
            Said::Loot(what) if after || under.fight.ending == Ending::Killed => {
                event.what = What::Loot;
                event.who = YOU.into();
                event.with = what;
                under.push(event);
            }
            _ => {}
        }
    }

    /// The fight as kept; None for a moment's flicker of Combat mode
    /// with nothing in it.
    fn finish(&mut self) -> Option<Fight> {
        let mut under = self.under.take()?;
        under.settle(None);
        let mut fight = under.fight;
        let blows = fight.events.iter().any(|e| matches!(e.what, What::Hit | What::Miss | What::Damage | What::Died | What::Fled));
        (blows || fight.length >= 2_000).then(|| {
            fight.events.sort_by_key(|e| e.at);
            fight
        })
    }
}

// ---- The fights kept ----

/// A fight in brief: the list's row, and what's kept of it once its
/// details are gone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub id: i64,
    pub started: i64,
    pub length: i64,
    pub foes: Vec<String>,
    /// The first foe's letters and digits: the fights with it before.
    pub foe_key: String,
    pub ending: Ending,
    pub place: Option<String>,
    pub xp: i64,
    /// Damage dealt and taken, in all.
    pub dealt: i64,
    pub taken: i64,
    /// Some of it is estimated.
    pub about: bool,
    /// The details are still kept.
    pub full: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Kept {
    character: String,
    overview: Overview,
    fight: Option<Fight>,
}

/// Every fight kept in a world, each marked with its character.
#[derive(Default, Serialize, Deserialize)]
pub struct Fights {
    next: i64,
    kept: Vec<Kept>,
    #[serde(skip)]
    changed: bool,
}

impl Fight {
    /// Damage given or taken by the player, and whether any is estimated.
    fn damage(&self, dealt: bool) -> (i64, bool) {
        let mut total = 0;
        let mut about = false;
        for e in &self.events {
            let counts = match e.what {
                What::Hit | What::Damage => if dealt { e.who == YOU } else { e.whom == YOU },
                _ => false,
            };
            if counts {
                total += e.amount.unwrap_or(0);
                about |= !e.exact;
            }
        }
        (total, about)
    }

    fn overview(&self, id: i64) -> Overview {
        let (dealt, about_dealt) = self.damage(true);
        let (taken, about_taken) = self.damage(false);
        Overview {
            id,
            started: self.started,
            length: self.length,
            foes: self.foes.clone(),
            foe_key: self.foes.first().map(|f| letters_and_digits(f)).unwrap_or_default(),
            ending: self.ending,
            place: self.place.clone(),
            xp: self.events.iter().filter(|e| e.what == What::Xp).filter_map(|e| e.amount).sum(),
            dealt,
            taken,
            about: about_dealt || about_taken,
            full: true,
        }
    }
}

impl Fights {
    pub fn from_json(json: &str) -> Result<Fights, String> {
        serde_json::from_str(json).map_err(|e| format!("The fights kept couldn't be read. ({e})"))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Changed since last asked: wants saving.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// Keeps a fight of `character`'s; the oldest lose their details past `FULL`.
    pub fn keep(&mut self, character: &str, fight: Fight) -> Overview {
        self.next += 1;
        let overview = fight.overview(self.next);
        self.kept.push(Kept { character: character.to_string(), overview: overview.clone(), fight: Some(fight) });
        let mut full = 0;
        let mut all = 0;
        for kept in self.kept.iter_mut().rev().filter(|k| k.character == character) {
            all += 1;
            if kept.fight.is_some() {
                full += 1;
                if full > FULL {
                    kept.fight = None;
                    kept.overview.full = false;
                }
            }
        }
        if all > ALL {
            let mut drop = all - ALL;
            self.kept.retain(|k| {
                let gone = drop > 0 && k.character == character;
                if gone {
                    drop -= 1;
                }
                !gone
            });
        }
        self.changed = true;
        overview
    }

    /// `character`'s fights, the latest first.
    pub fn list(&self, character: &str) -> Vec<Overview> {
        self.kept.iter().rev().filter(|k| k.character == character).map(|k| k.overview.clone()).collect()
    }

    /// One fight's Combat Summary.
    pub fn page(&self, character: &str, id: i64) -> Option<Page> {
        let kept = self.kept.iter().find(|k| k.character == character && k.overview.id == id)?;
        let before: Vec<&Overview> = self
            .kept
            .iter()
            .filter(|k| k.character == character && k.overview.id != id && k.overview.started < kept.overview.started && !kept.overview.foe_key.is_empty() && k.overview.foe_key == kept.overview.foe_key)
            .map(|k| &k.overview)
            .collect();
        Some(page(&kept.overview, kept.fight.as_ref(), &before))
    }
}

// ---- The Combat Summary, in words ----

/// A heading and its lines.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub heading: String,
    pub lines: Vec<String>,
    /// Charts drawn under the lines, each with its name.
    pub charts: Vec<Chart>,
}

/// A line of health over time, in rows of cells, top first.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chart {
    pub name: String,
    pub rows: Vec<String>,
}

/// One fight's Combat Summary: one flat page of headings.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub overview: Overview,
    pub sections: Vec<Section>,
}

/// "0:42", "12:05".
pub fn clock(ms: i64) -> String {
    let seconds = ms.max(0) / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn capital(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn about(exact: bool) -> &'static str {
    if exact {
        ""
    } else {
        "about "
    }
}

/// A list in words: "a, b and c".
fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The ending in words.
fn ending_words(o: &Overview) -> String {
    let foe = o.foes.first().map(String::as_str).unwrap_or("your foe");
    match o.ending {
        Ending::Killed => format!("You won: {foe} is dead."),
        Ending::Fled => format!("{} fled.", capital(foe)),
        Ending::YouFled => "You fled.".into(),
        Ending::YouDied => "You died.".into(),
        Ending::Over => "The fight ended.".into(),
    }
}

/// Blows by one hand: hits, misses, damage, the biggest.
#[derive(Default)]
struct Tally {
    hits: i64,
    misses: i64,
    damage: i64,
    biggest: i64,
    exact: bool,
    any: bool,
}

impl Tally {
    fn add(&mut self, e: &Event) {
        match e.what {
            What::Miss => self.misses += 1,
            _ => {
                if e.what == What::Hit {
                    self.hits += 1;
                }
                let n = e.amount.unwrap_or(0);
                self.damage += n;
                self.biggest = self.biggest.max(n);
                self.exact = if self.any { self.exact && e.exact } else { e.exact };
                self.any = true;
            }
        }
    }

    fn words(&self, name: &str) -> String {
        let mut parts = Vec::new();
        if self.hits > 0 || self.misses > 0 {
            parts.push(format!("{}, {}", plural(self.hits, "hit", "hits"), plural(self.misses, "miss", "misses")));
        }
        if self.any {
            let mut damage = format!("{}{} damage", about(self.exact), self.damage);
            if self.hits > 1 {
                damage.push_str(&format!("; biggest {}, average {}", self.biggest, (self.damage as f64 / self.hits as f64).round() as i64));
            }
            parts.push(damage);
        }
        format!("{}: {}.", capital(name), parts.join(", "))
    }
}

/// Tallies by a key, in the order first met.
fn tally_by<'a>(events: impl Iterator<Item = &'a Event>, key: impl Fn(&Event) -> String) -> Vec<(String, Tally)> {
    let mut tallies: Vec<(String, Tally)> = Vec::new();
    for e in events {
        let k = key(e);
        match tallies.iter_mut().find(|(name, _)| *name == k) {
            Some((_, t)) => t.add(e),
            None => {
                let mut t = Tally::default();
                t.add(e);
                tallies.push((k, t));
            }
        }
    }
    tallies
}

fn section(heading: &str, lines: Vec<String>) -> Section {
    Section { heading: heading.into(), lines, charts: Vec::new() }
}

/// The page: what's left of a fight whose details are gone is its overview.
pub fn page(o: &Overview, fight: Option<&Fight>, before: &[&Overview]) -> Page {
    let mut sections = vec![overview_section(o, fight)];
    if let Some(f) = fight {
        sections.push(dealt_section(f));
        sections.push(taken_section(f));
        sections.push(spent_section(f));
        sections.push(effects_section(f));
        if let Some(group) = group_section(f) {
            sections.push(group);
        }
        sections.push(health_section(f));
        sections.push(timeline_section(f));
    }
    sections.push(before_section(o, before));
    Page { overview: o.clone(), sections }
}

fn overview_section(o: &Overview, fight: Option<&Fight>) -> Section {
    let mut lines = vec![format!("Against {}.", listed(&o.foes)), ending_words(o), format!("It lasted {}.", clock(o.length))];
    if let Some(place) = &o.place {
        lines.push(format!("In {place}."));
    }
    let a = about(!o.about);
    lines.push(format!("You dealt {a}{} damage and took {a}{}.", o.dealt, o.taken));
    if o.xp > 0 {
        lines.push(format!("You gained {}.", plural(o.xp, "experience point", "experience points")));
    }
    if let Some(f) = fight {
        let loot: Vec<String> = f.events.iter().filter(|e| e.what == What::Loot).map(|e| e.with.clone()).collect();
        if !loot.is_empty() {
            lines.push(format!("You took {}.", listed(&loot)));
        }
    } else {
        lines.push("Only this much is kept of a fight this old.".into());
    }
    section("Overview", lines)
}

fn dealt_section(f: &Fight) -> Section {
    let yours: Vec<&Event> = f.events.iter().filter(|e| e.who == YOU && matches!(e.what, What::Hit | What::Miss | What::Damage)).collect();
    let mut lines = Vec::new();
    if yours.is_empty() {
        lines.push("You landed nothing that Coupler saw.".into());
        return section("Damage dealt", lines);
    }
    let mut all = Tally::default();
    yours.iter().for_each(|e| all.add(e));
    let seconds = (f.length as f64 / 1000.0).max(1.0);
    let a = about(all.exact);
    lines.push(format!("In all: {a}{} damage, {}, {}; {a}{:.1} a second.", all.damage, plural(all.hits, "hit", "hits"), plural(all.misses, "miss", "misses"), all.damage as f64 / seconds));
    for (with, t) in tally_by(yours.iter().copied().filter(|e| e.what != What::Damage), |e| if e.with.is_empty() { "Your bare hands".into() } else { e.with.clone() }) {
        lines.push(t.words(&with));
    }
    for (with, t) in tally_by(yours.iter().copied().filter(|e| e.what == What::Damage), |e| e.with.clone()) {
        lines.push(format!("{}: {}{} damage, from the foe's health falling.", capital(&with), about(t.exact), t.damage));
    }
    let unseen: i64 = f.events.iter().filter(|e| e.what == What::Damage && e.who.is_empty() && e.whom != YOU).filter_map(|e| e.amount).sum();
    if unseen > 0 {
        lines.push(format!("Something unseen: about {unseen} more of the foe's health."));
    }
    let defended = f.events.iter().filter(|e| e.what == What::Defend && e.whom == YOU).count() as i64;
    if defended > 0 {
        lines.push(format!("Dodged, parried or blocked: {} of your attacks.", defended));
    }
    section("Damage dealt", lines)
}

fn taken_section(f: &Fight) -> Section {
    let on_you: Vec<&Event> = f.events.iter().filter(|e| e.whom == YOU && matches!(e.what, What::Hit | What::Miss | What::Damage)).collect();
    let mut lines = Vec::new();
    if on_you.is_empty() {
        lines.push("Nothing touched you.".into());
    } else {
        let mut all = Tally::default();
        on_you.iter().for_each(|e| all.add(e));
        lines.push(format!("In all: {}{} damage, {}, {}.", about(all.exact), all.damage, plural(all.hits, "hit", "hits"), plural(all.misses, "miss", "misses")));
        lines.push("By attacker:".into());
        for (who, t) in tally_by(on_you.iter().copied(), |e| if e.who.is_empty() { "Something unseen".into() } else { e.who.clone() }) {
            lines.push(t.words(&who));
        }
        let attacks: Vec<(String, Tally)> = tally_by(on_you.iter().copied().filter(|e| !e.with.is_empty()), |e| e.with.clone());
        if !attacks.is_empty() {
            lines.push("By attack:".into());
            for (with, t) in attacks {
                lines.push(t.words(&with));
            }
        }
    }
    let defended = f.events.iter().filter(|e| e.what == What::Defend && e.who == YOU).count() as i64;
    if defended > 0 {
        lines.push(format!("You dodged, parried or blocked {}.", plural(defended, "attack", "attacks")));
    }
    let healed: Vec<&Event> = f.events.iter().filter(|e| e.what == What::Heal && e.whom == YOU).collect();
    if !healed.is_empty() {
        let total: i64 = healed.iter().filter_map(|e| e.amount).sum();
        let by: Vec<String> = tally_by(healed.iter().copied().filter(|e| !e.with.is_empty()), |e| e.with.clone()).into_iter().map(|(w, _)| w).collect();
        lines.push(format!("You healed {total}{}.", if by.is_empty() { String::new() } else { format!(", after {}", listed(&by)) }));
    }
    section("Damage taken", lines)
}

fn spent_section(f: &Fight) -> Section {
    let mut lines = Vec::new();
    let (first, last) = (f.samples.first(), f.samples.last());
    type Figure = fn(&Sample) -> Option<i64>;
    let kinds: [(&str, Figure, Figure, What); 2] = [("Mana", |s| s.mana, |s| s.max_mana, What::Mana), ("Moves", |s| s.moves, |s| s.max_moves, What::Moves)];
    for (name, now, most, what) in kinds {
        let spent: i64 = f.events.iter().filter(|e| e.what == what).filter_map(|e| e.amount).sum();
        let (start, end) = (first.and_then(now), last.and_then(now));
        let of = last.and_then(most).map(|m| format!(" of {m}")).unwrap_or_default();
        match (start, end) {
            (Some(s), Some(e)) => lines.push(format!("{name}: {s}{of} at the start, {e} at the end; {spent} spent.")),
            _ => lines.push(format!("{name}: {spent} spent.")),
        }
    }
    // Each command: how often, and the mana it cost.
    let mut uses: Vec<(String, i64, i64)> = Vec::new();
    for e in &f.events {
        let slot = match e.what {
            What::Used | What::Mana if !e.with.is_empty() => &e.with,
            _ => continue,
        };
        let i = match uses.iter().position(|(c, _, _)| c == slot) {
            Some(i) => i,
            None => {
                uses.push((slot.clone(), 0, 0));
                uses.len() - 1
            }
        };
        if e.what == What::Used {
            uses[i].1 += 1;
        } else {
            uses[i].2 += e.amount.unwrap_or(0);
        }
    }
    if !uses.is_empty() {
        lines.push("Your commands:".into());
        for (command, times, mana) in uses {
            let times = if times > 0 { plural(times, "time", "times") } else { "before the fight".into() };
            lines.push(format!("{}: {times}{}.", capital(&command), if mana > 0 { format!(", {mana} mana") } else { String::new() }));
        }
    }
    section("Mana and moves", lines)
}

/// What was on the player, came on and wore off.
fn effects_section(f: &Fight) -> Section {
    let mut lines = Vec::new();
    if f.wearing.is_empty() {
        lines.push("At the start: nothing that Coupler knew of.".into());
    } else {
        lines.push(format!("At the start: {}.", listed(&f.wearing)));
    }
    for e in f.events.iter().filter(|e| matches!(e.what, What::Gained | What::Lost)) {
        let how = if e.what == What::Gained { "came on you" } else { "wore off" };
        lines.push(format!("{}: {} {how}.", clock(e.at), capital(&e.with)));
    }
    if lines.len() == 1 {
        lines.push("Nothing came on you or wore off.".into());
    }
    section("Effects", lines)
}

/// Everyone else's blows: allies, and anyone else in the room.
fn group_section(f: &Fight) -> Option<Section> {
    let foe = |name: &str| f.foes.iter().any(|x| same(x, name));
    let others: Vec<&Event> = f.events.iter().filter(|e| matches!(e.what, What::Hit | What::Miss) && e.who != YOU && e.whom != YOU && !e.who.is_empty()).collect();
    let dealing: Vec<(String, Tally)> = tally_by(others.iter().copied().filter(|e| !foe(&e.who)), |e| e.who.clone());
    let taking: Vec<(String, Tally)> = tally_by(others.iter().copied().filter(|e| foe(&e.who)), |e| e.whom.clone());
    let tanks: Vec<&Event> = f.events.iter().filter(|e| e.what == What::Tank).collect();
    if dealing.is_empty() && taking.is_empty() && tanks.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    if !dealing.is_empty() {
        lines.push("Dealt:".into());
        lines.extend(dealing.iter().map(|(who, t)| t.words(who)));
    }
    if !taking.is_empty() {
        lines.push("Taken from the foe:".into());
        lines.extend(taking.iter().map(|(who, t)| t.words(who)));
    }
    if !tanks.is_empty() {
        let turns: Vec<String> = tanks.iter().map(|e| format!("{} at {}", if e.whom == YOU { "you".to_string() } else { capital(&e.whom) }, clock(e.at))).collect();
        lines.push(format!("The foe turned on {}.", listed(&turns)));
    }
    Some(section("Group", lines))
}

/// A share of health out of 100 at each column, from the samples.
fn chart(f: &Fight, value: impl Fn(&Sample) -> Option<i64>) -> Option<Vec<String>> {
    if f.samples.iter().all(|s| value(s).is_none()) {
        return None;
    }
    let seconds = (f.length / 1000 + 1).max(1) as usize;
    let columns = seconds.min(CHART_COLUMNS);
    let mut levels = Vec::with_capacity(columns);
    let mut last = None;
    let mut next = 0;
    for column in 0..columns {
        let until = ((column + 1) as i64 * f.length.max(1)) / columns as i64;
        while next < f.samples.len() && f.samples[next].at <= until {
            last = value(&f.samples[next]).or(last);
            next += 1;
        }
        levels.push(last.unwrap_or(0).clamp(0, 100));
    }
    let steps = (CHART_ROWS * 2) as i64;
    Some(
        (0..CHART_ROWS)
            .map(|row| {
                let full = (CHART_ROWS - row) as i64 * 2;
                levels
                    .iter()
                    .map(|&p| {
                        let level = (p * steps + 50) / 100;
                        if level >= full {
                            '█'
                        } else if level == full - 1 {
                            '▄'
                        } else {
                            ' '
                        }
                    })
                    .collect()
            })
            .collect(),
    )
}

fn health_section(f: &Fight) -> Section {
    let mut lines = Vec::new();
    let percent = |now: Option<i64>, most: Option<i64>| match (now, most) {
        (Some(n), Some(m)) if m > 0 => Some(n * 100 / m),
        _ => None,
    };
    let you = |s: &Sample| percent(s.hp, s.max_hp);
    let foe = |s: &Sample| s.foe;
    // Health in figures: at the start, the lowest and when, at the end.
    let words = |name: &str, figure: &dyn Fn(&Sample) -> Option<i64>, show: &dyn Fn(&Sample) -> String| {
        let known: Vec<&Sample> = f.samples.iter().filter(|s| figure(s).is_some()).collect();
        let (Some(first), Some(last)) = (known.first(), known.last()) else { return None };
        let lowest = known.iter().copied().min_by_key(|s| figure(s))?;
        Some(format!("{name}: {} at the start, lowest {} at {}, {} at the end.", show(first), show(lowest), clock(lowest.at), show(last)))
    };
    let your_show = |s: &Sample| match (s.hp, s.max_hp) {
        (Some(h), Some(m)) => format!("{h} of {m}"),
        (Some(h), None) => h.to_string(),
        _ => String::new(),
    };
    let foe_show = |s: &Sample| match (s.foe_hp, s.foe_max) {
        (Some(h), Some(m)) => format!("{h} of {m}"),
        _ => s.foe.map(|p| format!("{p}%")).unwrap_or_default(),
    };
    let foe_name = capital(f.foes.first().map(String::as_str).unwrap_or("Your foe"));
    lines.extend(words("You", &|s: &Sample| s.hp, &your_show));
    lines.extend(words(&foe_name, &foe, &foe_show));
    if lines.is_empty() {
        lines.push("No health was sent during this fight.".into());
    }
    let mut charts = Vec::new();
    if let Some(rows) = chart(f, you) {
        charts.push(Chart { name: "You".into(), rows });
    }
    if let Some(rows) = chart(f, foe) {
        charts.push(Chart { name: foe_name.clone(), rows });
    }
    Section { heading: "Health over time".into(), lines, charts }
}

/// One event in a line of words.
fn moment(e: &Event) -> Option<String> {
    let amount = e.amount.unwrap_or(0);
    let text = match e.what {
        What::Hit if !e.line.is_empty() => {
            if e.exact || e.amount.is_none() {
                e.line.clone()
            } else {
                format!("{} (about {amount})", e.line)
            }
        }
        What::Miss | What::Defend | What::Died | What::Fled | What::Xp | What::Loot | What::Hit => e.line.clone(),
        What::Used => format!("You: {}", e.with),
        What::Damage => {
            let whose = if e.whom == YOU { "Your".to_string() } else { format!("{}'s", capital(&e.whom)) };
            format!("{whose} health fell {amount}{}.", if e.with.is_empty() { String::new() } else { format!(", after {}", e.with) })
        }
        What::Heal => format!("{} healed {amount}{}.", if e.whom == YOU { "You".to_string() } else { capital(&e.whom) }, if e.with.is_empty() { String::new() } else { format!(", after {}", e.with) }),
        What::Mana => format!("You spent {amount} mana{}.", if e.with.is_empty() { String::new() } else { format!(" on {}", e.with) }),
        What::Moves => format!("You spent {}{}.", plural(amount, "move", "moves"), if e.with.is_empty() { String::new() } else { format!(" on {}", e.with) }),
        What::Switch => format!("Now fighting {}.", e.whom),
        What::Gained => format!("{} came on you.", capital(&e.with)),
        What::Lost => format!("{} wore off.", capital(&e.with)),
        What::Tank => format!("The foe turned on {}.", if e.whom == YOU { "you".to_string() } else { capital(&e.whom) }),
    };
    (!text.is_empty()).then(|| format!("{} {text}", clock(e.at)))
}

fn timeline_section(f: &Fight) -> Section {
    let mut lines: Vec<String> = f.events.iter().filter_map(moment).collect();
    if lines.is_empty() {
        lines.push("Nothing was seen happen.".into());
    }
    section("Timeline", lines)
}

fn before_section(o: &Overview, before: &[&Overview]) -> Section {
    let foe = o.foes.first().map(String::as_str).unwrap_or("this foe");
    let mut lines = Vec::new();
    if before.is_empty() {
        lines.push(format!("Your first fight with {foe} that Coupler kept."));
        return section("Against this foe before", lines);
    }
    let n = before.len() as i64;
    let count = |e: Ending| before.iter().filter(|b| b.ending == e).count() as i64;
    let mut how = vec![format!("won {}", count(Ending::Killed))];
    for (e, word) in [(Ending::YouFled, "fled"), (Ending::YouDied, "died"), (Ending::Fled, "it fled")] {
        if count(e) > 0 {
            how.push(format!("{word} {}", count(e)));
        }
    }
    lines.push(format!("{} with {foe} before: {}.", capital(&plural(n, "fight", "fights")), listed(&how)));
    let length = before.iter().map(|b| b.length).sum::<i64>() / n;
    let taken = before.iter().map(|b| b.taken).sum::<i64>() / n;
    let quicker = if o.length <= length { format!("{} faster", clock(length - o.length)) } else { format!("{} slower", clock(o.length - length)) };
    let hurt = if o.taken <= taken { format!("{} less damage taken", taken - o.taken) } else { format!("{} more damage taken", o.taken - taken) };
    lines.push(format!("On average {} long, taking about {taken} damage. This one: {quicker}, {hurt}.", clock(length)));
    section("Against this foe before", lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit_of(line: &str) -> (String, String, String, Option<i64>, String) {
        match read_line(line) {
            Some(Said::Hit { who, whom, word, damage, with }) => (who, whom, word, damage, with),
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn hit_words_cover_every_form_and_range() {
        assert_eq!(word_range("annoy"), Some((0, 0)));
        assert_eq!(word_range("scratches"), Some((1, 3)));
        assert_eq!(word_range("cut"), Some((11, 15)));
        assert_eq!(word_range("massacres"), Some((51, 70)));
        assert_eq!(word_range("MASSACRES"), Some((71, 100)));
        assert_eq!(word_range("--==::OBLITERATE::==--"), Some((496, TOP_DAMAGE)));
        // Pierce is 11 to 25 shooting, 26 to 35 piercing.
        assert_eq!(word_range("pierces"), Some((11, 35)));
        assert_eq!(word_range("bites"), None);
    }

    #[test]
    fn standard_hits_are_read_in_both_persons() {
        assert_eq!(hit_of("You slice a rat with a dagger."), ("you".into(), "a rat".into(), "slice".into(), None, "a dagger".into()));
        assert_eq!(hit_of("A rat scratches you with its claws."), ("a rat".into(), "you".into(), "scratches".into(), None, "its claws".into()));
        // SHOWDAMAGE on.
        assert_eq!(hit_of("The orc MASSACRES (85) you with a club!").3, Some(85));
        assert_eq!(hit_of("You fire an arrow at a rat and pierce it.").4, "an arrow");
        assert_eq!(hit_of("You fire an arrow at a rat and pierce it.").1, "a rat");
        assert_eq!(hit_of("Bob **OBLITERATES** the dragon with a sword.").2, "**OBLITERATES**");
    }

    #[test]
    fn misses_defences_deaths_and_the_rest() {
        assert_eq!(read_line("You swing at a rat with a dagger and miss."), Some(Said::Miss { who: "you".into(), whom: "a rat".into(), with: "a dagger".into() }));
        assert_eq!(read_line("A rat attacks you and misses."), Some(Said::Miss { who: "a rat".into(), whom: "you".into(), with: String::new() }));
        assert_eq!(read_line("You shoot an arrow at a rat and miss."), Some(Said::Miss { who: "you".into(), whom: "a rat".into(), with: "an arrow".into() }));
        assert_eq!(read_line("A goblin's swing misses you."), Some(Said::Miss { who: "a goblin".into(), whom: "you".into(), with: String::new() }));
        assert_eq!(read_line("You dodge the attack by a rat!"), Some(Said::Defend { who: "you".into(), whom: "a rat".into() }));
        assert_eq!(read_line("A guard parries your attack with a sword!"), Some(Said::Defend { who: "a guard".into(), whom: "you".into() }));
        assert_eq!(read_line("A rat is DEAD!!!"), Some(Said::Died { who: "a rat".into() }));
        assert_eq!(read_line("!!!!!!!!!!!!!!YOU ARE DEAD!!!!!!!!!!!!!!"), Some(Said::Died { who: "you".into() }));
        assert_eq!(read_line("You flee north."), Some(Said::Fled { who: "you".into() }));
        assert_eq!(read_line("A rat flees west."), Some(Said::Fled { who: "a rat".into() }));
        assert_eq!(read_line("You gain 1,250 experience points."), Some(Said::Xp(1250)));
        assert_eq!(read_line("You get 12 gold coins from the corpse of a rat."), Some(Said::Loot("12 gold coins from the corpse of a rat".into())));
        assert_eq!(read_line("A rat is here."), None);
        assert_eq!(read_line("You say 'hit him!'"), None);
    }

    fn rat(health: i64) -> Opponent {
        Opponent { name: Some("a rat".into()), health: Some(health), health_max: Some(40), percent: Some(health * 100 / 40), range: Some(0) }
    }

    fn vitals(hp: i64, mana: i64) -> Vitals {
        Vitals { hp: Some(hp), max_hp: Some(100), mana: Some(mana), max_mana: Some(50), moves: Some(80), max_moves: Some(80) }
    }

    fn read(r: &mut Recorder, o: Option<&Opponent>, v: Option<&Vitals>, lines: &[&str], now: i64) -> Option<Fight> {
        let lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        r.read(Reading { opponent: o, vitals: v, lines: &lines, place: Some("A Sewer"), effects: &[] }, now)
    }

    /// A captured shape of fight: KILL, three blows, a spell, the death,
    /// the experience and the loot.
    fn a_fight() -> Fight {
        let mut r = Recorder::default();
        r.sent("kill rat", 0);
        assert!(read(&mut r, Some(&rat(40)), Some(&vitals(100, 50)), &[], 0).is_none());
        // Two hits in one second: the 18 lost is shared by their words (cut 11-15, graze 4-6).
        read(&mut r, Some(&rat(40)), None, &["You cut a rat with a dagger.", "You graze a rat with a dagger.", "A rat bites you."], 1_000);
        read(&mut r, Some(&rat(22)), Some(&vitals(95, 50)), &["A rat grazes you with its teeth.", "You swing at a rat with a dagger and miss."], 2_000);
        r.sent("cast magic missile", 2_500);
        read(&mut r, Some(&rat(10)), Some(&vitals(95, 40)), &[], 3_000);
        read(&mut r, Some(&rat(10)), None, &["You wound a rat with a dagger.", "A rat is DEAD!!!", "You gain 25 experience points."], 4_000);
        assert!(read(&mut r, None, None, &[], 4_100).is_none());
        read(&mut r, None, None, &["You get a rat's tail from the corpse of a rat."], 6_000);
        assert!(r.tick(4_100 + AFTER_MS - 1).is_none());
        r.tick(4_100 + AFTER_MS).expect("kept once the moment after is over")
    }

    #[test]
    fn a_fight_is_recorded_blow_by_blow() {
        let f = a_fight();
        assert_eq!((f.ending, f.length, f.foes.clone(), f.place.as_deref()), (Ending::Killed, 4_100, vec!["a rat".to_string()], Some("A Sewer")));
        let damage = |what: What, word: &str| f.events.iter().find(|e| e.what == what && e.word == word).and_then(|e| e.amount);
        // 18 shared: cut's middle 13 to graze's 5, within their ranges.
        let (cut, graze) = (damage(What::Hit, "cut").unwrap(), damage(What::Hit, "graze").unwrap());
        assert_eq!(cut + graze, 18);
        assert!((11..=15).contains(&cut) && (4..=6).contains(&graze));
        // The spell's 12, by the health that fell with no blow.
        let spell = f.events.iter().find(|e| e.what == What::Damage).unwrap();
        assert_eq!((spell.amount, spell.with.as_str(), spell.who.as_str()), (Some(12), "cast magic missile", YOU));
        // The killing blow took the 10 left.
        assert_eq!(damage(What::Hit, "wound"), Some(10));
        // The 5 lost to one graze, the bite not being a hit word.
        assert_eq!(damage(What::Hit, "grazes"), Some(5));
        assert_eq!(f.events.iter().find(|e| e.what == What::Mana).map(|e| (e.amount, e.with.as_str())), Some((Some(10), "cast magic missile")));
        assert!(f.events.iter().any(|e| e.what == What::Loot));
        let o = f.overview(1);
        assert_eq!((o.dealt, o.taken, o.xp, o.about), (40, 5, 25, true));
    }

    #[test]
    fn exact_damage_is_kept_as_shown() {
        let mut r = Recorder::default();
        read(&mut r, Some(&rat(40)), Some(&vitals(100, 50)), &[], 0);
        read(&mut r, Some(&rat(40)), None, &["You cut (14) a rat with a dagger.", "You graze a rat with a dagger."], 500);
        read(&mut r, Some(&rat(20)), None, &[], 1_000);
        let f = r.leave(2_000).unwrap();
        let hits: Vec<(Option<i64>, bool)> = f.events.iter().filter(|e| e.what == What::Hit).map(|e| (e.amount, e.exact)).collect();
        assert_eq!(hits, vec![(Some(14), true), (Some(6), false)]);
        assert_eq!(f.ending, Ending::Over);
    }

    #[test]
    fn without_hit_points_a_blow_is_its_words_middle() {
        let mut r = Recorder::default();
        let o = Opponent { name: Some("an orc".into()), health: None, health_max: None, percent: Some(80), range: None };
        read(&mut r, Some(&o), None, &["You cut an orc with a sword."], 0);
        read(&mut r, Some(&o), None, &[], 3_000);
        let f = r.leave(4_000).unwrap();
        assert_eq!(f.events.iter().find(|e| e.what == What::Hit).unwrap().amount, Some(13));
    }

    #[test]
    fn a_new_fight_keeps_the_last_at_once_and_a_flicker_isnt_kept() {
        let mut r = Recorder::default();
        read(&mut r, Some(&rat(40)), None, &[], 0);
        read(&mut r, None, None, &[], 500);
        // Nothing happened in half a second: not a fight worth keeping.
        assert!(r.tick(500 + AFTER_MS).is_none());
        read(&mut r, Some(&rat(40)), None, &["You cut a rat with a dagger."], 20_000);
        read(&mut r, None, None, &[], 21_000);
        let kept = read(&mut r, Some(&rat(40)), None, &[], 22_000);
        assert!(kept.is_some(), "the new fight hands back the last one");
    }

    #[test]
    fn a_switch_and_fleeing_are_told() {
        let mut r = Recorder::default();
        read(&mut r, Some(&rat(40)), None, &[], 0);
        let orc = Opponent { name: Some("an orc".into()), ..rat(30) };
        read(&mut r, Some(&orc), None, &["You flee north."], 1_000);
        let f = r.leave(1_500).unwrap();
        assert_eq!(f.foes, vec!["a rat".to_string(), "an orc".to_string()]);
        assert_eq!(f.ending, Ending::YouFled);
        assert!(f.events.iter().any(|e| e.what == What::Switch && e.whom == "an orc"));
    }

    #[test]
    fn the_book_keeps_details_for_the_latest_and_the_overview_for_all() {
        let mut book = Fights::default();
        for i in 0..(FULL as i64 + 3) {
            let mut f = a_fight();
            f.started = i;
            book.keep("joash", f);
        }
        book.keep("ann", a_fight());
        let list = book.list("joash");
        assert_eq!(list.len(), FULL + 3);
        assert!(list[0].full && !list[FULL].full);
        assert_eq!(book.list("ann").len(), 1);
        assert!(book.take_changed());
        let again = Fights::from_json(&book.to_json()).unwrap();
        assert_eq!(again.list("joash"), list);
        // An old one's page is its overview and the fights before.
        let old = again.page("joash", list[FULL + 2].id).unwrap();
        assert_eq!(old.sections.iter().map(|s| s.heading.as_str()).collect::<Vec<_>>(), vec!["Overview", "Against this foe before"]);
    }

    #[test]
    fn the_page_says_it_all_in_words() {
        let mut book = Fights::default();
        let mut earlier = a_fight();
        earlier.started = -100_000;
        earlier.length = 10_000;
        book.keep("joash", earlier);
        let o = book.keep("joash", a_fight());
        let page = book.page("joash", o.id).unwrap();
        let headings: Vec<&str> = page.sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, vec!["Overview", "Damage dealt", "Damage taken", "Mana and moves", "Effects", "Health over time", "Timeline", "Against this foe before"]);
        let text = |heading: &str| page.sections.iter().find(|s| s.heading == heading).unwrap().lines.join("\n");
        let overview = text("Overview");
        assert!(overview.contains("Against a rat.") && overview.contains("You won: a rat is dead.") && overview.contains("It lasted 0:04."), "{overview}");
        assert!(overview.contains("You gained 25 experience points.") && overview.contains("You took a rat's tail"), "{overview}");
        let dealt = text("Damage dealt");
        assert!(dealt.contains("A dagger: 3 hits, 1 miss"), "{dealt}");
        assert!(dealt.contains("Cast magic missile: about 12 damage"), "{dealt}");
        assert!(text("Damage taken").contains("A rat: 1 hit, 0 misses, about 5 damage."), "{}", text("Damage taken"));
        assert!(text("Mana and moves").contains("Mana: 50 of 50 at the start, 40 at the end; 10 spent."), "{}", text("Mana and moves"));
        assert!(text("Mana and moves").contains("Cast magic missile: 1 time, 10 mana."), "{}", text("Mana and moves"));
        let health = page.sections.iter().find(|s| s.heading == "Health over time").unwrap();
        assert!(health.lines[0].starts_with("You: 100 of 100 at the start, lowest 95 of 100"), "{:?}", health.lines);
        assert_eq!(health.charts.len(), 2);
        assert!(health.charts.iter().all(|c| c.rows.len() == CHART_ROWS && c.rows.iter().all(|r| r.chars().count() == 5)));
        assert!(text("Timeline").starts_with("0:00 You: kill rat\n0:01 You cut a rat with a dagger. (about "), "{}", text("Timeline"));
        assert!(text("Against this foe before").contains("1 fight with a rat before: won 1."), "{}", text("Against this foe before"));
        assert!(text("Against this foe before").contains("This one: 0:05 faster"), "{}", text("Against this foe before"));
    }

    #[test]
    fn allies_are_the_group() {
        let mut r = Recorder::default();
        read(&mut r, Some(&rat(40)), None, &["Bob cuts a rat with a sword.", "A rat grazes Bob with its teeth."], 0);
        let f = r.leave(3_000).unwrap();
        let group = group_section(&f).unwrap();
        assert_eq!(group.lines, vec!["Dealt:", "Bob: 1 hit, 0 misses, about 13 damage.", "Taken from the foe:", "Bob: 1 hit, 0 misses, about 5 damage."]);
    }

    #[test]
    fn effects_and_the_tank() {
        let mut r = Recorder::default();
        let lines: Vec<String> = Vec::new();
        let on = [Effect { name: "Shield spell".into(), ends_at: Some(60_000) }];
        r.read(Reading { opponent: Some(&rat(40)), vitals: None, lines: &lines, place: None, effects: &on }, 0);
        r.effect(true, "Poison", 2_000);
        r.tank("Bob", 3_000);
        r.effect(false, "Shield", 4_000);
        let f = r.leave(5_000).unwrap();
        assert_eq!(f.wearing, ["Shield spell"]);
        assert_eq!(effects_section(&f).lines, ["At the start: Shield spell.", "0:02: Poison came on you.", "0:04: Shield wore off."]);
        assert_eq!(group_section(&f).unwrap().lines, ["The foe turned on Bob at 0:03."]);
        assert!(timeline_section(&f).lines.contains(&"0:04 Shield wore off.".to_string()));
        // Outside a fight, nothing's recorded.
        r.effect(true, "Bless", 6_000);
        assert!(r.leave(7_000).is_none());
    }
}

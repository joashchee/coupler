//! The cast: every character the player has met in a world, NPC or
//! player, each with a voice of their own for Immersive mode, so who's
//! talking is heard before what they say. Pure and unit-tested;
//! `session.rs` feeds it GMCP and the talk, keeps one per world at
//! `cast/<world>.json` in app data, and sends each line of talk out with
//! its speaker's voice.
//!
//! - **Who's met**: `room.mobiles` `{"npcs":[{"an orc":"an orc.2"}]}`
//!   names the NPCs in the room, `room.players` `{"pcs":[{"Bob":"Bob the
//!   Bold"}]}` the players (each re-sent when its set changes;
//!   docs/coffeemud-gmcp.md, 4.1), and anyone heard talking
//!   (`comm.channel`) is met too. The player's own character (from
//!   `char.base`) is left out.
//! - **The key**: `comm.channel`'s `player` is the speaker's name with
//!   only letters and digits (`Say.java`, `gmcpSaySend`), so everyone is
//!   keyed that way, lowercased: "an orc" and "anorc" are one character.
//!   NPCs of the same name share a voice, as they share a name.
//! - **The voice** is made from the name when first met (`auto_voice`):
//!   a feminine or masculine voice from words like "queen" or "monk",
//!   lower for a giant, higher for a pixie, slower when old or undead,
//!   then a little of the name's own hash on top so no two sound alike.
//!   Without a word to go on, the hash chooses. The seed picks among
//!   every engine's voices (`src/lib/voice.ts`): the system's of that
//!   kind and all the bundled ones (`synth.rs`), pitched to suit, for
//!   the most different voices. Once the player changes a voice it's
//!   theirs (`edited`) and never made again.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::senses::{letters_and_digits, Talk, TalkKind};

pub const PITCH: (u16, u16) = (50, 200);
pub const RATE: (u16, u16) = (60, 150);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gender {
    Feminine,
    Masculine,
}

/// Whether a character is played by someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Who {
    Npc,
    Pc,
    /// Heard talking, never seen in a room.
    Unknown,
}

/// How a character sounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub gender: Gender,
    /// Percent of the voice's own pitch.
    pub pitch: u16,
    /// Percent of the player's speed (the Mixer's RATE).
    pub rate: u16,
    /// The engine chosen: `system` (the WebView's) or a bundled one
    /// (`synth::VOICES`); None lets `seed` choose from all of them.
    #[serde(default)]
    pub engine: Option<String>,
    /// A voice of that engine, by name (system) or ID (bundled); None
    /// lets `seed` choose within the engine.
    #[serde(default, alias = "systemVoice")]
    pub voice_name: Option<String>,
    /// From the name: picks among the system's voices, and never changes.
    pub seed: u32,
    /// Their lines are written in Heard but not spoken.
    pub quiet: bool,
}

/// One character met.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub key: String,
    /// The name as the game shows it.
    pub name: String,
    pub who: Who,
    /// Unix seconds.
    pub first_met: u64,
    pub last_met: u64,
    /// Lines of talk heard from them.
    pub lines: u32,
    pub voice: Voice,
    /// The player changed the voice: it's never made again.
    pub edited: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Cast {
    characters: BTreeMap<String, Character>,
    /// The player's own character, keyed: never in the cast.
    #[serde(skip)]
    own: String,
}

/// FNV-1a: the same number for a name in every version, unlike std's hasher.
fn hash(key: &str) -> u32 {
    key.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
}

const FEMININE: &[&str] = &[
    "woman", "women", "lady", "ladies", "queen", "princess", "girl", "maiden", "maid", "mother", "wife", "daughter", "sister", "nun",
    "priestess", "witch", "hag", "matron", "madam", "mistress", "duchess", "countess", "empress", "sorceress", "enchantress",
    "seamstress", "waitress", "hostess", "actress", "goddess", "baroness", "heiress", "lass", "grandmother", "granny", "aunt", "nymph",
    "dryad", "siren", "harpy", "banshee", "she", "her", "mrs", "miss", "bride", "widow", "crone", "medusa", "valkyrie", "succubus",
    "lamia", "mermaid", "barmaid", "milkmaid", "huntress", "lioness", "tigress", "vixen", "cow", "hen", "mare", "doe", "ewe",
];
const MASCULINE: &[&str] = &[
    "man", "men", "lord", "king", "prince", "boy", "sir", "father", "son", "brother", "monk", "friar", "duke", "baron", "emperor",
    "husband", "lad", "grandfather", "gramps", "uncle", "he", "his", "mr", "wizard", "warlock", "sorcerer", "gentleman", "nobleman",
    "fisherman", "groom", "abbot", "sultan", "pharaoh", "incubus", "satyr", "merman", "bull", "stallion", "ram", "buck", "rooster",
    "boar", "monsieur", "squire", "guardsman", "watchman", "huntsman", "headman", "alderman",
];
/// Big things: low and slow.
const HUGE: &[&str] = &[
    "giant", "ogre", "troll", "dragon", "golem", "titan", "bear", "minotaur", "demon", "devil", "ettin", "cyclops", "behemoth", "wyrm",
    "hydra", "elephant", "mammoth", "colossus", "juggernaut", "balor", "treant", "gorilla", "bull", "hulk", "giantess", "drake",
];
/// Rough voices: a little low.
const GRUFF: &[&str] = &[
    "orc", "orcs", "dwarf", "dwarves", "hobgoblin", "gnoll", "bugbear", "lizardman", "wolf", "boar", "barbarian", "brute", "thug",
    "bandit", "pirate", "sailor", "brigand", "mercenary", "blacksmith", "smith", "guard", "soldier", "warrior", "knight", "captain",
];
/// Little things: high and quick.
const SMALL: &[&str] = &[
    "child", "kid", "baby", "infant", "fairy", "faerie", "pixie", "sprite", "imp", "mouse", "rat", "squirrel", "bird", "sparrow", "kobold",
    "gnome", "halfling", "hobbit", "goblin", "kitten", "puppy", "bunny", "rabbit", "chick", "frog", "toad", "leprechaun", "brownie",
    "cherub", "urchin", "gremlin", "boy", "girl", "lad", "lass", "squire", "page", "monkey", "parrot", "bat", "cat",
];
const OLD: &[&str] = &[
    "old", "elder", "elderly", "ancient", "aged", "grandfather", "grandmother", "granny", "gramps", "crone", "hag", "sage", "hermit",
    "venerable", "wise", "abbot",
];
/// The dead and the uncanny: low and slow.
const UNDEAD: &[&str] = &[
    "ghost", "spirit", "wraith", "specter", "spectre", "skeleton", "zombie", "lich", "ghoul", "mummy", "vampire", "phantom", "shade",
    "revenant", "wight", "undead", "apparition", "poltergeist",
];

/// The words of a name, lowercased: "Bob the Bold" is bob, the, bold.
fn words(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

fn any(words: &[String], list: &[&str]) -> bool {
    words.iter().any(|w| list.contains(&w.as_str()))
}

/// A voice made from a name.
pub fn auto_voice(key: &str, name: &str) -> Voice {
    let seed = hash(key);
    let w = words(name);
    // "-man" and "-woman" words not on the lists (a fisherwoman, a
    // ferryman), but not "human" or "shaman".
    let suffixed = |ending: &str| w.iter().any(|x| x.len() > ending.len() + 1 && x.ends_with(ending) && x != "human" && x != "shaman");
    let gender = if any(&w, FEMININE) || suffixed("woman") || suffixed("women") {
        Gender::Feminine
    } else if any(&w, MASCULINE) || suffixed("man") || suffixed("men") {
        Gender::Masculine
    } else if seed & 1 == 0 {
        Gender::Feminine
    } else {
        Gender::Masculine
    };
    let (mut pitch, mut rate): (i32, i32) = match gender {
        Gender::Feminine => (108, 100),
        Gender::Masculine => (94, 100),
    };
    if any(&w, UNDEAD) {
        (pitch, rate) = (72, 75);
    } else if any(&w, HUGE) {
        (pitch, rate) = (62, 85);
    } else if any(&w, SMALL) {
        (pitch, rate) = (152, 115);
    } else if any(&w, GRUFF) {
        pitch -= 14;
        rate -= 5;
    }
    if any(&w, OLD) {
        pitch -= 10;
        rate -= 18;
    }
    // A little of the name's own: up to 12 either way for pitch, 8 for speed.
    pitch += ((seed >> 8) % 25) as i32 - 12;
    rate += ((seed >> 16) % 17) as i32 - 8;
    Voice {
        gender,
        pitch: pitch.clamp(PITCH.0.into(), PITCH.1.into()) as u16,
        rate: rate.clamp(RATE.0.into(), RATE.1.into()) as u16,
        engine: None,
        voice_name: None,
        seed,
        quiet: false,
    }
}

impl Cast {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut cast: Cast = serde_json::from_str(json).map_err(|e| format!("The saved voices couldn't be read. ({e})"))?;
        // Saved before there were engines: a voice by name was the system's.
        for c in cast.characters.values_mut() {
            if c.voice.engine.is_none() && c.voice.voice_name.is_some() {
                c.voice.engine = Some("system".into());
            }
        }
        Ok(cast)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("the cast is plain data")
    }

    /// Meets `name`. Returns whether they're new. A name heard only in
    /// talk (letters and digits) gives way to the one the room shows,
    /// and its voice is made again from it unless the player changed it.
    fn meet(&mut self, name: &str, who: Who, now: u64) -> bool {
        let name = name.trim();
        let key = letters_and_digits(name);
        if key.is_empty() || key == self.own {
            return false;
        }
        if let Some(c) = self.characters.get_mut(&key) {
            c.last_met = now;
            if who != Who::Unknown && c.who != who {
                let renamed = c.who == Who::Unknown && c.name != name;
                c.who = who;
                if renamed {
                    c.name = name.to_string();
                    if !c.edited {
                        c.voice = auto_voice(&key, name);
                    }
                }
            }
            return false;
        }
        let voice = auto_voice(&key, name);
        self.characters.insert(key.clone(), Character { key, name: name.to_string(), who, first_met: now, last_met: now, lines: 0, voice, edited: false });
        true
    }

    /// One GMCP message. Returns whether anyone new was met.
    pub fn gmcp(&mut self, package: &str, data: &str, now: u64) -> bool {
        let package = package.to_ascii_lowercase();
        let (list, who) = match package.as_str() {
            "room.mobiles" => ("npcs", Who::Npc),
            "room.players" => ("pcs", Who::Pc),
            "char.base" => {
                if let Ok(Value::Object(o)) = serde_json::from_str::<Value>(data) {
                    if let Some(name) = o.get("name").and_then(Value::as_str) {
                        self.own = letters_and_digits(name);
                    }
                }
                return false;
            }
            _ => return false,
        };
        // room.players' names aren't escaped: a broken one is skipped.
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(data) else { return false };
        let Some(Value::Array(entries)) = o.get(list) else { return false };
        let mut new = false;
        for entry in entries {
            if let Value::Object(pair) = entry {
                for name in pair.keys() {
                    new |= self.meet(name, who, now);
                }
            }
        }
        new
    }

    /// A line of talk: meets the speaker and gives their voice, or None
    /// for the player's own line or one with no speaker. The second is
    /// whether they're new.
    pub fn talk(&mut self, talk: &Talk, now: u64) -> (Option<Voice>, bool) {
        if talk.mine || talk.from.trim().is_empty() {
            return (None, false);
        }
        // Only players use the public channels; anyone can say or tell.
        let who = if talk.kind == TalkKind::Channel { Who::Pc } else { Who::Unknown };
        let new = self.meet(&talk.from, who, now);
        let key = letters_and_digits(&talk.from);
        let Some(c) = self.characters.get_mut(&key) else { return (None, new) };
        c.lines = c.lines.saturating_add(1);
        (Some(c.voice.clone()), new)
    }

    /// Someone met, by key.
    pub fn character(&self, key: &str) -> Option<&Character> {
        self.characters.get(key)
    }

    /// Everyone met, the most recent first.
    pub fn list(&self) -> Vec<Character> {
        let mut all: Vec<Character> = self.characters.values().cloned().collect();
        all.sort_by(|a, b| b.last_met.cmp(&a.last_met).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        all
    }

    /// The player's own voice for someone: kept in range, the seed theirs.
    pub fn set_voice(&mut self, key: &str, voice: Voice) -> Result<Character, String> {
        let c = self.characters.get_mut(key).ok_or("Coupler hasn't met that character.")?;
        let named = |s: Option<String>| s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let engine = named(voice.engine);
        if engine.as_deref().is_some_and(|e| e != "system" && !crate::synth::VOICES.iter().any(|v| v.engine == e)) {
            return Err("Coupler doesn't have that speech engine.".into());
        }
        // A voice belongs to an engine: none chosen, none named.
        let voice_name = if engine.is_some() { named(voice.voice_name) } else { None };
        c.voice = Voice {
            pitch: voice.pitch.clamp(PITCH.0, PITCH.1),
            rate: voice.rate.clamp(RATE.0, RATE.1),
            engine,
            voice_name,
            seed: c.voice.seed,
            ..voice
        };
        c.edited = true;
        Ok(c.clone())
    }

    /// Makes someone's voice from their name again.
    pub fn reset(&mut self, key: &str) -> Result<Character, String> {
        let c = self.characters.get_mut(key).ok_or("Coupler hasn't met that character.")?;
        c.voice = auto_voice(key, &c.name);
        c.edited = false;
        Ok(c.clone())
    }

    /// Forgets someone: met again, they get a voice made afresh.
    pub fn forget(&mut self, key: &str) -> bool {
        self.characters.remove(key).is_some()
    }

    /// The player left the game or their character.
    pub fn leave(&mut self) {
        self.own.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn talk(from: &str, kind: TalkKind, mine: bool) -> Talk {
        Talk { kind, channel: String::new(), from: from.into(), text: "hi".into(), mine }
    }

    #[test]
    fn rooms_meet_npcs_and_players_but_not_yourself() {
        let mut c = Cast::default();
        c.gmcp("char.base", r#"{"name":"Joash"}"#, 1);
        assert!(c.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc"},{"an orc":"an orc.2"},{"the city guard":"guard"}]}"#, 1));
        assert!(c.gmcp("Room.Players", r#"{"pcs":[{"Joash":"Joash the Brave"},{"Bob":"Bob the Bold"}]}"#, 2));
        let all = c.list();
        assert_eq!(all.len(), 3);
        assert_eq!((all[0].name.as_str(), all[0].who), ("Bob", Who::Pc));
        assert!(all.iter().any(|x| x.key == "anorc" && x.who == Who::Npc));
        // Seen again: nobody new, but met later.
        assert!(!c.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc"}]}"#, 5));
        assert_eq!(c.list()[0].key, "anorc");
    }

    #[test]
    fn broken_lists_are_skipped() {
        let mut c = Cast::default();
        assert!(!c.gmcp("room.players", r#"{"pcs":[{"Bo"b":"x"}]}"#, 1));
        assert!(!c.gmcp("room.mobiles", r#"{"npcs":"none"}"#, 1));
        assert!(c.list().is_empty());
    }

    #[test]
    fn talk_meets_the_speaker_and_gives_their_voice() {
        let mut c = Cast::default();
        c.gmcp("room.mobiles", r#"{"npcs":[{"an orc":"an orc"}]}"#, 1);
        let (voice, new) = c.talk(&talk("anorc", TalkKind::Say, false), 2);
        assert!(!new);
        assert_eq!(voice, Some(c.list()[0].voice.clone()));
        assert_eq!(c.list()[0].lines, 1);
        // Your own line, and a line with no speaker, have no voice.
        assert_eq!(c.talk(&talk("Joash", TalkKind::Say, true), 3), (None, false));
        assert_eq!(c.talk(&talk(" ", TalkKind::Say, false), 3), (None, false));
        // A stranger on a channel is a player.
        let (voice, new) = c.talk(&talk("Ann", TalkKind::Channel, false), 4);
        assert!(voice.is_some() && new);
        assert_eq!(c.list()[0].who, Who::Pc);
    }

    #[test]
    fn heard_first_then_seen_takes_the_room_name_and_a_new_voice() {
        let mut c = Cast::default();
        c.talk(&talk("theoldwitch", TalkKind::Say, false), 1);
        assert_eq!(c.list()[0].who, Who::Unknown);
        c.gmcp("room.mobiles", r#"{"npcs":[{"the old witch":"witch"}]}"#, 2);
        let witch = &c.list()[0];
        assert_eq!((witch.name.as_str(), witch.who), ("the old witch", Who::Npc));
        assert_eq!(witch.voice, auto_voice("theoldwitch", "the old witch"));
        assert_eq!(witch.voice.gender, Gender::Feminine);
    }

    #[test]
    fn voices_follow_the_name() {
        let queen = auto_voice("thequeen", "the Queen");
        let king = auto_voice("theking", "the King");
        assert_eq!((queen.gender, king.gender), (Gender::Feminine, Gender::Masculine));
        assert_eq!(auto_voice("afisherwoman", "a fisherwoman").gender, Gender::Feminine);
        assert_eq!(auto_voice("aferryman", "a ferryman").gender, Gender::Masculine);
        let giant = auto_voice("ahillgiant", "a hill giant");
        let pixie = auto_voice("apixie", "a pixie");
        assert!(giant.pitch <= 80 && giant.rate <= 95);
        assert!(pixie.pitch >= 135 && pixie.rate >= 105);
        let sage = auto_voice("anoldsage", "an old sage");
        assert!(sage.rate <= 92);
        let ghost = auto_voice("aghost", "a ghost");
        assert!(ghost.pitch <= 85 && ghost.rate <= 85);
        // The same name, the same voice; every voice in range.
        assert_eq!(auto_voice("bob", "Bob"), auto_voice("bob", "Bob"));
        for name in ["Bob", "Ann", "Zed", "an ancient lich", "a tiny mouse", "the old giant"] {
            let v = auto_voice(&letters_and_digits(name), name);
            assert!((PITCH.0..=PITCH.1).contains(&v.pitch) && (RATE.0..=RATE.1).contains(&v.rate), "{name}");
        }
        // "human" isn't a man.
        assert_eq!(auto_voice("ahuman", "a human").gender, auto_voice("ahuman", "a person").gender);
    }

    #[test]
    fn voices_saved_before_engines_still_read() {
        let old = r#"{"characters":{"bob":{"key":"bob","name":"Bob","who":"pc","firstMet":1,"lastMet":1,"lines":0,"edited":true,
            "voice":{"gender":"masculine","pitch":90,"rate":100,"systemVoice":"Daniel","seed":3,"quiet":false}}}}"#;
        let c = Cast::from_json(old).unwrap();
        let v = &c.list()[0].voice;
        assert_eq!((v.engine.as_deref(), v.voice_name.as_deref()), (Some("system"), Some("Daniel")));
    }

    #[test]
    fn edits_stay_until_reset_and_survive_saving() {
        let mut c = Cast::default();
        c.gmcp("room.players", r#"{"pcs":[{"Bob":"Bob"}]}"#, 1);
        let mut v = c.list()[0].voice.clone();
        let seed = v.seed;
        v.pitch = 999;
        v.rate = 1;
        v.seed = 7;
        v.voice_name = Some(" Daniel ".into());
        v.quiet = true;
        // A voice with no engine isn't kept; an engine Coupler lacks is refused.
        assert_eq!(c.set_voice("bob", v.clone()).unwrap().voice.voice_name, None);
        v.engine = Some("espeak".into());
        assert!(c.set_voice("bob", v.clone()).is_err());
        v.engine = Some("system".into());
        let bob = c.set_voice("bob", v).unwrap();
        assert_eq!((bob.voice.pitch, bob.voice.rate, bob.voice.seed), (PITCH.1, RATE.0, seed));
        assert_eq!((bob.voice.engine.as_deref(), bob.voice.voice_name.as_deref()), (Some("system"), Some("Daniel")));
        let mut flite = bob.voice.clone();
        (flite.engine, flite.voice_name) = (Some("flite".into()), Some("kal16".into()));
        assert_eq!(c.set_voice("bob", flite).unwrap().voice.engine.as_deref(), Some("flite"));
        assert!(bob.edited && bob.voice.quiet);
        let mut again = Cast::from_json(&c.to_json()).unwrap();
        assert_eq!(again.list(), c.list());
        let bob = again.reset("bob").unwrap();
        assert!(!bob.edited);
        assert_eq!(bob.voice, auto_voice("bob", "Bob"));
        assert!(again.set_voice("nobody", bob.voice.clone()).is_err());
        assert!(again.forget("bob"));
        assert!(again.list().is_empty());
    }
}

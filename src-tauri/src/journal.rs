//! The journal and the log: every line of talk the player's character
//! comes across, kept by who said it and when, and whether the player
//! has heard it yet. `session.rs` feeds it each `comm.channel` line
//! (`senses.rs`) and opens it at `journal.sqlite` in app data.
//!
//! Shared with the web build. What's kept is `Journal`, from each
//! build's own `journal_store.rs`: SQLite on the desktop, memory kept as
//! JSON in the browser on the web; the tests here run against both.
//!
//! - **Two books.** The **journal** is what's said in the game: says in
//!   the room (an NPC's `postSay` comes the same way, `CommonMsgs.java`),
//!   tells and the group. The **log** is what's said outside it: OOC,
//!   INFO, GOSSIP and every other channel (`CMChannels.java`), kept so
//!   the immersion isn't broken by it (it isn't spoken on arrival).
//! - **One per character.** Each entry has its world (the port's) and
//!   the character's name as `char.base` gives it, letters and digits.
//! - **Heard** means heard to the end: Coupler's voice finished it, or
//!   the player said they'd read it. The player's own lines are heard.
//! - **Repeats.** A journal line someone not played by a person has said
//!   before (a shopkeeper's welcome, a guard's warning) is the same line
//!   with its numbers and punctuation ignored, and isn't kept again: only
//!   the first. A player's lines are always kept, since "ok" twice is
//!   two answers. The log keeps everything.
//!
//! What's in here is other players' names and words, so like the hooks
//! it stays on this machine (CLAUDE.md rule 1).

use serde::Serialize;

use crate::senses::TalkKind;
#[cfg(test)]
use crate::senses::Talk;

pub use crate::journal_store::Journal;

/// Which book a line goes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Book {
    /// Said in the game.
    Journal,
    /// Said outside it: the channels.
    Log,
}

impl Book {
    pub fn of(kind: TalkKind) -> Book {
        match kind {
            TalkKind::Channel => Book::Log,
            TalkKind::Tell | TalkKind::Group | TalkKind::Say => Book::Journal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Book::Journal => "journal",
            Book::Log => "log",
        }
    }

    pub fn parse(name: &str) -> Result<Book, String> {
        match name {
            "journal" => Ok(Book::Journal),
            "log" => Ok(Book::Log),
            _ => Err("Coupler has no book of that name.".into()),
        }
    }
}

/// Whose journal: a character in a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope<'a> {
    pub world: &'a str,
    /// Letters and digits, lowercased; empty before `char.base` names them.
    pub character: &'a str,
}

/// One line kept.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: i64,
    pub book: Book,
    pub kind: TalkKind,
    /// The channel's name as the game gives it: `say`, `tell`, `GTELL`, `OOC`.
    pub channel: String,
    /// Who said it, as best known (the cast's name for them).
    pub speaker: String,
    /// The speaker keyed as the cast keys them.
    pub speaker_key: String,
    /// The line as the game printed it, colors removed.
    pub text: String,
    /// Unix milliseconds.
    pub at: i64,
    pub mine: bool,
    pub heard: bool,
}

/// Someone in a book, for narrowing it to them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Speaker {
    pub key: String,
    pub name: String,
    pub lines: u32,
    pub unheard: u32,
}

/// What `list` answers.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    /// The newest `limit` that match, oldest first.
    pub entries: Vec<Entry>,
    /// How many match in all.
    pub matching: u32,
    /// Everyone in the book (not narrowed by the search), most lines first.
    pub speakers: Vec<Speaker>,
}

/// A count of what's unheard, under one name.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Count {
    pub name: String,
    pub count: u32,
}

/// What's still to be heard, in both books.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unheard {
    pub journal: u32,
    pub log: u32,
    /// The journal's by speaker, the most first.
    pub speakers: Vec<Count>,
    /// The log's by channel, the most first.
    pub channels: Vec<Count>,
}

/// A line as it's compared for repeats: lowercase words, every number
/// the same, punctuation and spacing ignored.
pub fn said(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        let c = if c.is_numeric() {
            '#'
        } else if c.is_alphabetic() {
            c
        } else if (c == ',' || c == '.') && out.ends_with('#') && !space {
            // Inside a number (9,000 or 1.5): still the one number.
            continue;
        } else {
            space = !out.is_empty();
            continue;
        };
        if space {
            out.push(' ');
            space = false;
        }
        if !(c == '#' && out.ends_with('#')) {
            out.push(c);
        }
    }
    out
}

/// A typed LAST's answer: the channel, how many were asked, and its
/// newest lines, oldest first.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastAnswer {
    pub channel: String,
    pub count: usize,
    pub entries: Vec<Entry>,
}

/// The channels CoffeeMUD comes with (`coffeemud.ini`'s CHANNELS): a
/// LAST for one is answered from the log even before it's heard any.
pub const STOCK_CHANNELS: [&str; 10] = ["GOSSIP", "GRATZ", "CHAT", "OOC", "QUESTCHAT", "WIZ", "CLANTALK", "INFO", "WIZINFO", "ICOFF"];
/// LAST's count when none is given, as the game's (`Commands/Channel.java`).
pub const LAST_COUNT: usize = 10;
/// LAST's most, as the game's for a player.
pub const LAST_MOST: usize = 100;

/// A channel's LAST, typed (`OOC LAST 10`, `GOSSIP LAST`, `ooc prev 5`):
/// the channel's word and how many. The game takes `last` in short, or
/// `list` or `previous` (`Commands/Channel.java`); a count of 0 is 1.
pub fn last_asked(line: &str) -> Option<(String, usize)> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let (channel, word, count) = match words.as_slice() {
        [channel, word] => (channel, word, LAST_COUNT),
        [channel, word, n] => (channel, word, n.parse::<usize>().ok()?),
        _ => return None,
    };
    let word = word.to_ascii_lowercase();
    let last = word.len() >= 2 && ("last".starts_with(&word) || "list".starts_with(&word) || "previous".starts_with(&word));
    (last && channel.chars().all(char::is_alphanumeric)).then(|| (channel.to_ascii_uppercase(), count.clamp(1, LAST_MOST)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERE: Scope = Scope { world: "standard", character: "tester" };

    fn talk(kind: TalkKind, channel: &str, from: &str, text: &str) -> Talk {
        Talk { kind, channel: channel.into(), from: from.into(), text: text.into(), mine: false }
    }

    #[test]
    fn talk_goes_in_its_book() {
        assert_eq!(Book::of(TalkKind::Say), Book::Journal);
        assert_eq!(Book::of(TalkKind::Tell), Book::Journal);
        assert_eq!(Book::of(TalkKind::Group), Book::Journal);
        assert_eq!(Book::of(TalkKind::Channel), Book::Log);
        let j = Journal::in_memory().unwrap();
        let ooc = j.record(HERE, &talk(TalkKind::Channel, "OOC", "Bob", "Bob OOCs 'brb'"), "Bob", true, 1).unwrap().unwrap();
        assert_eq!((ooc.book, ooc.heard), (Book::Log, false));
        let say = j.record(HERE, &talk(TalkKind::Say, "say", "aguard", "A guard says 'Halt!'"), "a guard", false, 2).unwrap().unwrap();
        assert_eq!((say.book, say.speaker.as_str(), say.speaker_key.as_str()), (Book::Journal, "a guard", "aguard"));
        let log = j.list(HERE, Book::Log, "", None, 50).unwrap();
        assert_eq!(log.entries.len(), 1);
        assert_eq!(j.list(HERE, Book::Journal, "", None, 50).unwrap().entries, vec![say]);
    }

    #[test]
    fn an_npc_repeating_itself_is_kept_once() {
        let j = Journal::in_memory().unwrap();
        let line = |text: &str| talk(TalkKind::Say, "say", "ashopkeeper", text);
        assert!(j.record(HERE, &line("A shopkeeper says 'I'll give you 12 gold.'"), "a shopkeeper", false, 1).unwrap().is_some());
        assert_eq!(j.record(HERE, &line("A shopkeeper says 'I'll give you 150 gold!'"), "a shopkeeper", false, 2).unwrap(), None);
        assert!(j.record(HERE, &line("A shopkeeper says 'Welcome.'"), "a shopkeeper", false, 3).unwrap().is_some());
        // Someone else saying it, or another character hearing it, is new.
        assert!(j.record(HERE, &talk(TalkKind::Say, "say", "aguard", "A shopkeeper says 'Welcome.'"), "a guard", false, 4).unwrap().is_some());
        let other = Scope { world: "standard", character: "ann" };
        assert!(j.record(other, &line("A shopkeeper says 'Welcome.'"), "a shopkeeper", false, 5).unwrap().is_some());
        // A player's lines and the log are always kept.
        let ok = talk(TalkKind::Tell, "tell", "Bob", "Bob tells you 'ok'");
        assert!(j.record(HERE, &ok, "Bob", true, 6).unwrap().is_some());
        assert!(j.record(HERE, &ok, "Bob", true, 7).unwrap().is_some());
        let info = talk(TalkKind::Channel, "INFO", "", "Bob has logged in.");
        assert!(j.record(HERE, &info, "", false, 8).unwrap().is_some());
        assert!(j.record(HERE, &info, "", false, 9).unwrap().is_some());
        assert_eq!(j.list(HERE, Book::Journal, "", None, 50).unwrap().matching, 5);
    }

    #[test]
    fn repeats_ignore_numbers_and_punctuation() {
        assert_eq!(said("A shopkeeper says 'I'll give you 12 gold.'"), said("a SHOPKEEPER says: I ll give you 9,000 gold!!"));
        assert_ne!(said("Welcome."), said("Welcome back."));
        assert_eq!(said("  'Hi!'  "), "hi");
    }

    #[test]
    fn own_lines_are_heard_and_the_rest_wait() {
        let j = Journal::in_memory().unwrap();
        let mut mine = talk(TalkKind::Say, "say", "Joash", "You say 'hello'");
        mine.mine = true;
        assert!(j.record(HERE, &mine, "Joash", true, 1).unwrap().unwrap().heard);
        let a = j.record(HERE, &talk(TalkKind::Say, "say", "aguard", "A guard says 'Halt!'"), "a guard", false, 2).unwrap().unwrap();
        let b = j.record(HERE, &talk(TalkKind::Tell, "tell", "Bob", "Bob tells you 'hi'"), "Bob", true, 3).unwrap().unwrap();
        j.record(HERE, &talk(TalkKind::Tell, "tell", "Bob", "Bob tells you 'there?'"), "Bob", true, 4).unwrap();
        j.record(HERE, &talk(TalkKind::Channel, "OOC", "Ann", "Ann OOCs 'hi all'"), "Ann", true, 5).unwrap();
        let u = j.unheard(HERE).unwrap();
        assert_eq!((u.journal, u.log), (3, 1));
        assert_eq!(u.speakers, vec![Count { name: "Bob".into(), count: 2 }, Count { name: "a guard".into(), count: 1 }]);
        assert_eq!(u.channels, vec![Count { name: "OOC".into(), count: 1 }]);
        assert_eq!(j.unheard_entries(HERE, Book::Journal, 10).unwrap().iter().map(|e| e.id).collect::<Vec<_>>(), vec![a.id, b.id, b.id + 1]);
        assert_eq!(j.set_heard(&[a.id, b.id], true).unwrap(), 2);
        assert_eq!(j.set_heard(&[a.id], true).unwrap(), 0);
        assert_eq!(j.unheard(HERE).unwrap().journal, 1);
        assert_eq!(j.heard_all(HERE, Book::Journal).unwrap(), 1);
        assert_eq!(j.unheard(HERE).unwrap(), Unheard { log: 1, channels: vec![Count { name: "OOC".into(), count: 1 }], ..Unheard::default() });
    }

    #[test]
    fn search_and_narrow_keep_the_order() {
        let j = Journal::in_memory().unwrap();
        for (i, (from, text)) in [("Bob", "Bob tells you 'the gate is north'"), ("aguard", "A guard says 'Move along.'"), ("Bob", "Bob tells you 'go NORTH, 100%'")].iter().enumerate() {
            j.record(HERE, &talk(TalkKind::Tell, "tell", from, text), from, *from == "Bob", i as i64).unwrap();
        }
        let found = j.list(HERE, Book::Journal, "north", None, 50).unwrap();
        assert_eq!(found.entries.iter().map(|e| e.at).collect::<Vec<_>>(), vec![0, 2]);
        assert_eq!(j.list(HERE, Book::Journal, "100%", None, 50).unwrap().matching, 1);
        assert_eq!(j.list(HERE, Book::Journal, "_", None, 50).unwrap().matching, 0);
        assert_eq!(j.list(HERE, Book::Journal, "guard", None, 50).unwrap().matching, 1);
        let bob = j.list(HERE, Book::Journal, "", Some("bob"), 50).unwrap();
        assert_eq!(bob.matching, 2);
        assert_eq!(bob.speakers[0], Speaker { key: "bob".into(), name: "Bob".into(), lines: 2, unheard: 2 });
        // The newest of many, still oldest first.
        let last = j.list(HERE, Book::Journal, "", None, 2).unwrap();
        assert_eq!((last.matching, last.entries.iter().map(|e| e.at).collect::<Vec<_>>()), (3, vec![1, 2]));
    }

    #[test]
    fn a_typed_last_is_read() {
        assert_eq!(last_asked("OOC LAST 10"), Some(("OOC".into(), 10)));
        assert_eq!(last_asked("gossip last"), Some(("GOSSIP".into(), LAST_COUNT)));
        assert_eq!(last_asked("ooc prev 5"), Some(("OOC".into(), 5)));
        assert_eq!(last_asked("ooc la 500"), Some(("OOC".into(), LAST_MOST)));
        assert_eq!(last_asked("ooc lastly"), None);
        assert_eq!(last_asked("ooc last ten"), None);
        assert_eq!(last_asked("ooc hello there"), None);
        assert_eq!(last_asked("ooc l"), None);
    }

    #[test]
    fn last_answers_a_channel_from_the_log() {
        let j = Journal::in_memory().unwrap();
        let ooc = |text: &str| talk(TalkKind::Channel, "OOC", "bob", text);
        for n in 1..=4 {
            j.record(HERE, &ooc(&format!("Bob OOC: 'line {n}'")), "Bob", true, n).unwrap();
        }
        j.record(HERE, &talk(TalkKind::Channel, "GOSSIP", "ann", "Ann gossips 'hi'"), "Ann", true, 9).unwrap();
        let (channel, entries) = j.last(HERE, "ooc", 2).unwrap().unwrap();
        assert_eq!(channel, "OOC");
        assert_eq!(entries.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(), ["Bob OOC: 'line 3'", "Bob OOC: 'line 4'"]);
        // A stock channel never heard: known, with nothing in it.
        assert_eq!(j.last(HERE, "GRATZ", 5).unwrap().unwrap().1.len(), 0);
        // The start of one: GOS is GOSSIP.
        assert_eq!(j.last(HERE, "gos", 5).unwrap().unwrap().0, "GOSSIP");
        // Not a channel: the game's to answer.
        assert_eq!(j.last(HERE, "say", 5).unwrap(), None);
        assert_eq!(j.last(HERE, "WI", 5).unwrap(), None);
    }
}

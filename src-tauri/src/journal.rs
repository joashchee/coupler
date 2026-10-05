//! The journal and the log: every line of talk the player's character
//! comes across, kept by who said it and when, and whether the player
//! has heard it yet. Pure and unit-tested against in-memory databases;
//! `session.rs` feeds it each `comm.channel` line (`senses.rs`) and
//! opens it at `journal.sqlite` in app data.
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

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::senses::{letters_and_digits, Talk, TalkKind};

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

    fn name(self) -> &'static str {
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

pub struct Journal {
    db: Connection,
}

fn failed(e: rusqlite::Error) -> String {
    format!("The journal couldn't be used. ({e})")
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

/// `%` and `_` taken literally in a LIKE.
fn like(query: &str) -> String {
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

fn entry(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let channel: String = row.get(2)?;
    let kind = TalkKind::of(&channel);
    Ok(Entry {
        id: row.get(0)?,
        book: if row.get::<_, String>(1)? == "log" { Book::Log } else { Book::Journal },
        kind,
        channel,
        speaker: row.get(3)?,
        speaker_key: row.get(4)?,
        text: row.get(5)?,
        at: row.get(6)?,
        mine: row.get(7)?,
        heard: row.get(8)?,
    })
}

const COLUMNS: &str = "id, book, channel, speaker, speaker_key, text, at, mine, heard";

impl Journal {
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::prepare(Connection::open(path).map_err(failed)?)
    }

    /// A journal that lives only as long as this value: for the tests,
    /// and for playing on when the file can't be used.
    pub fn in_memory() -> Result<Self, String> {
        Self::prepare(Connection::open_in_memory().map_err(failed)?)
    }

    fn prepare(db: Connection) -> Result<Self, String> {
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS entries (
                 id INTEGER PRIMARY KEY,
                 world TEXT NOT NULL,
                 character TEXT NOT NULL,
                 book TEXT NOT NULL,
                 channel TEXT NOT NULL,
                 speaker TEXT NOT NULL,
                 speaker_key TEXT NOT NULL,
                 text TEXT NOT NULL,
                 said TEXT NOT NULL,
                 at INTEGER NOT NULL,
                 mine INTEGER NOT NULL,
                 heard INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS entries_book ON entries (world, character, book, id);
             CREATE INDEX IF NOT EXISTS entries_said ON entries (world, character, speaker_key, said);
             CREATE INDEX IF NOT EXISTS entries_unheard ON entries (world, character, heard);",
        )
        .map_err(failed)?;
        Ok(Journal { db })
    }

    /// Keeps a line of talk. `speaker` is the name to show; `person` is
    /// whether a player says it (their lines are never repeats). Returns
    /// the entry, or None for a repeat, which isn't kept.
    pub fn record(&self, scope: Scope, talk: &Talk, speaker: &str, person: bool, at: i64) -> Result<Option<Entry>, String> {
        let book = Book::of(talk.kind);
        let speaker_key = letters_and_digits(&talk.from);
        let said = said(&talk.text);
        if book == Book::Journal && !talk.mine && !person {
            let before: Option<i64> = self
                .db
                .query_row(
                    "SELECT id FROM entries WHERE world = ?1 AND character = ?2 AND speaker_key = ?3 AND said = ?4 AND book = 'journal' LIMIT 1",
                    params![scope.world, scope.character, speaker_key, said],
                    |r| r.get(0),
                )
                .optional()
                .map_err(failed)?;
            if before.is_some() {
                return Ok(None);
            }
        }
        let speaker = if speaker.trim().is_empty() { talk.from.trim() } else { speaker.trim() };
        self.db
            .execute(
                "INSERT INTO entries (world, character, book, channel, speaker, speaker_key, text, said, at, mine, heard)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![scope.world, scope.character, book.name(), talk.channel, speaker, speaker_key, talk.text, said, at, talk.mine],
            )
            .map_err(failed)?;
        Ok(Some(Entry {
            id: self.db.last_insert_rowid(),
            book,
            kind: talk.kind,
            channel: talk.channel.clone(),
            speaker: speaker.to_string(),
            speaker_key,
            text: talk.text.clone(),
            at,
            mine: talk.mine,
            heard: talk.mine,
        }))
    }

    /// A book's newest `limit` lines matching `query` (in the words or
    /// the speaker's name; every line when it's empty), narrowed to one
    /// speaker or channel when `who` is given, oldest first.
    pub fn list(&self, scope: Scope, book: Book, query: &str, who: Option<&str>, limit: usize) -> Result<Listing, String> {
        let query = query.trim();
        let pattern = like(query);
        // ?4 is the search, ?5 the speaker (the journal) or channel (the log).
        let filter = "world = ?1 AND character = ?2 AND book = ?3
             AND (?4 = '' OR text LIKE ?6 ESCAPE '\\' OR speaker LIKE ?6 ESCAPE '\\')
             AND (?5 IS NULL OR (CASE WHEN book = 'log' THEN channel ELSE speaker_key END) = ?5)";
        let args = params![scope.world, scope.character, book.name(), query, who, pattern];
        let matching: u32 = self.db.query_row(&format!("SELECT COUNT(*) FROM entries WHERE {filter}"), args, |r| r.get(0)).map_err(failed)?;
        let mut statement = self
            .db
            .prepare(&format!("SELECT {COLUMNS} FROM entries WHERE {filter} ORDER BY id DESC LIMIT {limit}"))
            .map_err(failed)?;
        let mut entries: Vec<Entry> = statement.query_map(args, entry).map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
        entries.reverse();
        let mut statement = self
            .db
            .prepare(
                "SELECT CASE WHEN book = 'log' THEN channel ELSE speaker_key END AS who,
                        CASE WHEN book = 'log' THEN channel ELSE MAX(speaker) END,
                        COUNT(*), SUM(heard = 0)
                 FROM entries WHERE world = ?1 AND character = ?2 AND book = ?3 AND mine = 0
                 GROUP BY who ORDER BY COUNT(*) DESC, who",
            )
            .map_err(failed)?;
        let speakers = statement
            .query_map(params![scope.world, scope.character, book.name()], |r| {
                Ok(Speaker { key: r.get(0)?, name: r.get(1)?, lines: r.get(2)?, unheard: r.get(3)? })
            })
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)?;
        Ok(Listing { entries, matching, speakers })
    }

    /// The lines still to be heard, oldest first: what Play Unheard says.
    pub fn unheard_entries(&self, scope: Scope, book: Book, limit: usize) -> Result<Vec<Entry>, String> {
        let mut statement = self
            .db
            .prepare(&format!(
                "SELECT {COLUMNS} FROM entries WHERE world = ?1 AND character = ?2 AND book = ?3 AND heard = 0 ORDER BY id LIMIT {limit}"
            ))
            .map_err(failed)?;
        let entries = statement.query_map(params![scope.world, scope.character, book.name()], entry).map_err(failed)?.collect::<Result<_, _>>().map_err(failed);
        entries
    }

    /// Marks lines heard (or not). Returns how many changed.
    pub fn set_heard(&self, ids: &[i64], heard: bool) -> Result<usize, String> {
        let mut changed = 0;
        for id in ids {
            changed += self.db.execute("UPDATE entries SET heard = ?2 WHERE id = ?1 AND heard != ?2", params![id, heard]).map_err(failed)?;
        }
        Ok(changed)
    }

    /// Marks a whole book heard. Returns how many changed.
    pub fn heard_all(&self, scope: Scope, book: Book) -> Result<usize, String> {
        self.db
            .execute("UPDATE entries SET heard = 1 WHERE world = ?1 AND character = ?2 AND book = ?3 AND heard = 0", params![scope.world, scope.character, book.name()])
            .map_err(failed)
    }

    /// What's still to be heard in both books.
    pub fn unheard(&self, scope: Scope) -> Result<Unheard, String> {
        let mut statement = self
            .db
            .prepare(
                "SELECT book, CASE WHEN book = 'log' THEN channel ELSE MAX(speaker) END, COUNT(*)
                 FROM entries WHERE world = ?1 AND character = ?2 AND heard = 0
                 GROUP BY book, CASE WHEN book = 'log' THEN channel ELSE speaker_key END
                 ORDER BY COUNT(*) DESC, MIN(id)",
            )
            .map_err(failed)?;
        let rows = statement
            .query_map(params![scope.world, scope.character], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, u32>(2)?)))
            .map_err(failed)?;
        let mut unheard = Unheard::default();
        for row in rows {
            let (book, name, count) = row.map_err(failed)?;
            if book == "log" {
                unheard.log += count;
                unheard.channels.push(Count { name, count });
            } else {
                unheard.journal += count;
                unheard.speakers.push(Count { name, count });
            }
        }
        Ok(unheard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERE: Scope = Scope { world: "standard", character: "joash" };

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
}

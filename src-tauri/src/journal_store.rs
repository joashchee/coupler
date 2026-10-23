//! The desktop's journal and log (`journal.rs`): one SQLite database,
//! `journal.sqlite` in app data. The web build has its own
//! `journal_store.rs` with the same methods; `journal.rs`'s tests run
//! against both.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::journal::{said, Book, Count, Entry, Listing, Scope, Speaker, Unheard, STOCK_CHANNELS};
use crate::senses::{letters_and_digits, Talk, TalkKind};

pub struct Journal {
    db: Connection,
}

fn failed(e: rusqlite::Error) -> String {
    format!("The journal couldn't be used. ({e})")
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

    /// A channel's newest `count` lines, oldest first, from either book,
    /// for a typed LAST. `word` is the channel as typed: its name, or the
    /// start of one (three letters at least) that only it has. None when
    /// it's no channel Coupler knows (the stock ones, and any heard).
    pub fn last(&self, scope: Scope, word: &str, count: usize) -> Result<Option<(String, Vec<Entry>)>, String> {
        let mut statement = self
            .db
            .prepare("SELECT DISTINCT UPPER(channel) FROM entries WHERE world = ?1 AND character = ?2 AND book = 'log'")
            .map_err(failed)?;
        let mut known: Vec<String> = statement.query_map(params![scope.world, scope.character], |r| r.get(0)).map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
        known.extend(STOCK_CHANNELS.iter().map(|c| c.to_string()));
        known.sort();
        known.dedup();
        let word = word.to_ascii_uppercase();
        let channel = if known.contains(&word) {
            word
        } else {
            let starting: Vec<&String> = known.iter().filter(|c| word.len() >= 3 && c.starts_with(&word)).collect();
            match starting.as_slice() {
                [only] => (*only).clone(),
                _ => return Ok(None),
            }
        };
        let mut statement = self
            .db
            .prepare(&format!("SELECT {COLUMNS} FROM entries WHERE world = ?1 AND character = ?2 AND UPPER(channel) = ?3 ORDER BY id DESC LIMIT {count}"))
            .map_err(failed)?;
        let mut entries: Vec<Entry> = statement.query_map(params![scope.world, scope.character, channel], entry).map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
        entries.reverse();
        Ok(Some((channel, entries)))
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

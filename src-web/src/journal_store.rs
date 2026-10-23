//! The web build's journal and log (`journal.rs`): the desktop's
//! `journal_store.rs` is a SQLite database, and this is the same methods
//! over rows in memory, kept by the page in IndexedDB as JSON
//! (`to_json`, `from_json`). Each query does what the desktop's SQL does,
//! order and all: `journal.rs`'s tests run against both.

use std::cell::{Cell, RefCell};

use serde::{Deserialize, Serialize};

use crate::journal::{said, Book, Count, Entry, Listing, Scope, Speaker, Unheard, STOCK_CHANNELS};
use crate::senses::{letters_and_digits, Talk, TalkKind};

/// One line kept: an `entries` row of the desktop's.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Row {
    id: i64,
    world: String,
    character: String,
    log: bool,
    channel: String,
    speaker: String,
    speaker_key: String,
    text: String,
    said: String,
    at: i64,
    mine: bool,
    heard: bool,
}

impl Row {
    fn of(&self, scope: Scope) -> bool {
        self.world == scope.world && self.character == scope.character
    }

    fn book(&self) -> Book {
        if self.log {
            Book::Log
        } else {
            Book::Journal
        }
    }

    /// Who it's narrowed by: its channel in the log, its speaker in the journal.
    fn who(&self) -> &str {
        if self.log {
            &self.channel
        } else {
            &self.speaker_key
        }
    }

    fn entry(&self) -> Entry {
        Entry {
            id: self.id,
            book: self.book(),
            kind: TalkKind::of(&self.channel),
            channel: self.channel.clone(),
            speaker: self.speaker.clone(),
            speaker_key: self.speaker_key.clone(),
            text: self.text.clone(),
            at: self.at,
            mine: self.mine,
            heard: self.heard,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Kept {
    rows: Vec<Row>,
}

#[derive(Default)]
pub struct Journal {
    rows: RefCell<Vec<Row>>,
    /// Changed since `to_json` last kept it.
    changed: Cell<bool>,
}

/// SQLite's LIKE '%query%': ASCII letters in either case.
fn contains(text: &str, query: &str) -> bool {
    text.to_ascii_lowercase().contains(&query.to_ascii_lowercase())
}

/// SQLite's MAX of two texts: by their bytes.
fn max_name(a: &mut String, b: &str) {
    if b.as_bytes() > a.as_bytes() {
        *a = b.to_string();
    }
}

impl Journal {
    /// As the desktop's: a journal in memory, here the only kind.
    pub fn in_memory() -> Result<Self, String> {
        Ok(Journal::default())
    }

    /// The journal the page kept, or a new one when there's none.
    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.is_empty() {
            return Ok(Journal::default());
        }
        let kept: Kept = serde_json::from_str(json).map_err(|e| format!("The journal couldn't be read. ({e})"))?;
        Ok(Journal { rows: RefCell::new(kept.rows), changed: Cell::new(false) })
    }

    /// The journal as JSON for the page to keep; it's then unchanged.
    pub fn to_json(&self) -> String {
        self.changed.set(false);
        let rows = self.rows.borrow();
        serde_json::to_string(&serde_json::json!({ "rows": &*rows })).unwrap_or_default()
    }

    /// Whether there's anything new to keep.
    pub fn changed(&self) -> bool {
        self.changed.get()
    }

    pub fn record(&self, scope: Scope, talk: &Talk, speaker: &str, person: bool, at: i64) -> Result<Option<Entry>, String> {
        let book = Book::of(talk.kind);
        let speaker_key = letters_and_digits(&talk.from);
        let said = said(&talk.text);
        let mut rows = self.rows.borrow_mut();
        if book == Book::Journal && !talk.mine && !person && rows.iter().any(|r| r.of(scope) && !r.log && r.speaker_key == speaker_key && r.said == said) {
            return Ok(None);
        }
        let speaker = if speaker.trim().is_empty() { talk.from.trim() } else { speaker.trim() };
        let row = Row {
            id: rows.last().map_or(1, |r| r.id + 1),
            world: scope.world.to_string(),
            character: scope.character.to_string(),
            log: book == Book::Log,
            channel: talk.channel.clone(),
            speaker: speaker.to_string(),
            speaker_key,
            text: talk.text.clone(),
            said,
            at,
            mine: talk.mine,
            heard: talk.mine,
        };
        let entry = row.entry();
        rows.push(row);
        self.changed.set(true);
        Ok(Some(entry))
    }

    pub fn list(&self, scope: Scope, book: Book, query: &str, who: Option<&str>, limit: usize) -> Result<Listing, String> {
        let query = query.trim();
        let rows = self.rows.borrow();
        let in_book = || rows.iter().filter(|r| r.of(scope) && r.book() == book);
        let matching: Vec<&Row> = in_book()
            .filter(|r| query.is_empty() || contains(&r.text, query) || contains(&r.speaker, query))
            .filter(|r| who.is_none_or(|w| r.who() == w))
            .collect();
        let mut entries: Vec<Entry> = matching.iter().rev().take(limit).map(|r| r.entry()).collect();
        entries.reverse();
        let mut speakers: Vec<Speaker> = Vec::new();
        for r in in_book().filter(|r| !r.mine) {
            match speakers.iter_mut().find(|s| s.key == r.who()) {
                Some(s) => {
                    s.lines += 1;
                    s.unheard += u32::from(!r.heard);
                    if !r.log {
                        max_name(&mut s.name, &r.speaker);
                    }
                }
                None => speakers.push(Speaker {
                    key: r.who().to_string(),
                    name: if r.log { r.channel.clone() } else { r.speaker.clone() },
                    lines: 1,
                    unheard: u32::from(!r.heard),
                }),
            }
        }
        speakers.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.key.as_bytes().cmp(b.key.as_bytes())));
        Ok(Listing { entries, matching: matching.len() as u32, speakers })
    }

    pub fn unheard_entries(&self, scope: Scope, book: Book, limit: usize) -> Result<Vec<Entry>, String> {
        let rows = self.rows.borrow();
        Ok(rows.iter().filter(|r| r.of(scope) && r.book() == book && !r.heard).take(limit).map(Row::entry).collect())
    }

    pub fn last(&self, scope: Scope, word: &str, count: usize) -> Result<Option<(String, Vec<Entry>)>, String> {
        let rows = self.rows.borrow();
        let mut known: Vec<String> = rows.iter().filter(|r| r.of(scope) && r.log).map(|r| r.channel.to_ascii_uppercase()).collect();
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
        let mut entries: Vec<Entry> = rows.iter().rev().filter(|r| r.of(scope) && r.channel.to_ascii_uppercase() == channel).take(count).map(Row::entry).collect();
        entries.reverse();
        Ok(Some((channel, entries)))
    }

    pub fn set_heard(&self, ids: &[i64], heard: bool) -> Result<usize, String> {
        let mut rows = self.rows.borrow_mut();
        let mut changed = 0;
        for id in ids {
            if let Some(r) = rows.iter_mut().find(|r| r.id == *id && r.heard != heard) {
                r.heard = heard;
                changed += 1;
            }
        }
        self.changed.set(self.changed.get() || changed > 0);
        Ok(changed)
    }

    pub fn heard_all(&self, scope: Scope, book: Book) -> Result<usize, String> {
        let mut rows = self.rows.borrow_mut();
        let mut changed = 0;
        for r in rows.iter_mut().filter(|r| r.of(scope) && r.book() == book && !r.heard) {
            r.heard = true;
            changed += 1;
        }
        self.changed.set(self.changed.get() || changed > 0);
        Ok(changed)
    }

    pub fn unheard(&self, scope: Scope) -> Result<Unheard, String> {
        let rows = self.rows.borrow();
        // (log, who, name, count, first id), as the desktop's GROUP BY.
        let mut groups: Vec<(bool, &str, String, u32, i64)> = Vec::new();
        for r in rows.iter().filter(|r| r.of(scope) && !r.heard) {
            match groups.iter_mut().find(|g| g.0 == r.log && g.1 == r.who()) {
                Some(g) => {
                    g.3 += 1;
                    if !r.log {
                        max_name(&mut g.2, &r.speaker);
                    }
                }
                None => groups.push((r.log, r.who(), if r.log { r.channel.clone() } else { r.speaker.clone() }, 1, r.id)),
            }
        }
        groups.sort_by(|a, b| b.3.cmp(&a.3).then(a.4.cmp(&b.4)));
        let mut unheard = Unheard::default();
        for (log, _, name, count, _) in groups {
            if log {
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

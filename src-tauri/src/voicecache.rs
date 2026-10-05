//! Lines already rendered in a bundled voice (`synth.rs`), kept so a
//! line plays the moment it's wanted. Every line of talk is rendered as
//! it arrives, before anyone asks to hear it (`voice_prerender`), and
//! put here with when it was made, when it last played and how often.
//! Unit-tested against a folder in the system's temp dir.
//!
//! - **On disk**: `voice-cache/` in app data, one `<hash>.pcm` per line
//!   (the bytes `voice_synth` returns: the sample rate, then 16-bit
//!   mono), and `index.sqlite` beside them. The hash covers the engine,
//!   voice, gender, pitch, speed and words, and the index keeps the full
//!   key to rule out a collision.
//! - **Room**: the cache is kept to `budget`, worked out from the free
//!   space on the disk it's on (`free_space`), checked after each new
//!   render and every few minutes: at most 1 GB, at most 5% of the space
//!   it could have, and when the disk has under 2 GB free, it gives
//!   back what it takes to get there again.
//! - **What goes first** (`trim`): lines played once, the oldest first,
//!   players' channel lines (the log) before the game's (the journal);
//!   then lines played more, the fewest plays first, the oldest first;
//!   and last, lines rendered but never played, which are still waiting
//!   to be heard. "Oldest" is when it last played (or was made).
//! - **Order of work** (`Queue`): the journal's lines are rendered before
//!   the log's, each in the order they came. A line wanted now that's
//!   still waiting is taken out of the queue and rendered at once, and
//!   one being rendered is waited for rather than made twice.
//!
//! The words are other players' and the game's, so like the journal the
//! cache stays on this machine (CLAUDE.md rule 1).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

/// The most the cache ever holds.
pub const CAP: u64 = 1 << 30;
/// At most this share of the room it could have (free space and itself), in percent.
pub const SHARE: u64 = 5;
/// The free space the cache never eats into.
pub const RESERVE: u64 = 2 << 30;

/// Where a line came from: it decides what's flushed first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A channel: the log.
    Log = 0,
    /// Said in the game: the journal.
    Journal = 1,
}

impl Origin {
    pub fn parse(book: &str) -> Option<Origin> {
        match book {
            "log" => Some(Origin::Log),
            "journal" => Some(Origin::Journal),
            _ => None,
        }
    }
}

fn failed(e: rusqlite::Error) -> String {
    format!("The voice cache couldn't be used. ({e})")
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// What a line was rendered with, and its words: the whole key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub engine: String,
    pub voice: String,
    pub text: String,
    pub gender: String,
    pub pitch: u16,
    pub rate: u16,
}

impl Key {
    fn full(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}", self.engine, self.voice, self.gender, self.pitch, self.rate, self.text)
    }

    /// FNV-1a, 64 bits: the same name for a line in every version.
    fn hash(&self) -> String {
        let h = self.full().bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
        format!("{h:016x}")
    }
}

/// How big the cache may be, from the disk's free space (None when it
/// can't be read) and what the cache holds now.
pub fn budget(free: Option<u64>, held: u64) -> u64 {
    let Some(free) = free else { return CAP };
    let mut most = CAP.min((free + held) / 100 * SHARE);
    if free < RESERVE {
        most = most.min(held.saturating_sub(RESERVE - free));
    }
    most
}

/// The space free to Coupler on the disk holding `path`.
#[cfg(unix)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a valid C string and `stat` a valid out-parameter.
    if unsafe { libc::statvfs(c.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    #[allow(clippy::unnecessary_cast)] // u32 on macOS, u64 on Linux.
    Some(stat.f_bavail as u64 * stat.f_frsize as u64)
}

/// Not read elsewhere yet (docs/platform-parity.md): the cap alone.
#[cfg(not(unix))]
pub fn free_space(_path: &Path) -> Option<u64> {
    None
}

pub struct RenderCache {
    dir: PathBuf,
    db: Connection,
    /// The bytes held, kept in step with the index.
    held: u64,
}

impl RenderCache {
    /// Opens (or makes) the cache in `dir`. An index that can't be read
    /// is started again: the files are only renders, made again on need.
    pub fn open(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("The voice cache couldn't be made. ({e})"))?;
        let index = dir.join("index.sqlite");
        let db = Connection::open(&index).and_then(|db| Self::prepare(&db).map(|_| db));
        let db = match db {
            Ok(db) => db,
            Err(e) => {
                log::warn!("{}", failed(e));
                let _ = std::fs::remove_file(&index);
                let db = Connection::open(&index).map_err(failed)?;
                Self::prepare(&db).map_err(failed)?;
                db
            }
        };
        let held: i64 = db.query_row("SELECT COALESCE(SUM(bytes), 0) FROM renders", [], |r| r.get(0)).map_err(failed)?;
        let cache = RenderCache { dir: dir.to_path_buf(), db, held: held as u64 };
        cache.forget_strays();
        Ok(cache)
    }

    fn prepare(db: &Connection) -> rusqlite::Result<()> {
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS renders (
                 id TEXT PRIMARY KEY,
                 key TEXT NOT NULL,
                 bytes INTEGER NOT NULL,
                 made INTEGER NOT NULL,
                 used INTEGER NOT NULL,
                 plays INTEGER NOT NULL,
                 origin INTEGER NOT NULL
             );",
        )
    }

    /// Files with no row (a crash between writing and indexing) are deleted.
    fn forget_strays(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|x| x == "pcm") {
                let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let known: Option<i64> = self.db.query_row("SELECT 1 FROM renders WHERE id = ?1", [id], |r| r.get(0)).optional().ok().flatten();
                if known.is_none() {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.pcm"))
    }

    /// The bytes the cache holds.
    #[cfg(test)]
    pub fn held(&self) -> u64 {
        self.held
    }

    pub fn has(&self, key: &Key) -> bool {
        let found: Option<String> = self.db.query_row("SELECT key FROM renders WHERE id = ?1", [key.hash()], |r| r.get(0)).optional().ok().flatten();
        found.is_some_and(|k| k == key.full())
    }

    /// A line, if it's here; `play` counts it as played now.
    pub fn get(&mut self, key: &Key, play: bool) -> Option<Vec<u8>> {
        let id = key.hash();
        if !self.has(key) {
            return None;
        }
        match std::fs::read(self.file(&id)) {
            Ok(bytes) => {
                if play {
                    let _ = self.db.execute("UPDATE renders SET plays = plays + 1, used = ?2 WHERE id = ?1", params![id, now_ms()]);
                }
                Some(bytes)
            }
            Err(_) => {
                // The file's gone: forget it, and it's made again.
                self.remove(&id);
                None
            }
        }
    }

    /// Keeps a line; `played` when it was rendered to play now.
    pub fn put(&mut self, key: &Key, bytes: &[u8], origin: Origin, played: bool) -> Result<(), String> {
        let id = key.hash();
        self.remove(&id);
        std::fs::write(self.file(&id), bytes).map_err(|e| format!("A voice couldn't be cached. ({e})"))?;
        let now = now_ms();
        self.db
            .execute(
                "INSERT INTO renders (id, key, bytes, made, used, plays, origin) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)",
                params![id, key.full(), bytes.len() as i64, now, i64::from(played), origin as i64],
            )
            .map_err(failed)?;
        self.held += bytes.len() as u64;
        Ok(())
    }

    fn remove(&mut self, id: &str) {
        let bytes: Option<i64> = self.db.query_row("SELECT bytes FROM renders WHERE id = ?1", [id], |r| r.get(0)).optional().ok().flatten();
        if let Some(bytes) = bytes {
            let _ = std::fs::remove_file(self.file(id));
            let _ = self.db.execute("DELETE FROM renders WHERE id = ?1", [id]);
            self.held = self.held.saturating_sub(bytes as u64);
        }
    }

    /// Flushes lines, in the order the header gives, until the cache
    /// holds `most` bytes or less. Returns how many went.
    pub fn trim(&mut self, most: u64) -> usize {
        if self.held <= most {
            return 0;
        }
        let order: Vec<(String, i64)> = {
            let Ok(mut statement) = self.db.prepare(
                "SELECT id, bytes FROM renders ORDER BY
                     CASE WHEN plays = 1 THEN 0 WHEN plays > 1 THEN 1 ELSE 2 END,
                     CASE WHEN plays = 1 THEN origin ELSE 0 END,
                     plays,
                     used, made",
            ) else {
                return 0;
            };
            let Ok(rows) = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?))) else { return 0 };
            rows.flatten().collect()
        };
        let mut gone = 0;
        for (id, _) in order {
            if self.held <= most {
                break;
            }
            self.remove(&id);
            gone += 1;
        }
        gone
    }

    /// Keeps the cache within what the disk can spare now.
    pub fn fit(&mut self) -> usize {
        let most = budget(free_space(&self.dir), self.held);
        let gone = self.trim(most);
        if gone > 0 {
            log::info!("voice cache: flushed {gone} lines to stay within {most} bytes");
        }
        gone
    }
}

/// A line to render before it's wanted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub key: Key,
    pub gender: crate::cast::Gender,
    pub origin: Origin,
}

/// The lines waiting to be rendered, and the one being rendered.
#[derive(Default)]
struct Waiting {
    journal: VecDeque<Job>,
    log: VecDeque<Job>,
    busy: Option<Key>,
}

/// The work for the prerender thread: the journal's lines first.
#[derive(Default)]
pub struct Queue {
    waiting: Mutex<Waiting>,
    changed: Condvar,
}

/// Lines waiting at most in each book; the oldest give way.
const QUEUED: usize = 200;

impl Queue {
    pub fn push(&self, job: Job) {
        let mut w = self.waiting.lock().unwrap();
        if w.busy.as_ref() == Some(&job.key) || w.journal.iter().chain(w.log.iter()).any(|j| j.key == job.key) {
            return;
        }
        let list = if job.origin == Origin::Journal { &mut w.journal } else { &mut w.log };
        if list.len() == QUEUED {
            list.pop_front();
        }
        list.push_back(job);
        self.changed.notify_all();
    }

    /// The next line to render, waiting for one; it's busy until `done`.
    pub fn next(&self) -> Job {
        let mut w = self.waiting.lock().unwrap();
        loop {
            if let Some(job) = w.journal.pop_front().or_else(|| w.log.pop_front()) {
                w.busy = Some(job.key.clone());
                return job;
            }
            w = self.changed.wait(w).unwrap();
        }
    }

    /// The line being rendered is done (made or failed).
    pub fn done(&self) {
        self.waiting.lock().unwrap().busy = None;
        self.changed.notify_all();
    }

    /// A line is wanted now: taken out of the queue if it's waiting, and
    /// waited for if it's being rendered. Its origin, when it was queued.
    pub fn claim(&self, key: &Key) -> Option<Origin> {
        let mut w = self.waiting.lock().unwrap();
        let mut origin = None;
        let Waiting { journal, log, .. } = &mut *w;
        for list in [journal, log] {
            if let Some(i) = list.iter().position(|j| &j.key == key) {
                origin = list.remove(i).map(|j| j.origin);
            }
        }
        while w.busy.as_ref() == Some(key) {
            w = self.changed.wait(w).unwrap();
        }
        origin
    }

    #[cfg(test)]
    fn len(&self) -> (usize, usize) {
        let w = self.waiting.lock().unwrap();
        (w.journal.len(), w.log.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1 << 30;

    fn key(text: &str) -> Key {
        Key { engine: "flite".into(), voice: "kal".into(), text: text.into(), gender: "masculine".into(), pitch: 100, rate: 130 }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("coupler-voicecache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn budget_follows_the_free_space() {
        assert_eq!(budget(None, 0), CAP);
        // Plenty of room: the cap.
        assert_eq!(budget(Some(500 * GB), 0), CAP);
        // 5% of what it could have.
        assert_eq!(budget(Some(10 * GB), 0), 10 * GB / 100 * 5);
        // Under the reserve: give back what it takes to reach it again.
        const MB: u64 = 1 << 20;
        assert_eq!(budget(Some(RESERVE - 10 * MB), 50 * MB), 40 * MB);
        assert_eq!(budget(Some(GB + GB / 2), GB / 2), 0);
        // Never more than the share, though.
        assert_eq!(budget(Some(GB), 3 * GB), 4 * GB / 100 * 5);
        assert_eq!(budget(Some(0), 0), 0);
    }

    #[test]
    fn a_line_is_kept_and_counted() {
        let dir = temp("kept");
        let mut c = RenderCache::open(&dir).unwrap();
        assert_eq!(c.get(&key("hi"), true), None);
        c.put(&key("hi"), &[1, 2, 3, 4], Origin::Journal, false).unwrap();
        assert!(c.has(&key("hi")));
        assert!(!c.has(&key("hello")));
        assert_eq!(c.held(), 4);
        assert_eq!(c.get(&key("hi"), true), Some(vec![1, 2, 3, 4]));
        // Opened again, it's still there, and a file with no row is gone.
        std::fs::write(dir.join("stray.pcm"), [0]).unwrap();
        drop(c);
        let mut c = RenderCache::open(&dir).unwrap();
        assert_eq!((c.held(), c.get(&key("hi"), false)), (4, Some(vec![1, 2, 3, 4])));
        assert!(!dir.join("stray.pcm").exists());
        // A different pitch is a different render.
        assert!(!c.has(&Key { pitch: 120, ..key("hi") }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn flushed_in_order() {
        let dir = temp("order");
        let mut c = RenderCache::open(&dir).unwrap();
        let line = |c: &mut RenderCache, text: &str, origin: Origin, plays: u32| {
            c.put(&key(text), &[0; 10], origin, false).unwrap();
            for _ in 0..plays {
                c.get(&key(text), true);
            }
            std::thread::sleep(std::time::Duration::from_millis(3));
        };
        line(&mut c, "waiting", Origin::Journal, 0);
        line(&mut c, "game once old", Origin::Journal, 1);
        line(&mut c, "loved", Origin::Journal, 5);
        line(&mut c, "channel once", Origin::Log, 1);
        line(&mut c, "twice", Origin::Log, 2);
        line(&mut c, "game once new", Origin::Journal, 1);
        let order = ["channel once", "game once old", "game once new", "twice", "loved", "waiting"];
        for (i, text) in order.iter().enumerate() {
            assert!(c.has(&key(text)), "{text} went too soon");
            assert_eq!(c.trim(c.held() - 10), 1);
            assert!(!c.has(&key(text)), "{text} should be the {i}th to go");
        }
        assert_eq!(c.held(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_queue_puts_the_journal_first_and_gives_up_what_is_wanted() {
        let q = Queue::default();
        q.push(Job { key: key("ooc"), gender: crate::cast::Gender::Masculine, origin: Origin::Log });
        q.push(Job { key: key("say"), gender: crate::cast::Gender::Masculine, origin: Origin::Journal });
        q.push(Job { key: key("say"), gender: crate::cast::Gender::Masculine, origin: Origin::Journal });
        q.push(Job { key: key("tell"), gender: crate::cast::Gender::Masculine, origin: Origin::Journal });
        assert_eq!(q.len(), (2, 1));
        assert_eq!(q.claim(&key("tell")), Some(Origin::Journal));
        assert_eq!(q.next().key, key("say"));
        q.done();
        assert_eq!(q.next().key, key("ooc"));
        q.done();
        assert_eq!(q.claim(&key("nothing")), None);
    }
}

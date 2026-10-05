//! The hooks database: every key and value CoffeeMUD has sent by GMCP,
//! each pair kept once. It's the record of what the game can tell
//! Coupler, and later what a GMCP trigger can hang on.
//!
//! It's a SQLite database (`hooks.sqlite` in app data; `session` says
//! where and feeds it): one table, `hooks(key, value)`, whose primary
//! key is the pair, so the database itself refuses a duplicate. A new
//! pair is on disk as soon as it's recorded; nothing is saved later.
//! A second table, `filtered(key)`, holds the keys the player has moved
//! out of the list (the noisy ones). They're still recorded, so taking
//! a key back out of it brings back everything it ever had.
//!
//! A third, `triggers`, holds what a pair sets off each time the game
//! sends it: up to one of each kind of asset (`assets.rs`), a sound
//! effect, a piece of music (once or looping) and a picture at a place
//! on the screen. `record` returns the triggers a message set off; the
//! frontend plays and shows them. Two more slots are ambience, played
//! in a loop for as long as it applies rather than each time the pair
//! comes: background noise (BGN) on a room's pairs and background
//! weather (BGW) on the weather's (`ambient.rs` decides which plays). An asset that has left the Assets
//! folder is cleared from every trigger naming it (`clear_missing`).
//! Unit-tested against in-memory databases.
//!
//! A message is flattened to pairs. The key is the package in lower
//! case, then the path through the JSON, joined by dots:
//! `room.info {"zone":"Midgaard","idexits":{"N":"Midgaard#3001"}}` gives
//! `room.info.zone` and `room.info.idexits.N`. An array's items share the
//! array's key (a position isn't a name). The value is the JSON text of
//! what's there (`"Midgaard"`, `10`, `true`), so a string and a number
//! that look alike stay two values. A message with no payload is `null`.
//!
//! What's in here came from the game, other players' names and chat
//! included (`comm.channel`), so it stays on this machine like the logs.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

pub use crate::trigger::{pairs, Fired, Trigger};

pub struct Hooks {
    db: Connection,
    /// How many pairs the list holds (those whose key isn't filtered),
    /// kept in step so the status bar's total doesn't cost a count of
    /// the table.
    count: usize,
    /// The filtered keys, as in the `filtered` table.
    filtered: BTreeSet<String>,
    /// The triggers, as in the `triggers` table, by key and value: every
    /// message is checked against them, so they're kept at hand.
    triggers: HashMap<(String, String), Trigger>,
}

/// An asset no longer in the Assets folder, cleared from the triggers
/// that named it: its path, and how many.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Cleared {
    pub path: String,
    pub hooks: usize,
}

/// What recording a message did: how many of its pairs were new to the
/// list, and the triggers it set off, in the message's order.
#[derive(Debug, Default, PartialEq)]
pub struct Recorded {
    pub added: usize,
    pub fired: Vec<Fired>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Pair {
    pub key: String,
    pub value: String,
    /// What it sets off, if anything.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
}

/// A page of the list: the first pairs matching a query, and how many
/// there are in all. Pairs whose key is filtered aren't in it or counted.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Listing {
    pub total: usize,
    pub matching: usize,
    pub pairs: Vec<Pair>,
}

/// A key moved out of the list, and how many values are recorded for it.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct FilteredKey {
    pub key: String,
    pub values: usize,
}

/// A page of the filtered keys, like `Listing`.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct FilteredListing {
    pub total: usize,
    pub matching: usize,
    pub keys: Vec<FilteredKey>,
}

fn failed(e: rusqlite::Error) -> String {
    format!("The hooks database couldn't be used. ({e})")
}

/// A filter as a LIKE pattern: the text anywhere, with LIKE's own
/// wildcards taken as the characters they are.
fn like_pattern(query: &str) -> String {
    let mut pattern = String::from("%");
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

impl Hooks {
    /// Opens the database file, creating it if it isn't there.
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::prepare(Connection::open(path).map_err(failed)?)
    }

    /// A database that lives only as long as this value: for the tests,
    /// and for playing on when the file can't be used.
    pub fn in_memory() -> Result<Self, String> {
        Self::prepare(Connection::open_in_memory().map_err(failed)?)
    }

    fn prepare(db: Connection) -> Result<Self, String> {
        // WAL with NORMAL: a write doesn't wait for the disk each time,
        // and a crash can lose the last few pairs but not the database.
        // WITHOUT ROWID: the table is its primary key, sorted as listed.
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS hooks (
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 PRIMARY KEY (key, value)
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS filtered (key TEXT NOT NULL PRIMARY KEY) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS triggers (
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 sfx TEXT,
                 bgm TEXT,
                 bgm_loop INTEGER NOT NULL DEFAULT 0,
                 art TEXT,
                 art_x INTEGER NOT NULL DEFAULT 1,
                 art_y INTEGER NOT NULL DEFAULT 1,
                 sfx_volume INTEGER NOT NULL DEFAULT 100,
                 bgm_volume INTEGER NOT NULL DEFAULT 100,
                 bgn TEXT,
                 bgn_volume INTEGER NOT NULL DEFAULT 100,
                 bgw TEXT,
                 bgw_volume INTEGER NOT NULL DEFAULT 100,
                 art_fade INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (key, value)
             ) WITHOUT ROWID;",
        )
        .map_err(failed)?;
        let filtered = db
            .prepare("SELECT key FROM filtered")
            .and_then(|mut select| select.query_map([], |row| row.get(0))?.collect::<Result<BTreeSet<String>, _>>())
            .map_err(failed)?;
        // The volumes, the ambience and the fade came later: a database
        // from before gets them, the volumes at full, no ambience, and
        // pictures that stay.
        let columns = db
            .prepare("SELECT name FROM pragma_table_info('triggers')")
            .and_then(|mut select| select.query_map([], |row| row.get(0))?.collect::<Result<Vec<String>, _>>())
            .map_err(failed)?;
        for (column, kind) in [
            ("sfx_volume", "INTEGER NOT NULL DEFAULT 100"),
            ("bgm_volume", "INTEGER NOT NULL DEFAULT 100"),
            ("bgn", "TEXT"),
            ("bgn_volume", "INTEGER NOT NULL DEFAULT 100"),
            ("bgw", "TEXT"),
            ("bgw_volume", "INTEGER NOT NULL DEFAULT 100"),
            ("art_fade", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            if !columns.iter().any(|c| c == column) {
                db.execute_batch(&format!("ALTER TABLE triggers ADD COLUMN {column} {kind}")).map_err(failed)?;
            }
        }
        let triggers = db
            .prepare("SELECT key, value, sfx, bgm, bgm_loop, art, art_x, art_y, sfx_volume, bgm_volume, bgn, bgn_volume, bgw, bgw_volume, art_fade FROM triggers")
            .and_then(|mut select| {
                select
                    .query_map([], |row| {
                        let trigger = Trigger {
                            sfx: row.get(2)?,
                            bgm: row.get(3)?,
                            bgm_loop: row.get::<_, i64>(4)? != 0,
                            art: row.get(5)?,
                            art_x: row.get::<_, i64>(6)?.clamp(1, u16::MAX.into()) as u16,
                            art_y: row.get::<_, i64>(7)?.clamp(1, u16::MAX.into()) as u16,
                            sfx_volume: row.get::<_, i64>(8)?.clamp(0, 100) as u8,
                            bgm_volume: row.get::<_, i64>(9)?.clamp(0, 100) as u8,
                            bgn: row.get(10)?,
                            bgn_volume: row.get::<_, i64>(11)?.clamp(0, 100) as u8,
                            bgw: row.get(12)?,
                            bgw_volume: row.get::<_, i64>(13)?.clamp(0, 100) as u8,
                            art_fade: row.get::<_, i64>(14)?.clamp(0, u16::MAX.into()) as u16,
                        };
                        Ok(((row.get(0)?, row.get(1)?), trigger))
                    })?
                    .collect::<Result<HashMap<_, _>, _>>()
            })
            .map_err(failed)?;
        let mut hooks = Hooks { db, count: 0, filtered, triggers };
        hooks.recount()?;
        Ok(hooks)
    }

    /// Counts the list again, after the filtered keys changed.
    fn recount(&mut self) -> Result<(), String> {
        let count: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM hooks WHERE key NOT IN (SELECT key FROM filtered)", [], |row| row.get(0))
            .map_err(failed)?;
        self.count = count as usize;
        Ok(())
    }

    /// Records one GMCP message. Returns how many of its pairs were new
    /// to the list (a filtered key's are recorded but not counted), and
    /// the triggers its pairs set off, whether they were new or not.
    pub fn record(&mut self, package: &str, data: &str) -> Result<Recorded, String> {
        let found = pairs(package, data);
        // One transaction a message: all of its pairs, or none.
        let tx = self.db.transaction().map_err(failed)?;
        let mut added = 0;
        {
            let mut insert = tx.prepare_cached("INSERT OR IGNORE INTO hooks (key, value) VALUES (?1, ?2)").map_err(failed)?;
            for (key, value) in &found {
                if insert.execute((key, value)).map_err(failed)? > 0 && !self.filtered.contains(key) {
                    added += 1;
                }
            }
        }
        tx.commit().map_err(failed)?;
        self.count += added;
        let fired = found
            .into_iter()
            .filter_map(|(key, value)| {
                let trigger = self.triggers.get(&(key.clone(), value.clone()))?.clone();
                Some(Fired { key, value, trigger })
            })
            .collect();
        Ok(Recorded { added, fired })
    }

    /// Puts Coupler's own pairs in the list before the game has said
    /// them, so a trigger can be set on each at once. Sets nothing off.
    /// Returns how many were new to the list.
    pub fn add_own(&mut self, key: &str, values: &[&str]) -> Result<usize, String> {
        let mut added = 0;
        for value in values {
            let value = serde_json::Value::String((*value).to_string()).to_string();
            if self.db.execute("INSERT OR IGNORE INTO hooks (key, value) VALUES (?1, ?2)", (key, &value)).map_err(failed)? > 0 && !self.filtered.contains(key) {
                added += 1;
            }
        }
        self.count += added;
        Ok(added)
    }

    /// Sets what a pair sets off; an empty trigger removes it.
    pub fn set_trigger(&mut self, key: &str, value: &str, trigger: Trigger) -> Result<(), String> {
        if trigger.is_empty() {
            self.db.execute("DELETE FROM triggers WHERE key = ?1 AND value = ?2", (key, value)).map_err(failed)?;
            self.triggers.remove(&(key.to_string(), value.to_string()));
            return Ok(());
        }
        self.db
            .execute(
                "INSERT OR REPLACE INTO triggers (key, value, sfx, bgm, bgm_loop, art, art_x, art_y, sfx_volume, bgm_volume, bgn, bgn_volume, bgw, bgw_volume, art_fade)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                (
                    key,
                    value,
                    &trigger.sfx,
                    &trigger.bgm,
                    trigger.bgm_loop,
                    &trigger.art,
                    trigger.art_x.max(1),
                    trigger.art_y.max(1),
                    trigger.sfx_volume.min(100),
                    trigger.bgm_volume.min(100),
                    &trigger.bgn,
                    trigger.bgn_volume.min(100),
                    &trigger.bgw,
                    trigger.bgw_volume.min(100),
                    trigger.art_fade,
                ),
            )
            .map_err(failed)?;
        self.triggers.insert((key.to_string(), value.to_string()), trigger);
        Ok(())
    }

    /// What a pair sets off, if anything.
    pub fn trigger(&self, key: &str, value: &str) -> Option<&Trigger> {
        self.triggers.get(&(key.to_string(), value.to_string()))
    }

    /// Every trigger, by its pair, in order: for the web mirror
    /// (`examples/web_mirror.rs`).
    pub fn triggers(&self) -> Vec<(&str, &str, &Trigger)> {
        let mut all: Vec<_> = self.triggers.iter().map(|((k, v), t)| (k.as_str(), v.as_str(), t)).collect();
        all.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        all
    }

    /// How many triggers use each asset, by its path.
    pub fn asset_uses(&self) -> HashMap<String, usize> {
        let mut uses = HashMap::new();
        for trigger in self.triggers.values() {
            for path in [&trigger.sfx, &trigger.bgm, &trigger.art, &trigger.bgn, &trigger.bgw].into_iter().flatten() {
                *uses.entry(path.clone()).or_insert(0) += 1;
            }
        }
        uses
    }

    /// Clears every asset `exists` says is gone from the triggers naming
    /// it (a trigger left with nothing is removed). Returns each asset
    /// cleared and how many triggers named it, in order of path.
    pub fn clear_missing(&mut self, exists: impl Fn(&str) -> bool) -> Result<Vec<Cleared>, String> {
        let mut cleared: BTreeMap<String, usize> = BTreeMap::new();
        let mut changed = Vec::new();
        for ((key, value), trigger) in &self.triggers {
            let mut kept = trigger.clone();
            for slot in [&mut kept.sfx, &mut kept.bgm, &mut kept.art, &mut kept.bgn, &mut kept.bgw] {
                if slot.as_deref().is_some_and(|path| !exists(path)) {
                    *cleared.entry(slot.take().unwrap_or_default()).or_insert(0) += 1;
                }
            }
            if kept != *trigger {
                changed.push((key.clone(), value.clone(), kept));
            }
        }
        for (key, value, trigger) in changed {
            self.set_trigger(&key, &value, trigger)?;
        }
        Ok(cleared.into_iter().map(|(path, hooks)| Cleared { path, hooks }).collect())
    }

    /// Moves a key out of the list and into the filtered keys, or back.
    /// Only a key that has been recorded can be filtered.
    pub fn set_filtered(&mut self, key: &str, filtered: bool) -> Result<(), String> {
        let sql = if filtered {
            "INSERT OR IGNORE INTO filtered (key) SELECT ?1 WHERE EXISTS (SELECT 1 FROM hooks WHERE key = ?1)"
        } else {
            "DELETE FROM filtered WHERE key = ?1"
        };
        let changed = self.db.execute(sql, [key]).map_err(failed)? > 0;
        if changed && filtered {
            self.filtered.insert(key.to_string());
        } else if changed {
            self.filtered.remove(key);
        }
        self.recount()
    }

    /// How many unique pairs the list holds.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The first `limit` pairs whose key or value contains `query`, in
    /// order; every pair when it's empty. Letters match in either case
    /// (A to Z only: that's what SQLite's LIKE folds). Filtered keys are
    /// left out.
    pub fn list(&self, query: &str, limit: usize) -> Result<Listing, String> {
        const WHERE: &str = "WHERE (key LIKE ?1 ESCAPE '\\' OR value LIKE ?1 ESCAPE '\\') AND key NOT IN (SELECT key FROM filtered)";
        let pattern = like_pattern(query.trim());
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let matching: i64 =
            self.db.query_row(&format!("SELECT COUNT(*) FROM hooks {WHERE}"), [&pattern], |row| row.get(0)).map_err(failed)?;
        let mut select = self.db.prepare_cached(&format!("SELECT key, value FROM hooks {WHERE} ORDER BY key, value LIMIT ?2")).map_err(failed)?;
        let pairs = select
            .query_map((&pattern, limit), |row| {
                let (key, value): (String, String) = (row.get(0)?, row.get(1)?);
                let trigger = self.triggers.get(&(key.clone(), value.clone())).cloned();
                Ok(Pair { key, value, trigger })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed)?;
        Ok(Listing { total: self.count, matching: matching as usize, pairs })
    }

    /// The first `limit` filtered keys containing `query`, in order,
    /// each with how many values it has.
    pub fn filtered_list(&self, query: &str, limit: usize) -> Result<FilteredListing, String> {
        let pattern = like_pattern(query.trim());
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut select = self
            .db
            .prepare_cached(
                "SELECT key, (SELECT COUNT(*) FROM hooks WHERE hooks.key = filtered.key)
                 FROM filtered WHERE key LIKE ?1 ESCAPE '\\' ORDER BY key",
            )
            .map_err(failed)?;
        let mut keys = select
            .query_map([&pattern], |row| Ok(FilteredKey { key: row.get(0)?, values: row.get::<_, i64>(1)? as usize }))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed)?;
        let matching = keys.len();
        keys.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(FilteredListing { total: self.filtered.len(), matching, keys })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(hooks: &Hooks) -> Vec<(String, String)> {
        hooks.list("", usize::MAX).unwrap().pairs.into_iter().map(|p| (p.key, p.value)).collect()
    }

    fn pair(key: &str, value: &str) -> (String, String) {
        (key.to_string(), value.to_string())
    }

    #[test]
    fn a_message_becomes_a_pair_per_field() {
        let mut hooks = Hooks::in_memory().unwrap();
        assert_eq!(hooks.record("Char.Vitals", r#"{"hp":10,"maxhp":20}"#).unwrap().added, 2);
        assert_eq!(pairs(&hooks), vec![pair("char.vitals.hp", "10"), pair("char.vitals.maxhp", "20")]);
    }

    #[test]
    fn only_unique_pairs_are_kept() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("char.vitals", r#"{"hp":10,"maxhp":20}"#).unwrap();
        assert_eq!(hooks.record("char.vitals", r#"{"hp":10,"maxhp":20}"#).unwrap().added, 0);
        assert_eq!(hooks.record("Char.Vitals", r#"{"hp":12,"maxhp":20}"#).unwrap().added, 1);
        assert_eq!(hooks.len(), 3);
        assert_eq!(pairs(&hooks), vec![pair("char.vitals.hp", "10"), pair("char.vitals.hp", "12"), pair("char.vitals.maxhp", "20")]);
    }

    #[test]
    fn nested_objects_and_arrays_are_flattened() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("room.info", r#"{"zone":"Midgaard","idexits":{"N":"Midgaard#3001"},"coord":{"id":0,"x":-1}}"#).unwrap();
        hooks.record("room.mobiles", r#"{"npcs":[{"an orc":"an orc.2"},{"a rat":"a rat"}]}"#).unwrap();
        hooks.record("char.login.default", r#"{"type":["password-credentials"]}"#).unwrap();
        assert_eq!(
            pairs(&hooks),
            vec![
                pair("char.login.default.type", r#""password-credentials""#),
                pair("room.info.coord.id", "0"),
                pair("room.info.coord.x", "-1"),
                pair("room.info.idexits.N", r##""Midgaard#3001""##),
                pair("room.info.zone", r#""Midgaard""#),
                pair("room.mobiles.npcs.a rat", r#""a rat""#),
                pair("room.mobiles.npcs.an orc", r#""an orc.2""#),
            ]
        );
    }

    #[test]
    fn payloads_that_arent_objects_are_kept_under_the_package() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("core.ping", "").unwrap();
        hooks.record("room.wrongdir", r#""N""#).unwrap();
        hooks.record("room.info", r#"{"details":"","extradata":{},"list":[]}"#).unwrap();
        hooks.record("odd.one", "not json").unwrap();
        assert_eq!(
            pairs(&hooks),
            vec![
                pair("core.ping", "null"),
                pair("odd.one", r#""not json""#),
                pair("room.info.details", r#""""#),
                pair("room.info.extradata", "{}"),
                pair("room.info.list", "[]"),
                pair("room.wrongdir", r#""N""#),
            ]
        );
    }

    #[test]
    fn a_string_and_a_number_that_look_alike_are_two_values() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("char.status", r#"{"level":5}"#).unwrap();
        assert_eq!(hooks.record("char.status", r#"{"level":"5"}"#).unwrap().added, 1);
    }

    #[test]
    fn the_list_filters_by_key_or_value_and_stops_at_the_limit() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("char.vitals", r#"{"hp":10,"mana":30}"#).unwrap();
        hooks.record("room.info", r#"{"zone":"Midgaard","name":"The Temple"}"#).unwrap();
        let by_key = hooks.list("VITALS", 10).unwrap();
        assert_eq!((by_key.total, by_key.matching, by_key.pairs.len()), (4, 2, 2));
        let by_value = hooks.list("midg", 10).unwrap();
        assert_eq!(by_value.pairs, vec![Pair { key: "room.info.zone".into(), value: r#""Midgaard""#.into(), trigger: None }]);
        let cut = hooks.list("", 3).unwrap();
        assert_eq!((cut.matching, cut.pairs.len()), (4, 3));
    }

    #[test]
    fn a_filter_takes_likes_wildcards_as_plain_characters() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("char.status", r#"{"stink_pct":"50%","align":"good"}"#).unwrap();
        assert_eq!(hooks.list("%", 10).unwrap().matching, 1);
        assert_eq!(hooks.list("k_p", 10).unwrap().matching, 1);
        assert_eq!(hooks.list("a_i", 10).unwrap().matching, 0);
    }

    #[test]
    fn a_filtered_key_leaves_the_list_and_comes_back() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("char.vitals", r#"{"hp":10,"mana":30}"#).unwrap();
        hooks.record("char.vitals", r#"{"hp":12}"#).unwrap();
        hooks.set_filtered("char.vitals.hp", true).unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(pairs(&hooks), vec![pair("char.vitals.mana", "30")]);
        assert_eq!(hooks.list("hp", 10).unwrap().matching, 0);
        let filtered = hooks.filtered_list("", 10).unwrap();
        assert_eq!((filtered.total, filtered.matching), (1, 1));
        assert_eq!(filtered.keys, vec![FilteredKey { key: "char.vitals.hp".into(), values: 2 }]);
        assert_eq!(hooks.filtered_list("mana", 10).unwrap().matching, 0);

        // Still recorded while filtered, but not new to the list.
        assert_eq!(hooks.record("char.vitals", r#"{"hp":14,"mana":31}"#).unwrap().added, 1);
        assert_eq!(hooks.len(), 2);

        hooks.set_filtered("char.vitals.hp", false).unwrap();
        assert_eq!(hooks.len(), 5);
        assert_eq!(hooks.list("hp", 10).unwrap().matching, 3);
        assert_eq!(hooks.filtered_list("", 10).unwrap().total, 0);
    }

    #[test]
    fn only_a_recorded_key_can_be_filtered() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("char.vitals", r#"{"hp":10}"#).unwrap();
        hooks.set_filtered("never.seen", true).unwrap();
        hooks.set_filtered("char.vitals.hp", true).unwrap();
        hooks.set_filtered("char.vitals.hp", true).unwrap();
        assert_eq!(hooks.filtered_list("", 10).unwrap().total, 1);
    }

    /// A file in the temporary directory, removed when the test ends.
    struct TempDb(std::path::PathBuf);

    impl TempDb {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("coupler-hooks-{name}-{}.sqlite", std::process::id()));
            let db = TempDb(path);
            db.remove();
            db
        }

        fn remove(&self) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
            }
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            self.remove();
        }
    }

    #[test]
    fn closing_and_opening_gives_the_same_hooks() {
        let file = TempDb::new("reopen");
        let before = {
            let mut hooks = Hooks::open(&file.0).unwrap();
            hooks.record("char.vitals", r#"{"hp":10,"ratio":1.5}"#).unwrap();
            hooks.record("char.vitals", r#"{"hp":12}"#).unwrap();
            hooks.record("comm.channel", r#"{"chan":"GOSSIP","msg":"Bob GOSSIPs 'hi \"all\"'"}"#).unwrap();
            hooks.record("core.ping", "").unwrap();
            hooks.set_filtered("core.ping", true).unwrap();
            pairs(&hooks)
        };
        let mut back = Hooks::open(&file.0).unwrap();
        assert_eq!(back.len(), 5);
        assert_eq!(pairs(&back), before);
        assert_eq!(back.filtered_list("", 10).unwrap().keys, vec![FilteredKey { key: "core.ping".into(), values: 1 }]);
        // What was there before is still a duplicate.
        assert_eq!(back.record("char.vitals", r#"{"hp":10}"#).unwrap().added, 0);
    }

    fn door() -> Trigger {
        Trigger {
            sfx: Some("wav/door.wav".into()),
            sfx_volume: 40,
            bgm: Some("mid/town.mid".into()),
            bgm_loop: true,
            bgm_volume: 75,
            art: Some("png/gate.png".into()),
            art_x: 10,
            art_y: 5,
            art_fade: 8,
            bgn: Some("ogg/crowd.ogg".into()),
            bgn_volume: 30,
            bgw: None,
            bgw_volume: 100,
        }
    }

    #[test]
    fn a_trigger_goes_off_every_time_its_pair_comes() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.record("room.info", r#"{"zone":"Midgaard","name":"Gate"}"#).unwrap();
        hooks.set_trigger("room.info.zone", r#""Midgaard""#, door()).unwrap();
        let fired = Fired { key: "room.info.zone".into(), value: r#""Midgaard""#.into(), trigger: door() };
        // Not new to the list, and it still goes off, each time.
        for _ in 0..2 {
            let recorded = hooks.record("Room.Info", r#"{"zone":"Midgaard","name":"Square"}"#).unwrap();
            assert_eq!(recorded.fired, vec![fired.clone()]);
        }
        assert!(hooks.record("room.info", r#"{"zone":"Elsewhere"}"#).unwrap().fired.is_empty());
        let listed = hooks.list("zone", 10).unwrap().pairs;
        assert_eq!(listed.iter().find(|p| p.value == r#""Midgaard""#).unwrap().trigger, Some(door()));
    }

    #[test]
    fn couplers_own_pairs_are_listed_before_they_come() {
        let mut hooks = Hooks::in_memory().unwrap();
        assert_eq!(hooks.add_own("coupler.time", &["dawn", "night"]).unwrap(), 2);
        assert_eq!(hooks.add_own("coupler.time", &["dawn", "night"]).unwrap(), 0);
        assert_eq!(hooks.len(), 2);
        assert_eq!(pairs(&hooks), vec![pair("coupler.time", r#""dawn""#), pair("coupler.time", r#""night""#)]);
        // Then the same pair said goes off like any other.
        hooks.set_trigger("coupler.time", r#""dawn""#, door()).unwrap();
        assert_eq!(hooks.record("coupler.time", r#""dawn""#).unwrap().fired.len(), 1);
    }

    #[test]
    fn an_empty_trigger_is_removed() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.set_trigger("char.vitals.hp", "10", door()).unwrap();
        hooks.set_trigger("char.vitals.hp", "10", Trigger::default()).unwrap();
        assert!(hooks.record("char.vitals", r#"{"hp":10}"#).unwrap().fired.is_empty());
        assert_eq!(hooks.list("", 10).unwrap().pairs[0].trigger, None);
    }

    #[test]
    fn uses_are_counted_per_asset() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.set_trigger("char.vitals.hp", "10", door()).unwrap();
        hooks.set_trigger("char.vitals.hp", "11", Trigger { sfx: Some("wav/door.wav".into()), ..Trigger::default() }).unwrap();
        let uses = hooks.asset_uses();
        assert_eq!(uses.get("wav/door.wav"), Some(&2));
        assert_eq!(uses.get("png/gate.png"), Some(&1));
        assert_eq!(uses.get("wav/other.wav"), None);
    }

    #[test]
    fn a_missing_asset_is_cleared_from_every_trigger() {
        let mut hooks = Hooks::in_memory().unwrap();
        hooks.set_trigger("char.vitals.hp", "10", door()).unwrap();
        hooks.set_trigger("char.vitals.hp", "11", Trigger { sfx: Some("wav/door.wav".into()), ..Trigger::default() }).unwrap();
        let cleared = hooks.clear_missing(|path| path != "wav/door.wav").unwrap();
        assert_eq!(cleared, vec![Cleared { path: "wav/door.wav".into(), hooks: 2 }]);
        // The one with nothing left is gone; the other keeps the rest.
        let fired = hooks.record("char.vitals", r#"{"hp":10}"#).unwrap().fired;
        assert_eq!(fired[0].trigger, Trigger { sfx: None, ..door() });
        assert!(hooks.record("char.vitals", r#"{"hp":11}"#).unwrap().fired.is_empty());
        assert!(hooks.clear_missing(|_| true).unwrap().is_empty());
    }

    #[test]
    fn a_database_from_before_the_volumes_gets_them_at_full() {
        let file = TempDb::new("volumes");
        {
            let db = Connection::open(&file.0).unwrap();
            db.execute_batch(
                "CREATE TABLE triggers (key TEXT NOT NULL, value TEXT NOT NULL, sfx TEXT, bgm TEXT,
                     bgm_loop INTEGER NOT NULL DEFAULT 0, art TEXT, art_x INTEGER NOT NULL DEFAULT 1,
                     art_y INTEGER NOT NULL DEFAULT 1, PRIMARY KEY (key, value)) WITHOUT ROWID;
                 INSERT INTO triggers (key, value, sfx) VALUES ('char.vitals.hp', '10', 'wav/door.wav');",
            )
            .unwrap();
        }
        let mut hooks = Hooks::open(&file.0).unwrap();
        let fired = hooks.record("char.vitals", r#"{"hp":10}"#).unwrap().fired;
        assert_eq!((fired[0].trigger.sfx_volume, fired[0].trigger.bgm_volume), (100, 100));
        assert_eq!((fired[0].trigger.bgn.as_deref(), fired[0].trigger.bgw_volume), (None, 100));
        assert_eq!(fired[0].trigger.art_fade, 0);
    }

    #[test]
    fn triggers_are_kept_across_a_reopen() {
        let file = TempDb::new("triggers");
        {
            let mut hooks = Hooks::open(&file.0).unwrap();
            hooks.set_trigger("char.vitals.hp", "10", door()).unwrap();
        }
        let mut back = Hooks::open(&file.0).unwrap();
        assert_eq!(back.record("char.vitals", r#"{"hp":10}"#).unwrap().fired.len(), 1);
        assert_eq!(back.record("char.vitals", r#"{"hp":10}"#).unwrap().fired[0].trigger, door());
    }

    #[test]
    fn a_file_that_isnt_a_database_is_refused() {
        let file = TempDb::new("garbage");
        std::fs::write(&file.0, "nonsense, and long enough not to pass for an empty database file").unwrap();
        assert!(Hooks::open(&file.0).is_err());
    }
}

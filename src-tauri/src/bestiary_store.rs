//! The desktop's Bestiary and Items (`bestiary.rs`): one SQLite
//! database, `bestiary.sqlite` in app data. The web build has its own
//! `bestiary_store.rs` with the same methods; `bestiary.rs`'s tests run
//! against both.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::bestiary::{item_key, item_name, narrow, race_in_name, race_named, seems, Counted, Creature, Fact, ItemEntry, ItemRow, MobArea, MobEntry, MobRow};
use crate::senses::letters_and_digits;

pub struct Bestiary {
    db: Connection,
}

fn failed(e: rusqlite::Error) -> String {
    format!("The Bestiary couldn't be used. ({e})")
}

/// A search's LIKE pattern: its words in order, anything between.
fn like(query: &str) -> String {
    let words: Vec<String> = query.split_whitespace().map(|w| w.replace(['%', '_', '\\'], "")).collect();
    format!("%{}%", words.join("%"))
}

impl Bestiary {
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::prepare(Connection::open(path).map_err(failed)?)
    }

    /// A Bestiary that lives only as long as this value: for the tests,
    /// and for playing on when the file can't be used.
    pub fn in_memory() -> Result<Self, String> {
        Self::prepare(Connection::open_in_memory().map_err(failed)?)
    }

    fn prepare(db: Connection) -> Result<Self, String> {
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS mobs (
                 world TEXT NOT NULL, key TEXT NOT NULL, name TEXT NOT NULL,
                 race TEXT, race_seen INTEGER NOT NULL DEFAULT 0,
                 sex TEXT, height INTEGER, weight INTEGER, body TEXT, description TEXT,
                 first_seen INTEGER NOT NULL, last_seen INTEGER NOT NULL,
                 deaths INTEGER NOT NULL DEFAULT 0,
                 auto_looked INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (world, key));
             CREATE TABLE IF NOT EXISTS mob_areas (
                 world TEXT NOT NULL, key TEXT NOT NULL, area TEXT NOT NULL,
                 seen INTEGER NOT NULL DEFAULT 0, last_seen INTEGER NOT NULL,
                 level_low INTEGER, level_high INTEGER, max_hp INTEGER, alignment TEXT,
                 PRIMARY KEY (world, key, area));
             CREATE TABLE IF NOT EXISTS mob_stats (
                 world TEXT NOT NULL, key TEXT NOT NULL, stat TEXT NOT NULL,
                 PRIMARY KEY (world, key, stat));
             CREATE TABLE IF NOT EXISTS mob_abilities (
                 world TEXT NOT NULL, key TEXT NOT NULL, ability TEXT NOT NULL, kind TEXT NOT NULL,
                 count INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (world, key, ability));
             CREATE TABLE IF NOT EXISTS mob_items (
                 world TEXT NOT NULL, key TEXT NOT NULL, item TEXT NOT NULL, how TEXT NOT NULL,
                 place TEXT NOT NULL DEFAULT '', count INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (world, key, item, how, place));
             CREATE TABLE IF NOT EXISTS items (
                 world TEXT NOT NULL, key TEXT NOT NULL, name TEXT NOT NULL,
                 level_low INTEGER, level_high INTEGER, weight_low INTEGER, weight_high INTEGER,
                 material TEXT, description TEXT, container INTEGER NOT NULL DEFAULT 0,
                 first_seen INTEGER NOT NULL, last_seen INTEGER NOT NULL,
                 PRIMARY KEY (world, key));
             CREATE TABLE IF NOT EXISTS item_auras (
                 world TEXT NOT NULL, key TEXT NOT NULL, aura TEXT NOT NULL,
                 PRIMARY KEY (world, key, aura));
             CREATE TABLE IF NOT EXISTS item_areas (
                 world TEXT NOT NULL, key TEXT NOT NULL, area TEXT NOT NULL,
                 seen INTEGER NOT NULL DEFAULT 0, last_seen INTEGER NOT NULL,
                 PRIMARY KEY (world, key, area));",
        )
        .map_err(failed)?;
        // Made before Coupler looked by itself: the column comes after.
        let columns: Vec<String> = db.prepare("SELECT name FROM pragma_table_info('mobs')").and_then(|mut s| s.query_map([], |r| r.get(0))?.collect()).map_err(failed)?;
        if !columns.iter().any(|c| c == "auto_looked") {
            db.execute("ALTER TABLE mobs ADD COLUMN auto_looked INTEGER NOT NULL DEFAULT 0", []).map_err(failed)?;
        }
        let b = Bestiary { db };
        b.rekey_items()?;
        Ok(b)
    }

    /// Kept before an item's key left its colors out and a bundle its
    /// weight (`item_key`): each item under its key now, the ones that
    /// share it made one (seen in each area added up, the widest levels
    /// and weights, what's known of either).
    fn rekey_items(&self) -> Result<(), String> {
        let db = &self.db;
        let rows: Vec<(String, String, String)> =
            db.prepare("SELECT world, key, name FROM items").and_then(|mut s| s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect()).map_err(failed)?;
        let moves: Vec<_> = rows.into_iter().filter_map(|(world, key, name)| {
            let (to, kept) = (item_key(&name), item_name(&name));
            (to != key || kept != name).then_some((world, key, to, kept))
        }).collect();
        if moves.is_empty() {
            return Ok(());
        }
        let tx = db.unchecked_transaction().map_err(failed)?;
        for (world, from, to, name) in moves {
            if from == to {
                tx.execute("UPDATE items SET name = ?3 WHERE world = ?1 AND key = ?2", params![world, from, name]).map_err(failed)?;
                continue;
            }
            tx.execute(
                "INSERT INTO items (world, key, name, level_low, level_high, weight_low, weight_high, material, description, container, first_seen, last_seen)
                 SELECT world, ?3, ?4, level_low, level_high, weight_low, weight_high, material, description, container, first_seen, last_seen
                 FROM items WHERE world = ?1 AND key = ?2
                 ON CONFLICT (world, key) DO UPDATE SET
                     name = CASE WHEN instr(excluded.name, '^') > 0 AND instr(name, '^') = 0 THEN excluded.name ELSE name END,
                     level_low = min(coalesce(level_low, excluded.level_low), coalesce(excluded.level_low, level_low)),
                     level_high = max(coalesce(level_high, excluded.level_high), coalesce(excluded.level_high, level_high)),
                     weight_low = min(coalesce(weight_low, excluded.weight_low), coalesce(excluded.weight_low, weight_low)),
                     weight_high = max(coalesce(weight_high, excluded.weight_high), coalesce(excluded.weight_high, weight_high)),
                     material = coalesce(material, excluded.material), description = coalesce(description, excluded.description),
                     container = max(container, excluded.container),
                     first_seen = min(first_seen, excluded.first_seen), last_seen = max(last_seen, excluded.last_seen)",
                params![world, from, to, name],
            )
            .map_err(failed)?;
            tx.execute("INSERT OR IGNORE INTO item_auras (world, key, aura) SELECT world, ?3, aura FROM item_auras WHERE world = ?1 AND key = ?2", params![world, from, to]).map_err(failed)?;
            tx.execute(
                "INSERT INTO item_areas (world, key, area, seen, last_seen) SELECT world, ?3, area, seen, last_seen FROM item_areas WHERE world = ?1 AND key = ?2 AND true
                 ON CONFLICT (world, key, area) DO UPDATE SET seen = seen + excluded.seen, last_seen = max(last_seen, excluded.last_seen)",
                params![world, from, to],
            )
            .map_err(failed)?;
            tx.execute(
                "INSERT INTO mob_items (world, key, item, how, place, count) SELECT world, key, ?3, how, place, count FROM mob_items WHERE world = ?1 AND item = ?2 AND true
                 ON CONFLICT (world, key, item, how, place) DO UPDATE SET count = CASE WHEN how = 'carry' THEN max(count, excluded.count) ELSE count + excluded.count END",
                params![world, from, to],
            )
            .map_err(failed)?;
            for table in ["items", "item_auras", "item_areas"] {
                tx.execute(&format!("DELETE FROM {table} WHERE world = ?1 AND key = ?2"), params![world, from]).map_err(failed)?;
            }
            tx.execute("DELETE FROM mob_items WHERE world = ?1 AND item = ?2", params![world, from]).map_err(failed)?;
        }
        tx.commit().map_err(failed)
    }

    /// Makes sure a creature has its entry, its race read from its name
    /// when first met; returns its key.
    fn mob(&self, world: &str, name: &str, at: i64) -> Result<String, String> {
        let key = letters_and_digits(name);
        let race = race_in_name(name).map(|r| r.name);
        self.db
            .execute(
                "INSERT INTO mobs (world, key, name, race, first_seen, last_seen) VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                 ON CONFLICT (world, key) DO UPDATE SET last_seen = ?5",
                params![world, key, name, race, at],
            )
            .map_err(failed)?;
        Ok(key)
    }

    /// Makes sure an item has its entry (a bundle by what it holds,
    /// `item_name`), its name in the game's colors once one comes; returns
    /// its key.
    fn item(&self, world: &str, name: &str, at: i64) -> Result<String, String> {
        let name = item_name(name);
        let key = item_key(&name);
        self.db
            .execute(
                "INSERT INTO items (world, key, name, first_seen, last_seen) VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT (world, key) DO UPDATE SET last_seen = ?4,
                     name = CASE WHEN instr(?3, '^') > 0 AND instr(name, '^') = 0 THEN ?3 ELSE name END",
                params![world, key, name, at],
            )
            .map_err(failed)?;
        Ok(key)
    }

    fn mob_area(&self, world: &str, key: &str, area: Option<&str>, at: i64) -> Result<Option<String>, String> {
        let Some(area) = area else { return Ok(None) };
        self.db
            .execute(
                "INSERT INTO mob_areas (world, key, area, last_seen) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (world, key, area) DO UPDATE SET last_seen = ?4",
                params![world, key, area, at],
            )
            .map_err(failed)?;
        Ok(Some(area.to_string()))
    }

    /// Keeps what a fact says. `area` is where the player is.
    pub fn record(&self, world: &str, area: Option<&str>, fact: &Fact, at: i64) -> Result<(), String> {
        let db = &self.db;
        match fact {
            Fact::Seen { name } => {
                let key = self.mob(world, name, at)?;
                if let Some(area) = self.mob_area(world, &key, area, at)? {
                    db.execute("UPDATE mob_areas SET seen = seen + 1 WHERE world = ?1 AND key = ?2 AND area = ?3", params![world, key, area]).map_err(failed)?;
                }
            }
            Fact::ItemSeen { name, container } => {
                let key = self.item(world, name, at)?;
                if *container {
                    db.execute("UPDATE items SET container = 1 WHERE world = ?1 AND key = ?2", params![world, key]).map_err(failed)?;
                }
                if let Some(area) = area {
                    db.execute(
                        "INSERT INTO item_areas (world, key, area, seen, last_seen) VALUES (?1, ?2, ?3, 1, ?4)
                         ON CONFLICT (world, key, area) DO UPDATE SET seen = seen + 1, last_seen = ?4",
                        params![world, key, area, at],
                    )
                    .map_err(failed)?;
                }
            }
            Fact::Race { name, race } => {
                let key = self.mob(world, name, at)?;
                let race = race_named(race).map_or(race.as_str(), |r| r.name);
                db.execute("UPDATE mobs SET race = ?3, race_seen = 1 WHERE world = ?1 AND key = ?2", params![world, key, race]).map_err(failed)?;
            }
            Fact::Size { name, sex, height, weight } => {
                let key = self.mob(world, name, at)?;
                db.execute("UPDATE mobs SET sex = ?3, height = ?4, weight = ?5 WHERE world = ?1 AND key = ?2", params![world, key, sex, height, weight]).map_err(failed)?;
            }
            Fact::Body { name, body } => {
                let key = self.mob(world, name, at)?;
                db.execute("UPDATE mobs SET body = ?3 WHERE world = ?1 AND key = ?2", params![world, key, body]).map_err(failed)?;
            }
            Fact::Stat { name, stat } => {
                let key = self.mob(world, name, at)?;
                db.execute("INSERT OR IGNORE INTO mob_stats (world, key, stat) VALUES (?1, ?2, ?3)", params![world, key, stat]).map_err(failed)?;
            }
            Fact::Description { name, text } => {
                let key = self.mob(world, name, at)?;
                db.execute("UPDATE mobs SET description = ?3 WHERE world = ?1 AND key = ?2", params![world, key, text]).map_err(failed)?;
            }
            Fact::ItemAura { name, aura } => {
                let key = self.item(world, name, at)?;
                db.execute("INSERT OR IGNORE INTO item_auras (world, key, aura) VALUES (?1, ?2, ?3)", params![world, key, aura]).map_err(failed)?;
            }
            Fact::Wears { name, place, item } => {
                let key = self.mob(world, name, at)?;
                let item_key = self.item(world, item, at)?;
                db.execute(
                    "INSERT INTO mob_items (world, key, item, how, place, count) VALUES (?1, ?2, ?3, 'worn', ?4, 1)
                     ON CONFLICT (world, key, item, how, place) DO NOTHING",
                    params![world, key, item_key, place],
                )
                .map_err(failed)?;
            }
            Fact::Level { name, low, high } => {
                let key = self.mob(world, name, at)?;
                let Some(area) = self.mob_area(world, &key, area, at)? else { return Ok(()) };
                let old: (Option<i64>, Option<i64>) = db
                    .query_row("SELECT level_low, level_high FROM mob_areas WHERE world = ?1 AND key = ?2 AND area = ?3", params![world, key, area], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(failed)?;
                let (lo, hi) = narrow(old, (*low, *high));
                // "Far superior" has no top, "vastly inferior" no bottom but 1.
                let hi = (hi < i64::MAX / 8).then_some(hi);
                db.execute("UPDATE mob_areas SET level_low = ?4, level_high = ?5 WHERE world = ?1 AND key = ?2 AND area = ?3", params![world, key, area, lo, hi]).map_err(failed)?;
            }
            Fact::Health { name, max } => {
                let key = self.mob(world, name, at)?;
                if let Some(area) = self.mob_area(world, &key, area, at)? {
                    db.execute("UPDATE mob_areas SET max_hp = ?4 WHERE world = ?1 AND key = ?2 AND area = ?3", params![world, key, area, max]).map_err(failed)?;
                }
            }
            Fact::Alignment { name, alignment } => {
                let key = self.mob(world, name, at)?;
                if let Some(area) = self.mob_area(world, &key, area, at)? {
                    db.execute("UPDATE mob_areas SET alignment = ?4 WHERE world = ?1 AND key = ?2 AND area = ?3", params![world, key, area, alignment]).map_err(failed)?;
                }
            }
            Fact::Ability { name, ability, kind } => {
                let key = self.mob(world, name, at)?;
                db.execute(
                    "INSERT INTO mob_abilities (world, key, ability, kind, count) VALUES (?1, ?2, ?3, ?4, 1)
                     ON CONFLICT (world, key, ability) DO UPDATE SET count = count + 1",
                    params![world, key, ability, kind],
                )
                .map_err(failed)?;
            }
            Fact::Died { name } => {
                let key = self.mob(world, name, at)?;
                db.execute("UPDATE mobs SET deaths = deaths + 1 WHERE world = ?1 AND key = ?2", params![world, key]).map_err(failed)?;
            }
            Fact::Dropped { name, item } => {
                let key = self.mob(world, name, at)?;
                let item_key = self.item(world, item, at)?;
                db.execute(
                    "INSERT INTO mob_items (world, key, item, how, place, count) VALUES (?1, ?2, ?3, 'drop', '', 1)
                     ON CONFLICT (world, key, item, how, place) DO UPDATE SET count = count + 1",
                    params![world, key, item_key],
                )
                .map_err(failed)?;
            }
            Fact::Butchered { name, item } => {
                let key = self.mob(world, name, at)?;
                let item_key = self.item(world, item, at)?;
                db.execute(
                    "INSERT INTO mob_items (world, key, item, how, place, count) VALUES (?1, ?2, ?3, 'butcher', '', 1)
                     ON CONFLICT (world, key, item, how, place) DO UPDATE SET count = count + 1",
                    params![world, key, item_key],
                )
                .map_err(failed)?;
            }
            Fact::Carried { name, item, count } => {
                let key = self.mob(world, name, at)?;
                let item_key = self.item(world, item, at)?;
                db.execute(
                    "INSERT INTO mob_items (world, key, item, how, place, count) VALUES (?1, ?2, ?3, 'carry', '', ?4)
                     ON CONFLICT (world, key, item, how, place) DO UPDATE SET count = max(count, ?4)",
                    params![world, key, item_key, count],
                )
                .map_err(failed)?;
            }
            Fact::Weight { name, weight } => {
                let key = self.mob(world, name, at)?;
                db.execute("UPDATE mobs SET weight = ?3 WHERE world = ?1 AND key = ?2", params![world, key, weight]).map_err(failed)?;
            }
            Fact::Item { name, level, weight, material, description } => {
                let key = self.item(world, name, at)?;
                db.execute(
                    "UPDATE items SET level_low = coalesce(?3, level_low), level_high = coalesce(?4, level_high),
                         weight_low = coalesce(?5, weight_low), weight_high = coalesce(?6, weight_high),
                         material = coalesce(?7, material), description = coalesce(?8, description)
                     WHERE world = ?1 AND key = ?2",
                    params![world, key, level.map(|l| l.0), level.map(|l| l.1), weight.map(|w| w.0), weight.map(|w| w.1), material, description],
                )
                .map_err(failed)?;
            }
        }
        Ok(())
    }

    /// Whether a creature has never been looked at, by the player or by
    /// Coupler: nothing a look tells is known, and Coupler hasn't tried.
    pub fn wants_look(&self, world: &str, name: &str) -> bool {
        let known: Option<bool> = self
            .db
            .query_row(
                "SELECT height IS NOT NULL OR description IS NOT NULL OR auto_looked != 0 FROM mobs WHERE world = ?1 AND key = ?2",
                params![world, letters_and_digits(name)],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        known != Some(true)
    }

    /// Coupler looked at a creature by itself: never again.
    pub fn looked(&self, world: &str, name: &str, at: i64) -> Result<(), String> {
        let key = self.mob(world, name, at)?;
        self.db.execute("UPDATE mobs SET auto_looked = ?3 WHERE world = ?1 AND key = ?2", params![world, key, at]).map_err(failed)?;
        Ok(())
    }

    /// A creature's cry, from its race's category.
    pub fn creature(&self, world: &str, name: &str) -> Option<Creature> {
        let race: Option<String> = self
            .db
            .query_row("SELECT race FROM mobs WHERE world = ?1 AND key = ?2", params![world, letters_and_digits(name)], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
            .flatten();
        let race = race.and_then(|r| race_named(&r)).or_else(|| race_in_name(name))?;
        Creature::of_race(race)
    }

    fn areas_of(&self, table: &str, world: &str, key: &str) -> Result<Vec<String>, String> {
        let mut stmt = self.db.prepare(&format!("SELECT area FROM {table} WHERE world = ?1 AND key = ?2 ORDER BY last_seen DESC")).map_err(failed)?;
        let rows = stmt.query_map(params![world, key], |r| r.get(0)).map_err(failed)?;
        rows.collect::<Result<_, _>>().map_err(failed)
    }

    /// The creatures whose name, race or area has the words, the most
    /// lately seen first.
    pub fn mobs(&self, world: &str, query: &str) -> Result<Vec<MobRow>, String> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT key, name, race, deaths, last_seen FROM mobs m WHERE world = ?1
                 AND (name LIKE ?2 OR coalesce(race, '') LIKE ?2
                      OR EXISTS (SELECT 1 FROM mob_areas a WHERE a.world = m.world AND a.key = m.key AND a.area LIKE ?2))
                 ORDER BY last_seen DESC",
            )
            .map_err(failed)?;
        let rows = stmt
            .query_map(params![world, like(query)], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .map_err(failed)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(failed)?;
        rows.into_iter()
            .map(|(key, name, race, deaths, last_seen)| Ok(MobRow { areas: self.areas_of("mob_areas", world, &key)?, key, name, styled: Vec::new(), race, deaths, last_seen }))
            .collect()
    }

    /// Everything known of a creature.
    pub fn mob_entry(&self, world: &str, key: &str) -> Result<Option<MobEntry>, String> {
        let db = &self.db;
        let found = db
            .query_row(
                "SELECT name, race, race_seen, sex, height, weight, body, description, first_seen, last_seen, deaths FROM mobs WHERE world = ?1 AND key = ?2",
                params![world, key],
                |r| {
                    Ok(MobEntry {
                        key: key.to_string(),
                        name: r.get(0)?,
                        race: r.get(1)?,
                        race_seen: r.get(2)?,
                        category: None,
                        creature: None,
                        sex: r.get(3)?,
                        height: r.get(4)?,
                        weight: r.get(5)?,
                        body: r.get(6)?,
                        stats: Vec::new(),
                        description: r.get(7)?,
                        seems: None,
                        first_seen: r.get(8)?,
                        last_seen: r.get(9)?,
                        deaths: r.get(10)?,
                        areas: Vec::new(),
                        abilities: Vec::new(),
                        drops: Vec::new(),
                        wears: Vec::new(),
                        carried: Vec::new(),
                        butchered: Vec::new(),
                    })
                },
            )
            .optional()
            .map_err(failed)?;
        let Some(mut entry) = found else { return Ok(None) };
        let race = entry.race.as_deref().and_then(race_named);
        entry.category = race.map(|r| r.category.to_string());
        entry.creature = race.and_then(Creature::of_race).map(Creature::name);
        let mut stmt = db.prepare("SELECT stat FROM mob_stats WHERE world = ?1 AND key = ?2 ORDER BY stat").map_err(failed)?;
        entry.stats = stmt.query_map(params![world, key], |r| r.get(0)).map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
        let mut stmt = db
            .prepare("SELECT area, seen, last_seen, level_low, level_high, max_hp, alignment FROM mob_areas WHERE world = ?1 AND key = ?2 ORDER BY last_seen DESC")
            .map_err(failed)?;
        entry.areas = stmt
            .query_map(params![world, key], |r| {
                Ok(MobArea { area: r.get(0)?, seen: r.get(1)?, last_seen: r.get(2)?, level_low: r.get(3)?, level_high: r.get(4)?, max_hp: r.get(5)?, alignment: r.get(6)? })
            })
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)?;
        let mut stmt = db.prepare("SELECT ability, kind, count FROM mob_abilities WHERE world = ?1 AND key = ?2 ORDER BY count DESC, ability").map_err(failed)?;
        entry.abilities = stmt
            .query_map(params![world, key], |r| Ok(Counted { name: r.get(0)?, what: r.get(1)?, count: r.get(2)? }))
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)?;
        let mut kinds: Vec<(String, u32)> = Vec::new();
        for a in &entry.abilities {
            let kind = a.what.clone().unwrap_or_default();
            match kinds.iter_mut().find(|(k, _)| *k == kind) {
                Some((_, n)) => *n += a.count,
                None => kinds.push((kind, a.count)),
            }
        }
        entry.seems = seems(&kinds);
        let items = |how: &str| -> Result<Vec<Counted>, String> {
            let mut stmt = db
                .prepare(
                    "SELECT coalesce(i.name, m.item), m.place, m.count FROM mob_items m LEFT JOIN items i ON i.world = m.world AND i.key = m.item
                     WHERE m.world = ?1 AND m.key = ?2 AND m.how = ?3 ORDER BY m.count DESC, 1",
                )
                .map_err(failed)?;
            let rows = stmt.query_map(params![world, key, how], |r| Ok(Counted { name: r.get(0)?, what: r.get::<_, String>(1).map(|p| (!p.is_empty()).then_some(p))?, count: r.get(2)? }));
            let found: Vec<Counted> = rows.map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
            Ok(found)
        };
        entry.drops = items("drop")?;
        entry.wears = items("worn")?;
        entry.carried = items("carry")?;
        entry.butchered = items("butcher")?;
        Ok(Some(entry))
    }

    /// The items whose name, material or area has the words, the most
    /// lately seen first.
    pub fn items(&self, world: &str, query: &str) -> Result<Vec<ItemRow>, String> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT key, name, level_low, level_high, last_seen FROM items i WHERE world = ?1
                 AND (name LIKE ?2 OR coalesce(material, '') LIKE ?2
                      OR EXISTS (SELECT 1 FROM item_areas a WHERE a.world = i.world AND a.key = i.key AND a.area LIKE ?2))
                 ORDER BY last_seen DESC",
            )
            .map_err(failed)?;
        let rows = stmt
            .query_map(params![world, like(query)], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .map_err(failed)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(failed)?;
        rows.into_iter()
            .map(|(key, name, level_low, level_high, last_seen)| Ok(ItemRow { areas: self.areas_of("item_areas", world, &key)?, key, name, styled: Vec::new(), level_low, level_high, last_seen }))
            .collect()
    }

    /// Everything known of an item.
    pub fn item_entry(&self, world: &str, key: &str) -> Result<Option<ItemEntry>, String> {
        let db = &self.db;
        let found = db
            .query_row(
                "SELECT name, level_low, level_high, weight_low, weight_high, material, description, container, first_seen, last_seen FROM items WHERE world = ?1 AND key = ?2",
                params![world, key],
                |r| {
                    Ok(ItemEntry {
                        key: key.to_string(),
                        name: r.get(0)?,
                        level_low: r.get(1)?,
                        level_high: r.get(2)?,
                        weight_low: r.get(3)?,
                        weight_high: r.get(4)?,
                        material: r.get(5)?,
                        description: r.get(6)?,
                        container: r.get(7)?,
                        first_seen: r.get(8)?,
                        last_seen: r.get(9)?,
                        areas: Vec::new(),
                        dropped_by: Vec::new(),
                        worn_by: Vec::new(),
                        carried_by: Vec::new(),
                        butchered_from: Vec::new(),
                        auras: Vec::new(),
                    })
                },
            )
            .optional()
            .map_err(failed)?;
        let Some(mut entry) = found else { return Ok(None) };
        let mut stmt = db.prepare("SELECT area, seen FROM item_areas WHERE world = ?1 AND key = ?2 ORDER BY last_seen DESC").map_err(failed)?;
        entry.areas = stmt
            .query_map(params![world, key], |r| Ok(Counted { name: r.get(0)?, what: None, count: r.get(1)? }))
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)?;
        let by = |how: &str| -> Result<Vec<Counted>, String> {
            let mut stmt = db
                .prepare(
                    "SELECT coalesce(b.name, m.key), m.place, m.count FROM mob_items m LEFT JOIN mobs b ON b.world = m.world AND b.key = m.key
                     WHERE m.world = ?1 AND m.item = ?2 AND m.how = ?3 ORDER BY m.count DESC, 1",
                )
                .map_err(failed)?;
            let rows = stmt.query_map(params![world, key, how], |r| Ok(Counted { name: r.get(0)?, what: r.get::<_, String>(1).map(|p| (!p.is_empty()).then_some(p))?, count: r.get(2)? }));
            let found: Vec<Counted> = rows.map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
            Ok(found)
        };
        entry.dropped_by = by("drop")?;
        entry.worn_by = by("worn")?;
        entry.carried_by = by("carry")?;
        entry.butchered_from = by("butcher")?;
        let mut stmt = db.prepare("SELECT aura FROM item_auras WHERE world = ?1 AND key = ?2 ORDER BY rowid").map_err(failed)?;
        entry.auras = stmt.query_map(params![world, key], |r| r.get(0)).map_err(failed)?.collect::<Result<_, _>>().map_err(failed)?;
        Ok(Some(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_kept_before_are_made_one() {
        let b = Bestiary::in_memory().unwrap();
        let w = "coffeemud";
        // As kept before: each bundle its own, the colors' letters in the key.
        for (key, name, seen) in [("a25oakbundle", "a 25# oak bundle", 2), ("a3oakbundle", "a 3# oak bundle", 3), ("gagreenpotion", "^ga green potion^?", 1)] {
            b.db.execute("INSERT INTO items (world, key, name, first_seen, last_seen) VALUES (?1, ?2, ?3, ?4, ?4)", params![w, key, name, seen]).unwrap();
            b.db.execute("INSERT INTO item_areas (world, key, area, seen, last_seen) VALUES (?1, ?2, 'Midgaard', ?3, ?3)", params![w, key, seen]).unwrap();
            b.db.execute("INSERT INTO mob_items (world, key, item, how, place, count) VALUES (?1, 'anorc', ?2, 'drop', '', 1)", params![w, key]).unwrap();
        }
        b.rekey_items().unwrap();
        let rows = b.items(w, "").unwrap();
        let keys: Vec<(&str, &str)> = rows.iter().map(|r| (r.key.as_str(), r.name.as_str())).collect();
        assert_eq!(keys, [("anoakbundle", "an oak bundle"), ("agreenpotion", "^ga green potion^?")]);
        let bundle = b.item_entry(w, "anoakbundle").unwrap().unwrap();
        assert_eq!((bundle.areas[0].count, bundle.dropped_by[0].count, bundle.first_seen, bundle.last_seen), (5, 2, 2, 3));
        b.rekey_items().unwrap();
        assert_eq!(b.items(w, "").unwrap().len(), 2);
    }
}

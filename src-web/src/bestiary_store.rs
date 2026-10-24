//! The web build's Bestiary and Items (`bestiary.rs`): the desktop's
//! `bestiary_store.rs` is a SQLite database, and this is the same methods
//! over its tables' rows in memory, kept by the page in IndexedDB as JSON
//! (`to_json`, `from_json`). Each query does what the desktop's SQL does,
//! order and all: `bestiary.rs`'s tests run against both.

use std::cell::{Cell, RefCell};

use serde::{Deserialize, Serialize};

use crate::bestiary::{item_key, item_name, narrow, race_in_name, race_named, seems, Counted, Creature, Fact, ItemEntry, ItemRow, MobArea, MobEntry, MobRow};
use crate::senses::letters_and_digits;

/// A `mobs` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mob {
    world: String,
    key: String,
    name: String,
    race: Option<String>,
    race_seen: bool,
    sex: Option<String>,
    height: Option<u32>,
    weight: Option<u32>,
    body: Option<String>,
    description: Option<String>,
    first_seen: i64,
    last_seen: i64,
    deaths: u32,
    auto_looked: i64,
}

/// A `mob_areas` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Area {
    world: String,
    key: String,
    area: String,
    seen: u32,
    last_seen: i64,
    level_low: Option<i64>,
    level_high: Option<i64>,
    max_hp: Option<i64>,
    alignment: Option<String>,
}

/// A `mob_abilities` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Ability {
    world: String,
    key: String,
    ability: String,
    kind: String,
    count: u32,
}

/// A `mob_items` row: `how` is `worn`, `drop` or `carry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MobItem {
    world: String,
    key: String,
    item: String,
    how: String,
    place: String,
    count: u32,
}

/// An `items` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Item {
    world: String,
    key: String,
    name: String,
    level_low: Option<i64>,
    level_high: Option<i64>,
    weight_low: Option<i64>,
    weight_high: Option<i64>,
    material: Option<String>,
    description: Option<String>,
    container: bool,
    first_seen: i64,
    last_seen: i64,
}

/// An `item_areas` row.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ItemArea {
    world: String,
    key: String,
    area: String,
    seen: u32,
    last_seen: i64,
}

/// Every table. A `(world, key, value)` is a row of the two-column tables.
#[derive(Default, Serialize, Deserialize)]
struct Tables {
    mobs: Vec<Mob>,
    mob_areas: Vec<Area>,
    mob_stats: Vec<(String, String, String)>,
    mob_abilities: Vec<Ability>,
    mob_items: Vec<MobItem>,
    items: Vec<Item>,
    item_auras: Vec<(String, String, String)>,
    item_areas: Vec<ItemArea>,
}

#[derive(Default)]
pub struct Bestiary {
    t: RefCell<Tables>,
    /// Changed since `to_json` last kept it.
    changed: Cell<bool>,
}

/// SQLite's LIKE with the desktop's pattern: the query's words in order,
/// anything between, ASCII letters in either case.
fn like(text: &str, query: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let mut at = 0;
    for word in query.split_whitespace() {
        let word = word.replace(['%', '_', '\\'], "").to_ascii_lowercase();
        match text[at..].find(&word) {
            Some(i) => at += i + word.len(),
            None => return false,
        }
    }
    true
}

/// Newest first, the first kept first among equals, as SQLite gives them.
fn newest<T>(rows: &mut [&T], seen: impl Fn(&T) -> i64) {
    rows.sort_by_key(|r| std::cmp::Reverse(seen(r)));
}

/// `ORDER BY count DESC, name`.
fn by_count(rows: &mut [Counted]) {
    rows.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.as_bytes().cmp(b.name.as_bytes())));
}

impl Tables {
    fn mob(&mut self, world: &str, name: &str, at: i64) -> usize {
        let key = letters_and_digits(name);
        if let Some(i) = self.mobs.iter().position(|m| m.world == world && m.key == key) {
            self.mobs[i].last_seen = at;
            return i;
        }
        self.mobs.push(Mob {
            world: world.to_string(),
            key,
            name: name.to_string(),
            race: race_in_name(name).map(|r| r.name.to_string()),
            race_seen: false,
            sex: None,
            height: None,
            weight: None,
            body: None,
            description: None,
            first_seen: at,
            last_seen: at,
            deaths: 0,
            auto_looked: 0,
        });
        self.mobs.len() - 1
    }

    /// An item's row (a bundle by what it holds, `item_name`), its name in
    /// the game's colors once one comes.
    fn item(&mut self, world: &str, name: &str, at: i64) -> usize {
        let name = item_name(name);
        let key = item_key(&name);
        if let Some(i) = self.items.iter().position(|m| m.world == world && m.key == key) {
            let row = &mut self.items[i];
            row.last_seen = at;
            if name.contains('^') && !row.name.contains('^') {
                row.name = name;
            }
            return i;
        }
        self.items.push(Item {
            world: world.to_string(),
            key,
            name,
            level_low: None,
            level_high: None,
            weight_low: None,
            weight_high: None,
            material: None,
            description: None,
            container: false,
            first_seen: at,
            last_seen: at,
        });
        self.items.len() - 1
    }

    /// Kept before an item's key left its colors out and a bundle its
    /// weight (`item_key`): the desktop's `rekey_items`, row by row.
    /// Whether anything changed.
    fn rekey_items(&mut self) -> bool {
        let moves: Vec<(String, String, String, String)> = self
            .items
            .iter()
            .filter_map(|i| {
                let (to, kept) = (item_key(&i.name), item_name(&i.name));
                (to != i.key || kept != i.name).then(|| (i.world.clone(), i.key.clone(), to, kept))
            })
            .collect();
        let changed = !moves.is_empty();
        for (world, from, to, name) in moves {
            let at = self.items.iter().position(|i| i.world == world && i.key == from).expect("listed above");
            if from == to {
                self.items[at].name = name;
                continue;
            }
            let mut old = self.items.remove(at);
            match self.items.iter_mut().find(|i| i.world == world && i.key == to) {
                Some(row) => {
                    let low = |a: Option<i64>, b: Option<i64>| a.into_iter().chain(b).min();
                    let high = |a: Option<i64>, b: Option<i64>| a.into_iter().chain(b).max();
                    if name.contains('^') && !row.name.contains('^') {
                        row.name = name;
                    }
                    row.level_low = low(row.level_low, old.level_low);
                    row.level_high = high(row.level_high, old.level_high);
                    row.weight_low = low(row.weight_low, old.weight_low);
                    row.weight_high = high(row.weight_high, old.weight_high);
                    row.material = row.material.take().or(old.material);
                    row.description = row.description.take().or(old.description);
                    row.container |= old.container;
                    row.first_seen = row.first_seen.min(old.first_seen);
                    row.last_seen = row.last_seen.max(old.last_seen);
                }
                None => {
                    old.key = to.clone();
                    old.name = name;
                    self.items.push(old);
                }
            }
            for (w, k, aura) in std::mem::take(&mut self.item_auras) {
                let k = if w == world && k == from { to.clone() } else { k };
                if !self.item_auras.iter().any(|(w2, k2, a2)| *w2 == w && *k2 == k && *a2 == aura) {
                    self.item_auras.push((w, k, aura));
                }
            }
            for mut area in std::mem::take(&mut self.item_areas) {
                if area.world == world && area.key == from {
                    area.key = to.clone();
                }
                match self.item_areas.iter_mut().find(|a| a.world == area.world && a.key == area.key && a.area == area.area) {
                    Some(a) => {
                        a.seen += area.seen;
                        a.last_seen = a.last_seen.max(area.last_seen);
                    }
                    None => self.item_areas.push(area),
                }
            }
            for mut held in std::mem::take(&mut self.mob_items) {
                if held.world == world && held.item == from {
                    held.item = to.clone();
                }
                match self.mob_items.iter_mut().find(|m| m.world == held.world && m.key == held.key && m.item == held.item && m.how == held.how && m.place == held.place) {
                    Some(m) if m.how == "carry" => m.count = m.count.max(held.count),
                    Some(m) => m.count += held.count,
                    None => self.mob_items.push(held),
                }
            }
        }
        changed
    }

    fn mob_area(&mut self, world: &str, key: &str, area: Option<&str>, at: i64) -> Option<usize> {
        let area = area?;
        if let Some(i) = self.mob_areas.iter().position(|a| a.world == world && a.key == key && a.area == area) {
            self.mob_areas[i].last_seen = at;
            return Some(i);
        }
        self.mob_areas.push(Area {
            world: world.to_string(),
            key: key.to_string(),
            area: area.to_string(),
            seen: 0,
            last_seen: at,
            level_low: None,
            level_high: None,
            max_hp: None,
            alignment: None,
        });
        Some(self.mob_areas.len() - 1)
    }

    fn mob_item(&mut self, world: &str, key: &str, item: &str, how: &str, place: &str) -> Option<&mut MobItem> {
        self.mob_items.iter_mut().find(|m| m.world == world && m.key == key && m.item == item && m.how == how && m.place == place)
    }

    fn add_mob_item(&mut self, world: &str, key: &str, item: &str, how: &str, place: &str, count: u32) {
        self.mob_items.push(MobItem { world: world.to_string(), key: key.to_string(), item: item.to_string(), how: how.to_string(), place: place.to_string(), count });
    }
}

impl Bestiary {
    /// As the desktop's: a Bestiary in memory, here the only kind.
    pub fn in_memory() -> Result<Self, String> {
        Ok(Bestiary::default())
    }

    /// The Bestiary the page kept, or a new one when there's none.
    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.is_empty() {
            return Ok(Bestiary::default());
        }
        let mut t: Tables = serde_json::from_str(json).map_err(|e| format!("The Bestiary couldn't be read. ({e})"))?;
        let changed = t.rekey_items();
        Ok(Bestiary { t: RefCell::new(t), changed: Cell::new(changed) })
    }

    /// The Bestiary as JSON for the page to keep; it's then unchanged.
    pub fn to_json(&self) -> String {
        self.changed.set(false);
        serde_json::to_string(&*self.t.borrow()).unwrap_or_default()
    }

    /// Whether there's anything new to keep.
    pub fn changed(&self) -> bool {
        self.changed.get()
    }

    pub fn record(&self, world: &str, area: Option<&str>, fact: &Fact, at: i64) -> Result<(), String> {
        self.changed.set(true);
        let t = &mut *self.t.borrow_mut();
        match fact {
            Fact::Seen { name } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                if let Some(a) = t.mob_area(world, &key, area, at) {
                    t.mob_areas[a].seen += 1;
                }
            }
            Fact::ItemSeen { name, container } => {
                let i = t.item(world, name, at);
                if *container {
                    t.items[i].container = true;
                }
                if let Some(area) = area {
                    let key = t.items[i].key.clone();
                    match t.item_areas.iter_mut().find(|a| a.world == world && a.key == key && a.area == area) {
                        Some(a) => {
                            a.seen += 1;
                            a.last_seen = at;
                        }
                        None => t.item_areas.push(ItemArea { world: world.to_string(), key, area: area.to_string(), seen: 1, last_seen: at }),
                    }
                }
            }
            Fact::Race { name, race } => {
                let m = t.mob(world, name, at);
                t.mobs[m].race = Some(race_named(race).map_or(race.as_str(), |r| r.name).to_string());
                t.mobs[m].race_seen = true;
            }
            Fact::Size { name, sex, height, weight } => {
                let m = t.mob(world, name, at);
                t.mobs[m].sex = Some(sex.clone());
                t.mobs[m].height = Some(*height);
                t.mobs[m].weight = Some(*weight);
            }
            Fact::Body { name, body } => {
                let m = t.mob(world, name, at);
                t.mobs[m].body = Some(body.clone());
            }
            Fact::Stat { name, stat } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                if !t.mob_stats.iter().any(|(w, k, s)| w == world && *k == key && s == stat) {
                    t.mob_stats.push((world.to_string(), key, stat.clone()));
                }
            }
            Fact::Description { name, text } => {
                let m = t.mob(world, name, at);
                t.mobs[m].description = Some(text.clone());
            }
            Fact::ItemAura { name, aura } => {
                let i = t.item(world, name, at);
                let key = t.items[i].key.clone();
                if !t.item_auras.iter().any(|(w, k, a)| w == world && *k == key && a == aura) {
                    t.item_auras.push((world.to_string(), key, aura.clone()));
                }
            }
            Fact::Wears { name, place, item } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                let i = t.item(world, item, at);
                let item_key = t.items[i].key.clone();
                if t.mob_item(world, &key, &item_key, "worn", place).is_none() {
                    t.add_mob_item(world, &key, &item_key, "worn", place, 1);
                }
            }
            Fact::Level { name, low, high } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                let Some(a) = t.mob_area(world, &key, area, at) else { return Ok(()) };
                let row = &mut t.mob_areas[a];
                let (lo, hi) = narrow((row.level_low, row.level_high), (*low, *high));
                // "Far superior" has no top, "vastly inferior" no bottom but 1.
                row.level_low = Some(lo);
                row.level_high = (hi < i64::MAX / 8).then_some(hi);
            }
            Fact::Health { name, max } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                if let Some(a) = t.mob_area(world, &key, area, at) {
                    t.mob_areas[a].max_hp = Some(*max);
                }
            }
            Fact::Alignment { name, alignment } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                if let Some(a) = t.mob_area(world, &key, area, at) {
                    t.mob_areas[a].alignment = Some(alignment.to_string());
                }
            }
            Fact::Ability { name, ability, kind } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                match t.mob_abilities.iter_mut().find(|a| a.world == world && a.key == key && a.ability == *ability) {
                    Some(a) => a.count += 1,
                    None => t.mob_abilities.push(Ability { world: world.to_string(), key, ability: ability.clone(), kind: kind.clone(), count: 1 }),
                }
            }
            Fact::Died { name } => {
                let m = t.mob(world, name, at);
                t.mobs[m].deaths += 1;
            }
            Fact::Dropped { name, item } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                let i = t.item(world, item, at);
                let item_key = t.items[i].key.clone();
                match t.mob_item(world, &key, &item_key, "drop", "") {
                    Some(row) => row.count += 1,
                    None => t.add_mob_item(world, &key, &item_key, "drop", "", 1),
                }
            }
            Fact::Butchered { name, item } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                let i = t.item(world, item, at);
                let item_key = t.items[i].key.clone();
                match t.mob_item(world, &key, &item_key, "butcher", "") {
                    Some(row) => row.count += 1,
                    None => t.add_mob_item(world, &key, &item_key, "butcher", "", 1),
                }
            }
            Fact::Carried { name, item, count } => {
                let m = t.mob(world, name, at);
                let key = t.mobs[m].key.clone();
                let i = t.item(world, item, at);
                let item_key = t.items[i].key.clone();
                match t.mob_item(world, &key, &item_key, "carry", "") {
                    Some(row) => row.count = row.count.max(*count),
                    None => t.add_mob_item(world, &key, &item_key, "carry", "", *count),
                }
            }
            Fact::Weight { name, weight } => {
                let m = t.mob(world, name, at);
                t.mobs[m].weight = Some(*weight);
            }
            Fact::Item { name, level, weight, material, description } => {
                let i = t.item(world, name, at);
                let row = &mut t.items[i];
                if let Some((lo, hi)) = level {
                    row.level_low = Some(*lo);
                    row.level_high = Some(*hi);
                }
                if let Some((lo, hi)) = weight {
                    row.weight_low = Some(*lo);
                    row.weight_high = Some(*hi);
                }
                if material.is_some() {
                    row.material.clone_from(material);
                }
                if description.is_some() {
                    row.description.clone_from(description);
                }
            }
        }
        Ok(())
    }

    pub fn wants_look(&self, world: &str, name: &str) -> bool {
        let key = letters_and_digits(name);
        let t = self.t.borrow();
        !t.mobs.iter().any(|m| m.world == world && m.key == key && (m.height.is_some() || m.description.is_some() || m.auto_looked != 0))
    }

    pub fn looked(&self, world: &str, name: &str, at: i64) -> Result<(), String> {
        let t = &mut *self.t.borrow_mut();
        let m = t.mob(world, name, at);
        t.mobs[m].auto_looked = at;
        self.changed.set(true);
        Ok(())
    }

    pub fn creature(&self, world: &str, name: &str) -> Option<Creature> {
        let key = letters_and_digits(name);
        let t = self.t.borrow();
        let race = t.mobs.iter().find(|m| m.world == world && m.key == key).and_then(|m| m.race.clone());
        let race = race.and_then(|r| race_named(&r)).or_else(|| race_in_name(name))?;
        Creature::of_race(race)
    }

    fn mob_areas_of(t: &Tables, world: &str, key: &str) -> Vec<String> {
        let mut rows: Vec<&Area> = t.mob_areas.iter().filter(|a| a.world == world && a.key == key).collect();
        newest(&mut rows, |a| a.last_seen);
        rows.into_iter().map(|a| a.area.clone()).collect()
    }

    fn item_areas_of<'a>(t: &'a Tables, world: &str, key: &str) -> Vec<&'a ItemArea> {
        let mut rows: Vec<&ItemArea> = t.item_areas.iter().filter(|a| a.world == world && a.key == key).collect();
        newest(&mut rows, |a| a.last_seen);
        rows
    }

    pub fn mobs(&self, world: &str, query: &str) -> Result<Vec<MobRow>, String> {
        let t = self.t.borrow();
        let mut found: Vec<&Mob> = t
            .mobs
            .iter()
            .filter(|m| m.world == world)
            .filter(|m| like(&m.name, query) || like(m.race.as_deref().unwrap_or(""), query) || t.mob_areas.iter().any(|a| a.world == m.world && a.key == m.key && like(&a.area, query)))
            .collect();
        newest(&mut found, |m| m.last_seen);
        Ok(found
            .into_iter()
            .map(|m| MobRow { areas: Self::mob_areas_of(&t, world, &m.key), key: m.key.clone(), name: m.name.clone(), styled: Vec::new(), race: m.race.clone(), deaths: m.deaths, last_seen: m.last_seen })
            .collect())
    }

    /// What a creature wore, dropped or carried, by the items' names.
    fn mob_items(t: &Tables, world: &str, key: &str, how: &str) -> Vec<Counted> {
        let mut rows: Vec<Counted> = t
            .mob_items
            .iter()
            .filter(|m| m.world == world && m.key == key && m.how == how)
            .map(|m| Counted {
                name: t.items.iter().find(|i| i.world == world && i.key == m.item).map_or_else(|| m.item.clone(), |i| i.name.clone()),
                what: (!m.place.is_empty()).then(|| m.place.clone()),
                count: m.count,
            })
            .collect();
        by_count(&mut rows);
        rows
    }

    pub fn mob_entry(&self, world: &str, key: &str) -> Result<Option<MobEntry>, String> {
        let t = self.t.borrow();
        let Some(m) = t.mobs.iter().find(|m| m.world == world && m.key == key) else { return Ok(None) };
        let race = m.race.as_deref().and_then(race_named);
        let mut stats: Vec<String> = t.mob_stats.iter().filter(|(w, k, _)| w == world && k == key).map(|(_, _, s)| s.clone()).collect();
        stats.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        let mut areas: Vec<&Area> = t.mob_areas.iter().filter(|a| a.world == world && a.key == key).collect();
        newest(&mut areas, |a| a.last_seen);
        let mut abilities: Vec<Counted> =
            t.mob_abilities.iter().filter(|a| a.world == world && a.key == key).map(|a| Counted { name: a.ability.clone(), what: Some(a.kind.clone()), count: a.count }).collect();
        by_count(&mut abilities);
        let mut kinds: Vec<(String, u32)> = Vec::new();
        for a in &abilities {
            let kind = a.what.clone().unwrap_or_default();
            match kinds.iter_mut().find(|(k, _)| *k == kind) {
                Some((_, n)) => *n += a.count,
                None => kinds.push((kind, a.count)),
            }
        }
        Ok(Some(MobEntry {
            key: key.to_string(),
            name: m.name.clone(),
            race: m.race.clone(),
            race_seen: m.race_seen,
            category: race.map(|r| r.category.to_string()),
            creature: race.and_then(Creature::of_race).map(Creature::name),
            sex: m.sex.clone(),
            height: m.height,
            weight: m.weight,
            body: m.body.clone(),
            stats,
            description: m.description.clone(),
            seems: seems(&kinds),
            first_seen: m.first_seen,
            last_seen: m.last_seen,
            deaths: m.deaths,
            areas: areas
                .into_iter()
                .map(|a| MobArea {
                    area: a.area.clone(),
                    seen: a.seen,
                    last_seen: a.last_seen,
                    level_low: a.level_low,
                    level_high: a.level_high,
                    max_hp: a.max_hp,
                    alignment: a.alignment.clone(),
                })
                .collect(),
            abilities,
            drops: Self::mob_items(&t, world, key, "drop"),
            wears: Self::mob_items(&t, world, key, "worn"),
            carried: Self::mob_items(&t, world, key, "carry"),
            butchered: Self::mob_items(&t, world, key, "butcher"),
        }))
    }

    pub fn items(&self, world: &str, query: &str) -> Result<Vec<ItemRow>, String> {
        let t = self.t.borrow();
        let mut found: Vec<&Item> = t
            .items
            .iter()
            .filter(|i| i.world == world)
            .filter(|i| like(&i.name, query) || like(i.material.as_deref().unwrap_or(""), query) || t.item_areas.iter().any(|a| a.world == i.world && a.key == i.key && like(&a.area, query)))
            .collect();
        newest(&mut found, |i| i.last_seen);
        Ok(found
            .into_iter()
            .map(|i| ItemRow {
                areas: Self::item_areas_of(&t, world, &i.key).into_iter().map(|a| a.area.clone()).collect(),
                key: i.key.clone(),
                name: i.name.clone(),
                styled: Vec::new(),
                level_low: i.level_low,
                level_high: i.level_high,
                last_seen: i.last_seen,
            })
            .collect())
    }

    /// The creatures that wore, dropped or carried an item, by their names.
    fn holders(t: &Tables, world: &str, key: &str, how: &str) -> Vec<Counted> {
        let mut rows: Vec<Counted> = t
            .mob_items
            .iter()
            .filter(|m| m.world == world && m.item == key && m.how == how)
            .map(|m| Counted {
                name: t.mobs.iter().find(|b| b.world == world && b.key == m.key).map_or_else(|| m.key.clone(), |b| b.name.clone()),
                what: (!m.place.is_empty()).then(|| m.place.clone()),
                count: m.count,
            })
            .collect();
        by_count(&mut rows);
        rows
    }

    pub fn item_entry(&self, world: &str, key: &str) -> Result<Option<ItemEntry>, String> {
        let t = self.t.borrow();
        let Some(i) = t.items.iter().find(|i| i.world == world && i.key == key) else { return Ok(None) };
        Ok(Some(ItemEntry {
            key: key.to_string(),
            name: i.name.clone(),
            level_low: i.level_low,
            level_high: i.level_high,
            weight_low: i.weight_low,
            weight_high: i.weight_high,
            material: i.material.clone(),
            description: i.description.clone(),
            container: i.container,
            first_seen: i.first_seen,
            last_seen: i.last_seen,
            areas: Self::item_areas_of(&t, world, key).into_iter().map(|a| Counted { name: a.area.clone(), what: None, count: a.seen }).collect(),
            dropped_by: Self::holders(&t, world, key, "drop"),
            worn_by: Self::holders(&t, world, key, "worn"),
            carried_by: Self::holders(&t, world, key, "carry"),
            butchered_from: Self::holders(&t, world, key, "butcher"),
            auras: t.item_auras.iter().filter(|(w, k, _)| w == world && k == key).map(|(_, _, a)| a.clone()).collect(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_kept_before_are_made_one() {
        let w = "coffeemud";
        let mut t = Tables::default();
        // As kept before: each bundle its own, the colors' letters in the key.
        for (key, name, seen) in [("a25oakbundle", "a 25# oak bundle", 2), ("a3oakbundle", "a 3# oak bundle", 3), ("gagreenpotion", "^ga green potion^?", 1)] {
            t.item(w, name, seen);
            let row = t.items.last_mut().unwrap();
            (row.key, row.name) = (key.into(), name.into());
            t.item_areas.push(ItemArea { world: w.into(), key: key.into(), area: "Midgaard".into(), seen: seen as u32, last_seen: seen });
            t.mob_items.push(MobItem { world: w.into(), key: "anorc".into(), item: key.into(), how: "drop".into(), place: String::new(), count: 1 });
        }
        let b = Bestiary::from_json(&serde_json::to_string(&t).unwrap()).unwrap();
        assert!(b.changed());
        let rows = b.items(w, "").unwrap();
        let keys: Vec<(&str, &str)> = rows.iter().map(|r| (r.key.as_str(), r.name.as_str())).collect();
        assert_eq!(keys, [("anoakbundle", "an oak bundle"), ("agreenpotion", "^ga green potion^?")]);
        let bundle = b.item_entry(w, "anoakbundle").unwrap().unwrap();
        assert_eq!((bundle.areas[0].count, bundle.dropped_by[0].count, bundle.first_seen, bundle.last_seen), (5, 2, 2, 3));
        let again = Bestiary::from_json(&b.to_json()).unwrap();
        assert!(!again.changed());
    }
}

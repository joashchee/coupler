//! **Tables**: Coupler's own tables of what it reads in the game, and the
//! player's, kept per world as `tables/<world>.json` in app data. Pure
//! and unit-tested; shared with the web build. Gear → Tables… shows them
//! (`TablesDialog.tsx`); `#check <table>` says one (`lib/commands.ts`).
//!
//! - **Loot** (`LOOT`), Coupler's: every thing AUTOLOOT and AUTOGOLD took
//!   off a body (`loot.rs`), a row each, with the body, the character and
//!   the room. The rows of one death share its `haul`, so `#check loot`
//!   says only what the last fight brought, never the whole inventory.
//! - **The player's own**: a name, up to `MOST_COLUMNS` columns, rows
//!   added in the dialog or by `#add`.
//!
//! Other players' names can be in them (a body's, a note's), so they stay
//! on this machine (CLAUDE.md rule 1).

use serde::{Deserialize, Serialize};

use crate::loot::Got;

/// The Loot table's id.
pub const LOOT: &str = "loot";
/// Rows kept per table; the oldest go first.
const MOST_ROWS: usize = 5_000;
/// The player's tables at most.
const MOST_TABLES: usize = 50;
/// A table of the player's has this many columns at most.
pub const MOST_COLUMNS: usize = 5;
/// Names and cells at most, in characters.
const LONGEST: usize = 200;
/// The rows `#check` says of a player's table, the latest first.
const SAID: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: u64,
    /// When it was added, Unix milliseconds.
    pub at: i64,
    pub cells: Vec<String>,
    /// Loot's: the death it came from (its time), shared by its rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub haul: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub id: String,
    pub name: String,
    pub columns: Vec<String>,
    /// Coupler fills it: the player can't add rows nor delete it.
    #[serde(default)]
    pub coupler: bool,
    #[serde(default)]
    pub rows: Vec<Row>,
}

/// A table without its rows, for the list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub id: String,
    pub name: String,
    pub columns: Vec<String>,
    pub coupler: bool,
    pub rows: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Tables {
    #[serde(default)]
    tables: Vec<Table>,
    #[serde(default)]
    next_row: u64,
    #[serde(skip)]
    changed: bool,
}

/// A change the player makes, as the frontend sends it (both builds).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Change {
    Create { name: String, columns: Vec<String> },
    Delete { id: String },
    Add { id: String, cells: Vec<String> },
    #[serde(rename_all = "camelCase")]
    DeleteRow { id: String, row: u64 },
    Clear { id: String },
}

/// The Loot table's columns.
const LOOT_COLUMNS: [&str; 4] = ["Item", "From", "Character", "Where"];

fn loot_table() -> Table {
    Table { id: LOOT.into(), name: "Loot".into(), columns: LOOT_COLUMNS.map(String::from).to_vec(), coupler: true, rows: Vec::new() }
}

/// A name or a cell as kept: spaces squeezed, cut to `LONGEST`.
fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(LONGEST).collect()
}

/// A table's id from its name: lower case, letters and digits, `-` between.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// "a, b and c".
fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

impl Tables {
    pub fn from_json(json: &str) -> Result<Tables, String> {
        let mut tables: Tables = serde_json::from_str(json).map_err(|e| format!("The tables couldn't be read. ({e})"))?;
        tables.ensure();
        Ok(tables)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Coupler's own tables, there from the start.
    fn ensure(&mut self) {
        if !self.tables.iter().any(|t| t.id == LOOT) {
            self.tables.insert(0, loot_table());
        }
        let most = self.tables.iter().flat_map(|t| t.rows.iter().map(|r| r.id + 1)).max().unwrap_or(0);
        self.next_row = self.next_row.max(most);
    }

    pub fn new() -> Tables {
        let mut tables = Tables::default();
        tables.ensure();
        tables
    }

    /// Whether anything changed since last asked: it's to be saved.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    pub fn list(&self) -> Vec<Overview> {
        self.tables.iter().map(|t| Overview { id: t.id.clone(), name: t.name.clone(), columns: t.columns.clone(), coupler: t.coupler, rows: t.rows.len() }).collect()
    }

    pub fn table(&self, id: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.id == id)
    }

    /// The table a typed name means: its id or name, any case, or the
    /// start of only one.
    pub fn find(&self, typed: &str) -> Result<&Table, String> {
        let want = slug(typed);
        if want.is_empty() {
            return Err("Which table?".into());
        }
        if let Some(t) = self.tables.iter().find(|t| t.id == want) {
            return Ok(t);
        }
        let starting: Vec<&Table> = self.tables.iter().filter(|t| t.id.starts_with(&want)).collect();
        match starting.as_slice() {
            [one] => Ok(one),
            [] => Err(format!("There's no table called {}.", typed.trim())),
            many => Err(format!("{} could be {}.", typed.trim(), listed(&many.iter().map(|t| t.name.clone()).collect::<Vec<_>>()).replace(" and ", " or "))),
        }
    }

    fn push(&mut self, id: &str, at: i64, cells: Vec<String>, haul: Option<i64>) {
        let row = Row { id: self.next_row, at, cells, haul };
        self.next_row += 1;
        let Some(t) = self.tables.iter_mut().find(|t| t.id == id) else { return };
        t.rows.push(row);
        if t.rows.len() > MOST_ROWS {
            let over = t.rows.len() - MOST_ROWS;
            t.rows.drain(..over);
        }
        self.changed = true;
    }

    /// Loot taken off a body: a row each, in the death's haul.
    pub fn loot(&mut self, got: &[Got], haul: i64, at: i64, character: &str, place: &str) {
        for g in got {
            let cells = vec![tidy(&g.item), tidy(&g.from), tidy(character), tidy(place)];
            self.push(LOOT, at, cells, Some(haul));
        }
    }

    /// A new table of the player's: its id.
    pub fn create(&mut self, name: &str, columns: &[String]) -> Result<String, String> {
        let name = tidy(name);
        let id = slug(&name);
        if id.is_empty() {
            return Err("A table needs a name with a letter or a digit in it.".into());
        }
        if self.tables.iter().any(|t| t.id == id) {
            return Err(format!("There's a table called {name} already."));
        }
        if self.tables.iter().filter(|t| !t.coupler).count() >= MOST_TABLES {
            return Err(format!("Coupler keeps {MOST_TABLES} tables of yours at most: delete one first."));
        }
        let columns: Vec<String> = columns.iter().map(|c| tidy(c)).filter(|c| !c.is_empty()).collect();
        if columns.is_empty() {
            return Err("A table needs at least one column.".into());
        }
        if columns.len() > MOST_COLUMNS {
            return Err(format!("A table has {MOST_COLUMNS} columns at most."));
        }
        self.tables.push(Table { id: id.clone(), name, columns, coupler: false, rows: Vec::new() });
        self.changed = true;
        Ok(id)
    }

    /// Deletes a table of the player's.
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let t = self.table(id).ok_or("That table isn't there any more.")?;
        if t.coupler {
            return Err(format!("{} is Coupler's: it can be cleared, not deleted.", t.name));
        }
        self.tables.retain(|t| t.id != id);
        self.changed = true;
        Ok(())
    }

    /// A row added to a table of the player's: its cells, one a column
    /// (missing ones blank, extra ones joined into the last).
    pub fn add(&mut self, id: &str, cells: &[String], at: i64) -> Result<(), String> {
        let t = self.table(id).ok_or("That table isn't there any more.")?;
        if t.coupler {
            return Err(format!("Coupler fills {} itself.", t.name));
        }
        let n = t.columns.len();
        let mut row: Vec<String> = cells.iter().map(|c| tidy(c)).collect();
        if row.len() > n {
            let rest = row.split_off(n - 1).join(" | ");
            row.push(rest);
        }
        row.resize(n, String::new());
        if row.iter().all(String::is_empty) {
            return Err("The row is empty.".into());
        }
        self.push(id, at, row, None);
        Ok(())
    }

    /// Deletes one row of any table.
    pub fn delete_row(&mut self, id: &str, row: u64) -> Result<(), String> {
        let t = self.tables.iter_mut().find(|t| t.id == id).ok_or("That table isn't there any more.")?;
        let before = t.rows.len();
        t.rows.retain(|r| r.id != row);
        self.changed |= t.rows.len() != before;
        Ok(())
    }

    /// Every row of a table gone; the table stays.
    pub fn clear(&mut self, id: &str) -> Result<(), String> {
        let t = self.tables.iter_mut().find(|t| t.id == id).ok_or("That table isn't there any more.")?;
        self.changed |= !t.rows.is_empty();
        t.rows.clear();
        Ok(())
    }

    /// A change of the player's, at `at` (Unix milliseconds): a new
    /// table's id, for one made.
    pub fn apply(&mut self, change: &Change, at: i64) -> Result<Option<String>, String> {
        match change {
            Change::Create { name, columns } => self.create(name, columns).map(Some),
            Change::Delete { id } => self.delete(id).map(|_| None),
            Change::Add { id, cells } => self.add(id, cells, at).map(|_| None),
            Change::DeleteRow { id, row } => self.delete_row(id, *row).map(|_| None),
            Change::Clear { id } => self.clear(id).map(|_| None),
        }
    }

    /// What `#check` says of a table: for Loot, the last haul of this
    /// character's; for the player's, how many rows and the latest few.
    pub fn check(&self, typed: &str, character: &str) -> Result<String, String> {
        let t = self.find(typed)?;
        if t.id == LOOT {
            return Ok(last_haul(t, character));
        }
        if t.rows.is_empty() {
            return Ok(format!("{}: no rows yet.", t.name));
        }
        let latest: Vec<String> = t.rows.iter().rev().take(SAID).map(|r| r.cells.iter().filter(|c| !c.is_empty()).cloned().collect::<Vec<_>>().join(", ")).collect();
        let rows = if t.rows.len() == 1 { "1 row".to_string() } else { format!("{} rows", t.rows.len()) };
        let label = if latest.len() == 1 { "The latest" } else { "The latest first" };
        Ok(format!("{}: {rows}. {label}: {}.", t.name, latest.join("; ")))
    }
}

/// The things the last death brought this character, in words.
fn last_haul(t: &Table, character: &str) -> String {
    let theirs = |r: &&Row| character.is_empty() || r.cells.get(2).is_some_and(|c| c.eq_ignore_ascii_case(character));
    let Some(haul) = t.rows.iter().rev().find(theirs).and_then(|r| r.haul) else {
        return "No loot yet. With AUTOLOOT on, what you take off a body is listed here.".into();
    };
    let rows: Vec<&Row> = t.rows.iter().filter(theirs).filter(|r| r.haul == Some(haul)).collect();
    let items: Vec<String> = rows.iter().map(|r| r.cells[0].clone()).collect();
    let mut bodies: Vec<String> = rows.iter().map(|r| r.cells[1].clone()).collect();
    bodies.dedup();
    format!("Loot from {}: {}.", listed(&bodies), listed(&items))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn got(item: &str, from: &str) -> Got {
        Got { item: item.into(), from: from.into() }
    }

    fn strings(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn loot_is_there_from_the_start() {
        let t = Tables::new();
        assert_eq!(t.list()[0].id, LOOT);
        assert!(t.list()[0].coupler);
        let back = Tables::from_json("{}").unwrap();
        assert_eq!(back.list().len(), 1);
    }

    #[test]
    fn check_loot_says_only_the_last_haul() {
        let mut t = Tables::new();
        t.loot(&[got("a rat pelt", "the body of a rat")], 1, 1, "Hassan", "A Dark Alley");
        t.loot(&[got("a club", "the body of an orc"), got("12 gold coins", "the body of an orc")], 2, 2, "Hassan", "The Woods");
        t.loot(&[got("a wand", "the body of a mage")], 3, 3, "Bob", "The Tower");
        assert_eq!(t.check("loot", "Hassan").unwrap(), "Loot from the body of an orc: a club and 12 gold coins.");
        assert_eq!(t.check("LO", "Bob").unwrap(), "Loot from the body of a mage: a wand.");
        assert!(t.check("loot", "Zed").unwrap().starts_with("No loot yet."));
        assert!(t.take_changed());
        assert!(!t.take_changed());
    }

    #[test]
    fn the_players_own_tables() {
        let mut t = Tables::new();
        let id = t.create("Quest Notes", &strings(&["Who", "Wants"])).unwrap();
        assert_eq!(id, "quest-notes");
        assert!(t.create("quest notes", &strings(&["A"])).is_err());
        assert!(t.create("Empty", &[]).is_err());
        assert!(t.create("!!", &strings(&["A"])).is_err());
        assert!(t.create("Wide", &strings(&["1", "2", "3", "4", "5", "6"])).is_err());
        assert_eq!(t.check("quest", "").unwrap(), "Quest Notes: no rows yet.");
        t.add(&id, &strings(&["the mayor", "a lost ring"]), 1).unwrap();
        t.add(&id, &strings(&["the smith", "iron", "and coal"]), 2).unwrap();
        assert_eq!(t.table(&id).unwrap().rows[1].cells, strings(&["the smith", "iron | and coal"]));
        assert_eq!(t.check("Quest Notes", "").unwrap(), "Quest Notes: 2 rows. The latest first: the smith, iron | and coal; the mayor, a lost ring.");
        assert!(t.add(&id, &strings(&["", " "]), 3).is_err());
        assert!(t.add(LOOT, &strings(&["a sword"]), 3).is_err());
        assert!(t.delete(LOOT).is_err());
        let row = t.table(&id).unwrap().rows[0].id;
        t.delete_row(&id, row).unwrap();
        assert_eq!(t.table(&id).unwrap().rows.len(), 1);
        t.delete(&id).unwrap();
        assert!(t.find("quest").is_err());
    }

    #[test]
    fn changes_as_the_frontend_sends_them() {
        let mut t = Tables::new();
        let make: Change = serde_json::from_str(r#"{"kind":"create","name":"Notes","columns":["Note"]}"#).unwrap();
        assert_eq!(t.apply(&make, 1).unwrap().as_deref(), Some("notes"));
        let add: Change = serde_json::from_str(r#"{"kind":"add","id":"notes","cells":["hi"]}"#).unwrap();
        t.apply(&add, 2).unwrap();
        let row = t.table("notes").unwrap().rows[0].id;
        let gone: Change = serde_json::from_str(&format!(r#"{{"kind":"deleteRow","id":"notes","row":{row}}}"#)).unwrap();
        t.apply(&gone, 3).unwrap();
        assert!(t.table("notes").unwrap().rows.is_empty());
    }

    #[test]
    fn a_name_cut_short_finds_only_one() {
        let mut t = Tables::new();
        t.create("Lore", &strings(&["Fact"])).unwrap();
        assert!(t.find("lo").unwrap_err().contains("Loot or Lore"));
        assert_eq!(t.find("lor").unwrap().name, "Lore");
        assert!(t.find("nothing").is_err());
    }

    #[test]
    fn kept_and_read_back() {
        let mut t = Tables::new();
        t.loot(&[got("a pelt", "the body of a rat")], 5, 5, "Hassan", "Alley");
        let id = t.create("Notes", &strings(&["Note"])).unwrap();
        t.add(&id, &strings(&["hi"]), 6).unwrap();
        let back = Tables::from_json(&t.to_json()).unwrap();
        assert_eq!(back.list(), t.list());
        let mut back = back;
        back.add(&id, &strings(&["again"]), 7).unwrap();
        let ids: Vec<u64> = back.tables.iter().flat_map(|t| t.rows.iter().map(|r| r.id)).collect();
        let mut unique = ids.clone();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn the_oldest_rows_go_first() {
        let mut t = Tables::new();
        for i in 0..MOST_ROWS + 3 {
            t.loot(&[got(&format!("thing {i}"), "a body")], i as i64, i as i64, "Hassan", "");
        }
        let rows = &t.table(LOOT).unwrap().rows;
        assert_eq!(rows.len(), MOST_ROWS);
        assert_eq!(rows[0].cells[0], "thing 3");
    }
}

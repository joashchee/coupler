//! **What's on the player, and their group**: effects with their time
//! left, cooldowns, and the group's members, for The Fight panel, the say
//! key and Combat Summary. Pure and unit-tested; shared with the web
//! build. Times are Unix milliseconds, so a page counts them down itself.
//!
//! - **Effects, by GMCP.** With `Char.Effects 1` in the supports list the
//!   game pushes `char.effects.add` and `char.effects.remove`
//!   `{"name","group"}` as most effects start and end
//!   (`StdAbility.gmcpAddNotify`, `gmcpRemoveNotify`), and with `Char 1`
//!   it sends `char.effects.groups` when the number of effects changes
//!   (`CMProtocols.pingGmcp`). Neither says how long one lasts.
//! - **Times, by AFFECTS.** A hidden AFFECTS (its reply kept out of the
//!   output, as CHANNELS' is in `channels.rs`) lists what shows, each in
//!   its display words with its time left (`Commands/Affect.java`): the
//!   posture and the like first ("You are standing.", `getMOBState`), a
//!   blank line, then `You are affected by: ` and the effects two to a row,
//!   "(Shield spell) 3m 20s", or `Nothing!`. It's the list that counts;
//!   the pushes add or drop a name between reads. **Gentle**: asked only
//!   when the pushes say something changed, at most once in
//!   `READ_EVERY_MS`, and only when the session says Coupler's own
//!   commands go unnoticed (`Who::gentle`): AFFECTS costs no action
//!   (`StdCommand.actionsCost`'s default 0).
//! - **Cooldowns**: few abilities have one (`getTicksBetweenCasts`: Reflect
//!   and Deflect Prayer, Majesty, Center of Attention and a few that wait
//!   a day), so it's the game's refusal that sets one: "You must wait 2
//!   hours before you can do that again." (`StdAbility.invoke`, in the
//!   area's hours, `DefaultTimeClock.deriveEllapsedTimeString`), put to
//!   the command just sent; and "You need a second to recover before doing
//!   that again." (`minCastWaitTime`). The hours are the stock clock's
//!   (`MILLISPERMUDHOUR` 600000, six to a day), so the time is "about".
//! - **The group**, by GMCP `group` with `Group 1` in the supports list
//!   (no other key turns it on): its leader, the tank in a fight, and each
//!   member's health, mana, moves and level. The player is left out of
//!   the allies.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The fewest milliseconds between two hidden AFFECTS.
pub const READ_EVERY_MS: i64 = 15_000;
/// How long AFFECTS' answer is waited for before nothing more is hidden for it.
const ANSWER_WAITS_MS: i64 = 10_000;
/// A refusal this soon after a command is that command's.
const SENT_MS: i64 = 5_000;
/// The stock clock (coffeemud.ini `MILLISPERMUDHOUR`, `HOURSINDAY`).
const HOUR_MS: i64 = 600_000;
const DAY_HOURS: i64 = 6;
/// A day's months and a year's: the stock clock's (`DAYSINMONTH`, `MONTHSINYEAR`).
const MONTH_DAYS: i64 = 30;
const YEAR_MONTHS: i64 = 12;

const AFFECTED: &str = "You are affected by:";
const WAIT: (&str, &str) = ("You must wait ", " before you can do that again.");
/// `minCastWaitTime`'s refusals, and the least each stands for.
const RECOVER: &[(&str, i64)] = &[("You need a second to recover before doing that again.", 1_000), ("You need a few seconds to recover before doing that again.", 5_000)];

/// The lines before the list (`Affect.getMOBState`): what's worth keeping.
const STATES: &[&str] = &[
    "You could use a bath.",
    "You could really use a bath.",
    "You need to bathe, soon.",
    "You desperately need to bathe.",
    "Your stench is horrendous! Bathe dammit!",
    "You are bound.",
    "You are sleeping.",
    "You are resting.",
    "You are swimming.",
    "You are climbing.",
    "You are flying.",
    "You are standing.",
    "Your playerkill flag is on.",
    "You are invisible.",
    "You are hidden.",
    "You are sneaking.",
    "You are in QUIET mode.",
    "You are fatigued.",
    "You are hungry.",
    "You are thirsty.",
];

/// Something on the player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    /// As AFFECTS words it ("Shield spell"), else as GMCP names it ("Shield").
    pub name: String,
    /// When it wears off, if the game said.
    pub ends_at: Option<i64>,
}

/// An ability the game says must wait.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cooldown {
    /// The command that was refused ("majesty").
    pub name: String,
    pub ends_at: i64,
    /// From the game's hours, the stock clock's: about.
    pub about: bool,
}

/// Someone else in the player's group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ally {
    pub name: String,
    pub hp: Option<i64>,
    pub max_hp: Option<i64>,
    pub mana: Option<i64>,
    pub max_mana: Option<i64>,
    pub moves: Option<i64>,
    pub max_moves: Option<i64>,
    pub level: Option<i64>,
}

/// The player's group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub leader: String,
    /// Whom the foe is hitting, in a fight: "you" for the player.
    pub tank: Option<String>,
    pub allies: Vec<Ally>,
}

/// Everything known, as the frontend shows it (`status`).
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub effects: Vec<Effect>,
    pub cooldowns: Vec<Cooldown>,
    /// AFFECTS' first lines, in the game's words: "You are hungry."
    pub states: Vec<String>,
    pub group: Option<Group>,
    /// AFFECTS has been read since the character came in.
    pub read: bool,
}

/// What changed, for the cues and the fight's record.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "camelCase")]
pub enum Change {
    Gained(String),
    Lost(String),
    /// A cooldown's over.
    Ready(String),
    /// The foe turned on someone else: their name, "you" for the player.
    Tank(String),
}

/// AFFECTS' answer being read.
enum Reading {
    /// Before `You are affected by:`.
    States(Vec<String>),
    /// The rows.
    Rows(Vec<String>, Vec<Effect>),
}

#[derive(Default)]
pub struct Watch {
    status: Status,
    /// The player's name, to leave them out of the allies.
    me: String,
    /// The pushes say the list changed since AFFECTS was last read.
    stale: bool,
    last_read: Option<i64>,
    asked: Option<i64>,
    reading: Option<Reading>,
    /// The player's last command, and when.
    sent: Option<(String, i64)>,
    changes: Vec<Change>,
    /// The status changed since it was last taken.
    dirty: bool,
}

/// Spaces collapsed, ends trimmed.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether an effect's words and GMCP's name are the same effect:
/// "Shield spell" and "Shield", "Blessed" and "Bless".
fn same(shown: &str, name: &str) -> bool {
    let (a, b) = (shown.trim().to_lowercase(), name.trim().to_lowercase());
    !a.is_empty() && !b.is_empty() && (a.starts_with(&b) || b.starts_with(&a))
}

/// "3m 20s" in milliseconds (`CoffeeTime.date2EllapsedTime`'s shortest form).
fn short_time(text: &str) -> Option<i64> {
    let mut total = 0;
    let mut any = false;
    for word in text.split_whitespace() {
        let split = word.find(|c: char| !c.is_ascii_digit())?;
        let (n, unit) = word.split_at(split);
        let n: i64 = n.parse().ok()?;
        let ms = match unit {
            "y" => 365 * 86_400_000,
            "M" => 30 * 86_400_000,
            "w" => 7 * 86_400_000,
            "d" => 86_400_000,
            "h" => 3_600_000,
            "m" => 60_000,
            "s" => 1_000,
            _ => return None,
        };
        total += n * ms;
        any = true;
    }
    any.then_some(total)
}

/// The game's hours in milliseconds, at most: "under an hour", "2 hours",
/// "1 day, 3 hours" (`DefaultTimeClock.deriveEllapsedTimeString`).
fn game_time(text: &str) -> Option<i64> {
    let text = text.trim();
    if text == "under an hour" {
        return Some(HOUR_MS);
    }
    let mut hours = 0;
    for part in text.split(", ") {
        let (n, unit) = part.trim().split_once(' ')?;
        let n: i64 = n.parse().ok()?;
        hours += n * match unit.trim_end_matches('s') {
            "year" => YEAR_MONTHS * MONTH_DAYS * DAY_HOURS,
            "month" => MONTH_DAYS * DAY_HOURS,
            "day" => DAY_HOURS,
            "hour" => 1,
            _ => return None,
        };
    }
    // The game rounds down: up to an hour more.
    Some((hours + 1) * HOUR_MS)
}

/// One row of AFFECTS' list: its entries, each "(words) time" or plain words.
fn row(text: &str, now: i64) -> Vec<Effect> {
    // Entries are padded apart; one that fills its column runs into the next "(".
    let mut chunks: Vec<String> = Vec::new();
    for chunk in text.split("  ").map(str::trim).filter(|c| !c.is_empty()) {
        let mut current = String::new();
        let mut depth = 0;
        for c in chunk.chars() {
            if c == '(' && depth == 0 && current.contains(')') {
                chunks.push(std::mem::take(&mut current));
            }
            match c {
                '(' => depth += 1,
                ')' => depth = (depth - 1_i32).max(0),
                _ => {}
            }
            current.push(c);
        }
        chunks.push(current);
    }
    chunks
        .into_iter()
        .filter_map(|chunk| {
            let chunk = chunk.trim();
            if let Some(rest) = chunk.strip_prefix('(') {
                let close = rest.rfind(')')?;
                let name = squeeze(&rest[..close]);
                let left = short_time(&rest[close + 1..]);
                (!name.is_empty()).then(|| Effect { name, ends_at: left.map(|ms| now + ms) })
            } else {
                (!chunk.is_empty()).then(|| Effect { name: squeeze(chunk), ends_at: None })
            }
        })
        .collect()
}

fn number(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|n| n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).or_else(|| n.as_str().and_then(|s| s.trim().parse().ok())))
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string()
}

impl Watch {
    /// The connection ended: nothing's known.
    pub fn closed(&mut self) {
        *self = Watch { dirty: true, ..Watch::default() };
    }

    /// The player's name, as GMCP's `char.base` and `char.status` give it.
    pub fn me(&mut self, name: &str) {
        if name != self.me {
            self.me = name.to_string();
            if let Some(group) = self.status.group.take() {
                self.set_group(Some(group));
            }
        }
    }

    /// The player sent a command.
    pub fn sent(&mut self, command: &str, now: i64) {
        let command = command.trim();
        if !command.is_empty() {
            self.sent = Some((command.to_string(), now));
        }
    }

    /// One GMCP message. Returns whether it was one of these.
    pub fn gmcp(&mut self, package: &str, data: &str) -> bool {
        let package = package.to_ascii_lowercase();
        match package.as_str() {
            "char.effects.add" | "char.effects.remove" => {
                let Ok(v) = serde_json::from_str::<Value>(data) else { return true };
                let name = text(&v, "name");
                if name.is_empty() {
                    return true;
                }
                if package.ends_with("add") {
                    if !self.status.effects.iter().any(|e| same(&e.name, &name)) {
                        self.status.effects.push(Effect { name: name.clone(), ends_at: None });
                        self.dirty = true;
                    }
                    self.changes.push(Change::Gained(name));
                } else {
                    let before = self.status.effects.len();
                    self.status.effects.retain(|e| !same(&e.name, &name));
                    self.dirty |= self.status.effects.len() != before;
                    self.changes.push(Change::Lost(name));
                }
                self.stale = true;
                true
            }
            "char.effects.groups" => {
                self.stale = true;
                true
            }
            "group" => {
                let group = serde_json::from_str::<Value>(data).ok().and_then(|v| self.group_of(&v));
                self.set_group(group);
                true
            }
            _ => false,
        }
    }

    fn group_of(&self, v: &Value) -> Option<Group> {
        let members = v.get("members").and_then(Value::as_array)?;
        let allies: Vec<Ally> = members
            .iter()
            .filter_map(|m| {
                let name = text(m, "name");
                let info = m.get("info").cloned().unwrap_or(Value::Null);
                (!name.is_empty()).then(|| Ally {
                    name,
                    hp: number(&info, "hp"),
                    max_hp: number(&info, "mhp"),
                    mana: number(&info, "mn"),
                    max_mana: number(&info, "mmn"),
                    moves: number(&info, "mv"),
                    max_moves: number(&info, "mmv"),
                    level: number(&info, "lvl"),
                })
            })
            .collect();
        let tank = Some(text(v, "tank")).filter(|t| !t.is_empty());
        Some(Group { leader: text(v, "leader"), tank, allies })
    }

    /// The group as it is now: the player left out, alone is none.
    fn set_group(&mut self, group: Option<Group>) {
        let me = self.me.to_lowercase();
        let group = group.map(|mut g| {
            g.allies.retain(|a| me.is_empty() || a.name.to_lowercase() != me);
            if g.tank.as_ref().is_some_and(|t| !me.is_empty() && t.to_lowercase() == me) {
                g.tank = Some("you".into());
            }
            g
        });
        let group = group.filter(|g| !g.allies.is_empty());
        let tank = |g: &Option<Group>| g.as_ref().and_then(|g| g.tank.clone());
        if let Some(t) = tank(&group).filter(|t| Some(t) != tank(&self.status.group).as_ref()) {
            self.changes.push(Change::Tank(t));
        }
        if group != self.status.group {
            self.status.group = group;
            self.dirty = true;
        }
    }

    /// Whether a hidden AFFECTS is wanted: something changed, and it's
    /// been `READ_EVERY_MS` since the last. The session asks only when
    /// it's gentle to (`Who::gentle`).
    pub fn wants(&self, now: i64) -> bool {
        self.stale && self.asked.is_none() && self.last_read.is_none_or(|at| now - at >= READ_EVERY_MS)
    }

    /// What to send to read the list.
    pub fn ask(&mut self, now: i64) -> &'static str {
        self.asked = Some(now);
        self.last_read = Some(now);
        self.stale = false;
        self.reading = None;
        "affects"
    }

    /// Whether AFFECTS' answer is still coming: WHO and the tickers wait.
    pub fn awaiting(&self, now: i64) -> bool {
        self.asked.is_some_and(|at| now - at < ANSWER_WAITS_MS)
    }

    /// The clock: cooldowns over.
    pub fn tick(&mut self, now: i64) {
        let before = self.status.cooldowns.len();
        let (over, left): (Vec<Cooldown>, Vec<Cooldown>) = std::mem::take(&mut self.status.cooldowns).into_iter().partition(|c| c.ends_at <= now);
        self.status.cooldowns = left;
        self.changes.extend(over.into_iter().map(|c| Change::Ready(c.name)));
        self.dirty |= self.status.cooldowns.len() != before;
    }

    /// One finished line: whether it's AFFECTS' answer, to hide.
    pub fn line(&mut self, line: &str, now: i64) -> bool {
        let said = squeeze(line);
        self.refused(&said, now);
        if self.asked.is_some_and(|at| now - at >= ANSWER_WAITS_MS) {
            self.asked = None;
            self.reading = None;
        }
        if self.asked.is_none() {
            return false;
        }
        match self.reading.take() {
            Some(Reading::Rows(mut rows, mut effects)) => {
                if said.is_empty() {
                    self.finish(rows, effects);
                } else {
                    effects.extend(row(line, now));
                    rows.push(said);
                    self.reading = Some(Reading::Rows(rows, effects));
                }
                true
            }
            reading => {
                let begun = matches!(reading, Some(Reading::States(_)));
                let mut states = match reading {
                    Some(Reading::States(states)) => states,
                    _ => Vec::new(),
                };
                if let Some(rest) = said.strip_prefix(AFFECTED) {
                    let rest = rest.trim();
                    self.status.states = states;
                    if rest == "Nothing!" {
                        self.finish(Vec::new(), Vec::new());
                    } else {
                        let effects = if rest.is_empty() { Vec::new() } else { row(rest, now) };
                        self.reading = Some(Reading::Rows(Vec::new(), effects));
                    }
                    return true;
                }
                // The answer starts with a blank line: a posture line before
                // one is the game's own ("You are knocked down.").
                let begun = begun || said.is_empty();
                let state = begun && (STATES.contains(&said.as_str()) || (said.starts_with("You are ") && said.ends_with('.') && said.len() < 80));
                if said.is_empty() || state {
                    if state {
                        states.push(said);
                    }
                    self.reading = Some(Reading::States(states));
                    return true;
                }
                // Not the answer: the game's own line in between.
                if begun {
                    self.reading = Some(Reading::States(states));
                }
                false
            }
        }
    }

    fn finish(&mut self, _rows: Vec<String>, effects: Vec<Effect>) {
        self.asked = None;
        self.reading = None;
        self.status.effects = effects;
        self.status.read = true;
        self.dirty = true;
    }

    /// The game refused a command that must wait: its cooldown.
    fn refused(&mut self, said: &str, now: i64) {
        let left = if let Some(time) = said.strip_prefix(WAIT.0).and_then(|rest| rest.strip_suffix(WAIT.1)) {
            game_time(time).map(|ms| (ms, true))
        } else {
            RECOVER.iter().find(|(line, _)| *line == said).map(|&(_, ms)| (ms, false))
        };
        let Some((ms, about)) = left else { return };
        let Some((command, _)) = self.sent.as_ref().filter(|(_, at)| now - at <= SENT_MS) else { return };
        let name = command.to_lowercase();
        self.status.cooldowns.retain(|c| c.name != name);
        self.status.cooldowns.push(Cooldown { name, ends_at: now + ms, about });
        self.dirty = true;
    }

    /// The status, when it changed since last taken.
    pub fn take_status(&mut self) -> Option<Status> {
        std::mem::take(&mut self.dirty).then(|| self.status.clone())
    }

    /// What changed since last asked.
    pub fn take_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes)
    }

    pub fn status(&self) -> &Status {
        &self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(w: &mut Watch, lines: &[&str], now: i64) -> Vec<bool> {
        lines.iter().map(|l| w.line(l, now)).collect()
    }

    /// AFFECTS as `Affect.execute` prints it, its two columns 36 wide.
    const AFFECTS: &[&str] = &[
        "",
        "You are standing.",
        "You are hungry.",
        "",
        "You are affected by: ",
        "(Shield spell) 3m 20s                (Blessed) 45s                       ",
        "(Sanctuary) 1h 2m",
        "",
    ];

    #[test]
    fn reads_affects_and_hides_it() {
        let mut w = Watch::default();
        assert_eq!(read(&mut w, AFFECTS, 0), [false; 8], "not asked: the player's own AFFECTS shows");
        assert!(!w.status().read);
        assert!(w.gmcp("char.effects.groups", r#"["spell-abjuration"]"#));
        assert!(w.wants(0));
        assert_eq!(w.ask(1_000), "affects");
        assert!(w.awaiting(1_000) && !w.wants(1_000));
        assert_eq!(read(&mut w, AFFECTS, 1_000), [true; 8]);
        let s = w.take_status().unwrap();
        let effects: Vec<(&str, Option<i64>)> = s.effects.iter().map(|e| (e.name.as_str(), e.ends_at)).collect();
        assert_eq!(effects, [("Shield spell", Some(201_000)), ("Blessed", Some(46_000)), ("Sanctuary", Some(1_000 + 3_720_000))]);
        assert_eq!(s.states, ["You are standing.", "You are hungry."]);
        assert!(s.read && !w.awaiting(1_000));
        // Over: the game's next lines show.
        assert!(!w.line("", 1_000) && !w.line("A goblin arrives.", 1_000));
    }

    #[test]
    fn nothing_and_a_line_between() {
        let mut w = Watch::default();
        w.gmcp("char.effects.groups", "[]");
        w.ask(0);
        // A fight's line before the answer is the game's to show, a posture's too.
        assert!(!w.line("The goblin misses you.", 0) && !w.line("You are knocked down.", 0));
        assert_eq!(read(&mut w, &["", "You are resting.", "", "You are affected by: Nothing!"], 0), [true; 4]);
        let s = w.take_status().unwrap();
        assert!(s.effects.is_empty() && s.read);
        assert_eq!(s.states, ["You are resting."]);
        assert!(!w.line("", 0));
    }

    #[test]
    fn entries_that_fill_their_column() {
        let full = "(Protection from Evil spell) 12m 1s(Haste spell) 2s";
        let e = row(full, 0);
        assert_eq!(e.iter().map(|e| (e.name.as_str(), e.ends_at)).collect::<Vec<_>>(), [("Protection from Evil spell", Some(721_000)), ("Haste spell", Some(2_000))]);
        // No time: forever, or words of the effect's own.
        assert_eq!(row("(Infravision)", 0)[0].ends_at, None);
        assert_eq!(row("Cursed by the gods       (Bless)", 0).iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Cursed by the gods", "Bless"]);
    }

    #[test]
    fn pushes_add_and_drop_between_reads() {
        let mut w = Watch::default();
        w.ask(0);
        read(&mut w, AFFECTS, 0);
        w.take_status();
        w.gmcp("char.effects.remove", r#"{"name":"Shield","group":"spell-abjuration"}"#);
        w.gmcp("char.effects.add", r#"{"name":"Poison","group":"poison-poisons"}"#);
        let names: Vec<String> = w.take_status().unwrap().effects.into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["Blessed", "Sanctuary", "Poison"]);
        assert_eq!(w.take_changes(), [Change::Lost("Shield".into()), Change::Gained("Poison".into())]);
        // Its time wants a read, but not within `READ_EVERY_MS` of the last.
        assert!(!w.wants(READ_EVERY_MS - 1) && w.wants(READ_EVERY_MS));
        // A push for one already listed adds nothing.
        w.gmcp("char.effects.add", r#"{"name":"Bless","group":"prayer-blessing"}"#);
        assert!(w.take_status().is_none());
    }

    #[test]
    fn an_answer_never_coming_hides_nothing_after() {
        let mut w = Watch::default();
        w.ask(0);
        assert!(w.line("", 0) && w.line("You are standing.", 0));
        assert!(!w.awaiting(ANSWER_WAITS_MS));
        assert!(!w.line("You are standing.", ANSWER_WAITS_MS));
    }

    #[test]
    fn cooldowns_from_the_games_refusals() {
        let mut w = Watch::default();
        w.sent("cast majesty", 0);
        assert!(!w.line("You must wait 2 hours before you can do that again.", 1_000), "the refusal shows");
        w.sent("deflect", 2_000);
        w.line("You must wait under an hour before you can do that again.", 2_000);
        w.sent("kick", 3_000);
        w.line("You need a second to recover before doing that again.", 3_000);
        let s = w.take_status().unwrap();
        let cd: Vec<(&str, i64, bool)> = s.cooldowns.iter().map(|c| (c.name.as_str(), c.ends_at, c.about)).collect();
        assert_eq!(cd, [("cast majesty", 1_000 + 3 * HOUR_MS, true), ("deflect", 2_000 + HOUR_MS, true), ("kick", 4_000, false)]);
        w.tick(4_000);
        assert_eq!(w.take_changes(), [Change::Ready("kick".into())]);
        assert_eq!(w.take_status().unwrap().cooldowns.len(), 2);
        // A refusal long after any command is no one's.
        w.line("You need a second to recover before doing that again.", 60_000);
        assert!(w.take_status().is_none());
        assert_eq!(game_time("1 day, 3 hours"), Some((DAY_HOURS + 4) * HOUR_MS));
    }

    #[test]
    fn the_group_without_the_player() {
        let mut w = Watch::default();
        w.me("Joash");
        let group = r#"{"groupname":"Bobs group","leader":"Bob","status":"Private","count":2,"tank":"Joash",
            "members":[{"name":"Bob","info":{"hp":37,"mhp":90,"mn":30,"mmn":40,"mv":80,"mmv":90,"lvl":7,"align":500,"tnl":120}},
                       {"name":"Joash","info":{"hp":50,"mhp":60,"mn":10,"mmn":20,"mv":70,"mmv":70,"lvl":5,"tnl":300}}]}"#;
        assert!(w.gmcp("group", group));
        let g = w.take_status().unwrap().group.unwrap();
        assert_eq!((g.leader.as_str(), g.tank.as_deref()), ("Bob", Some("you")));
        assert_eq!(g.allies, [Ally { name: "Bob".into(), hp: Some(37), max_hp: Some(90), mana: Some(30), max_mana: Some(40), moves: Some(80), max_moves: Some(90), level: Some(7) }]);
        assert_eq!(w.take_changes(), [Change::Tank("you".into())]);
        // The same again changes nothing; alone is no group.
        w.gmcp("group", group);
        assert!(w.take_status().is_none() && w.take_changes().is_empty());
        w.gmcp("group", r#"{"leader":"Joash","count":1,"members":[{"name":"Joash","info":{}}]}"#);
        assert_eq!(w.take_status().unwrap().group, None);
    }
}

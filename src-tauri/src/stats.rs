//! **Stats**: everything the game's STAT command can tell the player
//! about their character, read now and then by hidden STATs (their
//! answers kept out of the output, as AFFECTS' are in `effects.rs`) and
//! kept in Coupler's Stats table (`tables.rs`). Pure and unit-tested;
//! shared with the web build.
//!
//! STAT (Commands/Stat.java; the wiki's STAT page, read 2026-10-26,
//! agrees) for a player:
//! - `STAT ?` lists what can be asked, after "STAT allows the following
//!   options:", comma-separated and wrapped (`mob.tell`): the char state
//!   (HITS, MANA, MOVE, HUNGER, THIRST, FATIGUE, and each with MAX), the
//!   char stats (STRENGTH…, the saving throws, and each with BASE), the
//!   phy stats (SENSES, LEVEL, ARMOR…), STINK, XP and the rest, and the
//!   factions SCORE shows. A game with FULLSTATS disabled lists only its
//!   `LMT_STATS` instead (lists.ini).
//! - `STAT HITS MANA …` answers with each value and a space, on **one
//!   line** never wrapped (`wraplessPrintln`), written straight after the
//!   prompt (the game's input ends `lastWasPrompt`). A value that's
//!   words (`WORDY`) can't be told from the next, so those are asked one
//!   at a time; the rest go `BATCH` at a time, and a batch whose answer
//!   isn't that many numbers is asked again one by one. DAMAGE says its
//!   number on a line first (`M.tell`). An unknown code answers
//!   `*UNKNOWN:CODE*`, and isn't kept.
//! - Left out: ATTRIBUTES, RESISTS and HEALTH (each of their parts is a
//!   code of its own), XPSTATS (the XP codes), and the staff's options.
//!
//! **Gentle**: one hidden STAT at a time, only when the session says
//! Coupler's own commands go unnoticed (`Who::gentle`), its answer
//! awaited before the next; STAT costs no action. Read when the character
//! comes into the game, when their level changes, every `AGAIN_MS`, and
//! when the player asks (`#check stats`).

use std::collections::VecDeque;

/// How often the stats are read again, Unix milliseconds.
pub const AGAIN_MS: i64 = 15 * 60 * 1000;
/// The fewest milliseconds between two hidden STATs.
const ASK_EVERY_MS: i64 = 2_000;
/// How long an answer is waited for before nothing more is hidden for it.
const ANSWER_WAITS_MS: i64 = 8_000;
/// Codes asked in one STAT.
const BATCH: usize = 12;

const OPTIONS: &str = "STAT allows the following options:";
/// Codes whose value is words, asked alone.
const WORDY: &[&str] = &["SENSES", "DISPOSITION", "ARMOR", "DAMAGE", "ATTACK", "FOLLOWERS", "AMBIANCES"];
/// Codes not asked: the ones made of other codes.
const SKIPPED: &[&str] = &["ATTRIBUTES", "RESISTS", "HEALTH", "XPSTATS"];

/// A stat's name in words, for the table.
pub fn stat_name(code: &str) -> String {
    let named = |code: &str| -> String {
        match code {
            "HITS" => "hit points".into(),
            "MOVE" => "movement".into(),
            "XP" => "experience".into(),
            "XPTNL" => "experience to next level".into(),
            "XPFNL" => "experience for next level".into(),
            "QUESTPOINTS" => "quest points".into(),
            "WEIGHTADJ" => "weight adjustment".into(),
            "REJUV" => "rejuvenation".into(),
            other => other.to_lowercase().replace('_', " "),
        }
    };
    let words = if let Some(rest) = code.strip_prefix("MAX").filter(|r| !r.is_empty()) {
        format!("most {}", named(rest))
    } else if let Some(rest) = code.strip_prefix("BASE").filter(|r| !r.is_empty()) {
        format!("base {}", named(rest))
    } else {
        named(code)
    };
    let mut chars = words.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// A value as kept: GENDER's letter (the game says its character code).
fn tidy_value(code: &str, value: &str) -> String {
    if code == "GENDER" {
        if let Some(c) = value.parse::<u32>().ok().and_then(char::from_u32).filter(char::is_ascii_uppercase) {
            return c.to_string();
        }
    }
    value.trim().to_string()
}

/// A number as STAT writes one: `12`, `-3`, `40%`.
fn numeric(token: &str) -> bool {
    let digits = token.strip_prefix('-').unwrap_or(token);
    let digits = digits.strip_suffix('%').unwrap_or(digits);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// The line without the prompt the game wrote it after (`<20Hp 9m> 12 30`).
fn own(line: &str) -> &str {
    let line = line.trim();
    if line.starts_with('<') {
        if let Some(at) = line.find("> ") {
            return line[at + 2..].trim();
        }
        if line.ends_with('>') {
            return "";
        }
    }
    line
}

#[derive(Debug, PartialEq)]
enum Asked {
    /// `STAT ?`: the list, its codes so far once the heading's been seen.
    Options(Option<Vec<String>>),
    /// Values for these codes; DAMAGE's number, once seen.
    Values(Vec<String>, Option<String>),
}

#[derive(Debug, Default)]
pub struct Stats {
    /// What STAT can be asked, once read.
    codes: Option<Vec<String>>,
    /// What's still to ask in this reading, a STAT's codes each.
    queue: VecDeque<Vec<String>>,
    /// What was asked, and when.
    asked: Option<(Asked, i64)>,
    /// When the last STAT went, and when the last reading began.
    last_ask: Option<i64>,
    last_read: Option<i64>,
    /// A reading is wanted (it's been a while, the level changed, the player asked).
    stale: bool,
    /// The values read in this reading, in the list's order.
    read: Vec<(String, String)>,
    /// A finished reading, to be kept.
    done: Option<Vec<(String, String)>>,
    level: Option<i64>,
}

impl Stats {
    /// The connection or the character changed: the list's read again.
    pub fn clear(&mut self) {
        *self = Stats { stale: true, ..Stats::default() };
    }

    /// The player asked (`#check stats`), or something changed: read them again.
    pub fn refresh(&mut self) {
        self.stale = true;
    }

    /// GMCP `char.status`: a new level is a new reading.
    pub fn gmcp(&mut self, package: &str, data: &str) {
        if !package.eq_ignore_ascii_case("char.status") {
            return;
        }
        let Some(level) = serde_json::from_str::<serde_json::Value>(data).ok().and_then(|d| d.get("level").and_then(|l| l.as_i64())) else {
            return;
        };
        if self.level.is_some_and(|was| was != level) {
            self.stale = true;
        }
        self.level = Some(level);
    }

    /// Whether a hidden STAT is wanted now. The session asks only when
    /// it's gentle to (`Who::gentle`).
    pub fn wants(&self, now: i64) -> bool {
        if self.asked.is_some() || self.last_ask.is_some_and(|at| now - at < ASK_EVERY_MS) {
            return false;
        }
        !self.queue.is_empty() || self.stale || self.codes.is_none() || self.last_read.is_none_or(|at| now - at >= AGAIN_MS)
    }

    /// What to send next.
    pub fn ask(&mut self, now: i64) -> String {
        self.last_ask = Some(now);
        if self.queue.is_empty() {
            self.stale = false;
            self.last_read = Some(now);
            self.read.clear();
            match &self.codes {
                None => {
                    self.asked = Some((Asked::Options(None), now));
                    return "stat ?".into();
                }
                Some(codes) => self.queue = batches(codes),
            }
        }
        let Some(codes) = self.queue.pop_front() else {
            // Nothing STAT can be asked: the list's read again next time.
            self.codes = None;
            self.asked = Some((Asked::Options(None), now));
            return "stat ?".into();
        };
        let command = format!("stat {}", codes.join(" ").to_lowercase());
        self.asked = Some((Asked::Values(codes, None), now));
        command
    }

    /// Whether a hidden STAT's answer is still coming: WHO and the tickers wait.
    pub fn awaiting(&self, now: i64) -> bool {
        self.asked.as_ref().is_some_and(|(_, at)| now - at < ANSWER_WAITS_MS)
    }

    /// One finished line: whether it's a hidden STAT's answer, to hide.
    pub fn line(&mut self, line: &str, now: i64) -> bool {
        if self.asked.as_ref().is_some_and(|(_, at)| now - at >= ANSWER_WAITS_MS) {
            self.asked = None;
            self.finish_if_done();
        }
        let Some((asked, at)) = self.asked.take() else { return false };
        let said = own(line);
        match asked {
            Asked::Options(None) => {
                if let Some(rest) = said.strip_prefix(OPTIONS) {
                    // The list comes on the lines after it (`\n\r` in the heading).
                    let mut codes = Vec::new();
                    let more = options(rest, &mut codes) || rest.trim().is_empty();
                    self.options_read(codes, more, at);
                    return true;
                }
                // The game's own line in between.
                self.asked = Some((Asked::Options(None), at));
                false
            }
            Asked::Options(Some(mut codes)) => {
                let more = options(said, &mut codes);
                self.options_read(codes, more, at);
                true
            }
            Asked::Values(codes, None) if codes.len() == 1 && codes[0] == "DAMAGE" => {
                if numeric(said) {
                    self.asked = Some((Asked::Values(codes, Some(said.to_string())), at));
                    return true;
                }
                self.asked = Some((Asked::Values(codes, None), at));
                false
            }
            Asked::Values(codes, number) => {
                if codes.len() == 1 {
                    let mut value = said.to_string();
                    if let Some(n) = number {
                        value = if value.is_empty() { n } else { format!("{value} ({n})") };
                    }
                    self.value(&codes[0], &value);
                    self.finish_if_done();
                    return true;
                }
                let tokens: Vec<&str> = said.split_whitespace().collect();
                if tokens.len() == codes.len() && tokens.iter().all(|t| numeric(t)) {
                    for (code, value) in codes.iter().zip(tokens) {
                        self.value(code, value);
                    }
                } else if said.contains("*UNKNOWN:") || tokens.iter().filter(|t| numeric(t)).count() * 2 >= tokens.len().max(1) {
                    // Not a number each: asked again one by one.
                    for code in codes.into_iter().rev() {
                        self.queue.push_front(vec![code]);
                    }
                } else {
                    // The game's own line in between.
                    self.asked = Some((Asked::Values(codes, None), at));
                    return false;
                }
                self.finish_if_done();
                true
            }
        }
    }

    fn options_read(&mut self, codes: Vec<String>, more: bool, at: i64) {
        if more {
            self.asked = Some((Asked::Options(Some(codes)), at));
            return;
        }
        self.queue = batches(&codes);
        self.codes = Some(codes);
    }

    fn value(&mut self, code: &str, value: &str) {
        if value.contains("*UNKNOWN:") {
            return;
        }
        self.read.push((code.to_string(), tidy_value(code, value)));
    }

    /// The reading's over once nothing's left to ask: it's kept.
    fn finish_if_done(&mut self) {
        if self.queue.is_empty() && self.asked.is_none() && !self.read.is_empty() {
            self.done = Some(std::mem::take(&mut self.read));
        }
    }

    /// A finished reading, code and value each, to keep (`Tables::stats`).
    pub fn take_read(&mut self) -> Option<Vec<(String, String)>> {
        self.done.take()
    }
}

/// The codes in a piece of the options list; whether more is to come (it ended in a comma).
fn options(text: &str, codes: &mut Vec<String>) -> bool {
    let text = text.trim();
    for code in text.split(',').map(str::trim) {
        let code = code.to_uppercase();
        let plain = !code.is_empty() && code.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
        if plain && !SKIPPED.contains(&code.as_str()) && !codes.contains(&code) {
            codes.push(code);
        }
    }
    text.ends_with(',')
}

/// The STATs to send for these codes: the numbers `BATCH` at a time, the words alone.
fn batches(codes: &[String]) -> VecDeque<Vec<String>> {
    let (wordy, numbers): (Vec<&String>, Vec<&String>) = codes.iter().partition(|c| WORDY.contains(&c.as_str()));
    let mut out: VecDeque<Vec<String>> = numbers.chunks(BATCH).map(|c| c.iter().map(|s| s.to_string()).collect()).collect();
    out.extend(wordy.into_iter().map(|c| vec![c.clone()]));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "STAT allows the following options: ";

    /// Answers each STAT asked from `values` till nothing more is wanted.
    fn read_all(s: &mut Stats, mut t: i64, answer: impl Fn(&str) -> Vec<String>) -> i64 {
        while s.wants(t) {
            let command = s.ask(t);
            for line in answer(&command) {
                assert!(s.line(&line, t), "{line}");
            }
            t += ASK_EVERY_MS;
        }
        t
    }

    #[test]
    fn reads_the_list_then_the_values() {
        let mut s = Stats::default();
        let t = read_all(&mut s, 0, |command| match command {
            "stat ?" => vec![LIST.into(), "HITS, MANA, MAXHITS, STRENGTH, GENDER, ATTRIBUTES,".into(), "ARMOR, DAMAGE, STINK, ALIGNMENT".into()],
            "stat hits mana maxhits strength gender stink alignment" => vec!["<20Hp 9m> 20 9 31 16 77 40% -500".into()],
            "stat armor" => vec!["<20Hp 9m> pretty well armored".into()],
            "stat damage" => vec!["4".into(), "<20Hp 9m> good".into()],
            other => panic!("{other}"),
        });
        let read = s.take_read().unwrap();
        assert_eq!(
            read,
            [("HITS", "20"), ("MANA", "9"), ("MAXHITS", "31"), ("STRENGTH", "16"), ("GENDER", "M"), ("STINK", "40%"), ("ALIGNMENT", "-500"), ("ARMOR", "pretty well armored"), ("DAMAGE", "good (4)")]
                .map(|(c, v)| (c.to_string(), v.to_string()))
        );
        // Not again till it's been a while.
        assert!(!s.wants(t));
        assert!(s.wants(AGAIN_MS + 1));
    }

    #[test]
    fn a_batch_that_isnt_numbers_is_asked_one_by_one() {
        let mut s = Stats { codes: Some(vec!["HITS".into(), "ODD".into(), "MANA".into()]), ..Stats::default() };
        let mut asked = Vec::new();
        read_all(&mut s, 0, |command| {
            let answer = match command {
                "stat hits odd mana" => "20 *UNKNOWN:ODD* 9",
                "stat hits" => "20",
                "stat odd" => " *UNKNOWN:ODD* ",
                "stat mana" => "9",
                other => panic!("{other}"),
            };
            vec![answer.to_string()]
        });
        asked.extend(s.take_read().unwrap());
        assert_eq!(asked, [("HITS".to_string(), "20".to_string()), ("MANA".to_string(), "9".to_string())]);
    }

    #[test]
    fn the_games_own_line_in_between_shows() {
        let mut s = Stats { codes: Some(vec!["HITS".into(), "MANA".into()]), ..Stats::default() };
        assert_eq!(s.ask(0), "stat hits mana");
        assert!(!s.line("Bob says 'hello there friend.'", 10));
        assert!(!s.line("A rat arrives from the north.", 10));
        assert!(s.line("20 9", 20));
        assert_eq!(s.take_read().unwrap().len(), 2);
        // An answer that never came: nothing more is hidden.
        s.refresh();
        let t = 10 * ASK_EVERY_MS;
        assert_eq!(s.ask(t), "stat hits mana");
        assert!(s.awaiting(t + 1));
        assert!(!s.line("20 9", t + ANSWER_WAITS_MS));
        assert!(!s.awaiting(t + ANSWER_WAITS_MS));
    }

    #[test]
    fn a_new_level_reads_them_again() {
        let mut s = Stats { codes: Some(vec!["HITS".into()]), last_read: Some(0), ..Stats::default() };
        s.gmcp("char.status", r#"{"level":4}"#);
        assert!(!s.wants(ASK_EVERY_MS));
        s.gmcp("char.status", r#"{"level":5}"#);
        assert!(s.wants(ASK_EVERY_MS));
    }

    #[test]
    fn names_in_words() {
        assert_eq!(stat_name("HITS"), "Hit points");
        assert_eq!(stat_name("MAXHITS"), "Most hit points");
        assert_eq!(stat_name("BASESTRENGTH"), "Base strength");
        assert_eq!(stat_name("XPTNL"), "Experience to next level");
        assert_eq!(stat_name("ALIGNMENT"), "Alignment");
        assert_eq!(stat_name("MAX"), "Max");
    }
}

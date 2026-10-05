//! **Making a character**: CoffeeMUD's character creation as a series of
//! questions, each with its choices, so the frontend can guide a new
//! player through it (components/CreationDialog.tsx) instead of leaving
//! them in front of screens of text. Pure and unit-tested; `session.rs`
//! tells it each line sent and gives it each read's lines after
//! `speech.rs` has kinded them.
//!
//! Every question is CoffeeMUD's own (`Libraries/CharCreation.java`,
//! snapshot `c1e556f`), and each is asked with `promptPrint`, so it's the
//! unfinished line the read leaves (or, for a prompt that ends in a bare
//! `: `, the last finished line before it). The text the game printed
//! since the last question is the question's context: the intro
//! (`resources/text/races.txt` and the rest), the list of choices, the
//! help on a pick, a problem with the last answer.
//!
//! The order, with an account (coffeemud.net's games but Classic):
//! `account name:` → "'X' does not exist. Is this a new account you would
//! like to create (y/N)?" → (ANSI colors, only without MTTS, which
//! Coupler sends) → "Enter an account password" → "Enter your e-mail
//! address:" and "Re-enter:" → the account menu, "Command or Name (?):"
//! → N → "Please enter a name for your character, or '*'" → "Create a
//! new character called 'X' (y/N)?" → then the character's own steps:
//! a theme where a game has several, the race (a list, then its help and
//! "Is X correct (Y/n)?"), the gender, the stats (points to spend, or a
//! random roll to keep or roll again), the class (likewise), each faction
//! with choices at creation (alignment, inclination), a deity where the
//! game asks, and the rules, "Press Enter to begin". Without accounts
//! (Classic) it's `name:` → "Is this a new character you would like to
//! create (y/N)?" → "Enter a password:" → e-mail → the character's steps.
//! A game can switch any step off; one switched off is just never asked.
//!
//! What only color says: in a list of classes, the ones whose attack
//! stat is the character's best are in HIGHLIGHT, the others in white
//! (`buildQualifyingClassList`). That's `Choice::suggested`.
//!
//! Guiding starts when a new account or character is on the way (and at
//! any question only character creation asks), and ends when the
//! character comes into the game (`room.info`), at a login's password,
//! or when the connection ends. The account's questions are steps only
//! while guiding, so logging in to play never opens the guide.
//!
//! Passwords: a typed line is never kept. What the player picked is
//! taken from the game's own words (the race named in "Is Elf correct"),
//! and nothing is read from a line typed at a password question.

use std::collections::HashMap;

use serde::Serialize;

use crate::ansi::{Color, Line};
use crate::speech::LineKind;

/// Lines kept between questions; the longest context (a race's help) is
/// well under this.
const KEPT: usize = 400;

/// Lines that report a problem with the last answer (`CharCreation.java`).
const PROBLEMS: &[&str] = &[
    "a valid email address is required",
    "that email address combination was invalid",
    "you must enter a password to continue",
    "that is not a valid choice",
    "is not a valid deity",
    "points to do that, but only have",
    "you don't have enough remaining points",
    "you can not lower",
    "you can not raise",
    "is not a positive or negative number",
    "is an unknown code",
    "you do not qualify for any classes",
    "is not recognized",
    "that name is also not available",
    "choose another name",
    "you may only have",
    "this server is not accepting new",
    "maximum daily new",
    "aborted.",
    "is an unknown character or command",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepKind {
    /// `account name:` again, after a new account was turned down.
    AccountName,
    /// `name:` again, the same without accounts (Classic).
    LoginName,
    /// "Is this a new account you would like to create (y/N)?"
    NewAccount,
    /// "Is this a new character you would like to create (y/N)?" (Classic)
    NewCharacter,
    /// "Enter an account password" (echoed by the game: Coupler hides it).
    AccountPassword,
    /// "Enter your e-mail address:"
    Email,
    /// "Re-enter:" the address again.
    EmailAgain,
    /// The account menu, "Command or Name (?):"
    AccountMenu,
    /// "Please enter a name for your character, or '*'"
    CharacterName,
    /// "Create a new character called 'X' (y/N)?"
    ConfirmName,
    /// "Enter a password:" (the character's, without accounts).
    Password,
    /// "Do you want ANSI colors (Y/n)?"
    Colors,
    /// "Please select from the following: F/H/T"
    Theme,
    Race,
    /// "Is Elf correct (Y/n)?" after the race's help.
    ConfirmRace,
    /// "What is your gender (M/F)?"
    Gender,
    /// The stats and the points left to spend on them.
    Stats,
    /// The stats rolled at random: "Would you like to re-roll (y/N)?"
    Reroll,
    /// "How many points to add or remove (ex: +4, -1):"
    StatAmount,
    Class,
    ConfirmClass,
    /// A faction chosen at creation: "Select one: good, neutral, evil."
    Faction,
    Deity,
    ConfirmDeity,
    /// "Character creation complete!", the rules, "Press Enter to begin".
    Rules,
    /// Any other yes or no question while guiding.
    YesNo,
    /// Any other question while guiding.
    Other,
}

use StepKind::*;

/// One of a question's answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// As the game names it ("Half Elf", "Good").
    pub name: String,
    /// What's sent to choose it.
    pub send: String,
    /// The game's own words about it, where it gave some.
    pub about: Option<String>,
    /// A class suited to the character's best stat: shown only by color.
    pub suggested: bool,
}

/// One of the character's stats, as the game's table shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    /// "Strength"
    pub name: String,
    /// With the race's change in.
    pub value: u32,
    /// The most it can be.
    pub most: u32,
    /// What the race adds (or takes), 0 for none.
    pub race: i32,
    pub about: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub stats: Vec<Stat>,
    /// "STATS TOTAL", and the most it could be.
    pub total: u32,
    pub most: u32,
    /// Points left to spend; None when the game rolls the stats instead.
    pub points: Option<u32>,
    /// The classes these stats qualify for.
    pub qualifies: Vec<Choice>,
}

/// Something the player has settled ("Race", "Elf").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chosen {
    pub what: String,
    pub value: String,
}

/// A question the game is asking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub kind: StepKind,
    /// The game's question, as it asked.
    pub asked: String,
    /// What it's about: the name, race, class or deity in "Is X correct",
    /// or the faction ("Alignment").
    pub subject: Option<String>,
    pub choices: Vec<Choice>,
    /// The game's text around the question that isn't a choice: an intro,
    /// a pick's help. Lines as the game wrapped them, empty ones kept.
    pub text: Vec<String>,
    /// What the game said was wrong with the last answer.
    pub problem: Option<String>,
    pub stats: Option<Stats>,
    /// The answer is a password: hidden, never kept.
    pub secret: bool,
    /// What's settled so far, in order.
    pub chosen: Vec<Chosen>,
}

/// What a read did to the guide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    Step(Box<Step>),
    /// Guiding's over: the character's in the game, or it was a login.
    Ended,
}

#[derive(Default)]
pub struct Creation {
    active: bool,
    /// The lines since the last question: their text, squeezed, and spans.
    lines: Vec<(String, Line)>,
    /// The unfinished line the last read left, squeezed and as it was.
    last_prompt: String,
    last_raw: String,
    /// The last question sent.
    sent: Option<Step>,
    /// The game's words about each race, class, theme and stat, by name
    /// in lower case: an intro's printed once, its list again and again.
    about: HashMap<String, String>,
    /// An answer typed, waiting for the next question to show it was taken.
    pending: Option<(StepKind, Option<String>, Chosen)>,
    chosen: Vec<Chosen>,
}

/// Spaces collapsed, ends trimmed: wrapping and padding don't count.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_of(line: &Line) -> String {
    line.iter().map(|span| span.text.as_str()).collect()
}

/// "half elf" → "Half Elf".
fn capitalized(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text between the first `'` and the next: the name in "'Bob' does not exist."
fn quoted(s: &str) -> Option<String> {
    let start = s.find('\'')? + 1;
    let end = start + s[start..].find('\'')?;
    Some(s[start..end].to_string()).filter(|n| !n.is_empty())
}

/// "Is Elf correct (Y/n)?" → "Elf".
fn correct(q: &str) -> Option<String> {
    const END: &str = " correct (y/n)?";
    let head = q.get(..3)?;
    let tail = q.get(q.len().checked_sub(END.len())?..)?;
    if !head.eq_ignore_ascii_case("is ") || !tail.eq_ignore_ascii_case(END) {
        return None;
    }
    Some(q.get(3..q.len() - END.len())?.trim().to_string()).filter(|n| !n.is_empty())
}

/// What tells colors apart in a list: the foreground, the game's grey for
/// none. Not bold: the game sends a class's name bold in white (`^w`) and
/// the commas between in plain grey, the same color to the eye.
fn shade(fg: Option<Color>, _bold: bool) -> (Color, bool) {
    (fg.unwrap_or(Color::Index { index: 7 }), false)
}

/// A line without its first `n` characters.
fn drop_chars(line: &Line, mut n: usize) -> Line {
    let mut out = Line::new();
    for span in line {
        let len = span.text.chars().count();
        if n >= len {
            n -= len;
            continue;
        }
        let mut span = span.clone();
        span.text = span.text.chars().skip(n).collect();
        n = 0;
        out.push(span);
    }
    out
}

/// A list the game prints as "A, B, C or D" (or "and"), from its
/// characters and their shades: each name, and whether it's in another
/// color than the commas and the "or" between them.
fn list(chars: &[(char, (Color, bool))]) -> Vec<(String, bool)> {
    let text: String = chars.iter().map(|(c, _)| *c).collect();
    // Where each name starts and ends, in characters.
    let mut names: Vec<(usize, usize)> = Vec::new();
    let mut seps: Vec<(usize, usize)> = Vec::new();
    let words: Vec<char> = text.chars().collect();
    let mut start = 0;
    let mut i = 0;
    while i <= words.len() {
        let rest: String = words[i.min(words.len())..].iter().collect();
        let sep = if i == words.len() {
            Some(0)
        } else if rest.starts_with(',') {
            Some(1)
        } else if rest.starts_with(" or ") {
            Some(4)
        } else if rest.starts_with(" and ") {
            Some(5)
        } else {
            None
        };
        match sep {
            Some(len) => {
                names.push((start, i));
                if len > 0 {
                    seps.push((i, i + len));
                }
                start = i + len;
                i += len.max(1);
            }
            None => i += 1,
        }
    }
    let majority = |from: usize, to: usize| {
        let mut counts: Vec<((Color, bool), usize)> = Vec::new();
        for (c, s) in &chars[from..to] {
            if c.is_whitespace() || *c == ',' {
                continue;
            }
            match counts.iter_mut().find(|(k, _)| k == s) {
                Some((_, n)) => *n += 1,
                None => counts.push((*s, 1)),
            }
        }
        counts.into_iter().max_by_key(|(_, n)| *n).map(|(s, _)| s)
    };
    // The separators' shade: the ", " and " or " themselves, or failing
    // that (commas only take a color as text does), the shade of most names.
    let plain = {
        let mut counts: Vec<((Color, bool), usize)> = Vec::new();
        for &(from, to) in &seps {
            for (c, s) in &chars[from..to] {
                if c.is_alphabetic() || *c == ',' {
                    match counts.iter_mut().find(|(k, _)| k == s) {
                        Some((_, n)) => *n += 1,
                        None => counts.push((*s, 1)),
                    }
                }
            }
        }
        counts.into_iter().max_by_key(|(_, n)| *n).map(|(s, _)| s)
    };
    names
        .into_iter()
        .filter_map(|(from, to)| {
            let name = squeeze(&words[from..to].iter().collect::<String>());
            let name = name.trim_matches(|c: char| c == '.' || c == '[' || c == ']').trim().to_string();
            if name.is_empty() {
                return None;
            }
            let own = majority(from, to);
            Some((name, !seps.is_empty() && own.is_some() && plain.is_some() && own != plain))
        })
        .collect()
}

/// The characters of some lines joined by spaces, with their shades.
fn chars_of(lines: &[(String, Line)]) -> Vec<(char, (Color, bool))> {
    let mut chars = Vec::new();
    for (i, (_, line)) in lines.iter().enumerate() {
        if i > 0 {
            chars.push((' ', shade(None, false)));
        }
        for span in line {
            let s = shade(span.style.fg, span.style.bold);
            chars.extend(span.text.chars().map(|c| (c, s)));
        }
    }
    chars
}

/// The part of `chars` between the first `open` and the last `close`.
fn between(chars: &[(char, (Color, bool))], open: char, close: char) -> Option<Vec<(char, (Color, bool))>> {
    let from = chars.iter().position(|(c, _)| *c == open)? + 1;
    let to = chars.iter().rposition(|(c, _)| *c == close)?;
    (to >= from).then(|| chars[from..to].to_vec())
}

/// Whether a line is a problem the game reported.
fn problem(text: &str) -> bool {
    let low = text.to_lowercase();
    PROBLEMS.iter().any(|p| low.contains(p))
}

/// A stats table's row: "Strength       : 10/18 +1 from Elf".
fn stat_row(text: &str) -> Option<(String, u32, u32, i32)> {
    let (name, rest) = text.split_once(':')?;
    let name = name.trim();
    if name.is_empty() || name.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = rest.trim();
    let (value, rest) = rest.split_once('/')?;
    let value: u32 = value.trim().parse().ok()?;
    let most_len = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let most: u32 = rest[..most_len].parse().ok()?;
    let rest = rest[most_len..].trim();
    let race = rest.split_whitespace().next().and_then(|n| n.trim_start_matches('+').parse::<i32>().ok()).unwrap_or(0);
    Some((capitalized(name), value, most, race))
}

impl Creation {
    /// The connection changed: nothing's being made.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// A line the player sent. Never kept: at most it tells which of the
    /// question's own choices was picked.
    pub fn typed(&mut self, line: &str) {
        let Some(step) = &self.sent else { return };
        if !self.active || step.secret {
            return;
        }
        let answer = line.trim().to_uppercase();
        // "(Y/n)" takes an empty answer as yes, "(y/N)" as no.
        let yes = |default: bool| if answer.is_empty() { default } else { answer.starts_with('Y') };
        let subject = || step.subject.clone().unwrap_or_default();
        let named = |what: &str, value: String| Chosen { what: what.into(), value };
        let picked = || -> Option<String> {
            if answer.is_empty() {
                return None;
            }
            let by = |f: &dyn Fn(&Choice) -> bool| step.choices.iter().find(|c| f(c)).map(|c| c.name.clone());
            by(&|c| c.send.to_uppercase() == answer || c.name.to_uppercase() == answer)
                .or_else(|| by(&|c| c.name.to_uppercase().starts_with(&answer)))
                .or_else(|| by(&|c| c.send.len() == 1 && answer.starts_with(&c.send.to_uppercase())))
        };
        let pick = match step.kind {
            NewAccount if yes(false) => Some(named("Account", subject())),
            NewCharacter | ConfirmName if yes(false) => Some(named("Name", subject())),
            ConfirmRace if yes(true) => Some(named("Race", subject())),
            ConfirmClass if yes(true) => Some(named("Class", subject())),
            ConfirmDeity if yes(true) => Some(named("Deity", subject())),
            Theme => picked().map(|v| named("Theme", v)),
            Gender => picked().map(|v| named("Gender", v)),
            Faction => picked().map(|v| named(step.subject.as_deref().unwrap_or("Faction"), v)),
            _ => None,
        };
        self.pending = pick.map(|c| (step.kind, step.subject.clone(), c));
    }

    /// Whether a character's being made (guiding is on).
    pub fn active(&self) -> bool {
        self.active
    }

    /// The character came into the game (`room.info`): guiding's over.
    pub fn entered(&mut self) -> bool {
        let was = self.active;
        if was {
            self.clear();
        }
        was
    }

    /// One read's finished lines, their kinds, and the line it left
    /// unfinished: a new question, or the end of guiding. `playing`: a
    /// character's in the game (the game named them).
    pub fn lines(&mut self, lines: &[Line], kinds: &[LineKind], partial: Option<&Line>, playing: bool) -> Option<Update> {
        let mut added = false;
        for (i, line) in lines.iter().enumerate() {
            // The last question, finished by the answer: already seen.
            if kinds.get(i) == Some(&LineKind::Prompt) {
                continue;
            }
            // The first line may go on from the last ": " (the answer isn't
            // echoed): ": Your current stats are:". That part was the question.
            let raw = text_of(line);
            let line = if i == 0 && !self.last_raw.trim().is_empty() && raw.len() > self.last_raw.len() && raw.starts_with(&self.last_raw) {
                drop_chars(line, self.last_raw.chars().count())
            } else {
                line.clone()
            };
            self.lines.push((squeeze(&text_of(&line)), line));
            added = true;
        }
        if self.lines.len() > KEPT {
            self.lines.drain(..self.lines.len() - KEPT);
        }
        let raw = partial.map(text_of).unwrap_or_default();
        let full = squeeze(&raw);
        if !added && full == self.last_prompt {
            return None;
        }
        // A question asked without a new line first goes on from the last
        // one's ": " (the game doesn't echo the answer): "": Create a new
        // character called 'Zed' (y/N)?"". The question is what's new.
        let prompt = match full.strip_prefix(self.last_prompt.as_str()) {
            Some(rest) if !added && !self.last_prompt.is_empty() && !rest.trim().is_empty() => rest.trim().to_string(),
            _ => full.clone(),
        };
        self.last_prompt = full;
        self.last_raw = raw;
        let step = self.question(&prompt, playing)?;
        let Some(mut step) = step else {
            let was = self.active;
            self.clear();
            return was.then_some(Update::Ended);
        };
        if !self.active {
            return None;
        }
        // The same question again with nothing new (a read cut in two): no news.
        if let Some(sent) = &self.sent {
            if sent.kind == step.kind && sent.asked == step.asked && step.text.is_empty() && step.problem.is_none() && step.choices.is_empty() && step.stats.is_none() {
                return None;
            }
        }
        // The last answer was taken if the game has moved on.
        if let Some((kind, subject, chosen)) = self.pending.take() {
            if kind != step.kind || subject != step.subject {
                self.chosen.retain(|c| c.what != chosen.what);
                self.chosen.push(chosen);
            }
        }
        step.chosen = self.chosen.clone();
        self.sent = Some(step.clone());
        Some(Update::Step(Box::new(step)))
    }

    /// The question the game's asking, if it's one: Some(None) for a
    /// login's password (guiding ends), None for no question yet. Takes
    /// the lines it used.
    fn question(&mut self, prompt: &str, playing: bool) -> Option<Option<Step>> {
        // The question: the unfinished line, or for a bare ": " the line before it.
        let mut body: Vec<(String, Line)> = std::mem::take(&mut self.lines);
        while body.last().is_some_and(|(t, _)| t.is_empty()) {
            body.pop();
        }
        let asked = match prompt {
            "" => {
                // "Press Enter to begin:" is printed, not prompted.
                let Some(i) = body.iter().rposition(|(t, _)| t.to_lowercase().contains("press enter to begin")) else {
                    self.lines = body;
                    return None;
                };
                body.truncate(i + 1);
                body.pop()?.0
            }
            // Kept in the context too: it may be a list ("[Dwarf, Elf]").
            ":" => body.last()?.0.clone(),
            // A list printed, its ": " not yet come (a read can end between them).
            p if p.starts_with('[') => {
                self.lines = body;
                return None;
            }
            p => p.to_string(),
        };
        let low = asked.to_lowercase();
        let context = body;
        let mut step = Step {
            kind: Other,
            asked: asked.clone(),
            subject: None,
            choices: Vec::new(),
            text: Vec::new(),
            problem: None,
            stats: None,
            secret: false,
            chosen: Vec::new(),
        };
        // The lines that are the question's furniture, not its text.
        let mut used = vec![false; context.len()];
        if prompt == ":" {
            used[context.len() - 1] = true;
        }
        let problems: Vec<String> = context.iter().filter(|(t, _)| problem(t)).map(|(t, _)| t.clone()).collect();
        for (i, (t, _)) in context.iter().enumerate() {
            used[i] |= problem(t);
        }
        let find = |needle: &str| context.iter().rposition(|(t, _)| t.to_lowercase().contains(needle));
        let yes_no = || -> Vec<Choice> {
            vec![
                Choice { name: "Yes".into(), send: "Y".into(), about: None, suggested: false },
                Choice { name: "No".into(), send: "N".into(), about: None, suggested: false },
            ]
        };

        if low == "password:" || low.starts_with("password for ") {
            return Some(None);
        } else if low.starts_with("try entering y or n") {
            // The last question again: the answer was neither.
            let mut again = self.sent.clone()?;
            again.problem = Some("Answer yes or no.".into());
            again.chosen = Vec::new();
            return Some(Some(again));
        } else if low.ends_with("is this a new account you would like to create (y/n)?") || low.ends_with("is this a new character you would like to create (y/n)?") {
            step.kind = if low.contains("new account") { NewAccount } else { NewCharacter };
            step.subject = find("does not exist").and_then(|i| {
                used[i] = true;
                quoted(&context[i].0)
            });
            step.choices = yes_no();
            self.active = true;
        } else if low == "account name:" || low == "name:" {
            step.kind = if low == "name:" { LoginName } else { AccountName };
        } else if low.starts_with("enter an account password") {
            step.kind = AccountPassword;
            step.secret = true;
            self.active = true;
        } else if low == "enter your e-mail address:" {
            step.kind = Email;
            self.active = true;
        } else if low == "re-enter:" {
            step.kind = EmailAgain;
            if let Some(i) = find("is correct by re-entering") {
                used[i] = true;
            }
        } else if low.starts_with("command or name") {
            step.kind = AccountMenu;
            for (i, (t, _)) in context.iter().enumerate() {
                // The menu's own lines ("L)ist characters"), its title and its footer.
                let mut cs = t.chars();
                let menu = cs.next().is_some_and(char::is_alphabetic) && cs.next() == Some(')');
                if menu || t.eq_ignore_ascii_case("account menu") || t.to_lowercase().contains("enter your character name to login") {
                    used[i] = true;
                }
            }
            let item = |name: &str, send: &str| Choice { name: name.into(), send: send.into(), about: None, suggested: false };
            step.choices = vec![item("Make a new character", "N"), item("List your characters", "L"), item("Help", "?"), item("Quit", "Q")];
        } else if low.starts_with("please enter a name for your character") {
            step.kind = CharacterName;
            step.choices = vec![Choice { name: "A random name".into(), send: "*".into(), about: None, suggested: false }];
            self.active = true;
        } else if low.starts_with("create a new character called") {
            step.kind = ConfirmName;
            step.subject = quoted(&asked);
            step.choices = yes_no();
            self.active = true;
        } else if low == "enter a password:" {
            step.kind = Password;
            step.secret = true;
            self.active = true;
        } else if low.starts_with("do you want ansi colors") {
            step.kind = Colors;
            step.choices = yes_no();
            self.active = true;
        } else if low.starts_with("please select from the following:") {
            step.kind = Theme;
            self.remember(&context, &mut used, &["fantasy", "technological", "heroic"]);
            let letters = asked.split_once(':').map_or("", |(_, l)| l);
            step.choices = letters
                .split('/')
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(|letter| {
                    let named = ["Fantasy", "Technological", "Heroic"].into_iter().find(|n| n.starts_with(&letter.to_uppercase()));
                    let name = named.map_or(letter.to_string(), str::to_string);
                    Choice { about: self.about.get(&name.to_lowercase()).cloned(), name, send: letter.to_string(), suggested: false }
                })
                .collect();
            self.active = true;
        } else if let Some(header) = find("please choose from the following races") {
            step.kind = Race;
            step.choices = self.listed(&context, &mut used, header, false);
            self.active = true;
        } else if let Some(header) = find("please choose from the following classes") {
            step.kind = Class;
            step.choices = self.listed(&context, &mut used, header, true);
            self.active = true;
        } else if let Some(header) = find("please choose from the following deities") {
            step.kind = Deity;
            step.choices = self.listed(&context, &mut used, header, false);
            self.active = true;
        } else if let Some(name) = correct(&asked) {
            step.kind = match self.sent.as_ref().map(|s| s.kind) {
                Some(Class | ConfirmClass) => ConfirmClass,
                Some(Deity | ConfirmDeity) => ConfirmDeity,
                _ => ConfirmRace,
            };
            step.subject = Some(name);
            step.choices = yes_no();
            self.active = true;
        } else if low.starts_with("what is your gender") {
            step.kind = Gender;
            let letters = asked.rsplit_once('(').map_or("", |(_, l)| l).trim_end_matches(['?', ')']);
            step.choices = letters
                .split('/')
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(|l| {
                    let name = match l.to_uppercase().as_str() {
                        "M" => "Male".to_string(),
                        "F" => "Female".to_string(),
                        "N" => "Neuter".to_string(),
                        other => other.to_string(),
                    };
                    Choice { name, send: l.to_string(), about: None, suggested: false }
                })
                .collect();
            self.active = true;
        } else if low.starts_with("enter a stat to") || low.starts_with("would you like to re-roll") {
            step.kind = if low.starts_with("would") { Reroll } else { Stats };
            step.stats = Some(self.stats(&context, &mut used));
            if step.kind == Reroll {
                step.choices = vec![
                    Choice { name: "Keep these".into(), send: "N".into(), about: None, suggested: false },
                    Choice { name: "Roll again".into(), send: "Y".into(), about: None, suggested: false },
                ];
            }
            self.active = true;
        } else if low.starts_with("how many points to add or remove") {
            step.kind = StatAmount;
        } else if low.starts_with("select one:") {
            step.kind = Faction;
            let choices = asked.split_once(':').map_or("", |(_, c)| c).trim().trim_end_matches('.');
            step.choices = choices
                .split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(|c| Choice { name: capitalized(c), send: c.to_lowercase(), about: None, suggested: false })
                .collect();
            let intro = context.iter().zip(&used).filter(|(_, u)| !**u).map(|((t, _), _)| t.to_lowercase()).collect::<Vec<_>>().join(" ");
            step.subject = if intro.contains("alignment") {
                Some("Alignment".into())
            } else if intro.contains("inclination") {
                Some("Inclination".into())
            } else if intro.trim().is_empty() {
                // The same faction asked again.
                self.sent.as_ref().filter(|s| s.kind == Faction).and_then(|s| s.subject.clone())
            } else {
                None
            };
            self.active = true;
        } else if low.contains("press enter to begin") {
            step.kind = Rules;
            step.choices = vec![Choice { name: "Begin".into(), send: String::new(), about: None, suggested: false }];
            self.active = true;
        } else if low.ends_with("(y/n)?") {
            step.kind = YesNo;
            step.choices = yes_no();
        }
        // In the game, a question not known is the game's (after the stats
        // rolled again in play, `Prop_ReRollStats`): guiding's over.
        if playing && matches!(step.kind, Other | YesNo) {
            return Some(None);
        }
        step.text = self.text(&context, &used);
        step.problem = (!problems.is_empty()).then(|| problems.join(" "));
        Some(Some(step))
    }

    /// The lines not used, empty runs made one and none at the ends.
    fn text(&self, context: &[(String, Line)], used: &[bool]) -> Vec<String> {
        let mut text: Vec<String> = Vec::new();
        for (i, (_, line)) in context.iter().enumerate() {
            if used[i] {
                continue;
            }
            let t = text_of(line).trim_end().to_string();
            if t.trim().is_empty() && text.last().is_none_or(|l| l.is_empty()) {
                continue;
            }
            text.push(if t.trim().is_empty() { String::new() } else { t });
        }
        while text.last().is_some_and(String::is_empty) {
            text.pop();
        }
        text
    }

    /// Entries like "Dwarf   : Dwarves are shorter than humans…" for the
    /// names given, kept in `about`. An
    /// entry goes on over the lines after it up to an empty one or the next.
    fn remember(&mut self, context: &[(String, Line)], used: &mut [bool], names: &[&str]) {
        let start = |t: &str| -> Option<(String, String)> {
            let (name, rest) = t.split_once(':')?;
            let name = name.trim();
            names.iter().any(|n| n.eq_ignore_ascii_case(name)).then(|| (name.to_lowercase(), rest.trim().to_string()))
        };
        let mut i = 0;
        while i < context.len() {
            let Some((name, mut about)) = start(&context[i].0).filter(|_| !used[i]) else {
                i += 1;
                continue;
            };
            used[i] = true;
            i += 1;
            while i < context.len() && !used[i] && !context[i].0.is_empty() && start(&context[i].0).is_none() {
                about.push(' ');
                about.push_str(&context[i].0);
                used[i] = true;
                i += 1;
            }
            if !about.trim().is_empty() {
                self.about.insert(name, squeeze(&about));
            }
        }
    }

    /// A list in brackets after its header ("[Dwarf, Elf or Gnome]"), each
    /// with what the intro before it said, and for classes which suit.
    fn listed(&mut self, context: &[(String, Line)], used: &mut [bool], header: usize, classes: bool) -> Vec<Choice> {
        used[header] = true;
        let rest = &context[header + 1..];
        let open = rest.iter().position(|(t, _)| t.contains('['));
        let close = rest.iter().rposition(|(t, _)| t.contains(']'));
        let names = match (open, close) {
            (Some(open), Some(close)) if close >= open => {
                for u in &mut used[header + 1 + open..=header + 1 + close] {
                    *u = true;
                }
                between(&chars_of(&rest[open..=close]), '[', ']').map(|chars| list(&chars)).unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let lower: Vec<String> = names.iter().map(|(n, _)| n.to_lowercase()).collect();
        let lower: Vec<&str> = lower.iter().map(String::as_str).collect();
        self.remember(&context[..header], &mut used[..header], &lower);
        names
            .into_iter()
            .map(|(name, suggested)| Choice { about: self.about.get(&name.to_lowercase()).cloned(), send: name.clone(), name, suggested: classes && suggested })
            .collect()
    }

    /// The stats table, the points left and the classes they'd qualify for.
    fn stats(&mut self, context: &[(String, Line)], used: &mut [bool]) -> Stats {
        let mut stats = Stats { stats: Vec::new(), total: 0, most: 0, points: None, qualifies: Vec::new() };
        let mut i = 0;
        while i < context.len() {
            let t = &context[i].0;
            let low = t.to_lowercase();
            if low.starts_with("your current stats are") {
                used[i] = true;
            } else if let Some((name, value, most, race)) = stat_row(t) {
                used[i] = true;
                if name.eq_ignore_ascii_case("stats total") {
                    stats.total = value;
                    stats.most = most;
                } else {
                    stats.stats.push(Stat { name, value, most, race, about: None });
                }
            } else if low.starts_with("you have no more points remaining") {
                used[i] = true;
                stats.points = Some(0);
            } else if let Some(n) = low.strip_prefix("you have ").and_then(|r| r.strip_suffix(" points remaining.")) {
                used[i] = true;
                stats.points = n.trim().parse().ok();
            } else if low.starts_with("this would qualify you for ") {
                // Its list may wrap: it runs to the line that ends it.
                let from = i;
                while i + 1 < context.len() && !context[i].0.ends_with('.') {
                    i += 1;
                }
                for u in &mut used[from..=i] {
                    *u = true;
                }
                let chars = chars_of(&context[from..=i]);
                let lead: Vec<char> = "qualify you for ".chars().collect();
                let skip = chars.windows(lead.len()).position(|w| w.iter().zip(&lead).all(|((c, _), l)| c.to_ascii_lowercase() == *l)).map_or(chars.len(), |at| at + lead.len());
                stats.qualifies = list(&chars[skip..])
                    .into_iter()
                    .map(|(name, suggested)| Choice { about: self.about.get(&name.to_lowercase()).cloned(), send: name.clone(), name, suggested })
                    .collect();
            }
            i += 1;
        }
        // stats.txt's entries ("Strength: Physical strength…"), the first time.
        let mut names: Vec<String> = stats.stats.iter().map(|s| s.name.to_lowercase()).collect();
        for n in ["strength", "intelligence", "dexterity", "constitution", "charisma", "wisdom"] {
            if !names.iter().any(|m| m == n) {
                names.push(n.to_string());
            }
        }
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        self.remember(context, used, &names);
        for stat in &mut stats.stats {
            stat.about = self.about.get(&stat.name.to_lowercase()).cloned();
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::{Span, Style};

    const GREY: Option<Color> = Some(Color::Index { index: 7 });
    const CYAN: Option<Color> = Some(Color::Index { index: 14 });

    fn plain(text: &str) -> Line {
        vec![Span { text: text.into(), style: Style::default() }]
    }

    /// A line from `|`-marked text: what's between bars is in HIGHLIGHT.
    fn marked(text: &str) -> Line {
        text.split('|')
            .enumerate()
            .filter(|(_, t)| !t.is_empty())
            .map(|(i, t)| Span { text: t.into(), style: Style { fg: if i % 2 == 1 { CYAN } else { GREY }, ..Style::default() } })
            .collect()
    }

    /// A read of plain lines and the unfinished one.
    fn read(c: &mut Creation, lines: &[&str], partial: &str) -> Option<Update> {
        let lines: Vec<Line> = lines.iter().map(|l| plain(l)).collect();
        let kinds = vec![LineKind::Game; lines.len()];
        let partial = plain(partial);
        c.lines(&lines, &kinds, Some(&partial), false)
    }

    fn step(update: Option<Update>) -> Step {
        match update {
            Some(Update::Step(step)) => *step,
            other => panic!("no step: {other:?}"),
        }
    }

    fn names(step: &Step) -> Vec<&str> {
        step.choices.iter().map(|c| c.name.as_str()).collect()
    }

    /// The account's questions, up to the menu.
    fn new_account(c: &mut Creation) {
        assert_eq!(read(c, &["", "Welcome!"], "account name: "), None);
        let s = step(read(c, &["", "'Bob' does not exist."], "Is this a new account you would like to create (y/N)?"));
        assert_eq!((s.kind, s.subject.as_deref()), (NewAccount, Some("Bob")));
        c.typed("y");
        let s = step(read(c, &["", "", "Welcome to CoffeeMud!", "", "Thank you for creating a new account.", "", "Enter an account password"], ": "));
        assert_eq!(s.kind, AccountPassword);
        assert!(s.secret);
        assert_eq!(s.text, vec!["Welcome to CoffeeMud!", "", "Thank you for creating a new account."]);
        assert_eq!(s.chosen, vec![Chosen { what: "Account".into(), value: "Bob".into() }]);
    }

    #[test]
    fn logging_in_never_guides() {
        let mut c = Creation::default();
        assert_eq!(read(&mut c, &["Welcome"], "account name: "), None);
        assert_eq!(read(&mut c, &[], "password: "), None);
        assert_eq!(read(&mut c, &["", "Account Menu", "L)ist characters"], "Command or Name (?): "), None);
        assert!(!c.active());
        // Nor anyone's text that looks like a yes or no.
        assert_eq!(read(&mut c, &["A rat."], "Really quit (y/N)?"), None);
    }

    #[test]
    fn a_new_account_then_its_first_character() {
        let mut c = Creation::default();
        new_account(&mut c);
        c.typed("secret!");
        let s = step(read(&mut c, &["", "CoffeeMud supports email!"], "Enter your e-mail address: "));
        assert_eq!((s.kind, s.text.clone()), (Email, vec!["CoffeeMud supports email!".to_string()]));
        let s = step(read(&mut c, &["Confirm that 'b@x.org' is correct by re-entering."], "Re-enter: "));
        assert_eq!(s.kind, EmailAgain);
        assert!(s.text.is_empty());
        let s = step(read(&mut c, &["", "That email address combination was invalid.", ""], "Enter your e-mail address: "));
        assert_eq!(s.problem.as_deref(), Some("That email address combination was invalid."));
        let s = step(read(&mut c, &["", "Welcome back!", " Account Menu", " L) ist characters", " N) ew character (50 remaining)", "", " (Enter your character name to login)"], "Command or Name (?): "));
        assert_eq!(s.kind, AccountMenu);
        assert_eq!(s.text, vec!["Welcome back!"]);
        assert_eq!(names(&s)[0], "Make a new character");
        let s = step(read(&mut c, &["", "Please enter a name for your character, or '*'"], ": "));
        assert_eq!(s.kind, CharacterName);
        let s = step(read(&mut c, &[], "Create a new character called 'Zed' (y/N)?"));
        assert_eq!((s.kind, s.subject.as_deref()), (ConfirmName, Some("Zed")));
        c.typed("y");
        let s = step(read(&mut c, &["", "Please choose from the following races (?):", "[Elf, Human]"], ": "));
        assert_eq!(s.chosen.iter().map(|c| c.value.as_str()).collect::<Vec<_>>(), vec!["Bob", "Zed"]);
    }

    #[test]
    fn without_accounts_it_starts_at_the_name() {
        let mut c = Creation::default();
        assert_eq!(read(&mut c, &[], "name: "), None);
        let s = step(read(&mut c, &["", "'Zed' does not exist."], "Is this a new character you would like to create (y/N)?"));
        assert_eq!((s.kind, s.subject.as_deref()), (NewCharacter, Some("Zed")));
        c.typed("Y");
        let s = step(read(&mut c, &["", "Welcome to CoffeeMud Classic!"], "Enter a password: "));
        assert_eq!(s.kind, Password);
        assert!(s.secret);
        assert_eq!(s.chosen[0].value, "Zed");
    }

    #[test]
    fn turning_down_a_new_account_asks_the_name_again() {
        let mut c = Creation::default();
        step(read(&mut c, &["'Bobb' does not exist."], "Is this a new account you would like to create (y/N)?"));
        c.typed("n");
        let s = step(read(&mut c, &[], "account name: "));
        assert_eq!(s.kind, AccountName);
        assert!(s.chosen.is_empty());
        // Logging in to the right one ends it.
        assert_eq!(read(&mut c, &[], "password: "), Some(Update::Ended));
        assert!(!c.active());
    }

    #[test]
    fn a_password_typed_is_never_kept() {
        let mut c = Creation::default();
        new_account(&mut c);
        c.typed("hunter2");
        let s = step(read(&mut c, &[], "Enter your e-mail address: "));
        assert!(!format!("{s:?}").contains("hunter2"));
        assert!(c.pending.is_none());
    }

    #[test]
    fn races_with_what_the_intro_says() {
        let mut c = Creation::default();
        let s = step(read(
            &mut c,
            &[
                "",
                "Choose a race for your character. Although races are meant primarily for role-playing purposes,",
                "there are some significant differences.",
                "Dwarf   : Dwarves are shorter than humans, but much stockier,",
                "and enjoy sporting beards.",
                "Half Elf: Half Elves are a mixture of humans and elves.",
                "Human   : Just like you and me!",
                "",
                "Please choose from the following races (?):",
                "[Dwarf, Half Elf, Human, Gnome,",
                "Elf]",
            ],
            ": ",
        ));
        assert_eq!(s.kind, Race);
        assert_eq!(names(&s), vec!["Dwarf", "Half Elf", "Human", "Gnome", "Elf"]);
        assert_eq!(s.choices[0].about.as_deref(), Some("Dwarves are shorter than humans, but much stockier, and enjoy sporting beards."));
        assert_eq!(s.choices[1].send, "Half Elf");
        assert_eq!(s.choices[3].about, None);
        assert!(s.choices.iter().all(|c| !c.suggested));
        assert_eq!(s.text.len(), 2);
        // The list again (after a No) has no intro, but the words are remembered.
        let s = step(read(&mut c, &["", "Please choose from the following races (?):", "[Dwarf, Human]"], ": "));
        assert_eq!(s.choices[0].about.as_deref(), Some("Dwarves are shorter than humans, but much stockier, and enjoy sporting beards."));
    }

    #[test]
    fn a_pick_then_its_help_then_yes() {
        let mut c = Creation::default();
        step(read(&mut c, &["Please choose from the following races (?):", "[Dwarf, Elf]"], ": "));
        c.typed("el");
        let s = step(read(&mut c, &["", "Race: Elf", "Elves are quick.", ""], "Is Elf correct (Y/n)?"));
        assert_eq!((s.kind, s.subject.as_deref()), (ConfirmRace, Some("Elf")));
        assert_eq!(s.text, vec!["Race: Elf", "Elves are quick."]);
        // Return alone is yes.
        c.typed("");
        let s = step(read(&mut c, &[], "What is your gender (M/F)?"));
        assert_eq!(s.kind, Gender);
        assert_eq!(names(&s), vec!["Male", "Female"]);
        assert_eq!(s.chosen, vec![Chosen { what: "Race".into(), value: "Elf".into() }]);
        c.typed("f");
        let s = step(read(&mut c, &["Your current stats are:", "Strength       : 9 /18"], "Enter a Stat to add or remove points, ? for help, or R for random roll."));
        assert_eq!(s.chosen.last().unwrap(), &Chosen { what: "Gender".into(), value: "Female".into() });
    }

    #[test]
    fn a_no_takes_nothing_and_neither_is_asked_again() {
        let mut c = Creation::default();
        step(read(&mut c, &["Please choose from the following races (?):", "[Dwarf, Elf]"], ": "));
        step(read(&mut c, &[], "Is Elf correct (Y/n)?"));
        c.typed("maybe");
        let s = step(read(&mut c, &[], "Try entering Y or N: "));
        assert_eq!((s.kind, s.subject.as_deref(), s.problem.as_deref()), (ConfirmRace, Some("Elf"), Some("Answer yes or no.")));
        c.typed("n");
        let s = step(read(&mut c, &["Please choose from the following races (?):", "[Dwarf, Elf]"], ": "));
        assert_eq!(s.kind, Race);
        assert!(s.chosen.is_empty());
    }

    #[test]
    fn the_stats_to_spend_points_on() {
        let mut c = Creation::default();
        let s = step(read(
            &mut c,
            &[
                "Your stats below reflect your physical and mental gifts.",
                "",
                "Strength: Physical strength and fighting prowess.",
                "Wisdom: Intuition and wit.",
                "",
                "Your current stats are: ",
                "Strength       : 10/18 ",
                "Intelligence   : 4 /18 -1 from Elf",
                "Dexterity      : 12/20 +2 from Elf",
                "Wisdom         : 3 /18 ",
                "STATS TOTAL    : 29/74",
                "",
                "This would qualify you for Fighter, Thief and",
                "Apprentice.",
                "",
                "You have 7 points remaining.",
                "Enter a Stat to add or remove points, ? for help, or R for random roll.",
            ],
            ": ",
        ));
        assert_eq!(s.kind, Stats);
        let st = s.stats.unwrap();
        assert_eq!(st.points, Some(7));
        assert_eq!((st.total, st.most), (29, 74));
        assert_eq!(st.stats.len(), 4);
        assert_eq!(st.stats[1], Stat { name: "Intelligence".into(), value: 4, most: 18, race: -1, about: None });
        assert_eq!(st.stats[0].about.as_deref(), Some("Physical strength and fighting prowess."));
        assert_eq!(st.qualifies.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["Fighter", "Thief", "Apprentice"]);
        assert_eq!(s.text, vec!["Your stats below reflect your physical and mental gifts."]);
        // Spent: none left, and the stats' words are still known.
        let s = step(read(&mut c, &["Your current stats are: ", "Strength       : 17/18 ", "", "You have no more points remaining."], "Enter a Stat to remove points, ? for help, R for random roll, or ENTER to complete."));
        let st = s.stats.unwrap();
        assert_eq!(st.points, Some(0));
        assert_eq!(st.stats[0].about.as_deref(), Some("Physical strength and fighting prowess."));
        // A problem with the last change.
        let s = step(read(&mut c, &["You need 2 points to do that, but only have 1 remaining."], "Enter a Stat to remove points, ? for help, R for random roll, or ENTER to complete."));
        assert_eq!(s.problem.as_deref(), Some("You need 2 points to do that, but only have 1 remaining."));
    }

    #[test]
    fn stats_rolled_at_random() {
        let mut c = Creation::default();
        let s = step(read(&mut c, &["Your current stats are:", "Strength       : 15/18 ", "STATS TOTAL    : 15/18"], "Would you like to re-roll (y/N)?"));
        assert_eq!(s.kind, Reroll);
        assert_eq!(s.stats.as_ref().unwrap().points, None);
        assert_eq!(names(&s), vec!["Keep these", "Roll again"]);
    }

    #[test]
    fn classes_suited_to_the_best_stat_are_told() {
        let mut c = Creation::default();
        let lines = vec![
            plain("Fighter   : Fighters are brutish weapon masters."),
            plain("Please choose from the following Classes:"),
            marked("[|Fighter|, Thief, |Barbarian| or Mage]"),
        ];
        let kinds = vec![LineKind::Game; 3];
        let s = step(c.lines(&lines, &kinds, Some(&plain(": ")), false));
        assert_eq!(s.kind, Class);
        assert_eq!(names(&s), vec!["Fighter", "Thief", "Barbarian", "Mage"]);
        assert_eq!(s.choices.iter().map(|c| c.suggested).collect::<Vec<_>>(), vec![true, false, true, false]);
        assert_eq!(s.choices[0].about.as_deref(), Some("Fighters are brutish weapon masters."));
        // Its confirmation is the class's.
        let s = step(read(&mut c, &["Fighter help."], "Is Fighter correct (Y/n)?"));
        assert_eq!(s.kind, ConfirmClass);
        c.typed("y");
        let s = step(read(&mut c, &["Your alignment represents your moral fiber."], "Select one: good, neutral, evil."));
        assert_eq!((s.kind, s.subject.as_deref()), (Faction, Some("Alignment")));
        assert_eq!(names(&s), vec!["Good", "Neutral", "Evil"]);
        assert_eq!(s.choices[2].send, "evil");
        assert_eq!(s.chosen[0], Chosen { what: "Class".into(), value: "Fighter".into() });
        c.typed("ne");
        let s = step(read(&mut c, &["Your inclination represents your tendency to social order."], "Select one: lawful, moderate, chaotic."));
        assert_eq!(s.subject.as_deref(), Some("Inclination"));
        assert_eq!(s.chosen[1], Chosen { what: "Alignment".into(), value: "Neutral".into() });
    }

    #[test]
    fn the_same_faction_asked_again_keeps_its_name() {
        let mut c = Creation::default();
        step(read(&mut c, &["Your alignment represents your moral fiber."], "Select one: good, neutral, evil."));
        c.typed("purple");
        // Asked again as the game does: the line printed, then ": ".
        let s = step(read(&mut c, &["Select one: good, neutral, evil."], ": "));
        assert_eq!(s.subject.as_deref(), Some("Alignment"));
        assert!(s.chosen.is_empty());
    }

    #[test]
    fn a_theme_a_deity_and_the_rules() {
        let mut c = Creation::default();
        let s = step(read(&mut c, &["Select from one of the following themes.", "", "Fantasy      : Battle the forces of good and evil.", "", "Heroic       : Play a superhero."], "Please select from the following: F/H"));
        assert_eq!(s.kind, Theme);
        assert_eq!(names(&s), vec!["Fantasy", "Heroic"]);
        assert_eq!(s.choices[1].about.as_deref(), Some("Play a superhero."));
        assert_eq!(s.text, vec!["Select from one of the following themes."]);
        let s = step(read(&mut c, &["Please choose from the following deities to serve:", "[Zeus, Odin or Ra]"], ": "));
        assert_eq!((s.kind, names(&s)), (Deity, vec!["Zeus", "Odin", "Ra"]));
        c.typed("Odin");
        step(read(&mut c, &["Odin, the all-father."], "Is Odin correct (Y/n)?"));
        c.typed("y");
        let s = step(read(&mut c, &["", "Character creation complete!", "The Rules of CoffeeMud!", "1. No killing newbies.", "", "Press Enter to begin:"], ""));
        assert_eq!(s.kind, Rules);
        assert_eq!(s.text, vec!["Character creation complete!", "The Rules of CoffeeMud!", "1. No killing newbies."]);
        assert_eq!(s.chosen[0], Chosen { what: "Deity".into(), value: "Odin".into() });
        assert!(c.entered());
        assert!(!c.active());
        assert!(!c.entered());
    }

    #[test]
    fn nothing_new_is_no_news() {
        let mut c = Creation::default();
        step(read(&mut c, &[], "What is your gender (M/F)?"));
        assert_eq!(read(&mut c, &[], "What is your gender (M/F)?"), None);
        // Text that isn't a question yet waits for one.
        assert_eq!(read(&mut c, &["Some words."], ""), None);
        let s = step(read(&mut c, &[], "What is your gender (M/F/N)?"));
        assert_eq!(s.text, vec!["Some words."]);
        assert_eq!(names(&s), vec!["Male", "Female", "Neuter"]);
    }

    #[test]
    fn other_questions_while_guiding() {
        let mut c = Creation::default();
        step(read(&mut c, &[], "What is your gender (M/F)?"));
        assert_eq!(step(read(&mut c, &[], "Quit -- are you sure (y/N)?")).kind, YesNo);
        assert_eq!(step(read(&mut c, &[], "Something new: ")).kind, Other);
    }

    #[test]
    fn stats_rolled_again_in_play_end_at_the_next_prompt() {
        let mut c = Creation::default();
        let lines = vec![plain("Your current stats are:"), plain("Strength       : 15/18 ")];
        let kinds = vec![LineKind::Game; 2];
        assert_eq!(step(c.lines(&lines, &kinds, Some(&plain("Would you like to re-roll (y/N)?")), true)).kind, Reroll);
        assert_eq!(c.lines(&[plain("You are hungry.")], &[LineKind::Game], Some(&plain("<20hp 100m> ")), true), Some(Update::Ended));
        assert!(!c.active());
    }

    #[test]
    fn a_list_cut_off_before_its_colon_waits_for_it() {
        let mut c = Creation::default();
        // As the game sends it: the list with print, then "\n\r: " in a write of its own.
        assert_eq!(read(&mut c, &["Choose a race.", "", "Please choose from the following races (?):"], "[Dwarf, Elf, Half Elf]"), None);
        let s = step(read(&mut c, &["[Dwarf, Elf, Half Elf]"], ": "));
        assert_eq!((s.kind, names(&s)), (Race, vec!["Dwarf", "Elf", "Half Elf"]));
        assert_eq!(s.text, vec!["Choose a race."]);
    }

    #[test]
    fn a_question_going_on_from_the_last_colon() {
        let mut c = Creation::default();
        step(read(&mut c, &["Please enter a name for your character, or '*'"], ": "));
        c.typed("Zed");
        // The answer isn't echoed: the game's next question goes on from the ": ".
        let s = step(read(&mut c, &[], ": Create a new character called 'Zed' (y/N)?"));
        assert_eq!((s.kind, s.subject.as_deref(), s.asked.as_str()), (ConfirmName, Some("Zed"), "Create a new character called 'Zed' (y/N)?"));
    }

    #[test]
    fn the_stats_again_go_on_from_the_colon() {
        let mut c = Creation::default();
        step(read(&mut c, &["Your current stats are:", "Strength       : 3 /18 ", "", "You have 52 points remaining.", "Enter a Stat to add or remove points, ? for help, or R for random roll."], ": "));
        c.typed("strength +1");
        let s = step(read(&mut c, &[": Your current stats are:", "Strength       : 4 /18 ", "", "You have 51 points remaining.", "Enter a Stat to add or remove points, ? for help, or R for random roll."], ": "));
        assert!(s.text.is_empty(), "{:?}", s.text);
        assert_eq!(s.stats.unwrap().stats[0].value, 4);
        let s = step(read(&mut c, &[": You need 405 points to do that, but only have 51 remaining.", "Enter a Stat to add or remove points, ? for help, or R for random roll."], ": "));
        assert_eq!(s.problem.as_deref(), Some("You need 405 points to do that, but only have 51 remaining."));
    }

    #[test]
    fn plain_class_names_are_bold_white_as_the_game_sends_them() {
        // As coffeemud's stock colors come: ^w bold grey, ^H bold cyan, the commas plain grey.
        let span = |t: &str, i: u8, bold: bool| Span { text: t.into(), style: Style { fg: Some(Color::Index { index: i }), bold, ..Style::default() } };
        let list = vec![span("[", 7, false), span("Bard", 7, true), span(", ", 7, false), span("Fighter", 6, true), span(", or ", 7, false), span("Thief", 7, true), span("]", 7, false)];
        let mut c = Creation::default();
        let lines = vec![plain("Please choose from the following Classes:"), list];
        let s = step(c.lines(&lines, &[LineKind::Game; 2], Some(&plain(": ")), false));
        assert_eq!(names(&s), vec!["Bard", "Fighter", "Thief"]);
        assert_eq!(s.choices.iter().map(|c| c.suggested).collect::<Vec<_>>(), vec![false, true, false]);
    }
}

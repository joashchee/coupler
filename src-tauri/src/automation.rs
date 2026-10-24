//! **Triggers, gags, aliases and tickers**: the player's own rules for
//! the game's lines and their commands, pure and unit-tested, shared with
//! the web build. The frontend keeps the rules (`coupler.automation`,
//! so they're in a backup) and hands them over with `automation_set`;
//! `session.rs` runs them.
//!
//! - **A trigger** is a pattern a finished line of the game's matches,
//!   and what then happens: commands sent, words the narrator says, a
//!   sound, and the line gagged or not.
//! - **A gag** is a pattern whose lines aren't shown, said or read.
//! - **An alias** is a word the player types that stands for commands:
//!   `%1` to `%9` its words after, `%*` all of them; with neither, those
//!   words go on the end of its last command. A line typed with `\`
//!   first is sent as it is.
//! - **A ticker** sends commands every so many seconds.
//!
//! A pattern is the whole line, any letters' case, `*` any text (none at
//! all too), each `*`'s text `%1`, `%2`… in what it sets off, `%0` the
//! whole line.
//!
//! **Gentle to a shared game** (CLAUDE.md, "Testing against the live
//! game"): nothing is sent unless a character is in the game and no
//! password is being typed; a ticker waits at least `MIN_EVERY` and not
//! once the player's been away 9 minutes (`who.rs`, the game's idle
//! timers); and everything Coupler sends on the player's rules shares
//! one allowance, `BURST` in `BURST_WINDOW`, so a trigger answering its
//! own line can't flood the game.
//!
//! The last lines seen (`RECENT`), gagged or not, are kept for the
//! dialog's Create and Gag buttons.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::clock::Instant;

/// How many of the last lines are kept, so each list has its ten.
const RECENT: usize = 40;
/// A ticker's shortest wait, in seconds.
pub const MIN_EVERY: u32 = 15;
/// The commands Coupler may send on the player's rules in a while.
const BURST: usize = 10;
const BURST_WINDOW: Duration = Duration::from_secs(10);
/// How deep an alias may use another.
const DEPTH: usize = 4;
/// The most commands one typed line may become.
const MOST_LINES: usize = 20;

/// The player's rules, as the frontend keeps them.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Rules {
    pub triggers: Vec<LineTrigger>,
    pub gags: Vec<Gag>,
    pub aliases: Vec<Alias>,
    pub tickers: Vec<Ticker>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LineTrigger {
    pub id: String,
    pub pattern: String,
    /// Commands, one a line.
    pub send: String,
    /// Words the narrator says.
    pub say: String,
    /// A sound effect: an asset's path, or one of CoffeeMUD's sounds (`bank/…`).
    pub sound: Option<String>,
    /// The line isn't shown either.
    pub gag: bool,
    pub on: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Gag {
    pub id: String,
    pub pattern: String,
    pub on: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Alias {
    pub id: String,
    /// One word.
    pub name: String,
    pub send: String,
    pub on: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Ticker {
    pub id: String,
    /// Seconds, `MIN_EVERY` at least.
    pub every: u32,
    pub send: String,
    pub on: bool,
}

/// A line seen lately.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recent {
    pub text: String,
    /// A gag or a trigger's gag hid it.
    pub gagged: bool,
}

/// What one read's lines set off.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Actions {
    /// Commands to send, aliases already expanded.
    #[serde(skip)]
    pub send: Vec<String>,
    pub say: Vec<String>,
    pub sounds: Vec<String>,
}

/// A pattern made ready: its literal pieces between the `*`s, lowercased.
#[derive(Clone, Debug)]
pub struct Pattern {
    pieces: Vec<String>,
}

impl Pattern {
    pub fn new(pattern: &str) -> Pattern {
        Pattern { pieces: pattern.trim().split('*').map(|p| p.to_ascii_lowercase()).collect() }
    }

    /// The `*`s' texts, if `line` matches.
    pub fn captures(&self, line: &str) -> Option<Vec<String>> {
        let line = line.trim();
        let lower = line.to_ascii_lowercase();
        let (first, rest) = self.pieces.split_first()?;
        let Some((last, middle)) = rest.split_last() else {
            return (lower == *first).then(Vec::new);
        };
        if !lower.starts_with(first.as_str()) || lower.len() < first.len() + last.len() || !lower.ends_with(last.as_str()) {
            return None;
        }
        let end = lower.len() - last.len();
        let mut at = first.len();
        let mut captures = Vec::new();
        for piece in middle {
            let found = lower[at..end].find(piece.as_str())? + at;
            captures.push(line[at..found].to_string());
            at = found + piece.len();
        }
        captures.push(line[at..end].to_string());
        Some(captures)
    }

    /// Matches every line: nothing but `*`s.
    pub fn everything(&self) -> bool {
        self.pieces.iter().all(String::is_empty) && self.pieces.len() > 1
    }
}

/// `template` with `%0` the whole, `%1`… the captures, `%*` them all.
fn fill(template: &str, whole: &str, captures: &[String]) -> String {
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('0') => {
                chars.next();
                out.push_str(whole);
            }
            Some(d @ '1'..='9') => {
                chars.next();
                out.push_str(captures.get(d as usize - '1' as usize).map_or("", String::as_str));
            }
            Some('*') => {
                chars.next();
                out.push_str(&captures.join(" "));
            }
            Some('%') => {
                chars.next();
                out.push('%');
            }
            _ => out.push('%'),
        }
    }
    out
}

/// Whether a template takes words: `%1`… or `%*`.
fn takes_words(template: &str) -> bool {
    let bytes = template.as_bytes();
    bytes.windows(2).any(|w| w[0] == b'%' && (w[1].is_ascii_digit() && w[1] != b'0' || w[1] == b'*'))
}

/// A rule's commands, one a line, blank ones left out.
pub fn commands(send: &str) -> Vec<String> {
    send.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()
}

/// The player's rules at work.
#[derive(Default)]
pub struct Automation {
    rules: Rules,
    triggers: Vec<Pattern>,
    gags: Vec<Pattern>,
    recent: VecDeque<Recent>,
    recent_changed: bool,
    /// When Coupler last sent on the player's rules, within `BURST_WINDOW`.
    sent: VecDeque<Instant>,
    /// Something was held back since last told.
    held: bool,
    /// Each ticker's next time, by its ID, and the wait it was set for.
    next: HashMap<String, (Instant, u32)>,
}

impl Automation {
    pub fn set(&mut self, rules: Rules) {
        self.triggers = rules.triggers.iter().map(|t| Pattern::new(&t.pattern)).collect();
        self.gags = rules.gags.iter().map(|g| Pattern::new(&g.pattern)).collect();
        self.next.retain(|id, (_, every)| rules.tickers.iter().any(|t| t.id == *id && t.every == *every && t.on));
        self.rules = rules;
    }

    /// The last lines seen, the newest last.
    pub fn recent(&self) -> Vec<Recent> {
        self.recent.iter().cloned().collect()
    }

    /// Whether new lines came since last asked.
    pub fn take_recent_changed(&mut self) -> bool {
        std::mem::take(&mut self.recent_changed)
    }

    /// Whether something was held back since last asked (too much at once).
    pub fn take_held(&mut self) -> bool {
        std::mem::take(&mut self.held)
    }

    /// The connection ended: tickers start over with the next.
    pub fn closed(&mut self) {
        self.next.clear();
        self.sent.clear();
    }

    /// One read's finished lines, as printed. Which to gag (by place), and
    /// what they set off. `acting`: a character in the game and no
    /// password being typed, so commands may go.
    pub fn lines(&mut self, texts: &[String], acting: bool, now: Instant) -> (Vec<usize>, Actions) {
        let mut gagged = Vec::new();
        let mut actions = Actions::default();
        for (i, text) in texts.iter().enumerate() {
            if text.trim().is_empty() {
                continue;
            }
            let mut gag = self.rules.gags.iter().zip(&self.gags).any(|(g, p)| g.on && p.captures(text).is_some());
            for (trigger, pattern) in self.rules.triggers.iter().zip(&self.triggers) {
                if !trigger.on {
                    continue;
                }
                let Some(captures) = pattern.captures(text) else { continue };
                gag |= trigger.gag;
                let whole = text.trim();
                if acting {
                    actions.send.extend(commands(&fill(&trigger.send, whole, &captures)));
                }
                let say = fill(&trigger.say, whole, &captures);
                if !say.trim().is_empty() {
                    actions.say.push(say.trim().to_string());
                }
                actions.sounds.extend(trigger.sound.clone().filter(|s| !s.is_empty()));
            }
            if gag {
                gagged.push(i);
            }
            if self.recent.len() == RECENT {
                self.recent.pop_front();
            }
            self.recent.push_back(Recent { text: text.trim_end().to_string(), gagged: gag });
            self.recent_changed = true;
        }
        let expanded: Vec<String> = actions.send.iter().flat_map(|line| self.expand(line).unwrap_or_else(|| vec![line.clone()])).collect();
        actions.send = self.allow(expanded, now);
        (gagged, actions)
    }

    /// A typed line as the commands its alias stands for, or None when
    /// it's no alias (sent as typed). `\` first sends the rest as it is.
    pub fn expand(&self, line: &str) -> Option<Vec<String>> {
        if let Some(rest) = line.trim_start().strip_prefix('\\') {
            return Some(vec![rest.to_string()]);
        }
        let mut out = Vec::new();
        self.expand_into(line, 0, &mut Vec::new(), &mut out).then_some(out)
    }

    /// Whether `line` was an alias; its commands go in `out`.
    fn expand_into(&self, line: &str, depth: usize, using: &mut Vec<String>, out: &mut Vec<String>) -> bool {
        let trimmed = line.trim();
        let (word, rest) = trimmed.split_once(char::is_whitespace).map_or((trimmed, ""), |(w, r)| (w, r.trim()));
        let alias = self.rules.aliases.iter().find(|a| a.on && !a.name.is_empty() && a.name.eq_ignore_ascii_case(word));
        let Some(alias) = alias.filter(|a| depth < DEPTH && !using.iter().any(|u| u.eq_ignore_ascii_case(&a.name))) else {
            return false;
        };
        let words: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
        let mut lines = commands(&fill(&alias.send, rest, &words));
        if !takes_words(&alias.send) && !rest.is_empty() {
            if let Some(last) = lines.last_mut() {
                last.push(' ');
                last.push_str(rest);
            }
        }
        using.push(alias.name.clone());
        for line in lines {
            if out.len() >= MOST_LINES {
                break;
            }
            if !self.expand_into(&line, depth + 1, using, out) {
                out.push(line);
            }
        }
        using.pop();
        true
    }

    /// What may go of `lines` now, within the allowance; the rest held back.
    fn allow(&mut self, lines: Vec<String>, now: Instant) -> Vec<String> {
        while self.sent.front().is_some_and(|&t| now.duration_since(t) >= BURST_WINDOW) {
            self.sent.pop_front();
        }
        let room = BURST.saturating_sub(self.sent.len());
        if lines.len() > room {
            self.held = true;
        }
        let allowed: Vec<String> = lines.into_iter().take(room).collect();
        self.sent.extend(std::iter::repeat_n(now, allowed.len()));
        allowed
    }

    /// The tickers due now: their commands, aliases expanded. `present`:
    /// a character in the game, no password, the player not away
    /// (`Who::present`); a ticker that comes due while not is skipped,
    /// never saved up. `awaiting`: Coupler's WHO is waiting for its reply
    /// (`Who::awaiting`); a ticker due then goes once it's done, so WHO's
    /// minute never makes it miss its turn.
    pub fn tick(&mut self, now: Instant, present: bool, awaiting: bool) -> Vec<String> {
        let mut due = Vec::new();
        for ticker in self.rules.tickers.iter().filter(|t| t.on) {
            let every = ticker.every.max(MIN_EVERY);
            let next = self.next.entry(ticker.id.clone()).or_insert((now + Duration::from_secs(every.into()), ticker.every));
            if now < next.0 || (present && awaiting) {
                continue;
            }
            next.0 = now + Duration::from_secs(every.into());
            if present {
                due.extend(commands(&ticker.send));
            }
        }
        let expanded: Vec<String> = due.iter().flat_map(|line| self.expand(line).unwrap_or_else(|| vec![line.clone()])).collect();
        self.allow(expanded, now)
    }
}

/// A pattern offered for a line, and what it catches in words.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub pattern: String,
    pub about: String,
}

/// Patterns for a trigger or a gag on `line`, the closest first. `names`
/// are the people and creatures the player knows of (the cast), so a
/// line with one can be about anyone.
pub fn suggest(line: &str, names: &[String]) -> Vec<Suggestion> {
    let line = line.trim();
    let mut out: Vec<Suggestion> = Vec::new();
    let mut offer = |pattern: String, about: String| {
        let pattern = squash_stars(&pattern);
        if !pattern.trim().is_empty() && !Pattern::new(&pattern).everything() && !out.iter().any(|s| s.pattern == pattern) {
            out.push(Suggestion { pattern, about });
        }
    };
    offer(line.to_string(), "Exactly this line.".into());

    let numbered = numbers_starred(line);
    if numbered != line {
        offer(numbered.clone(), "This line with any numbers in place of these.".into());
    }
    let (named, who) = names_starred(&numbered, names);
    if let Some(who) = &who {
        offer(named.clone(), format!("This line about anyone, not only {who}."));
    }
    let quoted = quotes_starred(&named);
    if quoted != named {
        offer(quoted.clone(), "This line, whatever's said in the quotes.".into());
    }
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() > 3 {
        let start = words[..3].join(" ");
        offer(format!("{} *", numbers_starred(&start)), format!("Any line that starts \"{start}\"."));
        let end = words[words.len() - 3..].join(" ");
        offer(format!("* {}", numbers_starred(&end)), format!("Any line that ends \"{end}\"."));
    }
    if let Some(longest) = words.iter().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric())).filter(|w| w.len() >= 4 && !w.chars().any(|c| c.is_ascii_digit())).max_by_key(|w| w.len()) {
        offer(format!("*{longest}*"), format!("Any line with \"{longest}\" in it."));
    }
    out
}

/// Runs of `*` as one.
fn squash_stars(pattern: &str) -> String {
    let mut out = String::new();
    for c in pattern.chars() {
        if c == '*' && out.ends_with('*') {
            continue;
        }
        out.push(c);
    }
    out
}

/// Every number (digits, with commas between) as `*`.
fn numbers_starred(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            while chars.peek().is_some_and(|n| n.is_ascii_digit() || *n == ',') {
                chars.next();
            }
            out.push('*');
        } else {
            out.push(c);
        }
    }
    out
}

/// The longest known name in the line (whole words) as `*`, and the name.
fn names_starred(line: &str, names: &[String]) -> (String, Option<String>) {
    let lower = line.to_ascii_lowercase();
    let mut best: Option<(usize, &str)> = None;
    for name in names {
        let name = name.trim();
        if name.len() < 3 {
            continue;
        }
        let Some(at) = lower.find(&name.to_ascii_lowercase()) else { continue };
        let before = lower[..at].chars().next_back();
        let after = lower[at + name.len()..].chars().next();
        let whole = before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric());
        if whole && best.is_none_or(|(_, b)| name.len() > b.len()) {
            best = Some((at, name));
        }
    }
    match best {
        Some((at, name)) => (format!("{}*{}", &line[..at], &line[at + name.len()..]), Some(line[at..at + name.len()].to_string())),
        None => (line.to_string(), None),
    }
}

/// What's in single or double quotes as `*`.
fn quotes_starred(line: &str) -> String {
    for quote in ['\'', '"'] {
        if let (Some(open), Some(close)) = (line.find(quote), line.rfind(quote)) {
            if close > open + 1 {
                return format!("{}{quote}*{quote}{}", &line[..open], &line[close + 1..]);
            }
        }
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Rules {
        Rules {
            triggers: vec![LineTrigger { id: "t".into(), pattern: "* says 'heal me'".into(), send: "cast heal %1".into(), say: "%1 wants healing".into(), sound: None, gag: false, on: true }],
            gags: vec![Gag { id: "g".into(), pattern: "The wind blows*".into(), on: true }],
            aliases: vec![
                Alias { id: "a".into(), name: "kk".into(), send: "kill %1\nbash %1".into(), on: true },
                Alias { id: "b".into(), name: "gg".into(), send: "get all".into(), on: true },
                Alias { id: "c".into(), name: "loop".into(), send: "loop".into(), on: true },
                Alias { id: "d".into(), name: "both".into(), send: "gg corpse\nkk orc".into(), on: true },
            ],
            tickers: vec![Ticker { id: "k".into(), every: 30, send: "drink skin".into(), on: true }],
        }
    }

    fn texts(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn patterns() {
        let p = Pattern::new("* says '*'");
        assert_eq!(p.captures("Bob says 'hello there'"), Some(vec!["Bob".into(), "hello there".into()]));
        assert_eq!(p.captures("BOB SAYS 'x'"), Some(vec!["BOB".into(), "x".into()]));
        assert_eq!(p.captures("Bob tells you 'hi'"), None);
        assert_eq!(Pattern::new("You are hungry.").captures("  You are HUNGRY.  "), Some(vec![]));
        assert_eq!(Pattern::new("You are hungry.").captures("You are hungry. Very."), None);
        assert_eq!(Pattern::new("*gold*").captures("You get 12 gold coins."), Some(vec!["You get 12 ".into(), " coins.".into()]));
        assert_eq!(Pattern::new("a*a").captures("a"), None);
        assert_eq!(Pattern::new("a*a").captures("aa"), Some(vec!["".into()]));
        assert!(Pattern::new("**").everything());
        assert!(!Pattern::new("*a*").everything());
    }

    #[test]
    fn triggers_and_gags() {
        let mut a = Automation::default();
        a.set(rules());
        let t = Instant::now();
        let (gagged, actions) = a.lines(&texts(&["The wind blows hard.", "Ann says 'heal me'", ""]), true, t);
        assert_eq!(gagged, [0]);
        assert_eq!(actions.send, ["cast heal Ann"]);
        assert_eq!(actions.say, ["Ann wants healing"]);
        assert_eq!(a.recent(), [Recent { text: "The wind blows hard.".into(), gagged: true }, Recent { text: "Ann says 'heal me'".into(), gagged: false }]);
        // Not in the game, or typing a password: nothing's sent, the rest still happens.
        let (_, actions) = a.lines(&texts(&["Ann says 'heal me'"]), false, t);
        assert!(actions.send.is_empty());
        assert_eq!(actions.say, ["Ann wants healing"]);
    }

    #[test]
    fn aliases() {
        let mut a = Automation::default();
        a.set(rules());
        assert_eq!(a.expand("kk orc"), Some(vec!["kill orc".into(), "bash orc".into()]));
        assert_eq!(a.expand("GG corpse"), Some(vec!["get all corpse".into()]));
        assert_eq!(a.expand("both"), Some(vec!["get all corpse".into(), "kill orc".into(), "bash orc".into()]));
        assert_eq!(a.expand("loop"), Some(vec!["loop".into()]));
        assert_eq!(a.expand("\\kk orc"), Some(vec!["kk orc".into()]));
        assert_eq!(a.expand("look"), None);
    }

    #[test]
    fn the_allowance() {
        let mut a = Automation::default();
        a.set(Rules { triggers: vec![LineTrigger { id: "t".into(), pattern: "*".into(), send: "say again".into(), on: true, ..Default::default() }], ..Rules::default() });
        let t = Instant::now();
        let many: Vec<String> = (0..15).map(|i| format!("line {i}")).collect();
        let (_, actions) = a.lines(&many, true, t);
        assert_eq!(actions.send.len(), BURST);
        assert!(a.take_held());
        assert!(a.lines(&many[..1], true, t + Duration::from_secs(1)).1.send.is_empty());
        assert_eq!(a.lines(&many[..1], true, t + BURST_WINDOW).1.send.len(), 1);
    }

    #[test]
    fn tickers() {
        let mut a = Automation::default();
        a.set(rules());
        let t = Instant::now();
        assert!(a.tick(t, true, false).is_empty());
        assert!(a.tick(t + Duration::from_secs(29), true, false).is_empty());
        assert_eq!(a.tick(t + Duration::from_secs(30), true, false), ["drink skin"]);
        // Away: skipped, not saved up.
        assert!(a.tick(t + Duration::from_secs(60), false, false).is_empty());
        assert!(a.tick(t + Duration::from_secs(61), true, false).is_empty());
        assert_eq!(a.tick(t + Duration::from_secs(90), true, false), ["drink skin"]);
        // WHO awaited: it waits for it, then goes, and keeps its time from then.
        assert!(a.tick(t + Duration::from_secs(120), true, true).is_empty());
        assert!(a.tick(t + Duration::from_secs(125), true, true).is_empty());
        assert_eq!(a.tick(t + Duration::from_secs(126), true, false), ["drink skin"]);
        assert!(a.tick(t + Duration::from_secs(155), true, false).is_empty());
        assert_eq!(a.tick(t + Duration::from_secs(156), true, false), ["drink skin"]);
        // Never more often than MIN_EVERY.
        a.set(Rules { tickers: vec![Ticker { id: "q".into(), every: 1, send: "look".into(), on: true }], ..Rules::default() });
        assert!(a.tick(t + Duration::from_secs(100), true, false).is_empty());
        assert!(a.tick(t + Duration::from_secs(105), true, false).is_empty());
        assert_eq!(a.tick(t + Duration::from_secs(115), true, false), ["look"]);
    }

    /// Two hours played, second by second, as `session.rs` runs them:
    /// the tickers, then WHO's clock, its reply a second later (now and
    /// then none). The player types every minute or two. Every ticker
    /// goes within a WHO's wait of when it's due, never while a WHO's
    /// reply is awaited (it would be hidden with it), and none is lost.
    #[test]
    fn tickers_keep_time_beside_who() {
        use crate::who::Who;
        const PROMPT: &str = "<100hp 50m 80mv> ";
        let start = Instant::now();
        let at = |s: u64| start + Duration::from_secs(s);
        for every in [15u32, 20, 30, 45, 59, 60, 61, 90, 120, 300, 480] {
            let mut who = Who::default();
            who.character("bob", at(0));
            who.typed("look", at(0));
            for _ in 0..3 {
                who.partial(Some(PROMPT), at(0));
            }
            let mut a = Automation::default();
            a.set(Rules { tickers: vec![Ticker { id: "k".into(), every, send: "drink skin".into(), on: true }], ..Rules::default() });
            let mut sent = Vec::new();
            let mut reply_at = None;
            let mut whos = 0;
            for s in 1..2 * 60 * 60 {
                let now = at(s);
                // The player, every 70 to 130 seconds.
                if s % (70 + (s / 7 % 61)) == 0 {
                    who.typed("look", now);
                    who.partial(Some(PROMPT), now);
                }
                let due = a.tick(now, who.present(now), who.awaiting());
                if !due.is_empty() {
                    assert!(reply_at.is_none(), "every {every}: a ticker went while WHO's reply was awaited, at {s}");
                    sent.push(s);
                    who.partial(Some(PROMPT), now);
                }
                if reply_at == Some(s) {
                    reply_at = None;
                    for line in [PROMPT, "[Race         Class        Level  ] Character name", "[Human        Fighter      5      ] Ann", ""] {
                        who.line(line, now);
                    }
                    who.partial(Some(PROMPT), now);
                }
                if who.due(now) {
                    whos += 1;
                    // One WHO in five goes unanswered.
                    reply_at = (whos % 5 != 0).then_some(s + 1);
                }
            }
            assert!(whos > 100, "every {every}: WHO was asked {whos} times");
            let late = u64::from(every) + 12;
            assert!(sent[0] <= late, "every {every}: first sent at {}", sent[0]);
            for pair in sent.windows(2) {
                let gap = pair[1] - pair[0];
                assert!(gap >= u64::from(every.max(MIN_EVERY)), "every {every}: sent {gap} s apart at {}", pair[1]);
                assert!(gap <= late, "every {every}: {gap} s between sends at {}", pair[1]);
            }
        }
    }

    #[test]
    fn suggestions() {
        let names = vec!["Ann".to_string()];
        let s: Vec<String> = suggest("Ann says 'I have 12 gold'", &names).into_iter().map(|s| s.pattern).collect();
        assert_eq!(s[0], "Ann says 'I have 12 gold'");
        assert!(s.contains(&"Ann says 'I have * gold'".to_string()));
        assert!(s.contains(&"* says 'I have * gold'".to_string()));
        assert!(s.contains(&"* says '*'".to_string()));
        assert!(s.contains(&"Ann says 'I *".to_string()));
        // A name inside a word isn't the name.
        assert_eq!(names_starred("Annie waves.", &names).1, None);
        assert!(suggest("Hi", &[]).iter().all(|s| !Pattern::new(&s.pattern).everything()));
    }

    #[test]
    fn filling() {
        assert_eq!(fill("%1 and %2, %* (%0) 100%% %x", "whole", &["a".into(), "b".into()]), "a and b, a b (whole) 100% %x");
        assert!(takes_words("kill %1"));
        assert!(!takes_words("get all %0"));
    }
}

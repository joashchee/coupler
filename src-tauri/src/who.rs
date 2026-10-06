//! **Who's online**: the players in the game now, kept from WHO, which
//! Coupler asks quietly once a minute, and from the game's announcements
//! of a character logging on or off. Pure and unit-tested; `session.rs`
//! (and the web build's) feeds it every finished line, the unfinished one
//! after each read, what the player types and whether the server echoes,
//! and sends WHO when `due` says so.
//!
//! CoffeeMUD has no GMCP for it: `Comm.Channel.Players` builds the list
//! and throws it away (docs/coffeemud-gmcp.md), and MSSP's `PLAYERS` is
//! only a count. So it's read from the text:
//!
//! - **WHO** (Commands/Who.java, `getWho`): a header
//!   `[Race         Class        Level  ] Character name`, then a row per
//!   character, `[Human        Fighter      5      ] Bob, the Brave`, the
//!   bracket padded to the header's width, the name titled (`titles.ini`:
//!   `*, the Charmer` or `Legendary Crafter *`), a cloaked one in
//!   brackets, then ` (idle: 5m)` for someone away and `[LFG]` for one
//!   looking for a group. No total line. The rows end at the first line
//!   that isn't one. WHO counts characters the player can see, so not
//!   cloaked ones, and includes the player.
//! - **Logging on and off** (CharCreation.java and DefaultSession.java):
//!   to friends who have AUTONOTIFY, `Bob has logged on.` and
//!   `Bob has logged off.`; on any channel with the LOGINS or LOGOFFS
//!   flag (stock: CLANTALK and WIZINFO), `Bob has logged on.` and
//!   `Bob has logged out` as a system message:
//!   `[CLANTALK] 'Bob has logged on.'`, after the clan's name, and as
//!   `comm.channel`'s `msg` too. A friends' notice and a channel's can
//!   both come for one login: it counts once.
//!
//! **What Coupler asks shows nowhere**: the reply to its own WHO, the
//! blank lines around it and the prompt line it finishes are taken out
//! of the game output (a WHO the player types shows as always), and so is
//! every announcement: each is a sound instead (`lib/earcons.ts`).
//!
//! **When it asks**, gently, since it's typing for the player on a
//! shared server:
//!
//! - only with a character in the game (`char.base` named them), not
//!   while the server echoes (a password), and only at the player's usual
//!   prompt (the unfinished line seen most, numbers aside), so never into
//!   a question, a menu or an editor that would take WHO as its answer;
//! - not within `QUIET_AFTER_TYPING` of a command, so its reply isn't
//!   mixed with the player's;
//! - **not once the player has been idle `IDLE_AFTER`**. Every line sent
//!   resets the game's idle clock (DefaultSession's `lastKeystroke`): it
//!   marks a player away after 10 minutes and logs them out after 90
//!   (`IDLETIMERS`). Asking for an away player would keep them "here"
//!   forever. The announcements still keep the list until they type.

use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;

use crate::clock::Instant;
use crate::senses::letters_and_digits;

/// How often WHO is asked while the player is about.
pub const ASK_EVERY: Duration = Duration::from_secs(60);
/// The first WHO after a character comes into the game.
const FIRST_ASK: Duration = Duration::from_secs(5);
/// An asked WHO whose reply hasn't come by now is given up on, so
/// nothing more is hidden for it.
const ANSWER_WAITS: Duration = Duration::from_secs(10);
/// No WHO this soon after the player sends a command.
const QUIET_AFTER_TYPING: Duration = Duration::from_secs(2);
/// No WHO once the player hasn't typed for this long: the game marks
/// them away at 10 minutes.
const IDLE_AFTER: Duration = Duration::from_secs(9 * 60);
/// The usual prompt has been seen at least this many times.
const USUAL_PROMPT: u32 = 3;
/// The unfinished lines remembered, for telling the usual prompt.
const PROMPTS_KEPT: usize = 64;
/// The changes remembered, for the report's last one.
const CHANGES_KEPT: usize = 16;

/// Someone online, as WHO showed them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Person {
    /// The name alone (`Bob`), for saying.
    pub name: String,
    /// As WHO shows it, title and all (`Bob, the Brave`).
    pub shown: String,
    /// Away: WHO shows how long they've been idle.
    pub idle: bool,
}

/// Someone logged on or off.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    pub name: String,
    pub on: bool,
}

/// The last change, for the report.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastChange {
    pub name: String,
    pub on: bool,
    pub seconds_ago: u64,
}

/// Who's online, for saying.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// A whole WHO has been read since the character came in.
    pub known: bool,
    /// Everyone else online, in WHO's order.
    pub online: Vec<Person>,
    pub last: Option<LastChange>,
}

/// Whether a finished line shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    Show,
    Hide,
}

/// A WHO reply being read.
struct Listing {
    rows: Vec<Person>,
    /// The bracket's width in characters, `[` to `]`, from the header.
    width: usize,
    /// Coupler asked for it: it doesn't show.
    hidden: bool,
    /// The whole list (a bare WHO), not one narrowed by a name or FRIENDS.
    full: bool,
    started: Instant,
}

#[derive(Default)]
pub struct Who {
    /// The player's character, letters and digits, lowercased; empty
    /// until `char.base` names them.
    me: String,
    /// Everyone else online, in WHO's order.
    online: Vec<Person>,
    known: bool,
    listing: Option<Listing>,
    /// When Coupler sent WHO, until its reply comes.
    asked: Option<Instant>,
    /// The player's last command was a bare WHO.
    player_asked: bool,
    /// How often each unfinished line was seen, numbers masked.
    prompts: HashMap<String, u32>,
    /// The unfinished line after the last read.
    prompt: Option<String>,
    typed_at: Option<Instant>,
    next_ask: Option<Instant>,
    /// The server echoes: what's typed is a password.
    secret: bool,
    /// A character's being made (`creation.rs`): WHO would be taken for an
    /// answer, and the creation's prompts aren't the usual one.
    creating: bool,
    /// Not yet taken by `take_changes`.
    changes: Vec<Change>,
    /// The latest changes, newest last, with when.
    recent: Vec<(Change, Instant)>,
    /// A character has come into the game this connection.
    played: bool,
}

/// Spaces collapsed, ends trimmed.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Digits masked, so a prompt is the same prompt whatever its numbers.
fn mask(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !out.ends_with('#') {
                out.push('#');
            }
        } else {
            out.push(c);
        }
    }
    squeeze(&out)
}

/// WHO's header: the bracket's width when `text` is one.
fn header(text: &str) -> Option<usize> {
    if !squeeze(text).ends_with("] Character name") {
        return None;
    }
    // From the bracket's own `[`: the prompt before it may have one too.
    let chars: Vec<char> = text.chars().collect();
    let close = chars.iter().rposition(|&c| c == ']')?;
    let open = chars[..close].iter().rposition(|&c| c == '[')?;
    Some(close - open)
}

/// A row of a WHO whose bracket is `width` wide.
fn row(text: &str, width: usize) -> Option<Person> {
    let chars: Vec<char> = text.trim_end().chars().collect();
    if chars.first() != Some(&'[') || chars.get(width) != Some(&']') {
        return None;
    }
    let mut shown = squeeze(&chars[width + 1..].iter().collect::<String>());
    if let Some(rest) = shown.strip_suffix("[LFG]") {
        shown = rest.trim_end().to_string();
    }
    let idle = shown.ends_with(')') && shown.contains(" (idle: ");
    if idle {
        shown.truncate(shown.rfind(" (idle: ").expect("checked"));
    }
    // A cloaked character, shown to those who may see them.
    if let Some(inner) = shown.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        shown = inner.to_string();
    }
    let name = bare_name(&shown)?;
    Some(Person { name, shown, idle })
}

/// Words that are a title's, never a name: those of CoffeeMUD's own
/// titles (`resources/achievements.ini`, `TITLE=`: "Prodigal Fighter *",
/// "Chef de Partie *", "Dread Admiral *"), its classes, and the ranks a
/// game's own titles use.
const TITLE_WORDS: &[&str] = &[
    "prodigal", "master", "legendary", "crafter", "scalp", "headhunter", "grand", "father", "mother", "patriarch", "matriarch", "apprenti",
    "cuisinier", "partie", "chef", "sous-chef", "cuisine", "builder", "architect", "chief", "crossbreeder", "hybridizer", "dread", "captain",
    "commander", "admiral", "lord", "lady", "sir", "dame", "king", "queen", "prince", "princess", "duke", "duchess", "baron", "baroness",
    "count", "countess", "earl", "knight", "squire", "elder", "saint", "brother", "sister", "high", "great", "young", "old", "abjurer",
    "alterer", "apprentice", "arcanist", "artisan", "assassin", "barbarian", "bard", "beastmaster", "burglar", "cavalier", "charlatan",
    "cleric", "conjurer", "delver", "diviner", "doomsayer", "druid", "enchanter", "evoker", "fighter", "gaian", "gaoler", "gypsy", "healer",
    "illusionist", "jester", "mage", "mer", "minstrel", "missionary", "monk", "necromancer", "oracle", "paladin", "pirate", "prancer",
    "purist", "ranger", "reliquist", "sailor", "scholar", "shaman", "skywatcher", "templar", "thief", "transmuter", "trapper", "wizard",
];

/// The name in a titled one, for saying who's online without the titles.
/// A title is `*` with words around it (`MOB.titledName`): before a comma
/// the name ends it (`Bob, the Brave`, `Prodigal Mage Bob, the Wise`);
/// otherwise it's the one capitalized word no title has (`Legendary
/// Crafter Bob`, `Bob the Brave`, `Lord Bob of Midgaard`), and of two,
/// the one before a small word like "the" or "of", else the last.
fn bare_name(titled: &str) -> Option<String> {
    let clean = |w: &str| w.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect::<String>();
    if let Some((before, _)) = titled.split_once(',') {
        let name = clean(before.split_whitespace().last()?).replace('-', "");
        return (!name.is_empty()).then_some(name);
    }
    let words: Vec<String> = titled.split_whitespace().map(clean).filter(|w| !w.is_empty()).collect();
    let capital = |w: &str| w.chars().next().is_some_and(char::is_uppercase);
    let names: Vec<usize> = (0..words.len()).filter(|&i| capital(&words[i]) && !TITLE_WORDS.contains(&words[i].to_lowercase().as_str())).collect();
    let small = words.iter().position(|w| !capital(w));
    let at = match (names.as_slice(), small) {
        ([], _) => words.len().checked_sub(1)?,
        ([one], _) => *one,
        (many, Some(small)) => many.iter().rev().find(|&&i| i < small).copied().unwrap_or(many[many.len() - 1]),
        (many, None) => many[many.len() - 1],
    };
    let name = words[at].replace('-', "");
    (!name.is_empty()).then_some(name)
}

/// An announcement of someone logging on (true) or off: the name and which.
pub fn announcement(text: &str) -> Option<(String, bool)> {
    let text = squeeze(text);
    // A channel's: `[CLANTALK] 'Bob has logged on.'`, maybe after the clan's name.
    let said = match text.strip_suffix('\'') {
        Some(quoted) => &quoted[quoted.rfind("] '")? + 3..],
        None => text.as_str(),
    };
    let (name, rest) = said.trim_end_matches('.').split_once(' ')?;
    let on = match rest {
        "has logged on" => true,
        "has logged off" | "has logged out" => false,
        _ => return None,
    };
    if name.is_empty() || !name.chars().all(char::is_alphanumeric) {
        return None;
    }
    Some((name.to_string(), on))
}

impl Who {
    /// After each read: the character playing (`senses`' name, empty
    /// before `char.base`). One coming in is asked about soon.
    pub fn character(&mut self, me: &str, now: Instant) {
        if me == self.me {
            return;
        }
        if self.me.is_empty() {
            self.next_ask = Some(now + FIRST_ASK);
            // The login's questions (the account menu's, seen many times)
            // aren't the game's prompt: counted from the first character,
            // the usual one is known after a few prompts, and WHO starts
            // soon after. A later character keeps it.
            if !self.played {
                self.prompts.clear();
            }
            self.played = true;
        }
        self.me = me.to_string();
    }

    /// A line the player sent.
    pub fn typed(&mut self, line: &str, now: Instant) {
        self.typed_at = Some(now);
        self.player_asked = matches!(line.trim().to_ascii_lowercase().as_str(), "who" | "wh");
    }

    /// Whether the server echoes (so a password is being typed).
    pub fn echo(&mut self, server_echoes: bool) {
        self.secret = server_echoes;
    }

    /// Whether a character's being made: never WHO then, and its questions
    /// aren't counted toward the usual prompt.
    pub fn creating(&mut self, creating: bool) {
        self.creating = creating;
    }

    /// Whether to send WHO now. When it says yes, it counts it as sent.
    pub fn due(&mut self, now: Instant) -> bool {
        if self.listing.as_ref().is_some_and(|l| now.saturating_duration_since(l.started) >= ANSWER_WAITS) {
            self.finish(now);
        }
        if let Some(at) = self.asked {
            if now.saturating_duration_since(at) < ANSWER_WAITS {
                return false;
            }
            // No reply: nothing more is hidden for it.
            self.asked = None;
        }
        if !self.gentle(now) || self.next_ask.is_none_or(|next| now < next) {
            return false;
        }
        self.asked = Some(now);
        self.next_ask = Some(now + ASK_EVERY);
        true
    }

    /// The player asked who's online before a WHO was read (the say
    /// key): whether to send WHO now, counted as Coupler's own (its reply
    /// hidden, then said). Only with a character in the game, no password
    /// or character being made, and no WHO already waiting; not at the
    /// usual prompt only, since the player asked this moment.
    pub fn ask_now(&mut self, now: Instant) -> bool {
        if self.me.is_empty() || self.secret || self.creating || self.listing.is_some() || self.asked.is_some_and(|at| now.saturating_duration_since(at) < ANSWER_WAITS) {
            return false;
        }
        self.asked = Some(now);
        self.next_ask = Some(now + ASK_EVERY);
        true
    }

    /// Whether a command Coupler sends on its own (WHO, or WEATHER for the
    /// painter's sky, `ambient.rs`) would go unnoticed now: a character
    /// in the game, at the usual prompt, no password being typed, nothing
    /// being made, no WHO waiting, the player not typing this moment and
    /// not away 9 minutes (it would defeat the game's idle timers).
    pub fn gentle(&self, now: Instant) -> bool {
        if self.me.is_empty() || self.secret || self.creating || self.listing.is_some() || self.asked.is_some() || !self.at_usual_prompt() {
            return false;
        }
        let Some(typed) = self.typed_at else { return false };
        let since_typed = now.saturating_duration_since(typed);
        (QUIET_AFTER_TYPING..IDLE_AFTER).contains(&since_typed)
    }

    /// The unfinished line is the one seen most, and often enough.
    fn at_usual_prompt(&self) -> bool {
        let Some(prompt) = &self.prompt else { return false };
        let Some(&count) = self.prompts.get(&mask(prompt)) else { return false };
        count >= USUAL_PROMPT && self.prompts.values().all(|&n| n <= count)
    }

    /// A finished line of the game's text: whether it shows.
    pub fn line(&mut self, text: &str, now: Instant) -> Seen {
        if let Some(width) = header(text) {
            if self.listing.is_some() {
                self.finish(now);
            }
            let hidden = self.asked.take().is_some();
            let full = hidden || std::mem::take(&mut self.player_asked);
            self.listing = Some(Listing { rows: Vec::new(), width, hidden, full, started: now });
            return if hidden { Seen::Hide } else { Seen::Show };
        }
        let squeezed = squeeze(text);
        if let Some(listing) = self.listing.as_mut() {
            if let Some(person) = row(text, listing.width) {
                listing.rows.push(person);
                return if listing.hidden { Seen::Hide } else { Seen::Show };
            }
            let hidden = listing.hidden;
            if squeezed.is_empty() && hidden {
                return Seen::Hide;
            }
            self.finish(now);
        }
        if self.asked.is_some() {
            // The prompt WHO's reply finishes, and the blank lines before it.
            if squeezed.is_empty() {
                return Seen::Hide;
            }
            if self.prompt.as_deref().is_some_and(|p| squeeze(p) == squeezed) {
                self.prompt = None;
                return Seen::Hide;
            }
        }
        // An announcement may follow the prompt on its line.
        let said = match self.prompt.as_deref().map(squeeze) {
            Some(p) if !p.is_empty() && squeezed.starts_with(&p) => squeezed[p.len()..].trim_start(),
            _ => squeezed.as_str(),
        };
        match announcement(said) {
            Some((name, on)) => {
                self.change(&name, on, now);
                Seen::Hide
            }
            None => Seen::Show,
        }
    }

    /// `comm.channel`'s message: an announcement is taken (the line that
    /// follows is hidden by `line`), and isn't talk.
    pub fn heard(&mut self, msg: &str, now: Instant) -> bool {
        match announcement(msg) {
            Some((name, on)) => {
                self.change(&name, on, now);
                true
            }
            None => false,
        }
    }

    /// The unfinished line after a read. Returns whether to keep showing
    /// the one before instead: a piece of a hidden WHO cut off by the read.
    pub fn partial(&mut self, text: Option<&str>, now: Instant) -> bool {
        let hiding = self.asked.is_some() || self.listing.as_ref().is_some_and(|l| l.hidden);
        if hiding && text.is_some_and(|t| t.starts_with('[')) {
            return true;
        }
        // The rows are over once something else is waiting, the prompt.
        if text.is_some() && self.listing.is_some() {
            self.finish(now);
        }
        if let Some(text) = text.filter(|_| !self.creating) {
            if self.prompts.len() >= PROMPTS_KEPT {
                self.prompts.retain(|_, n| *n >= USUAL_PROMPT);
            }
            *self.prompts.entry(mask(text)).or_default() += 1;
        }
        self.prompt = text.map(str::to_string);
        false
    }

    /// A WHO reply is over: a whole one is the new list, and whoever came
    /// or went since the last is a change.
    fn finish(&mut self, now: Instant) {
        let Some(listing) = self.listing.take() else { return };
        if !listing.full {
            return;
        }
        let me = self.me.clone();
        let mine = |p: &Person| letters_and_digits(&p.name) == me;
        let online: Vec<Person> = listing.rows.into_iter().filter(|p| !mine(p)).collect();
        if self.known {
            let key = |p: &Person| letters_and_digits(&p.name);
            let was: Vec<String> = self.online.iter().map(key).collect();
            let is: Vec<String> = online.iter().map(key).collect();
            for p in &online {
                if !was.contains(&key(p)) {
                    self.note(&p.name, true, now);
                }
            }
            for p in &self.online.clone() {
                if !is.contains(&key(p)) {
                    self.note(&p.name, false, now);
                }
            }
        }
        self.online = online;
        self.known = true;
        self.next_ask = Some(now + ASK_EVERY);
    }

    /// Someone announced logging on or off: the list follows.
    fn change(&mut self, name: &str, on: bool, now: Instant) {
        let key = letters_and_digits(name);
        if key == self.me {
            return;
        }
        let here = self.online.iter().position(|p| letters_and_digits(&p.name) == key);
        match (on, here) {
            // Both a friends' notice and a channel's: already counted.
            (true, Some(_)) => return,
            (true, None) => self.online.push(Person { name: name.to_string(), shown: name.to_string(), idle: false }),
            (false, Some(i)) => {
                self.online.remove(i);
            }
            (false, None) if self.known => return,
            (false, None) => {}
        }
        self.note(name, on, now);
    }

    /// A change to tell, unless it was just told.
    fn note(&mut self, name: &str, on: bool, now: Instant) {
        let key = letters_and_digits(name);
        let told = self.recent.iter().rev().find(|(c, _)| letters_and_digits(&c.name) == key);
        if told.is_some_and(|(c, at)| c.on == on && now.saturating_duration_since(*at) < ANSWER_WAITS) {
            return;
        }
        let change = Change { name: name.to_string(), on };
        if self.recent.len() == CHANGES_KEPT {
            self.recent.remove(0);
        }
        self.recent.push((change.clone(), now));
        self.changes.push(change);
    }

    /// The changes since last asked.
    pub fn take_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes)
    }

    /// Who's online now.
    pub fn report(&self, now: Instant) -> Report {
        Report {
            known: self.known,
            online: self.online.clone(),
            last: self.recent.last().map(|(c, at)| LastChange { name: c.name.clone(), on: c.on, seconds_ago: now.saturating_duration_since(*at).as_secs() }),
        }
    }

    /// The player left their character (a logout, a switch, hanging
    /// up): nothing is known. When they last typed and their usual
    /// prompt are still theirs.
    pub fn leave(&mut self) {
        *self = Who { prompts: std::mem::take(&mut self.prompts), typed_at: self.typed_at, played: self.played, ..Who::default() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "<100hp 50m 80mv> ";
    const HEADER: &str = "[Race         Class        Level  ] Character name";

    fn row_line(race: &str, class: &str, level: &str, name: &str) -> String {
        format!("[{race:<12} {class:<12} {level:<7}] {name}")
    }

    /// A player in the game at their prompt, last typing at `t`.
    fn playing(t: Instant) -> Who {
        let mut who = Who::default();
        who.character("bob", t);
        who.typed("look", t);
        for _ in 0..USUAL_PROMPT {
            who.partial(Some(PROMPT), t);
        }
        who
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// Coupler's WHO and its reply; returns what showed.
    fn ask(who: &mut Who, t: Instant, rows: &[String]) -> Vec<String> {
        assert!(who.due(t), "WHO should be due");
        let mut shown = Vec::new();
        let mut lines = vec![PROMPT.to_string(), HEADER.to_string()];
        lines.extend(rows.iter().cloned());
        lines.push(String::new());
        for l in lines {
            if who.line(&l, t) == Seen::Show {
                shown.push(l);
            }
        }
        who.partial(Some(PROMPT), t);
        shown
    }

    #[test]
    fn the_say_key_asks_before_the_usual_prompt_is_known() {
        let t = Instant::now();
        let mut who = Who::default();
        // Not before a character's in the game.
        assert!(!who.ask_now(t));
        who.character("bob", t);
        who.partial(Some(PROMPT), t);
        assert!(!who.due(t + secs(10)), "the usual prompt isn't known yet");
        assert!(who.ask_now(t));
        // Asked: a second press doesn't send it twice.
        assert!(!who.ask_now(t + secs(1)));
        let mut shown = Vec::new();
        for l in [PROMPT.to_string(), HEADER.to_string(), row_line("Elf", "Bard", "12", "Carl"), String::new()] {
            if who.line(&l, t) == Seen::Show {
                shown.push(l);
            }
        }
        who.partial(Some(PROMPT), t);
        assert!(shown.is_empty(), "{shown:?}");
        let report = who.report(t);
        assert!(report.known);
        assert_eq!(report.online[0].name, "Carl");
    }

    #[test]
    fn the_account_menu_doesnt_outnumber_the_game_prompt() {
        let t = Instant::now();
        let mut who = Who::default();
        for _ in 0..10 {
            who.partial(Some("Command or Name ('?' for help): "), t);
        }
        who.character("bob", t);
        who.typed("look", t);
        for _ in 0..USUAL_PROMPT {
            who.partial(Some(PROMPT), t);
        }
        assert!(who.due(t + FIRST_ASK + secs(1)));
    }

    #[test]
    fn reads_a_row() {
        let width = header(HEADER).unwrap();
        let p = row(&row_line("Human", "Fighter", "5", "Alice, the Brave (idle: 5m)"), width).unwrap();
        assert_eq!(p, Person { name: "Alice".into(), shown: "Alice, the Brave".into(), idle: true });
        let p = row(&format!("{}[LFG]", row_line("Elf", "Bard", "12", "Legendary Crafter Carl")), width).unwrap();
        assert_eq!((p.name.as_str(), p.idle), ("Carl", false));
        let p = row(&row_line("Dwarf", "Thief", "3", "(Dee)"), width).unwrap();
        assert_eq!(p.name, "Dee");
        // A channel line isn't a row: its bracket isn't the header's width.
        assert!(row("[GOSSIP] Erin: hi all", width).is_none());
    }

    #[test]
    fn a_name_without_its_title() {
        for (titled, name) in [
            ("Bob", "Bob"),
            ("Bob, the Brave", "Bob"),
            ("Bob, Master Fighter", "Bob"),
            ("Legendary Crafter Bob", "Bob"),
            ("Prodigal Mage Bob", "Bob"),
            ("Sous-Chef de Cuisine Bob", "Bob"),
            ("Chef de Partie Bob", "Bob"),
            ("Bob the Brave", "Bob"),
            ("Lord Bob of Midgaard", "Bob"),
            ("Dread Admiral Bob", "Bob"),
            ("Sir Bob the Bold", "Bob"),
        ] {
            assert_eq!(bare_name(titled).as_deref(), Some(name), "{titled}");
        }
    }

    #[test]
    fn a_header_after_a_bracketed_prompt() {
        assert_eq!(header(&format!("[100hp] {HEADER}")), header(HEADER));
    }

    #[test]
    fn reads_announcements() {
        assert_eq!(announcement("Alice has logged on."), Some(("Alice".into(), true)));
        assert_eq!(announcement("Alice has logged off."), Some(("Alice".into(), false)));
        assert_eq!(announcement("The Knights [CLANTALK] 'Alice has logged on.'"), Some(("Alice".into(), true)));
        assert_eq!(announcement("[WIZINFO] 'Alice has logged out'"), Some(("Alice".into(), false)));
        assert_eq!(announcement("Account alice has logged on."), None);
        assert_eq!(announcement("Erin says 'Alice has logged on.'"), None);
        assert_eq!(announcement("Alice has logged on to the forum."), None);
    }

    #[test]
    fn asks_only_when_gentle() {
        let t = Instant::now();
        let mut who = playing(t);
        // Not before the first ask's wait.
        assert!(!who.due(t + secs(1)));
        assert!(who.due(t + secs(5)));
        // Waiting for the reply, then a minute.
        assert!(!who.due(t + secs(6)));

        // Not just after the player typed.
        let mut who = playing(t);
        who.typed("kill orc", t + secs(5));
        assert!(!who.due(t + secs(6)));
        assert!(who.due(t + secs(8)));

        // Not into a question.
        let mut who = playing(t);
        who.partial(Some("Logout -- are you sure (y/N)?"), t);
        who.partial(Some("Logout -- are you sure (y/N)?"), t);
        assert!(!who.due(t + secs(10)));

        // Not while a password is typed.
        let mut who = playing(t);
        who.echo(true);
        assert!(!who.due(t + secs(10)));

        // Not for someone who's been away 9 minutes; again once they type.
        let mut who = playing(t);
        assert!(!who.due(t + IDLE_AFTER));
        who.typed("look", t + IDLE_AFTER);
        assert!(who.due(t + IDLE_AFTER + secs(3)));

        // Never while a character's being made: the stats prompt comes back
        // so often it'd pass for the usual one, and WHO would be an answer.
        let mut who = playing(t);
        who.creating(true);
        for _ in 0..20 {
            who.partial(Some("Enter a stat to add or subtract (or R)"), t);
        }
        assert!(!who.due(t + secs(10)));
        who.creating(false);
        who.partial(Some(PROMPT), t);
        assert!(who.due(t + secs(10)), "the creation's prompts didn't count");

        // Not before a character is in the game.
        let mut who = Who::default();
        who.typed("bob", t);
        for _ in 0..5 {
            who.partial(Some("Password: "), t);
        }
        assert!(!who.due(t + secs(60)));
    }

    #[test]
    fn hides_its_own_who_and_keeps_the_list() {
        let t = Instant::now();
        let mut who = playing(t);
        let rows = [row_line("Human", "Fighter", "5", "Bob"), row_line("Elf", "Mage", "9", "Alice, the Wise")];
        let shown = ask(&mut who, t + secs(5), &rows);
        assert!(shown.is_empty(), "showed {shown:?}");
        let report = who.report(t + secs(5));
        assert!(report.known);
        // The player isn't in their own list.
        assert_eq!(report.online.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Alice"]);
        // The first list is no news.
        assert!(who.take_changes().is_empty());

        // A minute on, Carl came and Alice went.
        let rows = [row_line("Human", "Fighter", "5", "Bob"), row_line("Gnome", "Thief", "2", "Carl")];
        let shown = ask(&mut who, t + secs(65), &rows);
        assert!(shown.is_empty());
        assert_eq!(who.take_changes(), [Change { name: "Carl".into(), on: true }, Change { name: "Alice".into(), on: false }]);
    }

    #[test]
    fn shows_the_players_who() {
        let t = Instant::now();
        let mut who = playing(t);
        who.typed("who", t);
        assert_eq!(who.line(HEADER, t), Seen::Show);
        assert_eq!(who.line(&row_line("Elf", "Mage", "9", "Alice"), t), Seen::Show);
        who.partial(Some(PROMPT), t);
        assert!(who.report(t).known);

        // WHO narrowed to a name or FRIENDS isn't the whole list.
        let mut who = playing(t);
        who.typed("who friends", t);
        who.line(HEADER, t);
        who.line(&row_line("Elf", "Mage", "9", "Alice"), t);
        who.partial(Some(PROMPT), t);
        assert!(!who.report(t).known);
    }

    #[test]
    fn a_reply_split_across_reads() {
        let t = Instant::now();
        let mut who = playing(t);
        assert!(who.due(t + secs(5)));
        assert_eq!(who.line(HEADER, t), Seen::Hide);
        // Cut off mid-row: the old prompt stays, and the rows go on.
        assert!(who.partial(Some("[Elf          Ma"), t));
        assert_eq!(who.line(&row_line("Elf", "Mage", "9", "Alice"), t), Seen::Hide);
        assert!(!who.partial(Some(PROMPT), t));
        assert_eq!(who.report(t).online.len(), 1);
        // Then the game's own lines show again.
        assert_eq!(who.line("You are hungry.", t), Seen::Show);
    }

    #[test]
    fn an_unanswered_who_stops_hiding() {
        let t = Instant::now();
        let mut who = playing(t);
        assert!(who.due(t + secs(5)));
        assert!(!who.due(t + secs(14)));
        who.due(t + secs(16));
        assert_eq!(who.line("", t + secs(16)), Seen::Show);
    }

    #[test]
    fn announcements_hide_and_count_once() {
        let t = Instant::now();
        let mut who = playing(t);
        ask(&mut who, t + secs(5), &[row_line("Human", "Fighter", "5", "Bob")]);
        // The channel's GMCP, then its line, then a friends' notice.
        assert!(who.heard("The Knights [CLANTALK] 'Alice has logged on.'", t + secs(10)));
        assert_eq!(who.line("The Knights [CLANTALK] 'Alice has logged on.'", t + secs(10)), Seen::Hide);
        assert_eq!(who.line("Alice has logged on.", t + secs(10)), Seen::Hide);
        assert_eq!(who.take_changes(), [Change { name: "Alice".into(), on: true }]);
        // After the prompt on the same line.
        assert_eq!(who.line(&format!("{PROMPT}Alice has logged off."), t + secs(20)), Seen::Hide);
        assert_eq!(who.line("[CLANTALK] 'Alice has logged out'", t + secs(20)), Seen::Hide);
        assert_eq!(who.take_changes(), [Change { name: "Alice".into(), on: false }]);
        let report = who.report(t + secs(80));
        assert!(report.online.is_empty());
        assert_eq!(report.last, Some(LastChange { name: "Alice".into(), on: false, seconds_ago: 60 }));
        // The player's own login, told to their clan, is no news.
        assert_eq!(who.line("[CLANTALK] 'Bob has logged on.'", t), Seen::Hide);
        assert!(who.take_changes().is_empty());
    }

    #[test]
    fn leaving_forgets_who_but_not_the_prompt() {
        let t = Instant::now();
        let mut who = playing(t);
        ask(&mut who, t + secs(5), &[row_line("Elf", "Mage", "9", "Alice")]);
        who.leave();
        assert!(!who.report(t).known);
        who.character("bob", t + secs(10));
        who.partial(Some(PROMPT), t + secs(10));
        assert!(who.due(t + secs(15)));
    }
}

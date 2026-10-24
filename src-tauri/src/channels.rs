//! **The game's channels, on and off**, pure and unit-tested, shared with
//! the web build: the Log's Mute Channel and its Channels dialog turn
//! them with CoffeeMUD's own commands, so it's the game that stops
//! sending a channel, for this character, in any client.
//!
//! - `CHANNELS` lists the channels the character may use, three a row,
//!   each padded to 24 columns, an off one followed by ` (OFF)`, between
//!   `Available channels:` and `Use NOCHANNELNAME (ex: NOGOSSIP) to turn
//!   a channel off.`; with none, `None, because you turned them all off.
//!   …` or `None, because you have QUIET mode on.` (`Commands/Channels.java`).
//! - `NO<NAME>` turns one off (`Commands/NoChannel.java`): `The NAME
//!   channel has been turned off.  Use `NAME` to turn it back on.`, or
//!   `… is already off.`
//! - `<NAME> ON` turns it back on (`Commands/Channel.java`: an off
//!   channel's own command turns it on, saying nothing on it; `ON` on
//!   one that's on only says so): `NAME has been turned on.  Use `NONAME`
//!   to turn it off again.`, or `NAME is already turned on.` Either may
//!   instead be `This channel is not available to you.` or, turning one
//!   on, `You have QUIET mode on.  You must turn it off first.`
//! - `CHANNELS ON` turns every one on and lists them; `CHANNELS OFF` turns
//!   them all off.
//!
//! The wiki (GOSSIP, read 2026-10-24) says the same: `NOGOSSIP` silences
//! the channel and GOSSIP turns it on again.
//!
//! Each command Coupler sends here is followed by `CHANNELS`, so the list
//! is always the game's own. Their answers aren't shown (`line`), but a
//! refusal is told (`Listing::refused`).

use std::time::Duration;

use serde::Serialize;

use crate::clock::Instant;

/// How long an answer is waited for before nothing more is hidden for it.
const ANSWER_WAITS: Duration = Duration::from_secs(10);

const LIST_STARTS: &str = "Available channels:";
const LIST_ENDS: &str = "Use NOCHANNELNAME";
const NONE: &str = "None, because";
/// What `CHANNELS ON` and `CHANNELS OFF` answer before (or instead of) the list.
const ALL_TURNED: &[&str] = &["Channels were already turned on, so removed all filters.", "Channels are now turned back on.", "Channels are now all turned off.", "Channels were already turned off."];
/// A turn the game refused.
const REFUSALS: &[&str] = &["This channel is not available to you.", "You have QUIET mode on.  You must turn it off first."];

/// A channel and whether it's on for this character.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub name: String,
    pub on: bool,
}

/// What `CHANNELS` answered.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub channels: Vec<Channel>,
    /// Why none are listed, in the game's words: all turned off, or QUIET.
    pub none: Option<String>,
    /// The game's words when it refused a turn since the last list.
    pub refused: Option<String>,
}

#[derive(Default)]
pub struct Channels {
    /// Coupler's own commands sent, their answers awaited, since when.
    asked: Option<Instant>,
    /// How many turns' answers are still to come.
    turns: usize,
    /// Within the list, its rows so far.
    reading: Option<Vec<Channel>>,
    refused: Option<String>,
    /// The list last read, once whole.
    done: Option<Listing>,
}

/// Spaces collapsed, ends trimmed.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One row of the list: names, each maybe followed by `(OFF)`.
fn row(text: &str) -> Vec<Channel> {
    let mut channels: Vec<Channel> = Vec::new();
    for word in text.split_whitespace() {
        if word.eq_ignore_ascii_case("(OFF)") {
            if let Some(last) = channels.last_mut() {
                last.on = false;
            }
        } else {
            channels.push(Channel { name: word.to_string(), on: true });
        }
    }
    channels
}

/// Whether `text` answers a turn of one channel.
fn turned(text: &str) -> bool {
    let said = squeeze(text);
    let one = |prefix: &str, suffix: &str| said.strip_prefix(prefix).is_some_and(|rest| rest.contains(suffix));
    one("The ", " channel has been turned off.")
        || one("The ", " channel is already off.")
        || said.contains(" has been turned on. Use `NO")
        || said.ends_with(" is already turned on.")
}

impl Channels {
    /// The connection ended: nothing's awaited.
    pub fn closed(&mut self) {
        *self = Channels { done: self.done.take(), ..Channels::default() };
    }

    /// What to send to read the list.
    pub fn ask(&mut self, now: Instant) -> Vec<String> {
        self.asked = Some(now);
        vec!["channels".into()]
    }

    /// What to send to turn `name` on or off, then read the list again.
    pub fn turn(&mut self, name: &str, on: bool, now: Instant) -> Vec<String> {
        let name = name.trim().to_ascii_uppercase();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Vec::new();
        }
        self.turns += 1;
        let turn = if on { format!("{name} on") } else { format!("no{name}") };
        let mut out = vec![turn];
        out.extend(self.ask(now));
        out
    }

    /// What to send to turn every channel on or off, then read the list.
    pub fn all(&mut self, on: bool, now: Instant) -> Vec<String> {
        let mut out = vec![if on { "channels on" } else { "channels off" }.to_string()];
        out.extend(self.ask(now));
        out
    }

    /// Whether Coupler's own commands here are awaiting their answers
    /// (never past `ANSWER_WAITS`): WHO and the tickers wait meanwhile.
    pub fn awaiting(&self, now: Instant) -> bool {
        self.asked.is_some_and(|at| now.saturating_duration_since(at) < ANSWER_WAITS)
    }

    /// One finished line: whether it's an answer to hide.
    pub fn line(&mut self, text: &str, now: Instant) -> bool {
        if self.asked.is_some_and(|at| now.saturating_duration_since(at) >= ANSWER_WAITS) {
            self.asked = None;
            self.turns = 0;
            self.reading = None;
        }
        if self.asked.is_none() {
            return false;
        }
        let said = squeeze(text);
        if let Some(rows) = self.reading.as_mut() {
            if said.starts_with(LIST_ENDS) {
                let channels = std::mem::take(rows);
                self.finish(channels, None);
            } else if said.starts_with(NONE) {
                self.finish(Vec::new(), Some(said));
            } else {
                rows.extend(row(&said));
            }
            return true;
        }
        if said == LIST_STARTS {
            self.reading = Some(Vec::new());
            return true;
        }
        if said.is_empty() || ALL_TURNED.iter().any(|a| squeeze(a) == said) {
            return true;
        }
        if self.turns > 0 {
            if REFUSALS.iter().any(|r| squeeze(r) == said) {
                self.turns -= 1;
                self.refused = Some(said);
                return true;
            }
            if turned(&said) {
                self.turns -= 1;
                return true;
            }
        }
        false
    }

    fn finish(&mut self, channels: Vec<Channel>, none: Option<String>) {
        self.reading = None;
        self.asked = None;
        self.turns = 0;
        self.done = Some(Listing { channels, none, refused: self.refused.take() });
    }

    /// The list, when one was read since last asked.
    pub fn take_listing(&mut self) -> Option<Listing> {
        self.done.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(c: &mut Channels, lines: &[&str], now: Instant) -> Vec<bool> {
        lines.iter().map(|l| c.line(l, now)).collect()
    }

    const LIST: &[&str] = &[
        "",
        "Available channels: ",
        "GOSSIP                  OOC (OFF)               INFO                    ",
        "ZCHAT                   AUCTION                 ",
        "Use NOCHANNELNAME (ex: NOGOSSIP) to turn a channel off.",
    ];

    #[test]
    fn reads_the_list_and_hides_it() {
        let t = Instant::now();
        let mut c = Channels::default();
        // Not asked: nothing's hidden, nothing's read.
        assert_eq!(read(&mut c, LIST, t), [false; 5]);
        assert!(c.take_listing().is_none());
        assert_eq!(c.ask(t), ["channels"]);
        assert_eq!(read(&mut c, LIST, t), [true; 5]);
        let l = c.take_listing().unwrap();
        let on: Vec<(&str, bool)> = l.channels.iter().map(|c| (c.name.as_str(), c.on)).collect();
        assert_eq!(on, [("GOSSIP", true), ("OOC", false), ("INFO", true), ("ZCHAT", true), ("AUCTION", true)]);
        assert_eq!((l.none, l.refused), (None, None));
        // Over: the game's next lines show.
        assert!(!c.line("", t) && !c.line("A goblin arrives.", t));
    }

    #[test]
    fn turning_one_and_its_answers() {
        let t = Instant::now();
        let mut c = Channels::default();
        assert_eq!(c.turn("zchat", false, t), ["noZCHAT", "channels"]);
        assert!(c.line("The ZCHAT channel has been turned off.  Use `ZCHAT` to turn it back on.", t));
        assert!(read(&mut c, LIST, t).iter().all(|h| *h));
        assert_eq!(c.take_listing().unwrap().refused, None);
        assert_eq!(c.turn("OOC", true, t), ["OOC on", "channels"]);
        assert!(c.line("OOC has been turned on.  Use `NOOOC` to turn it off again.", t));
        assert!(read(&mut c, LIST, t).iter().all(|h| *h));
        c.turn("OOC", true, t);
        assert!(c.line("OOC is already turned on.", t));
        c.turn("ZCHAT", false, t);
        assert!(c.line("The ZCHAT channel is already off.", t));
        // Only names: nothing else is sent.
        assert!(c.turn("ooc; quit", false, t).is_empty() && c.turn(" ", true, t).is_empty());
    }

    #[test]
    fn a_refusal_is_told() {
        let t = Instant::now();
        let mut c = Channels::default();
        c.turn("CLANTALK", true, t);
        assert!(c.line("You have QUIET mode on.  You must turn it off first.", t));
        assert!(c.line("", t) && c.line("Available channels: ", t) && c.line("None, because you have QUIET mode on.", t));
        let l = c.take_listing().unwrap();
        assert!(l.channels.is_empty());
        assert_eq!(l.none.as_deref(), Some("None, because you have QUIET mode on."));
        assert_eq!(l.refused.as_deref(), Some("You have QUIET mode on. You must turn it off first."));
        // A refusal Coupler didn't cause is the game's to show.
        assert!(!c.line("This channel is not available to you.", t));
    }

    #[test]
    fn all_on_and_off() {
        let t = Instant::now();
        let mut c = Channels::default();
        assert_eq!(c.all(false, t), ["channels off", "channels"]);
        assert!(c.line("Channels are now all turned off.", t));
        assert!(read(&mut c, &["", "Available channels: ", "None, because you turned them all off.  Use CHANNELS ON to turn them back on."], t).iter().all(|h| *h));
        assert!(c.take_listing().unwrap().none.unwrap().starts_with("None, because you turned them all off."));
        // CHANNELS ON lists them itself; the CHANNELS after lists them again.
        c.all(true, t);
        assert!(read(&mut c, &["Channels are now turned back on.", ""], t).iter().all(|h| *h));
        assert!(read(&mut c, &LIST[1..], t).iter().all(|h| *h));
        assert_eq!(c.take_listing().unwrap().channels.len(), 5);
    }

    #[test]
    fn an_answer_never_coming_hides_nothing_after() {
        let t = Instant::now();
        let mut c = Channels::default();
        c.ask(t);
        assert!(c.line("Available channels: ", t));
        assert!(c.awaiting(t + Duration::from_secs(9)));
        assert!(!c.awaiting(t + ANSWER_WAITS));
        assert!(!c.line("GOSSIP", t + ANSWER_WAITS));
    }
}

//! **Disconnections**: what Coupler saw in a connection's last moments,
//! kept when it ends without the player leaving, to tell why (and to
//! report a fault of the game's, if it was one). Pure and unit-tested;
//! shared with the web build. `session.rs` feeds it as bytes come and
//! go; at the end, `finish` names a likely cause and writes the
//! findings, kept as a row of Coupler's Disconnections table
//! (`tables.rs`), and offered to the player (`DisconnectDialog.tsx`)
//! unless they meant it.
//!
//! What it keeps, a connection's worth, the oldest dropped: the game's
//! last `LINES` lines as shown (Coupler's hidden answers left out), the
//! last `SENT` lines sent, the last `PACKAGES` GMCP packages (names
//! only), bytes each way and when, and the clock's longest pause (the
//! computer asleep, or a page in the background, stops it). A line
//! typed while the game hides what's typed (a password) is never kept,
//! only that one was sent (CLAUDE.md rule 3). Other players' names and
//! talk can be in the lines, so the findings stay on this machine (rule
//! 1) until the player copies them.
//!
//! The causes, first that fits (the game's words from its source):
//! - **You quit**: "Bye!" (Commands/Quit.java), or QUIT sent lately.
//! - **The game shut down**: "<name> is now shutting down!"
//!   (Commands/Shutdown.java).
//! - **Idle**: "You are being logged out!" (DefaultSession's idle timer,
//!   after "If you don't do something, you will be logged out in…").
//! - **Asleep**: the clock stopped `ASLEEP_MS` or more lately.
//! - **Unreadable**: Coupler couldn't read what came (a telnet or
//!   compression error): Coupler's fault, or the game's.
//! - **The network**: an error, or the line reset or dropped without a
//!   word from the game.
//! - **The game closed it** without a word: the one to report.

use serde::Serialize;
use std::collections::VecDeque;

/// The game's lines kept.
const LINES: usize = 40;
/// Lines sent kept.
const SENT: usize = 20;
/// GMCP packages kept.
const PACKAGES: usize = 12;
/// A pause of the clock this long means the computer slept (or the page was in the background).
const ASLEEP_MS: i64 = 30_000;
/// A pause this recent counts.
const ASLEEP_LATELY_MS: i64 = 10 * 60_000;
/// QUIT sent this recently means the player left.
const QUIT_LATELY_MS: i64 = 60_000;

/// How the connection ended, as the socket said.
#[derive(Debug, Clone, PartialEq)]
pub enum Ending {
    /// The game closed it (the end of the stream; the web: a clean close).
    Closed,
    /// It was reset or aborted.
    Reset,
    /// An error reading it.
    Error(String),
    /// What came couldn't be read (telnet, compression).
    Unreadable(String),
    /// The web build's WebSocket closed uncleanly, with its code.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Dropped(u16),
}

/// What's kept of a connection that ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// When it ended, Unix milliseconds.
    pub at: i64,
    /// The player meant it (they quit): not offered.
    pub expected: bool,
    pub how: String,
    pub cause: String,
    pub character: String,
    pub connected_for: String,
    /// Everything, a line each, for the player to read or copy.
    pub details: Vec<String>,
}

#[derive(Debug, Default)]
pub struct LastMoments {
    /// The port, in words, and when it connected; None when not connected.
    port: Option<(String, i64)>,
    lines: VecDeque<(i64, String)>,
    sent: VecDeque<(i64, String)>,
    packages: VecDeque<(i64, String)>,
    bytes_in: u64,
    bytes_out: u64,
    last_in: Option<i64>,
    last_out: Option<i64>,
    /// The clock's last tick, and its longest pause (how long, when it ended).
    ticked: Option<i64>,
    pause: Option<(i64, i64)>,
    /// What the game said that tells why it ended.
    heard: Option<(Heard, i64)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Heard {
    Bye,
    Shutdown,
    Idle,
    IdleWarning,
}

/// "45 s", "3 min 4 s", "2 h 5 min".
pub fn span(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} min {} s", s / 60, s % 60),
        _ => format!("{} h {} min", s / 3600, s / 60 % 60),
    }
}

/// The game's words about the session ending, in a line (after any prompt).
fn heard_in(line: &str) -> Option<Heard> {
    let line = line.trim();
    if line == "Bye!" || line.ends_with("> Bye!") {
        Some(Heard::Bye)
    } else if line.ends_with(" is now shutting down!") {
        Some(Heard::Shutdown)
    } else if line.ends_with("You are being logged out!") {
        Some(Heard::Idle)
    } else if line.contains("If you don't do something, you will be logged out in") {
        Some(Heard::IdleWarning)
    } else {
        None
    }
}

/// A command that's QUIT: CoffeeMUD takes the start of a command's word (QUI, QUIT).
fn quits(line: &str) -> bool {
    let word = line.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
    word.len() >= 3 && "quit".starts_with(&word)
}

fn push<T>(list: &mut VecDeque<T>, most: usize, item: T) {
    list.push_back(item);
    while list.len() > most {
        list.pop_front();
    }
}

impl LastMoments {
    /// A new connection, to `port` (in words).
    pub fn start(&mut self, port: &str, now: i64) {
        *self = LastMoments { port: Some((port.to_string(), now)), ticked: Some(now), ..LastMoments::default() };
    }

    /// Bytes read from the game.
    pub fn received(&mut self, bytes: usize, now: i64) {
        self.bytes_in += bytes as u64;
        self.last_in = Some(now);
    }

    /// The game's finished lines, as shown.
    pub fn lines(&mut self, texts: &[String], now: i64) {
        for text in texts {
            if let Some(heard) = heard_in(text) {
                self.heard = Some((heard, now));
            }
            if !text.trim().is_empty() {
                push(&mut self.lines, LINES, (now, text.trim_end().to_string()));
            }
        }
    }

    pub fn gmcp(&mut self, package: &str, now: i64) {
        push(&mut self.packages, PACKAGES, (now, package.to_string()));
    }

    /// Bytes sent to the game: its lines kept, unless `secret` (the game
    /// isn't echoing: a password), when only that one was sent is; telnet's
    /// own sequences counted, not kept.
    pub fn sent(&mut self, bytes: &[u8], secret: bool, now: i64) {
        self.bytes_out += bytes.len() as u64;
        self.last_out = Some(now);
        let (mut text, mut telnet) = (Vec::new(), 0);
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != 0xff {
                text.push(bytes[i]);
                i += 1;
                continue;
            }
            let length = match bytes.get(i + 1) {
                // An escaped 255, in the text.
                Some(0xff) => {
                    text.push(0xff);
                    i += 2;
                    continue;
                }
                // A subnegotiation runs to IAC SE.
                Some(0xfa) => bytes[i..].windows(2).position(|w| w == [0xff, 0xf0]).map_or(bytes.len() - i, |at| at + 2),
                Some(0xfb..=0xfe) => 3,
                _ => 2,
            };
            telnet += length.min(bytes.len() - i);
            i += length;
        }
        if telnet > 0 {
            push(&mut self.sent, SENT, (now, format!("(telnet negotiation, {telnet} bytes)")));
        }
        let text = String::from_utf8_lossy(&text).into_owned();
        for line in text.split("\r\n").filter(|l| !l.is_empty()) {
            let kept = if secret { "(a line typed while the game hid it, not kept)".to_string() } else { line.to_string() };
            push(&mut self.sent, SENT, (now, kept));
        }
    }

    /// The clock, each second: a long pause means the computer slept.
    pub fn tick(&mut self, now: i64) {
        if let Some(was) = self.ticked {
            let gap = now - was;
            if gap >= ASLEEP_MS && self.pause.is_none_or(|(longest, at)| gap >= longest || now - at > ASLEEP_LATELY_MS) {
                self.pause = Some((gap, now));
            }
        }
        self.ticked = Some(now);
    }

    /// The connection ended: why, as far as can be told, and everything
    /// seen. None if nothing was connected (it was told already).
    pub fn finish(&mut self, ending: Ending, character: &str, now: i64) -> Option<Finding> {
        let (port, started) = self.port.take()?;
        let heard = self.heard.filter(|(_, at)| now - at < QUIT_LATELY_MS * 5).map(|(h, _)| h);
        let quit_sent = self.sent.iter().any(|(at, line)| now - at < QUIT_LATELY_MS && quits(line));
        let asleep = self.pause.filter(|(_, at)| now - at < ASLEEP_LATELY_MS);
        let how = match &ending {
            Ending::Closed => "The game closed the connection.".to_string(),
            Ending::Reset => "The connection was reset.".to_string(),
            Ending::Error(e) => format!("Reading failed: {e}"),
            Ending::Unreadable(e) => format!("What came couldn't be read: {e}"),
            Ending::Dropped(code) => format!("The connection dropped (WebSocket code {code})."),
        };
        let (expected, cause) = if heard == Some(Heard::Bye) || quit_sent {
            (true, "You quit.".to_string())
        } else if heard == Some(Heard::Shutdown) {
            (false, "The game shut down or restarted: it said so before closing.".to_string())
        } else if heard == Some(Heard::Idle) {
            (false, "The game logged you out for being idle.".to_string())
        } else if let Some((gap, _)) = asleep {
            (false, format!("This device was asleep, or Coupler was in the background, for {}: the connection timed out meanwhile.", span(gap)))
        } else if let Ending::Unreadable(_) = ending {
            (false, "Coupler couldn't read what the game sent: a fault in Coupler or the game.".to_string())
        } else if heard == Some(Heard::IdleWarning) {
            (false, "Probably idle: the game had warned it would log you out.".to_string())
        } else {
            match ending {
                Ending::Closed => (false, "The game closed the connection without a word: worth reporting, with these findings.".to_string()),
                Ending::Reset => (false, "The game or the network between reset the connection without a word.".to_string()),
                _ => (false, "The network dropped: the internet connection, or the game's server, stopped answering.".to_string()),
            }
        };
        let ago = |at: i64| format!("{} before", span(now - at));
        let mut details = vec![
            format!("Way to play: {port}"),
            format!("Character: {}", if character.is_empty() { "none in the game yet" } else { character }),
            format!("Connected for {}", span(now - started)),
            format!("How it ended: {how}"),
            format!("Likely cause: {cause}"),
            format!("Last heard from the game: {}", self.last_in.map_or("nothing".to_string(), ago)),
            format!("Last sent to the game: {}", self.last_out.map_or("nothing".to_string(), ago)),
            format!("Received {} bytes, sent {} bytes", self.bytes_in, self.bytes_out),
            match self.pause {
                Some((gap, at)) => format!("Longest pause of Coupler's clock: {}, ended {}", span(gap), ago(at)),
                None => "Coupler's clock never paused (no sleep).".to_string(),
            },
            String::new(),
            "Sent lately, the latest last:".to_string(),
        ];
        details.extend(self.sent.iter().map(|(at, line)| format!("  {}: {line}", ago(*at))));
        details.push(String::new());
        details.push("GMCP lately, the latest last:".to_string());
        details.extend(self.packages.iter().map(|(at, p)| format!("  {}: {p}", ago(*at))));
        details.push(String::new());
        details.push("The game's last lines:".to_string());
        details.extend(self.lines.iter().map(|(at, line)| format!("  {}: {line}", ago(*at))));
        let finding = Finding { at: now, expected, how, cause, character: character.to_string(), connected_for: span(now - started), details };
        *self = LastMoments::default();
        Some(finding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected() -> LastMoments {
        let mut r = LastMoments::default();
        r.start("Standard: the main game", 0);
        r
    }

    #[test]
    fn a_quit_is_expected() {
        let mut r = connected();
        r.sent(b"quit\r\n", false, 50_000);
        r.lines(&["Quit -- are you sure (y/N)?".into()], 50_100);
        r.sent(b"y\r\n", false, 51_000);
        r.lines(&["<20Hp 9m> Bye!".into()], 51_100);
        let f = r.finish(Ending::Closed, "Hassan", 52_000).unwrap();
        assert!(f.expected);
        assert_eq!(f.cause, "You quit.");
        // Once only.
        assert!(r.finish(Ending::Closed, "Hassan", 52_000).is_none());
    }

    #[test]
    fn the_games_own_reasons() {
        let mut r = connected();
        r.lines(&["CoffeeMUD is now shutting down!".into()], 10_000);
        let f = r.finish(Ending::Closed, "Hassan", 11_000).unwrap();
        assert!(!f.expected);
        assert!(f.cause.starts_with("The game shut down"));

        let mut r = connected();
        r.lines(&["If you don't do something, you will be logged out in 1 minute(s)!".into()], 10_000);
        r.lines(&["You are being logged out!".into()], 70_000);
        assert!(r.finish(Ending::Closed, "Hassan", 71_000).unwrap().cause.contains("idle"));
    }

    #[test]
    fn asleep_and_unexplained() {
        let mut r = connected();
        r.tick(1_000);
        r.tick(2_000);
        r.tick(602_000);
        let f = r.finish(Ending::Reset, "", 603_000).unwrap();
        assert!(f.cause.contains("asleep"), "{}", f.cause);
        assert!(f.details.iter().any(|d| d == "Character: none in the game yet"));

        let mut r = connected();
        r.received(500, 5_000);
        let f = r.finish(Ending::Closed, "Hassan", 9_000).unwrap();
        assert!(f.cause.contains("without a word"));
        assert!(f.details.contains(&"Last heard from the game: 4 s before".to_string()));
        assert_eq!(f.connected_for, "9 s");
    }

    #[test]
    fn a_password_is_never_kept() {
        let mut r = connected();
        r.sent(b"Hassan\r\n", false, 1_000);
        r.sent(b"hunter2\r\n", true, 2_000);
        r.sent(&[0xff, 0xfa, 31, 0, 80, 0, 24, 0xff, 0xf0], false, 3_000);
        // The web build's bytes come in batches, text and telnet together.
        r.sent(&[b'w', b'h', b'o', b'\r', b'\n', 0xff, 0xfb, 24], false, 3_500);
        let f = r.finish(Ending::Error("timed out".into()), "", 4_000).unwrap();
        let all = f.details.join("\n");
        assert!(!all.contains("hunter2"));
        assert!(all.contains("Hassan"));
        assert!(all.contains("(telnet negotiation, 9 bytes)"));
        assert!(all.contains(": who\n") && all.contains("(telnet negotiation, 3 bytes)"));
        assert!(f.cause.starts_with("The network dropped"));
    }

    #[test]
    fn keeps_only_the_last() {
        let mut r = connected();
        let many: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
        r.lines(&many, 1_000);
        assert_eq!(r.lines.len(), LINES);
        assert_eq!(r.lines.back().unwrap().1, "line 99");
        assert_eq!(span(3_725_000), "1 h 2 min");
        assert_eq!(span(65_000), "1 min 5 s");
    }
}

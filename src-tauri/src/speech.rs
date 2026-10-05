//! What kind each line of the game output is, so speech can be told by
//! kind (the player picks which kinds are read). Pure and unit-tested;
//! `session.rs` feeds it each read's talk and lines and sends the kinds
//! with `mud-output`.
//!
//! - **Talk**: a line that `comm.channel` already gave as its `msg`.
//!   CoffeeMUD sends the GMCP before the text (`CommonMsgs.java`,
//!   `sendInlineCommand` then `executeMsg`), unwrapped (`unWWrap`), while
//!   the text may be wrapped over several lines: so each line that the
//!   waiting talk starts with is talk, and is taken off its front. Talk
//!   that's waited too long (the line never came, or was gagged) is
//!   forgotten.
//! - **Prompt**: the prompt the last read left unfinished, finished by
//!   this read (the game ends it when it answers a command). It was
//!   already shown, and said, as the unfinished line.
//! - **Time**: a line about the time of day (`daytime::about_time`): a
//!   change of the part of the day, or what `time` says. Immersive's
//!   narrator says these instead of writing them.
//! - **Combat**: any other line while the player is fighting
//!   (`combat.rs`).
//! - **Echo**: a command's one-line answer (`echo.rs`, which marks them
//!   after this): Immersive's narrator says them in short instead.
//! - **Game**: the rest.
//!
//! Not yet the room's lines: whether `room.info` comes before or after
//! the room's text is on the "to confirm" list (docs/coffeemud-gmcp.md).

use std::collections::VecDeque;
use std::time::Duration;

use crate::clock::Instant;
use crate::daytime;

use serde::Serialize;

/// How long talk waits for its line.
const TALK_WAITS: Duration = Duration::from_secs(3);
/// Talk waiting at most; older is dropped first.
const TALK_KEPT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Game,
    Talk,
    Time,
    Combat,
    Prompt,
    Echo,
}

#[derive(Default)]
pub struct Kinds {
    /// Talk not yet matched to its lines: what's left of it, and when it came.
    talk: VecDeque<(String, Instant)>,
    /// The unfinished line the last read left, if any.
    prompt: Option<String>,
}

/// Spaces collapsed, ends trimmed: wrapping and padding don't count.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Kinds {
    /// A line of talk (`comm.channel`'s `msg`) is coming.
    pub fn talk(&mut self, text: &str, now: Instant) {
        let text = squeeze(text);
        if text.is_empty() {
            return;
        }
        if self.talk.len() == TALK_KEPT {
            self.talk.pop_front();
        }
        self.talk.push_back((text, now));
    }

    /// The kinds of one read's finished lines (their plain text), then
    /// what it left unfinished, kept for the next read.
    pub fn lines(&mut self, lines: &[String], partial: Option<&str>, fighting: bool, now: Instant) -> Vec<LineKind> {
        self.talk.retain(|(_, at)| now.duration_since(*at) <= TALK_WAITS);
        let prompt = self.prompt.take();
        let kinds = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let text = squeeze(line);
                if i == 0 && prompt.as_deref().is_some_and(|p| !p.is_empty() && squeeze(p) == text) {
                    return LineKind::Prompt;
                }
                if !text.is_empty() && self.take_talk(&text) {
                    return LineKind::Talk;
                }
                if daytime::about_time(&text) {
                    return LineKind::Time;
                }
                if fighting {
                    LineKind::Combat
                } else {
                    LineKind::Game
                }
            })
            .collect();
        self.prompt = partial.map(str::to_string);
        kinds
    }

    /// Whether some waiting talk starts with `text`; if so it's taken off.
    fn take_talk(&mut self, text: &str) -> bool {
        let Some(i) = self.talk.iter().position(|(t, _)| t.starts_with(text)) else { return false };
        let rest = squeeze(&self.talk[i].0[text.len()..]);
        if rest.is_empty() {
            self.talk.remove(i);
        } else {
            self.talk[i].0 = rest;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LineKind::*;

    fn strings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn talk_is_matched_once() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk("Bob tells you 'hi'", now);
        let kinds = k.lines(&strings(&["A rat arrives.", "Bob tells you 'hi'", "Bob tells you 'hi'"]), None, false, now);
        assert_eq!(kinds, vec![Game, Talk, Game]);
    }

    #[test]
    fn wrapped_talk_is_every_line_of_it() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk("Ann gossips 'a long line that the game wraps over two'", now);
        let kinds = k.lines(&strings(&["Ann gossips 'a long line that the", "  game wraps over two'", "Done."]), None, false, now);
        assert_eq!(kinds, vec![Talk, Talk, Game]);
    }

    #[test]
    fn talk_may_come_a_read_early() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk("You say 'hello'", now);
        assert_eq!(k.lines(&[], Some("<10hp> "), false, now), Vec::<LineKind>::new());
        let later = now + Duration::from_millis(200);
        assert_eq!(k.lines(&strings(&["<10hp>", "You say 'hello'"]), None, false, later), vec![Prompt, Talk]);
    }

    #[test]
    fn old_talk_is_forgotten() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk("Bob tells you 'hi'", now);
        let later = now + TALK_WAITS + Duration::from_secs(1);
        assert_eq!(k.lines(&strings(&["Bob tells you 'hi'"]), None, false, later), vec![Game]);
    }

    #[test]
    fn the_finished_prompt_is_a_prompt_only_first() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.lines(&[], Some("<10hp 5m> "), false, now);
        let kinds = k.lines(&strings(&["<10hp 5m>", "<10hp 5m>"]), Some("<10hp 5m> "), false, now);
        assert_eq!(kinds, vec![Prompt, Game]);
        // No prompt left unfinished: nothing is a prompt.
        k.lines(&[], None, false, now);
        assert_eq!(k.lines(&strings(&["<10hp 5m>"]), None, false, now), vec![Game]);
    }

    #[test]
    fn a_fight_makes_the_rest_combat() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk("Bob tells you 'run'", now);
        let kinds = k.lines(&strings(&["You hit the rat.", "Bob tells you 'run'"]), None, true, now);
        assert_eq!(kinds, vec![Combat, Talk]);
    }

    #[test]
    fn lines_about_the_time() {
        let mut k = Kinds::default();
        let now = Instant::now();
        let kinds = k.lines(&strings(&["The sun begins to set in the east.", "It is dusk (Hour: 4/5)", "It is winter.", "A rat arrives."]), None, true, now);
        assert_eq!(kinds, vec![Time, Time, Time, Combat]);
    }
}

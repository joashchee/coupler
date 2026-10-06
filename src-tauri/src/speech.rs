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
//!
//!   **What's heard is what was printed** (`heard`, before `lines`), so
//!   the journal keeps what the player saw:
//!   - A say (SAY, YELL, SHOUT, ASK, `Commands/Say.java`, all sent as
//!     `say`) is sent by GMCP *before* the room's `okMessage`, so before
//!     a mood rewrites it ("Bob angrily says", FORMAL's "declares",
//!     PASSIVE's lowercase, `Abilities/Misc/Mood.java`) and before a
//!     language scrambles its words (`StdLanguage.okMessage`). Its line
//!     is found by its speaker instead, and the printed line is kept.
//!     Tells and channels are sent after, so they match as they are.
//!   - A language the player knows adds a line after, `<the line, its
//!     words in clear> (translated from Elvish)` (`translateOthersMessage`
//!     and the like, a trailer with no GMCP): it's talk too, and what's
//!     kept. One they don't know stays scrambled, as they saw it.
//!   - WHISPER (`Commands/Whisper.java`) and a yell or shout heard from
//!     the next room (`You hear someone yell '…' to the north.`) send no
//!     GMCP: they're read from the text (`from_text`).
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
use crate::senses::{Talk, TalkKind};

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

/// Talk waiting for its printed lines.
struct Waiting {
    /// As `comm.channel` gave it.
    talk: Talk,
    /// What's left of its `msg` to match exactly, squeezed.
    rest: String,
    at: Instant,
    /// Handed out by `heard` already.
    told: bool,
}

#[derive(Default)]
pub struct Kinds {
    /// Talk not yet matched to its lines.
    talk: VecDeque<Waiting>,
    /// The unfinished line the last read left, if any.
    prompt: Option<String>,
    /// The lines `heard` found to be talk, for `lines`.
    marked: Option<Vec<bool>>,
}

/// Spaces collapsed, ends trimmed: wrapping and padding don't count.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Who says a line of talk, as its first words: "Bob", "You", "A goblin".
fn speaker(text: &str) -> Option<String> {
    let head = &text[..text.find('\'')?];
    let mut words = head.split_whitespace();
    let first = words.next()?.trim_end_matches(',');
    let article = ["a", "an", "the", "some"].contains(&first.to_ascii_lowercase().as_str());
    Some(if article { format!("{first} {}", words.next()?.trim_end_matches(',')) } else { first.to_string() })
}

/// Whether `line` is said by whoever says `talk`, with words in quotes.
fn same_speaker(talk: &str, line: &str) -> bool {
    let (Some(who), Some(said)) = (speaker(talk), speaker(line)) else { return false };
    who.eq_ignore_ascii_case(&said)
}

/// Whether printed talk is over: its words' closing quote has come (one
/// followed by nothing, a space or a stop, not an apostrophe in a word).
fn closed(text: &str) -> bool {
    let Some(open) = text.find('\'') else { return true };
    let after = &text[open + 1..];
    after.char_indices().any(|(i, c)| c == '\'' && after[i + 1..].chars().next().is_none_or(|n| n.is_whitespace() || ".,!?)".contains(n)))
}

/// Talk that comes with no GMCP, from its text: a whisper, or a yell
/// heard from another room.
pub fn from_text(text: &str) -> Option<Talk> {
    let quote = text.find('\'')?;
    let head = text[..quote].trim();
    let words: Vec<&str> = head.split_whitespace().collect();
    if let Some(rest) = head.strip_prefix("You hear someone ") {
        if !rest.is_empty() && !rest.contains(' ') {
            return Some(Talk { kind: TalkKind::Say, channel: "say".into(), from: String::new(), text: text.to_string(), mine: false });
        }
        return None;
    }
    let at = words.iter().position(|w| w.eq_ignore_ascii_case("whisper") || w.eq_ignore_ascii_case("whispers"))?;
    if at == 0 || !matches!(words.get(at + 1).map(|w| w.to_ascii_lowercase()).as_deref(), Some("to" | "around")) {
        return None;
    }
    let from = words[..at].join(" ");
    let mine = from == "You";
    Some(Talk { kind: TalkKind::Say, channel: "whisper".into(), from: if mine { String::new() } else { from }, text: text.to_string(), mine })
}

/// How far a line of talk runs from `start`: the lines its words go on
/// over, till its closing quote (or as many words as `words`, and a few).
fn printed_to(texts: &[String], start: usize, taken: &[bool], words: usize) -> usize {
    let mut joined = squeeze(&texts[start]);
    let mut end = start + 1;
    while !closed(&joined) && end < texts.len() && !taken[end] && joined.split_whitespace().count() < words + 8 {
        let next = squeeze(&texts[end]);
        if next.is_empty() {
            break;
        }
        joined = format!("{joined} {next}");
        end += 1;
    }
    end
}

/// A language's translation after talk ends at `end`: the lines it
/// runs to, if the next line is one (same speaker, "(translated from").
fn translation_to(texts: &[String], end: usize, taken: &[bool], said: &str) -> Option<usize> {
    let first = squeeze(texts.get(end)?);
    if taken[end] || !same_speaker(said, &first) {
        return None;
    }
    let mut joined = first;
    let mut to = end + 1;
    while !joined.contains("(translated from ") || !joined.ends_with(')') {
        if to >= texts.len() || taken[to] || to - end > 6 {
            return None;
        }
        joined = format!("{joined} {}", squeeze(&texts[to]));
        to += 1;
    }
    Some(to)
}

impl Kinds {
    /// A line of talk (`comm.channel`) is coming.
    pub fn talk(&mut self, talk: Talk, now: Instant) {
        let rest = squeeze(&talk.text);
        if rest.is_empty() {
            return;
        }
        if self.talk.len() == TALK_KEPT {
            self.talk.pop_front();
        }
        self.talk.push_back(Waiting { talk, rest, at: now, told: false });
    }

    /// One read's finished lines (their plain text), before `lines`: the
    /// talk to keep, each as the player saw it (the printed line, a
    /// mood's or a language's; a translation when there's one), and the
    /// talk that came with no GMCP. Talk whose lines haven't come is
    /// given as `comm.channel` sent it, and still waits to be marked.
    pub fn heard(&mut self, texts: &[String], now: Instant) -> Vec<Talk> {
        let mut heard = Vec::new();
        // Waited too long: no longer marked, but never lost.
        self.talk.retain(|w| {
            let waiting = now.duration_since(w.at) <= TALK_WAITS;
            if !waiting && !w.told {
                heard.push(w.talk.clone());
            }
            waiting
        });
        let mut taken = vec![false; texts.len()];
        let mut i = 0;
        while i < texts.len() {
            let text = squeeze(&texts[i]);
            if text.is_empty() || taken[i] {
                i += 1;
                continue;
            }
            // Exactly as sent (a tell, a channel, a say with no mood).
            if let Some(w) = self.talk.iter().position(|w| w.rest.starts_with(&text)) {
                let mut end = i;
                let mut printed = Vec::new();
                while end < texts.len() && !taken[end] {
                    let line = squeeze(&texts[end]);
                    let rest = &self.talk[w].rest;
                    if line.is_empty() || !rest.starts_with(&line) {
                        break;
                    }
                    self.talk[w].rest = squeeze(&rest[line.len()..]);
                    taken[end] = true;
                    printed.push(line);
                    end += 1;
                    if self.talk[w].rest.is_empty() {
                        break;
                    }
                }
                let whole = self.talk[w].rest.is_empty();
                self.found(w, printed.join(" "), texts, end, &mut taken, &mut heard, whole);
                i = end;
                continue;
            }
            // Said, and changed after it was sent: found by who says it.
            if let Some(w) = self.talk.iter().position(|w| !w.told && w.talk.kind == TalkKind::Say && same_speaker(&w.talk.text, &text)) {
                let end = printed_to(texts, i, &taken, self.talk[w].rest.split_whitespace().count());
                taken[i..end].iter_mut().for_each(|t| *t = true);
                let printed = texts[i..end].iter().map(|t| squeeze(t)).collect::<Vec<_>>().join(" ");
                self.found(w, printed, texts, end, &mut taken, &mut heard, true);
                i = end;
                continue;
            }
            // No GMCP: a whisper, a yell from afar.
            if let Some(mut talk) = from_text(&text) {
                let end = printed_to(texts, i, &taken, usize::MAX / 2);
                taken[i..end].iter_mut().for_each(|t| *t = true);
                talk.text = texts[i..end].iter().map(|t| squeeze(t)).collect::<Vec<_>>().join(" ");
                heard.push(talk);
                i = end;
                continue;
            }
            i += 1;
        }
        // Not printed yet: kept as sent, its lines still to be marked.
        for w in self.talk.iter_mut().filter(|w| !w.told) {
            w.told = true;
            heard.push(w.talk.clone());
        }
        self.marked = Some(taken);
        heard
    }

    /// Talk `w` was printed as `printed`, its lines ending before `end`:
    /// taken with its translation if one follows, and heard if it wasn't.
    #[allow(clippy::too_many_arguments)]
    fn found(&mut self, w: usize, printed: String, texts: &[String], end: usize, taken: &mut [bool], heard: &mut Vec<Talk>, whole: bool) {
        let mut said = printed;
        if let Some(to) = translation_to(texts, end, taken, &said) {
            taken[end..to].iter_mut().for_each(|t| *t = true);
            said = texts[end..to].iter().map(|t| squeeze(t)).collect::<Vec<_>>().join(" ");
        }
        let waiting = &mut self.talk[w];
        if !waiting.told {
            waiting.told = true;
            heard.push(Talk { text: said, ..waiting.talk.clone() });
        }
        if whole {
            self.talk.remove(w);
        }
    }

    /// The kinds of one read's finished lines (their plain text), then
    /// what it left unfinished, kept for the next read. Talk is what
    /// `heard` found, when it was asked first.
    pub fn lines(&mut self, lines: &[String], partial: Option<&str>, fighting: bool, now: Instant) -> Vec<LineKind> {
        let talk = match self.marked.take() {
            Some(marked) if marked.len() == lines.len() => marked,
            _ => {
                self.heard(lines, now);
                self.marked.take().unwrap_or_default()
            }
        };
        let prompt = self.prompt.take();
        let kinds = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let text = squeeze(line);
                if i == 0 && prompt.as_deref().is_some_and(|p| !p.is_empty() && squeeze(p) == text) {
                    return LineKind::Prompt;
                }
                if talk.get(i).copied().unwrap_or(false) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use LineKind::*;

    fn strings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    /// Talk as `comm.channel` sends it: a say when it says so, else a channel.
    fn said(text: &str) -> crate::senses::Talk {
        let say = text.contains(" say") || text.contains(" yell") || text.contains(" shout") || text.contains(" ask");
        let kind = if text.contains(" tell") { TalkKind::Tell } else if say { TalkKind::Say } else { TalkKind::Channel };
        let from = text.split_whitespace().next().unwrap_or("").to_string();
        crate::senses::Talk { kind, channel: if say { "say".into() } else { "GOSSIP".into() }, mine: from == "You", from, text: text.into() }
    }

    fn heard_texts(k: &mut Kinds, lines: &[&str], now: Instant) -> (Vec<String>, Vec<LineKind>) {
        let lines = strings(lines);
        let heard = k.heard(&lines, now).into_iter().map(|t| t.text).collect();
        (heard, k.lines(&lines, None, false, now))
    }

    #[test]
    fn a_mood_changes_the_say_and_the_printed_line_is_kept() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Bob says 'hello there'"), now);
        let (heard, kinds) = heard_texts(&mut k, &["Bob angrily says 'HELLO THERE'", "A rat arrives."], now);
        assert_eq!(heard, ["Bob angrily says 'HELLO THERE'"]);
        assert_eq!(kinds, vec![Talk, Game]);
        // FORMAL's verb, PASSIVE's lowercase.
        k.talk(said("Bob says 'hi'"), now);
        k.talk(said("You say 'yo'"), now);
        let (heard, kinds) = heard_texts(&mut k, &["Bob declares 'Hello.'", "you whisper 'yo'"], now);
        assert_eq!(heard, ["Bob declares 'Hello.'", "you whisper 'yo'"]);
        assert_eq!(kinds, vec![Talk, Talk]);
    }

    #[test]
    fn a_mooded_say_wrapped_over_lines() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("A goblin says 'we have been waiting for you for a very long time'"), now);
        let (heard, kinds) = heard_texts(&mut k, &["A goblin sadly says 'we have been waiting for you for a", "very long time'", "A goblin leaves."], now);
        assert_eq!(heard, ["A goblin sadly says 'we have been waiting for you for a very long time'"]);
        assert_eq!(kinds, vec![Talk, Talk, Game]);
    }

    #[test]
    fn a_language_not_known_is_kept_scrambled() {
        let mut k = Kinds::default();
        let now = Instant::now();
        // The GMCP has the words in clear; the player sees them scrambled.
        k.talk(said("Elrond says 'well met'"), now);
        let (heard, kinds) = heard_texts(&mut k, &["Elrond says 'wyth nar'"], now);
        assert_eq!(heard, ["Elrond says 'wyth nar'"]);
        assert_eq!(kinds, vec![Talk]);
    }

    #[test]
    fn a_language_known_is_kept_translated() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Elrond says 'well met, friend'"), now);
        let (heard, kinds) = heard_texts(&mut k, &["Elrond says 'wyth nar, fryond'", "Elrond says 'well met, friend' (translated from", "Elvish)", "Done."], now);
        assert_eq!(heard, ["Elrond says 'well met, friend' (translated from Elvish)"]);
        assert_eq!(kinds, vec![Talk, Talk, Talk, Game]);
        // A tell's GMCP is sent scrambled; its translation follows the same way.
        k.talk(said("Elrond tells you 'wyth'"), now);
        let (heard, _) = heard_texts(&mut k, &["Elrond tells you 'wyth'", "Elrond tells you 'well' (translated from Elvish)"], now);
        assert_eq!(heard, ["Elrond tells you 'well' (translated from Elvish)"]);
    }

    #[test]
    fn whispers_and_far_yells_come_with_no_gmcp() {
        let mut k = Kinds::default();
        let now = Instant::now();
        let lines = strings(&["Bob whispers to you 'the key is under the mat'.", "You whisper to Bob 'thanks'.", "Bob whispers something to Ann.", "You hear someone yell 'help!' to the north."]);
        let heard = k.heard(&lines, now);
        assert_eq!(heard.len(), 3);
        assert_eq!((heard[0].from.as_str(), heard[0].channel.as_str(), heard[0].mine), ("Bob", "whisper", false));
        assert!(heard[1].mine);
        assert_eq!((heard[2].from.as_str(), heard[2].text.as_str()), ("", "You hear someone yell 'help!' to the north."));
        assert_eq!(k.lines(&lines, None, false, now), vec![Talk, Talk, Game, Talk]);
    }

    #[test]
    fn talk_not_yet_printed_is_heard_as_sent_and_marked_later() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Bob tells you 'hi'"), now);
        let (heard, _) = heard_texts(&mut k, &[], now);
        assert_eq!(heard, ["Bob tells you 'hi'"]);
        let (heard, kinds) = heard_texts(&mut k, &["Bob tells you 'hi'"], now);
        assert!(heard.is_empty());
        assert_eq!(kinds, vec![Talk]);
    }

    #[test]
    fn an_emote_by_the_speaker_isnt_their_say() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Bob says 'hi'"), now);
        let (_, kinds) = heard_texts(&mut k, &["Bob nods.", "Bob says 'hi'"], now);
        assert_eq!(kinds, vec![Game, Talk]);
    }

    #[test]
    fn talk_is_matched_once() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Bob tells you 'hi'"), now);
        let kinds = k.lines(&strings(&["A rat arrives.", "Bob tells you 'hi'", "Bob tells you 'hi'"]), None, false, now);
        assert_eq!(kinds, vec![Game, Talk, Game]);
    }

    #[test]
    fn wrapped_talk_is_every_line_of_it() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Ann gossips 'a long line that the game wraps over two'"), now);
        let kinds = k.lines(&strings(&["Ann gossips 'a long line that the", "  game wraps over two'", "Done."]), None, false, now);
        assert_eq!(kinds, vec![Talk, Talk, Game]);
    }

    #[test]
    fn talk_may_come_a_read_early() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("You say 'hello'"), now);
        assert_eq!(k.lines(&[], Some("<10hp> "), false, now), Vec::<LineKind>::new());
        let later = now + Duration::from_millis(200);
        assert_eq!(k.lines(&strings(&["<10hp>", "You say 'hello'"]), None, false, later), vec![Prompt, Talk]);
    }

    #[test]
    fn old_talk_is_forgotten() {
        let mut k = Kinds::default();
        let now = Instant::now();
        k.talk(said("Bob tells you 'hi'"), now);
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
        k.talk(said("Bob tells you 'run'"), now);
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

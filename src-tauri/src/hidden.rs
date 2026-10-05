//! **Hidden details**: what a long look shows only by color. Pure and
//! unit-tested; `session.rs` tells it each line the player sends and gives
//! it each read's lines after `speech.rs` has kinded them.
//!
//! CoffeeMUD's long look at a room (EXAMINE with nothing after it, or
//! LONGLOOK, LLOOK, LL, EXAM, EXA, ID: `Commands/Examine.java`) builds the
//! room's view with `LookView.LOOK_LONG` (`CommonMsgs.getFullRoomView`):
//! for each item in the room with no display text (scenery the builder
//! wrote into the description, which a plain LOOK never lists), every
//! word of its name found in the description is wrapped in
//! `^H…^?`, the player's HIGHLIGHT color (light cyan by default) inside
//! ROOMDESC (white). With MXP off, as Coupler has it, the color is all
//! that marks them: a sighted player sees which words can be looked at,
//! got or searched, and a screen reader hears none of it.
//!
//! So after a long look, the reply's room is found (its title, the
//! current room's name from `room.info`, or else its first line), the
//! description is the lines after it up to the first empty one, and
//! anything there in a color other than the description's own (the most
//! of its letters) is a hidden detail. Highlighted words next to each
//! other are one detail ("marble fountain"), each named once.
//!
//! The player's colors don't matter (any HIGHLIGHT unlike their ROOMDESC
//! works). What can mislead: a builder's own colors inside a description,
//! and EXVIEW PARAGRAPH or MIXED, which adds the exits in color to it
//! (coffeemud.ini has DEFAULT).

use std::time::Duration;

use serde::Serialize;

use crate::ansi::{Color, Line};
use crate::clock::Instant;
use crate::speech::LineKind;

/// How long a long look waits for its room.
const WAITS: Duration = Duration::from_secs(5);

/// The words that call a long look (`Examine.java`'s access words).
const LONG_LOOK: &[&str] = &["EXAMINE", "EXAM", "EXA", "LONGLOOK", "LLOOK", "LL", "ID"];

/// A long look's room: its description, and what was in color in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hidden {
    /// The description's first line in the read.
    pub line: usize,
    /// How many lines it took.
    pub lines: usize,
    /// The description, its lines joined back up.
    pub text: String,
    /// The hidden details, in the order they came; none found, empty.
    pub words: Vec<String>,
}

#[derive(Default)]
pub struct LongLook {
    /// When a long look was sent, until its room comes.
    asked: Option<Instant>,
}

/// Spaces collapsed, ends trimmed: wrapping and padding don't count.
fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a typed line is a long look at the room: one of the words
/// alone (or with UNOBTRUSIVELY, which only keeps the room from hearing).
fn long_look(line: &str) -> bool {
    let words: Vec<String> = line.split_whitespace().map(str::to_uppercase).collect();
    match words.as_slice() {
        [word] => LONG_LOOK.contains(&word.as_str()),
        [word, quiet] => LONG_LOOK.contains(&word.as_str()) && quiet == "UNOBTRUSIVELY",
        _ => false,
    }
}

/// What tells one color from another here: the foreground, the
/// background and bold (the 16 colors' bright half, as some servers send it).
type Shade = (Option<Color>, Option<Color>, bool);

/// The details highlighted in a description's lines.
fn details(lines: &[Line]) -> Vec<String> {
    // Each character, and its shade; a line's end is a space.
    let mut chars: Vec<(char, Shade)> = Vec::new();
    for line in lines {
        for span in line {
            let shade = (span.style.fg, span.style.bg, span.style.bold);
            chars.extend(span.text.chars().map(|c| (c, shade)));
        }
        chars.push((' ', (None, None, false)));
    }
    // The description's own shade: the one most of its letters are in.
    let mut counts: Vec<(Shade, usize)> = Vec::new();
    for (c, shade) in &chars {
        if c.is_whitespace() {
            continue;
        }
        match counts.iter_mut().find(|(s, _)| s == shade) {
            Some((_, n)) => *n += 1,
            None => counts.push((*shade, 1)),
        }
    }
    let Some(&(own, _)) = counts.iter().max_by_key(|(_, n)| *n) else { return Vec::new() };
    // Words (runs between spaces), each in color or not; a word with any
    // of it in color counts (the server colors whole words, punctuation
    // and all, but a builder's might not).
    let mut words: Vec<(String, bool)> = Vec::new();
    let mut word = String::new();
    let mut colored = false;
    for (c, shade) in chars {
        if c.is_whitespace() {
            if !word.is_empty() {
                words.push((std::mem::take(&mut word), colored));
            }
            colored = false;
        } else {
            word.push(c);
            colored |= shade != own;
        }
    }
    // Colored words next to each other are one detail.
    let mut found: Vec<String> = Vec::new();
    let mut phrase: Vec<&str> = Vec::new();
    for (i, (word, colored)) in words.iter().enumerate() {
        if *colored {
            phrase.push(word);
        }
        if !*colored || i + 1 == words.len() {
            let detail = phrase.join(" ");
            let detail = detail.trim_matches(|c: char| !c.is_alphanumeric());
            if !detail.is_empty() && !found.iter().any(|f| names(f, detail)) {
                found.push(detail.to_string());
            }
            phrase.clear();
        }
    }
    found
}

/// Whether `detail` is said already by `found`: the same words, or some
/// of them ("lever" after "rusty lever").
fn names(found: &str, detail: &str) -> bool {
    let found: Vec<String> = found.split_whitespace().map(str::to_lowercase).collect();
    let detail: Vec<String> = detail.split_whitespace().map(str::to_lowercase).collect();
    found.windows(detail.len()).any(|w| w == detail.as_slice())
}

/// The room in a read: its description and its details. `room` is the
/// current room's name, if the map knows it.
fn find(lines: &[Line], kinds: &[LineKind], room: Option<&str>) -> Option<Hidden> {
    let text = |i: usize| squeeze(&lines[i].iter().map(|s| s.text.as_str()).collect::<String>());
    let ours = |i: usize| matches!(kinds.get(i), Some(LineKind::Game | LineKind::Combat) | None);
    let room = room.map(squeeze).filter(|r| !r.is_empty()).map(|r| r.to_lowercase());
    let title = (0..lines.len()).find(|&i| {
        let line = text(i).to_lowercase();
        ours(i)
            && !line.is_empty()
            && match &room {
                Some(room) => line.starts_with(room.as_str()),
                None => !line.starts_with("you examine"),
            }
    })?;
    let first = title + 1;
    let end = (first..lines.len()).find(|&i| !ours(i) || text(i).is_empty()).unwrap_or(lines.len());
    if end == first {
        return None;
    }
    let text = (first..end).map(text).collect::<Vec<_>>().join(" ");
    Some(Hidden { line: first, lines: end - first, text, words: details(&lines[first..end]) })
}

impl LongLook {
    /// A line the player sent.
    pub fn typed(&mut self, line: &str, now: Instant) {
        if long_look(line) {
            self.asked = Some(now);
        }
    }

    /// The connection or the character changed: nothing is waiting.
    pub fn clear(&mut self) {
        self.asked = None;
    }

    /// The long look's room, once a read has it. `room` is the current
    /// room's name, if the map knows it.
    pub fn lines(&mut self, lines: &[Line], kinds: &[LineKind], room: Option<&str>, now: Instant) -> Option<Hidden> {
        let asked = self.asked?;
        if now.duration_since(asked) > WAITS {
            self.asked = None;
            return None;
        }
        let found = find(lines, kinds, room)?;
        self.asked = None;
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::{Span, Style};

    const WHITE: Option<Color> = Some(Color::Index { index: 15 });
    const CYAN: Option<Color> = Some(Color::Index { index: 14 });

    fn span(text: &str, fg: Option<Color>) -> Span {
        Span { text: text.into(), style: Style { fg, ..Style::default() } }
    }

    /// A line from `|`-marked text: what's between bars is highlighted.
    fn line(text: &str) -> Line {
        text.split('|').enumerate().filter(|(_, t)| !t.is_empty()).map(|(i, t)| span(t, if i % 2 == 1 { CYAN } else { WHITE })).collect()
    }

    fn look(lines: &[&str], room: Option<&str>) -> Option<Hidden> {
        let lines: Vec<Line> = lines.iter().map(|l| line(l)).collect();
        let kinds = vec![LineKind::Game; lines.len()];
        let mut look = LongLook::default();
        let now = Instant::now();
        look.typed("ll", now);
        look.lines(&lines, &kinds, room, now)
    }

    #[test]
    fn only_a_long_look_at_the_room_counts() {
        for line in ["ll", "LL", " examine ", "longlook", "llook", "exa", "exam", "id", "ll unobtrusively"] {
            assert!(long_look(line), "{line}");
        }
        for line in ["look", "ll sword", "examine me", "l", ""] {
            assert!(!long_look(line), "{line}");
        }
    }

    #[test]
    fn the_highlighted_words_are_the_details() {
        let found = look(
            &[
                "The Town Square",
                "A wide square, paved in cobbles. An old |marble| |fountain| stands",
                "at its middle, beside a |statue.|",
                "",
                "A rat is here.",
            ],
            Some("The Town Square"),
        )
        .unwrap();
        assert_eq!(found.line, 1);
        assert_eq!(found.lines, 2);
        assert_eq!(found.text, "A wide square, paved in cobbles. An old marble fountain stands at its middle, beside a statue.");
        assert_eq!(found.words, vec!["marble fountain", "statue"]);
    }

    #[test]
    fn a_detail_across_a_wrap_is_one_and_each_is_named_once() {
        let found = look(&["Hall", "A |rusty|", "|lever| sticks out. Pull the |lever?|", ""], Some("Hall")).unwrap();
        assert_eq!(found.words, vec!["rusty lever"]);
        let found = look(&["Hall", "A stone hall. The |lever| is old, and the |LEVER.| is stuck.", ""], Some("Hall")).unwrap();
        assert_eq!(found.words, vec!["lever"]);
    }

    #[test]
    fn nothing_in_color_is_no_details() {
        let found = look(&["Hall", "A bare hall.", ""], Some("Hall")).unwrap();
        assert_eq!(found.text, "A bare hall.");
        assert!(found.words.is_empty());
    }

    #[test]
    fn any_colors_the_player_picked_work() {
        // HIGHLIGHT in white on a cyan ROOMDESC: the description's own is the most of it.
        let lines = vec![line("Hall"), vec![span("A long hall with a ", CYAN), span("tapestry", WHITE), span(" on the wall.", CYAN)]];
        let mut look = LongLook::default();
        let now = Instant::now();
        look.typed("examine", now);
        let found = look.lines(&lines, &[LineKind::Game, LineKind::Game], Some("Hall"), now).unwrap();
        assert_eq!(found.words, vec!["tapestry"]);
    }

    #[test]
    fn the_title_is_found_by_the_rooms_name_or_comes_first() {
        let found = look(&["A rat arrives from the north.", "The Square (lit)", "A |well| here.", ""], Some("The Square")).unwrap();
        assert_eq!((found.line, found.words.clone()), (2, vec!["well".to_string()]));
        // Without a map, the first line but the game's "You examine".
        let found = look(&["You examine around carefully.", "The Square", "A |well| here.", ""], None).unwrap();
        assert_eq!(found.line, 2);
    }

    #[test]
    fn a_read_without_the_room_keeps_waiting() {
        let mut look = LongLook::default();
        let now = Instant::now();
        look.typed("ll", now);
        let rat = vec![line("A rat arrives.")];
        assert_eq!(look.lines(&rat, &[LineKind::Game], Some("Hall"), now), None);
        let room = vec![line("Hall"), line("A |bell| hangs."), line("")];
        assert!(look.lines(&room, &[LineKind::Game; 3], Some("Hall"), now).is_some());
        // Answered: the next look at the room is no one's.
        assert_eq!(look.lines(&room, &[LineKind::Game; 3], Some("Hall"), now), None);
    }

    #[test]
    fn nothing_unasked_late_or_in_talk() {
        let room = vec![line("Hall"), line("A |bell| hangs."), line("")];
        let mut look = LongLook::default();
        let now = Instant::now();
        assert_eq!(look.lines(&room, &[LineKind::Game; 3], Some("Hall"), now), None);
        look.typed("look", now);
        assert_eq!(look.lines(&room, &[LineKind::Game; 3], Some("Hall"), now), None);
        look.typed("ll", now);
        assert_eq!(look.lines(&room, &[LineKind::Game; 3], Some("Hall"), now + WAITS + Duration::from_secs(1)), None);
        look.typed("ll", now);
        assert_eq!(look.lines(&room, &[LineKind::Talk, LineKind::Talk, LineKind::Game], Some("Hall"), now), None);
    }
}

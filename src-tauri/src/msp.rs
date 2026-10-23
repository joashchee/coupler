//! MSP, the MUD Sound Protocol: CoffeeMUD's sounds, each named in the
//! text where it happens. Pure, so every rule here is unit-tested, and
//! the web build shares it.
//!
//! With MSP accepted (`telnet.rs`) and the character's `SOUNDS` on,
//! `CMProtocols.msp` puts ` !!SOUND(fireball.wav V=50 P=33 U=…) ` into
//! the message where the sound belongs, a space either side; CoffeeFilter
//! keeps it only when the player can hear its source, and never wraps a
//! line inside one. `take` cuts each out of a finished line and gives it
//! back as a `Sound`, which `session.rs` records as the hook pair
//! `msp.sound` = `"fireball.wav"`: the player's trigger on it plays, or
//! else the sound coffeemud.sf2 has for that name (`soundbank::by_msp`).
//! `V` is always 50 and `P` a die roll (`msp(name, priority)`), so
//! neither is kept. `U=` is where the game would have the client
//! download the file from: nothing is ever fetched (rule 2).
//! `!!MUSIC(…)` is cut out and recorded the same way (`msp.music`),
//! though stock CoffeeMUD never sends it; `Off`, MSP's word for "stop",
//! is cut out and not recorded.

use std::ops::Range;

use crate::ansi::Line;

/// The hook key a sound is recorded under.
pub const SOUND_KEY: &str = "msp.sound";
/// The hook key music is recorded under.
pub const MUSIC_KEY: &str = "msp.music";

/// What's said when the game turns its sounds off at login: the
/// character's `SOUNDS` is off (`CharCreation`'s login check). It's set
/// only for a character made in a client that accepted MSP, and Coupler
/// leaves turning it on to the player.
pub const SOUNDS_OFF: &str = "This character has CoffeeMUD's sounds off. Type SOUNDS to hear them.";

const SOUND: &str = "!!SOUND(";
const MUSIC: &str = "!!MUSIC(";

/// One sound the game named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    /// `!!MUSIC`, rather than `!!SOUND`.
    pub music: bool,
    /// The file it names, in lower case: `fireball.wav`.
    pub name: String,
}

impl Sound {
    /// The hook key it's recorded under.
    pub fn key(&self) -> &'static str {
        if self.music {
            MUSIC_KEY
        } else {
            SOUND_KEY
        }
    }
}

/// Where a token starts at `at` in `text`, whether it's music.
fn starts(text: &str, at: usize) -> Option<bool> {
    let rest = text.get(at..)?;
    let head = rest.get(..SOUND.len())?;
    if head.eq_ignore_ascii_case(SOUND) {
        Some(false)
    } else if head.eq_ignore_ascii_case(MUSIC) {
        Some(true)
    } else {
        None
    }
}

/// The first token in `text`: its byte range, padding included, and
/// what it names (None for `Off`, or a token with no name).
fn first(text: &str) -> Option<(Range<usize>, Option<Sound>)> {
    let bytes = text.as_bytes();
    let mut at = 0;
    loop {
        let i = text[at..].find("!!")? + at;
        let Some(music) = starts(text, i) else {
            at = i + 1;
            continue;
        };
        let open = i + SOUND.len();
        let close = text[open..].find(')')? + open;
        let name = text[open..close].split_whitespace().next().unwrap_or_default().to_ascii_lowercase();
        let sound = (!name.is_empty() && name != "off").then_some(Sound { music, name });
        // The padding msp() puts round it: a space each side, but a space
        // kept between words that were on either side of it.
        let space = |at: usize| bytes.get(at) == Some(&b' ');
        let mut start = i;
        let mut end = close + 1;
        if start > 0 && space(start - 1) {
            start -= 1;
        }
        if space(end) {
            end += 1;
        }
        let words = !text[..start].trim().is_empty() && !text[end..].trim().is_empty();
        let joined = (start > 0 && space(start - 1)) || space(end);
        if words && !joined && (start < i || end > close + 1) {
            if end > close + 1 {
                end -= 1;
            } else {
                start += 1;
            }
        }
        return Some((start..end, sound));
    }
}

/// Deletes a byte range of the line's text from its spans, dropping any
/// span left empty.
fn cut(line: &mut Line, range: Range<usize>) {
    let mut offset = 0;
    for span in line.iter_mut() {
        let len = span.text.len();
        let (a, b) = (range.start.saturating_sub(offset).min(len), range.end.saturating_sub(offset).min(len));
        offset += len;
        if a < b {
            span.text.replace_range(a..b, "");
        }
    }
    line.retain(|span| !span.text.is_empty());
}

fn plain(line: &Line) -> String {
    line.iter().map(|span| span.text.as_str()).collect()
}

/// Cuts every sound out of a finished line, and gives them back in order.
pub fn take(line: &mut Line) -> Vec<Sound> {
    let mut sounds = Vec::new();
    // One at a time, so the padding of two side by side comes out right.
    while let Some((range, sound)) = Some(plain(line)).filter(|t| t.contains("!!")).and_then(|t| first(&t)) {
        cut(line, range);
        sounds.extend(sound);
    }
    sounds
}

/// The unfinished line, for showing: its whole sounds cut out (they're
/// taken once the line is finished), and a sound still coming in at its
/// end hidden until the rest of it arrives.
pub fn tidy(line: &mut Line) {
    take(line);
    let text = plain(line);
    // A token begun but not closed, or the start of one at the very end
    // (`!!SOU`; `!!` alone could as well be the game's own).
    let Some(i) = text.rfind("!!") else { return };
    let rest = &text[i..];
    let begun = starts(&text, i).is_some() && !rest.contains(')');
    let partway = (3..SOUND.len()).contains(&rest.len()) && (SOUND[..rest.len()].eq_ignore_ascii_case(rest) || MUSIC[..rest.len()].eq_ignore_ascii_case(rest));
    if begun || partway {
        let start = if i > 0 && text.as_bytes()[i - 1] == b' ' { i - 1 } else { i };
        cut(line, start..text.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::{Screen, Span, Style};

    fn line(text: &str) -> Line {
        vec![Span { text: text.into(), style: Style::default() }]
    }

    fn sound(name: &str) -> Sound {
        Sound { music: false, name: name.into() }
    }

    #[test]
    fn a_sound_at_the_end_is_taken_with_its_padding() {
        let mut l = line("You are struck by a fireball! !!SOUND(fireball.wav V=50 P=33 U=http://localhost:27744/sounds/) ");
        assert_eq!(take(&mut l), vec![sound("fireball.wav")]);
        assert_eq!(plain(&l), "You are struck by a fireball!");
    }

    #[test]
    fn a_sound_at_the_start_or_between_words() {
        let mut l = line(" !!SOUND(dooropen.wav V=50 P=12) Bob opens the door.");
        assert_eq!(take(&mut l), vec![sound("dooropen.wav")]);
        assert_eq!(plain(&l), "Bob opens the door.");
        let mut l = line("Bob !!SOUND(laugh.wav V=50 P=20) laughs.");
        assert_eq!(take(&mut l), vec![sound("laugh.wav")]);
        assert_eq!(plain(&l), "Bob laughs.");
        let mut l = line("Bob!!SOUND(laugh.wav V=50 P=20)laughs.");
        assert_eq!(take(&mut l), vec![sound("laugh.wav")]);
        assert_eq!(plain(&l), "Boblaughs.");
    }

    #[test]
    fn several_in_a_line_any_case_and_music() {
        let mut l = line("Boom! !!sound(Explode.WAV V=50 P=3)  !!MUSIC(theme.mid L=1 V=50) Quiet.");
        assert_eq!(take(&mut l), vec![sound("explode.wav"), Sound { music: true, name: "theme.mid".into() }]);
        assert_eq!(plain(&l), "Boom! Quiet.");
    }

    #[test]
    fn off_and_empty_are_cut_but_not_kept() {
        let mut l = line("Silence. !!SOUND(Off) !!MUSIC() ");
        assert!(take(&mut l).is_empty());
        assert_eq!(plain(&l), "Silence.");
    }

    #[test]
    fn a_line_that_is_only_a_sound_is_left_empty() {
        let mut l = line(" !!SOUND(rain.wav V=50 P=40) ");
        assert_eq!(take(&mut l), vec![sound("rain.wav")]);
        assert!(l.is_empty());
    }

    #[test]
    fn plain_text_is_untouched() {
        for text in ["Hello!! How are you?", "!!SOUND without a bracket", "Wow!!", "!!SOUND(never closed"] {
            let mut l = line(text);
            assert!(take(&mut l).is_empty());
            assert_eq!(plain(&l), text);
        }
    }

    #[test]
    fn colors_either_side_are_kept() {
        let red = Style { fg: Some(crate::ansi::Color::Index { index: 1 }), ..Style::default() };
        let mut l = vec![Span { text: "The orc ".into(), style: Style::default() }, Span { text: "hits".into(), style: red }, Span { text: " you. !!SOUND(bashed1.wav V=50 P=31) ".into(), style: Style::default() }];
        assert_eq!(take(&mut l), vec![sound("bashed1.wav")]);
        assert_eq!(l.len(), 3);
        assert_eq!(l[1].style, red);
        assert_eq!(plain(&l), "The orc hits you.");
    }

    #[test]
    fn a_sound_across_spans() {
        let mut l = vec![Span { text: "Zap !!SOUND(light".into(), style: Style::default() }, Span { text: "ning.wav V=50 P=2) done".into(), style: Style { bold: true, ..Style::default() } }];
        assert_eq!(take(&mut l), vec![sound("lightning.wav")]);
        assert_eq!(plain(&l), "Zap done");
    }

    #[test]
    fn the_unfinished_line_hides_a_sound_still_coming() {
        let mut l = line("<20Hp 30m> You swing. !!SOUND(bash");
        tidy(&mut l);
        assert_eq!(plain(&l), "<20Hp 30m> You swing.");
        let mut l = line("You swing. !!SO");
        tidy(&mut l);
        assert_eq!(plain(&l), "You swing.");
        let mut l = line("A prompt!! ");
        tidy(&mut l);
        assert_eq!(plain(&l), "A prompt!! ");
        let mut l = line("Bang !!SOUND(explode.wav V=50 P=1) <prompt> ");
        tidy(&mut l);
        assert_eq!(plain(&l), "Bang <prompt> ");
    }

    #[test]
    fn from_the_game_bytes_split_across_reads() {
        let mut screen = Screen::default();
        assert!(screen.push(b"You open the door. !!SOUND(door").is_empty());
        let mut partial = screen.partial().unwrap();
        tidy(&mut partial);
        assert_eq!(plain(&partial), "You open the door.");
        let mut done = screen.push(b"open.wav V=50 P=7) \n\r");
        assert_eq!(take(&mut done[0]), vec![sound("dooropen.wav")]);
        assert_eq!(plain(&done[0]), "You open the door.");
    }
}

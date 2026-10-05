//! Game text to styled lines: decodes UTF-8 (holding back a character
//! split across reads), or Latin-1 where the bytes aren't UTF-8 (the
//! stock CoffeeMUD's `CHARSETOUTPUT=iso-8859-1`), follows ANSI SGR colors (16, 256 and 24-bit, as
//! CoffeeMUD sends with ANSI-TRUECOLOR), and cuts the text into lines of
//! same-style spans for the terminal view.
//!
//! Colors stay as palette indexes or RGB here: which RGB an index means
//! (and bold-as-bright) is the view's call, so a theme can change it.
//! Other escape sequences (cursor moves, MXP's line-mode codes) are
//! dropped, as Coupler draws a scrolling log, not a screen. MCP's
//! out-of-band lines (`#$#…`) are dropped too (`mcp_out_of_band`).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Color {
    /// 0–255: the 16 ANSI colors, then the 256-color cube and grays.
    Index { index: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<Color>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub inverse: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Span {
    pub text: String,
    #[serde(flatten)]
    pub style: Style,
}

pub type Line = Vec<Span>;

#[derive(Default, Clone, Copy, PartialEq)]
enum Esc {
    #[default]
    None,
    /// Saw ESC.
    Start,
    /// Inside ESC [ … collecting parameters.
    Csi,
}

#[derive(Default)]
pub struct Screen {
    style: Style,
    esc: Esc,
    params: String,
    /// Bytes of a UTF-8 character still waiting for the rest of it.
    pending: Vec<u8>,
    /// The line being built: finished spans plus the current text.
    line: Line,
    text: String,
    /// The last character was a CR or LF, for folding CR LF and LF CR.
    last_break: Option<char>,
}

impl Screen {
    /// Adds game text. Returns the lines it finished.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Line> {
        let mut data = std::mem::take(&mut self.pending);
        data.extend_from_slice(bytes);
        let mut done = Vec::new();
        let mut rest = &data[..];
        while !rest.is_empty() {
            match std::str::from_utf8(rest) {
                Ok(s) => {
                    self.chars(s, &mut done);
                    break;
                }
                Err(e) => {
                    let (good, bad) = rest.split_at(e.valid_up_to());
                    self.chars(std::str::from_utf8(good).expect("checked"), &mut done);
                    match e.error_len() {
                        // Cut off mid-character: wait for the next read.
                        None => {
                            self.pending = bad.to_vec();
                            break;
                        }
                        // Not UTF-8: Latin-1, CoffeeMUD's stock character set.
                        Some(n) => {
                            let latin1: String = bad[..n].iter().map(|&b| latin1(b)).collect();
                            self.chars(&latin1, &mut done);
                            rest = &bad[n..];
                        }
                    }
                }
            }
        }
        done
    }

    /// The unfinished line (usually a prompt), without taking it.
    ///
    /// A Latin-1 letter at the very end of a read (one that could start
    /// a UTF-8 character) shows only once the next read arrives.
    pub fn partial(&self) -> Option<Line> {
        let mut line = self.line.clone();
        if !self.text.is_empty() {
            line.push(Span { text: self.text.clone(), style: self.style });
        }
        (!line.is_empty() && !line[0].text.starts_with(MCP_LINE)).then_some(line)
    }

    fn chars(&mut self, s: &str, done: &mut Vec<Line>) {
        for c in s.chars() {
            match self.esc {
                Esc::Start => {
                    self.esc = if c == '[' { Esc::Csi } else { Esc::None };
                    self.params.clear();
                    continue;
                }
                Esc::Csi => {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        self.esc = Esc::None;
                        if c == 'm' {
                            self.sgr();
                        }
                    } else {
                        self.params.push(c);
                    }
                    continue;
                }
                Esc::None => {}
            }
            match c {
                '\x1b' => self.esc = Esc::Start,
                '\r' | '\n' => {
                    // CR LF or LF CR is one break; CR CR or LF LF is two.
                    match self.last_break.take() {
                        Some(prev) if prev != c => {}
                        _ => {
                            let mut line = self.finish_line();
                            if !mcp_out_of_band(&mut line) {
                                done.push(line);
                            }
                            self.last_break = Some(c);
                        }
                    }
                    continue;
                }
                '\t' => self.text.push_str("    "),
                c if c.is_control() => {} // BEL and friends: nothing to draw
                c => self.text.push(c),
            }
            self.last_break = None;
        }
    }

    fn flush_span(&mut self) {
        if !self.text.is_empty() {
            let text = std::mem::take(&mut self.text);
            self.line.push(Span { text, style: self.style });
        }
    }

    fn finish_line(&mut self) -> Line {
        self.flush_span();
        std::mem::take(&mut self.line)
    }

    fn sgr(&mut self) {
        let mut s = self.style;
        apply_sgr(&mut s, &self.params);
        if s != self.style {
            self.flush_span();
            self.style = s;
        }
    }
}

/// MCP 2.1's out-of-band prefix. CoffeeMUD starts every connection with
/// `#$#mcp version: 2.1 to: 2.1` as a line of text (`DefaultSession`,
/// unless `DISABLE=MCP`), and its builder editor can send more. Coupler
/// doesn't speak MCP (GMCP's `Siplet.Input` does the same), so these
/// lines are dropped rather than shown or read aloud.
const MCP_LINE: &str = "#$#";
/// MCP's quote: a game line that really starts with `#$#` is sent as
/// `#$"#$#…`, and the quote is taken off.
const MCP_QUOTE: &str = "#$\"";

/// A byte that isn't UTF-8, read as ISO-8859-1. 0x80 to 0x9F are
/// controls there, not letters, so they show as the replacement character.
fn latin1(b: u8) -> char {
    if (0x80..0xA0).contains(&b) {
        '\u{FFFD}'
    } else {
        char::from(b)
    }
}

/// Whether a finished line is MCP's (drop it). Unquotes a quoted one.
fn mcp_out_of_band(line: &mut Line) -> bool {
    let Some(first) = line.first_mut() else { return false };
    if first.text.starts_with(MCP_LINE) {
        return true;
    }
    if first.text.starts_with(MCP_QUOTE) {
        first.text.drain(..MCP_QUOTE.len());
        if first.text.is_empty() {
            line.remove(0);
        }
    }
    false
}

/// Applies an SGR sequence's parameters (what's between `ESC [` and
/// `m`) to a style. Also used for ANSI art (`ansi_art.rs`).
pub fn apply_sgr(style: &mut Style, params: &str) {
    let codes: Vec<u16> = if params.is_empty() { vec![0] } else { params.split([';', ':']).map(|p| p.parse().unwrap_or(0)).collect() };
    let s = style;
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            0 => *s = Style::default(),
            1 => s.bold = true,
            3 => s.italic = true,
            4 => s.underline = true,
            7 => s.inverse = true,
            22 => s.bold = false,
            23 => s.italic = false,
            24 => s.underline = false,
            27 => s.inverse = false,
            n @ 30..=37 => s.fg = Some(Color::Index { index: (n - 30) as u8 }),
            39 => s.fg = None,
            n @ 40..=47 => s.bg = Some(Color::Index { index: (n - 40) as u8 }),
            49 => s.bg = None,
            n @ 90..=97 => s.fg = Some(Color::Index { index: (n - 90 + 8) as u8 }),
            n @ 100..=107 => s.bg = Some(Color::Index { index: (n - 100 + 8) as u8 }),
            n @ (38 | 48) => {
                let (color, used) = extended(&codes[i + 1..]);
                i += used;
                if n == 38 {
                    s.fg = color.or(s.fg);
                } else {
                    s.bg = color.or(s.bg);
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// `5;n` or `2;r;g;b` after a 38/48. Returns the color and how many codes
/// it used.
fn extended(rest: &[u16]) -> (Option<Color>, usize) {
    let byte = |i: usize| rest.get(i).map(|&v| v.min(255) as u8);
    match rest.first() {
        Some(5) => (byte(1).map(|index| Color::Index { index }), 2.min(rest.len())),
        Some(2) => match (byte(1), byte(2), byte(3)) {
            (Some(r), Some(g), Some(b)) => (Some(Color::Rgb { r, g, b }), 4),
            _ => (None, rest.len()),
        },
        _ => (None, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(line: &Line) -> String {
        line.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn lines_and_breaks() {
        let mut s = Screen::default();
        let lines = s.push(b"one\r\ntwo\n\rthree\n\nfour");
        assert_eq!(lines.iter().map(plain).collect::<Vec<_>>(), ["one", "two", "three", ""]);
        assert_eq!(plain(&s.partial().unwrap()), "four");
    }

    #[test]
    fn colors_split_spans() {
        let mut s = Screen::default();
        let lines = s.push(b"\x1b[1;31mred\x1b[0m plain\r\n");
        assert_eq!(lines[0].len(), 2);
        assert_eq!(lines[0][0].style.fg, Some(Color::Index { index: 1 }));
        assert!(lines[0][0].style.bold);
        assert_eq!(lines[0][1].style, Style::default());
        assert_eq!(lines[0][1].text, " plain");
    }

    #[test]
    fn style_carries_to_the_next_line() {
        let mut s = Screen::default();
        let lines = s.push(b"\x1b[32mgreen\r\nstill\r\n");
        assert_eq!(lines[1][0].style.fg, Some(Color::Index { index: 2 }));
    }

    #[test]
    fn bright_256_and_truecolor() {
        let mut s = Screen::default();
        let l = &s.push(b"\x1b[93ma\x1b[38;5;208mb\x1b[48;2;1;2;3mc\n")[0];
        assert_eq!(l[0].style.fg, Some(Color::Index { index: 11 }));
        assert_eq!(l[1].style.fg, Some(Color::Index { index: 208 }));
        assert_eq!(l[2].style.bg, Some(Color::Rgb { r: 1, g: 2, b: 3 }));
        assert_eq!(l[2].style.fg, Some(Color::Index { index: 208 }));
    }

    #[test]
    fn escape_split_across_reads() {
        let mut s = Screen::default();
        s.push(b"a\x1b[3");
        let l = &s.push(b"4mb\n")[0];
        assert_eq!(l[1].style.fg, Some(Color::Index { index: 4 }));
        assert_eq!(plain(l), "ab");
    }

    #[test]
    fn utf8_split_across_reads() {
        let mut s = Screen::default();
        let bytes = "café\n".as_bytes();
        s.push(&bytes[..4]); // "caf" + first byte of é
        assert_eq!(plain(&s.push(&bytes[4..])[0]), "café");
    }

    #[test]
    fn latin1_when_not_utf8() {
        // CoffeeMUD's stock CHARSETOUTPUT is iso-8859-1: "café" and "naïve" as Latin-1.
        let mut s = Screen::default();
        let lines = s.push(b"caf\xe9\nna\xefve\n\xff\n");
        assert_eq!(plain(&lines[0]), "café");
        assert_eq!(plain(&lines[1]), "naïve");
        assert_eq!(plain(&lines[2]), "ÿ");
    }

    #[test]
    fn latin1_at_the_end_of_a_read_waits_for_the_next() {
        // 0xE9 starts a three-byte UTF-8 character, so it's held until the next read shows it isn't one.
        let mut s = Screen::default();
        assert!(s.push(b"caf\xe9").is_empty());
        assert_eq!(plain(&s.push(b"\n")[0]), "café");
    }

    #[test]
    fn utf8_still_wins_when_valid() {
        let mut s = Screen::default();
        assert_eq!(plain(&s.push("café ünd\n".as_bytes())[0]), "café ünd");
    }

    #[test]
    fn latin1_control_range_becomes_replacement_chars() {
        // 0x80 to 0x9F are controls in Latin-1, never letters.
        let mut s = Screen::default();
        assert_eq!(plain(&s.push(b"a\x85b\n")[0]), "a\u{FFFD}b");
    }

    #[test]
    fn other_escapes_and_controls_are_dropped() {
        let mut s = Screen::default();
        assert_eq!(plain(&s.push(b"\x1b[2J\x1b[1zhi\x07\n")[0]), "hi");
    }

    #[test]
    fn mcp_greeting_is_dropped() {
        // CoffeeMUD's first bytes on connecting (DefaultSession.java).
        let mut s = Screen::default();
        let lines = s.push(b"\n\r#$#mcp version: 2.1 to: 2.1\n\rConnecting to CoffeeMUD...\n\r");
        let lines: Vec<String> = lines.iter().map(plain).collect();
        assert_eq!(lines, ["", "Connecting to CoffeeMUD..."]);
    }

    #[test]
    fn mcp_line_split_across_reads_never_shows() {
        let mut s = Screen::default();
        assert!(s.push(b"#$#mcp vers").is_empty());
        assert_eq!(s.partial(), None);
        assert!(s.push(b"ion: 2.1 to: 2.1\r\n").is_empty());
        assert_eq!(plain(&s.push(b"hi\n")[0]), "hi");
    }

    #[test]
    fn mcp_quoted_line_is_unquoted() {
        let mut s = Screen::default();
        assert_eq!(plain(&s.push(b"#$\"#$# not MCP\n")[0]), "#$# not MCP");
        // Only at the start of a line.
        assert_eq!(plain(&s.push(b"say #$#\n")[0]), "say #$#");
    }
}

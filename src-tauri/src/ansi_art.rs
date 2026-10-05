//! ANSI art, for a trigger's picture (`assets.rs`, `hooks.rs`): a `.ans`
//! or `.asc` file drawn onto a grid of character cells, then cut into the
//! same styled lines the game output is made of (`ansi.rs`), so the view
//! draws it with the game's palette in the IBM VGA font.
//!
//! It takes both kinds of ANSI file:
//! - Classic art, as the BBS scene drew it: CP437 bytes, a canvas 80
//!   columns wide unless its SAUCE record says otherwise, cursor moves
//!   (up, down, left, right, to a row and column, save and restore), and
//!   iCE colors (blink meaning a bright background) when SAUCE asks.
//! - The kind Coupler reads from the game: UTF-8 text with SGR colors in
//!   16, 256 and 24-bit. A file that's valid UTF-8 and uses any character
//!   past ASCII is taken as UTF-8; anything else is CP437.
//!
//! The SAUCE record (and its comment block), and everything after the
//! end-of-file mark (Ctrl-Z), aren't part of the picture. Unit-tested.

use serde::Serialize;

use crate::ansi::{apply_sgr, Color, Line, Span, Style};

/// The widest and tallest a picture is drawn: more than the screen's
/// 160 by 45, so a big file is cut rather than refused.
const MOST_COLUMNS: usize = 400;
const MOST_ROWS: usize = 1000;
/// A canvas with no SAUCE width is the classic 80 columns.
const CLASSIC_COLUMNS: usize = 80;

/// CP437 as Unicode: the characters the IBM VGA font draws for each byte.
pub(crate) const CP437: &str = "\u{0}☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼ !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~⌂ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}";

/// A picture drawn from ANSI: its size in cells, and its rows.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Art {
    pub columns: usize,
    pub rows: usize,
    pub lines: Vec<Line>,
}

/// What a SAUCE record says that matters here.
#[derive(Debug, Default, PartialEq)]
struct Sauce {
    columns: Option<usize>,
    ice: bool,
}

/// Splits off a SAUCE record (the last 128 bytes, when they start
/// `SAUCE`) and its comment block, and cuts at the end-of-file mark.
fn content(bytes: &[u8]) -> (&[u8], Sauce) {
    let mut body = bytes;
    let mut sauce = Sauce::default();
    if bytes.len() >= 128 && bytes[bytes.len() - 128..].starts_with(b"SAUCE") {
        let record = &bytes[bytes.len() - 128..];
        body = &bytes[..bytes.len() - 128];
        // Character data (1) or a plain ANSI/ASCII file type: TInfo1 is the width.
        let (data_type, width) = (record[94], u16::from_le_bytes([record[96], record[97]]));
        if matches!(data_type, 1 | 5) && width > 0 {
            sauce.columns = Some(usize::from(width));
        }
        sauce.ice = record[105] & 1 == 1;
        let comments = usize::from(record[104]);
        let block = 5 + comments * 64;
        if comments > 0 && body.len() >= block && body[body.len() - block..].starts_with(b"COMNT") {
            body = &body[..body.len() - block];
        }
    }
    if let Some(end) = body.iter().position(|&b| b == 0x1a) {
        body = &body[..end];
    }
    (body, sauce)
}

/// The file's text as characters: UTF-8 if it's valid UTF-8 past ASCII,
/// else each byte as its CP437 character (the control codes excepted).
fn characters(body: &[u8]) -> Vec<char> {
    match std::str::from_utf8(body) {
        Ok(text) if !text.is_ascii() => text.chars().collect(),
        _ => {
            let table: Vec<char> = CP437.chars().collect();
            // ASCII as itself, and the control codes a file moves and
            // colors with; every other byte is its CP437 character.
            body.iter()
                .map(|&b| match b {
                    0x20..=0x7e | 0x00 | 0x07 | 0x08 | 0x09 | 0x0a | 0x0d | 0x1b => b as char,
                    _ => table[usize::from(b)],
                })
                .collect()
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Cell {
    c: char,
    style: Style,
}

const BLANK: Cell = Cell { c: ' ', style: Style { fg: None, bg: None, bold: false, italic: false, underline: false, inverse: false } };

struct Canvas {
    columns: usize,
    grid: Vec<Vec<Cell>>,
    x: usize,
    y: usize,
    saved: (usize, usize),
    style: Style,
    /// SGR 5, which in iCE colors makes the background bright.
    blink: bool,
    ice: bool,
}

impl Canvas {
    fn row(&mut self, y: usize) -> &mut Vec<Cell> {
        while self.grid.len() <= y {
            self.grid.push(vec![BLANK; self.columns]);
        }
        &mut self.grid[y]
    }

    fn put(&mut self, c: char) {
        if self.x >= self.columns {
            self.x = 0;
            self.y += 1;
        }
        if self.y >= MOST_ROWS {
            return;
        }
        let mut style = self.style;
        if self.ice && self.blink {
            if let Some(Color::Index { index }) = style.bg {
                if index < 8 {
                    style.bg = Some(Color::Index { index: index + 8 });
                }
            }
        }
        let (x, y) = (self.x, self.y);
        self.row(y)[x] = Cell { c, style };
        self.x += 1;
    }

    fn csi(&mut self, params: &str, command: char) {
        let numbers: Vec<usize> = params.split(';').map(|p| p.trim_start_matches('?').parse().unwrap_or(0)).collect();
        let n = numbers.first().copied().unwrap_or(0).max(1);
        match command {
            'A' => self.y = self.y.saturating_sub(n),
            'B' => self.y = (self.y + n).min(MOST_ROWS - 1),
            'C' => self.x = (self.x + n).min(self.columns),
            'D' => self.x = self.x.min(self.columns).saturating_sub(n),
            'H' | 'f' => {
                self.y = numbers.first().copied().unwrap_or(1).max(1) - 1;
                self.x = (numbers.get(1).copied().unwrap_or(1).max(1) - 1).min(self.columns - 1);
                self.y = self.y.min(MOST_ROWS - 1);
            }
            's' => self.saved = (self.x, self.y),
            'u' => (self.x, self.y) = self.saved,
            'J' if numbers.first() == Some(&2) => {
                self.grid.clear();
                (self.x, self.y) = (0, 0);
            }
            'K' => {
                let (x, y, columns) = (self.x, self.y, self.columns);
                let row = self.row(y);
                row[x.min(columns)..].fill(BLANK);
            }
            'm' => {
                apply_sgr(&mut self.style, params);
                let codes: Vec<&str> = params.split([';', ':']).collect();
                if params.is_empty() || codes.contains(&"0") {
                    self.blink = false;
                }
                if codes.contains(&"5") {
                    self.blink = true;
                }
                if codes.contains(&"25") {
                    self.blink = false;
                }
            }
            _ => {}
        }
    }
}

/// Draws an ANSI or ASCII art file.
pub fn render(bytes: &[u8]) -> Art {
    let (body, sauce) = content(bytes);
    let columns = sauce.columns.unwrap_or(CLASSIC_COLUMNS).clamp(1, MOST_COLUMNS);
    let mut canvas = Canvas { columns, grid: Vec::new(), x: 0, y: 0, saved: (0, 0), style: Style::default(), blink: false, ice: sauce.ice };
    let text = characters(body);
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        i += 1;
        match c {
            '\x1b' if text.get(i) == Some(&'[') => {
                i += 1;
                let start = i;
                while i < text.len() && !('\u{40}'..='\u{7e}').contains(&text[i]) {
                    i += 1;
                }
                if let Some(&command) = text.get(i) {
                    let params: String = text[start..i].iter().collect();
                    canvas.csi(&params, command);
                    i += 1;
                }
            }
            '\x1b' => {}
            '\r' => canvas.x = 0,
            '\n' => {
                canvas.x = 0;
                canvas.y = (canvas.y + 1).min(MOST_ROWS - 1);
                canvas.row(canvas.y);
            }
            '\t' => canvas.x = ((canvas.x / 8) + 1) * 8,
            '\u{8}' => canvas.x = canvas.x.saturating_sub(1),
            // BEL rings nothing, and NUL is an empty cell.
            '\u{7}' => {}
            '\u{0}' => canvas.put(' '),
            c => canvas.put(c),
        }
    }
    // A last line break doesn't make a row of its own.
    while canvas.grid.len() > 1 && canvas.grid.last().is_some_and(|row| row.iter().all(|cell| *cell == BLANK)) {
        canvas.grid.pop();
    }
    let lines: Vec<Line> = canvas.grid.iter().map(|row| spans(row)).collect();
    Art { columns, rows: lines.len(), lines }
}

/// A row of cells as runs of one style.
fn spans(row: &[Cell]) -> Line {
    let mut line: Line = Vec::new();
    for cell in row {
        match line.last_mut() {
            Some(span) if span.style == cell.style => span.text.push(cell.c),
            _ => line.push(Span { text: cell.c.to_string(), style: cell.style }),
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(art: &Art) -> Vec<String> {
        art.lines.iter().map(|l| l.iter().map(|s| s.text.as_str()).collect::<String>().trim_end().to_string()).collect()
    }

    fn sauce(columns: u16, ice: bool) -> Vec<u8> {
        let mut record = vec![0u8; 128];
        record[..7].copy_from_slice(b"SAUCE00");
        record[94] = 1;
        record[95] = 1;
        record[96..98].copy_from_slice(&columns.to_le_bytes());
        record[105] = u8::from(ice);
        record
    }

    #[test]
    fn the_table_has_a_character_per_byte() {
        assert_eq!(CP437.chars().count(), 256);
    }

    #[test]
    fn cp437_bytes_are_the_ibm_characters() {
        let art = render(&[0xdb, 0xb0, b'A', 0xc9, 0xcd, 0xbb]);
        assert_eq!(text(&art), vec!["█░A╔═╗"]);
        assert_eq!(art.columns, 80);
    }

    #[test]
    fn utf8_with_truecolor_is_read_as_the_game_sends_it() {
        let art = render("\x1b[38;2;255;0;0m█▀\x1b[0mok".as_bytes());
        assert_eq!(text(&art), vec!["█▀ok"]);
        assert_eq!(art.lines[0][0].style.fg, Some(Color::Rgb { r: 255, g: 0, b: 0 }));
        assert_eq!(art.lines[0][1].style.fg, None);
    }

    #[test]
    fn lines_wrap_at_the_canvas_width_and_cursor_moves_place_text() {
        let mut bytes = b"abcd\x1b[2Cx\r\n\x1b[3;2Hy".to_vec();
        bytes.extend(sauce(4, false));
        let art = render(&bytes);
        assert_eq!(art.columns, 4);
        // "abcd" fills row 1; the move forward goes past the edge, so x wraps to row 2.
        assert_eq!(text(&art), vec!["abcd", "x", " y"]);
    }

    #[test]
    fn sauce_and_what_follows_ctrl_z_are_not_drawn() {
        let mut bytes = b"hi\x1agarbage".to_vec();
        bytes.extend(sauce(0, false));
        assert_eq!(text(&render(&bytes)), vec!["hi"]);
    }

    #[test]
    fn ice_colors_make_blink_a_bright_background() {
        let mut bytes = b"\x1b[5;44mA".to_vec();
        bytes.extend(sauce(80, true));
        assert_eq!(render(&bytes).lines[0][0].style.bg, Some(Color::Index { index: 12 }));
        assert_eq!(render(b"\x1b[5;44mA").lines[0][0].style.bg, Some(Color::Index { index: 4 }));
    }

    #[test]
    fn a_last_line_break_adds_no_row() {
        assert_eq!(render(b"one\r\ntwo\r\n").rows, 2);
    }
}

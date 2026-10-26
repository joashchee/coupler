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
//!
//! **ANSImations** (`show`): a file that draws over what it drew (moves
//! the cursor back and changes a cell, or clears the screen once
//! something's on it) was made to be watched arriving over a modem.
//! It's played as the BBS showed it, at 14,400 bps, in steps of the rows
//! each tenth of a second changed; a long one is sped up to fit two
//! minutes. A step that changes a quarter of the picture's look is a
//! flash, and flashes are held back to at most three a second (as The
//! Fight's are, `fight_art.rs`). A file that only draws forward is a
//! still picture, as before. The frontend plays the steps and shows the
//! finished picture at once with reduced motion (`AnsiPicture.tsx`).

use std::collections::BTreeSet;

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

/// How fast an ANSImation arrives: 14,400 bps, ten bits a character.
const CHARACTERS_A_SECOND: usize = 1440;
/// How often a step is shown, in ms.
const TICK_MS: u32 = 100;
/// The most steps (two minutes of them): a longer file comes faster.
const MOST_STEPS: usize = 1200;
/// The soonest a flash follows another: never more than three a second.
const FLASH_GAP_MS: u32 = 334;

/// One step of an ANSImation: the rows that changed, `at` ms from the
/// start, on a blank picture when `clear` (the screen was cleared).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Step {
    pub at: u32,
    pub clear: bool,
    /// Each changed row's index and its whole row.
    pub rows: Vec<(usize, Line)>,
}

/// An ANSI art file as the screen shows it: the finished picture, and
/// its steps when it's an ANSImation (none for a still picture).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Shown {
    #[serde(flatten)]
    pub art: Art,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub play: Vec<Step>,
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
    /// Whether it ever drew over what it had drawn: an ANSImation.
    over: bool,
    /// The rows changed since the last step, and whether the screen was cleared.
    dirty: BTreeSet<usize>,
    cleared: bool,
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
        let cell = Cell { c, style };
        let was = std::mem::replace(&mut self.row(y)[x], cell);
        if was != BLANK && was != cell {
            self.over = true;
        }
        self.dirty.insert(y);
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
                if self.grid.iter().flatten().any(|cell| *cell != BLANK) {
                    self.over = true;
                }
                self.grid.clear();
                self.dirty.clear();
                self.cleared = true;
                (self.x, self.y) = (0, 0);
            }
            'K' => {
                let (x, y, columns) = (self.x, self.y, self.columns);
                let row = &mut self.row(y)[x.min(columns)..];
                if row.iter().any(|cell| *cell != BLANK) {
                    row.fill(BLANK);
                    self.over = true;
                    self.dirty.insert(y);
                }
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

    fn new(sauce: &Sauce) -> Canvas {
        let columns = sauce.columns.unwrap_or(CLASSIC_COLUMNS).clamp(1, MOST_COLUMNS);
        Canvas { columns, grid: Vec::new(), x: 0, y: 0, saved: (0, 0), style: Style::default(), blink: false, ice: sauce.ice, over: false, dirty: BTreeSet::new(), cleared: false }
    }

    /// Draws what starts at `text[i]` (a character, or a whole escape
    /// sequence), and says where the next starts.
    fn feed(&mut self, text: &[char], mut i: usize) -> usize {
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
                    self.csi(&params, command);
                    i += 1;
                }
            }
            '\x1b' => {}
            '\r' => self.x = 0,
            '\n' => {
                self.x = 0;
                self.y = (self.y + 1).min(MOST_ROWS - 1);
                self.row(self.y);
            }
            '\t' => self.x = ((self.x / 8) + 1) * 8,
            '\u{8}' => self.x = self.x.saturating_sub(1),
            // BEL rings nothing, and NUL is an empty cell.
            '\u{7}' => {}
            '\u{0}' => self.put(' '),
            c => self.put(c),
        }
        i
    }

    /// The picture as it stands.
    fn art(&self) -> Art {
        let mut grid = self.grid.as_slice();
        // A last line break doesn't make a row of its own.
        while grid.len() > 1 && grid.last().is_some_and(|row| row.iter().all(|cell| *cell == BLANK)) {
            grid = &grid[..grid.len() - 1];
        }
        let lines: Vec<Line> = grid.iter().map(|row| spans(row)).collect();
        Art { columns: self.columns, rows: lines.len(), lines }
    }

    /// The rows changed since the last step, as a step `at` ms; `seen` is
    /// what the last steps left on the screen. With how many cells
    /// changed their look. None when nothing changed.
    fn step(&mut self, at: u32, seen: &mut Vec<Vec<Cell>>) -> Option<(Step, usize)> {
        let clear = std::mem::take(&mut self.cleared);
        let mut looks = 0;
        if clear {
            looks += seen.iter().flatten().filter(|cell| look(cell) != look(&BLANK)).count();
            seen.clear();
        }
        let mut rows = Vec::new();
        for y in std::mem::take(&mut self.dirty) {
            let Some(row) = self.grid.get(y) else { continue };
            while seen.len() <= y {
                seen.push(vec![BLANK; self.columns]);
            }
            looks += row.iter().zip(&seen[y]).filter(|(now, was)| look(now) != look(was)).count();
            seen[y].clone_from(row);
            rows.push((y, spans(row)));
        }
        (clear || !rows.is_empty()).then_some((Step { at, clear, rows }, looks))
    }
}

/// What a cell looks like from afar: its background, and its color
/// where there's a mark.
fn look(cell: &Cell) -> (Option<Color>, Option<Color>, bool) {
    let mark = !matches!(cell.c, ' ' | '\u{a0}');
    (cell.style.bg, if mark { cell.style.fg } else { None }, cell.style.inverse)
}

/// Draws an ANSI or ASCII art file.
pub fn render(bytes: &[u8]) -> Art {
    let (body, sauce) = content(bytes);
    let mut canvas = Canvas::new(&sauce);
    let text = characters(body);
    let mut i = 0;
    while i < text.len() {
        i = canvas.feed(&text, i);
    }
    canvas.art()
}

/// An ANSI art file to show: the finished picture, and its steps if
/// it's an ANSImation.
pub fn show(bytes: &[u8]) -> Shown {
    let (body, sauce) = content(bytes);
    let text = characters(body);
    let mut canvas = Canvas::new(&sauce);
    let mut i = 0;
    while i < text.len() {
        i = canvas.feed(&text, i);
    }
    let art = canvas.art();
    if !canvas.over {
        return Shown { art, play: Vec::new() };
    }
    // Again, a step each tick; a flash is a quarter of the picture's cells changing their look.
    let a_tick = (CHARACTERS_A_SECOND * TICK_MS as usize / 1000).max(text.len().div_ceil(MOST_STEPS));
    let area = (art.columns * art.rows).max(1);
    let mut canvas = Canvas::new(&sauce);
    let (mut seen, mut play) = (Vec::new(), Vec::new());
    let (mut i, mut tick, mut held, mut last_flash) = (0, 0u32, 0u32, None::<u32>);
    while i < text.len() {
        let end = (i + a_tick).min(text.len());
        while i < end {
            i = canvas.feed(&text, i);
        }
        tick += 1;
        if let Some((mut step, looks)) = canvas.step(tick * TICK_MS + held, &mut seen) {
            if looks * 4 >= area {
                if let Some(last) = last_flash.filter(|last| step.at < last + FLASH_GAP_MS) {
                    held += last + FLASH_GAP_MS - step.at;
                    step.at = last + FLASH_GAP_MS;
                }
                last_flash = Some(step.at);
            }
            play.push(step);
        }
    }
    Shown { art, play }
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

    /// `n` bells: time passing in a file, nothing drawn.
    fn wait(n: usize) -> String {
        "\x07".repeat(n)
    }

    fn row_text(line: &Line) -> String {
        line.iter().map(|s| s.text.as_str()).collect::<String>().trim_end().to_string()
    }

    #[test]
    fn a_still_picture_has_no_steps() {
        let mut bytes = b"\x1b[2J\x1b[1;31m\xdb\xdb\r\nhi\x1b[K\r\n".to_vec();
        bytes.extend(sauce(80, false));
        let shown = show(&bytes);
        assert!(shown.play.is_empty());
        assert_eq!(shown.art, render(&bytes));
        assert_eq!(serde_json::to_value(&shown).unwrap(), serde_json::to_value(render(&bytes)).unwrap());
    }

    #[test]
    fn drawing_over_itself_is_played_at_the_modems_pace() {
        let bytes = format!("A{}\x1b[HB", wait(300));
        let shown = show(bytes.as_bytes());
        assert_eq!(text(&shown.art), vec!["B"]);
        let steps: Vec<(u32, String)> = shown.play.iter().map(|s| (s.at, row_text(&s.rows[0].1))).collect();
        // 144 characters a tenth of a second: A at once, B in the third.
        assert_eq!(steps, vec![(100, "A".to_string()), (300, "B".to_string())]);
    }

    #[test]
    fn clearing_a_drawn_screen_is_a_step_on_a_blank_picture() {
        let shown = show(format!("X{}\x1b[2JY", wait(150)).as_bytes());
        let last = shown.play.last().unwrap();
        assert!(last.clear);
        assert_eq!(row_text(&last.rows[0].1), "Y");
        assert!(!shown.play[0].clear);
    }

    #[test]
    fn erasing_drawn_cells_makes_an_animation_too() {
        let shown = show(format!("gone{}\r\x1b[K", wait(150)).as_bytes());
        assert_eq!(shown.play.len(), 2);
        assert_eq!(row_text(&shown.play[1].rows[0].1), "");
    }

    #[test]
    fn flashes_come_at_most_three_a_second() {
        let mut bytes = String::new();
        for i in 0..10 {
            bytes += &format!("\x1b[H\x1b[{}m  {}", if i % 2 == 0 { 44 } else { 41 }, wait(150));
        }
        let mut bytes = bytes.into_bytes();
        bytes.extend(sauce(2, false));
        let at: Vec<u32> = show(&bytes).play.iter().map(|s| s.at).collect();
        assert_eq!(at.len(), 10);
        assert!(at.windows(2).all(|w| w[1] - w[0] >= FLASH_GAP_MS), "{at:?}");
    }

    #[test]
    fn small_changes_are_not_held_back() {
        let mut bytes = String::from("\x1b[44m          ");
        for i in 0..10 {
            bytes += &format!("\x1b[1;1H{i}{}", wait(150));
        }
        let mut bytes = bytes.into_bytes();
        bytes.extend(sauce(10, false));
        let at: Vec<u32> = show(&bytes).play.iter().map(|s| s.at).collect();
        assert!(at.windows(2).all(|w| w[1] - w[0] <= 2 * TICK_MS), "{at:?}");
    }

    #[test]
    fn a_long_animation_is_sped_up_to_fit_two_minutes() {
        let bytes = "\x1b[HA\x1b[HB".repeat(100_000);
        let play = show(bytes.as_bytes()).play;
        assert!(play.len() <= MOST_STEPS + 1);
        assert!(play.last().unwrap().at <= (MOST_STEPS as u32 + 1) * TICK_MS);
    }
}

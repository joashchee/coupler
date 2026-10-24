// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! **TMF, the Text Music Format**: a [`Score`] (and a title) as a short
//! run of plain text that survives being typed into a game, stored, and
//! printed back wrapped to any width. Pure and unit-tested.
//!
//! ```
//! use neumetik_midi::{tmf, Score};
//!
//! let score = Score::new();
//! let text = tmf::write(&score, "An empty song");
//! assert!(text.starts_with("TMF1 "));
//! let (back, title) = tmf::read(&text)?;
//! assert_eq!((back, title.as_str()), (score, "An empty song"));
//! # Ok::<(), neumetik_midi::Error>(())
//! ```
//!
//! It was made for CoffeeMUD's journals (a clan's journal is its forum):
//! WRITE takes an entry as one typed line, and READ prints it through the
//! game's output filter, which wraps at spaces and changes a good many
//! characters (`' " \ % ^ ( ) < > & !` and more). So the text is:
//!
//! - `TMF1`, the format and its version;
//! - the data in groups of 64 characters of base64url (`A`–`Z`, `a`–`z`,
//!   `0`–`9`, `-`, `_`: nothing any filter touches), a space between
//!   groups, so the game wraps a group a line;
//! - ` =`, the end.
//!
//! Reading ignores every space and line break between `TMF1` and `=`, so
//! however the text was wrapped it reads the same, and text around it
//! (the entry's header, the prompt after) is left alone.
//!
//! **The data**, before the base64url: a flags byte (bit 0: packed), the
//! body (packed by [`pack`], an LZ77 of its own, when that's smaller),
//! then the body's CRC-32, so a damaged copy is refused rather than read
//! wrong. The body, in LEB128 numbers ("n"), zigzag for signed ("z"):
//!
//! - n ticks a quarter note, n the grid (every time in the score is a
//!   multiple of it, so times are written in grid steps), n the tempo in
//!   hundredths of a beat a minute, a byte the beats and a byte the
//!   unit's power of two, n the bars, the title (n bytes of UTF-8);
//! - the score's kept events: n of them, each z its step from the last
//!   one, n bytes;
//! - n parts, each its name, a byte each for channel, bank and program,
//!   n notes, then its kept events. A note is n of (z its step from the
//!   last note × 4, +1 when its length is the last note's, +2 when its
//!   velocity is), z its key from the last note's (60 first), then its
//!   length in steps unless it's the same, then its velocity byte unless
//!   it's the same.
//!
//! So repeated bars come out as repeated bytes, which the packing turns
//! into a few bytes each. Notes are written in the score's own order,
//! so a score reads back exactly as it was written.

use crate::score::{Kept, Note, Part, Score};
use crate::Error;

/// The first word of the text: the format and its version.
pub const MAGIC: &str = "TMF1";
/// The last word of the text.
pub const END: &str = "=";
/// Characters in a group, between spaces.
const GROUP: usize = 64;
/// The most a body unpacks to: far past any song, short of a trap.
const MOST: usize = 8 << 20;

/// The score and its title as TMF text.
pub fn write(score: &Score, title: &str) -> String {
    let body = body(score, title);
    let packed = pack(&body);
    let mut data = Vec::with_capacity(body.len() + 5);
    if packed.len() < body.len() {
        data.push(1);
        data.extend_from_slice(&packed);
    } else {
        data.push(0);
        data.extend_from_slice(&body);
    }
    data.extend_from_slice(&crc32(&body).to_be_bytes());
    let encoded = base64url(&data);
    let mut text = String::with_capacity(encoded.len() + encoded.len() / GROUP + 8);
    text.push_str(MAGIC);
    for group in encoded.as_bytes().chunks(GROUP) {
        text.push(' ');
        text.push_str(std::str::from_utf8(group).unwrap_or_default());
    }
    text.push(' ');
    text.push_str(END);
    text
}

/// The score and title in TMF text: the first `TMF1` to the `=` after
/// it, whatever spaces and line breaks are in between, and whatever is
/// around it.
pub fn read(text: &str) -> Result<(Score, String), Error> {
    let data = unbase64url(&found(text).ok_or(Error::NotTmf)?)?;
    let (&flags, rest) = data.split_first().ok_or(Error::NotTmf)?;
    if rest.len() < 4 || flags > 1 {
        return Err(Error::NotTmf);
    }
    let (inner, crc) = rest.split_at(rest.len() - 4);
    let body = if flags & 1 == 1 { unpack(inner)? } else { inner.to_vec() };
    if crc32(&body).to_be_bytes() != crc {
        return Err(Error::NotTmf);
    }
    from_body(&body)
}

/// Whether text holds the start of TMF.
pub fn starts_in(text: &str) -> bool {
    text.split_whitespace().any(|w| w == MAGIC)
}

/// A MIDI file as TMF text.
pub fn from_midi(bytes: &[u8], title: &str) -> Result<String, Error> {
    Ok(write(&Score::from_midi(bytes)?, title))
}

/// TMF text as a MIDI file, to play or keep.
pub fn to_midi(text: &str) -> Result<Vec<u8>, Error> {
    Ok(read(text)?.0.to_midi())
}

/// The base64url between the first `TMF1` and its `=`, run together.
fn found(text: &str) -> Option<String> {
    let mut words = text.split_whitespace().skip_while(|w| *w != MAGIC).skip(1);
    let mut out = String::new();
    loop {
        let word = words.next()?;
        if word == END {
            return Some(out);
        }
        // An end run onto the last group, if something joined them.
        if let Some(last) = word.strip_suffix(END) {
            out.push_str(last);
            return Some(out);
        }
        out.push_str(word);
    }
}

// ---- The body ----

fn body(score: &Score, title: &str) -> Vec<u8> {
    let grid = grid(score);
    let mut out = Vec::new();
    n(u64::from(score.ppq), &mut out);
    n(u64::from(grid), &mut out);
    let bpm = if score.bpm.is_finite() { (score.bpm * 100.0).round().clamp(0.0, u32::MAX as f64) as u64 } else { 12_000 };
    n(bpm, &mut out);
    out.push(score.beats);
    // The unit is a power of two (1 to 32); anything else as it is, past 5.
    out.push(if score.unit.is_power_of_two() { score.unit.trailing_zeros() as u8 } else { 0x80 | score.unit });
    n(u64::from(score.bars), &mut out);
    text(title, &mut out);
    kept(&score.kept, grid, &mut out);
    n(score.parts.len() as u64, &mut out);
    for part in &score.parts {
        text(&part.name, &mut out);
        out.extend_from_slice(&[part.channel, part.bank, part.program]);
        n(part.notes.len() as u64, &mut out);
        let (mut at, mut key, mut length, mut velocity) = (0i64, 60i64, None, None);
        for note in &part.notes {
            let step = i64::from(note.at / grid);
            let same_length = length == Some(note.length);
            let same_velocity = velocity == Some(note.velocity);
            n(zig(step - at) << 2 | u64::from(same_length) | u64::from(same_velocity) << 1, &mut out);
            n(zig(i64::from(note.key) - key), &mut out);
            if !same_length {
                n(u64::from(note.length / grid), &mut out);
            }
            if !same_velocity {
                out.push(note.velocity);
            }
            (at, key, length, velocity) = (step, i64::from(note.key), Some(note.length), Some(note.velocity));
        }
        kept(&part.kept, grid, &mut out);
    }
    out
}

/// The largest step every time in the score is a multiple of (1 when
/// there are no times but zero).
fn grid(score: &Score) -> u32 {
    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let notes = score.parts.iter().flat_map(|p| p.notes.iter().flat_map(|n| [n.at, n.length]));
    let kept = score.kept.iter().chain(score.parts.iter().flat_map(|p| p.kept.iter())).map(|k| k.at);
    notes.chain(kept).fold(0, gcd).max(1)
}

fn kept(events: &[Kept], grid: u32, out: &mut Vec<u8>) {
    n(events.len() as u64, out);
    let mut at = 0i64;
    for k in events {
        let step = i64::from(k.at / grid);
        n(zig(step - at), out);
        n(k.bytes.len() as u64, out);
        out.extend_from_slice(&k.bytes);
        at = step;
    }
}

fn text(s: &str, out: &mut Vec<u8>) {
    n(s.len() as u64, out);
    out.extend_from_slice(s.as_bytes());
}

fn n(mut v: u64, out: &mut Vec<u8>) {
    while v >= 0x80 {
        out.push(v as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn zig(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

fn unzig(v: u64) -> i64 {
    (v >> 1) as i64 ^ -((v & 1) as i64)
}

struct Body<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Body<'_> {
    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.bytes.get(self.at).ok_or(Error::NotTmf)?;
        self.at += 1;
        Ok(b)
    }
    fn n(&mut self) -> Result<u64, Error> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.byte()?;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(Error::NotTmf)
    }
    fn z(&mut self) -> Result<i64, Error> {
        Ok(unzig(self.n()?))
    }
    /// A number that has to fit a `u32`.
    fn n32(&mut self) -> Result<u32, Error> {
        u32::try_from(self.n()?).map_err(|_| Error::NotTmf)
    }
    /// A count: never more than the bytes left could hold.
    fn count(&mut self) -> Result<usize, Error> {
        let c = self.n()?;
        usize::try_from(c).ok().filter(|&c| c <= self.bytes.len() - self.at).ok_or(Error::NotTmf)
    }
    fn take(&mut self, len: usize) -> Result<&[u8], Error> {
        let end = self.at.checked_add(len).filter(|&e| e <= self.bytes.len()).ok_or(Error::NotTmf)?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn text(&mut self) -> Result<String, Error> {
        let len = self.count()?;
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| Error::NotTmf)
    }
    /// A time: steps of the grid, from the last.
    fn time(&mut self, last: &mut i64, grid: u32) -> Result<u32, Error> {
        let step = last.checked_add(self.z()?).filter(|s| (0..=i64::from(u32::MAX)).contains(s)).ok_or(Error::NotTmf)?;
        *last = step;
        u32::try_from(step).ok().and_then(|s| s.checked_mul(grid)).ok_or(Error::NotTmf)
    }
    fn kept(&mut self, grid: u32) -> Result<Vec<Kept>, Error> {
        let count = self.count()?;
        let mut events = Vec::with_capacity(count);
        let mut at = 0i64;
        for _ in 0..count {
            let when = self.time(&mut at, grid)?;
            let len = self.count()?;
            events.push(Kept { at: when, bytes: self.take(len)?.to_vec() });
        }
        Ok(events)
    }
}

fn from_body(bytes: &[u8]) -> Result<(Score, String), Error> {
    let mut b = Body { bytes, at: 0 };
    let ppq = u16::try_from(b.n()?).ok().filter(|&p| p > 0).ok_or(Error::NotTmf)?;
    let grid = b.n32()?.max(1);
    let bpm = b.n()? as f64 / 100.0;
    let beats = b.byte()?;
    let unit = match b.byte()? {
        u if u & 0x80 != 0 => u & 0x7f,
        u if u < 8 => 1 << u,
        _ => return Err(Error::NotTmf),
    };
    let bars = b.n32()?;
    let title = b.text()?;
    let kept = b.kept(grid)?;
    let count = b.count()?;
    let mut parts = Vec::with_capacity(count);
    for _ in 0..count {
        let name = b.text()?;
        let (channel, bank, program) = (b.byte()?, b.byte()?, b.byte()?);
        let count = b.count()?;
        let mut notes = Vec::with_capacity(count);
        let (mut at, mut key, mut length, mut velocity) = (0i64, 60i64, None, None);
        for _ in 0..count {
            let token = b.n()?;
            let step = at.checked_add(unzig(token >> 2)).filter(|s| (0..=i64::from(u32::MAX)).contains(s)).ok_or(Error::NotTmf)?;
            key = key.checked_add(b.z()?).ok_or(Error::NotTmf)?;
            let k = u8::try_from(key).map_err(|_| Error::NotTmf)?;
            let l = match (token & 1 == 1, length) {
                (true, Some(l)) => l,
                (false, _) => b.n32()?.checked_mul(grid).ok_or(Error::NotTmf)?,
                (true, None) => return Err(Error::NotTmf),
            };
            let v = match (token & 2 == 2, velocity) {
                (true, Some(v)) => v,
                (false, _) => b.byte()?,
                (true, None) => return Err(Error::NotTmf),
            };
            let a = u32::try_from(step).ok().and_then(|s| s.checked_mul(grid)).ok_or(Error::NotTmf)?;
            notes.push(Note { at: a, length: l, key: k, velocity: v });
            (at, length, velocity) = (step, Some(l), Some(v));
        }
        let kept = b.kept(grid)?;
        parts.push(Part { name, channel, bank, program, notes, kept });
    }
    if b.at != bytes.len() {
        return Err(Error::NotTmf);
    }
    Ok((Score { ppq, bpm, beats, unit, bars, parts, kept }, title))
}

// ---- Packing: LZ77, standard library only ----

/// The shortest repeat worth a reference.
const MIN_MATCH: usize = 3;
/// How many earlier places with the same three bytes are tried.
const CHAIN: usize = 128;

/// Bytes packed: runs of literals and references back to bytes already
/// written. Each step is n literals and the literals, then (unless
/// that's the end) n the length less 3 and n the distance less 1.
pub fn pack(input: &[u8]) -> Vec<u8> {
    const BITS: u32 = 15;
    let hash = |i: usize| -> usize {
        let v = u32::from(input[i]) | u32::from(input[i + 1]) << 8 | u32::from(input[i + 2]) << 16;
        (v.wrapping_mul(2_654_435_761) >> (32 - BITS)) as usize
    };
    let mut head = vec![usize::MAX; 1 << BITS];
    let mut prev = vec![usize::MAX; input.len()];
    let insert = |i: usize, head: &mut Vec<usize>, prev: &mut Vec<usize>| {
        if i + MIN_MATCH <= input.len() {
            let h = hash(i);
            prev[i] = head[h];
            head[h] = i;
        }
    };
    let longest = |i: usize, head: &Vec<usize>, prev: &Vec<usize>| -> (usize, usize) {
        if i + MIN_MATCH > input.len() {
            return (0, 0);
        }
        let (mut best, mut from) = (0, 0);
        let mut j = head[hash(i)];
        for _ in 0..CHAIN {
            if j == usize::MAX {
                break;
            }
            let len = input[j..].iter().zip(&input[i..]).take_while(|(a, b)| a == b).count();
            if len > best {
                (best, from) = (len, j);
            }
            j = prev[j];
        }
        if best >= MIN_MATCH { (best, i - from) } else { (0, 0) }
    };
    let mut out = Vec::new();
    let mut literals = 0usize;
    let mut i = 0;
    while i < input.len() {
        let (len, dist) = longest(i, &head, &prev);
        // One step lazy: a longer repeat starting at the next byte wins.
        let take = len > 0 && {
            insert(i, &mut head, &mut prev);
            let (next, _) = longest(i + 1, &head, &prev);
            next <= len
        };
        if !take {
            if len == 0 {
                insert(i, &mut head, &mut prev);
            }
            i += 1;
            literals += 1;
            continue;
        }
        n(literals as u64, &mut out);
        out.extend_from_slice(&input[i - literals..i]);
        n((len - MIN_MATCH) as u64, &mut out);
        n((dist - 1) as u64, &mut out);
        for k in i + 1..i + len {
            insert(k, &mut head, &mut prev);
        }
        i += len;
        literals = 0;
    }
    n(literals as u64, &mut out);
    out.extend_from_slice(&input[i - literals..i]);
    out
}

/// What [`pack`] packed.
pub fn unpack(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut b = Body { bytes: input, at: 0 };
    let mut out: Vec<u8> = Vec::new();
    loop {
        let literals = b.count()?;
        out.extend_from_slice(b.take(literals)?);
        if b.at == input.len() {
            return Ok(out);
        }
        let len = usize::try_from(b.n()?).map_err(|_| Error::NotTmf)? + MIN_MATCH;
        let dist = usize::try_from(b.n()?).map_err(|_| Error::NotTmf)? + 1;
        if dist > out.len() || out.len() + len > MOST {
            return Err(Error::NotTmf);
        }
        let from = out.len() - dist;
        for k in 0..len {
            out.push(out[from + k]);
        }
    }
}

// ---- Base64url and CRC-32 ----

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let v = u32::from(chunk[0]) << 16 | u32::from(*chunk.get(1).unwrap_or(&0)) << 8 | u32::from(*chunk.get(2).unwrap_or(&0));
        for k in 0..=chunk.len() {
            out.push(ALPHABET[(v >> (18 - 6 * k) & 63) as usize] as char);
        }
    }
    out
}

fn unbase64url(text: &str) -> Result<Vec<u8>, Error> {
    let digit = |c: u8| ALPHABET.iter().position(|&a| a == c).map(|p| p as u32).ok_or(Error::NotTmf);
    let bytes = text.as_bytes();
    if bytes.len() % 4 == 1 {
        return Err(Error::NotTmf);
    }
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut v = 0u32;
        for (k, &c) in chunk.iter().enumerate() {
            v |= digit(c)? << (18 - 6 * k);
        }
        for k in 0..chunk.len() - 1 {
            out.push((v >> (16 - 8 * k)) as u8);
        }
    }
    Ok(out)
}

/// CRC-32 (IEEE, as zip and PNG have it).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A busy score: a drum beat, a bass line and chords over 16 bars,
    /// repeating, a pitch bend and a controller kept, a marker on the score.
    fn busy() -> Score {
        let mut score = Score { bars: 16, ..Score::new() };
        let bar = score.bar_ticks();
        let step = 120;
        score.parts[1].notes = (0..16)
            .flat_map(|b| (0..16).map(move |s| (b, s)))
            .filter_map(|(b, s)| {
                let key = match s % 8 {
                    0 => 36,
                    4 => 38,
                    _ if s % 2 == 0 => 42,
                    _ => return None,
                };
                Some(Note { at: b * bar + s * step, length: step, key, velocity: if s % 4 == 0 { 110 } else { 80 } })
            })
            .collect();
        score.parts[0].notes = (0..16)
            .flat_map(|b| [(b, 0, 48), (b, 1, 55), (b, 2, 52), (b, 3, 55)])
            .map(|(b, beat, key)| Note { at: b * bar + beat * 480, length: 480, key: key + (b % 4) as u8 * 2, velocity: 96 })
            .collect();
        score.parts[0].kept = vec![Kept { at: 960, bytes: vec![0xe0, 0, 0x50] }, Kept { at: 1920, bytes: vec![0xb0, 7, 100] }];
        score.parts.push(Part { name: "Pad".into(), channel: 1, bank: 80, program: 3, notes: vec![Note { at: 7, length: 30_713, key: 72, velocity: 1 }], kept: vec![] });
        score.kept = vec![Kept { at: 0, bytes: vec![0xff, 0x06, 4, b'l', b'o', b'o', b'p'] }];
        score
    }

    #[test]
    fn it_reads_back_exactly() {
        for (score, title) in [(Score::new(), ""), (busy(), "A Busy Song: über ♪")] {
            let text = write(&score, title);
            assert_eq!(read(&text).unwrap(), (score, title.to_string()));
        }
    }

    #[test]
    fn it_reads_back_from_midi_and_to_midi() {
        let midi = crate::score::tests::little();
        let text = from_midi(&midi, "little").unwrap();
        let score = Score::from_midi(&midi).unwrap();
        assert_eq!(to_midi(&text).unwrap(), score.to_midi());
    }

    #[test]
    fn only_characters_no_game_changes() {
        let text = write(&busy(), "Who's \"there\" (100%)? ^r<b>&!");
        assert!(text.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ' ' | '=')), "{text}");
        assert!(text.split(' ').all(|w| w.len() <= GROUP));
    }

    #[test]
    fn wrapping_and_text_around_it_change_nothing() {
        let score = busy();
        let text = write(&score, "song");
        // Every other space a line break, as a game wraps it, with the entry's header and prompt about it.
        let mut wrapped = String::from("FROM: Bob\r\nSUBJ: song\r\n");
        for (i, w) in text.split(' ').enumerate() {
            wrapped.push_str(w);
            wrapped.push_str(if i % 2 == 0 { "\r\n" } else { " " });
        }
        wrapped.push_str("\r\nR)eply N)ext or RETURN: ");
        assert_eq!(read(&wrapped).unwrap().0, score);
        assert!(starts_in(&wrapped));
    }

    #[test]
    fn damage_is_refused() {
        let text = write(&busy(), "song");
        // One character changed.
        let i = text.len() / 2;
        let swap = if &text[i..=i] == "A" { "B" } else { "A" };
        let damaged = format!("{}{swap}{}", &text[..i], &text[i + 1..]);
        assert_eq!(read(&damaged), Err(Error::NotTmf));
        // Cut short, or no end.
        assert_eq!(read(&text[..text.len() / 2]), Err(Error::NotTmf));
        assert_eq!(read("TMF1 AAAA"), Err(Error::NotTmf));
        assert_eq!(read("hello"), Err(Error::NotTmf));
        assert_eq!(read("TMF1 =").err(), Some(Error::NotTmf));
    }

    #[test]
    fn it_is_compact() {
        let score = busy();
        let notes: usize = score.parts.iter().map(|p| p.notes.len()).sum();
        let text = write(&score, "song");
        let midi = score.to_midi();
        assert!(text.len() * 4 < midi.len(), "{} characters for a {}-byte MIDI file", text.len(), midi.len());
        assert!(text.len() < notes, "{} characters for {notes} notes", text.len());
        println!("{notes} notes: {} characters of TMF, {} bytes of MIDI", text.len(), midi.len());
    }

    #[test]
    fn packing_round_trips() {
        let mut noise = 12345u32;
        let random: Vec<u8> = (0..5000)
            .map(|_| {
                noise = noise.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (noise >> 16) as u8
            })
            .collect();
        let repeats: Vec<u8> = (0..5000).map(|i| (i % 37) as u8).collect();
        for input in [vec![], vec![7], vec![1, 1, 1, 1, 1, 1, 1, 1], random, repeats.clone()] {
            assert_eq!(unpack(&pack(&input)).unwrap(), input);
        }
        assert!(pack(&repeats).len() < 100);
    }

    #[test]
    fn base64url_and_crc() {
        for bytes in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            assert_eq!(unbase64url(&base64url(bytes)).unwrap(), bytes);
        }
        assert_eq!(base64url(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn odd_meters_and_tempi() {
        let score = Score { bpm: 87.35, beats: 7, unit: 8, ..Score::new() };
        assert_eq!(read(&write(&score, "")).unwrap().0, score);
        let odd = Score { unit: 3, ..Score::new() };
        assert_eq!(read(&write(&odd, "")).unwrap().0.unit, 3);
    }
}

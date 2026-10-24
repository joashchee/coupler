// Copyright 2026 ansiapps. Neumetik MIDI, MIT licensed: see LICENSE.

//! **A score**: a MIDI file as an editor has it (parts, bars and notes),
//! read and written back with nothing lost. Pure and unit-tested; the
//! form TMF writes as text ([`crate::tmf`]).
//!
//! A part is one channel of one track (a format 1 file's tracks split by
//! channel, so a format 0 file's channels become parts too). Its notes
//! are what's edited; its first bank select and program change before
//! any note are its instrument; everything else on it (controllers,
//! pitch bends, later program changes) is kept as it was, at its tick,
//! and written back. The first tempo and time signature are the score's;
//! every other meta event and system exclusive message is kept the same
//! way, on the first track.
//!
//! [`Score::to_midi`] makes a format 1 file: the first track the tempo,
//! the meter and what was kept, then a track a part, each ending at the
//! score's last bar, so the length is exact and the music loops cleanly.

use crate::Error;

/// A score: the music, its tempo, meter and length in bars.
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    /// Ticks a quarter note.
    pub ppq: u16,
    /// Beats (quarter notes) a minute.
    pub bpm: f64,
    /// The meter: `beats` of `unit` (4 a quarter, 8 an eighth) a bar.
    pub beats: u8,
    pub unit: u8,
    pub bars: u32,
    pub parts: Vec<Part>,
    /// The score's other events, as they were.
    pub kept: Vec<Kept>,
}

/// One instrument's line: a channel of a track.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: String,
    /// 0 to 15; 9 is the drums.
    pub channel: u8,
    /// The bank (Bank Select MSB) and program: the instrument, or a drum kit.
    pub bank: u8,
    pub program: u8,
    pub notes: Vec<Note>,
    /// Its other events, as they were.
    pub kept: Vec<Kept>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    /// In ticks from the start.
    pub at: u32,
    pub length: u32,
    pub key: u8,
    pub velocity: u8,
}

/// An event a score doesn't show, kept to write back: its tick and its
/// bytes as a track has them (status first; a meta or system exclusive
/// message with its length).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub at: u32,
    pub bytes: Vec<u8>,
}

/// The drums' channel.
pub const DRUMS: u8 = 9;
/// The most parts a score has: a part a channel a track, but within reason.
pub const MOST_PARTS: usize = 64;

impl Score {
    /// Ticks a bar.
    pub fn bar_ticks(&self) -> u32 {
        (u32::from(self.ppq) * 4 * u32::from(self.beats.max(1)) / u32::from(self.unit.max(1))).max(1)
    }

    /// A new score: four bars of 4/4 at 120, a piano and the drums, empty.
    pub fn new() -> Score {
        Score {
            ppq: 480,
            bpm: 120.0,
            beats: 4,
            unit: 4,
            bars: 4,
            parts: vec![
                Part { name: "Piano".into(), channel: 0, bank: 0, program: 0, notes: vec![], kept: vec![] },
                Part { name: "Drums".into(), channel: DRUMS, bank: 0, program: 0, notes: vec![], kept: vec![] },
            ],
            kept: vec![],
        }
    }

    /// A MIDI file's score.
    pub fn from_midi(bytes: &[u8]) -> Result<Score, Error> {
        let mut r = Reader { bytes: unwrap_rmid(bytes), at: 0 };
        if r.take(4).map_err(|_| Error::NotMidi)? != b"MThd" {
            return Err(Error::NotMidi);
        }
        let len = r.u32()? as usize;
        let header = r.take(len.max(6))?;
        let (tracks, division) = (u16::from_be_bytes([header[2], header[3]]), u16::from_be_bytes([header[4], header[5]]));
        // Film frames (SMPTE) aren't beats.
        if division & 0x8000 != 0 || division == 0 {
            return Err(Error::BadTiming);
        }
        let mut score = Score { ppq: division, bpm: 120.0, beats: 4, unit: 4, bars: 1, parts: vec![], kept: vec![] };
        let (mut tempo_set, mut meter_set) = (false, false);
        let mut end = 0u64;
        for track in 0..tracks {
            let Ok(id) = r.take(4) else { break };
            let Ok(len) = r.u32() else { break };
            let Ok(body) = r.take(len as usize) else { break };
            if id != b"MTrk" {
                continue;
            }
            let last = read_track(body, track, &mut score, &mut tempo_set, &mut meter_set);
            end = end.max(last);
        }
        for part in &mut score.parts {
            part.notes.sort_by_key(|n| (n.at, n.key));
            end = end.max(part.notes.iter().map(|n| u64::from(n.at) + u64::from(n.length)).max().unwrap_or(0));
            end = end.max(part.kept.iter().map(|k| u64::from(k.at)).max().unwrap_or(0));
        }
        end = end.max(score.kept.iter().map(|k| u64::from(k.at)).max().unwrap_or(0));
        let bar = u64::from(score.bar_ticks());
        score.bars = end.div_ceil(bar).clamp(1, 9999) as u32;
        Ok(score)
    }

    /// The score as a format 1 MIDI file.
    pub fn to_midi(&self) -> Vec<u8> {
        let end = self.bars.max(1).saturating_mul(self.bar_ticks());
        let micros = (60_000_000.0 / self.bpm.clamp(10.0, 1000.0)).round() as u32;
        let mut conductor: Vec<Timed> = vec![
            (0, 0, meta(0x51, &micros.to_be_bytes()[1..])),
            (0, 0, meta(0x58, &[self.beats.max(1), self.unit.max(1).trailing_zeros() as u8, 24, 8])),
        ];
        conductor.extend(self.kept.iter().map(|k| (k.at, 1, k.bytes.clone())));
        let mut tracks = vec![track(conductor, end)];
        for part in &self.parts {
            let ch = part.channel & 0x0f;
            let mut events: Vec<Timed> = vec![(0, 0, meta(0x03, part.name.as_bytes()))];
            if part.bank != 0 {
                events.push((0, 0, vec![0xb0 | ch, 0, part.bank & 0x7f]));
            }
            events.push((0, 0, vec![0xc0 | ch, part.program & 0x7f]));
            events.extend(part.kept.iter().map(|k| (k.at, 1, k.bytes.clone())));
            for n in &part.notes {
                events.push((n.at, 3, vec![0x90 | ch, n.key & 0x7f, n.velocity.clamp(1, 127)]));
                events.push((n.at.saturating_add(n.length.max(1)), 2, vec![0x80 | ch, n.key & 0x7f, 64]));
            }
            tracks.push(track(events, end));
        }
        let mut out = b"MThd".to_vec();
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.ppq.to_be_bytes());
        for t in tracks {
            out.extend(t);
        }
        out
    }
}

impl Default for Score {
    fn default() -> Self {
        Score::new()
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.bytes.get(self.at).ok_or(Error::Truncated)?;
        self.at += 1;
        Ok(b)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or(Error::Truncated)?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn vlq(&mut self) -> Result<u32, Error> {
        let mut n = 0u32;
        for _ in 0..4 {
            let b = self.byte()?;
            n = (n << 7) | u32::from(b & 0x7f);
            if b & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err(Error::Truncated)
    }
}

/// A number as a track writes it, seven bits a byte.
pub(crate) fn vlq(mut n: u32, out: &mut Vec<u8>) {
    let mut bytes = vec![(n & 0x7f) as u8];
    n >>= 7;
    while n > 0 {
        bytes.push((n & 0x7f) as u8 | 0x80);
        n >>= 7;
    }
    out.extend(bytes.iter().rev());
}

/// A RIFF RMID file's MIDI inside it.
fn unwrap_rmid(bytes: &[u8]) -> &[u8] {
    if bytes.len() > 20 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"RMID" {
        if let Some(at) = bytes.windows(4).position(|w| w == b"MThd") {
            return &bytes[at..];
        }
    }
    bytes
}

/// One track's events into the score: its parts, and what's kept.
/// Returns its last tick. A track damaged partway is read to there.
fn read_track(body: &[u8], track: u16, score: &mut Score, tempo_set: &mut bool, meter_set: &mut bool) -> u64 {
    let mut r = Reader { bytes: body, at: 0 };
    let mut tick = 0u64;
    let mut running: Option<u8> = None;
    let mut name: Option<String> = None;
    // This track's parts, by channel: their index in the score, whether
    // the instrument is settled, and the notes sounding.
    let mut parts: [Option<usize>; 16] = [None; 16];
    let mut settled = [(false, false); 16];
    let mut open: Vec<(u8, u8, u64, u8)> = Vec::new();
    let first_part = score.parts.len();
    let at = |t: u64| t.min(u64::from(u32::MAX)) as u32;
    while r.at < r.bytes.len() {
        let Ok(delta) = r.vlq() else { break };
        tick += u64::from(delta);
        let Ok(b) = r.byte() else { break };
        let (status, first) = if b & 0x80 != 0 {
            if b < 0xf0 {
                running = Some(b);
            }
            (b, None)
        } else {
            match running {
                Some(s) => (s, Some(b)),
                None => break,
            }
        };
        match status {
            0xff => {
                let Ok(kind) = r.byte() else { break };
                let Ok(len) = r.vlq() else { break };
                let Ok(data) = r.take(len as usize) else { break };
                match kind {
                    0x2f => break,
                    0x03 if tick == 0 && name.is_none() => name = Some(String::from_utf8_lossy(data).trim().to_string()),
                    0x51 if !*tempo_set && data.len() == 3 => {
                        let micros = u32::from(data[0]) << 16 | u32::from(data[1]) << 8 | u32::from(data[2]);
                        score.bpm = if micros > 0 { (60_000_000.0 / f64::from(micros) * 100.0).round() / 100.0 } else { 120.0 };
                        *tempo_set = true;
                    }
                    0x58 if !*meter_set && data.len() >= 2 => {
                        score.beats = data[0].clamp(1, 32);
                        score.unit = 1u8 << data[1].min(5);
                        *meter_set = true;
                    }
                    _ => {
                        let mut bytes = vec![0xff, kind];
                        vlq(len, &mut bytes);
                        bytes.extend_from_slice(data);
                        score.kept.push(Kept { at: at(tick), bytes });
                    }
                }
            }
            0xf0 | 0xf7 => {
                running = None;
                let Ok(len) = r.vlq() else { break };
                let Ok(data) = r.take(len as usize) else { break };
                let mut bytes = vec![status];
                vlq(len, &mut bytes);
                bytes.extend_from_slice(data);
                score.kept.push(Kept { at: at(tick), bytes });
            }
            0x80..=0xef => {
                let a = match first {
                    Some(a) => a,
                    None => {
                        let Ok(a) = r.byte() else { break };
                        a
                    }
                } & 0x7f;
                let two = !matches!(status & 0xf0, 0xc0 | 0xd0);
                let b = if two {
                    let Ok(b) = r.byte() else { break };
                    b & 0x7f
                } else {
                    0
                };
                let ch = status & 0x0f;
                let index = match parts[usize::from(ch)] {
                    Some(i) => i,
                    None => {
                        if score.parts.len() >= MOST_PARTS {
                            continue;
                        }
                        score.parts.push(Part { name: String::new(), channel: ch, bank: 0, program: 0, notes: vec![], kept: vec![] });
                        parts[usize::from(ch)] = Some(score.parts.len() - 1);
                        score.parts.len() - 1
                    }
                };
                let part = &mut score.parts[index];
                let (program_set, bank_set) = &mut settled[usize::from(ch)];
                match status & 0xf0 {
                    0x90 if b > 0 => open.push((ch, a, tick, b)),
                    0x80 | 0x90 => {
                        if let Some(i) = open.iter().position(|o| o.0 == ch && o.1 == a) {
                            let (_, key, from, velocity) = open.remove(i);
                            part.notes.push(Note { at: at(from), length: at(tick - from).max(1), key, velocity });
                        }
                    }
                    0xc0 if !*program_set && part.notes.is_empty() && !open.iter().any(|o| o.0 == ch) => {
                        part.program = a;
                        *program_set = true;
                    }
                    0xb0 if a == 0 && !*bank_set && !*program_set && part.notes.is_empty() => {
                        part.bank = b;
                        *bank_set = true;
                    }
                    _ => {
                        let mut bytes = vec![status, a];
                        if two {
                            bytes.push(b);
                        }
                        part.kept.push(Kept { at: at(tick), bytes });
                    }
                }
            }
            _ => break,
        }
    }
    // Notes never let go end with the track.
    for (ch, key, from, velocity) in open {
        if let Some(i) = parts[usize::from(ch)] {
            score.parts[i].notes.push(Note { at: at(from), length: at(tick - from).max(1), key, velocity });
        }
    }
    let mine: Vec<usize> = (first_part..score.parts.len()).collect();
    for &i in &mine {
        let ch = score.parts[i].channel;
        let base = name.clone().filter(|n| !n.is_empty());
        score.parts[i].name = match (base, mine.len() > 1) {
            (Some(n), false) => n,
            (Some(n), true) => format!("{n} ({})", channel_name(ch)),
            (None, _) if ch == DRUMS => "Drums".into(),
            (None, _) => format!("Track {} ({})", track + 1, channel_name(ch)),
        };
    }
    tick
}

fn channel_name(ch: u8) -> String {
    if ch == DRUMS { "drums".into() } else { format!("channel {}", ch + 1) }
}

/// An event to write: its tick, an order among those at the same tick
/// (a name and the instrument first, then what's kept, notes let go,
/// notes played), and its bytes.
type Timed = (u32, u8, Vec<u8>);

fn track(mut events: Vec<Timed>, end: u32) -> Vec<u8> {
    events.sort_by_key(|e| (e.0, e.1));
    let mut body = Vec::new();
    let mut now = 0u32;
    for (at, _, bytes) in &events {
        vlq(at - now, &mut body);
        body.extend_from_slice(bytes);
        now = *at;
    }
    vlq(end.max(now) - now, &mut body);
    body.extend_from_slice(&[0xff, 0x2f, 0]);
    let mut out = b"MTrk".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend(body);
    out
}

fn meta(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xff, kind];
    vlq(data.len() as u32, &mut bytes);
    bytes.extend_from_slice(data);
    bytes
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A little format 0 file: a tempo of 100, 3/4, a piano note and a drum, a pitch bend kept.
    pub(crate) fn little() -> Vec<u8> {
        let mut t = vec![];
        t.extend([0, 0xff, 0x51, 3, 0x09, 0x27, 0xc0]); // 600000 us: 100 bpm
        t.extend([0, 0xff, 0x58, 4, 3, 2, 24, 8]);
        t.extend([0, 0xc0, 24]); // nylon guitar
        t.extend([0, 0x90, 60, 100]);
        t.extend([0x83, 0x60, 0xe0, 0, 0x50]); // 480 ticks on: a pitch bend
        t.extend([0, 0x80, 60, 0]);
        t.extend([0, 0x99, 36, 90]);
        t.extend([0x60, 36, 0]); // running status: note on, velocity 0
        let mut out = b"MThd".to_vec();
        out.extend(6u32.to_be_bytes());
        out.extend(0u16.to_be_bytes());
        out.extend(1u16.to_be_bytes());
        out.extend(480u16.to_be_bytes());
        out.extend(b"MTrk");
        out.extend((t.len() as u32 + 4).to_be_bytes());
        out.extend(t);
        out.extend([0, 0xff, 0x2f, 0]);
        out
    }

    #[test]
    fn reads_parts_bars_and_notes() {
        let score = Score::from_midi(&little()).unwrap();
        assert_eq!((score.ppq, score.bpm, score.beats, score.unit), (480, 100.0, 3, 4));
        assert_eq!(score.parts.len(), 2);
        let guitar = &score.parts[0];
        assert_eq!((guitar.channel, guitar.program), (0, 24));
        assert_eq!(guitar.notes, vec![Note { at: 0, length: 480, key: 60, velocity: 100 }]);
        assert_eq!(guitar.kept, vec![Kept { at: 480, bytes: vec![0xe0, 0, 0x50] }]);
        let drums = &score.parts[1];
        assert_eq!((drums.channel, drums.name.as_str()), (DRUMS, "Drums"));
        assert_eq!(drums.notes, vec![Note { at: 480, length: 96, key: 36, velocity: 90 }]);
        assert_eq!(score.bars, 1);
    }

    #[test]
    fn writes_what_it_reads() {
        let score = Score::from_midi(&little()).unwrap();
        let again = Score::from_midi(&score.to_midi()).unwrap();
        assert_eq!(again, Score { parts: again.parts.clone(), ..score.clone() });
        for (a, b) in again.parts.iter().zip(&score.parts) {
            assert_eq!((a.channel, a.program, &a.notes, &a.kept), (b.channel, b.program, &b.notes, &b.kept));
        }
        // And it plays.
        assert!(!crate::render(&again.to_midi(), 22_050, 0.0, 60.0).unwrap().is_empty());
    }

    #[test]
    fn not_midi() {
        assert_eq!(Score::from_midi(b"hello"), Err(Error::NotMidi));
        assert_eq!(Score::from_midi(b"MThd\0\0\0\x06\0\0\0\x01\xe7\x28"), Err(Error::BadTiming), "SMPTE timing");
    }

    #[test]
    fn numbers_are_written_seven_bits_a_byte() {
        let mut out = vec![];
        vlq(0, &mut out);
        vlq(127, &mut out);
        vlq(128, &mut out);
        vlq(0x0fff_ffff, &mut out);
        assert_eq!(out, vec![0, 0x7f, 0x81, 0x00, 0xff, 0xff, 0xff, 0x7f]);
    }
}

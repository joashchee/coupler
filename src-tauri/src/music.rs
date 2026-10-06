//! **The Music Editor's song**: a MIDI file in the Assets folder read
//! into what the editor shows (parts, bars and notes) and written back,
//! pure and unit-tested.
//!
//! A part is one channel of one track (a format 1 file's tracks split by
//! channel, so a format 0 file's channels become parts too). Its notes
//! are what's edited; its first bank select and program change before
//! any note are its instrument; everything else on it (controllers,
//! pitch bends, later program changes) is kept as it was, at its tick,
//! and written back. The first tempo and time signature are the song's;
//! every other meta event and system exclusive message is kept the same
//! way, on the first track. Nothing the editor doesn't show is lost.
//!
//! `write` makes a format 1 file: the first track the tempo, the meter
//! and what was kept, then a track a part, each ending at the song's
//! last bar, so the length is exact and the music loops cleanly.
//! `preview` makes the bars from one to another for Neumetik to play.

use serde::{Deserialize, Serialize};

/// A song, as the editor has it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    /// Ticks a quarter note.
    pub ppq: u16,
    /// Beats (quarter notes) a minute.
    pub bpm: f64,
    /// The meter: `beats` of `unit` (4 a quarter, 8 an eighth) a bar.
    pub beats: u8,
    pub unit: u8,
    pub bars: u32,
    pub parts: Vec<Part>,
    /// The song's other events, as they were.
    #[serde(default)]
    pub kept: Vec<Kept>,
}

/// One instrument's line: a channel of a track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub name: String,
    /// 0 to 15; 9 is the drums.
    pub channel: u8,
    /// The bank (Bank Select MSB) and program: the instrument, or a drum kit.
    pub bank: u8,
    pub program: u8,
    pub notes: Vec<Note>,
    /// Its other events, as they were.
    #[serde(default)]
    pub kept: Vec<Kept>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// In ticks from the start.
    pub at: u32,
    pub length: u32,
    pub key: u8,
    pub velocity: u8,
}

/// An event the editor doesn't show, kept to write back: its tick and
/// its bytes as a track has them (status first; a meta or system
/// exclusive message with its length).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kept {
    pub at: u32,
    pub bytes: Vec<u8>,
}

/// The drums' channel.
pub const DRUMS: u8 = 9;
/// The most parts a song has: a part a channel a track, but within reason.
const MOST_PARTS: usize = 64;

impl Song {
    /// Ticks a bar.
    pub fn bar_ticks(&self) -> u32 {
        (u32::from(self.ppq) * 4 * u32::from(self.beats.max(1)) / u32::from(self.unit.max(1))).max(1)
    }

    /// A new song: four bars of 4/4 at 120, a piano and the drums, empty.
    pub fn new() -> Song {
        Song {
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
}

impl Default for Song {
    fn default() -> Self {
        Song::new()
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.bytes.get(self.at).ok_or("it ends too soon")?;
        self.at += 1;
        Ok(b)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or("it ends too soon")?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn vlq(&mut self) -> Result<u32, String> {
        let mut n = 0u32;
        for _ in 0..4 {
            let b = self.byte()?;
            n = (n << 7) | u32::from(b & 0x7f);
            if b & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err("a length is too long".into())
    }
}

fn vlq(mut n: u32, out: &mut Vec<u8>) {
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

/// A MIDI file's song.
pub fn read(bytes: &[u8]) -> Result<Song, String> {
    let mut r = Reader { bytes: unwrap_rmid(bytes), at: 0 };
    if r.take(4)? != b"MThd" {
        return Err("it isn't a MIDI file".into());
    }
    let len = r.u32()? as usize;
    let header = r.take(len.max(6))?;
    let (tracks, division) = (u16::from_be_bytes([header[2], header[3]]), u16::from_be_bytes([header[4], header[5]]));
    if division & 0x8000 != 0 || division == 0 {
        return Err("it's timed in film frames, not beats, which the editor doesn't read".into());
    }
    let mut song = Song { ppq: division, bpm: 120.0, beats: 4, unit: 4, bars: 1, parts: vec![], kept: vec![] };
    let (mut tempo_set, mut meter_set) = (false, false);
    let mut end = 0u64;
    for track in 0..tracks {
        let Ok(id) = r.take(4) else { break };
        let Ok(len) = r.u32() else { break };
        let Ok(body) = r.take(len as usize) else { break };
        if id != b"MTrk" {
            continue;
        }
        let last = read_track(body, track, &mut song, &mut tempo_set, &mut meter_set)?;
        end = end.max(last);
    }
    for part in &mut song.parts {
        part.notes.sort_by_key(|n| (n.at, n.key));
        end = end.max(part.notes.iter().map(|n| u64::from(n.at) + u64::from(n.length)).max().unwrap_or(0));
        end = end.max(part.kept.iter().map(|k| u64::from(k.at)).max().unwrap_or(0));
    }
    end = end.max(song.kept.iter().map(|k| u64::from(k.at)).max().unwrap_or(0));
    let bar = u64::from(song.bar_ticks());
    song.bars = end.div_ceil(bar).clamp(1, 9999) as u32;
    Ok(song)
}

/// One track's events into the song: its parts, and what's kept. Returns its last tick.
fn read_track(body: &[u8], track: u16, song: &mut Song, tempo_set: &mut bool, meter_set: &mut bool) -> Result<u64, String> {
    let mut r = Reader { bytes: body, at: 0 };
    let mut tick = 0u64;
    let mut running: Option<u8> = None;
    let mut name: Option<String> = None;
    // This track's parts, by channel: their index in the song, whether
    // the instrument is settled, and the notes sounding.
    let mut parts: [Option<usize>; 16] = [None; 16];
    let mut settled = [(false, false); 16];
    let mut open: Vec<(u8, u8, u64, u8)> = Vec::new();
    let first_part = song.parts.len();
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
                        song.bpm = if micros > 0 { (60_000_000.0 / f64::from(micros) * 100.0).round() / 100.0 } else { 120.0 };
                        *tempo_set = true;
                    }
                    0x58 if !*meter_set && data.len() >= 2 => {
                        song.beats = data[0].clamp(1, 32);
                        song.unit = 1u8 << data[1].min(5);
                        *meter_set = true;
                    }
                    _ => {
                        let mut bytes = vec![0xff, kind];
                        vlq(len, &mut bytes);
                        bytes.extend_from_slice(data);
                        song.kept.push(Kept { at: at(tick), bytes });
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
                song.kept.push(Kept { at: at(tick), bytes });
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
                        if song.parts.len() >= MOST_PARTS {
                            continue;
                        }
                        song.parts.push(Part { name: String::new(), channel: ch, bank: 0, program: 0, notes: vec![], kept: vec![] });
                        parts[usize::from(ch)] = Some(song.parts.len() - 1);
                        song.parts.len() - 1
                    }
                };
                let part = &mut song.parts[index];
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
            song.parts[i].notes.push(Note { at: at(from), length: at(tick - from).max(1), key, velocity });
        }
    }
    let mine: Vec<usize> = (first_part..song.parts.len()).collect();
    for &i in &mine {
        let ch = song.parts[i].channel;
        let base = name.clone().filter(|n| !n.is_empty());
        song.parts[i].name = match (base, mine.len() > 1) {
            (Some(n), false) => n,
            (Some(n), true) => format!("{n} ({})", channel_name(ch)),
            (None, _) if ch == DRUMS => "Drums".into(),
            (None, _) => format!("Track {} ({})", track + 1, channel_name(ch)),
        };
    }
    Ok(tick)
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

/// The song as a format 1 MIDI file.
pub fn write(song: &Song) -> Vec<u8> {
    let end = song.bars.max(1).saturating_mul(song.bar_ticks());
    let micros = (60_000_000.0 / song.bpm.clamp(10.0, 1000.0)).round() as u32;
    let mut conductor: Vec<Timed> = vec![
        (0, 0, meta(0x51, &micros.to_be_bytes()[1..])),
        (0, 0, meta(0x58, &[song.beats.max(1), song.unit.max(1).trailing_zeros() as u8, 24, 8])),
    ];
    conductor.extend(song.kept.iter().map(|k| (k.at, 1, k.bytes.clone())));
    let mut tracks = vec![track(conductor, end)];
    for part in &song.parts {
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
    out.extend_from_slice(&song.ppq.to_be_bytes());
    for t in tracks {
        out.extend(t);
    }
    out
}

/// The bars `from` to `to` (counted from 0, `to` not included) as a
/// file of their own, for Neumetik to play: each part's controllers
/// and program changes from before `from` set at the start, notes cut
/// at `to`, and a last event (an unused controller) at the very end,
/// so it's exactly as long as the bars and a loop doesn't pause.
pub fn preview(song: &Song, from: u32, to: u32) -> Vec<u8> {
    let bar = song.bar_ticks();
    let to = to.clamp(from + 1, song.bars.max(from + 1));
    let (start, end) = (from * bar, to * bar);
    let mut cut = Song { bars: to - from, kept: vec![], ..song.clone() };
    for part in &mut cut.parts {
        part.notes = part
            .notes
            .iter()
            .filter(|n| n.at >= start && n.at < end)
            .map(|n| Note { at: n.at - start, length: n.length.min(end - n.at), ..*n })
            .collect();
        // Channel messages only: what's set before plays from the start.
        part.kept = part
            .kept
            .iter()
            .filter(|k| k.at < end && k.bytes.first().is_some_and(|s| (0x80..0xf0).contains(s)))
            .map(|k| Kept { at: k.at.saturating_sub(start), bytes: k.bytes.clone() })
            .collect();
    }
    if let Some(part) = cut.parts.first_mut() {
        part.kept.push(Kept { at: end - start, bytes: vec![0xb0 | (part.channel & 0x0f), 110, 0] });
    }
    write(&cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A little format 0 file: a tempo of 100, 3/4, a piano note and a drum, a pitch bend kept.
    fn little() -> Vec<u8> {
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
        let song = read(&little()).unwrap();
        assert_eq!((song.ppq, song.bpm, song.beats, song.unit), (480, 100.0, 3, 4));
        assert_eq!(song.parts.len(), 2);
        let guitar = &song.parts[0];
        assert_eq!((guitar.channel, guitar.program), (0, 24));
        assert_eq!(guitar.notes, vec![Note { at: 0, length: 480, key: 60, velocity: 100 }]);
        assert_eq!(guitar.kept, vec![Kept { at: 480, bytes: vec![0xe0, 0, 0x50] }]);
        let drums = &song.parts[1];
        assert_eq!(drums.channel, DRUMS);
        assert_eq!(drums.name, "Drums");
        assert_eq!(drums.notes, vec![Note { at: 480, length: 96, key: 36, velocity: 90 }]);
        assert_eq!(song.bars, 1);
    }

    #[test]
    fn writes_what_it_reads() {
        let song = read(&little()).unwrap();
        let again = read(&write(&song)).unwrap();
        assert_eq!(again.bpm, song.bpm);
        assert_eq!((again.beats, again.unit, again.bars), (3, 4, 1));
        assert_eq!(again.parts.len(), 2);
        for (a, b) in again.parts.iter().zip(&song.parts) {
            assert_eq!((a.channel, a.program, &a.notes, &a.kept), (b.channel, b.program, &b.notes, &b.kept));
        }
        // Neumetik can play it, and it's as long as its bar.
        let samples = neumetik_midi::render(&preview(&again, 0, 1), 22_050, 0.0, 60.0).unwrap();
        let seconds = samples.len() as f64 / 2.0 / 22_050.0;
        assert!((seconds - 1.8).abs() < 0.01, "3 beats at 100: {seconds}");
    }

    #[test]
    fn a_new_song_round_trips() {
        let mut song = Song::new();
        song.parts[0].notes.push(Note { at: 960, length: 240, key: 64, velocity: 80 });
        song.parts[1].bank = 0;
        song.parts[1].program = 25;
        song.bars = 8;
        let back = read(&write(&song)).unwrap();
        assert_eq!(back.bars, 8, "the length is kept by the tracks' end");
        assert_eq!(back.parts[0].notes, song.parts[0].notes);
        assert_eq!(back.parts[0].name, "Piano");
        assert_eq!(back.parts[1].program, 25);
        assert_eq!(back.parts[1].name, "Drums");
    }

    #[test]
    fn the_preview_cuts_the_bars() {
        let mut song = Song::new();
        song.parts[0].notes = vec![
            Note { at: 0, length: 480, key: 60, velocity: 80 },
            Note { at: 1920 + 1440, length: 960, key: 62, velocity: 80 },
        ];
        let bar = read(&preview(&song, 1, 2)).unwrap();
        assert_eq!(bar.parts[0].notes, vec![Note { at: 1440, length: 480, key: 62, velocity: 80 }]);
        assert_eq!(bar.bars, 1);
    }

    #[test]
    fn not_midi() {
        assert!(read(b"hello").is_err());
        assert!(read(b"MThd\0\0\0\x06\0\0\0\x01\xe7\x28").is_err(), "SMPTE timing");
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

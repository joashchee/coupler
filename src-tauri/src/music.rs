//! **The Music Editor's song**: a MIDI file in the Assets folder read
//! into what the editor shows (parts, bars and notes) and written back,
//! pure and unit-tested. Neumetik reads and writes the file
//! (`neumetik_midi::Score`, this Song without serde), and the song as
//! TMF, the Text Music Format, for a journal entry (`to_tmf`, `from_tmf`).
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

use neumetik_midi::score::{self, Score};
use neumetik_midi::tmf;
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
pub const DRUMS: u8 = score::DRUMS;

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

// The MIDI file is read and written by Neumetik (`neumetik_midi::score`),
// whose Score is this Song without serde; TMF is Neumetik's too.

impl From<Score> for Song {
    fn from(s: Score) -> Song {
        let kept = |k: Vec<score::Kept>| k.into_iter().map(|k| Kept { at: k.at, bytes: k.bytes }).collect();
        Song {
            ppq: s.ppq,
            bpm: s.bpm,
            beats: s.beats,
            unit: s.unit,
            bars: s.bars,
            parts: s
                .parts
                .into_iter()
                .map(|p| Part {
                    name: p.name,
                    channel: p.channel,
                    bank: p.bank,
                    program: p.program,
                    notes: p.notes.into_iter().map(|n| Note { at: n.at, length: n.length, key: n.key, velocity: n.velocity }).collect(),
                    kept: kept(p.kept),
                })
                .collect(),
            kept: kept(s.kept),
        }
    }
}

impl From<&Song> for Score {
    fn from(s: &Song) -> Score {
        let kept = |k: &[Kept]| k.iter().map(|k| score::Kept { at: k.at, bytes: k.bytes.clone() }).collect();
        Score {
            ppq: s.ppq,
            bpm: s.bpm,
            beats: s.beats,
            unit: s.unit,
            bars: s.bars,
            parts: s
                .parts
                .iter()
                .map(|p| score::Part {
                    name: p.name.clone(),
                    channel: p.channel,
                    bank: p.bank,
                    program: p.program,
                    notes: p.notes.iter().map(|n| score::Note { at: n.at, length: n.length, key: n.key, velocity: n.velocity }).collect(),
                    kept: kept(&p.kept),
                })
                .collect(),
            kept: kept(&s.kept),
        }
    }
}

/// A MIDI file's song.
pub fn read(bytes: &[u8]) -> Result<Song, String> {
    Score::from_midi(bytes).map(Song::from).map_err(|e| match e {
        neumetik_midi::Error::BadTiming => "it's timed in film frames, not beats, which the editor doesn't read".to_string(),
        e => e.to_string(),
    })
}

/// The song as a format 1 MIDI file.
pub fn write(song: &Song) -> Vec<u8> {
    Score::from(song).to_midi()
}

/// The song and its title as TMF, the Text Music Format
/// (`neumetik_midi::tmf`): what a journal entry holds.
pub fn to_tmf(song: &Song, title: &str) -> String {
    tmf::write(&Score::from(song), title)
}

/// The song and title in TMF text, wherever in the text it is.
pub fn from_tmf(text: &str) -> Result<(Song, String), String> {
    tmf::read(text).map(|(s, title)| (Song::from(s), title)).map_err(|e| e.to_string())
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

// ---- The same music saved twice ----

/// One part's music: its channel and instrument, its notes and its
/// other channel events, in order, on a common grid.
type PartMusic = (u8, u8, u8, Vec<(u64, u64, u8, u8)>, Vec<(u64, Vec<u8>)>);
/// A song's music: its tempo, meter, bars, parts and own events.
type SongMusic = (u64, u8, u8, u32, Vec<PartMusic>, Vec<(u64, Vec<u8>)>);

/// A song's music alone, ticks scaled by `scale`: the tempo (as a file
/// holds it, microseconds a beat), the meter, the bars, the parts that
/// hold anything, in no order, and the song's own events. Left out:
/// every name (the song's, the parts'), every meta event (titles, text,
/// copyright, markers) and what a part holds nothing of.
fn music_of(song: &Song, scale: u64) -> SongMusic {
    let events = |kept: &[Kept]| {
        let mut events: Vec<(u64, Vec<u8>)> = kept.iter().filter(|k| k.bytes.first() != Some(&0xff)).map(|k| (u64::from(k.at) * scale, k.bytes.clone())).collect();
        events.sort();
        events
    };
    let mut parts: Vec<PartMusic> = song
        .parts
        .iter()
        .map(|p| {
            let mut notes: Vec<_> = p.notes.iter().map(|n| (u64::from(n.at) * scale, u64::from(n.length) * scale, n.key, n.velocity)).collect();
            notes.sort();
            (p.channel, p.bank, p.program, notes, events(&p.kept))
        })
        .filter(|(_, _, _, notes, kept)| !notes.is_empty() || !kept.is_empty())
        .collect();
    parts.sort();
    let tempo = if song.bpm > 0.0 { (60_000_000.0 / song.bpm).round() as u64 } else { 0 };
    (tempo, song.beats, song.unit, song.bars, parts, events(&song.kept))
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Whether two songs are the same music, whatever they're called: the
/// same notes on the same instruments at the same tempo and meter, the
/// same length, however finely each file counts its beats.
pub fn same_music(a: &Song, b: &Song) -> bool {
    let (pa, pb) = (u64::from(a.ppq.max(1)), u64::from(b.ppq.max(1)));
    let lcm = pa / gcd(pa, pb) * pb;
    music_of(a, lcm / pa) == music_of(b, lcm / pb)
}

// ---- What the parts play ----

/// What the Music Editor's parts can play, bank by bank, and the drum
/// kits: Neumetik's own (General MIDI, the GS variation tones, the
/// classic synths), or, through a SoundFont or DLS bank, what that bank
/// has as well, each named as it plays (the bank's preset where it has
/// one, else Neumetik's, as Neumetik MIDI picks them).
#[derive(Serialize)]
pub struct Instruments {
    banks: Vec<InstrumentBank>,
    kits: Vec<(u8, String)>,
}

/// A bank's programs and their names; every bank but General MIDI's
/// and the classic synths' has only some.
#[derive(Serialize)]
pub struct InstrumentBank {
    msb: u8,
    programs: Vec<(u8, String)>,
}

pub fn instruments(bank: Option<std::sync::Arc<neumetik_midi::SoundBank>>) -> Instruments {
    use std::collections::{BTreeMap, BTreeSet};
    // Each bank's programs: Neumetik's, the bank's own too.
    let mut programs: BTreeMap<u8, BTreeSet<u8>> = BTreeMap::new();
    for b in &neumetik_midi::BANKS {
        programs.entry(b.msb).or_default().extend(0..b.patches.len() as u8);
    }
    for (msb, program, _) in neumetik_midi::variations() {
        programs.entry(msb).or_default().insert(program);
    }
    let mut kits: BTreeSet<u8> = neumetik_midi::drums::KITS.iter().map(|(p, _)| *p).collect();
    if let Some(bank) = &bank {
        for i in bank.instruments() {
            if i.drums {
                kits.insert(i.program);
            } else if i.bank < 128 {
                programs.entry(i.bank).or_default().insert(i.program);
            }
        }
    }
    // Named by what plays them, as the synthesizer chooses.
    let mut synth = neumetik_midi::Synth::new(neumetik_midi::MIN_RATE);
    synth.set_sound_bank(bank);
    let mut name = |channel: u8, msb: u8, program: u8| {
        synth.set_instrument(channel, msb, program);
        synth.instrument_name(channel).to_string()
    };
    Instruments {
        banks: programs.into_iter().map(|(msb, ps)| InstrumentBank { msb, programs: ps.into_iter().map(|p| (p, name(0, msb, p))).collect() }).collect(),
        kits: kits.into_iter().map(|p| (p, name(9, 0, p))).collect(),
    }
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
    fn the_same_music_whatever_its_called() {
        let song = read(&little()).unwrap();
        let mut renamed = song.clone();
        renamed.parts[0].name = "Lead".into();
        renamed.parts.reverse();
        // A title (a meta event) and an empty part aren't music.
        renamed.kept.push(Kept { at: 0, bytes: vec![0xff, 0x03, 2, b'H', b'i'] });
        renamed.parts.push(Part { name: "Empty".into(), channel: 3, bank: 0, program: 5, notes: vec![], kept: vec![] });
        assert!(same_music(&song, &renamed));
        // Twice as fine a grid, the same music.
        let mut finer = song.clone();
        finer.ppq *= 2;
        for part in &mut finer.parts {
            part.notes.iter_mut().for_each(|n| (n.at, n.length) = (n.at * 2, n.length * 2));
            part.kept.iter_mut().for_each(|k| k.at *= 2);
        }
        assert!(same_music(&song, &finer));
        // As written to a file and read back.
        assert!(same_music(&song, &read(&write(&renamed)).unwrap()));
        // A note, an instrument, the tempo: different music.
        let mut moved = song.clone();
        moved.parts[0].notes[0].key += 1;
        assert!(!same_music(&song, &moved));
        let mut other = song.clone();
        other.parts[0].program += 1;
        assert!(!same_music(&song, &other));
        let mut faster = song.clone();
        faster.bpm += 10.0;
        assert!(!same_music(&song, &faster));
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
    fn a_song_round_trips_through_tmf() {
        let mut song = read(&little()).unwrap();
        song.parts[0].notes.push(Note { at: 1440, length: 30, key: 64, velocity: 7 });
        song.parts[0].name = "Gittern".into();
        song.bars = 3;
        let text = to_tmf(&song, "A Clan Song");
        assert_eq!(from_tmf(&text).unwrap(), (song.clone(), "A Clan Song".to_string()));
        // As CoffeeMUD prints a journal entry: a header, wrapped at 40 columns, the prompt after.
        let mut entry = String::from("FROM: Bob\r\nSUBJ: A Clan Song\r\n");
        let mut column = 0;
        for word in text.split(' ') {
            if column > 0 && column + word.len() > 40 {
                entry.push_str("\r\n");
                column = 0;
            } else if column > 0 {
                entry.push(' ');
                column += 1;
            }
            entry.push_str(word);
            column += word.len();
        }
        entry.push_str("\r\nR)eply N)ext or RETURN: ");
        let (back, title) = from_tmf(&entry).unwrap();
        assert_eq!((&back, title.as_str()), (&song, "A Clan Song"));
        // And the MIDI file it saves is the one the song makes.
        assert_eq!(write(&back), write(&song));
    }

    #[test]
    fn instruments_are_named_as_they_play() {
        let own = instruments(None);
        let gm = own.banks.iter().find(|b| b.msb == 0).unwrap();
        assert_eq!(gm.programs.len(), 128);
        assert!(own.banks.iter().any(|b| b.msb == 80) && own.banks.iter().any(|b| (1..64).contains(&b.msb)), "the classic synths and the GS variations");
        assert!(own.kits.iter().any(|(p, _)| *p == 0));
        // Through a bank: its piano is its own sine; the rest, Neumetik's.
        let bank = std::sync::Arc::new(neumetik_midi::SoundBank::from_bytes(&neumetik_midi::doc_bank()).unwrap());
        let through = instruments(Some(bank));
        let gm = through.banks.iter().find(|b| b.msb == 0).unwrap();
        assert_eq!(gm.programs[0], (0, "Sine".to_string()));
        assert_eq!(gm.programs[1].1, own.banks[0].programs[1].1);
    }
}

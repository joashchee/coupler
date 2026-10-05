//! Coupler's composer: a short piece of looping music made from a few
//! words the player types (gear's Hooks, Assets, Create Asset), written
//! as a MIDI file for Neumetik to play (`vendor/neumetik`). No model and no
//! samples: the words pick a mood (a mode, a tempo, a meter, the
//! instruments and the drums), then a seeded hand writes eight bars of
//! chords, a bass, a pad, a melody and sometimes a broken chord over
//! them. Pure and unit-tested.
//!
//! **The words.** A mood's words (`MOODS`: "tavern", "battle",
//! "dungeon", "sea"...), the first one named winning a tie; an
//! instrument's name puts it on the tune (`INSTRUMENTS`); "slow",
//! "fast", "soft", "loud", "major", "minor", "waltz", "jig", "march"
//! and a number of beats a minute ("90 bpm") change what the mood
//! would do. Words it doesn't know are only the seed, so the same words
//! always make the same piece, and `take` makes another.
//!
//! **The loop.** Eight bars, the last chord leading back to the first,
//! and every note ends by the last bar's end: an event sits exactly on
//! it, so the file's length is the loop's and Neumetik (or a SoundFont)
//! plays it round with no gap (`assets::audio`, `looping`).

use serde::Serialize;

/// Ticks a quarter note.
const DIVISION: u32 = 480;
/// Bars in the loop.
const BARS: u32 = 8;
/// The drum part's channel (GM's tenth).
const DRUMS: u8 = 9;

/// A piece made: the MIDI file, and what it is in words.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Piece {
    #[serde(skip)]
    pub midi: Vec<u8>,
    /// "A dark piece in E Phrygian, 68 beats a minute in 4/4, for oboe,
    /// choir and contrabass, with low drums: 8 bars, 28 seconds, looping."
    pub about: String,
    pub seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Ionian,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    /// Harmonic minor from its fifth: the desert's.
    PhrygianDominant,
}

impl Mode {
    fn steps(self) -> [u8; 7] {
        match self {
            Mode::Ionian => [0, 2, 4, 5, 7, 9, 11],
            Mode::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Mode::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Mode::Lydian => [0, 2, 4, 6, 7, 9, 11],
            Mode::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
            Mode::Aeolian => [0, 2, 3, 5, 7, 8, 10],
            Mode::PhrygianDominant => [0, 1, 4, 5, 7, 8, 10],
        }
    }

    fn name(self) -> &'static str {
        match self {
            Mode::Ionian => "major",
            Mode::Dorian => "Dorian",
            Mode::Phrygian => "Phrygian",
            Mode::Lydian => "Lydian",
            Mode::Mixolydian => "Mixolydian",
            Mode::Aeolian => "minor",
            Mode::PhrygianDominant => "Phrygian dominant",
        }
    }

    /// Four chords (scale degrees from 0) the first half of the loop
    /// takes one of; the second half's last is the turnaround.
    fn progressions(self) -> &'static [[u8; 4]] {
        match self {
            Mode::Ionian => &[[0, 4, 5, 3], [0, 3, 4, 3], [0, 5, 3, 4], [0, 3, 0, 4], [5, 3, 0, 4]],
            Mode::Lydian => &[[0, 1, 0, 1], [0, 1, 4, 0], [0, 1, 5, 1]],
            Mode::Mixolydian => &[[0, 6, 3, 0], [0, 3, 6, 0], [0, 6, 0, 3]],
            Mode::Dorian => &[[0, 3, 0, 3], [0, 6, 3, 0], [0, 2, 3, 0]],
            Mode::Aeolian => &[[0, 5, 2, 6], [0, 3, 5, 6], [0, 5, 3, 4], [0, 6, 5, 4]],
            Mode::Phrygian => &[[0, 1, 0, 6], [0, 1, 6, 0], [0, 6, 1, 0]],
            Mode::PhrygianDominant => &[[0, 1, 0, 1], [0, 1, 6, 0], [0, 6, 1, 0]],
        }
    }

    /// The chord that leads back to the first: the fifth, the seventh, or
    /// the Phrygian second.
    fn turnaround(self) -> u8 {
        match self {
            Mode::Ionian | Mode::Lydian => 4,
            Mode::Phrygian | Mode::PhrygianDominant => 1,
            Mode::Dorian | Mode::Mixolydian | Mode::Aeolian => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meter {
    Four,
    Three,
    SixEight,
}

impl Meter {
    /// Eighth notes in a bar.
    fn eighths(self) -> u32 {
        match self {
            Meter::Four => 8,
            Meter::Three | Meter::SixEight => 6,
        }
    }

    /// Where the beats fall, in eighths.
    fn beats(self) -> &'static [u32] {
        match self {
            Meter::Four => &[0, 2, 4, 6],
            Meter::Three => &[0, 2, 4],
            Meter::SixEight => &[0, 3],
        }
    }

    fn name(self) -> &'static str {
        match self {
            Meter::Four => "4/4",
            Meter::Three => "3/4",
            Meter::SixEight => "6/8",
        }
    }

    /// The melody's rhythms for a bar, in eighths.
    fn rhythms(self) -> &'static [&'static [u32]] {
        match self {
            Meter::Four => &[&[2, 2, 2, 2], &[3, 1, 2, 2], &[2, 1, 1, 2, 2], &[4, 2, 2], &[2, 2, 4], &[1, 1, 2, 2, 2], &[6, 2], &[3, 3, 2]],
            Meter::Three => &[&[2, 2, 2], &[4, 2], &[3, 1, 2], &[2, 1, 1, 2], &[2, 4]],
            Meter::SixEight => &[&[3, 3], &[2, 1, 3], &[2, 1, 2, 1], &[3, 2, 1], &[1, 1, 1, 3]],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bass {
    /// A whole bar.
    Held,
    /// The root on each beat, the fifth between.
    Walk,
    /// Eighths on the root.
    Drive,
    /// The root and its octave by turns, in eighths.
    Bounce,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drums {
    None,
    /// A bodhrán's low tom and a tambourine.
    Folk,
    /// Kick, snare, hats, a crash and a fill.
    Battle,
    /// A snare in eighths and the bass drum on the strong beats.
    March,
    /// A low tom now and then, far off.
    Low,
    /// Congas and bongos.
    Hand,
    /// Four on the floor on a drum machine.
    Dance,
    /// Brushes: a soft kick and a swish.
    Brush,
}

impl Drums {
    fn name(self) -> Option<&'static str> {
        match self {
            Drums::None => None,
            Drums::Folk => Some("a frame drum and tambourine"),
            Drums::Battle => Some("battle drums"),
            Drums::March => Some("a marching snare"),
            Drums::Low => Some("low drums"),
            Drums::Hand => Some("hand drums"),
            Drums::Dance => Some("an analog drum machine"),
            Drums::Brush => Some("brushes"),
        }
    }

    /// The kit's program on the drum part (`neumetik::drums::KITS`).
    fn kit(self) -> u8 {
        match self {
            Drums::Battle | Drums::Low | Drums::March => 48,
            Drums::Dance => 25,
            Drums::Brush => 40,
            _ => 0,
        }
    }
}

/// What a mood plays. Instruments are GM programs.
struct Mood {
    name: &'static str,
    words: &'static [&'static str],
    mode: Mode,
    tempo: u32,
    meter: Meter,
    lead: u8,
    pad: u8,
    bass: u8,
    arp: Option<u8>,
    bass_style: Bass,
    drums: Drums,
}

const MOODS: [Mood; 14] = [
    Mood {
        name: "peaceful",
        words: &["peaceful", "calm", "town", "village", "home", "pastoral", "meadow", "field", "fields", "farm", "morning", "sunny", "hope"],
        mode: Mode::Ionian,
        tempo: 88,
        meter: Meter::Four,
        lead: 73,
        pad: 48,
        bass: 32,
        arp: Some(46),
        bass_style: Bass::Held,
        drums: Drums::None,
    },
    Mood {
        name: "merry",
        words: &["tavern", "inn", "jolly", "cheerful", "merry", "dance", "dancing", "drinking", "festival", "feast", "happy", "celebration", "fair", "bard"],
        mode: Mode::Mixolydian,
        tempo: 128,
        meter: Meter::SixEight,
        lead: 110,
        pad: 21,
        bass: 32,
        arp: Some(24),
        bass_style: Bass::Walk,
        drums: Drums::Folk,
    },
    Mood {
        name: "dark",
        words: &["dark", "dungeon", "crypt", "tomb", "evil", "haunted", "undead", "necromancer", "shadow", "shadows", "cursed", "sinister", "creepy", "horror", "ghost", "ghosts", "lair", "demon"],
        mode: Mode::Phrygian,
        tempo: 66,
        meter: Meter::Four,
        lead: 68,
        pad: 52,
        bass: 43,
        arp: None,
        bass_style: Bass::Held,
        drums: Drums::Low,
    },
    Mood {
        name: "fierce",
        words: &["battle", "fight", "fighting", "war", "combat", "boss", "siege", "attack", "charge", "danger", "chase", "duel", "dragon", "monster", "army"],
        mode: Mode::Aeolian,
        tempo: 152,
        meter: Meter::Four,
        lead: 61,
        pad: 48,
        bass: 48,
        arp: Some(44),
        bass_style: Bass::Drive,
        drums: Drums::Battle,
    },
    Mood {
        name: "heroic",
        words: &["heroic", "epic", "hero", "glory", "triumph", "triumphant", "quest", "adventure", "kingdom", "royal", "castle", "knight", "knights", "victory", "king", "queen"],
        mode: Mode::Ionian,
        tempo: 108,
        meter: Meter::Four,
        lead: 60,
        pad: 48,
        bass: 58,
        arp: None,
        bass_style: Bass::Walk,
        drums: Drums::March,
    },
    Mood {
        name: "mysterious",
        words: &["mystery", "mysterious", "magic", "magical", "arcane", "wizard", "enchanted", "spell", "fairy", "fae", "dream", "dreamy", "ethereal", "mystic", "mage", "tower"],
        mode: Mode::Lydian,
        tempo: 80,
        meter: Meter::Four,
        lead: 8,
        pad: 89,
        bass: 35,
        arp: Some(10),
        bass_style: Bass::Held,
        drums: Drums::None,
    },
    Mood {
        name: "sad",
        words: &["sad", "lament", "sorrow", "funeral", "grief", "mourning", "loss", "melancholy", "tears", "lonely", "farewell", "ruins"],
        mode: Mode::Aeolian,
        tempo: 64,
        meter: Meter::Three,
        lead: 42,
        pad: 49,
        bass: 43,
        arp: Some(0),
        bass_style: Bass::Held,
        drums: Drums::None,
    },
    Mood {
        name: "seafaring",
        words: &["sea", "ocean", "sailing", "sail", "ship", "pirate", "pirates", "harbor", "harbour", "waves", "shanty", "port", "docks", "sailor", "sailors", "island"],
        mode: Mode::Dorian,
        tempo: 104,
        meter: Meter::SixEight,
        lead: 21,
        pad: 48,
        bass: 32,
        arp: Some(24),
        bass_style: Bass::Walk,
        drums: Drums::Folk,
    },
    Mood {
        name: "woodland",
        words: &["forest", "woods", "wood", "elven", "elf", "elves", "nature", "grove", "leaves", "spring", "wild", "glade", "trees", "druid"],
        mode: Mode::Dorian,
        tempo: 92,
        meter: Meter::Three,
        lead: 75,
        pad: 52,
        bass: 32,
        arp: Some(46),
        bass_style: Bass::Held,
        drums: Drums::None,
    },
    Mood {
        name: "desert",
        words: &["desert", "sand", "sands", "eastern", "bazaar", "oasis", "dunes", "nomad", "nomads", "caravan", "pyramid", "pharaoh"],
        mode: Mode::PhrygianDominant,
        tempo: 100,
        meter: Meter::Four,
        lead: 104,
        pad: 48,
        bass: 32,
        arp: Some(15),
        bass_style: Bass::Walk,
        drums: Drums::Hand,
    },
    Mood {
        name: "wintry",
        words: &["snow", "snowy", "ice", "icy", "winter", "frozen", "frost", "north", "northern", "glacier", "tundra"],
        mode: Mode::Aeolian,
        tempo: 76,
        meter: Meter::Three,
        lead: 9,
        pad: 92,
        bass: 32,
        arp: Some(10),
        bass_style: Bass::Held,
        drums: Drums::None,
    },
    Mood {
        name: "nighttime",
        words: &["night", "lullaby", "sleep", "sleepy", "moon", "moonlight", "stars", "starry", "evening", "dusk", "twilight", "midnight"],
        mode: Mode::Ionian,
        tempo: 66,
        meter: Meter::Three,
        lead: 11,
        pad: 89,
        bass: 32,
        arp: Some(46),
        bass_style: Bass::Held,
        drums: Drums::Brush,
    },
    Mood {
        name: "electronic",
        words: &["synth", "synthwave", "eighties", "80s", "retro", "techno", "cyber", "cyberpunk", "neon", "space", "future", "robot", "robots", "electronic", "spaceport", "starship"],
        mode: Mode::Aeolian,
        tempo: 118,
        meter: Meter::Four,
        lead: 81,
        pad: 90,
        bass: 38,
        arp: Some(80),
        bass_style: Bass::Bounce,
        drums: Drums::Dance,
    },
    Mood {
        name: "holy",
        words: &["holy", "temple", "church", "cathedral", "prayer", "divine", "sacred", "angel", "angels", "heaven", "chapel", "monastery", "shrine", "priest"],
        mode: Mode::Mixolydian,
        tempo: 66,
        meter: Meter::Four,
        lead: 19,
        pad: 52,
        bass: 19,
        arp: None,
        bass_style: Bass::Held,
        drums: Drums::None,
    },
];

/// Instruments by name, for the tune: GM programs, and the name said.
const INSTRUMENTS: [(&str, u8, &str); 46] = [
    ("piano", 0, "piano"),
    ("harpsichord", 6, "harpsichord"),
    ("celesta", 8, "celesta"),
    ("glockenspiel", 9, "glockenspiel"),
    ("musicbox", 10, "music box"),
    ("vibraphone", 11, "vibraphone"),
    ("marimba", 12, "marimba"),
    ("xylophone", 13, "xylophone"),
    ("bells", 14, "bells"),
    ("chimes", 14, "chimes"),
    ("dulcimer", 15, "dulcimer"),
    ("organ", 19, "organ"),
    ("accordion", 21, "accordion"),
    ("harmonica", 22, "harmonica"),
    ("guitar", 24, "guitar"),
    ("lute", 24, "lute"),
    ("harp", 46, "harp"),
    ("violin", 40, "violin"),
    ("fiddle", 110, "fiddle"),
    ("viola", 41, "viola"),
    ("cello", 42, "cello"),
    ("strings", 48, "strings"),
    ("choir", 52, "choir"),
    ("voices", 52, "voices"),
    ("trumpet", 56, "trumpet"),
    ("trombone", 57, "trombone"),
    ("tuba", 58, "tuba"),
    ("horn", 60, "horn"),
    ("horns", 60, "horns"),
    ("brass", 61, "brass"),
    ("sax", 65, "sax"),
    ("saxophone", 65, "saxophone"),
    ("oboe", 68, "oboe"),
    ("bassoon", 70, "bassoon"),
    ("clarinet", 71, "clarinet"),
    ("piccolo", 72, "piccolo"),
    ("flute", 73, "flute"),
    ("recorder", 74, "recorder"),
    ("panpipes", 75, "pan pipes"),
    ("shakuhachi", 77, "shakuhachi"),
    ("ocarina", 79, "ocarina"),
    ("sitar", 104, "sitar"),
    ("banjo", 105, "banjo"),
    ("koto", 107, "koto"),
    ("kalimba", 108, "kalimba"),
    ("bagpipes", 109, "bagpipes"),
];

/// GM programs' names, as said in what was made.
fn program_name(program: u8) -> &'static str {
    if let Some((_, _, name)) = INSTRUMENTS.iter().find(|(_, p, _)| *p == program) {
        return name;
    }
    match program {
        32 => "upright bass",
        35 => "fretless bass",
        38 => "synth bass",
        43 => "contrabass",
        44 => "tremolo strings",
        49 => "slow strings",
        80 => "square lead",
        81 => "saw lead",
        89 => "warm pad",
        90 => "polysynth",
        92 => "bowed pad",
        _ => "an instrument",
    }
}

const NOTE_NAMES: [&str; 12] = ["C", "D flat", "D", "E flat", "E", "F", "F sharp", "G", "A flat", "A", "B flat", "B"];

/// The prompt's words, lower case, letters and digits only ("music box" is also "musicbox").
fn words(prompt: &str) -> Vec<String> {
    let mut out: Vec<String> = prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let pairs: Vec<String> = out.windows(2).map(|w| format!("{}{}", w[0], w[1])).collect();
    for (i, pair) in pairs.into_iter().enumerate() {
        if matches!(pair.as_str(), "musicbox" | "panpipes" | "panflute") {
            out[i] = if pair == "panflute" { "panpipes".into() } else { pair };
        }
    }
    out
}

/// FNV-1a of the words and the take: the same words, the same piece.
fn seed(words: &[String], take: u32) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in words.join(" ").bytes().chain(take.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// SplitMix64: small, and the same everywhere.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    fn pick<'a, T>(&mut self, from: &'a [T]) -> &'a T {
        &from[self.below(from.len())]
    }
}

/// The mood the words name: most of its words, the first named on a tie; peaceful if none.
fn mood_of(words: &[String]) -> &'static Mood {
    let first = |m: &Mood| words.iter().position(|w| m.words.contains(&w.as_str())).unwrap_or(usize::MAX);
    let count = |m: &Mood| words.iter().filter(|w| m.words.contains(&w.as_str())).count();
    MOODS.iter().filter(|m| count(m) > 0).max_by_key(|m| (count(m), std::cmp::Reverse(first(m)))).unwrap_or(&MOODS[0])
}

/// The plan the words make, before the notes.
struct Plan {
    mood: &'static Mood,
    mode: Mode,
    tempo: u32,
    meter: Meter,
    lead: u8,
    /// Loudness: soft 0.75, loud 1.15.
    level: f32,
}

fn plan(words: &[String]) -> Plan {
    let mood = mood_of(words);
    let has = |w: &str| words.iter().any(|x| x == w);
    let mut tempo = mood.tempo;
    if ["slow", "slowly", "lazy", "relaxed"].iter().any(|w| has(w)) {
        tempo = tempo * 3 / 4;
    }
    if ["fast", "quick", "rapid", "frantic", "lively", "upbeat"].iter().any(|w| has(w)) {
        tempo = tempo * 5 / 4;
    }
    // "90 bpm", "at 90 beats".
    for pair in words.windows(2) {
        if matches!(pair[1].as_str(), "bpm" | "beats") {
            if let Ok(n) = pair[0].parse::<u32>() {
                tempo = n;
            }
        }
    }
    for w in words {
        if let Some(n) = w.strip_suffix("bpm").and_then(|n| n.parse::<u32>().ok()) {
            tempo = n;
        }
    }
    let mut mode = mood.mode;
    if has("minor") {
        mode = Mode::Aeolian;
    } else if has("major") {
        mode = Mode::Ionian;
    }
    let mut meter = mood.meter;
    if has("waltz") {
        meter = Meter::Three;
    } else if has("jig") || has("shanty") {
        meter = Meter::SixEight;
    } else if has("march") {
        meter = Meter::Four;
    }
    let lead = words.iter().find_map(|w| INSTRUMENTS.iter().find(|(name, _, _)| name == w).map(|(_, p, _)| *p)).unwrap_or(mood.lead);
    let level = if ["soft", "quiet", "gentle", "hushed", "distant"].iter().any(|w| has(w)) {
        0.75
    } else if ["loud", "mighty", "thunderous", "powerful"].iter().any(|w| has(w)) {
        1.15
    } else {
        1.0
    };
    Plan { mood, mode, tempo: tempo.clamp(40, 220), meter, lead, level }
}

/// A MIDI event at a tick. `order` puts note-offs before anything else
/// at the same tick, and note-ons last.
struct Event {
    tick: u32,
    order: u8,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct Score(Vec<Event>);

impl Score {
    fn setup(&mut self, bytes: Vec<u8>) {
        self.0.push(Event { tick: 0, order: 1, bytes });
    }

    fn note(&mut self, channel: u8, key: i32, start: u32, length: u32, velocity: f32) {
        let key = key.clamp(0, 127) as u8;
        let velocity = velocity.round().clamp(1.0, 127.0) as u8;
        self.0.push(Event { tick: start, order: 2, bytes: vec![0x90 | channel, key, velocity] });
        self.0.push(Event { tick: start + length.max(1), order: 0, bytes: vec![0x80 | channel, key, 0] });
    }

    /// Format 0, one track, every event in time order.
    fn file(mut self, tempo: u32) -> Vec<u8> {
        self.0.sort_by_key(|e| (e.tick, e.order));
        let mut track = Vec::new();
        let micros = 60_000_000 / tempo;
        track.extend_from_slice(&[0, 0xff, 0x51, 3]);
        track.extend_from_slice(&micros.to_be_bytes()[1..]);
        let mut at = 0;
        for e in &self.0 {
            vlq(&mut track, e.tick - at);
            track.extend_from_slice(&e.bytes);
            at = e.tick;
        }
        track.extend_from_slice(&[0, 0xff, 0x2f, 0]);
        let mut out = b"MThd".to_vec();
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&(DIVISION as u16).to_be_bytes());
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&(track.len() as u32).to_be_bytes());
        out.extend(track);
        out
    }
}

/// A MIDI variable-length number.
fn vlq(out: &mut Vec<u8>, mut n: u32) {
    let mut bytes = vec![(n & 0x7f) as u8];
    n >>= 7;
    while n > 0 {
        bytes.push((n & 0x7f) as u8 | 0x80);
        n >>= 7;
    }
    bytes.reverse();
    out.extend(bytes);
}

/// The key a scale step plays: `step` from the tonic (negative below),
/// the tonic being `tonic`.
fn key(steps: &[u8; 7], tonic: i32, step: i32) -> i32 {
    let octave = step.div_euclid(7);
    tonic + octave * 12 + i32::from(steps[step.rem_euclid(7) as usize])
}

/// The steps of a chord on a degree: its root, third and fifth.
fn chord(degree: u8) -> [i32; 3] {
    let d = i32::from(degree);
    [d, d + 2, d + 4]
}

/// Whether a step is one of a chord's, in any octave.
fn in_chord(step: i32, degree: u8) -> bool {
    chord(degree).iter().any(|c| (step - c).rem_euclid(7) == 0)
}

/// Makes a piece from the player's words; `take` makes another from the same words.
pub fn compose(prompt: &str, take: u32) -> Piece {
    let words = words(prompt);
    let plan = plan(&words);
    let mut rng = Rng(seed(&words, take));
    let mood = plan.mood;
    let steps = plan.mode.steps();
    let root = rng.below(12) as i32;
    // The tune's tonic between F above middle C and E an octave up; the rest below.
    let tonic = 60 + (root + 7) % 12 - 2;
    let eighth = DIVISION / 2;
    let bar = plan.meter.eighths() * eighth;
    let end = BARS * bar;

    // The chords: a progression twice, the second ending on the turnaround.
    let progression = *rng.pick(plan.mode.progressions());
    let mut chords: Vec<u8> = progression.iter().chain(progression.iter()).copied().collect();
    if chords[7] == 0 || rng.chance(60) {
        chords[7] = plan.mode.turnaround();
    }

    let mut score = Score::default();
    let (lead, pad, bass, arp) = (0u8, 1u8, 2u8, 3u8);
    let reverb = if mood.drums == Drums::Dance { 40 } else { 80 };
    let parts: [(u8, u8, u8, u8); 3] = [(lead, plan.lead, 100, 64), (pad, mood.pad, 78, 44), (bass, mood.bass, 100, 64)];
    for (channel, program, volume, pan) in parts {
        score.setup(vec![0xc0 | channel, program]);
        score.setup(vec![0xb0 | channel, 7, volume]);
        score.setup(vec![0xb0 | channel, 10, pan]);
        score.setup(vec![0xb0 | channel, 91, reverb]);
    }
    if let Some(program) = mood.arp {
        score.setup(vec![0xc0 | arp, program]);
        score.setup(vec![0xb0 | arp, 7, 72]);
        score.setup(vec![0xb0 | arp, 10, 84]);
        score.setup(vec![0xb0 | arp, 91, reverb]);
    }
    if mood.drums != Drums::None {
        score.setup(vec![0xc0 | DRUMS, mood.drums.kit()]);
        score.setup(vec![0xb0 | DRUMS, 7, 96]);
        score.setup(vec![0xb0 | DRUMS, 91, reverb / 2]);
    }
    score.setup(vec![0xb0 | pad, 93, 50]);
    let level = plan.level;

    // The pad: each bar's chord held, each voice moving as little as it can.
    let mut voicing: Vec<i32> = chord(chords[0]).iter().map(|&s| key(&steps, tonic - 12, s)).collect();
    for (b, &degree) in chords.iter().enumerate() {
        let mut next = Vec::new();
        for (i, &s) in chord(degree).iter().enumerate() {
            let was = voicing[i];
            let near = (-2..=2).map(|o| key(&steps, tonic - 12, s + o * 7)).min_by_key(|k| (k - was).abs()).unwrap_or(was);
            next.push(near.clamp(48, 72));
        }
        voicing = next;
        for &k in &voicing {
            score.note(pad, k, b as u32 * bar, bar, 62.0 * level);
        }
    }

    // The bass.
    for (b, &degree) in chords.iter().enumerate() {
        let start = b as u32 * bar;
        let low = key(&steps, tonic - 24, i32::from(degree));
        let low = if low > 47 { low - 12 } else { low };
        let fifth = key(&steps, tonic - 24, i32::from(degree) + 4);
        let fifth = if fifth > low + 12 { fifth - 12 } else { fifth };
        let v = 92.0 * level;
        match mood.bass_style {
            Bass::Held => score.note(bass, low, start, bar, v),
            Bass::Walk => {
                for (i, &beat) in plan.meter.beats().iter().enumerate() {
                    let k = if i % 2 == 0 { low } else { fifth };
                    let length = plan.meter.beats().get(i + 1).copied().unwrap_or(plan.meter.eighths()) - beat;
                    score.note(bass, k, start + beat * eighth, length * eighth - eighth / 4, v);
                }
            }
            Bass::Drive => {
                for e in 0..plan.meter.eighths() {
                    let accent = if plan.meter.beats().contains(&e) { 1.0 } else { 0.8 };
                    score.note(bass, low, start + e * eighth, eighth - eighth / 4, v * accent);
                }
            }
            Bass::Bounce => {
                for e in 0..plan.meter.eighths() {
                    let k = if e % 2 == 0 { low } else { low + 12 };
                    score.note(bass, k, start + e * eighth, eighth - eighth / 3, v);
                }
            }
        }
    }

    // The broken chord, up and down in eighths.
    if mood.arp.is_some() {
        for (b, &degree) in chords.iter().enumerate() {
            let tones: Vec<i32> = chord(degree).iter().map(|&s| key(&steps, tonic, s)).chain([key(&steps, tonic, i32::from(degree) + 7)]).collect();
            let order = [0usize, 1, 2, 3, 2, 1, 0, 1];
            for e in 0..plan.meter.eighths() {
                let k = tones[order[e as usize % order.len()]] - 12;
                score.note(arp, k, b as u32 * bar + e * eighth, eighth, 58.0 * level);
            }
        }
    }

    // The tune: two bars, answered, the first two again, and an ending.
    let rhythms = plan.meter.rhythms();
    let mut bar_rhythms: Vec<&[u32]> = (0..BARS).map(|_| *rng.pick(rhythms)).collect();
    bar_rhythms[4] = bar_rhythms[0];
    bar_rhythms[5] = bar_rhythms[1];
    // The last bar holds its note.
    bar_rhythms[7] = match plan.meter {
        Meter::Four => &[4, 4],
        Meter::Three => &[2, 4],
        Meter::SixEight => &[3, 3],
    };
    let mut step: i32 = chord(chords[0])[rng.below(3)];
    let mut sung: Vec<Vec<i32>> = Vec::new();
    for (b, rhythm) in bar_rhythms.iter().enumerate() {
        let degree = chords[b];
        let mut bar_steps = Vec::new();
        let mut at = 0;
        for (n, &length) in rhythm.iter().enumerate() {
            let strong = plan.meter.beats().contains(&at);
            // Bars five and six echo one and two (the same rhythm), where the chord allows.
            let echo = if (4..6).contains(&b) { sung[b - 4].get(n).copied().filter(|&s| !strong || in_chord(s, degree)) } else { None };
            if let Some(s) = echo {
                step = s;
            } else if strong {
                // The nearest note of the chord, toward the middle.
                step = (-3..=3).map(|d| step + d).filter(|&s| in_chord(s, degree)).min_by_key(|&s| ((s - step).abs(), (s - 4).abs())).unwrap_or(step);
            } else {
                let mut m = *rng.pick(&[-2, -1, -1, 1, 1, 2]);
                if (step > 8 && m > 0) || (step < -1 && m < 0) {
                    m = -m;
                }
                step += m;
            }
            step = step.clamp(-3, 11);
            bar_steps.push(step);
            let rest = n > 0 && !strong && rng.chance(10);
            if !rest {
                let accent = if at == 0 { 1.0 } else if strong { 0.92 } else { 0.84 };
                let velocity = (88.0 + rng.below(12) as f32) * accent * level;
                score.note(lead, key(&steps, tonic, step), b as u32 * bar + at * eighth, length * eighth - eighth / 8, velocity);
            }
            at += length;
        }
        sung.push(bar_steps);
    }

    drums(&mut score, mood.drums, plan.meter, bar, level);

    // The loop's end, exactly: nothing rings past it, and the file's length is the loop's.
    score.0.push(Event { tick: end, order: 1, bytes: vec![0xb0, 11, 127] });

    let seconds = f64::from(end) / f64::from(DIVISION) * 60.0 / f64::from(plan.tempo);
    let mut instruments = vec![program_name(plan.lead), program_name(mood.pad), program_name(mood.bass)];
    if let Some(a) = mood.arp {
        instruments.push(program_name(a));
    }
    instruments.dedup();
    let mut seen = Vec::new();
    instruments.retain(|i| if seen.contains(i) { false } else { seen.push(*i); true });
    let list = match instruments.as_slice() {
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    };
    let drums_said = mood.drums.name().map(|d| format!(", with {d}")).unwrap_or_default();
    let speed = match plan.tempo {
        0..=72 => "slow",
        73..=100 => "steady",
        101..=132 => "lively",
        _ => "fast",
    };
    let about = format!(
        "A {speed} {} piece in {} {}, {} beats a minute in {}, for {list}{drums_said}: {BARS} bars, {} seconds, looping.",
        mood.name,
        NOTE_NAMES[(tonic.rem_euclid(12)) as usize],
        plan.mode.name(),
        plan.tempo,
        plan.meter.name(),
        seconds.round()
    );
    Piece { midi: score.file(plan.tempo), about, seconds }
}

/// The drum part, every bar the same but the first's crash and the last's fill.
fn drums(score: &mut Score, style: Drums, meter: Meter, bar: u32, level: f32) {
    let eighth = DIVISION / 2;
    let sixteenth = eighth / 2;
    let hit = |score: &mut Score, k: i32, at: u32, v: f32| score.note(DRUMS, k, at, sixteenth, v * level);
    let beats = meter.beats();
    for b in 0..BARS {
        let start = b * bar;
        let last = b == BARS - 1;
        match style {
            Drums::None => {}
            Drums::Folk => {
                for &beat in beats {
                    hit(score, if beat == 0 { 41 } else { 45 }, start + beat * eighth, if beat == 0 { 100.0 } else { 80.0 });
                }
                for e in 0..meter.eighths() {
                    hit(score, 54, start + e * eighth, if beats.contains(&e) { 70.0 } else { 50.0 });
                }
            }
            Drums::Battle => {
                if b == 0 {
                    hit(score, 49, start, 100.0);
                }
                for e in 0..meter.eighths() {
                    let at = start + e * eighth;
                    if last && e >= meter.eighths() / 2 {
                        let tom = [50, 48, 47, 45, 43, 41][(e - meter.eighths() / 2) as usize % 6];
                        hit(score, tom, at, 105.0);
                        hit(score, tom, at + sixteenth, 90.0);
                        continue;
                    }
                    hit(score, 42, at, 60.0);
                    if e == 0 || e == 3 || (meter == Meter::Four && e == 4) {
                        hit(score, 36, at, 110.0);
                    }
                    if beats.iter().skip(1).step_by(2).any(|&x| x == e) {
                        hit(score, 38, at, 105.0);
                    }
                }
            }
            Drums::March => {
                for e in 0..meter.eighths() {
                    let at = start + e * eighth;
                    let accent = if beats.contains(&e) { 90.0 } else { 62.0 };
                    hit(score, 38, at, accent);
                    if e % 4 == 3 {
                        hit(score, 38, at + sixteenth, 55.0);
                    }
                }
                hit(score, 36, start, 100.0);
                if let Some(&mid) = beats.get(beats.len() / 2) {
                    hit(score, 36, start + mid * eighth, 90.0);
                }
                if b % 4 == 0 {
                    hit(score, 49, start, 80.0);
                }
            }
            Drums::Low => {
                if b % 2 == 0 {
                    hit(score, 41, start, 90.0);
                }
                if last {
                    hit(score, 41, start + bar / 2, 70.0);
                    hit(score, 43, start + bar * 3 / 4, 80.0);
                }
            }
            Drums::Hand => {
                let pattern = [(64, 90.0), (63, 60.0), (62, 70.0), (63, 60.0), (64, 85.0), (60, 60.0), (62, 70.0), (61, 65.0)];
                for e in 0..meter.eighths() {
                    let (k, v) = pattern[e as usize % pattern.len()];
                    hit(score, k, start + e * eighth, v);
                }
            }
            Drums::Dance => {
                for &beat in beats {
                    hit(score, 36, start + beat * eighth, 110.0);
                }
                for (i, &beat) in beats.iter().enumerate() {
                    if i % 2 == 1 {
                        hit(score, 39, start + beat * eighth, 95.0);
                    }
                }
                for s in 0..meter.eighths() * 2 {
                    let at = start + s * sixteenth;
                    if s % 4 == 2 {
                        hit(score, 46, at, 70.0);
                    } else {
                        hit(score, 42, at, if s % 2 == 0 { 65.0 } else { 45.0 });
                    }
                }
            }
            Drums::Brush => {
                hit(score, 36, start, 60.0);
                for e in 0..meter.eighths() {
                    hit(score, 40, start + e * eighth, if beats.contains(&e) { 50.0 } else { 35.0 });
                }
            }
        }
    }
}

/// A SoundFont's sample: a bright tune with every part, for hearing one before choosing it.
pub fn sample() -> Vec<u8> {
    compose("a bright town square", 0).midi
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(prompt: &str) -> Vec<String> {
        words(prompt)
    }

    #[test]
    fn the_words_pick_the_mood_and_the_first_named_wins_a_tie() {
        assert_eq!(mood_of(&w("A battle with the dragon!")).name, "fierce");
        assert_eq!(mood_of(&w("a quiet tavern")).name, "merry");
        assert_eq!(mood_of(&w("the haunted crypt by the sea")).name, "dark");
        assert_eq!(mood_of(&w("sailing the sea past a crypt")).name, "seafaring");
        assert_eq!(mood_of(&w("something")).name, "peaceful");
        assert_eq!(mood_of(&w("")).name, "peaceful");
    }

    #[test]
    fn instruments_tempo_and_meter_follow_the_words() {
        let p = plan(&w("a sad waltz on the music box at 90 bpm"));
        assert_eq!((p.mood.name, p.lead, p.tempo, p.meter), ("sad", 10, 90, Meter::Three));
        assert_eq!(plan(&w("slow dungeon")).tempo, 66 * 3 / 4);
        assert_eq!(plan(&w("a 120bpm forest")).tempo, 120);
        assert_eq!(plan(&w("a 999 bpm forest")).tempo, 220);
        assert_eq!(plan(&w("heroic in a minor key")).mode, Mode::Aeolian);
        assert_eq!(plan(&w("a pan flute in the forest")).lead, 75);
    }

    #[test]
    fn the_same_words_make_the_same_piece_and_a_new_take_another() {
        let a = compose("A tavern by the docks", 0);
        assert_eq!(a, compose("a tavern, by the docks", 0));
        assert_ne!(a.midi, compose("A tavern by the docks", 1).midi);
        assert!(a.about.starts_with("A lively merry piece in "), "{}", a.about);
        assert!(a.about.contains("fiddle, accordion, upright bass and guitar, with a frame drum and tambourine: 8 bars"), "{}", a.about);
    }

    #[test]
    fn every_mood_plays_through_neumetik_as_a_seamless_loop() {
        for mood in &MOODS {
            for take in 0..3 {
                let piece = compose(mood.words[0], take);
                let rate = 8_000;
                let samples = neumetik::render(&piece.midi, rate, 0.0, 120.0).unwrap();
                // The file is exactly the loop: no tail, nothing cut.
                let frames = samples.len() as f64 / 2.0;
                assert!((frames - piece.seconds * f64::from(rate)).abs() <= 2.0, "{} take {take}: {frames} frames for {}s", mood.name, piece.seconds);
                assert!(piece.seconds >= 8.0 && piece.seconds <= 40.0, "{}: {}s", mood.name, piece.seconds);
                let loudest = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
                assert!(loudest > 0.01 && loudest.is_finite(), "{}: {loudest}", mood.name);
            }
        }
    }

    #[test]
    fn a_number_is_written_as_midi_writes_them() {
        let mut out = Vec::new();
        for n in [0, 0x7f, 0x80, 0x3fff, 0x4000] {
            vlq(&mut out, n);
        }
        assert_eq!(out, vec![0x00, 0x7f, 0x81, 0x00, 0xff, 0x7f, 0x81, 0x80, 0x00]);
    }

    #[test]
    fn the_sample_plays() {
        assert!(neumetik::render(&sample(), 8_000, 0.0, 60.0).unwrap().len() > 8_000);
    }
}

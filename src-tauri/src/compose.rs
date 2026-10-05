//! Coupler's composer: a short piece of music made from a few words the
//! player types (gear's Hooks, Assets, Create Asset), written as a MIDI
//! file for Neumetik to play (`vendor/neumetik`). No model and no
//! samples: the words pick a mood (a mode, a tempo, a meter, the
//! instruments, the drums), then a seeded hand writes the drums, a bass
//! that follows them, held chords, a tune in two-bar phrases, and
//! sometimes a broken chord and a second line in counterpoint against the
//! tune. Every decision is said in words (`Piece::rules`). Pure and
//! unit-tested.
//!
//! **The words.** A mood's words (`MOODS`: "tavern", "battle",
//! "dungeon", "sea"...), the first one named winning a tie; an
//! instrument's name puts it on the tune (`INSTRUMENTS`); "slow",
//! "fast", "soft", "loud", "major", "minor", a scale ("dorian",
//! "harmonic minor"), a key ("in D minor", "key of F sharp"), "waltz",
//! "jig", "march", a number of beats a minute ("90 bpm") or bars
//! ("16 bars"), "with an ending", "rising", "fading", "swing", "loose",
//! "bright", "driving", "tense", "no drums" and the like change what the
//! mood would do. Words it doesn't know are only the seed, so the same
//! words always make the same piece, and `take` makes another.
//!
//! **The options** (`Options`, Create Asset's controls) win over the
//! words. They lean on the mood rather than replace it: brightness moves
//! the mode one shade (Phrygian, minor, Dorian; Mixolydian, major,
//! Lydian), drive the tempo by 8% and how the tune is phrased, tension
//! the sevenths and the notes outside the chord. Each part has its own
//! seeded hand, so leaning on one never rewrites another.
//!
//! **The loop.** A loop's last chord leads back to the first, and every
//! note ends by the last bar's end: an event sits exactly on it, so the
//! file's length is the loop's and Neumetik (or a SoundFont) plays it
//! round with no gap (`assets::audio`, `looping`). A piece cadences
//! instead and ends on home. The file carries its time and key
//! signatures, so the Music Editor shows its bars as written.
//!
//! The controls follow those of notebin.fm's rule-based generator
//! (studied 2026-10-06; its code isn't published, so this is Coupler's
//! own): style, key, scale, length, loop or piece, arc, swing,
//! humanize, brightness, drive, tension, fills and stems.

use serde::{Deserialize, Serialize};

/// Ticks a quarter note.
const DIVISION: u32 = 480;
/// The parts' channels; the drums on GM's tenth.
const LEAD: u8 = 0;
const PAD: u8 = 1;
const BASS: u8 = 2;
const ARP: u8 = 3;
const COUNTER: u8 = 4;
const DRUMS: u8 = 9;
/// Below this a bar's energy (`Arc`) thins its decoration.
const THIN: f32 = 0.72;

/// A piece made: the MIDI file, and what it is in words.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Piece {
    #[serde(skip)]
    pub midi: Vec<u8>,
    /// "A dark piece in E Phrygian, 68 beats a minute in 4/4, for oboe,
    /// choir and contrabass, with low drums: 8 bars, 28 seconds, looping."
    pub about: String,
    pub seconds: f64,
    /// How it was written, a sentence for each part and decision.
    pub rules: Vec<String>,
}

/// What the player chose in Create Asset; each one left out is the
/// words' (or the mood's).
#[derive(Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    /// The key's note, C 0 to B 11.
    pub key: Option<u8>,
    pub scale: Option<Mode>,
    pub bars: Option<u32>,
    pub form: Option<Form>,
    pub arc: Option<Arc>,
    /// 50 straight, 67 a triplet.
    pub swing: Option<u8>,
    /// How far off the grid, 0 to 100.
    pub humanize: Option<u8>,
    pub brightness: Option<Lean>,
    pub drive: Option<Lean>,
    pub tension: Option<Lean>,
    pub fills: Option<Fills>,
    pub parts: Option<Stems>,
}

/// A feel control's lean on the mood: the mood as written is neither.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Lean {
    Less,
    More,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Form {
    /// The last bar hands back to the first.
    Loop,
    /// The last bars cadence and the tune ends on home.
    Piece,
}

/// How the energy moves across the bars: how loud each is and how much
/// decoration is in it. Never the tune, the bass, the kick or the snare.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Arc {
    Steady,
    Arch,
    Rise,
    Dissolve,
    Waves,
}

impl Arc {
    /// A bar's energy, about 0.45 to 1.
    fn energy(self, bar: u32, bars: u32) -> f32 {
        let at = (bar as f32 + 0.5) / bars.max(1) as f32;
        let along = bar as f32 / (bars.max(2) - 1) as f32;
        match self {
            Arc::Steady => 1.0,
            Arc::Arch => 0.6 + 0.4 * (std::f32::consts::PI * at).sin(),
            Arc::Rise => 0.5 + 0.5 * along,
            Arc::Dissolve => 1.0 - 0.55 * along,
            Arc::Waves => 0.6 + 0.4 * (2.0 * std::f32::consts::PI * at).sin().abs(),
        }
    }

    fn said(self) -> &'static str {
        match self {
            Arc::Steady => "every bar at the same level",
            Arc::Arch => "rising to the middle and settling back",
            Arc::Rise => "thin and quiet at first, building to the end",
            Arc::Dissolve => "full at first, thinning away",
            Arc::Waves => "two swells",
        }
    }
}

/// Where the drummer fills.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Fills {
    /// Every fourth bar and the last.
    Every,
    /// The last bar only.
    End,
    None,
}

/// Which parts play: the mood's, or one of these.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Stems {
    All,
    /// Everything but the tune and the second line: a bed to talk over.
    Bed,
    /// The bass and drums.
    Rhythm,
    Drums,
    NoDrums,
    /// The tune alone.
    Tune,
    /// The tune and the second line.
    Duet,
}

/// The scales, by notebin.fm's IDs (the frontend's `SCALES`).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[serde(rename = "major")]
    Ionian,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    #[serde(rename = "minor")]
    Aeolian,
    /// Harmonic minor from its fifth: the desert's.
    PhrygianDominant,
    HarmonicMinor,
    HarmonicMajor,
    MelodicMinor,
    LydianDominant,
    HungarianMinor,
    DoubleHarmonic,
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
            Mode::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11],
            Mode::HarmonicMajor => [0, 2, 4, 5, 7, 8, 11],
            Mode::MelodicMinor => [0, 2, 3, 5, 7, 9, 11],
            Mode::LydianDominant => [0, 2, 4, 6, 7, 9, 10],
            Mode::HungarianMinor => [0, 2, 3, 6, 7, 8, 11],
            Mode::DoubleHarmonic => [0, 1, 4, 5, 7, 8, 11],
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
            Mode::HarmonicMinor => "harmonic minor",
            Mode::HarmonicMajor => "harmonic major",
            Mode::MelodicMinor => "melodic minor",
            Mode::LydianDominant => "Lydian dominant",
            Mode::HungarianMinor => "Hungarian minor",
            Mode::DoubleHarmonic => "double harmonic",
        }
    }

    /// Whether its third is minor (the key signature's).
    fn minor(self) -> bool {
        self.steps()[2] == 3
    }

    /// Four chords (scale degrees from 0) the loop takes one of.
    fn progressions(self) -> &'static [&'static [u8]] {
        match self {
            Mode::Ionian => &[&[0, 4, 5, 3], &[0, 3, 4, 3], &[0, 5, 3, 4], &[0, 3, 0, 4], &[5, 3, 0, 4]],
            Mode::Lydian => &[&[0, 1, 0, 1], &[0, 1, 4, 0], &[0, 1, 5, 1]],
            Mode::Mixolydian => &[&[0, 6, 3, 0], &[0, 3, 6, 0], &[0, 6, 0, 3]],
            Mode::Dorian => &[&[0, 3, 0, 3], &[0, 6, 3, 0], &[0, 2, 3, 0]],
            Mode::Aeolian => &[&[0, 5, 2, 6], &[0, 3, 5, 6], &[0, 5, 3, 4], &[0, 6, 5, 4]],
            Mode::Phrygian => &[&[0, 1, 0, 6], &[0, 1, 6, 0], &[0, 6, 1, 0]],
            Mode::PhrygianDominant | Mode::DoubleHarmonic => &[&[0, 1, 0, 1], &[0, 1, 6, 0], &[0, 6, 1, 0]],
            Mode::HarmonicMinor => &[&[0, 3, 4, 0], &[0, 5, 3, 4], &[0, 3, 5, 4]],
            Mode::HarmonicMajor => &[&[0, 3, 4, 0], &[0, 5, 3, 4]],
            Mode::MelodicMinor => &[&[0, 1, 4, 0], &[0, 3, 4, 0]],
            Mode::LydianDominant => &[&[0, 1, 0, 6], &[0, 1, 4, 0]],
            Mode::HungarianMinor => &[&[0, 5, 4, 0], &[0, 3, 4, 0]],
        }
    }

    /// The chord that leads back to the first: the fifth, the seventh, or
    /// the Phrygian second.
    fn turnaround(self) -> u8 {
        match self {
            Mode::Phrygian | Mode::PhrygianDominant | Mode::DoubleHarmonic => 1,
            Mode::Dorian | Mode::Mixolydian | Mode::Aeolian => 6,
            _ => 4,
        }
    }

    /// One shade darker or brighter within its own side: Phrygian, minor,
    /// Dorian; Mixolydian, major, Lydian. The rest have no shade to move.
    fn shade(self, lean: Lean) -> Mode {
        const LADDERS: [[Mode; 3]; 2] = [[Mode::Phrygian, Mode::Aeolian, Mode::Dorian], [Mode::Mixolydian, Mode::Ionian, Mode::Lydian]];
        for ladder in LADDERS {
            if let Some(i) = ladder.iter().position(|m| *m == self) {
                return ladder[if lean == Lean::Less { i.saturating_sub(1) } else { (i + 1).min(2) }];
            }
        }
        self
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

    /// A beat, in eighths.
    fn beat(self) -> u32 {
        if self == Meter::SixEight { 3 } else { 2 }
    }

    fn name(self) -> &'static str {
        match self {
            Meter::Four => "4/4",
            Meter::Three => "3/4",
            Meter::SixEight => "6/8",
        }
    }

    /// The time signature's meta event.
    fn signature(self) -> Vec<u8> {
        match self {
            Meter::Four => vec![0xff, 0x58, 4, 4, 2, 24, 8],
            Meter::Three => vec![0xff, 0x58, 4, 3, 2, 24, 8],
            Meter::SixEight => vec![0xff, 0x58, 4, 6, 3, 36, 8],
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

    /// A last bar that holds its note.
    fn held(self) -> &'static [u32] {
        match self {
            Meter::Four => &[4, 4],
            Meter::Three => &[2, 4],
            Meter::SixEight => &[3, 3],
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
    /// A root on every kick, held to the next.
    Kick,
}

impl Bass {
    fn said(self) -> &'static str {
        match self {
            Bass::Held => "each bar's root, held",
            Bass::Walk => "the root on the beat and the fifth between",
            Bass::Drive => "eighths on the root",
            Bass::Bounce => "the root and its octave by turns, in eighths",
            Bass::Kick => "written after the drums, a root on every kick held to the next, so the two agree",
        }
    }
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
    /// The jazz ride.
    Swing,
    /// Half time: the snare on three.
    Halftime,
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
            Drums::Swing => Some("a jazz kit"),
            Drums::Halftime => Some("heavy half-time drums"),
        }
    }

    fn groove(self) -> &'static str {
        match self {
            Drums::None => "",
            Drums::Folk => "a frame drum on the beats, the low one first, and a tambourine on every eighth",
            Drums::Battle => "the kick on one and before three, the snare on two and four, the hats in eighths and a crash to begin",
            Drums::March => "a snare in eighths with a ruff each half bar, the bass drum on the strong beats",
            Drums::Low => "a low drum every other bar, far off",
            Drums::Hand => "congas and bongos in eighths",
            Drums::Dance => "four on the floor, a clap on two and four, the hats in sixteenths, open on the offbeat",
            Drums::Brush => "brushes: a soft kick on the downbeat and a swish on every eighth",
            Drums::Swing => "the ride's ding, ding-a ding, the hi-hat closed by the foot on two and four, the kick feathered on the beats",
            Drums::Halftime => "half time: the kick on one and pushing before three, the snare on three alone, the hats in eighths",
        }
    }

    /// The kit's program on the drum part (`neumetik::drums::KITS`).
    fn kit(self) -> u8 {
        match self {
            Drums::Battle | Drums::Low | Drums::March => 48,
            Drums::Dance => 25,
            Drums::Brush => 40,
            Drums::Swing => 32,
            Drums::Halftime => 16,
            _ => 0,
        }
    }

    /// The fill this drummer plays, and where.
    fn fill(self) -> (Fill, Fills) {
        match self {
            Drums::Battle => (Fill::Toms, Fills::Every),
            Drums::March => (Fill::SnareRoll, Fills::Every),
            Drums::Dance => (Fill::HatRoll, Fills::Every),
            Drums::Swing => (Fill::RideComp, Fills::Every),
            Drums::Halftime => (Fill::Sparse, Fills::Every),
            Drums::Low => (Fill::Cinematic, Fills::End),
            Drums::Brush => (Fill::BrushSweep, Fills::End),
            Drums::Folk | Drums::Hand | Drums::None => (Fill::HandDrums, Fills::End),
        }
    }

    /// Whether the kit has a crash to answer a fill.
    fn crash(self) -> bool {
        matches!(self, Drums::Battle | Drums::March | Drums::Dance | Drums::Swing | Drums::Halftime)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fill {
    Toms,
    SnareRoll,
    HatRoll,
    Sparse,
    BrushSweep,
    RideComp,
    HandDrums,
    Cinematic,
}

impl Fill {
    fn said(self) -> &'static str {
        match self {
            Fill::Toms => "a run down the toms over the last two beats",
            Fill::SnareRoll => "a snare roll on the last beat, closing into thirty-seconds",
            Fill::HatRoll => "closed hats stuttering on the last beat, the snare landing at its end",
            Fill::Sparse => "a kick more, an open hat and a low tom, filling with space",
            Fill::BrushSweep => "a brush swell instead of a run",
            Fill::RideComp => "the ride breaking its pattern and the snare answering",
            Fill::HandDrums => "the hand drums doubling over the last beat",
            Fill::Cinematic => "a low hit and a timpani swell",
        }
    }

    /// Two beats rather than one.
    fn long(self) -> bool {
        matches!(self, Fill::Toms | Fill::Cinematic)
    }
}

/// What a mood plays. Instruments are GM programs.
#[derive(Clone, Copy)]
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
    /// The second line's instrument, and whether the mood has one.
    counter: u8,
    duet: bool,
    bass_style: Bass,
    drums: Drums,
    sevenths: bool,
    /// Every chord a dominant seventh, as the blues has them, whatever the scale.
    dominant: bool,
    swing: u8,
    humanize: u8,
    bars: u32,
    /// Its own progressions, or (empty) its mode's.
    progressions: &'static [&'static [u8]],
}

const BASE: Mood = Mood {
    name: "peaceful",
    words: &[],
    mode: Mode::Ionian,
    tempo: 88,
    meter: Meter::Four,
    lead: 73,
    pad: 48,
    bass: 32,
    arp: None,
    counter: 42,
    duet: false,
    bass_style: Bass::Held,
    drums: Drums::None,
    sevenths: false,
    dominant: false,
    swing: 50,
    humanize: 0,
    bars: 8,
    progressions: &[],
};

const MOODS: [Mood; 22] = [
    Mood {
        words: &["peaceful", "calm", "town", "village", "home", "pastoral", "meadow", "field", "fields", "farm", "morning", "sunny", "hope"],
        arp: Some(46),
        ..BASE
    },
    Mood {
        name: "merry",
        words: &["tavern", "inn", "jolly", "cheerful", "merry", "dance", "dancing", "drinking", "festival", "feast", "happy", "celebration", "fair", "bard"],
        mode: Mode::Mixolydian,
        tempo: 128,
        meter: Meter::SixEight,
        lead: 110,
        pad: 21,
        arp: Some(24),
        bass_style: Bass::Walk,
        drums: Drums::Folk,
        humanize: 15,
        ..BASE
    },
    Mood {
        name: "dark",
        words: &["dark", "dungeon", "crypt", "tomb", "evil", "haunted", "undead", "necromancer", "shadow", "shadows", "cursed", "sinister", "creepy", "horror", "ghost", "ghosts", "lair", "demon"],
        mode: Mode::Phrygian,
        tempo: 66,
        lead: 68,
        pad: 52,
        bass: 43,
        drums: Drums::Low,
        ..BASE
    },
    Mood {
        name: "fierce",
        words: &["battle", "fight", "fighting", "war", "combat", "siege", "attack", "charge", "danger", "chase", "duel", "dragon", "monster", "army"],
        mode: Mode::Aeolian,
        tempo: 152,
        lead: 61,
        bass: 48,
        arp: Some(44),
        bass_style: Bass::Drive,
        drums: Drums::Battle,
        ..BASE
    },
    Mood {
        name: "heroic",
        words: &["heroic", "epic", "hero", "glory", "triumph", "triumphant", "quest", "adventure", "kingdom", "royal", "castle", "knight", "knights", "victory", "king", "queen"],
        tempo: 108,
        lead: 60,
        bass: 58,
        counter: 57,
        duet: true,
        bass_style: Bass::Walk,
        drums: Drums::March,
        ..BASE
    },
    Mood {
        name: "mysterious",
        words: &["mystery", "mysterious", "magic", "magical", "arcane", "wizard", "enchanted", "spell", "fairy", "fae", "dream", "dreamy", "ethereal", "mystic", "mage", "tower"],
        mode: Mode::Lydian,
        tempo: 80,
        lead: 8,
        pad: 89,
        bass: 35,
        arp: Some(10),
        sevenths: true,
        ..BASE
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
        counter: 70,
        ..BASE
    },
    Mood {
        name: "seafaring",
        words: &["sea", "ocean", "sailing", "sail", "ship", "pirate", "pirates", "harbor", "harbour", "waves", "shanty", "port", "docks", "sailor", "sailors", "island"],
        mode: Mode::Dorian,
        tempo: 104,
        meter: Meter::SixEight,
        lead: 21,
        arp: Some(24),
        bass_style: Bass::Walk,
        drums: Drums::Folk,
        humanize: 15,
        ..BASE
    },
    Mood {
        name: "woodland",
        words: &["forest", "woods", "wood", "elven", "elf", "elves", "nature", "grove", "leaves", "spring", "wild", "glade", "trees", "druid"],
        mode: Mode::Dorian,
        tempo: 92,
        meter: Meter::Three,
        lead: 75,
        pad: 52,
        arp: Some(46),
        ..BASE
    },
    Mood {
        name: "desert",
        words: &["desert", "sand", "sands", "eastern", "bazaar", "oasis", "dunes", "nomad", "nomads", "caravan", "pyramid", "pharaoh"],
        mode: Mode::PhrygianDominant,
        tempo: 100,
        lead: 104,
        arp: Some(15),
        bass_style: Bass::Walk,
        drums: Drums::Hand,
        ..BASE
    },
    Mood {
        name: "wintry",
        words: &["snow", "snowy", "ice", "icy", "winter", "frozen", "frost", "north", "northern", "glacier", "tundra"],
        mode: Mode::Aeolian,
        tempo: 76,
        meter: Meter::Three,
        lead: 9,
        pad: 92,
        arp: Some(10),
        ..BASE
    },
    Mood {
        name: "nighttime",
        words: &["night", "lullaby", "sleep", "sleepy", "moon", "moonlight", "stars", "starry", "evening", "dusk", "twilight", "midnight"],
        tempo: 66,
        meter: Meter::Three,
        lead: 11,
        pad: 89,
        arp: Some(46),
        drums: Drums::Brush,
        sevenths: true,
        ..BASE
    },
    Mood {
        name: "electronic",
        words: &["synth", "synthwave", "eighties", "80s", "retro", "techno", "cyber", "cyberpunk", "neon", "space", "future", "robot", "robots", "electronic", "spaceport", "starship"],
        mode: Mode::Aeolian,
        tempo: 118,
        lead: 81,
        pad: 90,
        bass: 38,
        arp: Some(80),
        counter: 81,
        bass_style: Bass::Bounce,
        drums: Drums::Dance,
        ..BASE
    },
    Mood {
        name: "holy",
        words: &["holy", "temple", "church", "cathedral", "prayer", "divine", "sacred", "angel", "angels", "heaven", "chapel", "monastery", "shrine", "priest"],
        mode: Mode::Mixolydian,
        tempo: 66,
        lead: 19,
        pad: 52,
        bass: 19,
        counter: 70,
        duet: true,
        ..BASE
    },
    Mood {
        name: "dire",
        words: &["boss", "nemesis", "doom", "dire", "titan", "overlord", "warlord", "final", "showdown", "lich", "abyss"],
        mode: Mode::Phrygian,
        tempo: 168,
        lead: 30,
        bass: 38,
        arp: Some(81),
        counter: 48,
        bass_style: Bass::Kick,
        drums: Drums::Halftime,
        ..BASE
    },
    Mood {
        name: "courtly",
        words: &["court", "palace", "ballroom", "noble", "nobles", "nobility", "minuet", "baroque", "courtly", "manor", "duke", "duchess", "banquet"],
        tempo: 112,
        meter: Meter::Three,
        lead: 6,
        bass: 43,
        counter: 41,
        duet: true,
        bass_style: Bass::Walk,
        ..BASE
    },
    Mood {
        name: "chiptune",
        words: &["chiptune", "chip", "8bit", "arcade", "pixel", "pixels", "videogame", "console", "bleep", "bleeps"],
        tempo: 150,
        lead: 80,
        pad: 81,
        bass: 38,
        arp: Some(80),
        counter: 80,
        bass_style: Bass::Bounce,
        drums: Drums::Dance,
        ..BASE
    },
    Mood {
        name: "celtic",
        words: &["celtic", "irish", "scottish", "highland", "highlands", "clan", "reel", "hornpipe", "moor", "moors", "heather", "glen"],
        mode: Mode::Dorian,
        tempo: 116,
        meter: Meter::SixEight,
        lead: 78,
        pad: 109,
        arp: Some(46),
        counter: 21,
        bass_style: Bass::Walk,
        drums: Drums::Folk,
        humanize: 15,
        ..BASE
    },
    Mood {
        name: "bluesy",
        words: &["blues", "bluesy", "saloon", "juke", "bayou", "swamp", "honkytonk", "whiskey", "outlaw", "frontier"],
        mode: Mode::Mixolydian,
        tempo: 104,
        lead: 22,
        pad: 16,
        arp: None,
        bass_style: Bass::Walk,
        drums: Drums::Swing,
        sevenths: true,
        dominant: true,
        swing: 67,
        humanize: 30,
        bars: 12,
        progressions: &[&[0, 3, 0, 0, 3, 3, 0, 0, 4, 3, 0, 4], &[0, 0, 0, 0, 3, 3, 0, 0, 4, 3, 0, 4]],
        ..BASE
    },
    Mood {
        name: "jazzy",
        words: &["jazz", "jazzy", "lounge", "speakeasy", "nightclub", "cocktail", "smoky", "club", "casino", "gambling"],
        tempo: 132,
        lead: 65,
        pad: 0,
        counter: 57,
        duet: true,
        bass_style: Bass::Walk,
        drums: Drums::Swing,
        sevenths: true,
        swing: 64,
        humanize: 30,
        progressions: &[&[1, 4, 0, 0], &[0, 5, 1, 4], &[2, 5, 1, 4]],
        ..BASE
    },
    Mood {
        name: "suspenseful",
        words: &["suspense", "suspenseful", "unease", "uneasy", "eerie", "ominous", "stalking", "lurking", "sneaking", "stealth", "thief", "thieves", "assassin", "spy"],
        mode: Mode::HungarianMinor,
        tempo: 72,
        lead: 44,
        pad: 49,
        bass: 43,
        arp: Some(13),
        drums: Drums::Low,
        ..BASE
    },
    Mood {
        name: "drifting",
        words: &["ambient", "drift", "drifting", "cave", "caves", "cavern", "caverns", "underground", "underwater", "deep", "void", "astral", "floating"],
        mode: Mode::Lydian,
        tempo: 64,
        lead: 98,
        pad: 89,
        bass: 35,
        arp: Some(11),
        sevenths: true,
        humanize: 10,
        ..BASE
    },
];

/// Instruments by name, for the tune: GM programs, and the name said.
const INSTRUMENTS: [(&str, u8, &str); 48] = [
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
    ("whistle", 78, "whistle"),
    ("ocarina", 79, "ocarina"),
    ("sitar", 104, "sitar"),
    ("banjo", 105, "banjo"),
    ("koto", 107, "koto"),
    ("kalimba", 108, "kalimba"),
    ("bagpipes", 109, "bagpipes"),
    ("electricguitar", 30, "electric guitar"),
];

/// GM programs' names, as said in what was made.
fn program_name(program: u8) -> &'static str {
    if let Some((_, _, name)) = INSTRUMENTS.iter().find(|(_, p, _)| *p == program) {
        return name;
    }
    match program {
        16 => "drawbar organ",
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
        98 => "crystal",
        _ => "an instrument",
    }
}

const NOTE_NAMES: [&str; 12] = ["C", "D flat", "D", "E flat", "E", "F", "F sharp", "G", "A flat", "A", "B flat", "B"];
/// Sharps (or flats, below 0) in each major key's signature, C to B.
const SIGNATURES: [i8; 12] = [0, -5, 2, -3, 4, -1, 6, 1, -4, 3, -2, 5];
const ROMAN: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];

/// The prompt's words, lower case, letters and digits only ("music box"
/// is also "musicbox", "F#" is "f sharp").
fn words(prompt: &str) -> Vec<String> {
    let prompt = prompt.replace('#', " sharp ");
    let mut out: Vec<String> = prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let pairs: Vec<String> = out.windows(2).map(|w| format!("{}{}", w[0], w[1])).collect();
    for (i, pair) in pairs.into_iter().enumerate() {
        match pair.as_str() {
            "musicbox" | "panpipes" | "electricguitar" | "honkytonk" | "videogame" => out[i] = pair,
            "panflute" => out[i] = "panpipes".into(),
            "8bit" => out[i] = pair,
            _ => {}
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
    /// A part's own hand, from the piece's seed.
    fn part(seed: u64, part: u64) -> Rng {
        Rng(seed ^ part.wrapping_mul(0xd6e8_feb8_6659_fd93))
    }

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

    /// 0 to 1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
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

/// A scale named by a word, or by it and the next ("harmonic minor").
fn scale_word(word: &str, next: Option<&str>) -> Option<Mode> {
    let pair = next.map(|n| format!("{word} {n}"));
    match pair.as_deref() {
        Some("harmonic minor") => return Some(Mode::HarmonicMinor),
        Some("harmonic major") => return Some(Mode::HarmonicMajor),
        Some("melodic minor") => return Some(Mode::MelodicMinor),
        Some("lydian dominant") => return Some(Mode::LydianDominant),
        Some("phrygian dominant") => return Some(Mode::PhrygianDominant),
        Some("hungarian minor") => return Some(Mode::HungarianMinor),
        Some("double harmonic") => return Some(Mode::DoubleHarmonic),
        _ => {}
    }
    match word {
        "dorian" => Some(Mode::Dorian),
        "phrygian" => Some(Mode::Phrygian),
        "lydian" => Some(Mode::Lydian),
        "mixolydian" => Some(Mode::Mixolydian),
        "ionian" => Some(Mode::Ionian),
        "aeolian" => Some(Mode::Aeolian),
        "flamenco" => Some(Mode::PhrygianDominant),
        "byzantine" => Some(Mode::DoubleHarmonic),
        "hungarian" => Some(Mode::HungarianMinor),
        _ => None,
    }
}

/// A key named "in D minor", "in F sharp", "key of E flat" or "B flat
/// key": its note, C 0 to B 11. A lone letter needs one of those around
/// it, as "a" is mostly an article.
fn key_of(words: &[String]) -> Option<u8> {
    const LETTERS: [(&str, i32); 7] = [("c", 0), ("d", 2), ("e", 4), ("f", 5), ("g", 7), ("a", 9), ("b", 11)];
    for i in 0..words.len() {
        let w = words[i].as_str();
        // "Bb", "Eb": a flat in two letters is a key, not a word.
        let (mut note, mut sure) = match LETTERS.iter().find(|(l, _)| *l == w) {
            Some((_, n)) => (*n, false),
            None => match w.strip_suffix('b').and_then(|l| LETTERS.iter().find(|(x, _)| *x == l)) {
                Some((_, n)) if w.len() == 2 => (n - 1, true),
                _ => continue,
            },
        };
        let mut j = i + 1;
        match words.get(j).map(String::as_str) {
            Some("sharp") => {
                note += 1;
                (j, sure) = (j + 1, true);
            }
            Some("flat") => {
                note -= 1;
                (j, sure) = (j + 1, true);
            }
            _ => {}
        }
        let after = words.get(j).map(String::as_str);
        let named_after = matches!(after, Some("major" | "minor")) || after.is_some_and(|a| scale_word(a, words.get(j + 1).map(String::as_str)).is_some());
        let before_in = i >= 1 && words[i - 1] == "in";
        let key_of = i >= 2 && words[i - 2] == "key" && words[i - 1] == "of";
        if key_of || after == Some("key") || (before_in && (named_after || sure)) {
            return Some(note.rem_euclid(12) as u8);
        }
    }
    None
}

/// Which parts play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Parts {
    tune: bool,
    counter: bool,
    chords: bool,
    arp: bool,
    bass: bool,
    drums: bool,
}

/// The plan the words and options make, before the notes.
struct Plan {
    mood: &'static Mood,
    /// The scale played, and the one the mood's chords are written in.
    mode: Mode,
    written: Mode,
    /// The player named a scale; brightness then leaves it alone.
    scale_chosen: bool,
    brightness: Option<Lean>,
    key: Option<u8>,
    tempo: u32,
    meter: Meter,
    lead: u8,
    /// Loudness: soft 0.75, loud 1.15.
    level: f32,
    bars: u32,
    form: Form,
    arc: Arc,
    swing: u8,
    humanize: u8,
    drive: Option<Lean>,
    tension: Option<Lean>,
    sevenths: bool,
    parts: Parts,
    drums: Drums,
    fill: Fill,
    fills: Fills,
}

fn plan(words: &[String], options: &Options) -> Plan {
    let mood = mood_of(words);
    let has = |w: &str| words.iter().any(|x| x == w);
    let any = |list: &[&str]| list.iter().any(|w| has(w));
    let lean = |more: &[&str], less: &[&str]| {
        if any(more) {
            Some(Lean::More)
        } else if any(less) {
            Some(Lean::Less)
        } else {
            None
        }
    };
    let mut tempo = mood.tempo;
    if any(&["slow", "slowly", "lazy", "relaxed"]) {
        tempo = tempo * 3 / 4;
    }
    if any(&["fast", "quick", "rapid", "frantic", "lively", "upbeat"]) {
        tempo = tempo * 5 / 4;
    }
    // "90 bpm", "at 90 beats", "16 bars", "a 12 bar blues".
    let mut bars = mood.bars;
    for pair in words.windows(2) {
        let n = pair[0].parse::<u32>().ok().or(if pair[0] == "twelve" { Some(12) } else { None });
        match (n, pair[1].as_str()) {
            (Some(n), "bpm" | "beats") => tempo = n,
            (Some(n), "bar" | "bars") => bars = n,
            _ => {}
        }
    }
    for w in words {
        if let Some(n) = w.strip_suffix("bpm").and_then(|n| n.parse::<u32>().ok()) {
            tempo = n;
        }
    }
    let bars = options.bars.unwrap_or(bars);
    let mut written = mood.mode;
    if has("minor") {
        written = Mode::Aeolian;
    } else if has("major") {
        written = Mode::Ionian;
    }
    let named = words.iter().enumerate().find_map(|(i, w)| scale_word(w, words.get(i + 1).map(String::as_str)));
    let scale_chosen = options.scale.is_some() || named.is_some();
    let brightness = options.brightness.or(lean(&["bright", "brighter", "brilliant", "radiant", "shining"], &["darker", "dim", "gloomy", "bleak", "grim"]));
    let mut mode = options.scale.or(named).unwrap_or(written);
    if let (false, Some(b)) = (scale_chosen, brightness) {
        mode = mode.shade(b);
    }
    let mut meter = mood.meter;
    if has("waltz") || has("minuet") {
        meter = Meter::Three;
    } else if has("jig") || has("shanty") {
        meter = Meter::SixEight;
    } else if has("march") {
        meter = Meter::Four;
    }
    let lead = words.iter().find_map(|w| INSTRUMENTS.iter().find(|(name, _, _)| name == w).map(|(_, p, _)| *p)).unwrap_or(mood.lead);
    let level = if any(&["soft", "quiet", "gentle", "hushed", "distant"]) {
        0.75
    } else if any(&["loud", "mighty", "thunderous", "powerful"]) {
        1.15
    } else {
        1.0
    };
    let drive = options.drive.or(lean(&["driving", "busy", "energetic", "pumping", "punchy"], &["sparse", "spacious", "airy", "laidback"]));
    match drive {
        Some(Lean::More) => tempo = tempo * 108 / 100,
        Some(Lean::Less) => tempo = tempo * 92 / 100,
        None => {}
    }
    let tension = options.tension.or(lean(&["tense", "unresolved", "dissonant", "restless"], &["simple", "plain", "innocent", "pure"]));
    let sevenths = match tension {
        Some(lean) => lean == Lean::More,
        None => mood.sevenths,
    };
    let form = options.form.unwrap_or(if any(&["ending", "ends", "finale", "fanfare", "jingle"]) { Form::Piece } else { Form::Loop });
    let arc = options.arc.unwrap_or(if any(&["rising", "building", "builds", "crescendo", "swelling"]) {
        Arc::Rise
    } else if any(&["fading", "fades", "dying", "dissolving", "diminuendo"]) {
        Arc::Dissolve
    } else if any(&["arch", "arching"]) {
        Arc::Arch
    } else if any(&["breathing", "ebbing", "surging"]) {
        Arc::Waves
    } else {
        Arc::Steady
    });
    let swing = options.swing.unwrap_or(if any(&["swing", "swung", "shuffle", "swinging"]) {
        67
    } else if has("straight") {
        50
    } else {
        mood.swing
    });
    let humanize = options.humanize.unwrap_or(if any(&["loose", "human", "humanized", "sloppy", "organic"]) {
        60
    } else if any(&["tight", "exact", "quantized", "robotic"]) {
        0
    } else {
        mood.humanize
    });

    // The parts: the mood's, the words' "no drums", "duet" and the like, or the option's.
    let mut negated: Vec<&str> = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if matches!(w.as_str(), "no" | "without" | "minus") {
            let mut j = i + 1;
            if words.get(j).is_some_and(|n| matches!(n.as_str(), "a" | "an" | "the" | "any")) {
                j += 1;
            }
            if let Some(n) = words.get(j) {
                negated.push(n.as_str());
            }
        }
    }
    let off = |list: &[&str]| negated.iter().any(|n| list.contains(n));
    let on = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()) && !negated.contains(&w.as_str()));
    const DRUM_WORDS: [&str; 4] = ["drums", "drum", "percussion", "beat"];
    let mut parts = Parts {
        tune: !off(&["tune", "melody", "lead", "topline"]) && !any(&["bed", "backing"]),
        counter: (mood.duet || on(&["duet", "countermelody", "counterpoint", "descant"])) && !off(&["countermelody", "counterpoint", "descant"]),
        chords: !off(&["chords", "pad", "harmony"]),
        arp: mood.arp.is_some() && !off(&["arpeggio", "arpeggios", "arp"]),
        bass: !off(&["bass", "bassline"]),
        drums: (mood.drums != Drums::None || on(&DRUM_WORDS)) && !off(&DRUM_WORDS) && !has("drumless"),
    };
    let drums_only = words.windows(2).any(|w| (DRUM_WORDS.contains(&w[0].as_str()) && w[1] == "only") || (matches!(w[0].as_str(), "only" | "just") && DRUM_WORDS.contains(&w[1].as_str())));
    let stems = options.parts.or(if drums_only { Some(Stems::Drums) } else { None });
    if let Some(stems) = stems {
        let all = Parts { tune: true, counter: true, chords: true, arp: true, bass: true, drums: true };
        let none = Parts { tune: false, counter: false, chords: false, arp: false, bass: false, drums: false };
        parts = match stems {
            Stems::All => all,
            Stems::Bed => Parts { tune: false, counter: false, ..all },
            Stems::Rhythm => Parts { bass: true, drums: true, ..none },
            Stems::Drums => Parts { drums: true, ..none },
            Stems::NoDrums => Parts { drums: false, ..parts },
            Stems::Tune => Parts { tune: true, ..none },
            Stems::Duet => Parts { tune: true, counter: true, ..none },
        };
    }
    // A mood with no broken chord or drums gets them only when asked.
    parts.arp &= mood.arp.is_some() || stems == Some(Stems::All);
    let drums = match mood.drums {
        Drums::None if meter == Meter::Four && tempo < 90 => Drums::Brush,
        Drums::None => Drums::Folk,
        d => d,
    };
    let (fill, fills) = drums.fill();
    Plan {
        mood,
        mode,
        written,
        scale_chosen,
        brightness,
        key: options.key.map(|k| k % 12).or_else(|| key_of(words)),
        tempo: tempo.clamp(40, 220),
        meter,
        lead,
        level,
        bars: bars.clamp(2, 32).div_ceil(2) * 2,
        form,
        arc,
        swing: if meter == Meter::SixEight { 50 } else { swing.clamp(50, 75) },
        humanize: humanize.min(100),
        drive,
        tension,
        sevenths,
        parts,
        drums,
        fill,
        fills: options.fills.unwrap_or(fills),
    }
}

/// A MIDI event at a tick. `order` puts note-offs before anything else
/// at the same tick, and note-ons last.
struct Event {
    tick: u32,
    order: u8,
    bytes: Vec<u8>,
}

/// The notes as they're written: swung, nudged off the grid by
/// `humanize`, each bar as loud as its energy, and never past the end.
struct Score {
    events: Vec<Event>,
    bar: u32,
    end: u32,
    energy: Vec<f32>,
    /// Where the offbeat eighth falls, 0.5 straight.
    swing: f64,
    /// 0 to 1.
    humanize: f32,
    hand: Rng,
}

impl Score {
    fn setup(&mut self, bytes: Vec<u8>) {
        self.events.push(Event { tick: 0, order: 1, bytes });
    }

    /// A tick swung: the offbeat eighth of each beat moved later, the beats where they were.
    fn swung(&self, tick: u32) -> u32 {
        if self.swing <= 0.5 {
            return tick;
        }
        let d = f64::from(DIVISION);
        let pos = f64::from(tick % DIVISION);
        let half = d / 2.0;
        let moved = if pos <= half { pos * self.swing * 2.0 } else { self.swing * d + (pos - half) * (1.0 - self.swing) * 2.0 };
        tick - tick % DIVISION + moved.round() as u32
    }

    fn note(&mut self, channel: u8, key: i32, start: u32, length: u32, velocity: f32) {
        let key = key.clamp(0, 127);
        let energy = self.energy.get((start / self.bar.max(1)) as usize).copied().unwrap_or(1.0);
        let mut velocity = velocity * (0.55 + 0.45 * energy);
        let (mut on, mut off) = (self.swung(start), self.swung(start + length.max(1)));
        if self.humanize > 0.0 {
            // The kick holds the centre, the snare sits a hair behind, the hats drift most.
            let (lean, spread) = match (channel, key) {
                (DRUMS, 35 | 36) => (0.0, 0.2),
                (DRUMS, 37..=40) => (0.35, 0.3),
                (DRUMS, _) => (0.0, 1.0),
                (LEAD | COUNTER, _) => (0.0, 0.6),
                (PAD, _) => (0.0, 0.4),
                _ => (0.1, 0.3),
            };
            let most = self.humanize * DIVISION as f32 / 16.0;
            let r = self.hand.unit() - self.hand.unit();
            let shift = (most * (lean + spread * r)).round() as i64;
            on = (i64::from(on) + shift).max(0) as u32;
            off = (i64::from(off) + shift).max(0) as u32;
            velocity *= 1.0 + self.humanize * 0.12 * (self.hand.unit() - self.hand.unit());
        }
        let on = on.min(self.end - 1);
        let off = off.clamp(on + 1, self.end);
        let velocity = velocity.round().clamp(1.0, 127.0) as u8;
        self.events.push(Event { tick: on, order: 2, bytes: vec![0x90 | channel, key as u8, velocity] });
        self.events.push(Event { tick: off, order: 0, bytes: vec![0x80 | channel, key as u8, 0] });
    }

    /// Format 0, one track, every event in time order.
    fn file(mut self, tempo: u32) -> Vec<u8> {
        self.events.sort_by_key(|e| (e.tick, e.order));
        let mut track = Vec::new();
        let micros = 60_000_000 / tempo;
        track.extend_from_slice(&[0, 0xff, 0x51, 3]);
        track.extend_from_slice(&micros.to_be_bytes()[1..]);
        let mut at = 0;
        for e in &self.events {
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

/// The steps of a chord on a degree: its root, third and fifth, and its
/// seventh if asked.
fn chord(degree: u8, sevenths: bool) -> Vec<i32> {
    let d = i32::from(degree);
    if sevenths { vec![d, d + 2, d + 4, d + 6] } else { vec![d, d + 2, d + 4] }
}

/// Whether a step is one of a chord's (its triad), in any octave.
fn in_chord(step: i32, degree: u8) -> bool {
    chord(degree, false).iter().any(|c| (step - c).rem_euclid(7) == 0)
}

/// A chord's Roman numeral in a scale: upper case major, lower minor,
/// ° diminished, + augmented, 7 or maj7 with a seventh.
fn roman(steps: &[u8; 7], degree: u8, sevenths: bool) -> String {
    let d = i32::from(degree);
    let at = |s: i32| key(steps, 0, s) - key(steps, 0, d);
    let (third, fifth, seventh) = (at(d + 2), at(d + 4), at(d + 6));
    let numeral = ROMAN[degree as usize % 7];
    let mut out = if third == 3 { numeral.to_lowercase() } else { numeral.to_string() };
    match (third, fifth) {
        (3, 6) => out.push('°'),
        (4, 8) => out.push('+'),
        _ => {}
    }
    if sevenths {
        out.push_str(if third == 4 && seventh == 11 { "maj7" } else { "7" });
    }
    out
}

/// The tune's two-bar phrases, by letter: an idea is written once and
/// comes back; `Z` is the ending.
fn phrases(bars: u32) -> Vec<char> {
    const ORDER: [char; 8] = ['A', 'B', 'A', 'C', 'D', 'E', 'A', 'C'];
    let n = (bars / 2).max(1) as usize;
    (0..n).map(|i| if i == n - 1 { 'Z' } else { ORDER[i % ORDER.len()] }).collect()
}

/// A phrase's idea: its letter, its two bars' rhythms and, once sung, their steps.
type Idea = (char, [&'static [u32]; 2], Option<[Vec<i32>; 2]>);

/// Makes a piece from the player's words and options; `take` makes
/// another from the same words.
pub fn compose(prompt: &str, take: u32, options: &Options) -> Piece {
    let words = words(prompt);
    let plan = plan(&words, options);
    let seed = seed(&words, take);
    let mood = plan.mood;
    let steps = plan.mode.steps();
    let root = Rng(seed).below(12) as i32;
    // The tune's tonic between B flat below middle C and A above it; the rest below.
    let tonic = match plan.key {
        Some(k) => 58 + (i32::from(k) + 2) % 12,
        None => 60 + (root + 7) % 12 - 2,
    };
    let eighth = DIVISION / 2;
    let sixteenth = eighth / 2;
    let bar = plan.meter.eighths() * eighth;
    let bars = plan.bars;
    let end = bars * bar;
    let level = plan.level;
    let energy: Vec<f32> = (0..bars).map(|b| plan.arc.energy(b, bars)).collect();
    let thin: Vec<bool> = energy.iter().map(|&e| e < THIN).collect();
    let parts = plan.parts;

    // The chords: a progression round to the end, then the loop's
    // turnaround or the piece's cadence.
    let mut harmony = Rng::part(seed, 1);
    let progressions = if mood.progressions.is_empty() { plan.written.progressions() } else { mood.progressions };
    let progression = *harmony.pick(progressions);
    let mut chords: Vec<u8> = (0..bars as usize).map(|b| progression[b % progression.len()]).collect();
    let last = chords.len() - 1;
    let turn = harmony.chance(60);
    match plan.form {
        Form::Loop if chords[last] == 0 || (progression.len() == 4 && turn) => chords[last] = plan.written.turnaround(),
        Form::Loop => {}
        Form::Piece => {
            if last > 0 {
                chords[last - 1] = plan.written.turnaround();
            }
            chords[last] = 0;
        }
    }

    let mut score = Score {
        events: Vec::new(),
        bar,
        end,
        energy: energy.clone(),
        // Past 180 beats a minute the swing eases toward straight.
        swing: 0.5 + (f64::from(plan.swing) / 100.0 - 0.5) * (f64::from(220 - plan.tempo) / 40.0).min(1.0),
        humanize: f32::from(plan.humanize) / 100.0,
        hand: Rng::part(seed, 9),
    };
    score.setup(plan.meter.signature());
    let pc = tonic.rem_euclid(12) as usize;
    let major = if plan.mode.minor() { (pc + 3) % 12 } else { pc };
    score.setup(vec![0xff, 0x59, 2, SIGNATURES[major] as u8, u8::from(plan.mode.minor())]);
    let reverb = if matches!(plan.drums, Drums::Dance | Drums::Halftime) { 40 } else { 80 };
    let voices: [(bool, u8, u8, u8, u8); 5] = [
        (parts.tune, LEAD, plan.lead, 100, 64),
        (parts.chords, PAD, mood.pad, 78, 44),
        (parts.bass, BASS, mood.bass, 100, 64),
        (parts.arp, ARP, mood.arp.unwrap_or(46), 72, 84),
        (parts.counter, COUNTER, mood.counter, 84, 76),
    ];
    for (playing, channel, program, volume, pan) in voices {
        if playing {
            score.setup(vec![0xc0 | channel, program]);
            score.setup(vec![0xb0 | channel, 7, volume]);
            score.setup(vec![0xb0 | channel, 10, pan]);
            score.setup(vec![0xb0 | channel, 91, reverb]);
        }
    }
    if parts.chords {
        score.setup(vec![0xb0 | PAD, 93, 50]);
    }

    // The drums first: the bass follows the kick.
    let hits = drum_hits(&plan, bar, &thin);
    if parts.drums {
        score.setup(vec![0xc0 | DRUMS, plan.drums.kit()]);
        score.setup(vec![0xb0 | DRUMS, 7, 96]);
        score.setup(vec![0xb0 | DRUMS, 91, reverb / 2]);
        for &(k, at, v) in &hits {
            score.note(DRUMS, k, at, sixteenth, v * level);
        }
    }
    let kicks: Vec<u32> = hits.iter().filter(|h| matches!(h.0, 35 | 36)).map(|h| h.1).collect();

    // The bass.
    let bass_style = if mood.bass_style == Bass::Kick && kicks.is_empty() { Bass::Drive } else { mood.bass_style };
    if parts.bass {
        for (b, &degree) in chords.iter().enumerate() {
            let start = b as u32 * bar;
            let low = key(&steps, tonic - 24, i32::from(degree));
            let low = if low > 47 { low - 12 } else { low };
            let fifth = key(&steps, tonic - 24, i32::from(degree) + 4);
            let fifth = if fifth > low + 12 { fifth - 12 } else { fifth };
            let v = 92.0 * level;
            match bass_style {
                Bass::Held => score.note(BASS, low, start, bar, v),
                Bass::Walk => {
                    for (i, &beat) in plan.meter.beats().iter().enumerate() {
                        let k = if i % 2 == 0 { low } else { fifth };
                        let length = plan.meter.beats().get(i + 1).copied().unwrap_or(plan.meter.eighths()) - beat;
                        score.note(BASS, k, start + beat * eighth, length * eighth - eighth / 4, v);
                    }
                }
                Bass::Drive => {
                    for e in 0..plan.meter.eighths() {
                        let accent = if plan.meter.beats().contains(&e) { 1.0 } else { 0.8 };
                        score.note(BASS, low, start + e * eighth, eighth - eighth / 4, v * accent);
                    }
                }
                Bass::Bounce => {
                    for e in 0..plan.meter.eighths() {
                        let k = if e % 2 == 0 { low } else { low + 12 };
                        score.note(BASS, k, start + e * eighth, eighth - eighth / 3, v);
                    }
                }
                Bass::Kick => {
                    let here: Vec<u32> = kicks.iter().copied().filter(|&k| k >= start && k < start + bar).collect();
                    if here.is_empty() {
                        score.note(BASS, low, start, bar, v);
                    }
                    for (i, &at) in here.iter().enumerate() {
                        let until = here.get(i + 1).copied().unwrap_or(start + bar);
                        score.note(BASS, low, at, until - at - eighth / 8, v);
                    }
                }
            }
        }
    }

    // The chords held, each voice moving as little as it can.
    if parts.chords {
        let tones = |degree: u8| -> Vec<i32> {
            if mood.dominant {
                let root = key(&steps, tonic - 12, i32::from(degree));
                [0, 4, 7, 10].iter().take(if plan.sevenths { 4 } else { 3 }).map(|i| root + i).collect()
            } else {
                chord(degree, plan.sevenths).iter().map(|&s| key(&steps, tonic - 12, s)).collect()
            }
        };
        let mut voicing = tones(chords[0]);
        for (b, &degree) in chords.iter().enumerate() {
            let mut next: Vec<i32> = Vec::new();
            for (i, &k) in tones(degree).iter().enumerate() {
                let was = voicing[i];
                let near = (-2..=2).map(|o| k + o * 12).min_by_key(|k| (k - was).abs()).unwrap_or(was);
                next.push(near.clamp(48, 72));
            }
            voicing = next;
            let mut played: Vec<i32> = Vec::new();
            for &k in &voicing {
                if !played.contains(&k) {
                    played.push(k);
                    score.note(PAD, k, b as u32 * bar, bar, 62.0 * level);
                }
            }
        }
    }

    // The broken chord, up and down in eighths; a thin bar keeps the beats.
    if parts.arp {
        for (b, &degree) in chords.iter().enumerate() {
            let tones: Vec<i32> = chord(degree, false).iter().map(|&s| key(&steps, tonic, s)).chain([key(&steps, tonic, i32::from(degree) + 7)]).collect();
            let order = [0usize, 1, 2, 3, 2, 1, 0, 1];
            for e in 0..plan.meter.eighths() {
                if energy[b] < 0.6 || (thin[b] && !plan.meter.beats().contains(&e)) {
                    continue;
                }
                let k = tones[order[e as usize % order.len()]] - 12;
                score.note(ARP, k, b as u32 * bar + e * eighth, eighth, 58.0 * level);
            }
        }
    }

    // The tune: two-bar phrases, each idea written once and fitted again
    // to the chords it comes back over.
    let mut melody = Rng::part(seed, 2);
    let letters = phrases(bars);
    let rest_chance = match plan.drive {
        Some(Lean::Less) => 25,
        Some(Lean::More) => 3,
        None => 10,
    };
    let mut ideas: Vec<Idea> = Vec::new();
    let mut step: i32 = chord(chords[0], false)[melody.below(3)];
    let mut sung: Vec<(u32, i32)> = Vec::new();
    for (p, &letter) in letters.iter().enumerate() {
        let rhythms = plan.meter.rhythms();
        let idea = match ideas.iter().position(|(l, _, _)| *l == letter) {
            Some(i) => i,
            None => {
                let pair = if letter == 'Z' { [*melody.pick(rhythms), plan.meter.held()] } else { [*melody.pick(rhythms), *melody.pick(rhythms)] };
                ideas.push((letter, pair, None));
                ideas.len() - 1
            }
        };
        let (_, pair, before) = ideas[idea].clone();
        let mut written: [Vec<i32>; 2] = [Vec::new(), Vec::new()];
        for half in 0..2 {
            let b = p * 2 + half;
            if b >= bars as usize {
                break;
            }
            let degree = chords[b];
            let rhythm = pair[half];
            let mut at = 0;
            for (n, &length) in rhythm.iter().enumerate() {
                let strong = plan.meter.beats().contains(&at);
                let echo = before.as_ref().and_then(|w| w[half].get(n).copied()).filter(|&s| !strong || in_chord(s, degree));
                let lean_out = melody.chance(25);
                let wander = *melody.pick(&[-2, -1, -1, 1, 1, 2]);
                if let Some(s) = echo {
                    step = s;
                } else if strong {
                    // The nearest note of the chord, toward the middle; with
                    // more tension, now and then the note above it.
                    step = (-3..=3).map(|d| step + d).filter(|&s| in_chord(s, degree)).min_by_key(|&s| ((s - step).abs(), (s - 4).abs())).unwrap_or(step);
                    if plan.tension == Some(Lean::More) && at > 0 && lean_out && !in_chord(step + 1, degree) {
                        step += 1;
                    }
                } else {
                    let mut m = wander;
                    if (step > 8 && m > 0) || (step < -1 && m < 0) {
                        m = -m;
                    }
                    step += m;
                }
                step = step.clamp(-3, 11);
                let last_note = b == bars as usize - 1 && n == rhythm.len() - 1;
                if last_note && plan.form == Form::Piece {
                    step = if step >= 4 { 7 } else { 0 };
                }
                written[half].push(step);
                let rest = n > 0 && !strong && !last_note && melody.chance(rest_chance);
                let lift = melody.below(12) as f32;
                if parts.tune && !rest {
                    let accent = if at == 0 { 1.0 } else if strong { 0.92 } else { 0.84 };
                    let velocity = (88.0 + lift) * accent * level;
                    let held = match plan.drive {
                        Some(Lean::Less) => length * eighth - eighth / 16,
                        Some(Lean::More) => length * eighth * 2 / 3,
                        None => length * eighth - eighth / 8,
                    };
                    let start = b as u32 * bar + at * eighth;
                    score.note(LEAD, key(&steps, tonic, step), start, held, velocity);
                }
                sung.push((b as u32 * bar + at * eighth, key(&steps, tonic, step)));
                at += length;
            }
        }
        if before.is_none() {
            ideas[idea].2 = Some(written);
        }
    }

    // The second line: a note a beat against the tune.
    if parts.counter {
        let line = counter_line(&sung, &chords, &steps, tonic, plan.meter, bar, &thin, plan.form, Rng::part(seed, 3));
        for (start, length, k) in line {
            score.note(COUNTER, k, start, length, 72.0 * level);
        }
    }

    // The loop's end, exactly: nothing rings past it, and the file's length is the loop's.
    score.events.push(Event { tick: end, order: 1, bytes: vec![0xb0, 11, 127] });

    let seconds = f64::from(end) / f64::from(DIVISION) * 60.0 / f64::from(plan.tempo);
    let mut instruments = Vec::new();
    for (playing, program) in [(parts.tune, plan.lead), (parts.chords, mood.pad), (parts.bass, mood.bass), (parts.arp, mood.arp.unwrap_or(46)), (parts.counter, mood.counter)] {
        if playing && !instruments.contains(&program_name(program)) {
            instruments.push(program_name(program));
        }
    }
    let list = match instruments.as_slice() {
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    };
    let drums_said = plan.drums.name().filter(|_| parts.drums).map(|d| if list.is_empty() { format!("for {d}") } else { format!(", with {d}") }).unwrap_or_default();
    let speed = match plan.tempo {
        0..=72 => "slow",
        73..=100 => "steady",
        101..=132 => "lively",
        _ => "fast",
    };
    let key_name = format!("{} {}", NOTE_NAMES[pc], plan.mode.name());
    let about = format!(
        "A {speed} {} piece in {key_name}, {} beats a minute in {}, {}{list}{drums_said}: {bars} bars, {} seconds, {}.",
        mood.name,
        plan.tempo,
        plan.meter.name(),
        if list.is_empty() { "" } else { "for " },
        seconds.round(),
        if plan.form == Form::Loop { "looping" } else { "ending on home" }
    );
    let rules = rules(&plan, &steps, progression, &chords, &letters, bass_style, &key_name);
    Piece { midi: score.file(plan.tempo), about, seconds, rules }
}

/// How the piece was written, in words: a sentence for each part and decision.
fn rules(plan: &Plan, steps: &[u8; 7], progression: &[u8], chords: &[u8], letters: &[char], bass: Bass, key_name: &str) -> Vec<String> {
    let mood = plan.mood;
    let parts = plan.parts;
    let name = |d: u8| if mood.dominant { format!("{}{}", ROMAN[d as usize % 7], if plan.sevenths { "7" } else { "" }) } else { roman(steps, d, plan.sevenths) };
    let shown: Vec<String> = progression.iter().take(chords.len()).map(|&d| name(d)).collect();
    let mut out = Vec::new();
    let mut harmony = if mood.dominant {
        format!("Harmony: {} in {key_name}, every chord a major triad{}, as the blues has them, whatever the scale says.", shown.join("-"), if plan.sevenths { " with a flat seventh" } else { "" })
    } else {
        format!(
            "Harmony: {} in {key_name}, each chord stacked in thirds inside the scale, so whether it's major or minor comes from the scale{}.",
            shown.join("-"),
            if plan.sevenths { ", with sevenths" } else { "" }
        )
    };
    if plan.scale_chosen && plan.mode != plan.written {
        harmony.push_str(&format!(" The mood is written in {}; the scale you chose replaces it.", plan.written.name()));
    } else if let Some(lean) = plan.brightness {
        let word = if lean == Lean::More { "brighter" } else { "darker" };
        if plan.mode == plan.written {
            harmony.push_str(&format!(" Brightness had no shade {word} to move {} to.", plan.written.name()));
        } else {
            harmony.push_str(&format!(" Brightness moved {} one shade {word}, to {}.", plan.written.name(), plan.mode.name()));
        }
    }
    out.push(harmony);
    let last = chords.len() - 1;
    out.push(match plan.form {
        Form::Loop => format!("Form: a loop; its last chord, {}, leads back to the first, and the tune's last phrase is left open.", name(chords[last])),
        Form::Piece => format!("Form: a piece; it cadences {} to {} and the tune ends on home.", name(chords[last.saturating_sub(1)]), name(chords[last])),
    });
    if parts.tune {
        let order: Vec<String> = letters.iter().map(|&l| if l == 'Z' { "the ending".to_string() } else { l.to_string() }).collect();
        let mut tune = format!(
            "Tune, on {}: two-bar phrases, {}, each idea written once and fitted again to the chords it returns over: a note of the chord on a strong beat, steps and passing notes between.",
            program_name(plan.lead),
            order.join(", ")
        );
        if plan.tension == Some(Lean::More) {
            tune.push_str(" With more tension, a strong beat now and then leans on the note above the chord.");
        }
        out.push(tune);
    }
    if parts.counter {
        out.push(format!(
            "Second line, on {}: a note a beat under the tune, always a third, sixth, fifth or octave from it, moving against it where it can and never in parallel fifths or octaves.",
            program_name(mood.counter)
        ));
    }
    if parts.chords {
        out.push(format!("Chords, on {}: each bar's chord held below middle C, each voice moving as little as it can.", program_name(mood.pad)));
    }
    if parts.arp {
        out.push(format!("Broken chord, on {}: up and down the chord in eighths.", program_name(mood.arp.unwrap_or(46))));
    }
    if parts.bass {
        out.push(format!("Bass, on {}: {}.", program_name(mood.bass), bass.said()));
    }
    if parts.drums {
        let fills = match plan.fills {
            Fills::Every => format!(" Fills every fourth bar and on the last: {}.", plan.fill.said()),
            Fills::End => format!(" A fill on the last bar: {}.", plan.fill.said()),
            Fills::None => " No fills.".to_string(),
        };
        let crash = if plan.drums.crash() && plan.fills != Fills::None { " A crash answers each." } else { "" };
        out.push(format!("Drums: {}.{fills}{crash}", plan.drums.groove()));
    }
    if plan.arc != Arc::Steady {
        out.push(format!("Arc: {}; how loud each bar is and how much decoration is in it, never the tune, bass, kick or snare.", plan.arc.said()));
    }
    out.push(if plan.meter == Meter::SixEight {
        "Swing: none; 6/8 already moves in threes.".to_string()
    } else if plan.swing > 50 {
        format!(
            "Swing: {}%, the offbeat eighth that far through its beat{}{}, every part swung alike so they agree where it falls.",
            plan.swing,
            if plan.swing == 67 { " (a triplet)" } else { "" },
            if plan.tempo > 180 { ", easing toward straight at this speed" } else { "" }
        )
    } else {
        "Swing: none, straight eighths.".to_string()
    });
    out.push(if plan.humanize > 0 {
        format!(
            "Timing: {} off the grid ({} in 100), the kick steadiest, the snare a hair behind and the hats loosest.",
            match plan.humanize {
                0..=25 => "a little",
                26..=60 => "loosely",
                _ => "well",
            },
            plan.humanize
        )
    } else {
        "Timing: exactly on the grid.".to_string()
    });
    match plan.drive {
        Some(Lean::Less) => out.push("Drive: less; 8% slower, more rests, the notes held.".to_string()),
        Some(Lean::More) => out.push("Drive: more; 8% faster, fewer rests, the notes cut short.".to_string()),
        None => {}
    }
    out
}

/// A second line against the tune, first-species style: one note a beat
/// below it, consonant with it, a note of the chord on the downbeat,
/// stepping where it can, against the tune's motion, never in parallel
/// fifths or octaves. A thin bar holds its downbeat; a piece ends on home.
#[allow(clippy::too_many_arguments)]
fn counter_line(sung: &[(u32, i32)], chords: &[u8], steps: &[u8; 7], tonic: i32, meter: Meter, bar: u32, thin: &[bool], form: Form, mut hand: Rng) -> Vec<(u32, u32, i32)> {
    let eighth = DIVISION / 2;
    let beats = meter.beats();
    let mut out: Vec<(u32, u32, i32)> = Vec::new();
    let mut before: Option<(i32, i32)> = None;
    for (b, &degree) in chords.iter().enumerate() {
        let start = b as u32 * bar;
        for (i, &beat) in beats.iter().enumerate() {
            if thin[b] && i > 0 {
                continue;
            }
            let at = start + beat * eighth;
            let until = if thin[b] { bar / eighth } else { beats.get(i + 1).copied().unwrap_or(meter.eighths()) };
            let length = (until - beat) * eighth;
            let tune = sung.iter().rev().find(|(t, _)| *t <= at).or(sung.first()).map_or(tonic, |s| s.1);
            let last = b == chords.len() - 1 && (i == beats.len() - 1 || thin[b]);
            let mut best: Option<(i32, i32)> = None;
            for s in -14..=7 {
                let k = key(steps, tonic - 12, s);
                let gap = tune - k;
                if !(3..=19).contains(&gap) {
                    continue;
                }
                let iv = gap.rem_euclid(12);
                if !matches!(iv, 0 | 3 | 4 | 7 | 8 | 9) {
                    continue;
                }
                let perfect = matches!(iv, 0 | 7);
                let mut cost = (gap - 9).abs() / 3;
                if i == 0 && !in_chord(s, degree) {
                    cost += 6;
                }
                if perfect && i != 0 {
                    cost += 4;
                }
                if last && form == Form::Piece && s.rem_euclid(7) != 0 {
                    cost += 50;
                }
                if let Some((was, tune_was)) = before {
                    let leap = (k - was).abs();
                    cost += match leap {
                        0 => 2,
                        1..=2 => 0,
                        3..=4 => 1,
                        5..=7 => 3,
                        _ => 8,
                    };
                    let mine = (k - was).signum();
                    let theirs = (tune - tune_was).signum();
                    if mine != 0 && mine == -theirs {
                        cost -= 2;
                    }
                    if perfect && mine == theirs && mine != 0 {
                        // Parallel fifths or octaves; similar motion into one.
                        cost += if (tune_was - was).rem_euclid(12) == iv { 20 } else { 3 };
                    }
                }
                let cost = cost * 4 + hand.below(3) as i32;
                if best.is_none_or(|(c, _)| cost < c) {
                    best = Some((cost, k));
                }
            }
            if let Some((_, k)) = best {
                out.push((at, length - eighth / 8, k));
                before = Some((k, tune));
            }
        }
    }
    out
}

/// The drum part: the groove each bar, a fill where the drummer fills
/// (and a crash on the bar after), a thin bar's offbeat hats left out.
/// Each hit is its key, tick and velocity.
fn drum_hits(plan: &Plan, bar: u32, thin: &[bool]) -> Vec<(i32, u32, f32)> {
    let style = plan.drums;
    let meter = plan.meter;
    let bars = plan.bars;
    let eighth = DIVISION / 2;
    let sixteenth = eighth / 2;
    let beats = meter.beats();
    let mut out: Vec<(i32, u32, f32)> = Vec::new();
    let fills_at = |b: u32| match plan.fills {
        Fills::Every => (b + 1).is_multiple_of(4) || b == bars - 1,
        Fills::End => b == bars - 1,
        Fills::None => false,
    };
    for b in 0..bars {
        let start = b * bar;
        let filling = fills_at(b);
        let span = if plan.fill.long() && meter != Meter::SixEight { 2 * meter.beat() } else { meter.beat() } * eighth;
        let from = if filling { start + bar - span } else { start + bar };
        let mut hit = |k: i32, at: u32, v: f32| {
            if at < from {
                out.push((k, at, v));
            }
        };
        let thin = thin[b as usize];
        let after_fill = if b == 0 { fills_at(bars - 1) } else { fills_at(b - 1) };
        if style.crash() && (after_fill || (b == 0 && style == Drums::Battle)) {
            hit(49, start, 100.0);
        }
        match style {
            Drums::None => {}
            Drums::Folk => {
                for &beat in beats {
                    hit(if beat == 0 { 41 } else { 45 }, start + beat * eighth, if beat == 0 { 100.0 } else { 80.0 });
                }
                for e in 0..meter.eighths() {
                    if !thin || beats.contains(&e) {
                        hit(54, start + e * eighth, if beats.contains(&e) { 70.0 } else { 50.0 });
                    }
                }
            }
            Drums::Battle => {
                for e in 0..meter.eighths() {
                    let at = start + e * eighth;
                    if !thin || beats.contains(&e) {
                        hit(42, at, 60.0);
                    }
                    if e == 0 || e == 3 || (meter == Meter::Four && e == 4) {
                        hit(36, at, 110.0);
                    }
                    if beats.iter().skip(1).step_by(2).any(|&x| x == e) {
                        hit(38, at, 105.0);
                    }
                }
            }
            Drums::March => {
                for e in 0..meter.eighths() {
                    let at = start + e * eighth;
                    let accent = if beats.contains(&e) { 90.0 } else { 62.0 };
                    hit(38, at, accent);
                    if e % 4 == 3 && !thin {
                        hit(38, at + sixteenth, 55.0);
                    }
                }
                hit(36, start, 100.0);
                if let Some(&mid) = beats.get(beats.len() / 2) {
                    hit(36, start + mid * eighth, 90.0);
                }
            }
            Drums::Low => {
                if b % 2 == 0 {
                    hit(41, start, 90.0);
                }
            }
            Drums::Hand => {
                let pattern = [(64, 90.0), (63, 60.0), (62, 70.0), (63, 60.0), (64, 85.0), (60, 60.0), (62, 70.0), (61, 65.0)];
                for e in 0..meter.eighths() {
                    if !thin || beats.contains(&e) {
                        let (k, v) = pattern[e as usize % pattern.len()];
                        hit(k, start + e * eighth, v);
                    }
                }
            }
            Drums::Dance => {
                for &beat in beats {
                    hit(36, start + beat * eighth, 110.0);
                }
                for (i, &beat) in beats.iter().enumerate() {
                    if i % 2 == 1 {
                        hit(39, start + beat * eighth, 95.0);
                    }
                }
                for s in 0..meter.eighths() * 2 {
                    let at = start + s * sixteenth;
                    if s % 4 == 2 {
                        hit(46, at, 70.0);
                    } else if !thin || s % 4 == 0 {
                        hit(42, at, if s % 2 == 0 { 65.0 } else { 45.0 });
                    }
                }
            }
            Drums::Brush => {
                hit(36, start, 60.0);
                for e in 0..meter.eighths() {
                    hit(40, start + e * eighth, if beats.contains(&e) { 50.0 } else { 35.0 });
                }
            }
            Drums::Swing => {
                for (i, &beat) in beats.iter().enumerate() {
                    let at = start + beat * eighth;
                    hit(51, at, if i % 2 == 1 { 80.0 } else { 70.0 });
                    hit(36, at, 35.0);
                    if i % 2 == 1 {
                        hit(44, at, 60.0);
                        if !thin {
                            hit(51, at + eighth, 55.0);
                        }
                    }
                }
                if b % 2 == 1 && !thin {
                    hit(38, start + (meter.eighths() - 3) * eighth, 45.0);
                }
            }
            Drums::Halftime => {
                let three = meter.eighths() / 2;
                hit(36, start, 115.0);
                hit(36, start + (three - 1) * eighth + sixteenth, 95.0);
                hit(38, start + three * eighth, 110.0);
                for e in 0..meter.eighths() {
                    if !thin || beats.contains(&e) {
                        hit(42, start + e * eighth, if beats.contains(&e) { 70.0 } else { 50.0 });
                    }
                }
            }
        }
        if filling && style != Drums::None {
            fill(&mut out, plan.fill, from, start + bar);
        }
    }
    out
}

/// A fill from `from` to the bar's end.
fn fill(out: &mut Vec<(i32, u32, f32)>, fill: Fill, from: u32, to: u32) {
    let sixteenth = DIVISION / 4;
    let thirty_second = DIVISION / 8;
    let n = (to - from) / sixteenth;
    let rising = |i: u32, n: u32, low: f32, high: f32| low + (high - low) * i as f32 / n.max(2).saturating_sub(1) as f32;
    match fill {
        Fill::Toms => {
            const TOMS: [i32; 6] = [50, 48, 47, 45, 43, 41];
            for i in 0..n {
                out.push((TOMS[(i * 6 / n.max(1)) as usize], from + i * sixteenth, 95.0 + 10.0 * (i % 2) as f32));
            }
        }
        Fill::SnareRoll => {
            let half = n / 2;
            for i in 0..half {
                out.push((38, from + i * sixteenth, rising(i, n * 2, 60.0, 110.0)));
            }
            let rest = (to - from - half * sixteenth) / thirty_second;
            for i in 0..rest {
                out.push((38, from + half * sixteenth + i * thirty_second, rising(half * 2 + i, n * 2, 60.0, 110.0)));
            }
        }
        Fill::HatRoll => {
            let rolls = (to - from - sixteenth) / thirty_second;
            for i in 0..rolls {
                out.push((42, from + i * thirty_second, rising(i, rolls, 50.0, 90.0)));
            }
            out.push((38, to - sixteenth, 110.0));
        }
        Fill::Sparse => {
            out.push((46, from, 80.0));
            out.push((36, from + sixteenth * 2, 100.0));
            out.push((41, to - sixteenth * 2, 90.0));
        }
        Fill::BrushSweep => {
            for i in 0..n {
                out.push((39, from + i * sixteenth, rising(i, n, 30.0, 75.0)));
            }
        }
        Fill::RideComp => {
            out.push((38, from, 70.0));
            out.push((51, from + sixteenth * 2, 85.0));
            out.push((38, from + sixteenth * 3, 60.0));
            out.push((38, to - sixteenth, 90.0));
        }
        Fill::HandDrums => {
            const HANDS: [i32; 4] = [62, 63, 64, 63];
            for i in 0..n {
                out.push((HANDS[i as usize % 4], from + i * sixteenth, 75.0 + 20.0 * (i % 2 == 0) as u8 as f32));
            }
        }
        Fill::Cinematic => {
            out.push((36, from, 110.0));
            for i in 1..n {
                out.push((41, from + i * sixteenth, rising(i, n, 40.0, 105.0)));
            }
        }
    }
}

/// A SoundFont's sample: a bright tune with every part, for hearing one before choosing it.
pub fn sample() -> Vec<u8> {
    compose("a bright town square", 0, &Options::default()).midi
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(prompt: &str) -> Vec<String> {
        words(prompt)
    }

    fn none() -> Options {
        Options::default()
    }

    /// Every note-on in a file: its tick, channel and key.
    fn notes(midi: &[u8]) -> Vec<(u32, u8, u8)> {
        let mut out = Vec::new();
        let mut i = 22;
        let mut tick = 0;
        while i < midi.len() {
            let mut delta = 0u32;
            loop {
                let b = midi[i];
                i += 1;
                delta = (delta << 7) | u32::from(b & 0x7f);
                if b & 0x80 == 0 {
                    break;
                }
            }
            tick += delta;
            let status = midi[i];
            match status {
                0xff => {
                    let len = midi[i + 2] as usize;
                    i += 3 + len;
                }
                s if s & 0xf0 == 0xc0 => i += 2,
                s => {
                    if s & 0xf0 == 0x90 {
                        out.push((tick, s & 0x0f, midi[i + 1]));
                    }
                    i += 3;
                }
            }
        }
        out
    }

    #[test]
    fn the_words_pick_the_mood_and_the_first_named_wins_a_tie() {
        assert_eq!(mood_of(&w("A battle with the dragon!")).name, "fierce");
        assert_eq!(mood_of(&w("a quiet tavern")).name, "merry");
        assert_eq!(mood_of(&w("the haunted crypt by the sea")).name, "dark");
        assert_eq!(mood_of(&w("sailing the sea past a crypt")).name, "seafaring");
        assert_eq!(mood_of(&w("the boss of the abyss")).name, "dire");
        assert_eq!(mood_of(&w("a twelve bar blues")).name, "bluesy");
        assert_eq!(mood_of(&w("something")).name, "peaceful");
        assert_eq!(mood_of(&w("")).name, "peaceful");
    }

    #[test]
    fn instruments_tempo_and_meter_follow_the_words() {
        let p = plan(&w("a sad waltz on the music box at 90 bpm"), &none());
        assert_eq!((p.mood.name, p.lead, p.tempo, p.meter), ("sad", 10, 90, Meter::Three));
        assert_eq!(plan(&w("slow dungeon"), &none()).tempo, 66 * 3 / 4);
        assert_eq!(plan(&w("a 120bpm forest"), &none()).tempo, 120);
        assert_eq!(plan(&w("a 999 bpm forest"), &none()).tempo, 220);
        assert_eq!(plan(&w("heroic in a minor key"), &none()).mode, Mode::Aeolian);
        assert_eq!(plan(&w("a pan flute in the forest"), &none()).lead, 75);
    }

    #[test]
    fn keys_scales_and_lengths_follow_the_words() {
        assert_eq!(key_of(&w("heroic in a minor key")), Some(9));
        assert_eq!(key_of(&w("a dirge in D minor")), Some(2));
        assert_eq!(key_of(&w("a jig in F# dorian")), Some(6));
        assert_eq!(key_of(&w("in the key of E flat")), Some(3));
        assert_eq!(key_of(&w("a march in Bb")), Some(10));
        assert_eq!(key_of(&w("a battle at a castle")), None);
        let p = plan(&w("a forest in harmonic minor, 16 bars, with an ending"), &none());
        assert_eq!((p.mode, p.written, p.bars, p.form), (Mode::HarmonicMinor, Mode::Aeolian, 16, Form::Piece));
        assert_eq!(plan(&w("a 12 bar blues"), &none()).bars, 12);
        assert_eq!(plan(&w("a blues"), &none()).bars, 12);
        assert_eq!(plan(&w("a 7 bar tune"), &none()).bars, 8);
    }

    #[test]
    fn options_win_over_the_words_and_lean_on_the_mood() {
        let options = Options { key: Some(7), scale: Some(Mode::Lydian), bars: Some(4), swing: Some(60), ..none() };
        let p = plan(&w("a 16 bar forest in D minor"), &options);
        assert_eq!((p.key, p.mode, p.bars, p.swing), (Some(7), Mode::Lydian, 4, 60));
        // Brightness moves the mood's mode a shade, and only when no scale was named.
        let bright = Options { brightness: Some(Lean::More), ..none() };
        assert_eq!(plan(&w("a forest"), &bright).mode, Mode::Dorian);
        assert_eq!(plan(&w("a dungeon"), &Options { brightness: Some(Lean::More), ..none() }).mode, Mode::Aeolian);
        assert_eq!(plan(&w("a town"), &Options { brightness: Some(Lean::Less), ..none() }).mode, Mode::Mixolydian);
        assert_eq!(plan(&w("a dorian dungeon"), &Options { brightness: Some(Lean::Less), ..none() }).mode, Mode::Dorian);
        // Drive leans the tempo by 8%; tension the sevenths.
        assert_eq!(plan(&w("a forest"), &Options { drive: Some(Lean::More), ..none() }).tempo, 92 * 108 / 100);
        assert!(plan(&w("a forest"), &Options { tension: Some(Lean::More), ..none() }).sevenths);
        assert!(!plan(&w("a lounge"), &Options { tension: Some(Lean::Less), ..none() }).sevenths);
        // 6/8 doesn't swing.
        assert_eq!(plan(&w("a swung jig"), &none()).swing, 50);
        assert_eq!(plan(&w("a swung waltz"), &none()).swing, 67);
    }

    #[test]
    fn the_parts_follow_the_words_and_the_option() {
        let p = plan(&w("a battle with no drums"), &none()).parts;
        assert!(!p.drums && p.tune && p.bass);
        let p = plan(&w("a tavern bed"), &none()).parts;
        assert!(!p.tune && p.chords);
        let p = plan(&w("a forest duet"), &none()).parts;
        assert!(p.counter);
        let p = plan(&w("just drums for the battle"), &none()).parts;
        assert_eq!(p, Parts { tune: false, counter: false, chords: false, arp: false, bass: false, drums: true });
        let p = plan(&w("a battle"), &Options { parts: Some(Stems::Rhythm), ..none() }).parts;
        assert!(p.bass && p.drums && !p.tune && !p.chords);
        // Leaving a part out leaves the others' notes as they were.
        let all = compose("a battle", 0, &none());
        let bed = compose("a battle", 0, &Options { parts: Some(Stems::NoDrums), ..none() });
        let tune = |m: &[u8]| notes(m).into_iter().filter(|n| n.1 == LEAD).collect::<Vec<_>>();
        assert_eq!(tune(&all.midi), tune(&bed.midi));
        assert!(notes(&bed.midi).iter().all(|n| n.1 != DRUMS));
    }

    #[test]
    fn the_same_words_make_the_same_piece_and_a_new_take_another() {
        let a = compose("A tavern by the docks", 0, &none());
        assert_eq!(a, compose("a tavern, by the docks", 0, &none()));
        assert_ne!(a.midi, compose("A tavern by the docks", 1, &none()).midi);
        assert!(a.about.starts_with("A lively merry piece in "), "{}", a.about);
        assert!(a.about.contains("fiddle, accordion, upright bass and guitar, with a frame drum and tambourine: 8 bars"), "{}", a.about);
        assert!(a.rules.iter().any(|r| r.starts_with("Harmony: ")), "{:?}", a.rules);
    }

    #[test]
    fn chords_are_named_from_the_scale() {
        let major = Mode::Ionian.steps();
        assert_eq!([0, 1, 4, 6].map(|d| roman(&major, d, false)), ["I", "ii", "V", "vii°"].map(String::from));
        assert_eq!(roman(&major, 0, true), "Imaj7");
        assert_eq!(roman(&major, 4, true), "V7");
        assert_eq!(roman(&Mode::HarmonicMinor.steps(), 2, false), "III+");
    }

    #[test]
    fn the_second_line_stays_under_the_tune_and_consonant() {
        for prompt in ["a heroic quest", "a courtly minuet", "a jazz lounge", "a forest duet in 16 bars"] {
            let piece = compose(prompt, 0, &Options { humanize: Some(0), swing: Some(50), ..none() });
            let all = notes(&piece.midi);
            let tune: Vec<_> = all.iter().filter(|n| n.1 == LEAD).collect();
            let counter: Vec<_> = all.iter().filter(|n| n.1 == COUNTER).collect();
            assert!(!counter.is_empty(), "{prompt}");
            for c in counter {
                if let Some(t) = tune.iter().find(|t| t.0 == c.0) {
                    let gap = i32::from(t.2) - i32::from(c.2);
                    assert!((3..=19).contains(&gap) && matches!(gap % 12, 0 | 3 | 4 | 7 | 8 | 9), "{prompt}: tune {} counter {}", t.2, c.2);
                }
            }
        }
    }

    #[test]
    fn a_piece_ends_on_home_and_drums_fill_and_the_bass_follows_the_kick() {
        let piece = compose("a boss", 0, &Options { key: Some(4), form: Some(Form::Piece), humanize: Some(0), ..none() });
        let all = notes(&piece.midi);
        let last = all.iter().filter(|n| n.1 == LEAD).max_by_key(|n| n.0).unwrap();
        assert_eq!(last.2 % 12, 4, "{}", piece.about);
        assert!(piece.about.ends_with("ending on home."), "{}", piece.about);
        // Every kick has a bass note on it.
        for kick in all.iter().filter(|n| n.1 == DRUMS && n.2 == 36) {
            assert!(all.iter().any(|n| n.1 == BASS && n.0 == kick.0), "no bass on the kick at {}", kick.0);
        }
        // A fill on the fourth bar: the low tom of the half-time kit's sparse fill.
        let bar = 4 * DIVISION;
        assert!(all.iter().any(|n| n.1 == DRUMS && n.2 == 41 && n.0 >= 3 * bar && n.0 < 4 * bar));
        assert!(all.iter().any(|n| n.1 == DRUMS && n.2 == 49 && n.0 == 4 * bar), "a crash after the fill");
    }

    #[test]
    fn swing_moves_the_offbeats_and_an_arc_shapes_the_bars() {
        let straight = compose("a jazz lounge", 0, &Options { swing: Some(50), humanize: Some(0), ..none() });
        let swung = compose("a jazz lounge", 0, &Options { swing: Some(67), humanize: Some(0), ..none() });
        let offbeat = |m: &[u8], at: u32| notes(m).iter().any(|n| n.0 == at);
        assert!(offbeat(&straight.midi, DIVISION + DIVISION / 2));
        assert!(offbeat(&swung.midi, DIVISION + (f64::from(DIVISION) * 0.67).round() as u32));
        // Rising: the first bar quieter than the last.
        let rise = compose("a heroic quest", 0, &Options { arc: Some(Arc::Rise), humanize: Some(0), ..none() });
        let loud = |bar: u32| -> u32 {
            let m = &rise.midi;
            let mut sum = 0;
            let mut i = 22;
            let mut tick = 0;
            while i < m.len() {
                let mut d = 0u32;
                loop {
                    let b = m[i];
                    i += 1;
                    d = (d << 7) | u32::from(b & 0x7f);
                    if b & 0x80 == 0 {
                        break;
                    }
                }
                tick += d;
                match m[i] {
                    0xff => i += 3 + m[i + 2] as usize,
                    s if s & 0xf0 == 0xc0 => i += 2,
                    s => {
                        if s & 0xf0 == 0x90 && s & 0x0f == BASS && tick / (4 * DIVISION) == bar {
                            sum += u32::from(m[i + 2]);
                        }
                        i += 3;
                    }
                }
            }
            sum
        };
        assert!(loud(0) < loud(7), "{} {}", loud(0), loud(7));
    }

    #[test]
    fn every_mood_plays_through_neumetik_as_a_seamless_loop() {
        let lengths = [Options::default(), Options { bars: Some(4), humanize: Some(100), swing: Some(75), ..none() }, Options { bars: Some(16), arc: Some(Arc::Waves), parts: Some(Stems::All), ..none() }];
        for mood in &MOODS {
            for (take, options) in lengths.iter().enumerate() {
                let piece = compose(mood.words[0], take as u32, options);
                let rate = 8_000;
                let samples = neumetik::render(&piece.midi, rate, 0.0, 120.0).unwrap();
                // The file is exactly the loop: no tail, nothing cut.
                let frames = samples.len() as f64 / 2.0;
                assert!((frames - piece.seconds * f64::from(rate)).abs() <= 2.0, "{} take {take}: {frames} frames for {}s", mood.name, piece.seconds);
                if take == 0 {
                    assert!(piece.seconds >= 8.0 && piece.seconds <= 40.0, "{}: {}s", mood.name, piece.seconds);
                }
                let loudest = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
                assert!(loudest > 0.01 && loudest.is_finite(), "{}: {loudest}", mood.name);
            }
        }
    }

    #[test]
    fn the_music_editor_reads_its_meter() {
        let song = crate::music::read(&compose("a merry jig", 0, &none()).midi).unwrap();
        assert_eq!((song.beats, song.unit), (6, 8));
    }

    /// Prints what some prompts make, to read while tuning:
    /// `cargo test compose::tests::show -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn show() {
        for prompt in ["a merry tavern jig with a fiddle", "the boss of the abyss", "a twelve bar blues", "a courtly minuet with an ending", "a rising heroic quest in 16 bars", "a sad waltz in D minor, no drums"] {
            let piece = compose(prompt, 0, &none());
            println!("{prompt}\n  {}", piece.about);
            for rule in &piece.rules {
                println!("  - {rule}");
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

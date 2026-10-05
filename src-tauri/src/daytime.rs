//! The time of day: dawn, day, dusk or night, recorded as Coupler's own
//! pair, `coupler.time`, for the hooks' triggers to hang on. Pure and
//! unit-tested; `session.rs` feeds it.
//!
//! CoffeeMUD's GMCP has no time (`Libraries/CMProtocols.java`), and its
//! text never says when day starts. Coupler puts three things together:
//!
//! - **The game's hour, by MSDP**: `WORLD_TIME`, `"12/3/7 HR:2"` (year,
//!   month, day, hour of the world's clock,
//!   `DefaultTimeClock.getShortestTimeDescription`), sent when Coupler
//!   connects and again within a second of each new hour (`telnet.rs`
//!   asks for it). It says the hour, not the part of the day: that comes
//!   from where each part starts, the server's `DAWNHR`, `DAYHR`,
//!   `DUSKHR` and `NIGHTHR`, which no protocol sends. Coupler starts from
//!   the stock clock's (`coffeemud.ini`: 0, 1, 4, 5 of a six-hour day)
//!   and corrects them from the two below.
//! - **The change**, said to every player when the area's clock moves
//!   into a new part of the day (`DefaultTimeClock.handleTimeChange`),
//!   from `resources/lists.ini`: `TOD_CHANGE_OUTSIDE` under the sky,
//!   awake and seeing, else `TOD_CHANGE_INSIDE`. Outside says dawn (one
//!   of three lines), dusk and night (one of three); inside only dawn
//!   ("It is now daytime.") and night. **Nobody is told when day
//!   starts**, and indoors nobody is told of dusk. A change line that
//!   comes within seconds of a new hour shows where that part starts.
//! - **The `time` command**: "It is dawn (Hour: 0/5)" (`timeDescription`;
//!   blind, just "(Hour: 0/5)"). Where it disagrees with the starts by
//!   one part of the day (dawn where day was expected), the start between
//!   them moves to fit. This is how day's start is learned off stock.
//!
//! A line always wins at the moment it's said. The next hour is worked
//! out from the starts as corrected. Without MSDP (a server that turns
//! it off), only what's said counts: day only from `time`.
//!
//! As with the weather (`ambient.rs`), a line counts only at the start
//! of a line or just after a prompt's closing `>`, so a player saying the
//! words changes nothing. The game turns a backquote into an apostrophe
//! ("Dawn's light"); both are read.
//!
//! **Where the sun or the moon is** (`arc`), for the room's picture: how
//! far across the sky, left to right, in thousandths. The sun crosses
//! from dawn's start to night's, the moon from night's to the next dawn.
//! The day's length is the stock six hours until `time` says otherwise
//! ("Hour: 2/5"), and the hour is split in quarters by the real time
//! since it began, once two new hours have shown how long one lasts.
//!
//! **Lines about the time** (`about_time`), which Immersive's narrator
//! says instead of writing: the change lines, and what `time` says
//! (`DefaultTimeClock.timeDescription`): the hour, the date, the season
//! and the moon.
//!
//! Known gaps: `WORLD_TIME` is the world's clock, and the lines and `time`
//! are the area's; they're the same object unless an area keeps a clock of
//! its own (rare), and there a change line holds only until the next hour.

use std::time::Duration;

use crate::clock::Instant;

/// The pair Coupler records for the time of day.
pub const TIME_KEY: &str = "coupler.time";

/// A part of the day, in `TimeClock.TimeOfDay`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Dawn,
    Day,
    Dusk,
    Night,
}

impl Phase {
    pub const ALL: [Phase; 4] = [Phase::Dawn, Phase::Day, Phase::Dusk, Phase::Night];

    /// The value Coupler records it as.
    pub fn name(self) -> &'static str {
        match self {
            Phase::Dawn => "dawn",
            Phase::Day => "day",
            Phase::Dusk => "dusk",
            Phase::Night => "night",
        }
    }
}

/// The stock clock's starts (`DAWNHR`, `DAYHR`, `DUSKHR`, `NIGHTHR`).
const STOCK_STARTS: [u32; 4] = [0, 1, 4, 5];
/// The stock clock's hours in a day (`HOURSINDAY`).
const STOCK_DAY: u32 = 6;
/// The parts an hour is split into for where the sun or the moon is.
const QUARTERS: u32 = 4;
/// A change line and a new hour this close together came together: the
/// line is said as the hour turns, MSDP's hour within a second of it.
const TOGETHER: Duration = Duration::from_secs(5);

/// The hour in MSDP's `WORLD_TIME` (`"12/3/7 HR:2"`).
pub fn world_time_hour(value: &str) -> Option<u32> {
    value.rsplit_once("HR:")?.1.trim().parse().ok()
}

/// The change lines (`lists.ini`), outside and inside, with what they start.
const CHANGES: [(&str, Phase); 9] = [
    ("The sun begins to rise in the west.", Phase::Dawn),
    ("Dawn's light brightens the sky.", Phase::Dawn),
    ("The sun gently rises in the west.", Phase::Dawn),
    ("The sun begins to set in the east.", Phase::Dusk),
    ("The sun has set and darkness again covers the world.", Phase::Night),
    ("The darkness of night envelops the world.", Phase::Night),
    ("Light fades with the setting sun. It is now night.", Phase::Night),
    ("It is now daytime.", Phase::Dawn),
    ("It is nighttime.", Phase::Night),
];

/// What `time` says before the hour (`TimeOfDay.getDesc`).
const TIME_SAYS: [(&str, Phase); 4] = [("It is dawn", Phase::Dawn), ("It is daytime", Phase::Day), ("It is dusk", Phase::Dusk), ("It is nighttime", Phase::Night)];

/// The rest of what `time` says, after its first line: the date
/// ("It is Monday, the 3rd day of Winter, year 12."), the season and the
/// moon (`TimeClock.MoonPhase`, or what hides it).
const SEASONS: [&str; 4] = ["It is fall.", "It is spring.", "It is summer.", "It is winter."];
const MOON: [&str; 13] = [
    "There is a new moon in the sky.",
    "The moon is in the waxing crescent phase.",
    "The moon is in its first quarter.",
    "The moon is in the waxing gibbous phase (almost full).",
    "There is a full moon in the sky.",
    "The moon is in the waning gibbous phase (no longer full).",
    "The moon is in its last quarter.",
    "The moon is in the waning crescent phase.",
    "There is a BLUE MOON! Oh my GOD! Run away!!!!!",
    "The clouds obscure the moon.",
    "The dust obscures the moon.",
    "It is very foggy.",
    "You can't see the moon.",
];

/// Whether a line (its text, without color) is about the time: a change
/// of the part of the day, or any line of what `time` says.
pub fn about_time(line: &str) -> bool {
    if read(line).is_some() {
        return true;
    }
    let text = squeeze(line);
    let text = text.trim();
    SEASONS.contains(&text)
        || MOON.iter().any(|m| text == *m || (m.starts_with("You can't") && text.ends_with(m)))
        || (text.starts_with("It is ") && text.contains(" day of ") && text.ends_with('.'))
}

/// What a line said about the time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Told {
    /// This part of the day just began.
    Began(Phase),
    /// The `time` command: the hour, and the part of the day if it said
    /// (not to a blind character).
    Time { hour: u32, last: u32, phase: Option<Phase> },
}

/// Spaces run together, and backquotes as the game prints them.
fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").replace('`', "'")
}

/// What a line (its text, without color) says about the time of day.
pub fn read(line: &str) -> Option<Told> {
    let line = squeeze(line);
    // Where the game can start a line: its start, or after a prompt.
    let starts = std::iter::once(0).chain(line.match_indices('>').map(|(i, _)| i + 1));
    for start in starts {
        let text = line[start..].trim_start();
        if let Some(told) = read_at(text) {
            return Some(told);
        }
    }
    None
}

fn read_at(text: &str) -> Option<Told> {
    // The whole line: "It is nighttime." is a change, "It is nighttime
    // (Hour: 5/5)" is `time`.
    if let Some((_, phase)) = CHANGES.iter().find(|(said, _)| text == *said) {
        return Some(Told::Began(*phase));
    }
    let (phase, rest) = match TIME_SAYS.iter().find(|(said, _)| text.starts_with(said)) {
        Some((said, phase)) => (Some(*phase), text[said.len()..].trim_start()),
        None => (None, text),
    };
    let numbers = rest.strip_prefix("(Hour: ")?.strip_suffix(')')?;
    let (hour, last) = numbers.split_once('/')?;
    let (hour, last): (u32, u32) = (hour.parse().ok()?, last.parse().ok()?);
    (hour <= last).then_some(Told::Time { hour, last, phase })
}

/// The time of day as known, and the server's clock as learned.
#[derive(Debug)]
pub struct Daytime {
    now: Option<Phase>,
    /// The hour each part of the day starts at, in `Phase::ALL`'s order.
    starts: [u32; 4],
    /// MSDP's hour, and when it came.
    hour: Option<(u32, Instant)>,
    /// A change line waiting for its hour, and when it came.
    began: Option<(Phase, Instant)>,
    /// Hours in the server's day (`time`'s "Hour: 2/5" says 6).
    day: u32,
    /// When the hour now began, if it was seen to turn.
    turned: Option<Instant>,
    /// How long an hour lasts, once two turns have shown it.
    length: Option<Duration>,
}

impl Default for Daytime {
    fn default() -> Self {
        Daytime { now: None, starts: STOCK_STARTS, hour: None, began: None, day: STOCK_DAY, turned: None, length: None }
    }
}

impl Daytime {
    /// The part of the day at an hour, as the server works it out
    /// (`getTODCode`): the latest start at or before it; before dawn is
    /// still night.
    fn at(&self, hour: u32) -> Phase {
        Phase::ALL.into_iter().rev().find(|p| hour >= self.starts[*p as usize]).unwrap_or(Phase::Night)
    }

    /// This part of the day starts at this hour. The others move only as
    /// far as they must to stay in order.
    fn learn(&mut self, phase: Phase, hour: u32) {
        let i = phase as usize;
        self.starts[i] = hour;
        for j in i + 1..4 {
            if self.starts[j] <= self.starts[j - 1] {
                self.starts[j] = self.starts[j - 1] + 1;
            }
        }
        for j in (0..i).rev() {
            if self.starts[j] >= self.starts[j + 1] {
                self.starts[j] = self.starts[j + 1].saturating_sub(1);
            }
        }
    }

    /// What a line said, at the moment it came. Returns the part of the
    /// day to record if it's a change.
    pub fn told(&mut self, told: Told, at: Instant) -> Option<Phase> {
        let phase = match told {
            Told::Began(phase) => {
                match self.hour {
                    Some((hour, since)) if at.saturating_duration_since(since) <= TOGETHER => self.learn(phase, hour),
                    // The new hour may come just after: wait for it.
                    _ => self.began = Some((phase, at)),
                }
                phase
            }
            Told::Time { hour, last, phase: None } => {
                self.time_hour(hour, last, at);
                self.at(hour)
            }
            Told::Time { hour, last, phase: Some(said) } => {
                self.time_hour(hour, last, at);
                // One part off: the start between the two is elsewhere.
                let expected = self.at(hour);
                if said as usize == expected as usize + 1 {
                    self.learn(said, hour);
                } else if expected as usize == said as usize + 1 {
                    self.learn(expected, hour + 1);
                }
                said
            }
        };
        self.become_(phase)
    }

    /// `time`'s hour, right at the moment it's said, and the day's length
    /// from its last hour.
    fn time_hour(&mut self, hour: u32, last: u32, at: Instant) {
        if self.hour.is_none_or(|(was, _)| was != hour) {
            self.hour = Some((hour, at));
        }
        self.day = last + 1;
    }

    /// MSDP's hour, at the moment it came. Returns the part of the day to
    /// record if it's a change.
    pub fn hour(&mut self, hour: u32, at: Instant) -> Option<Phase> {
        if self.hour.is_some_and(|(was, _)| was == hour) {
            return None;
        }
        // Seen to turn (not the first hour of a connection): when this
        // one began, and from the last turn, how long one lasts.
        if let Some((was, _)) = self.hour {
            if hour == was + 1 || hour == 0 {
                if let Some(turned) = self.turned {
                    self.length = Some(at.saturating_duration_since(turned));
                }
                self.turned = Some(at);
            } else {
                self.turned = None;
            }
        }
        self.day = self.day.max(hour + 1);
        self.hour = Some((hour, at));
        if let Some((phase, said)) = self.began.take() {
            if at.saturating_duration_since(said) <= TOGETHER {
                self.learn(phase, hour);
            }
        }
        let phase = self.at(hour);
        self.become_(phase)
    }

    fn become_(&mut self, phase: Phase) -> Option<Phase> {
        (self.now != Some(phase)).then(|| {
            self.now = Some(phase);
            phase
        })
    }

    /// The part of the day now, if it's known.
    pub fn now(&self) -> Option<Phase> {
        self.now
    }

    /// Where the sun (dawn to dusk) or the moon (night) is: how far
    /// across the sky, left to right, in thousandths; None when the hour
    /// isn't known. The middle of the hour until the hour's length is.
    pub fn arc(&self, at: Instant) -> Option<u16> {
        let now = self.now?;
        let (hour, _) = self.hour?;
        let day = self.day.max(self.starts[3] + 1);
        let (from, to) = if now == Phase::Night {
            (self.starts[Phase::Night as usize], self.starts[Phase::Dawn as usize] + day)
        } else {
            (self.starts[Phase::Dawn as usize], self.starts[Phase::Night as usize])
        };
        let span = (to - from).max(1);
        // Hours since this crossing began; one outside it (a line said
        // the part of the day before the hour turned) sits at an edge.
        let into = (hour + day - from % day) % day;
        let quarter = match (self.turned, self.length) {
            (Some(turned), Some(length)) if !length.is_zero() => {
                let q = at.saturating_duration_since(turned).as_secs_f64() / length.as_secs_f64() * QUARTERS as f64;
                (q.floor() as u32).min(QUARTERS - 1) * 2 + 1
            }
            _ => QUARTERS,
        };
        let thousandths = if into >= span {
            // Just set, or not yet risen: whichever end is nearer.
            if into >= span + (day - span).div_ceil(2) {
                0
            } else {
                1000
            }
        } else {
            (into * QUARTERS * 2 + quarter) * 1000 / (span * QUARTERS * 2)
        };
        Some(thousandths.min(1000) as u16)
    }

    /// The connection ended: nothing known. What was learned of the
    /// server's clock is kept.
    pub fn leave(&mut self) {
        *self = Daytime { starts: self.starts, day: self.day, length: self.length, ..Daytime::default() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn every_change_line_is_read() {
        for (said, phase) in CHANGES {
            assert_eq!(read(said), Some(Told::Began(phase)), "{said}");
        }
        assert_eq!(read("Dawn`s light brightens the sky."), Some(Told::Began(Phase::Dawn)));
        assert_eq!(read("Light fades with the setting sun.  It is now night."), Some(Told::Began(Phase::Night)));
    }

    #[test]
    fn the_time_command() {
        assert_eq!(read("It is dawn (Hour: 0/5)"), Some(Told::Time { hour: 0, last: 5, phase: Some(Phase::Dawn) }));
        assert_eq!(read("It is daytime (Hour: 2/5)"), Some(Told::Time { hour: 2, last: 5, phase: Some(Phase::Day) }));
        assert_eq!(read("It is nighttime (Hour: 5/5)"), Some(Told::Time { hour: 5, last: 5, phase: Some(Phase::Night) }));
        // Blind: the hour alone.
        assert_eq!(read("(Hour: 4/5)"), Some(Told::Time { hour: 4, last: 5, phase: None }));
        assert_eq!(read("It is dusk (Hour: 9/7)"), None);
    }

    #[test]
    fn after_a_prompt_but_not_inside_speech() {
        assert_eq!(read("<100Hp 50m 80mv> The sun begins to set in the east."), Some(Told::Began(Phase::Dusk)));
        assert_eq!(read("Bob says 'It is nighttime.'"), None);
        assert_eq!(read("You gossip 'The sun begins to set in the east.'"), None);
        assert_eq!(read("It is nighttime. Bob arrives."), None);
    }

    #[test]
    fn the_world_time() {
        assert_eq!(world_time_hour("12/3/7 HR:2"), Some(2));
        assert_eq!(world_time_hour("1/0/24 HR:15"), Some(15));
        assert_eq!(world_time_hour("12/3/7"), None);
    }

    #[test]
    fn msdps_hour_gives_the_part_of_the_day() {
        let mut d = Daytime::default();
        let t = Instant::now();
        assert_eq!(d.hour(2, t), Some(Phase::Day));
        assert_eq!(d.hour(2, t + secs(1)), None);
        assert_eq!(d.hour(3, t + secs(600)), None);
        assert_eq!(d.hour(4, t + secs(1200)), Some(Phase::Dusk));
        assert_eq!(d.hour(5, t + secs(1800)), Some(Phase::Night));
        assert_eq!(d.hour(0, t + secs(2400)), Some(Phase::Dawn));
        assert_eq!(d.hour(1, t + secs(3000)), Some(Phase::Day));
    }

    #[test]
    fn a_change_and_its_hour_agree_once() {
        let mut d = Daytime::default();
        let t = Instant::now();
        d.hour(3, t);
        // The line first, then the hour: recorded once.
        assert_eq!(d.told(Told::Began(Phase::Dusk), t + secs(600)), Some(Phase::Dusk));
        assert_eq!(d.hour(4, t + secs(601)), None);
        // The hour first, then the line: the same.
        assert_eq!(d.hour(5, t + secs(1200)), Some(Phase::Night));
        assert_eq!(d.told(Told::Began(Phase::Night), t + secs(1201)), None);
    }

    #[test]
    fn a_change_teaches_where_its_part_starts() {
        // A server whose dawn is at hour 2: day at 3 then, not 1.
        let mut d = Daytime::default();
        let t = Instant::now();
        assert_eq!(d.hour(1, t), Some(Phase::Day));
        assert_eq!(d.told(Told::Began(Phase::Dawn), t + secs(600)), Some(Phase::Dawn));
        assert_eq!(d.hour(2, t + secs(601)), None);
        assert_eq!(d.hour(3, t + secs(1200)), Some(Phase::Day));
        // And the other way round: the hour, then the line.
        let mut d = Daytime::default();
        d.hour(5, t);
        assert_eq!(d.hour(6, t + secs(600)), None);
        assert_eq!(d.told(Told::Began(Phase::Dusk), t + secs(601)), Some(Phase::Dusk));
        assert_eq!(d.starts, [0, 1, 6, 7]);
    }

    #[test]
    fn a_line_away_from_the_hour_teaches_nothing() {
        // Another clock's change (an area of its own): said, not learned,
        // and the next hour goes by the world's clock again.
        let mut d = Daytime::default();
        let t = Instant::now();
        d.hour(2, t);
        assert_eq!(d.told(Told::Began(Phase::Night), t + secs(300)), Some(Phase::Night));
        assert_eq!(d.hour(3, t + secs(600)), Some(Phase::Day));
        assert_eq!(d.starts, STOCK_STARTS);
    }

    #[test]
    fn time_moves_the_start_between_two_parts() {
        let mut d = Daytime::default();
        let t = Instant::now();
        // Dawn where day was expected: day starts later.
        assert_eq!(d.told(Told::Time { hour: 1, last: 5, phase: Some(Phase::Dawn) }, t), Some(Phase::Dawn));
        assert_eq!(d.hour(1, t), None);
        assert_eq!(d.hour(2, t + secs(600)), Some(Phase::Day));
        // Dusk where day was expected: dusk starts sooner.
        assert_eq!(d.told(Told::Time { hour: 3, last: 5, phase: Some(Phase::Dusk) }, t + secs(1200)), Some(Phase::Dusk));
        assert_eq!(d.starts, [0, 2, 3, 5]);
        // Blind: the starts say which.
        let mut d = Daytime::default();
        assert_eq!(d.told(Told::Time { hour: 5, last: 5, phase: None }, t), Some(Phase::Night));
    }

    #[test]
    fn without_msdp_only_whats_said_counts() {
        let mut d = Daytime::default();
        let t = Instant::now();
        assert_eq!(d.told(Told::Began(Phase::Dawn), t), Some(Phase::Dawn));
        assert_eq!(d.told(Told::Began(Phase::Dusk), t + secs(2400)), Some(Phase::Dusk));
        assert_eq!(d.starts, STOCK_STARTS);
    }

    #[test]
    fn leaving_forgets_the_time_but_not_the_clock() {
        let mut d = Daytime::default();
        let t = Instant::now();
        d.hour(1, t);
        d.told(Told::Began(Phase::Dawn), t + secs(1));
        d.leave();
        assert_eq!(d.now, None);
        assert_eq!(d.starts, [1, 2, 4, 5]);
        assert_eq!(d.hour(2, t + secs(600)), Some(Phase::Day));
    }

    #[test]
    fn lines_about_the_time() {
        for line in [
            "The sun begins to set in the east.",
            "It is dawn (Hour: 0/5)",
            "It is Tuesday, the 3rd day of Winter, year 12.",
            "It is the 14th day of Spring.",
            "It is fall.",
            "There is a full moon in the sky.",
            "The clouds obscure the moon.",
            "A heavy rain falls. You can't see the moon.",
        ] {
            assert!(about_time(line), "{line}");
        }
        for line in ["Bob says 'It is fall.'", "It is dark in here.", "A full moon is carved on the door.", "The day of reckoning comes."] {
            assert!(!about_time(line), "{line}");
        }
    }

    #[test]
    fn the_sun_crosses_left_to_right_and_the_moon_after_it() {
        let mut d = Daytime::default();
        let t = Instant::now();
        assert_eq!(d.arc(t), None);
        // Stock: the sun from hour 0 to 4, the moon in hour 5.
        let suns: Vec<u16> = (0..5)
            .map(|h| {
                d.hour(h, t + secs(600 * h as u64));
                d.arc(t + secs(600 * h as u64)).unwrap()
            })
            .collect();
        assert!(suns.windows(2).all(|w| w[0] < w[1]), "{suns:?}");
        assert!(suns[0] < 200 && suns[4] > 800, "{suns:?}");
        d.hour(5, t + secs(3000));
        assert_eq!(d.now(), Some(Phase::Night));
        // Two turns seen: the hour is ten minutes, split in quarters.
        let early = d.arc(t + secs(3000)).unwrap();
        let late = d.arc(t + secs(3000 + 550)).unwrap();
        assert!(early < 500 && late > 500, "{early} {late}");
    }

    #[test]
    fn the_middle_of_the_hour_until_its_length_is_known() {
        let mut d = Daytime::default();
        let t = Instant::now();
        d.hour(2, t);
        // Hours 0 to 4 are the sun's: hour 2 is the middle of the sky.
        assert_eq!(d.arc(t + secs(500)), Some(500));
        // `time` says a longer day: the night grows, the sun's crossing doesn't.
        d.told(Told::Time { hour: 6, last: 9, phase: Some(Phase::Night) }, t + secs(600));
        assert_eq!(d.day, 10);
        // The moon from hour 5 to 9: hour 6 is the second of five.
        assert_eq!(d.arc(t + secs(600)), Some(300));
    }

    #[test]
    fn a_change_said_before_its_hour_sits_at_the_edge() {
        let mut d = Daytime::default();
        let t = Instant::now();
        d.hour(4, t);
        // Night said while the hour is still dusk's: the moon just risen.
        d.told(Told::Began(Phase::Night), t + secs(590));
        assert_eq!(d.arc(t + secs(590)), Some(0));
    }
}

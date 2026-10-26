//! The body's needs: hungry, thirsty, fatigued, stinky, read from GMCP
//! `char.status` and said to the player now and then, for as long as
//! they last. Pure and unit-tested; `session.rs` feeds it, ticks it each
//! second, and emits `needs` with the words to say.
//!
//! The game's own tests (Commands/Affect.java `getMOBState`, the lines
//! AFFECTS shows):
//! - hungry: `hunger` below 1; thirsty: `thirst` below 1. The game says
//!   "You are hungry." itself only now and then (MUDFight.java,
//!   `ANNOYANCE_DEFAULT_TICKS`), easily lost in a busy screen.
//! - fatigued: `fatigue` (milliseconds awake) past
//!   `CharState.FATIGUED_MILLIS`, 3000 ticks: 12,000,000 at CoffeeMUD's
//!   4-second tick. The game never says this one unasked.
//! - stinky: `stink_pct` (hygiene over `PlayerStats.HYGIENE_DELIMIT`, as
//!   a percentage) at 100 or more, when AFFECTS starts on "You could
//!   use a bath." The game never says this one unasked either.
//!
//! A need that begins is said at once; while any lasts, all of them are
//! said again every `AGAIN`. Never in a fight (it waits for the fight's
//! end), and never on a port without hunger (the PK port: the values
//! never fall that low).

use serde_json::Value;
use crate::clock::Instant;
use std::time::Duration;

/// How often a need that lasts is said again.
pub const AGAIN: Duration = Duration::from_secs(5 * 60);
/// `CharState.FATIGUED_MILLIS` at a 4-second tick.
const FATIGUED: f64 = 12_000_000.0;

/// What the body needs, in the order they're said.
const NAMES: [&str; 4] = ["hungry", "thirsty", "fatigued", "stinky"];

#[derive(Debug, Default)]
pub struct Needs {
    /// Each of `NAMES`, as the last `char.status` had it.
    now: [bool; 4],
    /// What was said last, and when; None till a need first shows.
    said: Option<([bool; 4], Instant)>,
}

/// A field as a number: CoffeeMUD sends them bare, `stink_pct` maybe with a `%`.
fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.trim().trim_end_matches('%').trim().parse().ok())
}

impl Needs {
    /// Reads a GMCP message (its JSON); only `char.status` counts, and only the fields it has.
    pub fn gmcp(&mut self, package: &str, data: &str) {
        if !package.eq_ignore_ascii_case("char.status") {
            return;
        }
        let Ok(data) = serde_json::from_str::<Value>(data) else {
            return;
        };
        let field = |name: &str| data.get(name).and_then(number);
        if let Some(n) = field("hunger") {
            self.now[0] = n < 1.0;
        }
        if let Some(n) = field("thirst") {
            self.now[1] = n < 1.0;
        }
        if let Some(n) = field("fatigue") {
            self.now[2] = n > FATIGUED;
        }
        if let Some(n) = field("stink_pct") {
            self.now[3] = n >= 100.0;
        }
    }

    /// The words to say now, if any: a need just begun, or `AGAIN` since
    /// they were last said. Not while `fighting`.
    pub fn due(&mut self, now: Instant, fighting: bool) -> Option<String> {
        if !self.now.contains(&true) {
            self.said = None;
            return None;
        }
        if fighting {
            return None;
        }
        let new = match self.said {
            None => true,
            Some((said, at)) => self.now.iter().zip(said).any(|(&n, s)| n && !s) || now.duration_since(at) >= AGAIN,
        };
        if !new {
            // One that's gone doesn't count as said: it's new if it comes back.
            if let Some((said, _)) = &mut self.said {
                for (s, n) in said.iter_mut().zip(self.now) {
                    *s &= n;
                }
            }
            return None;
        }
        self.said = Some((self.now, now));
        Some(self.words())
    }

    /// "You're hungry.", "You're hungry and thirsty.", "You're hungry, thirsty and stinky."
    pub fn words(&self) -> String {
        let names: Vec<&str> = NAMES.iter().zip(self.now).filter(|(_, n)| *n).map(|(name, _)| *name).collect();
        let list = match names.as_slice() {
            [] => return String::new(),
            [one] => one.to_string(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        };
        format!("You're {list}.")
    }

    /// The connection or the character changed.
    pub fn clear(&mut self) {
        *self = Needs::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn status(needs: &mut Needs, data: Value) {
        needs.gmcp("char.status", &data.to_string());
    }

    #[test]
    fn the_games_own_tests() {
        let mut n = Needs::default();
        status(&mut n, json!({"hunger": 0, "thirst": 1, "fatigue": 12_000_001, "stink_pct": 99}));
        assert_eq!(n.words(), "You're hungry and fatigued.");
        status(&mut n, json!({"hunger": 5, "thirst": 0, "fatigue": 100, "stink_pct": "100%"}));
        assert_eq!(n.words(), "You're thirsty and stinky.");
        status(&mut n, json!({"hunger": 0, "fatigue": 20_000_000}));
        assert_eq!(n.words(), "You're hungry, thirsty, fatigued and stinky.");
        // Another package changes nothing.
        n.gmcp("char.vitals", r#"{"hunger": 9}"#);
        assert!(n.now[0]);
    }

    #[test]
    fn said_at_once_then_every_while_it_lasts() {
        let mut n = Needs::default();
        let t = Instant::now();
        status(&mut n, json!({"hunger": 100, "thirst": 100}));
        assert_eq!(n.due(t, false), None);
        status(&mut n, json!({"hunger": 0}));
        assert_eq!(n.due(t, false).as_deref(), Some("You're hungry."));
        assert_eq!(n.due(t + Duration::from_secs(60), false), None);
        // A new need is said at once, with the old.
        status(&mut n, json!({"thirst": 0}));
        assert_eq!(n.due(t + Duration::from_secs(61), false).as_deref(), Some("You're hungry and thirsty."));
        assert_eq!(n.due(t + Duration::from_secs(62), false), None);
        assert_eq!(n.due(t + Duration::from_secs(61) + AGAIN, false).as_deref(), Some("You're hungry and thirsty."));
    }

    #[test]
    fn a_fight_holds_it_and_eating_ends_it() {
        let mut n = Needs::default();
        let t = Instant::now();
        status(&mut n, json!({"hunger": 0}));
        assert_eq!(n.due(t, true), None);
        assert_eq!(n.due(t + Duration::from_secs(1), false).as_deref(), Some("You're hungry."));
        status(&mut n, json!({"hunger": 50}));
        assert_eq!(n.due(t + AGAIN * 2, false), None);
        // Hungry again later: said at once.
        status(&mut n, json!({"hunger": 0}));
        assert_eq!(n.due(t + AGAIN * 2, false).as_deref(), Some("You're hungry."));
    }

    #[test]
    fn one_gone_and_back_is_new() {
        let mut n = Needs::default();
        let t = Instant::now();
        status(&mut n, json!({"hunger": 0, "thirst": 0}));
        assert!(n.due(t, false).is_some());
        status(&mut n, json!({"thirst": 9}));
        assert_eq!(n.due(t + Duration::from_secs(5), false), None);
        status(&mut n, json!({"thirst": 0}));
        assert_eq!(n.due(t + Duration::from_secs(10), false).as_deref(), Some("You're hungry and thirsty."));
    }
}

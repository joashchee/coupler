//! Whether the player is fighting (Combat mode) or not (Explore mode),
//! and who and how: the opponent's name, health and range. Pure and
//! unit-tested; `session.rs` feeds it and emits `combat` when it changes.
//!
//! In a fight means the character has a victim (`MOB.getVictim()`, kept
//! only while both are in the same room and alive, `StdMOB.isInCombat`).
//! Two protocols say so, and **MSDP's exact figures win** where both
//! have one:
//!
//! - **MSDP** (`CMProtocols.processMsdpSend`, reported by `telnet.rs`):
//!   `OPPONENT_HEALTH` and `OPPONENT_HEALTH_MAX` (hit points, not a
//!   percentage) and `OPPONENT_RANGE`. The server checks them every
//!   second (`MSDPPINGINTERVAL`) and sends what changed; without a
//!   victim each is sent empty. So a fight starts and ends here within
//!   a second.
//! - **GMCP** `char.status`: `enemy`, `enemypct`, `enemyrange`, only with
//!   a victim, each game tick (about four seconds) if changed. A fight
//!   starting also pushes it at once (`StdMOB.combatStarted`,
//!   `GMCP_PING_MED`); its end waits for the tick. The name always comes
//!   from here, and the figures only when the server sends no MSDP
//!   (`DISABLE=MSDP`). Not `state` 8 ("fighting"): sitting, sleeping or
//!   AFK replace it while the fight goes on.
//!
//! Each source's verdict (a victim or none) counts when it *changes*, and
//! the latest change wins: GMCP starts a fight first, MSDP ends it first,
//! and neither's lag undoes the other. Nothing else says it: MSDP has no
//! combat flag, and MXP's `<FIGHT>` wraps "You are now in combat." only
//! for players with NOBATTLESPAM.
//!
//! Not asked of MSDP, on purpose:
//! - `OPPONENT_NAME`: it's only sent when a fight starts or ends (its
//!   change check compares the player's own name), never when the
//!   player turns on another foe. `enemy` keeps up.
//! - `OPPONENT_LEVEL`: the server sends the player's own level.
//! - `OPPONENT_STRENGTH`: working it out runs CONSIDER, which tells the
//!   room "<player> considers <foe>." each time it's sent.

use serde::Serialize;
use serde_json::Value;

/// The opponent as Combat mode shows and says it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opponent {
    /// From `char.status`; None for the second or so before it comes.
    pub name: Option<String>,
    /// Hit points left and in all, from MSDP; None without it.
    pub health: Option<i64>,
    pub health_max: Option<i64>,
    /// Health left out of 100: worked out from MSDP's figures, else GMCP's.
    pub percent: Option<i64>,
    /// How far away (0 is toe to toe), MSDP's else GMCP's.
    pub range: Option<i64>,
}

#[derive(Default)]
pub struct Combat {
    /// Combat mode: the latest change of either source's verdict.
    fighting: bool,
    /// The server has sent an opponent variable by MSDP this connection,
    /// so the figures are MSDP's.
    msdp: bool,
    health: Option<i64>,
    health_max: Option<i64>,
    range: Option<i64>,
    /// `char.status`'s enemy fields, None outside a fight.
    enemy: Option<String>,
    enemy_pct: Option<i64>,
    enemy_range: Option<i64>,
}

fn number(value: &str) -> Option<i64> {
    value.trim().parse().ok()
}

impl Combat {
    /// One MSDP variable. Returns whether it was one of Combat's.
    pub fn msdp(&mut self, name: &str, value: &str) -> bool {
        let slot = match name {
            "OPPONENT_HEALTH" => {
                let was = self.health.is_some();
                self.health = number(value);
                if self.health.is_some() != was {
                    self.fighting = !was;
                }
                self.msdp = true;
                return true;
            }
            "OPPONENT_HEALTH_MAX" => &mut self.health_max,
            "OPPONENT_RANGE" => &mut self.range,
            _ => return false,
        };
        *slot = number(value);
        self.msdp = true;
        true
    }

    /// A GMCP message: `char.status` updates the enemy, anything else is
    /// ignored. A status without `enemy` is the fight over.
    pub fn gmcp(&mut self, package: &str, data: &str) {
        if !package.eq_ignore_ascii_case("char.status") {
            return;
        }
        let Ok(Value::Object(status)) = serde_json::from_str::<Value>(data) else { return };
        let was = self.enemy.is_some();
        self.enemy = status.get("enemy").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
        let int = |key: &str| self.enemy.as_ref().and(status.get(key)).and_then(Value::as_i64);
        self.enemy_pct = int("enemypct");
        self.enemy_range = int("enemyrange");
        if self.enemy.is_some() != was {
            self.fighting = !was;
        }
    }

    /// The fight now (Combat mode), or None (Explore mode).
    pub fn opponent(&self) -> Option<Opponent> {
        if !self.fighting {
            return None;
        }
        let (health, health_max) = if self.msdp { (self.health, self.health_max) } else { (None, None) };
        let exact = match (health, health_max) {
            (Some(hp), Some(max)) if max > 0 => Some((hp * 100 + max / 2) / max),
            _ => None,
        };
        Some(Opponent {
            name: self.enemy.clone(),
            health,
            health_max,
            percent: exact.or(self.enemy_pct),
            range: if self.msdp { self.range.or(self.enemy_range) } else { self.enemy_range },
        })
    }

    /// The player left the game or their character: no fight. MSDP stays
    /// on for the connection.
    pub fn leave(&mut self) {
        *self = Combat { msdp: self.msdp, ..Combat::default() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS_FIGHTING: &str = r#"{"level":5,"state":8,"pos":"Standing","enemy":"an orc","enemyrange":0,"enemypct":40}"#;
    const STATUS_CALM: &str = r#"{"level":5,"state":3,"pos":"Standing"}"#;

    #[test]
    fn msdp_figures_win_over_gmcp() {
        let mut c = Combat::default();
        c.gmcp("char.status", STATUS_FIGHTING);
        assert!(c.msdp("OPPONENT_HEALTH", "37"));
        assert!(c.msdp("OPPONENT_HEALTH_MAX", "90"));
        assert!(c.msdp("OPPONENT_RANGE", "1"));
        let o = c.opponent().unwrap();
        assert_eq!(o.name.as_deref(), Some("an orc"));
        assert_eq!((o.health, o.health_max), (Some(37), Some(90)));
        // 41.1%, from the hit points, not GMCP's stale 40.
        assert_eq!(o.percent, Some(41));
        assert_eq!(o.range, Some(1));
    }

    #[test]
    fn msdp_alone_starts_the_fight_before_the_name_comes() {
        let mut c = Combat::default();
        c.msdp("OPPONENT_HEALTH", "50");
        c.msdp("OPPONENT_HEALTH_MAX", "50");
        let o = c.opponent().unwrap();
        assert_eq!(o.name, None);
        assert_eq!(o.percent, Some(100));
    }

    #[test]
    fn empty_msdp_health_ends_the_fight_even_if_gmcp_lags() {
        let mut c = Combat::default();
        c.gmcp("char.status", STATUS_FIGHTING);
        c.msdp("OPPONENT_HEALTH", "3");
        c.msdp("OPPONENT_HEALTH", "");
        assert_eq!(c.opponent(), None);
    }

    #[test]
    fn gmcp_only_when_the_server_sends_no_msdp() {
        let mut c = Combat::default();
        c.gmcp("Char.Status", STATUS_FIGHTING);
        let o = c.opponent().unwrap();
        assert_eq!((o.health, o.health_max), (None, None));
        assert_eq!(o.percent, Some(40));
        assert_eq!(o.range, Some(0));
        c.gmcp("char.status", STATUS_CALM);
        assert_eq!(c.opponent(), None);
    }

    #[test]
    fn other_messages_are_ignored() {
        let mut c = Combat::default();
        assert!(!c.msdp("WORLD_TIME", "12/3/7 HR:2"));
        c.gmcp("char.vitals", r#"{"hp":10,"enemy":"x"}"#);
        c.gmcp("char.status", "not json");
        assert_eq!(c.opponent(), None);
    }

    #[test]
    fn gmcp_starts_the_fight_before_msdp_catches_up() {
        let mut c = Combat::default();
        // MSDP's answer to the REPORT on connecting: no victim.
        c.msdp("OPPONENT_HEALTH", "");
        // combatStarted pushes char.status at once.
        c.gmcp("char.status", STATUS_FIGHTING);
        let o = c.opponent().unwrap();
        assert_eq!((o.health, o.percent), (None, Some(40)));
        // A second later, MSDP's exact figures.
        c.msdp("OPPONENT_HEALTH", "37");
        c.msdp("OPPONENT_HEALTH_MAX", "90");
        assert_eq!(c.opponent().unwrap().percent, Some(41));
        // It ends by MSDP; GMCP's last status still names the enemy.
        c.msdp("OPPONENT_HEALTH", "");
        assert_eq!(c.opponent(), None);
        // GMCP catching up a tick later changes nothing.
        c.gmcp("char.status", STATUS_CALM);
        assert_eq!(c.opponent(), None);
    }

    #[test]
    fn a_status_repeating_the_enemy_doesnt_restart_an_ended_fight() {
        let mut c = Combat::default();
        c.gmcp("char.status", STATUS_FIGHTING);
        c.msdp("OPPONENT_HEALTH", "3");
        c.msdp("OPPONENT_HEALTH", "");
        // Hunger changed, the stale enemy is still in it.
        c.gmcp("char.status", &STATUS_FIGHTING.replace("\"level\":5", "\"level\":5,\"hunger\":1"));
        assert_eq!(c.opponent(), None);
    }

    #[test]
    fn sitting_or_sleeping_in_a_fight_is_still_a_fight() {
        let mut c = Combat::default();
        c.gmcp("char.status", r#"{"state":11,"pos":"Sitting","enemy":"an orc","enemyrange":0,"enemypct":90}"#);
        assert!(c.opponent().is_some());
    }

    #[test]
    fn leaving_ends_the_fight() {
        let mut c = Combat::default();
        c.gmcp("char.status", STATUS_FIGHTING);
        c.msdp("OPPONENT_HEALTH", "3");
        c.leave();
        assert_eq!(c.opponent(), None);
        // The next character's fight starts as usual.
        c.gmcp("char.status", STATUS_FIGHTING);
        assert!(c.opponent().is_some());
    }
}

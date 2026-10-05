//! What Immersive mode turns into sound and the odd spoken line: the
//! character's vitals and the talk addressed to them. Pure and
//! unit-tested; `session.rs` feeds it GMCP and emits `vitals` when they
//! change and `talk` for each line.
//!
//! - **Vitals**: `char.vitals` `{"hp","mana","moves","maxhp","maxmana",
//!   "maxmoves"}`, sent on every ping when changed; `char.maxstats`
//!   repeats the maximums every ~16 s. Fields are merged, so a message
//!   with only some of them leaves the rest as they were.
//! - **Talk**: `comm.channel` `{"chan","msg","player"}`
//!   (docs/coffeemud-gmcp.md, 4.3). `chan` is `tell` for a tell
//!   (`CommonMsgs.java`), `GTELL` for the group (`GTell.java`), `say` for
//!   a say in the room (`Say.java`), otherwise a channel's name
//!   (`CMChannels.java`). The server also sends the sender their own
//!   line; `player` is the sender, with only letters and digits, so it's
//!   compared with `char.base`'s `name` the same way.

use serde::Serialize;
use serde_json::{Map, Value};

/// The character's hit points, mana and movement: now and at most.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vitals {
    pub hp: Option<i64>,
    pub max_hp: Option<i64>,
    pub mana: Option<i64>,
    pub max_mana: Option<i64>,
    pub moves: Option<i64>,
    pub max_moves: Option<i64>,
}

/// What kind of talk a line is: it decides how it's heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TalkKind {
    /// To the player alone.
    Tell,
    /// To the player's group.
    Group,
    /// Said aloud in the room.
    Say,
    /// A public channel (GOSSIP, …).
    Channel,
}

impl TalkKind {
    /// The kind of a `comm.channel` line, from its `chan`.
    pub fn of(channel: &str) -> TalkKind {
        match channel.to_ascii_lowercase().as_str() {
            "tell" => TalkKind::Tell,
            "gtell" => TalkKind::Group,
            "say" => TalkKind::Say,
            _ => TalkKind::Channel,
        }
    }
}

/// One line of talk the player heard.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Talk {
    pub kind: TalkKind,
    /// The channel's name as the game gives it: `tell`, `GTELL`, `say`, `GOSSIP`.
    pub channel: String,
    /// Who said it.
    pub from: String,
    /// The line as the game prints it, colors removed.
    pub text: String,
    /// The player said it themselves.
    pub mine: bool,
}

#[derive(Default)]
pub struct Senses {
    vitals: Vitals,
    /// The character's name, letters and digits only, from `char.base`.
    name: String,
}

/// A number, sent as one or as text.
fn int(object: &Map<String, Value>, key: &str) -> Option<i64> {
    match object.get(key)? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f.round() as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub(crate) fn letters_and_digits(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

impl Senses {
    /// One GMCP message. Returns a line of talk, if it was one; the
    /// vitals are read with `vitals()` afterwards.
    pub fn gmcp(&mut self, package: &str, data: &str) -> Option<Talk> {
        let package = package.to_ascii_lowercase();
        if !matches!(package.as_str(), "char.vitals" | "char.maxstats" | "char.base" | "comm.channel") {
            return None;
        }
        let Ok(Value::Object(object)) = serde_json::from_str::<Value>(data) else { return None };
        match package.as_str() {
            "char.vitals" | "char.maxstats" => {
                let v = &mut self.vitals;
                for (key, slot) in [
                    ("hp", &mut v.hp),
                    ("maxhp", &mut v.max_hp),
                    ("mana", &mut v.mana),
                    ("maxmana", &mut v.max_mana),
                    ("moves", &mut v.moves),
                    ("maxmoves", &mut v.max_moves),
                ] {
                    if let Some(n) = int(&object, key) {
                        *slot = Some(n);
                    }
                }
                None
            }
            "char.base" => {
                if let Some(name) = object.get("name").and_then(Value::as_str) {
                    self.name = letters_and_digits(name);
                }
                None
            }
            _ => {
                let text = object.get("msg").and_then(Value::as_str)?.trim().to_string();
                if text.is_empty() {
                    return None;
                }
                let channel = object.get("chan").and_then(Value::as_str).unwrap_or("").to_string();
                let from = object.get("player").and_then(Value::as_str).unwrap_or("").to_string();
                let kind = TalkKind::of(&channel);
                let mine = !self.name.is_empty() && letters_and_digits(&from) == self.name;
                Some(Talk { kind, channel, from, text, mine })
            }
        }
    }

    /// The character's name, letters and digits, lowercased; empty
    /// before `char.base` has named them.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The vitals now, or None before the game has sent any.
    pub fn vitals(&self) -> Option<Vitals> {
        (self.vitals != Vitals::default()).then(|| self.vitals.clone())
    }

    /// The player left the game or their character: nothing is known.
    pub fn leave(&mut self) {
        *self = Senses::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vitals_merge_and_maxstats_fill_in() {
        let mut s = Senses::default();
        assert_eq!(s.vitals(), None);
        s.gmcp("char.vitals", r#"{"hp":82,"mana":40,"moves":"95","maxhp":100,"maxmana":50,"maxmoves":100}"#);
        let v = s.vitals().unwrap();
        assert_eq!((v.hp, v.max_hp, v.mana, v.moves), (Some(82), Some(100), Some(40), Some(95)));
        s.gmcp("Char.Vitals", r#"{"hp":60}"#);
        assert_eq!(s.vitals().unwrap().hp, Some(60));
        assert_eq!(s.vitals().unwrap().mana, Some(40));
        s.gmcp("char.maxstats", r#"{"maxhp":120,"maxmana":50,"maxmoves":100}"#);
        assert_eq!(s.vitals().unwrap().max_hp, Some(120));
    }

    #[test]
    fn talk_is_classified_and_own_lines_marked() {
        let mut s = Senses::default();
        s.gmcp("char.base", r#"{"name":"Joash","class":"Fighter"}"#);
        let tell = s.gmcp("comm.channel", r#"{"chan":"tell","msg":"Bob tells you 'hi'","player":"Bob"}"#).unwrap();
        assert_eq!((tell.kind, tell.from.as_str(), tell.mine), (TalkKind::Tell, "Bob", false));
        let group = s.gmcp("comm.channel", r#"{"chan":"GTELL","msg":"Ann tells the group 'go'","player":"Ann"}"#).unwrap();
        assert_eq!(group.kind, TalkKind::Group);
        let say = s.gmcp("comm.channel", r#"{"chan":"say","msg":"You say 'hello'","player":"Joash"}"#).unwrap();
        assert_eq!((say.kind, say.mine), (TalkKind::Say, true));
        let gossip = s.gmcp("comm.channel", r#"{"chan":"GOSSIP","msg":"Bob GOSSIPs 'hi'","player":"Bob"}"#).unwrap();
        assert_eq!((gossip.kind, gossip.channel.as_str()), (TalkKind::Channel, "GOSSIP"));
    }

    #[test]
    fn empty_or_broken_talk_is_nothing() {
        let mut s = Senses::default();
        assert_eq!(s.gmcp("comm.channel", r#"{"chan":"tell","msg":"  "}"#), None);
        assert_eq!(s.gmcp("comm.channel", "not json"), None);
        assert_eq!(s.gmcp("room.info", r#"{"num":1}"#), None);
    }

    #[test]
    fn leaving_forgets() {
        let mut s = Senses::default();
        s.gmcp("char.vitals", r#"{"hp":5}"#);
        s.leave();
        assert_eq!(s.vitals(), None);
    }
}

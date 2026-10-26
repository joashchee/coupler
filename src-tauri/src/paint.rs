//! What a room's picture is made from: the scene `session.rs` gathers from `room.info`
//! and Coupler's own pairs, the style the player picked, and the seed
//! that keeps a room's look. Pure and unit-tested.
//!
//! Only the game's own words about the room go in: never talk, other
//! players' names or anything the player typed (rule 1's spirit).
//! Coupler's painter (`painter.rs`) reads the facts; the models, once
//! they come, will read a prompt built here from the same request.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The room as a picture sees it: sent with the `picture` event when any
/// of it changes, and handed back to `picture_paint`.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    /// The world (`Port::world`), so the same room ID in two worlds looks different.
    pub world: String,
    /// The game's room ID (`room.info.id`).
    pub room: String,
    pub name: String,
    pub zone: String,
    /// One of CoffeeMUD's 24 terrains, in lower case (`room.info.terrain`).
    pub terrain: String,
    /// The weather in the room's area (`coupler.weather`), if it's been read.
    pub weather: Option<String>,
    /// The time of day (`coupler.time`: dawn, day, dusk, night), if known.
    pub time: Option<String>,
    /// Where the sun (or at night the moon) is, left to right across the
    /// sky in thousandths (`Daytime::arc`), when the hour is known.
    #[serde(default)]
    pub arc: Option<u16>,
    /// Which of the room's looks: 0 until the player asks for a new one
    /// (`Looks`), the `again` of its seed.
    #[serde(default)]
    pub look: u32,
    /// The player's own ART trigger for this room (on `room.info.id`), an
    /// Assets path: it wins over any painted picture.
    #[serde(default)]
    pub art: Option<String>,
}

/// The looks the player chose: each room's count of "paint it again"
/// (Cmd+Shift+P), so a new look stays the room's. Kept per world at
/// `pictures/<world>.json` in app data. Rooms never repainted aren't kept.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Looks {
    rooms: BTreeMap<String, u32>,
}

impl Looks {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("The rooms' chosen looks couldn't be read. ({e})"))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// The room's look: 0 until asked for another.
    pub fn of(&self, room: &str) -> u32 {
        self.rooms.get(room).copied().unwrap_or(0)
    }

    /// A new look for the room, kept; returns it.
    pub fn again(&mut self, room: &str) -> u32 {
        let next = self.of(room).wrapping_add(1);
        self.rooms.insert(room.to_string(), next);
        next
    }

    /// Every room back to its first look.
    pub fn forget(&mut self) {
        self.rooms.clear();
    }
}

/// The two looks the player picks from (decided 2026-10-03).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Grounded medieval fantasy: the plain landscape.
    #[default]
    Fantasy,
    /// The epic kind: castles on the horizon, a second moon, auroras,
    /// banners and braziers indoors.
    High,
}

/// How many colors the player has the game send them, which the
/// painter's pictures follow (decided 2026-10-25): 16 until the game
/// says more. CoffeeMUD keeps it as the `16 COLOR ONLY` and `256 COLOR
/// ONLY` flags, changed by `ANSI 16`, `ANSI 256` and `ANSI TRUE`
/// (Commands/ANSI.java) or the account menu's `ANSI16`, `ANSI256`
/// (Libraries/CharCreation.java), each answered with the colors now on.
/// The wiki's ANSI page (read 2026-10-25) names only `ANSI 16`; the
/// source has the other two, and Coupler reads all three.
/// With more than 16, a picture's dithers and gradients are drawn in
/// the colors between (`painter::Canvas::art_in`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Colors {
    #[default]
    #[serde(rename = "16")]
    Sixteen,
    #[serde(rename = "256")]
    More,
    #[serde(rename = "true")]
    True,
}

impl Colors {
    /// The game's answer to a change of colors, read from a line:
    /// `ANSI 256 colour enabled.`, `ANSI True colour re-enabled.`, `ANSI
    /// 16 color is already enabled.`, the account menu's `ANSI 16 color
    /// is now ON.`; `ANSI color is now OFF.` and NOANSI's are 16. The
    /// game writes its answer straight after the prompt (`<20Hp 100m>
    /// ANSI 256 colour enabled.`), so it's looked for from each word on.
    pub fn heard(line: &str) -> Option<Colors> {
        let line = line.trim().to_ascii_lowercase();
        let line = line.strip_suffix('.').unwrap_or(&line);
        line.match_indices("ansi ")
            .filter(|&(at, _)| at == 0 || line[..at].ends_with(char::is_whitespace))
            .find_map(|(at, _)| Self::answer(&line[at..]))
    }

    /// One answer, the line from its `ansi` on, lowercased and without its full stop.
    fn answer(line: &str) -> Option<Colors> {
        if line == "ansi color is now off" || line == "ansi colour disabled" {
            return Some(Colors::Sixteen);
        }
        let rest = line.strip_prefix("ansi ")?;
        let (depth, rest) = rest.split_once(' ')?;
        let rest = rest.strip_prefix("colour ").or_else(|| rest.strip_prefix("color "))?;
        if !matches!(rest, "enabled" | "re-enabled" | "is already enabled" | "is now on") {
            return None;
        }
        match depth {
            "16" => Some(Colors::Sixteen),
            "256" => Some(Colors::More),
            "true" => Some(Colors::True),
            _ => None,
        }
    }
}

/// A room's seed: FNV-1a of its world and ID (as `cast.rs` seeds a
/// voice), so a room keeps its look from visit to visit. `again` counts
/// how many times the player asked for a new one.
pub fn seed(world: &str, room: &str, again: u32) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in world.bytes().chain([0]).chain(room.bytes()).chain(again.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_room_keeps_its_seed_and_another_room_differs() {
        assert_eq!(seed("standard", "Midgaard#3001", 0), seed("standard", "Midgaard#3001", 0));
        assert_ne!(seed("standard", "Midgaard#3001", 0), seed("standard", "Midgaard#3002", 0));
        assert_ne!(seed("standard", "Midgaard#3001", 0), seed("classic", "Midgaard#3001", 0));
        assert_ne!(seed("standard", "Midgaard#3001", 0), seed("standard", "Midgaard#3001", 1));
    }

    #[test]
    fn the_games_answers_tell_its_colors() {
        // Commands/ANSI.java and NoANSI.java, color codes taken out.
        assert_eq!(Colors::heard("ANSI 256 colour enabled."), Some(Colors::More));
        assert_eq!(Colors::heard("ANSI True colour re-enabled."), Some(Colors::True));
        assert_eq!(Colors::heard("ANSI 16 color is already enabled."), Some(Colors::Sixteen));
        assert_eq!(Colors::heard("ANSI colour disabled."), Some(Colors::Sixteen));
        // The account menu (Libraries/CharCreation.java).
        assert_eq!(Colors::heard("ANSI True color is now ON."), Some(Colors::True));
        assert_eq!(Colors::heard("ANSI color is now OFF."), Some(Colors::Sixteen));
        assert_eq!(Colors::heard("ANSI is already enabled."), None);
        assert_eq!(Colors::heard("Bob says 'ANSI 256 colour enabled.'"), None);
        // After the prompt it was typed at.
        assert_eq!(Colors::heard("<20Hp 100m 100mv> ANSI 256 colour enabled."), Some(Colors::More));
        assert_eq!(Colors::heard("<20Hp 100m 100mv> ANSI True colour enabled."), Some(Colors::True));
        assert_eq!(Colors::heard("<20Hp> ANSI colour disabled."), Some(Colors::Sixteen));
        assert_eq!(Colors::heard("<20Hp> Bob says 'ANSI 256 colour enabled.'"), None);
        assert_eq!(serde_json::to_string(&Colors::More).unwrap(), "\"256\"");
    }

    #[test]
    fn a_new_look_is_kept_per_room() {
        let mut looks = Looks::default();
        assert_eq!(looks.of("a"), 0);
        assert_eq!(looks.again("a"), 1);
        assert_eq!(looks.again("a"), 2);
        assert_eq!(looks.of("b"), 0);
        let back = Looks::from_json(&looks.to_json()).unwrap();
        assert_eq!(back.of("a"), 2);
        looks.forget();
        assert_eq!(looks, Looks::default());
        assert!(Looks::from_json("not json").is_err());
    }

    #[test]
    fn a_scene_from_before_looks_still_reads() {
        let json = r#"{"world":"standard","room":"r","name":"n","zone":"z","terrain":"woods","weather":null,"time":null}"#;
        let scene: Scene = serde_json::from_str(json).unwrap();
        assert_eq!(scene.look, 0);
        assert_eq!(scene.art, None);
    }

    #[test]
    fn the_style_reads_as_the_frontend_writes_it() {
        assert_eq!(serde_json::from_str::<Style>("\"high\"").unwrap(), Style::High);
        assert_eq!(serde_json::to_string(&Style::Fantasy).unwrap(), "\"fantasy\"");
    }
}

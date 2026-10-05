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

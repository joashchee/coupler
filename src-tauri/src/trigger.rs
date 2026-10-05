//! What a GMCP pair sets off (`Trigger`), what went off (`Fired`), and
//! how a message becomes pairs (`pairs`). Pure, so the web build shares
//! it: there the triggers come from the mirror (`src-web/src/hooks.rs`)
//! rather than the hooks database (`hooks.rs`), which re-exports these.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What a pair sets off when the game sends it. Each asset is its path
/// inside the Assets folder (`wav/door.wav`); none of them is set in an
/// empty trigger, which isn't kept.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    /// A sound effect, played once.
    pub sfx: Option<String>,
    /// How loud it plays, 0 to 100 percent of the Mixer's SFX volume.
    #[serde(default = "full")]
    pub sfx_volume: u8,
    /// Music, which replaces whatever music is playing.
    pub bgm: Option<String>,
    /// Whether the music plays on in a loop, rather than once.
    #[serde(default)]
    pub bgm_loop: bool,
    /// How loud it plays, 0 to 100 percent of the Mixer's BGM volume.
    #[serde(default = "full")]
    pub bgm_volume: u8,
    /// A picture, shown with its top left corner at `art_x`, `art_y`: a
    /// column and row of the screen, counted from 1 like the status
    /// bar's readout.
    pub art: Option<String>,
    #[serde(default = "first")]
    pub art_x: u16,
    #[serde(default = "first")]
    pub art_y: u16,
    /// Seconds the picture stays before it fades away; 0 keeps it until
    /// it's clicked away or another picture takes its place.
    #[serde(default)]
    pub art_fade: u16,
    /// Background noise, looped while the player is in a room it's set
    /// for: on `room.info.id` (that room), `room.info.terrain` or
    /// `coupler.room.type` (`ambient.rs`). A sound effect's types.
    #[serde(default)]
    pub bgn: Option<String>,
    #[serde(default = "full")]
    pub bgn_volume: u8,
    /// Background weather, looped while that weather is known and the
    /// player is under the sky: on `coupler.weather`.
    #[serde(default)]
    pub bgw: Option<String>,
    #[serde(default = "full")]
    pub bgw_volume: u8,
}

fn first() -> u16 {
    1
}

fn full() -> u8 {
    100
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger { sfx: None, sfx_volume: 100, bgm: None, bgm_loop: false, bgm_volume: 100, art: None, art_x: 1, art_y: 1, art_fade: 0, bgn: None, bgn_volume: 100, bgw: None, bgw_volume: 100 }
    }
}

impl Trigger {
    pub fn is_empty(&self) -> bool {
        self.sfx.is_none() && self.bgm.is_none() && self.art.is_none() && self.bgn.is_none() && self.bgw.is_none()
    }
}

/// A trigger that went off: the pair the game sent, and what it sets off.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Fired {
    pub key: String,
    pub value: String,
    pub trigger: Trigger,
}

fn flatten(key: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            for (name, inner) in fields {
                flatten(&format!("{key}.{name}"), inner, out);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for inner in items {
                flatten(key, inner, out);
            }
        }
        // A scalar, or an empty object or array, which is a value too.
        _ => out.push((key.to_string(), value.to_string())),
    }
}

/// A GMCP message as its pairs, in order: the key is the package in
/// lower case, then the path through the JSON. A payload that isn't JSON
/// is kept as the text it is; no payload is `null`.
pub fn pairs(package: &str, data: &str) -> Vec<(String, String)> {
    let data = data.trim();
    let value = if data.is_empty() { Value::Null } else { serde_json::from_str(data).unwrap_or_else(|_| Value::String(data.to_string())) };
    let mut found = Vec::new();
    flatten(&package.to_ascii_lowercase(), &value, &mut found);
    found
}

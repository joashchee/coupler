//! The web build's hooks: the desktop's triggers, mirrored. There's no
//! hooks database here and nothing is recorded: `hooks.json`, made by
//! `src-tauri/examples/web_mirror.rs` from the maintainer's Coupler and
//! deployed beside the page, is every trigger, and a GMCP message sets
//! off the ones its pairs match, as `hooks.rs` does on the desktop.

use std::collections::HashMap;

use serde::Deserialize;

pub use crate::trigger::{pairs, Fired, Trigger};

#[derive(Default)]
pub struct Hooks {
    triggers: HashMap<(String, String), Trigger>,
}

#[derive(Deserialize)]
struct Mirror {
    triggers: Vec<Mirrored>,
}

#[derive(Deserialize)]
struct Mirrored {
    key: String,
    value: String,
    trigger: Trigger,
}

impl Hooks {
    /// The mirror's `hooks.json`; no triggers when it's empty or unreadable.
    pub fn from_mirror(json: &str) -> Result<Self, String> {
        if json.trim().is_empty() {
            return Ok(Hooks::default());
        }
        let mirror: Mirror = serde_json::from_str(json).map_err(|e| format!("The hooks couldn't be read. ({e})"))?;
        Ok(Hooks { triggers: mirror.triggers.into_iter().map(|m| ((m.key, m.value), m.trigger)).collect() })
    }

    /// The triggers a GMCP message sets off, in the message's order.
    pub fn record(&self, package: &str, data: &str) -> Vec<Fired> {
        pairs(package, data)
            .into_iter()
            .filter_map(|(key, value)| {
                let trigger = self.triggers.get(&(key.clone(), value.clone()))?.clone();
                Some(Fired { key, value, trigger })
            })
            .collect()
    }

    /// What a pair sets off, if anything.
    pub fn trigger(&self, key: &str, value: &str) -> Option<&Trigger> {
        self.triggers.get(&(key.to_string(), value.to_string()))
    }

    pub fn len(&self) -> usize {
        self.triggers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mirrored_trigger_goes_off_on_its_pair() {
        let json = r#"{"version":1,"files":{},"triggers":[{"key":"room.info.zone","value":"\"Midgaard\"","trigger":{"sfx":"wav/bell.wav","sfxVolume":50,"bgm":null,"art":null}}]}"#;
        let hooks = Hooks::from_mirror(json).unwrap();
        let fired = hooks.record("Room.Info", r#"{"zone":"Midgaard","id":"Midgaard#3001"}"#);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].trigger.sfx.as_deref(), Some("wav/bell.wav"));
        assert_eq!(fired[0].trigger.sfx_volume, 50);
        assert!(hooks.record("Room.Info", r#"{"zone":"Elsewhere"}"#).is_empty());
    }

    #[test]
    fn no_mirror_is_no_triggers() {
        assert_eq!(Hooks::from_mirror("").unwrap().len(), 0);
    }
}

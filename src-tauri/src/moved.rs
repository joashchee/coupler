//! The Mac's move to Scoreless Music's bundle ID (decided 2026-10-19):
//! every ansiapps app on Apple's platforms is released as
//! `com.scorelessmusic.*`, so Coupler went from `com.ansiapps.coupler` to
//! `com.scorelessmusic.coupler`. The bundle ID names the app-data folder
//! and the WebView's storage (the `coupler.` settings), so at launch,
//! before either is opened, what the old ID kept is moved to the new
//! one's place. Only when the new place doesn't exist yet: a player who
//! already has data under the new ID keeps it, and the old stays put.
//! Windows and Linux keep the old ID (`tauri.windows.conf.json`,
//! `tauri.linux.conf.json`), so nothing moves there.

use std::path::Path;

/// The ID Coupler's Mac builds shipped with up to 0.28.0.
pub const OLD_ID: &str = "com.ansiapps.coupler";

/// The folders under `~/Library` that are named by the bundle ID: the
/// app data (rule 8) and WebKit's storage, where localStorage lives.
const KEPT_BY_ID: [&str; 2] = ["Application Support", "WebKit"];

/// Moves each folder the old ID kept to the new ID's name, under
/// `library` (`~/Library`). Returns what was moved, for the log.
pub fn from_old_id(library: &Path, new_id: &str) -> Vec<String> {
    let mut moved = Vec::new();
    for kind in KEPT_BY_ID {
        let old = library.join(kind).join(OLD_ID);
        let new = library.join(kind).join(new_id);
        if new_id == OLD_ID || !old.is_dir() || new.exists() {
            continue;
        }
        match std::fs::rename(&old, &new) {
            Ok(()) => moved.push(kind.to_string()),
            Err(e) => log::warn!("Coupler's {kind} folder couldn't be moved to its new name. ({e})"),
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("coupler-moved-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn moves_the_old_folders_once() {
        let lib = library("once");
        std::fs::create_dir_all(lib.join("Application Support").join(OLD_ID).join("maps")).unwrap();
        std::fs::create_dir_all(lib.join("WebKit").join(OLD_ID)).unwrap();
        let new = "com.scorelessmusic.coupler";
        assert_eq!(from_old_id(&lib, new), vec!["Application Support", "WebKit"]);
        assert!(lib.join("Application Support").join(new).join("maps").is_dir());
        assert!(!lib.join("Application Support").join(OLD_ID).exists());
        // Again: nothing left to move.
        assert!(from_old_id(&lib, new).is_empty());
        let _ = std::fs::remove_dir_all(&lib);
    }

    #[test]
    fn never_over_the_new_ids_own() {
        let lib = library("kept");
        let new = "com.scorelessmusic.coupler";
        std::fs::create_dir_all(lib.join("Application Support").join(OLD_ID)).unwrap();
        std::fs::create_dir_all(lib.join("Application Support").join(new)).unwrap();
        assert!(from_old_id(&lib, new).is_empty());
        assert!(lib.join("Application Support").join(OLD_ID).is_dir());
        // Nothing old at all: nothing happens.
        assert!(from_old_id(&library("none"), new).is_empty());
        let _ = std::fs::remove_dir_all(&lib);
    }
}

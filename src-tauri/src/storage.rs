//! **Storage**: what Coupler keeps in app data, by kind, and making room
//! (gear → Storage…, `components/StorageDialog.tsx`). Unit-tested
//! against a folder in the system's temp dir.
//!
//! `report` sizes each kind (`KINDS`): the player's assets by type, the
//! caches, the copy from before the last restore, the maps, the rooms'
//! looks, the voices, the hooks, the journal, the Bestiary, and the
//! rest. The caches (`CACHES`: the voice cache, CoffeeMUD's sounds as
//! rendered, the smaller copies waiting for an answer, a backup's
//! leftover copies) are made again as they're wanted, so clearing them
//! loses nothing but a moment the next time. Everything else the
//! dialog offers to delete is the player's and gone for good, after a
//! warning that offers a backup first: `delete_unused` (assets no
//! trigger uses), `remove` (a kind's files, for `session.rs`'s
//! `forget` once what has them open has let go).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::assets::{self, Kind};
use crate::backup::{BEFORE, RESTORING};

/// What each kind is, by the names it has in app data. A name ending in
/// `/` is a folder; `*` after a file's name takes in what SQLite keeps
/// beside it (`-wal`, `-shm`, `-journal`).
pub const KINDS: &[(&str, &[&str])] = &[
    ("voiceCache", &["voice-cache/"]),
    ("soundbankCache", &["soundbank-cache/"]),
    ("assetsWaiting", &["Assets/.smaller/"]),
    ("backupLeftovers", &[".backup-snapshots/"]),
    ("beforeRestore", &[BEFORE]),
    ("restoring", &[RESTORING]),
    ("maps", &["maps/"]),
    ("looks", &["pictures/"]),
    ("cast", &["cast/"]),
    ("hooks", &["hooks.sqlite*"]),
    ("journal", &["journal.sqlite*"]),
    ("bestiary", &["bestiary.sqlite*"]),
];

/// The kinds made again as they're wanted.
pub const CACHES: &[&str] = &["voiceCache", "soundbankCache", "assetsWaiting", "backupLeftovers"];

/// How much one kind takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Use {
    pub id: String,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Each kind, with the assets by type (`assetsSfx`, `assetsBgm`,
    /// `assetsArt`, `assetsFonts`) and everything else (`other`), in a
    /// fixed order; a kind with nothing is there with 0.
    pub uses: Vec<Use>,
    pub total: u64,
    /// The free space on the disk app data is on.
    pub free: Option<u64>,
}

fn size(path: &Path) -> (u64, u64) {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::read_dir(path).into_iter().flatten().flatten().fold((0, 0), |(b, f), e| {
            let (eb, ef) = size(&e.path());
            (b + eb, f + ef)
        }),
        Ok(m) => (m.len(), 1),
        Err(_) => (0, 0),
    }
}

/// The names in app data a kind's entry takes in.
fn matches(name: &str, pattern: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(file) => name == file || name.strip_prefix(file).is_some_and(|rest| rest.starts_with('-')),
        None => name == pattern.trim_end_matches('/'),
    }
}

/// What app data (`root`) holds, by kind.
pub fn report(root: &Path) -> Report {
    let mut uses: Vec<Use> = Vec::new();
    let mut counted: BTreeSet<String> = BTreeSet::new();
    // The assets by type, their folders inside Assets.
    let assets_dir = root.join("Assets");
    let mut by_kind = [("assetsSfx", Kind::Sfx, 0u64, 0u64), ("assetsBgm", Kind::Bgm, 0, 0), ("assetsArt", Kind::Art, 0, 0), ("assetsFonts", Kind::SoundFont, 0, 0)];
    let mut assets_other = (0, 0);
    for entry in fs::read_dir(&assets_dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".smaller" {
            continue;
        }
        let (b, f) = size(&entry.path());
        match assets::kind_of(&name).and_then(|k| by_kind.iter_mut().find(|(_, kind, _, _)| *kind == k)) {
            Some(slot) => {
                slot.2 += b;
                slot.3 += f;
            }
            None => {
                assets_other.0 += b;
                assets_other.1 += f;
            }
        }
    }
    counted.insert("Assets".into());
    for (id, _, bytes, files) in by_kind {
        uses.push(Use { id: id.into(), bytes, files });
    }
    for (id, names) in KINDS {
        let (mut bytes, mut files) = (0, 0);
        for pattern in *names {
            if let Some(inside) = pattern.strip_prefix("Assets/") {
                let (b, f) = size(&assets_dir.join(inside.trim_end_matches('/')));
                bytes += b;
                files += f;
                continue;
            }
            for entry in fs::read_dir(root).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if matches(&name, pattern) {
                    let (b, f) = size(&entry.path());
                    bytes += b;
                    files += f;
                    counted.insert(name);
                }
            }
        }
        uses.push(Use { id: (*id).into(), bytes, files });
    }
    let (mut bytes, mut files) = assets_other;
    for entry in fs::read_dir(root).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !counted.contains(&name) {
            let (b, f) = size(&entry.path());
            bytes += b;
            files += f;
        }
    }
    uses.push(Use { id: "other".into(), bytes, files });
    let total = uses.iter().map(|u| u.bytes).sum();
    Report { uses, total, free: crate::voicecache::free_space(root) }
}

/// Deletes a kind's files and folders in app data, and says how many
/// bytes that was. What has them open must have let go first.
pub fn remove(root: &Path, id: &str) -> u64 {
    let Some((_, names)) = KINDS.iter().find(|(k, _)| *k == id) else { return 0 };
    let mut freed = 0;
    for pattern in *names {
        let mut gone = |path: &Path| {
            freed += size(path).0;
            let _ = if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
        };
        if let Some(inside) = pattern.strip_prefix("Assets/") {
            gone(&root.join("Assets").join(inside.trim_end_matches('/')));
            continue;
        }
        for entry in fs::read_dir(root).into_iter().flatten().flatten() {
            if matches(&entry.file_name().to_string_lossy(), pattern) {
                gone(&entry.path());
            }
        }
    }
    freed
}

/// What deleting the assets not in use did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deleted {
    pub files: u64,
    pub bytes: u64,
}

/// Deletes every asset `in_use` says no trigger plays or shows, but a
/// SoundFont a MIDI file is played through and `keep` (the file open in
/// the Music Editor).
pub fn delete_unused(root: &Path, in_use: impl Fn(&str) -> bool, keep: Option<&str>) -> Deleted {
    let fonts = assets::fonts(root);
    let chosen: BTreeSet<&String> = fonts.values().collect();
    let mut deleted = Deleted { files: 0, bytes: 0 };
    for asset in assets::list(root) {
        if in_use(&asset.path) || chosen.contains(&asset.path) || Some(asset.path.as_str()) == keep {
            continue;
        }
        let Ok(file) = assets::resolve(root, &asset.path) else { continue };
        let bytes = size(&file).0;
        if fs::remove_file(&file).is_ok() {
            deleted.files += 1;
            deleted.bytes += bytes;
            if fonts.contains_key(&asset.path) {
                let _ = assets::set_font(root, &asset.path, None);
            }
        }
    }
    deleted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("coupler-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn file(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0u8; bytes]).unwrap();
    }

    fn bytes(r: &Report, id: &str) -> u64 {
        r.uses.iter().find(|u| u.id == id).unwrap().bytes
    }

    #[test]
    fn every_kind_is_sized_and_nothing_twice() {
        let root = temp("report");
        file(&root.join("Assets/wav/door.wav"), 100);
        file(&root.join("Assets/mid/song.mid"), 20);
        file(&root.join("Assets/.smaller/wav/door.wav.opus"), 30);
        file(&root.join("Assets/.soundfonts.json"), 2);
        file(&root.join("voice-cache/a.pcm"), 400);
        file(&root.join("voice-cache/index.sqlite"), 8);
        file(&root.join("journal.sqlite"), 50);
        file(&root.join("journal.sqlite-wal"), 5);
        file(&root.join("maps/coffeemud.json"), 7);
        file(&root.join("played.json"), 3);
        let r = report(&root);
        assert_eq!(bytes(&r, "assetsSfx"), 100);
        assert_eq!(bytes(&r, "assetsBgm"), 20);
        assert_eq!(bytes(&r, "assetsWaiting"), 30);
        assert_eq!(bytes(&r, "voiceCache"), 408);
        assert_eq!(bytes(&r, "journal"), 55);
        assert_eq!(bytes(&r, "maps"), 7);
        assert_eq!(bytes(&r, "bestiary"), 0);
        assert_eq!(bytes(&r, "other"), 5);
        assert_eq!(r.total, 100 + 20 + 30 + 2 + 408 + 55 + 7 + 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_kind_is_removed_whole() {
        let root = temp("remove");
        file(&root.join("bestiary.sqlite"), 10);
        file(&root.join("bestiary.sqlite-shm"), 4);
        file(&root.join("bestiary.sqlite.unreadable"), 1);
        file(&root.join("Assets/.smaller/wav/a.opus"), 6);
        assert_eq!(remove(&root, "bestiary"), 14);
        assert!(!root.join("bestiary.sqlite").exists() && !root.join("bestiary.sqlite-shm").exists());
        // Not one of its names.
        assert!(root.join("bestiary.sqlite.unreadable").exists());
        assert_eq!(remove(&root, "assetsWaiting"), 6);
        assert!(root.join("Assets").is_dir() && !root.join("Assets/.smaller").exists());
        assert_eq!(remove(&root, "nothing"), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn only_what_nothing_uses_is_deleted() {
        let root = temp("unused");
        let assets = root.join("Assets");
        file(&assets.join("wav/used.wav"), 10);
        file(&assets.join("wav/unused.wav"), 20);
        file(&assets.join("sf2/chosen.sf2"), 30);
        file(&assets.join("sf2/spare.sf2"), 40);
        file(&assets.join("mid/open.mid"), 5);
        file(&assets.join("mid/old.mid"), 6);
        assets::set_font(&assets, "mid/open.mid", Some("sf2/chosen.sf2")).unwrap();
        let done = delete_unused(&assets, |p| p == "wav/used.wav", Some("mid/open.mid"));
        assert_eq!(done, Deleted { files: 3, bytes: 20 + 40 + 6 });
        let left: Vec<String> = assets::list(&assets).into_iter().map(|a| a.path).collect();
        assert_eq!(left, ["mid/open.mid", "sf2/chosen.sf2", "wav/used.wav"]);
        let _ = fs::remove_dir_all(&root);
    }
}

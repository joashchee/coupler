//! **Coupler Backup**: everything the player has made Coupler theirs
//! with, in one file as small as it can be made, and put back from it.
//!
//! What's kept (`KEPT`, the app-data items by name): the Assets folder
//! (but the smaller copies waiting for an answer), the maps, the rooms'
//! looks, the casts and their voices, the hooks database, the journal and
//! the log, the Bestiary and the Items, the fights kept, and the times
//! played; with the
//! settings the frontend keeps (every `coupler.` key in its storage, as
//! JSON). Not the voice cache
//! (made again as lines come) nor the first-run warning's answer. A new
//! kind of data in app data joins `KEPT`.
//!
//! The file (`.coupler`) is `backup_format.rs`, shared with the web
//! build, which writes and reads it in memory.
//!
//! The databases are read through SQLite's `VACUUM INTO`, a consistent,
//! compacted copy even while the game is being played.
//!
//! Restoring never touches what's open: `unpack` checks every block and
//! writes the backup to `RESTORING` in app data, and on the next launch,
//! before anything is opened, `apply` moves what's there now aside (to
//! `BEFORE`, one restore's worth, cleared by the next) and the backup in.
//! The settings wait in `SETTINGS` for the window (`take_settings`).
//! Pure but for files; unit-tested.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};

pub use crate::backup_format::*;

/// Inside the kept items, left out: the smaller copies waiting for an answer.
const LEFT_OUT: &[&str] = &["Assets/.smaller"];

/// A backup waiting to be applied at the next launch.
pub const RESTORING: &str = ".restoring";
/// What was there before the last restore.
pub const BEFORE: &str = ".before-restore";
/// The restored settings, waiting for the window.
pub const SETTINGS: &str = "restored-settings.json";
/// Marks a backup unpacked whole into `RESTORING`.
const COMPLETE: &str = ".complete";
/// Where the databases' copies are made while backing up.
const SNAPSHOTS: &str = ".backup-snapshots";

/// Left out wherever it is: what SQLite keeps beside a database, a file
/// put aside as unreadable, the Finder's.
fn ignored(name: &str) -> bool {
    name.ends_with("-wal") || name.ends_with("-shm") || name.ends_with("-journal") || name.ends_with(".unreadable") || name == ".DS_Store"
}

/// Every file to back up in `root` (app data), databases as their
/// copies in `SNAPSHOTS`, in the order they're written: the small ones
/// by type, then the rest by size.
pub fn collect(root: &Path) -> Result<Vec<Item>, String> {
    let snapshots = root.join(SNAPSHOTS);
    let _ = fs::remove_dir_all(&snapshots);
    let mut items = Vec::new();
    for name in KEPT {
        let path = root.join(name);
        if path.is_dir() {
            walk(&path, name, &mut items).map_err(|e| format!("Couldn't read {name}. ({e})"))?;
        } else if path.is_file() {
            let source = if name.ends_with(".sqlite") { snapshot(&path, &snapshots, name)? } else { path };
            let size = fs::metadata(&source).map_err(|e| e.to_string())?.len();
            items.push(Item { path: (*name).to_string(), source, size });
        }
    }
    items.sort_by(|a, b| (own_block(a), extension(&a.path), a.size, &a.path).cmp(&(own_block(b), extension(&b.path), b.size, &b.path)));
    Ok(items)
}

fn walk(dir: &Path, prefix: &str, items: &mut Vec<Item>) -> io::Result<()> {
    if LEFT_OUT.contains(&prefix) {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
        if ignored(&name) {
            continue;
        }
        let path = format!("{prefix}/{name}");
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), &path, items)?;
        } else if kind.is_file() {
            items.push(Item { path, source: entry.path(), size: entry.metadata()?.len() });
        }
    }
    Ok(())
}

/// A database's consistent, compacted copy.
fn snapshot(path: &Path, snapshots: &Path, name: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(snapshots).map_err(|e| e.to_string())?;
    let copy = snapshots.join(name);
    let db = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| format!("Couldn't open {name}. ({e})"))?;
    db.execute("VACUUM INTO ?1", [copy.to_string_lossy()]).map_err(|e| format!("Couldn't copy {name}. ({e})"))?;
    Ok(copy)
}

/// Clears the databases' copies, once a backup is written (or isn't).
pub fn clear_snapshots(root: &Path) {
    let _ = fs::remove_dir_all(root.join(SNAPSHOTS));
}

/// Writes a backup of `items`, read from their files, and `settings`
/// (`smallest`: `write_with`'s).
pub fn write<W: Write + Seek>(out: &mut W, app: &str, made: u64, settings: &str, items: &[Item], smallest: bool, progress: impl FnMut(Progress)) -> Result<Manifest, String> {
    write_with(out, app, made, settings, items, smallest, |item| File::open(&item.source), progress)
}

/// Unpacks a backup into `RESTORING` in `root`, checking every block,
/// for `apply` at the next launch. Anything wrong and nothing's left
/// there.
pub fn unpack<R: Read>(r: &mut R, root: &Path, progress: impl FnMut(Progress)) -> Result<Manifest, String> {
    let into = root.join(RESTORING);
    let _ = fs::remove_dir_all(&into);
    let result = unpack_into(r, &into, progress);
    if result.is_err() {
        let _ = fs::remove_dir_all(&into);
    }
    result
}
fn unpack_into<R: Read>(r: &mut R, into: &Path, progress: impl FnMut(Progress)) -> Result<Manifest, String> {
    fs::create_dir_all(into).map_err(|e| format!("Couldn't make room for the backup. ({e})"))?;
    let manifest = read(
        r,
        |json| fs::write(into.join("settings.json"), json).map_err(|e| e.to_string()),
        |path, _, from| {
            let to = into.join(path);
            fs::create_dir_all(to.parent().expect("a kept path has a parent")).map_err(|e| e.to_string())?;
            let mut file = BufWriter::new(File::create(&to).map_err(|e| format!("Couldn't write {path}. ({e})"))?);
            io::copy(from, &mut file).map_err(|_| DAMAGED.to_string())?;
            file.flush().map_err(|e| e.to_string())
        },
        progress,
    )?;
    fs::write(into.join(COMPLETE), b"").map_err(|e| e.to_string())?;
    Ok(manifest)
}

/// At launch, before anything's opened: a backup unpacked whole takes
/// the place of what's there now, which is moved to `BEFORE`. True when
/// one was applied.
pub fn apply(root: &Path) -> Result<bool, String> {
    let staged = root.join(RESTORING);
    if !staged.join(COMPLETE).is_file() {
        // Half unpacked (Coupler quit during it): nothing to apply.
        if staged.exists() {
            let _ = fs::remove_dir_all(&staged);
        }
        return Ok(false);
    }
    let before = root.join(BEFORE);
    let _ = fs::remove_dir_all(&before);
    fs::create_dir_all(&before).map_err(|e| e.to_string())?;
    let io = |e: io::Error| format!("Couldn't restore the backup. ({e})");
    for name in KEPT {
        let now = root.join(name);
        let mut aside = vec![(now.clone(), before.join(name))];
        if name.ends_with(".sqlite") {
            for side in ["-wal", "-shm", "-journal"] {
                aside.push((root.join(format!("{name}{side}")), before.join(format!("{name}{side}"))));
            }
        }
        for (from, to) in aside {
            if from.exists() {
                fs::rename(&from, &to).map_err(io)?;
            }
        }
        let new = staged.join(name);
        if new.exists() {
            fs::rename(&new, &now).map_err(io)?;
        }
    }
    let settings = staged.join("settings.json");
    if settings.is_file() {
        fs::rename(&settings, root.join(SETTINGS)).map_err(io)?;
    }
    fs::remove_dir_all(&staged).map_err(io)?;
    Ok(true)
}

/// The restored settings, once, as a JSON object of strings: read and
/// the file removed.
pub fn take_settings(root: &Path) -> Option<BTreeMap<String, String>> {
    let path = root.join(SETTINGS);
    let json = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(&path);
    serde_json::from_str(&json).ok()
}

/// The script that puts the restored settings in the WebView's storage
/// before the page's own scripts run: every `coupler.` key replaced
/// (but the dev-only App Testing results), once per restore (`nonce`).
pub fn settings_script(settings: &BTreeMap<String, String>, nonce: u64) -> String {
    let json = serde_json::to_string(settings).expect("strings serialize");
    format!(
        "(() => {{ try {{ const s = localStorage; if (s.getItem(\"coupler.restored\") === \"{nonce}\") return; \
         for (const k of Object.keys(s)) if (k.startsWith(\"coupler.\") && k !== \"coupler.appTestingResults\") s.removeItem(k); \
         for (const [k, v] of Object.entries({json})) if (k.startsWith(\"coupler.\")) s.setItem(k, v); \
         s.setItem(\"coupler.restored\", \"{nonce}\"); }} catch {{}} }})();"
    )
}

/// A backup's file, read buffered.
pub fn open(path: &Path) -> Result<BufReader<File>, String> {
    File::open(path).map(|f| BufReader::with_capacity(1 << 20, f)).map_err(|e| format!("Couldn't open the backup. ({e})"))
}

/// Writes a backup of `root` to `to`, through a file beside it renamed
/// once whole, so a backup cut short never takes the name; `smallest`,
/// as small as Brotli makes it, however long that takes.
pub fn export(root: &Path, to: &Path, app: &str, made: u64, settings: &str, smallest: bool, progress: impl FnMut(Progress)) -> Result<Made, String> {
    let items = collect(root);
    let result = items.and_then(|items| {
        let part = to.with_extension(format!("{EXTENSION}.part"));
        let written = File::create(&part).map_err(|e| format!("Couldn't write the backup. ({e})")).and_then(|f| {
            let mut out = BufWriter::with_capacity(1 << 20, f);
            let manifest = write(&mut out, app, made, settings, &items, smallest, progress)?;
            out.into_inner().map_err(|e| e.to_string())?.sync_all().map_err(|e| e.to_string())?;
            Ok(manifest)
        });
        match written {
            Ok(manifest) => {
                fs::rename(&part, to).map_err(|e| format!("Couldn't write the backup. ({e})"))?;
                let size = fs::metadata(to).map(|m| m.len()).unwrap_or(0);
                Ok(Made { manifest, size })
            }
            Err(e) => {
                let _ = fs::remove_file(&part);
                Err(e)
            }
        }
    });
    clear_snapshots(root);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!("coupler-backup-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn put(root: &Path, path: &str, bytes: &[u8]) {
        let to = root.join(path);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(to, bytes).unwrap();
    }

    /// App data as Coupler leaves it.
    fn data(root: &Path) {
        put(root, "Assets/wav/door.wav", &[7u8; 5000]);
        put(root, "Assets/ogg/rain.ogg", b"OggS already small");
        put(root, "Assets/ans/gate.ans", "\x1b[1;33mA gate\x1b[0m\r\n".repeat(50).as_bytes());
        put(root, "Assets/.smaller/door.opus", b"waiting");
        put(root, "Assets/.soundfonts.json", b"{}");
        put(root, "maps/coffeemud.json", br#"{"rooms":[]}"#);
        put(root, "maps/old.json.unreadable", b"broken");
        put(root, "cast/coffeemud.json", br#"{"people":{}}"#);
        put(root, "played.json", br#"{"times":3}"#);
        put(root, "in-development-accepted", b"");
        put(root, "voice-cache/abc.pcm", b"cached");
        let db = rusqlite::Connection::open(root.join("hooks.sqlite")).unwrap();
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE pairs (k TEXT); INSERT INTO pairs VALUES ('room.info.zone');").unwrap();
        drop(db);
    }

    const SETTINGS_JSON: &str = r#"{"coupler.theme":"modern","coupler.ux":"immersive"}"#;

    fn backup(root: &Path) -> Vec<u8> {
        let items = collect(root).unwrap();
        let mut out = Cursor::new(Vec::new());
        write(&mut out, "0.27.0", 1_800_000_000, SETTINGS_JSON, &items, false, |_| ()).unwrap();
        clear_snapshots(root);
        out.into_inner()
    }

    #[test]
    fn collects_what_is_kept_and_nothing_else() {
        let root = TempDir::new("collect");
        data(&root.0);
        let paths: Vec<String> = collect(&root.0).unwrap().into_iter().map(|i| i.path).collect();
        for kept in ["Assets/wav/door.wav", "Assets/ogg/rain.ogg", "Assets/ans/gate.ans", "Assets/.soundfonts.json", "maps/coffeemud.json", "cast/coffeemud.json", "played.json", "hooks.sqlite"] {
            assert!(paths.contains(&kept.to_string()), "{kept} in {paths:?}");
        }
        for left in ["Assets/.smaller/door.opus", "maps/old.json.unreadable", "in-development-accepted", "voice-cache/abc.pcm"] {
            assert!(!paths.iter().any(|p| p == left), "{left} left out");
        }
        // The small files first, the ones in blocks of their own after.
        let first_own = paths.iter().position(|p| p.ends_with(".wav") || p.ends_with(".ogg")).unwrap();
        assert!(paths[first_own..].iter().all(|p| p.ends_with(".wav") || p.ends_with(".ogg")));
        clear_snapshots(&root.0);
        assert!(!root.0.join(SNAPSHOTS).exists());
    }

    #[test]
    fn round_trips_through_a_launch() {
        let from = TempDir::new("from");
        data(&from.0);
        let bytes = backup(&from.0);
        assert!(bytes.len() < 5000, "compressed: {} bytes", bytes.len());

        let to = TempDir::new("to");
        put(&to.0, "maps/coffeemud.json", b"the old map");
        put(&to.0, "Assets/sfx/old.wav", b"old");
        put(&to.0, "voice-cache/keep.pcm", b"kept");
        let mut steps = Vec::new();
        let manifest = unpack(&mut Cursor::new(&bytes), &to.0, |p| steps.push(p)).unwrap();
        assert_eq!(manifest.app, "0.27.0");
        assert_eq!(manifest.settings, 2);
        assert_eq!(steps.last().unwrap().done, manifest.bytes);
        // Nothing changes until the next launch.
        assert_eq!(fs::read(to.0.join("maps/coffeemud.json")).unwrap(), b"the old map");

        assert!(apply(&to.0).unwrap());
        assert_eq!(fs::read(to.0.join("maps/coffeemud.json")).unwrap(), br#"{"rooms":[]}"#);
        assert_eq!(fs::read(to.0.join("Assets/wav/door.wav")).unwrap(), vec![7u8; 5000]);
        assert_eq!(fs::read(to.0.join("Assets/ogg/rain.ogg")).unwrap(), b"OggS already small");
        assert!(!to.0.join("Assets/sfx/old.wav").exists());
        assert_eq!(fs::read(to.0.join(BEFORE).join("Assets/sfx/old.wav")).unwrap(), b"old");
        assert_eq!(fs::read(to.0.join("voice-cache/keep.pcm")).unwrap(), b"kept");
        let db = rusqlite::Connection::open(to.0.join("hooks.sqlite")).unwrap();
        let k: String = db.query_row("SELECT k FROM pairs", [], |r| r.get(0)).unwrap();
        assert_eq!(k, "room.info.zone");
        assert!(!to.0.join(RESTORING).exists());

        let settings = take_settings(&to.0).unwrap();
        assert_eq!(settings.get("coupler.theme").map(String::as_str), Some("modern"));
        assert!(take_settings(&to.0).is_none(), "only once");
        assert!(!apply(&to.0).unwrap(), "nothing more to apply");
    }

    #[test]
    fn a_damaged_backup_restores_nothing() {
        let from = TempDir::new("damaged");
        data(&from.0);
        let mut bytes = backup(&from.0);
        let at = bytes.len() / 2;
        bytes[at] ^= 0x55;
        let to = TempDir::new("damaged-to");
        assert!(unpack(&mut Cursor::new(&bytes), &to.0, |_| ()).is_err());
        assert!(!to.0.join(RESTORING).exists());
        assert!(!apply(&to.0).unwrap());
        // Cut short too.
        let whole = backup(&from.0);
        assert!(unpack(&mut Cursor::new(&whole[..whole.len() - 10]), &to.0, |_| ()).is_err());
    }

    #[test]
    fn not_a_backup() {
        assert_eq!(inspect(&mut Cursor::new(b"hello there, this is a text file")).unwrap_err(), NOT_A_BACKUP);
    }

    #[test]
    fn paths_stay_inside() {
        assert!(safe("Assets/wav/door.wav"));
        assert!(safe("hooks.sqlite"));
        for bad in ["../evil", "Assets/../../evil", "/etc/passwd", "voice-cache/x", "Assets//x", "Assets/a\\..\\b", "first-run", "C:/x"] {
            assert!(!safe(bad), "{bad}");
        }
    }

    #[test]
    fn a_path_outside_is_refused() {
        // A backup naming a path outside what's kept is damaged.
        let mut out = Cursor::new(Vec::new());
        let root = TempDir::new("outside");
        put(&root.0, "evil.txt", b"x");
        let items = vec![Item { path: "../evil.txt".into(), source: root.0.join("evil.txt"), size: 1 }];
        write(&mut out, "0.27.0", 0, "{}", &items, false, |_| ()).unwrap();
        let to = TempDir::new("outside-to");
        assert_eq!(unpack(&mut Cursor::new(out.into_inner()), &to.0, |_| ()).unwrap_err(), DAMAGED);
        assert!(!to.0.parent().unwrap().join("evil.txt").exists());
    }

    #[test]
    fn the_settings_script_sets_each_once() {
        let mut s = BTreeMap::new();
        s.insert("coupler.theme".to_string(), "modern\"</script>".to_string());
        let script = settings_script(&s, 42);
        assert!(script.contains(r#""coupler.restored") === "42""#));
        assert!(script.contains(r#""coupler.theme":"modern\"</script>""#));
    }
}

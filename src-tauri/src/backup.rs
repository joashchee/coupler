//! **Coupler Backup**: everything the player has made Coupler theirs
//! with, in one file as small as it can be made, and put back from it.
//!
//! What's kept (`KEPT`, the app-data items by name): the Assets folder
//! (but the smaller copies waiting for an answer), the maps, the rooms'
//! looks, the casts and their voices, the hooks database, the journal and
//! the log, and the times played; with the settings the frontend keeps
//! (every `coupler.` key in its storage, as JSON). Not the voice cache
//! (made again as lines come) nor the first-run warning's answer. A new
//! kind of data in app data joins `KEPT`.
//!
//! The file (`.coupler`): `MAGIC`, the manifest's length (u32, little
//! endian) and the manifest (JSON: what made it and what's in it), then
//! blocks, each a method (`STORED` or `BROTLI`), its length (u64), its
//! bytes and the CRC-32 of what they hold, and `END`. What a block holds
//! is records: a path's length (u16), the path, the size (u64) and the
//! bytes. The first block is the settings and every small file together,
//! in Brotli's strongest setting with its largest window, files of a
//! type side by side so each finds its likes; a sound or SoundFont (raw
//! samples) is a block of its own, Brotli again; what's compressed
//! already (Ogg, Opus, MP3, WavPack, PNG, JPEG) is stored as it is,
//! since squeezing it again only takes time.
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
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use brotli::enc::BrotliEncoderParams;
use serde::{Deserialize, Serialize};

/// The start of every backup.
pub const MAGIC: &[u8; 16] = b"COUPLER BACKUP\n\0";
/// The format this writes and reads.
pub const FORMAT: u32 = 1;
/// The extension a backup's file name ends in.
pub const EXTENSION: &str = "coupler";

/// What's backed up and restored, by name in app data.
pub const KEPT: &[&str] = &["Assets", "maps", "pictures", "cast", "hooks.sqlite", "journal.sqlite", "played.json"];
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
/// The settings' record in a backup.
const SETTINGS_RECORD: &str = "@settings";

const STORED: u8 = 0;
const BROTLI: u8 = 1;
const END: u8 = 0xFF;

/// Already compressed: stored as it is.
const PACKED: &[&str] = &["ogg", "opus", "mp3", "wv", "png", "jpg", "jpeg"];
/// Raw samples: a block of their own.
const SAMPLES: &[&str] = &["wav", "sf2"];
/// A file this big is a block of its own whatever it is.
const OWN_BLOCK: u64 = 16 << 20;
/// Up to this size a block of its own gets Brotli's strongest setting;
/// past it, one a little faster (the strongest takes minutes a gigabyte).
const STRONGEST_UP_TO: u64 = 8 << 20;

/// What's in a backup, written at its start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    /// The Coupler version that made it.
    pub app: String,
    /// When, in seconds since 1970.
    pub made: u64,
    /// How many files.
    pub files: usize,
    /// The files' size, uncompressed.
    pub bytes: u64,
    /// How many settings.
    pub settings: usize,
}

/// How far a backup or a restore has got, in bytes of the files.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

/// A backup made.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Made {
    pub manifest: Manifest,
    /// The backup's own size.
    pub size: u64,
}

/// A file to back up: its path in the backup, and where to read it.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub path: String,
    pub source: PathBuf,
    pub size: u64,
}

fn extension(path: &str) -> String {
    Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

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

/// Whether a file is a block of its own.
fn own_block(item: &Item) -> bool {
    let ext = extension(&item.path);
    item.size > OWN_BLOCK || PACKED.contains(&ext.as_str()) || SAMPLES.contains(&ext.as_str())
}

fn params(quality: i32, size_hint: u64) -> BrotliEncoderParams {
    BrotliEncoderParams { quality, lgwin: 24, size_hint: size_hint.min(usize::MAX as u64) as usize, ..Default::default() }
}

/// Counts what's written through it, and the CRC.
struct Counting<W: Write> {
    inner: W,
    crc: flate2::Crc,
}

impl<W: Write> Write for Counting<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.crc.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Writes a backup of `items` and `settings` (a JSON object of strings).
pub fn write<W: Write + Seek>(out: &mut W, app: &str, made: u64, settings: &str, items: &[Item], mut progress: impl FnMut(Progress)) -> Result<Manifest, String> {
    let parsed: BTreeMap<String, String> = serde_json::from_str(settings).map_err(|e| format!("The settings couldn't be read. ({e})"))?;
    let manifest = Manifest { format: FORMAT, app: app.to_string(), made, files: items.len(), bytes: items.iter().map(|i| i.size).sum(), settings: parsed.len() };
    let io = |e: io::Error| format!("Couldn't write the backup. ({e})");
    out.write_all(MAGIC).map_err(io)?;
    let json = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
    out.write_all(&(json.len() as u32).to_le_bytes()).map_err(io)?;
    out.write_all(&json).map_err(io)?;
    let total = manifest.bytes;
    let mut done = 0u64;
    progress(Progress { done, total });

    // The settings and the small files, together.
    let (own, together): (Vec<&Item>, Vec<&Item>) = items.iter().partition(|i| own_block(i));
    let hint = settings.len() as u64 + together.iter().map(|i| i.size).sum::<u64>();
    block(out, BROTLI, Some(params(11, hint)), |w| {
        record(w, SETTINGS_RECORD, settings.len() as u64, &mut settings.as_bytes())?;
        for item in &together {
            let mut file = File::open(&item.source)?;
            record(w, &item.path, item.size, &mut file)?;
            done += item.size;
            progress(Progress { done, total });
        }
        Ok(())
    })
    .map_err(io)?;

    for item in own {
        let packed = PACKED.contains(&extension(&item.path).as_str());
        let (method, how) = if packed { (STORED, None) } else { (BROTLI, Some(params(if item.size <= STRONGEST_UP_TO { 11 } else { 9 }, item.size))) };
        block(out, method, how, |w| {
            let mut file = File::open(&item.source)?;
            record(w, &item.path, item.size, &mut file)
        })
        .map_err(io)?;
        done += item.size;
        progress(Progress { done, total });
    }
    out.write_all(&[END]).map_err(io)?;
    out.flush().map_err(io)?;
    Ok(manifest)
}

/// One block: its method, a length filled in once known, what `fill`
/// writes (compressed or not), and the CRC of what it wrote.
fn block<W: Write + Seek>(out: &mut W, method: u8, how: Option<BrotliEncoderParams>, fill: impl FnOnce(&mut dyn Write) -> io::Result<()>) -> io::Result<()> {
    out.write_all(&[method])?;
    let at = out.stream_position()?;
    out.write_all(&0u64.to_le_bytes())?;
    let crc = {
        let sink = BufWriter::with_capacity(1 << 20, &mut *out);
        match how {
            Some(p) => {
                let mut counting = Counting { inner: brotli::CompressorWriter::with_params(sink, 1 << 16, &p), crc: flate2::Crc::new() };
                fill(&mut counting)?;
                let crc = counting.crc.sum();
                let mut sink = counting.inner.into_inner();
                sink.flush()?;
                crc
            }
            None => {
                let mut counting = Counting { inner: sink, crc: flate2::Crc::new() };
                fill(&mut counting)?;
                counting.inner.flush()?;
                counting.crc.sum()
            }
        }
    };
    let end = out.stream_position()?;
    out.seek(SeekFrom::Start(at))?;
    out.write_all(&(end - at - 8).to_le_bytes())?;
    out.seek(SeekFrom::Start(end))?;
    out.write_all(&crc.to_le_bytes())
}

fn record(w: &mut dyn Write, path: &str, size: u64, from: &mut dyn Read) -> io::Result<()> {
    w.write_all(&(path.len() as u16).to_le_bytes())?;
    w.write_all(path.as_bytes())?;
    w.write_all(&size.to_le_bytes())?;
    let copied = io::copy(&mut from.take(size), w)?;
    if copied != size {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, format!("{path} changed while it was being backed up")));
    }
    Ok(())
}

const NOT_A_BACKUP: &str = "That isn't a Coupler Backup.";

/// A backup's manifest, read from its start.
pub fn inspect<R: Read>(r: &mut R) -> Result<Manifest, String> {
    let mut magic = [0u8; 16];
    r.read_exact(&mut magic).map_err(|_| NOT_A_BACKUP.to_string())?;
    if &magic != MAGIC {
        return Err(NOT_A_BACKUP.to_string());
    }
    let mut len = [0u8; 4];
    r.read_exact(&mut len).map_err(|_| NOT_A_BACKUP.to_string())?;
    let len = u32::from_le_bytes(len) as usize;
    if len > 1 << 16 {
        return Err(NOT_A_BACKUP.to_string());
    }
    let mut json = vec![0u8; len];
    r.read_exact(&mut json).map_err(|_| NOT_A_BACKUP.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&json).map_err(|_| NOT_A_BACKUP.to_string())?;
    if manifest.format != FORMAT {
        return Err(format!("This backup was made by a newer Coupler ({}). Update Coupler to restore it.", manifest.app));
    }
    Ok(manifest)
}

/// Whether a path from a backup is one Coupler would have written: inside
/// a kept item, nothing but plain names.
fn safe(path: &str) -> bool {
    let mut parts = path.split('/');
    let first = parts.next().unwrap_or("");
    KEPT.contains(&first) && path.split('/').all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains(['\\', ':', '\0']))
}

const DAMAGED: &str = "The backup is damaged, so nothing was restored.";

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

fn unpack_into<R: Read>(r: &mut R, into: &Path, mut progress: impl FnMut(Progress)) -> Result<Manifest, String> {
    let manifest = inspect(r)?;
    fs::create_dir_all(into).map_err(|e| format!("Couldn't make room for the backup. ({e})"))?;
    let total = manifest.bytes;
    let mut done = 0u64;
    let mut files = 0usize;
    let mut settings = false;
    progress(Progress { done, total });
    loop {
        let mut method = [0u8; 1];
        r.read_exact(&mut method).map_err(|_| DAMAGED.to_string())?;
        if method[0] == END {
            break;
        }
        let mut len = [0u8; 8];
        r.read_exact(&mut len).map_err(|_| DAMAGED.to_string())?;
        let mut body = Read::take(&mut *r, u64::from_le_bytes(len));
        let mut crc = flate2::Crc::new();
        {
            let mut reader: Box<dyn Read> = match method[0] {
                STORED => Box::new(&mut body),
                BROTLI => Box::new(brotli::Decompressor::new(&mut body, 1 << 16)),
                _ => return Err(DAMAGED.to_string()),
            };
            let mut reader = CrcReader { inner: &mut reader, crc: &mut crc };
            while let Some((path, size)) = next_record(&mut reader)? {
                if path == SETTINGS_RECORD {
                    let mut json = String::new();
                    (&mut reader).take(size).read_to_string(&mut json).map_err(|_| DAMAGED.to_string())?;
                    serde_json::from_str::<BTreeMap<String, String>>(&json).map_err(|_| DAMAGED.to_string())?;
                    fs::write(into.join("settings.json"), json).map_err(|e| e.to_string())?;
                    settings = true;
                    continue;
                }
                if !safe(&path) {
                    return Err(DAMAGED.to_string());
                }
                let to = into.join(&path);
                fs::create_dir_all(to.parent().expect("a kept path has a parent")).map_err(|e| e.to_string())?;
                let mut file = BufWriter::new(File::create(&to).map_err(|e| format!("Couldn't write {path}. ({e})"))?);
                let copied = io::copy(&mut (&mut reader).take(size), &mut file).map_err(|_| DAMAGED.to_string())?;
                file.flush().map_err(|e| e.to_string())?;
                if copied != size {
                    return Err(DAMAGED.to_string());
                }
                files += 1;
                done += size;
                progress(Progress { done, total });
            }
        }
        // Whatever the decompressor left unread, then the CRC.
        io::copy(&mut body, &mut io::sink()).map_err(|_| DAMAGED.to_string())?;
        let mut sum = [0u8; 4];
        r.read_exact(&mut sum).map_err(|_| DAMAGED.to_string())?;
        if u32::from_le_bytes(sum) != crc.sum() {
            return Err(DAMAGED.to_string());
        }
    }
    if !settings || files != manifest.files {
        return Err(DAMAGED.to_string());
    }
    fs::write(into.join(COMPLETE), b"").map_err(|e| e.to_string())?;
    Ok(manifest)
}

struct CrcReader<'a, R: Read + ?Sized> {
    inner: &'a mut R,
    crc: &'a mut flate2::Crc,
}

impl<R: Read + ?Sized> Read for CrcReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.crc.update(&buf[..n]);
        Ok(n)
    }
}

/// The next record's path and size, or None at the block's end.
fn next_record(r: &mut impl Read) -> Result<Option<(String, u64)>, String> {
    let mut len = [0u8; 2];
    match r.read(&mut len[..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(_) => return Err(DAMAGED.to_string()),
    }
    r.read_exact(&mut len[1..]).map_err(|_| DAMAGED.to_string())?;
    let mut path = vec![0u8; u16::from_le_bytes(len) as usize];
    r.read_exact(&mut path).map_err(|_| DAMAGED.to_string())?;
    let path = String::from_utf8(path).map_err(|_| DAMAGED.to_string())?;
    let mut size = [0u8; 8];
    r.read_exact(&mut size).map_err(|_| DAMAGED.to_string())?;
    Ok(Some((path, u64::from_le_bytes(size))))
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
/// once whole, so a backup cut short never takes the name.
pub fn export(root: &Path, to: &Path, app: &str, made: u64, settings: &str, progress: impl FnMut(Progress)) -> Result<Made, String> {
    let items = collect(root);
    let result = items.and_then(|items| {
        let part = to.with_extension(format!("{EXTENSION}.part"));
        let written = File::create(&part).map_err(|e| format!("Couldn't write the backup. ({e})")).and_then(|f| {
            let mut out = BufWriter::with_capacity(1 << 20, f);
            let manifest = write(&mut out, app, made, settings, &items, progress)?;
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
        write(&mut out, "0.27.0", 1_800_000_000, SETTINGS_JSON, &items, |_| ()).unwrap();
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
        write(&mut out, "0.27.0", 0, "{}", &items, |_| ()).unwrap();
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

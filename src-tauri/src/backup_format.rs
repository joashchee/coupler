//! **Coupler Backup**'s file (`.coupler`), shared with the web build:
//! what `backup.rs` writes and reads on the desktop, and what the web
//! build writes and reads in memory (`write_all`, `read_all`), so a
//! backup made in one opens in the other.
//!
//! The file: `MAGIC`, the manifest's length (u32, little endian) and the
//! manifest (JSON: what made it and what's in it), then blocks, each a
//! method (`STORED` or `BROTLI`), its length (u64), its bytes and the
//! CRC-32 of what they hold, and `END`. What a block holds is records: a
//! path's length (u16), the path, the size (u64) and the bytes. The
//! first block is the settings and every small file together, in
//! Brotli's strongest setting with its largest window, files of a type
//! side by side so each finds its likes; a sound or SoundFont (raw
//! samples) is a block of its own, Brotli again; what's compressed
//! already (Ogg, Opus, MP3, WavPack, PNG, JPEG) is stored as it is,
//! since squeezing it again only takes time. Pure; unit-tested here and
//! in `backup.rs`.

use std::collections::BTreeMap;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use brotli::enc::BrotliEncoderParams;
use serde::{Deserialize, Serialize};

/// The start of every backup.
pub const MAGIC: &[u8; 16] = b"COUPLER BACKUP\n\0";
/// The format this writes and reads.
pub const FORMAT: u32 = 1;
/// The extension a backup's file name ends in.
pub const EXTENSION: &str = "coupler";

/// What's backed up and restored, by name in app data. `web` is the web
/// build's journal and Bestiary (its own JSON, not the desktop's
/// databases), kept so a web backup restored here loses nothing.
pub const KEPT: &[&str] = &["Assets", "maps", "pictures", "cast", "hooks.sqlite", "journal.sqlite", "bestiary.sqlite", "fights", "played.json", "web"];

/// The settings' record in a backup.
const SETTINGS_RECORD: &str = "@settings";

const STORED: u8 = 0;
const BROTLI: u8 = 1;
const END: u8 = 0xFF;

/// Already compressed: stored as it is.
const PACKED: &[&str] = &["ogg", "opus", "mp3", "wv", "png", "jpg", "jpeg"];
/// Raw samples: a block of their own.
const SAMPLES: &[&str] = &["wav", "sf2", "dls"];
/// A file this big is a block of its own whatever it is.
const OWN_BLOCK: u64 = 16 << 20;
/// Up to this size a block of its own gets Brotli's strongest setting;
/// past it, one a little faster (the strongest takes minutes a gigabyte),
/// unless the player asked for the smallest backup (`smallest`).
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

pub fn extension(path: &str) -> String {
    Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// Whether a file is a block of its own.
pub fn own_block(item: &Item) -> bool {
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

/// Writes a backup of `items` and `settings` (a JSON object of strings),
/// each item's bytes read from what `open` gives for it; `smallest`, in
/// Brotli's strongest setting throughout, however long it takes.
#[allow(clippy::too_many_arguments)]
pub fn write_with<W: Write + Seek, S: Read>(
    out: &mut W,
    app: &str,
    made: u64,
    settings: &str,
    items: &[Item],
    smallest: bool,
    mut open: impl FnMut(&Item) -> io::Result<S>,
    mut progress: impl FnMut(Progress),
) -> Result<Manifest, String> {
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
            let mut file = open(item)?;
            record(w, &item.path, item.size, &mut file)?;
            done += item.size;
            progress(Progress { done, total });
        }
        Ok(())
    })
    .map_err(io)?;

    for item in own {
        let packed = PACKED.contains(&extension(&item.path).as_str());
        let (method, how) = if packed { (STORED, None) } else { (BROTLI, Some(params(if smallest || item.size <= STRONGEST_UP_TO { 11 } else { 9 }, item.size))) };
        block(out, method, how, |w| {
            let mut file = open(item)?;
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

pub const NOT_A_BACKUP: &str = "That isn't a Coupler Backup.";

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
pub fn safe(path: &str) -> bool {
    let mut parts = path.split('/');
    let first = parts.next().unwrap_or("");
    KEPT.contains(&first) && path.split('/').all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains(['\\', ':', '\0']))
}

pub const DAMAGED: &str = "The backup is damaged, so nothing was restored.";

/// Reads a backup through, checking every block: the settings (a JSON
/// object of strings) to `settings`, each file's path, size and bytes to
/// `file`, which must read them all. Anything wrong is an error, and
/// whatever was handed over before it is the caller's to throw away.
pub fn read<R: Read>(
    r: &mut R,
    mut settings: impl FnMut(String) -> Result<(), String>,
    mut file: impl FnMut(&str, u64, &mut dyn Read) -> Result<(), String>,
    mut progress: impl FnMut(Progress),
) -> Result<Manifest, String> {
    let manifest = inspect(r)?;
    let total = manifest.bytes;
    let mut done = 0u64;
    let mut files = 0usize;
    let mut had_settings = false;
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
                    settings(json)?;
                    had_settings = true;
                    continue;
                }
                if !safe(&path) {
                    return Err(DAMAGED.to_string());
                }
                let mut counted = CountingReader { inner: (&mut reader).take(size), read: 0 };
                file(&path, size, &mut counted)?;
                if counted.read != size {
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
    if !had_settings || files != manifest.files {
        return Err(DAMAGED.to_string());
    }
    Ok(manifest)
}

/// Counts what a file's reader took.
struct CountingReader<R: Read> {
    inner: R,
    read: u64,
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n as u64;
        Ok(n)
    }
}

/// Files in memory: each a path as the desktop keeps it in app data, and its bytes.
pub type Files = Vec<(String, Vec<u8>)>;

/// A backup's files and settings, read into memory, as the web build
/// restores one (it has no files to write them to).
#[allow(dead_code)] // The web build's.
pub fn read_all<R: Read>(r: &mut R) -> Result<(Manifest, String, Files), String> {
    let mut settings = String::new();
    let mut files = Vec::new();
    let manifest = read(
        r,
        |json| {
            settings = json;
            Ok(())
        },
        |path, size, from| {
            let mut bytes = Vec::with_capacity(size.min(1 << 24) as usize);
            from.read_to_end(&mut bytes).map_err(|_| DAMAGED.to_string())?;
            files.push((path.to_string(), bytes));
            Ok(())
        },
        |_| (),
    )?;
    Ok((manifest, settings, files))
}

/// A backup of files held in memory, as the web build makes one: each a
/// path as the desktop keeps it in app data, and its bytes. One block,
/// stored as it is: the web's files are small JSON, and Brotli's encoder
/// would weigh more in the page than it saves.
#[allow(dead_code)] // The web build's.
pub fn write_all(app: &str, made: u64, settings: &str, files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let parsed: BTreeMap<String, String> = serde_json::from_str(settings).map_err(|e| format!("The settings couldn't be read. ({e})"))?;
    let manifest = Manifest { format: FORMAT, app: app.to_string(), made, files: files.len(), bytes: files.iter().map(|(_, b)| b.len() as u64).sum(), settings: parsed.len() };
    let io = |e: io::Error| format!("Couldn't write the backup. ({e})");
    let mut body = Vec::new();
    record(&mut body, SETTINGS_RECORD, settings.len() as u64, &mut settings.as_bytes()).map_err(io)?;
    for (path, bytes) in files {
        record(&mut body, path, bytes.len() as u64, &mut bytes.as_slice()).map_err(io)?;
    }
    let mut crc = flate2::Crc::new();
    crc.update(&body);
    let json = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(body.len() + json.len() + 64);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    out.push(STORED);
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc.sum().to_le_bytes());
    out.push(END);
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn a_backup_in_memory_round_trips() {
        let files = vec![
            ("maps/coffeemud.json".to_string(), br#"{"rooms":[]}"#.to_vec()),
            ("web/journal.json".to_string(), b"{}".to_vec()),
            ("Assets/ogg/rain.ogg".to_string(), b"OggS small".to_vec()),
        ];
        let bytes = write_all("0.33.0", 7, r#"{"coupler.theme":"modern"}"#, &files).unwrap();
        let (manifest, settings, mut back) = read_all(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!((manifest.app.as_str(), manifest.files, manifest.settings), ("0.33.0", 3, 1));
        assert_eq!(settings, r#"{"coupler.theme":"modern"}"#);
        back.sort();
        let mut want = files.clone();
        want.sort();
        assert_eq!(back, want);
        let mut damaged = bytes.clone();
        let at = damaged.len() / 2;
        damaged[at] ^= 0x55;
        assert!(read_all(&mut Cursor::new(&damaged)).is_err());
    }

    #[test]
    fn a_compressed_backup_reads_in_memory() {
        let files = [("maps/coffeemud.json", br#"{"rooms":[]}"#.repeat(40))];
        let items: Vec<Item> = files.iter().map(|(p, b)| Item { path: p.to_string(), source: PathBuf::from(p), size: b.len() as u64 }).collect();
        let mut out = Cursor::new(Vec::new());
        write_with(&mut out, "0.33.0", 7, "{}", &items, false, |_| Ok(files[0].1.as_slice()), |_| ()).unwrap();
        let bytes = out.into_inner();
        assert!(bytes.len() < files[0].1.len(), "compressed");
        let (_, _, back) = read_all(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(back, vec![(files[0].0.to_string(), files[0].1.clone())]);
    }
}

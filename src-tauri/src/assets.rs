//! The Assets folder: the sounds, music and pictures a player gives
//! Coupler by dropping files on its window (or choosing them), for the
//! hooks' triggers to play and show (`hooks.rs`).
//!
//! It's `Assets` in app data (CLAUDE.md rule 8), made at launch. A file
//! is copied into a folder named for its type, in lower case: `wav/`,
//! `ogg/`, `mid/`, `png/`... A name already taken gets a number
//! (`door (2).wav`); the same file dropped twice is kept once. A file of
//! any other type is refused, with the reason.
//!
//! Three kinds, as the hooks list offers them:
//! - SFX, sound effects played once: `.wav`, `.ogg`, `.opus`, `.wv`.
//! - BGM, music played once or in a loop: `.mid`, `.midi`, `.mp3`, and the
//!   tracker modules `.mod`, `.xm`, `.s3m`, `.it`.
//! - ART, pictures: `.png`, `.jpg`, `.jpeg`, and ANSI art, `.ans` and
//!   `.asc` (classic CP437 art with SAUCE, or UTF-8 with the 256 and
//!   24-bit colors Coupler reads from the game; `ansi_art.rs` draws it).
//!
//! Plus `.sf2`, a SoundFont: MIDI is only notes, and a SoundFont is the
//! instruments that play them. Coupler ships none (a good one is tens of
//! megabytes): MIDI plays through Neumetik, Coupler's own synthesizer
//! (`vendor/neumetik`, GM and GS without samples), whatever SoundFonts there
//! are. The player can choose a SoundFont for one MIDI file (`set_font`,
//! kept in `.soundfonts.json`); if that SoundFont leaves the folder, the
//! file plays through Neumetik again. A SoundFont itself plays a sample
//! tune (`compose::sample`), to hear it before choosing it.
//!
//! **Made here.** `add_made` saves an asset Coupler made from the
//! player's words (`create.rs`, `compose.rs`) as any other, the take
//! counting up with the name's number so the same words make another.
//!
//! The WebView plays WAV and MP3 itself. The rest is rendered here to a
//! WAV it can play: Ogg Vorbis decoded (`lewton`), Opus and WavPack
//! (`codecs.rs`, libopus and libwavpack), MIDI through Neumetik or the
//! SoundFont chosen for it (`rustysynth`), modules through a tracker player (`xmrsplayer`). A
//! path from the frontend is only ever a type's folder and a file name
//! inside Assets; nothing else can be read.
//!
//! **Smaller sounds.** A WAV is raw samples, so as it comes in, Coupler
//! also makes it as Opus (lossy, far smaller) and as WavPack (lossless)
//! and keeps whichever is smallest, if smaller than the WAV, waiting in
//! `.smaller/` (`.smaller/wav/door.wav.opus`; no type is called that, so
//! it's never listed). The import says what it would save; the player
//! says yes (`compress`: the smaller file takes the WAV's place in
//! Assets and the WAV is deleted) or no (`keep`). What's left waiting is
//! cleared at launch. The file the player added from is never touched.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

use crate::codecs;

/// Rendered music is cut off here, whatever the file says.
const LONGEST_SECONDS: usize = 15 * 60;
/// A file bigger than this isn't taken.
const LARGEST_FILE: u64 = 256 * 1024 * 1024;
/// What Coupler renders at.
const RATE: u32 = 44_100;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Sfx,
    Bgm,
    Art,
    SoundFont,
}

/// What kind of asset a file type is, by its extension (any case).
pub fn kind_of(extension: &str) -> Option<Kind> {
    match extension.to_ascii_lowercase().as_str() {
        "wav" | "ogg" | "opus" | "wv" => Some(Kind::Sfx),
        "mid" | "midi" | "mp3" | "mod" | "xm" | "s3m" | "it" => Some(Kind::Bgm),
        "png" | "jpg" | "jpeg" | "ans" | "asc" => Some(Kind::Art),
        "sf2" => Some(Kind::SoundFont),
        _ => None,
    }
}

/// A file in the Assets folder.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Asset {
    /// Its folder and name, `wav/door.wav`: how a trigger names it.
    pub path: String,
    pub name: String,
    pub kind: Kind,
    /// How many hooks' triggers use it (filled in by the command, which
    /// has the hooks; 0 here).
    pub uses: usize,
}

/// What became of a file brought in.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum How {
    /// Copied in under its own name.
    New,
    /// Copied in under a new name: another file had its name.
    Renamed,
    /// Not copied: the same file was already there (under `asset.name`).
    Already,
}

/// A file an import took, and how.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Added {
    #[serde(flatten)]
    pub asset: Asset,
    /// Its name where it came from.
    pub from: String,
    pub how: How,
    /// A WAV that could be smaller, and how (waiting in `.smaller/`).
    pub smaller: Option<Smaller>,
}

/// A smaller copy of a WAV, made as it came in, for the player to take or not.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Smaller {
    /// Its type: `opus` (lossy) or `wv` (WavPack, lossless).
    pub to: String,
    /// The WAV's size and the smaller copy's, in bytes.
    pub before: u64,
    pub after: u64,
}

/// What an import did: the files now in Assets, and each file refused,
/// with why, in words.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Imported {
    pub added: Vec<Added>,
    pub skipped: Vec<String>,
}

/// Where an import is, sent as it goes (the frontend's progress bar and
/// status): the file it's on (from 1) of how many, that file's name,
/// what it's doing to it, and the bytes read so far of all the files.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub file: usize,
    pub files: usize,
    pub name: String,
    pub step: Step,
    pub done_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    /// Reading the file in.
    Reading,
    /// Looking for the same file already in its folder.
    Checking,
    /// Writing the copy.
    Saving,
    /// Making a WAV as Opus and as WavPack, to offer the smaller.
    Compressing,
}

/// Files are read this much at a time, so a big one moves the bar.
const CHUNK: usize = 1024 * 1024;

fn extension(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase)
}

/// A name in `dir` that isn't taken: `name`, else `stem (2).ext` and on.
fn free_name(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
    (2..).map(|n| format!("{stem} ({n}){ext}")).find(|n| !dir.join(n).exists()).expect("some number is free")
}

/// The file in `dir` with the same contents as `bytes`, if there is one.
fn same_file(dir: &Path, bytes: &[u8]) -> Option<String> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
        let len = entry.metadata().ok()?.len();
        (len == bytes.len() as u64 && std::fs::read(entry.path()).ok()? == bytes).then(|| entry.file_name().to_string_lossy().into_owned())
    })
}

/// Copies files into the Assets folder at `root`, each into its type's
/// folder, telling `progress` where it is as it goes.
pub fn import(root: &Path, files: &[PathBuf], mut progress: impl FnMut(Progress)) -> Imported {
    let mut done = Imported::default();
    // A WAV counts twice on the bar: read, then compressed.
    let sizes: Vec<u64> = files
        .iter()
        .map(|f| {
            let len = std::fs::metadata(f).map(|m| if m.is_file() { m.len() } else { 0 }).unwrap_or(0);
            if extension(f).as_deref() == Some("wav") { len * 2 } else { len }
        })
        .collect();
    let total_bytes = sizes.iter().sum();
    let mut before = 0;
    for (i, file) in files.iter().enumerate() {
        let shown = file.file_name().map_or_else(|| file.display().to_string(), |n| n.to_string_lossy().into_owned());
        let mut tell = |step, read| {
            progress(Progress { file: i + 1, files: files.len(), name: shown.clone(), step, done_bytes: before + read, total_bytes });
        };
        match import_one(root, file, &mut tell) {
            Ok((asset, how, smaller)) => done.added.push(Added { asset, from: shown.clone(), how, smaller }),
            Err(why) => done.skipped.push(format!("{shown}: {why}")),
        }
        before += sizes[i];
    }
    done
}

/// Reads a file in chunks, telling `tell` how much is read.
fn read_counted(file: &Path, len: u64, tell: &mut impl FnMut(Step, u64)) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut from = std::fs::File::open(file)?;
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    let mut chunk = vec![0; CHUNK];
    loop {
        let n = from.read(&mut chunk)?;
        if n == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..n]);
        tell(Step::Reading, (bytes.len() as u64).min(len));
    }
}

fn import_one(root: &Path, file: &Path, tell: &mut impl FnMut(Step, u64)) -> Result<(Asset, How, Option<Smaller>), String> {
    let meta = std::fs::metadata(file).map_err(|_| "it couldn't be read.".to_string())?;
    if meta.is_dir() {
        return Err("it's a folder. Drop the files in it instead.".into());
    }
    let ext = extension(file).ok_or("it has no type (no .wav, .png or the like on its name).")?;
    let kind = kind_of(&ext)
        .ok_or_else(|| format!(".{ext} isn't a type Coupler uses. It takes .wav, .ogg, .opus and .wv sounds; .mid, .mp3, .mod, .xm, .s3m and .it music; .png and .jpg pictures and .ans and .asc ANSI art; and .sf2 SoundFonts."))?;
    if meta.len() > LARGEST_FILE {
        return Err("it's bigger than 256 MB.".into());
    }
    let name = file.file_name().and_then(|n| n.to_str()).ok_or("its name can't be used.")?;
    let dir = root.join(&ext);
    std::fs::create_dir_all(&dir).map_err(|e| format!("the {ext} folder couldn't be made. ({e})"))?;
    tell(Step::Reading, 0);
    let bytes = read_counted(file, meta.len(), tell).map_err(|_| "it couldn't be read.".to_string())?;
    let read = meta.len();
    tell(Step::Checking, read);
    let (kept, how) = match same_file(&dir, &bytes) {
        Some(already) => (already, How::Already),
        None => {
            let free = free_name(&dir, name);
            tell(Step::Saving, read);
            std::fs::write(dir.join(&free), &bytes).map_err(|e| format!("it couldn't be saved. ({e})"))?;
            let how = if free == name { How::New } else { How::Renamed };
            (free, how)
        }
    };
    let path = format!("{ext}/{kept}");
    // Only a WAV just added: one already there may be in use.
    let smaller = if ext == "wav" && how != How::Already {
        try_smaller(root, &path, &bytes, &mut |part| tell(Step::Compressing, read + (part * read as f32) as u64))
    } else {
        None
    };
    Ok((Asset { path, name: kept, kind, uses: 0 }, how, smaller))
}

/// Where smaller copies wait for the player's answer.
const WAITING: &str = ".smaller";

fn waiting(root: &Path, path: &str, to: &str) -> PathBuf {
    root.join(WAITING).join(format!("{path}.{to}"))
}

/// Makes the WAV at `path` as Opus and as WavPack, and keeps the smallest
/// waiting in `.smaller/` if it's smaller than the WAV. Nothing if the
/// WAV can't be read (an ADPCM one, say): it's kept as it is.
///
/// WavPack's smallest settings are slow (`codecs.rs`), so it's made
/// quickly first, and made again at its smallest only if that could
/// beat Opus: they save at most this share more.
fn try_smaller(root: &Path, path: &str, bytes: &[u8], tell: &mut impl FnMut(f32)) -> Option<Smaller> {
    const THOROUGH_SAVES: f64 = 0.15;
    let pcm = codecs::read_wav(bytes).ok()?;
    // The bar: Opus, then the quick WavPack, then the thorough one if it's needed.
    let opus = (pcm.channels <= 2).then(|| codecs::encode_opus(&pcm, &mut |f| tell(f * 0.1)).ok()).flatten();
    let wavpack = codecs::wavpack_exact(&pcm);
    let mut wv = wavpack.then(|| codecs::encode_wavpack(&pcm, false, &mut |f| tell(0.1 + f * 0.1)).ok()).flatten();
    if let Some(quick) = &wv {
        if opus.as_ref().is_none_or(|o| (quick.len() as f64 * (1.0 - THOROUGH_SAVES)) < o.len() as f64) {
            if let Ok(thorough) = codecs::encode_wavpack(&pcm, true, &mut |f| tell(0.2 + f * 0.8)) {
                wv = Some(thorough);
            }
        }
    }
    let made = [opus.map(|b| ("opus", b)), wv.map(|b| ("wv", b))];
    let (to, smallest) = made.into_iter().flatten().min_by_key(|(_, b)| b.len())?;
    if smallest.len() >= bytes.len() {
        return None;
    }
    let file = waiting(root, path, to);
    std::fs::create_dir_all(file.parent()?).ok()?;
    std::fs::write(&file, &smallest).ok()?;
    Some(Smaller { to: to.into(), before: bytes.len() as u64, after: smallest.len() as u64 })
}

/// A WAV that became its smaller copy.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Shrunk {
    /// The WAV's asset path, now gone.
    pub from: String,
    #[serde(flatten)]
    pub asset: Asset,
    pub before: u64,
    pub after: u64,
}

/// What `compress` did: the WAVs replaced, and each one not, with why.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Compressed {
    pub done: Vec<Shrunk>,
    pub skipped: Vec<String>,
}

/// Puts each WAV's smaller copy in Assets in its place and deletes the
/// WAV (Coupler's copy; the file it came from is never touched). One a
/// hook now uses (`in_use`) is kept as it is.
pub fn compress(root: &Path, paths: &[String], in_use: impl Fn(&str) -> bool) -> Compressed {
    let mut done = Compressed::default();
    for path in paths {
        match compress_one(root, path, &in_use) {
            Ok(shrunk) => done.done.push(shrunk),
            Err(why) => done.skipped.push(format!("{}: {why}", folder_name(path))),
        }
    }
    keep(root, paths);
    done
}

fn folder_name(path: &str) -> &str {
    path.split_once('/').map_or(path, |(_, name)| name)
}

fn compress_one(root: &Path, path: &str, in_use: &impl Fn(&str) -> bool) -> Result<Shrunk, String> {
    let wav = resolve(root, path)?;
    if folder_of(path) != "wav" {
        return Err("it isn't a WAV.".into());
    }
    let (to, from) = ["opus", "wv"].iter().map(|to| (*to, waiting(root, path, to))).find(|(_, f)| f.is_file()).ok_or("its smaller copy isn't there any more. Add it again to make one.")?;
    if in_use(path) {
        return Err("a hook uses it now, so it was kept as it is.".into());
    }
    let before = std::fs::metadata(&wav).map_err(|e| format!("it couldn't be read. ({e})"))?.len();
    let after = std::fs::metadata(&from).map_err(|e| format!("its smaller copy couldn't be read. ({e})"))?.len();
    let dir = root.join(to);
    std::fs::create_dir_all(&dir).map_err(|e| format!("the {to} folder couldn't be made. ({e})"))?;
    let stem = Path::new(folder_name(path)).file_stem().and_then(|s| s.to_str()).unwrap_or("sound");
    let name = free_name(&dir, &format!("{stem}.{to}"));
    std::fs::rename(&from, dir.join(&name)).map_err(|e| format!("its smaller copy couldn't be put in place. ({e})"))?;
    std::fs::remove_file(&wav).map_err(|e| format!("the smaller copy was added, but the WAV couldn't be deleted. ({e})"))?;
    Ok(Shrunk { from: path.into(), asset: Asset { path: format!("{to}/{name}"), name, kind: Kind::Sfx, uses: 0 }, before, after })
}

/// Forgets the smaller copies waiting for these WAVs: they stay as they are.
pub fn keep(root: &Path, paths: &[String]) {
    for path in paths {
        if is_asset_path(path) && folder_of(path) == "wav" {
            for to in ["opus", "wv"] {
                let _ = std::fs::remove_file(waiting(root, path, to));
            }
        }
    }
}

/// Forgets every smaller copy left waiting (at launch: the question that
/// made them is gone).
pub fn forget_waiting(root: &Path) {
    let _ = std::fs::remove_dir_all(root.join(WAITING));
}

/// Every asset, sorted by folder and name. SoundFonts included.
pub fn list(root: &Path) -> Vec<Asset> {
    let mut all = Vec::new();
    let Ok(folders) = std::fs::read_dir(root) else { return all };
    for folder in folders.flatten() {
        let ext = folder.file_name().to_string_lossy().into_owned();
        let Some(kind) = kind_of(&ext) else { continue };
        let Ok(files) = std::fs::read_dir(folder.path()) else { continue };
        for file in files.flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            if file.path().is_file() && extension(&file.path()).is_some_and(|e| e == ext) {
                all.push(Asset { path: format!("{ext}/{name}"), name, kind, uses: 0 });
            }
        }
    }
    all.sort_by_key(|a| a.path.to_lowercase());
    all
}

/// The file an asset path names: a type's folder, then a file in it,
/// and nothing that could reach outside the Assets folder.
pub fn resolve(root: &Path, path: &str) -> Result<PathBuf, String> {
    if !is_asset_path(path) {
        return Err(format!("{path} isn't an asset."));
    }
    let file = root.join(path);
    if !file.is_file() {
        return Err(format!("{path} isn't in the Assets folder any more."));
    }
    Ok(file)
}

/// Whether `path` is a type's folder and a file of that type in it, and
/// nothing more.
fn is_asset_path(path: &str) -> bool {
    let rel = Path::new(path);
    let parts: Vec<_> = rel.components().collect();
    let (Some(Component::Normal(folder)), Some(Component::Normal(_)), 2) = (parts.first(), parts.get(1), parts.len()) else {
        return false;
    };
    let folder = folder.to_str().unwrap_or_default();
    kind_of(folder).is_some() && extension(rel).as_deref() == Some(folder)
}

/// Whether a picture is ANSI art, drawn in characters, rather than an image.
pub fn is_ansi(path: &str) -> bool {
    matches!(folder_of(path), "ans" | "asc")
}

/// An image's bytes (PNG or JPEG), as the file has them.
pub fn picture(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let file = resolve(root, path)?;
    if kind_of(folder_of(path)) != Some(Kind::Art) || is_ansi(path) {
        return Err(format!("{path} isn't an image."));
    }
    std::fs::read(file).map_err(|e| format!("{path} couldn't be read. ({e})"))
}

/// ANSI art, drawn into styled rows of characters.
pub fn ansi(root: &Path, path: &str) -> Result<crate::ansi_art::Art, String> {
    let file = resolve(root, path)?;
    if !is_ansi(path) {
        return Err(format!("{path} isn't ANSI art."));
    }
    let bytes = std::fs::read(file).map_err(|e| format!("{path} couldn't be read. ({e})"))?;
    Ok(crate::ansi_art::render(&bytes))
}

fn folder_of(path: &str) -> &str {
    path.split('/').next().unwrap_or_default()
}

/// A sound or piece of music, as something the WebView can play: WAV
/// and MP3 as they are, everything else rendered to a WAV. `looping`
/// music is rendered without the silence a MIDI file lets its last
/// notes ring out in, so the loop doesn't pause. A SoundFont plays its
/// sample tune.
pub fn audio(root: &Path, path: &str, looping: bool) -> Result<Vec<u8>, String> {
    let file = resolve(root, path)?;
    let bytes = std::fs::read(&file).map_err(|e| format!("{path} couldn't be read. ({e})"))?;
    match folder_of(path) {
        "wav" | "mp3" => Ok(bytes),
        "ogg" => ogg_to_wav(&bytes).map_err(|e| format!("{path} couldn't be played. ({e})")),
        "opus" | "wv" => {
            let decode = if folder_of(path) == "opus" { codecs::decode_opus } else { codecs::decode_wavpack };
            let (channels, rate, samples) = decode(&bytes, LONGEST_SECONDS).map_err(|e| format!("{path} couldn't be played. ({e})"))?;
            Ok(wav(channels, rate, &codecs::to_i16(&samples)))
        }
        "mid" | "midi" => match font_for(root, path) {
            Some(font) => midi_to_wav(&bytes, &font, looping),
            None => neumetik_to_wav(&bytes, looping),
        }
        .map_err(|e| format!("{path} couldn't be played. ({e})")),
        "mod" | "xm" | "s3m" | "it" => module_to_wav(&bytes).map_err(|e| format!("{path} couldn't be played. ({e})")),
        "sf2" => midi_to_wav(&crate::compose::sample(), &file, false).map_err(|e| format!("{path} couldn't be played. ({e})")),
        _ => Err(format!("{path} isn't a sound.")),
    }
}

/// MIDI made elsewhere (the Music Editor's) played through Neumetik, or
/// through `font` (a SoundFont asset) when the player chose one there.
pub fn midi_audio(root: &Path, font: Option<&str>, bytes: &[u8], looping: bool) -> Result<Vec<u8>, String> {
    match font {
        Some(font) => {
            if folder_of(font) != "sf2" {
                return Err(format!("{font} isn't a SoundFont."));
            }
            midi_to_wav(bytes, &resolve(root, font)?, looping)
        }
        None => neumetik_to_wav(bytes, looping),
    }
}

/// Writes over a MIDI file in the Assets folder (the Music Editor's
/// Save), through a file beside it renamed into place.
pub fn replace_midi(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let file = resolve(root, path)?;
    if !is_midi(path) {
        return Err(format!("{path} isn't MIDI music."));
    }
    let part = file.with_extension("saving");
    std::fs::write(&part, bytes).and_then(|_| std::fs::rename(&part, &file)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        format!("{path} couldn't be saved. ({e})")
    })
}

// ---- Which SoundFont plays a MIDI file ----

/// Where the choices are kept, in the Assets folder (no type is called
/// that, so it's never listed or named by a trigger).
const FONTS: &str = ".soundfonts.json";

fn is_midi(path: &str) -> bool {
    matches!(folder_of(path), "mid" | "midi")
}

/// Each MIDI file the player chose a SoundFont for, and the SoundFont
/// (both asset paths). The SoundFont may have left the folder since.
pub fn fonts(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(root.join(FONTS)).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default()
}

/// Chooses the SoundFont a MIDI file plays through, or Neumetik (None).
pub fn set_font(root: &Path, path: &str, font: Option<&str>) -> Result<(), String> {
    resolve(root, path)?;
    if !is_midi(path) {
        return Err(format!("{path} isn't MIDI music."));
    }
    let mut all = fonts(root);
    match font {
        Some(font) => {
            resolve(root, font)?;
            if folder_of(font) != "sf2" {
                return Err(format!("{font} isn't a SoundFont."));
            }
            all.insert(path.into(), font.into());
        }
        None => {
            all.remove(path);
        }
    }
    // Choices for files that have gone aren't kept.
    all.retain(|midi, _| resolve(root, midi).is_ok());
    let json = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(root.join(FONTS), json).map_err(|e| format!("The choice couldn't be saved. ({e})"))
}

/// The SoundFont a MIDI file plays through: the one chosen for it, if
/// it's still in the folder; else none, and Neumetik plays it.
fn font_for(root: &Path, path: &str) -> Option<PathBuf> {
    let font = fonts(root).remove(path)?;
    (folder_of(&font) == "sf2").then(|| resolve(root, &font).ok()).flatten()
}

// ---- Made from the player's words ----

/// Saves an asset Coupler made into its type's folder, as `stem.ext` or,
/// that being taken, `stem (2).ext` and on. `make` is given the take (0
/// for the first, 1 for `(2)`...) and returns the file's bytes.
pub fn add_made(root: &Path, ext: &str, stem: &str, make: impl FnOnce(u32) -> Vec<u8>) -> Result<Asset, String> {
    let kind = kind_of(ext).ok_or_else(|| format!(".{ext} isn't a type Coupler uses."))?;
    let dir = root.join(ext);
    std::fs::create_dir_all(&dir).map_err(|e| format!("the {ext} folder couldn't be made. ({e})"))?;
    let name = free_name(&dir, &format!("{stem}.{ext}"));
    let take = name
        .strip_suffix(&format!(".{ext}"))
        .and_then(|n| n.strip_prefix(stem))
        .and_then(|n| n.strip_prefix(" ("))
        .and_then(|n| n.strip_suffix(')'))
        .and_then(|n| n.parse::<u32>().ok())
        .map_or(0, |n| n - 1);
    let path = format!("{ext}/{name}");
    if !is_asset_path(&path) {
        return Err(format!("{name} can't be used as a name."));
    }
    std::fs::write(dir.join(&name), make(take)).map_err(|e| format!("it couldn't be saved. ({e})"))?;
    Ok(Asset { path, name, kind, uses: 0 })
}

/// 16-bit PCM samples, interleaved, as a WAV file.
pub fn wav(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn ogg_to_wav(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let channels = u16::from(reader.ident_hdr.audio_channels);
    let rate = reader.ident_hdr.audio_sample_rate;
    let most = LONGEST_SECONDS * rate as usize * usize::from(channels);
    let mut samples = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl().map_err(|e| e.to_string())? {
        samples.extend(packet);
        if samples.len() >= most {
            samples.truncate(most);
            break;
        }
    }
    Ok(wav(channels, rate, &samples))
}

fn midi_to_wav(bytes: &[u8], font: &Path, looping: bool) -> Result<Vec<u8>, String> {
    use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
    let font = std::fs::read(font).map_err(|e| format!("the SoundFont couldn't be read: {e}"))?;
    let font = Arc::new(SoundFont::new(&mut Cursor::new(font)).map_err(|e| format!("the SoundFont couldn't be used: {e:?}"))?);
    let midi = Arc::new(MidiFile::new(&mut Cursor::new(bytes)).map_err(|e| format!("{e:?}"))?);
    let synth = Synthesizer::new(&font, &SynthesizerSettings::new(RATE as i32)).map_err(|e| format!("{e:?}"))?;
    let mut sequencer = MidiFileSequencer::new(synth);
    sequencer.play(&midi, false);
    // Once, the last notes ring out for two seconds; in a loop, the next
    // time round starts right away.
    let tail = if looping { 0.0 } else { 2.0 };
    let seconds = (midi.get_length() + tail).min(LONGEST_SECONDS as f64);
    let frames = (seconds * f64::from(RATE)) as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    let (mut left, mut right) = (vec![0f32; 4096], vec![0f32; 4096]);
    let mut done = 0;
    while done < frames {
        let n = (frames - done).min(4096);
        sequencer.render(&mut left[..n], &mut right[..n]);
        for i in 0..n {
            samples.push(left[i]);
            samples.push(right[i]);
        }
        done += n;
    }
    Ok(wav(2, RATE, &louder(&samples)))
}

/// MIDI through Neumetik, Coupler's own synthesizer: no SoundFont needed.
fn neumetik_to_wav(bytes: &[u8], looping: bool) -> Result<Vec<u8>, String> {
    let tail = if looping { 0.0 } else { 2.0 };
    let samples = neumetik::render(bytes, RATE, tail, LONGEST_SECONDS as f64)?;
    Ok(wav(2, RATE, &louder(&samples)))
}

/// Rendered music at a proper level: brought up (or down) so its loudest
/// moment is just under full scale. A synthesizer plays at whatever level
/// its SoundFont or module was made at, often far too quiet to hear
/// beside sound effects; the Mixer and each trigger's volume then work
/// from the same starting point for every piece. Silence stays silent,
/// and the gain is capped so a nearly silent file isn't blown up into hiss.
pub(crate) fn louder(samples: &[f32]) -> Vec<i16> {
    const TARGET: f32 = 0.89; // about -1 dB
    const MOST_GAIN: f32 = 32.0;
    let peak = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
    let gain = if peak > 0.0 { (TARGET / peak).min(MOST_GAIN) } else { 1.0 };
    samples.iter().map(|s| ((s * gain).clamp(-1.0, 1.0) * 32767.0) as i16).collect()
}

fn module_to_wav(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let module = xmrs::prelude::Module::load(bytes).map_err(|e| format!("{e:?}"))?;
    let mut player = xmrsplayer::prelude::XmrsPlayer::new(&module, RATE, 0);
    // Through once: a module's own loop would otherwise play forever.
    // Coupler loops the whole of it when the trigger asks.
    player.set_max_loop_count(1);
    let samples: Vec<f32> = player.take(LONGEST_SECONDS * RATE as usize * 2).map(|s| f32::from(s) / 32768.0).collect();
    if samples.is_empty() {
        return Err("it has no music in it".into());
    }
    Ok(wav(2, RATE, &louder(&samples)))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("coupler-assets-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn types_sort_into_the_three_kinds() {
        for sound in ["WAV", "ogg", "opus", "wv"] {
            assert_eq!(kind_of(sound), Some(Kind::Sfx), "{sound}");
        }
        for music in ["mid", "midi", "mp3", "mod", "xm", "s3m", "it"] {
            assert_eq!(kind_of(music), Some(Kind::Bgm), "{music}");
        }
        assert_eq!(kind_of("Jpeg"), Some(Kind::Art));
        assert_eq!(kind_of("ANS"), Some(Kind::Art));
        assert_eq!(kind_of("asc"), Some(Kind::Art));
        assert_eq!(kind_of("sf2"), Some(Kind::SoundFont));
        assert_eq!(kind_of("txt"), None);
        assert_eq!(kind_of("gif"), None);
    }

    #[test]
    fn dropped_files_go_into_a_folder_for_their_type() {
        let (from, root) = (TempDir::new("from"), TempDir::new("root"));
        std::fs::write(from.0.join("Door.WAV"), b"one").unwrap();
        std::fs::write(from.0.join("gate.png"), b"picture").unwrap();
        std::fs::write(from.0.join("notes.txt"), b"words").unwrap();
        let mut told = Vec::new();
        let done = import(&root.0, &[from.0.join("Door.WAV"), from.0.join("gate.png"), from.0.join("notes.txt"), from.0.clone()], |p| told.push(p));
        assert_eq!(
            done.added.iter().map(|a| a.asset.clone()).collect::<Vec<_>>(),
            vec![
                Asset { path: "wav/Door.WAV".into(), name: "Door.WAV".into(), kind: Kind::Sfx, uses: 0 },
                Asset { path: "png/gate.png".into(), name: "gate.png".into(), kind: Kind::Art, uses: 0 },
            ]
        );
        assert!(done.added.iter().all(|a| a.how == How::New));
        // Each file read, checked and saved, the bytes counting up to all of them.
        let last = told.last().unwrap();
        assert_eq!((last.file, last.files, last.name.as_str(), last.step), (2, 4, "gate.png", Step::Saving));
        // The WAV counts twice (read, then compressed); "one" isn't a real
        // WAV, so it's just kept, with nothing offered.
        assert_eq!((last.done_bytes, last.total_bytes), (13, 18));
        assert!(done.added.iter().all(|a| a.smaller.is_none()));
        assert!(told.windows(2).all(|w| w[0].done_bytes <= w[1].done_bytes));
        assert_eq!(done.skipped.len(), 2);
        assert!(done.skipped[0].starts_with("notes.txt: .txt isn't a type Coupler uses."));
        assert_eq!(std::fs::read(root.0.join("wav/Door.WAV")).unwrap(), b"one");
        assert_eq!(list(&root.0).len(), 2);
    }

    #[test]
    fn a_taken_name_gets_a_number_and_the_same_file_is_kept_once() {
        let (a, b, root) = (TempDir::new("a"), TempDir::new("b"), TempDir::new("taken"));
        std::fs::write(a.0.join("door.wav"), b"one").unwrap();
        std::fs::write(b.0.join("door.wav"), b"two").unwrap();
        import(&root.0, &[a.0.join("door.wav")], |_| ());
        let second = import(&root.0, &[b.0.join("door.wav")], |_| ());
        assert_eq!(second.added[0].asset.path, "wav/door (2).wav");
        assert_eq!(second.added[0].how, How::Renamed);
        let again = import(&root.0, &[a.0.join("door.wav")], |_| ());
        assert_eq!(again.added[0].how, How::Already);
        assert_eq!(again.added[0].asset.path, "wav/door.wav");
        assert_eq!(list(&root.0).len(), 2);
    }

    #[test]
    fn only_a_file_in_a_types_folder_can_be_named() {
        let root = TempDir::new("resolve");
        std::fs::create_dir_all(root.0.join("wav")).unwrap();
        std::fs::write(root.0.join("wav/door.wav"), b"one").unwrap();
        assert!(resolve(&root.0, "wav/door.wav").is_ok());
        for bad in ["../wav/door.wav", "wav/../wav/door.wav", "/etc/hosts", "wav", "txt/a.txt", "wav/missing.wav", "png/door.wav"] {
            assert!(resolve(&root.0, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_wav_header_says_what_follows() {
        let file = wav(2, 44_100, &[1, -1, 2, -2]);
        assert_eq!(&file[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!(u16::from_le_bytes(file[22..24].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(file[40..44].try_into().unwrap()), 8);
        assert_eq!(file.len(), 44 + 8);
    }

    #[test]
    fn quiet_music_is_brought_up_and_silence_stays_silent() {
        let quiet = louder(&[0.05, -0.1, 0.02]);
        assert_eq!(quiet[1], (-0.89f32 * 32767.0) as i16);
        assert_eq!(louder(&[0.0, 0.0]), vec![0, 0]);
        // A whisper isn't blown up past the cap.
        assert_eq!(louder(&[0.001])[0], (0.032f32 * 32767.0) as i16);
    }

    /// Half a second of noise-free tone as a 16-bit mono WAV file.
    fn tone_wav() -> Vec<u8> {
        let samples: Vec<i16> = (0..22_050).map(|i| ((i as f32 / 44_100.0 * 330.0 * std::f32::consts::TAU).sin() * 12_000.0) as i16).collect();
        wav(1, 44_100, &samples)
    }

    #[test]
    fn a_wav_coming_in_is_offered_smaller_and_replaced_only_when_taken() {
        let (from, root) = (TempDir::new("shrink-from"), TempDir::new("shrink"));
        std::fs::write(from.0.join("bell.wav"), tone_wav()).unwrap();
        let mut horn = tone_wav();
        horn[100] ^= 1;
        std::fs::write(from.0.join("horn.wav"), horn).unwrap();
        let mut steps = Vec::new();
        let done = import(&root.0, &[from.0.join("bell.wav"), from.0.join("horn.wav")], |p| steps.push(p.step));
        assert!(steps.contains(&Step::Compressing));
        let bell = done.added[0].smaller.clone().unwrap();
        // Opus wins: far smaller than lossless could be.
        assert_eq!(bell.to, "opus");
        assert_eq!(bell.before, tone_wav().len() as u64);
        assert!(bell.after < bell.before / 5, "{bell:?}");
        // Nothing listed until the player says yes.
        assert_eq!(list(&root.0).len(), 2);

        // Yes for the bell (the horn now used by a hook, so kept); no for
        // a WAV with nothing waiting.
        let paths = vec!["wav/bell.wav".to_string(), "wav/horn.wav".to_string(), "wav/gone.wav".to_string()];
        let shrunk = compress(&root.0, &paths, |p| p == "wav/horn.wav");
        assert_eq!(shrunk.done.len(), 1);
        assert_eq!(shrunk.done[0].from, "wav/bell.wav");
        assert_eq!(shrunk.done[0].asset.path, "opus/bell.opus");
        assert_eq!(shrunk.done[0].after, bell.after);
        assert_eq!(shrunk.skipped.len(), 2);
        assert!(shrunk.skipped[0].starts_with("horn.wav: a hook uses it"));
        assert!(shrunk.skipped[1].starts_with("gone.wav: "));
        let now: Vec<_> = list(&root.0).into_iter().map(|a| a.path).collect();
        assert_eq!(now, vec!["opus/bell.opus", "wav/horn.wav"]);
        // The file it came from is untouched; what was waiting is gone.
        assert!(from.0.join("bell.wav").is_file());
        assert!(!waiting(&root.0, "wav/horn.wav", "opus").exists());
        // And it plays, as the same half second at 48 kHz.
        let played = audio(&root.0, "opus/bell.opus", false).unwrap();
        assert_eq!(u32::from_le_bytes(played[24..28].try_into().unwrap()), 48_000);
        assert!(((played.len() - 44) as i64 / 2 - 24_000).abs() <= 2);
    }

    #[test]
    fn saying_no_forgets_the_smaller_copy_and_nothing_else() {
        let (from, root) = (TempDir::new("keep-from"), TempDir::new("keep"));
        std::fs::write(from.0.join("bell.wav"), tone_wav()).unwrap();
        import(&root.0, &[from.0.join("bell.wav")], |_| ());
        assert!(waiting(&root.0, "wav/bell.wav", "opus").is_file());
        // A path that would reach outside is ignored.
        std::fs::write(root.0.join("outside.opus"), b"keep me").unwrap();
        keep(&root.0, &["wav/bell.wav".into(), "../outside".into(), "wav/../../outside".into()]);
        assert!(!waiting(&root.0, "wav/bell.wav", "opus").exists());
        assert!(root.0.join("outside.opus").is_file());
        assert_eq!(list(&root.0)[0].path, "wav/bell.wav");
        assert!(compress(&root.0, &["wav/bell.wav".into()], |_| false).skipped[0].contains("isn't there any more"));
        forget_waiting(&root.0);
        assert!(!root.0.join(WAITING).exists());
    }

    #[test]
    fn a_wavpack_asset_plays() {
        let root = TempDir::new("wv");
        std::fs::create_dir_all(root.0.join("wv")).unwrap();
        let pcm = codecs::read_wav(&tone_wav()).unwrap();
        std::fs::write(root.0.join("wv/bell.wv"), codecs::encode_wavpack(&pcm, false, &mut |_| ()).unwrap()).unwrap();
        // Lossless: the very WAV it was made from.
        assert_eq!(audio(&root.0, "wv/bell.wv", false).unwrap(), tone_wav());
    }

    #[test]
    fn a_soundfont_is_chosen_per_file_and_neumetik_plays_without_it() {
        let root = TempDir::new("fonts");
        std::fs::create_dir_all(root.0.join("mid")).unwrap();
        std::fs::create_dir_all(root.0.join("sf2")).unwrap();
        std::fs::write(root.0.join("mid/town.mid"), crate::compose::sample()).unwrap();
        // Not a real SoundFont: only chosen, it's used and fails.
        std::fs::write(root.0.join("sf2/broken.sf2"), b"RIFF").unwrap();
        assert!(audio(&root.0, "mid/town.mid", true).is_ok());
        assert!(set_font(&root.0, "mid/town.mid", Some("sf2/missing.sf2")).is_err());
        assert!(set_font(&root.0, "sf2/broken.sf2", Some("sf2/broken.sf2")).is_err());
        assert!(set_font(&root.0, "mid/town.mid", Some("mid/town.mid")).is_err());
        set_font(&root.0, "mid/town.mid", Some("sf2/broken.sf2")).unwrap();
        assert_eq!(fonts(&root.0).get("mid/town.mid").map(String::as_str), Some("sf2/broken.sf2"));
        assert!(audio(&root.0, "mid/town.mid", true).unwrap_err().contains("SoundFont"));
        // The SoundFont gone: Neumetik again.
        std::fs::remove_file(root.0.join("sf2/broken.sf2")).unwrap();
        assert!(audio(&root.0, "mid/town.mid", true).is_ok());
        set_font(&root.0, "mid/town.mid", None).unwrap();
        assert!(fonts(&root.0).is_empty());
        // The choices' file isn't an asset.
        assert!(list(&root.0).iter().all(|a| !a.path.contains("soundfonts")));
        assert!(resolve(&root.0, FONTS).is_err());
    }

    #[test]
    fn a_made_asset_takes_a_free_name_and_counts_its_takes() {
        let root = TempDir::new("made");
        let mut takes = Vec::new();
        for _ in 0..3 {
            let asset = add_made(&root.0, "mid", "the sea", |take| {
                takes.push(take);
                vec![take as u8]
            })
            .unwrap();
            assert_eq!(asset.kind, Kind::Bgm);
        }
        assert_eq!(takes, vec![0, 1, 2]);
        let names: Vec<_> = list(&root.0).into_iter().map(|a| a.path).collect();
        assert_eq!(names, vec!["mid/the sea (2).mid", "mid/the sea (3).mid", "mid/the sea.mid"]);
        assert!(add_made(&root.0, "txt", "x", |_| Vec::new()).is_err());
    }

    #[test]
    fn midi_without_a_soundfont_plays_through_neumetik() {
        let root = TempDir::new("midi");
        std::fs::create_dir_all(root.0.join("mid")).unwrap();
        std::fs::write(root.0.join("mid/broken.mid"), b"MThd").unwrap();
        assert!(audio(&root.0, "mid/broken.mid", false).unwrap_err().contains("couldn't be played"));
        // Middle C on a piano for a quarter note, at 120 bpm.
        let mut town = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk\0\0\0\x0c".to_vec();
        town.extend_from_slice(&[0, 0x90, 60, 100, 0x60, 0x80, 60, 0, 0, 0xff, 0x2f, 0]);
        std::fs::write(root.0.join("mid/town.mid"), town).unwrap();
        let played = codecs::read_wav(&audio(&root.0, "mid/town.mid", true).unwrap()).unwrap();
        assert_eq!((played.channels, played.rate), (2, RATE));
        // Half a second, looping: no tail.
        let codecs::Samples::Int { data, .. } = played.samples else { panic!("not 16-bit") };
        assert_eq!(data.len(), 44_100);
    }
}

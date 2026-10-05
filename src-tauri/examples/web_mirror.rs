//! **The web mirror**: the desktop Coupler's hooks, and the assets they
//! use, made into the files the web build (`src-web/`, coupler.ansiapps.com)
//! plays them from. For the maintainer only, run on the Mac whose Coupler
//! has the hooks to publish; `scripts/web-publish.sh` runs it before each
//! deploy, and anyone can read what it does here. Never part of the app.
//!
//! ```sh
//! cargo run --release --manifest-path src-tauri/Cargo.toml --example web_mirror -- \
//!     [--from <Coupler's app-data folder>] [--to web-mirror] [--allow-private]
//! ```
//!
//! It reads a **copy** of `hooks.sqlite` (the app may be running), takes
//! only the `triggers` table (never the recorded pairs: those hold other
//! players' names and chat), and writes into `--to` (`web-mirror/`,
//! gitignored), replacing whatever was there:
//!
//! - `hooks.json`: every trigger, by its pair, as the desktop has it,
//!   and `files`, each asset path the triggers name and the file the web
//!   build fetches for it.
//! - One file per asset, named by a hash of its contents so a deploy
//!   can cache it forever: sounds and music as **Ogg Opus** (rendered
//!   first the way the desktop plays them: Ogg, WavPack, MIDI through the
//!   first SoundFont, tracker modules), MP3 as it is; PNG and JPEG as
//!   they are; ANSI art drawn here (`ansi_art.rs`) into the JSON the
//!   view shows, so the web build carries no ANSI art reader. MIDI is
//!   rendered twice when a trigger loops it and another doesn't (`#loop`).
//!
//! A trigger on a pair that may name a person or hold what someone said
//! (`PRIVATE_KEYS`) is left out unless `--allow-private` is given: the
//! mirror is public. Everything it publishes is listed as it goes.
//!
//! Assets published here are served to anyone who opens the web build:
//! publish only what you may share.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use coupler_lib::{assets, codecs, hooks::Hooks, trigger::Trigger};
use serde_json::json;

/// Keys whose values can be a person's name or their words.
const PRIVATE_KEYS: &[&str] = &["comm.", "char.base.name", "char.base.perlevel", "room.players", "group."];

struct Args {
    from: PathBuf,
    to: PathBuf,
    allow_private: bool,
}

fn args() -> Result<Args, String> {
    let home = std::env::var_os("HOME").ok_or("HOME isn't set; pass --from.")?;
    // Tauri's app-data folder on macOS for com.ansiapps.coupler (CLAUDE.md rule 4).
    let mut a = Args { from: Path::new(&home).join("Library/Application Support/com.ansiapps.coupler"), to: PathBuf::from("web-mirror"), allow_private: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--from" => a.from = it.next().ok_or("--from needs a folder")?.into(),
            "--to" => a.to = it.next().ok_or("--to needs a folder")?.into(),
            "--allow-private" => a.allow_private = true,
            "-h" | "--help" => {
                println!("web_mirror [--from <app data>] [--to <folder>] [--allow-private]");
                std::process::exit(0);
            }
            other => return Err(format!("Unknown argument {other}")),
        }
    }
    Ok(a)
}

/// FNV-1a, 64 bits: names a file by its contents, for caching, not security.
fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Opens a copy of the hooks database, so a running Coupler is never
/// disturbed and nothing is written to the real one.
fn open_copy(from: &Path, scratch: &Path) -> Result<Hooks, String> {
    let db = from.join("hooks.sqlite");
    if !db.is_file() {
        return Err(format!("No hooks database at {}", db.display()));
    }
    std::fs::create_dir_all(scratch).map_err(|e| e.to_string())?;
    for suffix in ["", "-wal"] {
        let src = from.join(format!("hooks.sqlite{suffix}"));
        let dst = scratch.join(format!("hooks.sqlite{suffix}"));
        let _ = std::fs::remove_file(&dst);
        if src.is_file() {
            std::fs::copy(&src, &dst).map_err(|e| format!("Couldn't copy {}: {e}", src.display()))?;
        }
    }
    Hooks::open(&scratch.join("hooks.sqlite"))
}

/// The file the web build plays for a sound: MP3 as it is, anything else
/// as the desktop would render it, then made Ogg Opus.
fn sound(root: &Path, path: &str, looping: bool) -> Result<(Vec<u8>, &'static str), String> {
    let bytes = assets::audio(root, path, looping)?;
    if path.starts_with("mp3/") {
        return Ok((bytes, "mp3"));
    }
    let pcm = codecs::read_wav(&bytes)?;
    Ok((codecs::encode_opus(&pcm, &mut |_| {})?, "opus"))
}

fn run() -> Result<(), String> {
    let a = args()?;
    let root = a.from.join("Assets");
    let scratch = std::env::temp_dir().join(format!("coupler-web-mirror-{}", std::process::id()));
    let hooks = open_copy(&a.from, &scratch)?;

    let mut kept: Vec<(String, String, Trigger)> = Vec::new();
    let mut skipped = 0;
    for (key, value, trigger) in hooks.triggers() {
        if !a.allow_private && PRIVATE_KEYS.iter().any(|p| key.starts_with(p)) {
            println!("  skipped (may be private): {key} = {value}");
            skipped += 1;
            continue;
        }
        kept.push((key.to_string(), value.to_string(), trigger.clone()));
    }
    drop(hooks);
    let _ = std::fs::remove_dir_all(&scratch);

    // Each asset once, with whether something loops it (only MIDI
    // renders differently when it loops).
    let mut wanted: BTreeMap<String, bool> = BTreeMap::new();
    let mut once: BTreeMap<String, bool> = BTreeMap::new();
    for (_, _, t) in &kept {
        for (path, looping) in [(&t.sfx, false), (&t.bgm, t.bgm_loop), (&t.bgn, true), (&t.bgw, true), (&t.art, false)] {
            if let Some(p) = path {
                *wanted.entry(p.clone()).or_default() |= looping;
                *once.entry(p.clone()).or_default() |= !looping;
            }
        }
    }

    // A fresh folder: the mirror is exactly what this run made.
    if a.to.exists() {
        std::fs::remove_dir_all(&a.to).map_err(|e| format!("Couldn't clear {}: {e}", a.to.display()))?;
    }
    std::fs::create_dir_all(&a.to).map_err(|e| e.to_string())?;
    let write = |bytes: &[u8], ext: &str| -> Result<String, String> {
        let name = format!("{}.{ext}", fnv(bytes));
        std::fs::write(a.to.join(&name), bytes).map_err(|e| e.to_string())?;
        Ok(name)
    };

    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let mut failed = Vec::new();
    let mut bytes_out = 0u64;
    for (path, looping) in &wanted {
        let folder = path.split('/').next().unwrap_or_default();
        let made: Result<Vec<(String, String)>, String> = (|| {
            match assets::kind_of(folder) {
                Some(assets::Kind::Art) if assets::is_ansi(path) => {
                    let art = assets::ansi(&root, path)?;
                    Ok(vec![(path.clone(), write(&serde_json::to_vec(&art).map_err(|e| e.to_string())?, "json")?)])
                }
                Some(assets::Kind::Art) => {
                    let ext = if folder == "png" { "png" } else { "jpg" };
                    Ok(vec![(path.clone(), write(&assets::picture(&root, path)?, ext)?)])
                }
                Some(assets::Kind::Sfx | assets::Kind::Bgm) => {
                    let midi = matches!(folder, "mid" | "midi");
                    let mut out = Vec::new();
                    if !midi || once[path] {
                        let (b, ext) = sound(&root, path, false)?;
                        out.push((path.clone(), write(&b, ext)?));
                    }
                    if midi && *looping {
                        let (b, ext) = sound(&root, path, true)?;
                        out.push((format!("{path}#loop"), write(&b, ext)?));
                    }
                    Ok(out)
                }
                _ => Err("not an asset a trigger can use".into()),
            }
        })();
        match made {
            Ok(made) => {
                for (k, name) in made {
                    let size = std::fs::metadata(a.to.join(&name)).map_or(0, |m| m.len());
                    bytes_out += size;
                    println!("  {k} -> {name} ({} KB)", size.div_ceil(1024));
                    files.insert(k, name);
                }
            }
            Err(e) => {
                println!("  FAILED {path}: {e}");
                failed.push(path.clone());
            }
        }
    }

    let triggers: Vec<_> = kept.iter().map(|(key, value, trigger)| json!({ "key": key, "value": value, "trigger": trigger })).collect();
    let manifest = json!({ "version": 1, "triggers": triggers, "files": files });
    std::fs::write(a.to.join("hooks.json"), serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    println!(
        "Mirrored {} triggers and {} files ({:.1} MB) into {}. Skipped {skipped} private, {} failed.",
        kept.len(),
        files.len(),
        bytes_out as f64 / 1_048_576.0,
        a.to.display(),
        failed.len()
    );
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("These assets couldn't be mirrored: {}", failed.join(", ")))
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("web_mirror: {e}");
        std::process::exit(1);
    }
}

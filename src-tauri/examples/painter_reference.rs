//! **Reference pictures for the Painter engine** (`docs/coupler-handoff.md`, B1):
//! the Rust painter's own output for a fixed matrix, as UTF-8 `.ans` files
//! (what Create Asset writes: `create::to_ansi`), plus `manifest.json` (each
//! file's scene, size, colors and seed) and `seeds.json` (the FNV-1a values,
//! as decimal strings, for cross-checking `roomSeed`). For the maintainer
//! only; never part of the app. The PNGs come from
//! `scripts/painter-reference-png.py`, which reads these files.
//!
//! ```sh
//! cargo run --release --manifest-path src-tauri/Cargo.toml --example painter_reference -- [--to painter-data/reference]
//! ```
//!
//! Only the game's own words about a room go in (`paint.rs`): a terrain, a
//! time, a weather. No room names, no zones, nothing a player typed.

use coupler_lib::ansi::{Color, Line};
use coupler_lib::create::to_ansi;
use coupler_lib::paint::{seed, Colors, Scene, Style};
use coupler_lib::painter::paint;
use serde_json::json;
use std::path::PathBuf;

/// CoffeeMUD's 24 terrains: outdoors, then indoors (`ambient::room_type`).
const TERRAINS: [&str; 24] = [
    "city", "woods", "rocky", "plains", "underwater", "air", "watersurface", "jungle", "swamp", "desert", "hills", "mountains", "spaceport", "seaport", "stone", "wooden", "cave", "magic", "in_underwater", "gap", "cavelakesurface", "metal", "innerseaport", "caveseaport",
];
const SCENES: [(&str, &str); 3] = [("day", "clear"), ("night", "thunderstorm"), ("dusk", "snow")];
const WORLD: &str = "standard";
const ROOM: &str = "Midgaard#3001";
const OTHER_ROOM: &str = "Midgaard#3002";

struct Job {
    terrain: &'static str,
    time: &'static str,
    weather: &'static str,
    style: Style,
    columns: usize,
    rows: usize,
    colors: Colors,
    arc: Option<u16>,
    room: &'static str,
}

fn style_name(style: Style) -> &'static str {
    match style {
        Style::Fantasy => "fantasy",
        Style::High => "high",
    }
}

fn colors_name(colors: Colors) -> &'static str {
    match colors {
        Colors::Sixteen => "16",
        Colors::More => "256",
        Colors::True => "true",
    }
}

fn job(terrain: &'static str, (time, weather): (&'static str, &'static str), style: Style) -> Job {
    Job { terrain, time, weather, style, columns: 80, rows: 24, colors: Colors::Sixteen, arc: None, room: ROOM }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut to = PathBuf::from("painter-data/reference");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--to" => to = PathBuf::from(args.next().expect("--to needs a folder")),
            other => panic!("unknown argument {other}"),
        }
    }
    std::fs::create_dir_all(&to).expect("the folder");

    let mut jobs = Vec::new();
    // Every terrain x three scenes x two styles at 80 by 24, in 16 colors.
    for terrain in TERRAINS {
        for scene in SCENES {
            for style in [Style::Fantasy, Style::High] {
                jobs.push(job(terrain, scene, style));
            }
        }
    }
    // The other sizes, for a spread of terrains: outdoors, a city, a cave, a room, the deep.
    for terrain in ["plains", "woods", "city", "mountains", "watersurface", "underwater", "stone", "cave"] {
        for (columns, rows) in [(25, 12), (8, 4), (160, 45)] {
            for scene in [SCENES[0], SCENES[1]] {
                jobs.push(Job { columns, rows, ..job(terrain, scene, Style::Fantasy) });
            }
        }
    }
    // The sun's and the moon's place: the arc at both ends and the middle.
    for arc in [0u16, 500, 1000] {
        for scene in [("day", "clear"), ("night", "clear")] {
            for style in [Style::Fantasy, Style::High] {
                jobs.push(Job { arc: Some(arc), ..job("plains", scene, style) });
            }
        }
    }
    // The same scene in each of the game's colors.
    for terrain in ["plains", "woods", "mountains", "city", "watersurface", "stone", "cave", "magic"] {
        for scene in [SCENES[0], SCENES[1], SCENES[2]] {
            for colors in [Colors::More, Colors::True] {
                jobs.push(Job { colors, ..job(terrain, scene, Style::Fantasy) });
            }
        }
    }
    // A second room: the seed changes where the hills rise and the trees stand.
    for terrain in ["plains", "woods", "hills", "mountains", "city", "stone", "wooden", "cave"] {
        jobs.push(Job { room: OTHER_ROOM, ..job(terrain, SCENES[0], Style::Fantasy) });
    }
    let mut files = Vec::new();
    for j in &jobs {
        let scene = Scene {
            world: WORLD.into(),
            room: j.room.into(),
            terrain: j.terrain.into(),
            weather: Some(j.weather.into()),
            time: Some(j.time.into()),
            arc: j.arc,
            ..Scene::default()
        };
        let art = paint(&scene, j.style, j.columns, j.rows, 0, j.colors);
        let name = format!(
            "{}_{}-{}_{}{}_{}x{}_c{}_{}.utf8.ans",
            j.terrain,
            j.time,
            j.weather,
            style_name(j.style),
            j.arc.map(|a| format!("_arc{a}")).unwrap_or_default(),
            art.columns,
            art.rows,
            colors_name(j.colors),
            if j.room == ROOM { "room1" } else { "room2" }
        );
        std::fs::write(to.join(&name), to_ansi(&art, &format!("{} {} {}", j.terrain, j.time, j.weather))).expect("a file");
        files.push(json!({
            "file": name,
            "encoding": "utf-8",
            "terrain": j.terrain,
            "time": j.time,
            "weather": j.weather,
            "style": style_name(j.style),
            "arc": j.arc,
            "colors": colors_name(j.colors),
            "columns": art.columns,
            "rows": art.rows,
            "world": WORLD,
            "room": j.room,
            "look": 0,
            "seed": seed(WORLD, j.room, 0).to_string(),
            "distinctColors": distinct(&art.lines),
        }));
    }

    let manifest = json!({
        "source": "Coupler painter.rs at this checkout, made by examples/painter_reference.rs",
        "coffeemud": "720aec6 (2026-10-03, CoffeeMUD 5.11.0.4)",
        "made": "2026-10-29",
        "note": "UTF-8 .ans, CRLF, one SGR per span, SAUCE record at the end. 256 and true colors write 38;5;n and 38;2;r;g;b. Pictures are drawn with look 0 (again = 0).",
        "files": files,
    });
    std::fs::write(to.join("manifest.json"), serde_json::to_string_pretty(&manifest).unwrap() + "\n").unwrap();

    let mut seeds = Vec::new();
    for room in [ROOM, OTHER_ROOM] {
        for world in [WORLD, "classic"] {
            for look in [0u32, 1, 2] {
                seeds.push(json!({ "world": world, "room": room, "look": look, "seed": seed(world, room, look).to_string() }));
            }
        }
    }
    let seeds = json!({
        "source": "paint::seed: FNV-1a 64 over world bytes, 0, room bytes, then `again` as 4 little-endian bytes",
        "made": "2026-10-29",
        "seeds": seeds,
    });
    std::fs::write(to.join("seeds.json"), serde_json::to_string_pretty(&seeds).unwrap() + "\n").unwrap();
    println!("{} pictures in {}", jobs.len(), to.display());
}

/// How many different colors a picture uses, to see 256 and true differ from 16.
fn distinct(lines: &[Line]) -> usize {
    let mut seen: Vec<Color> = Vec::new();
    for span in lines.iter().flatten() {
        for c in [span.style.fg, span.style.bg].into_iter().flatten() {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
    }
    seen.len()
}

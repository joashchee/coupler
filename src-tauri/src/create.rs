//! Making an asset from the player's words (the Hooks dialog's Assets
//! tab, Create Asset): an ART picture by Coupler's painter
//! (`painter.rs`, CLAUDE.md rule 11) or a BGM loop by Coupler's composer
//! (`compose.rs`). Pure and unit-tested; `assets::add_made` saves it.
//!
//! **A picture.** The painter reads a `Scene`, not words, so the words
//! are read for what it can draw: a terrain (CoffeeMUD's 24, the first
//! one named: "a tavern in the woods" is a tavern), the weather, the
//! time of day, and the high-fantasy style ("epic", "castle", "aurora").
//! Every other word is only the seed, so the same words paint the same
//! picture, and a take paints another. What it understood comes back in
//! words, said and shown beside the picture (rule 10).
//!
//! **The file.** The painting is written as ANSI art: UTF-8 with SGR
//! colors (the 16 VGA colors, bright ones as 90-97 and 100-107), the
//! kind `ansi_art.rs` reads back cell for cell, with a SAUCE record
//! giving its width so a row that fills it doesn't wrap.

use crate::ansi::Color;
use crate::ansi_art::Art;
use crate::paint::{Scene, Style};

/// Terrain words, the painter's terrains (`ambient.rs`'s), and the name said.
const TERRAINS: [(&[&str], &str, &str); 21] = [
    (&["underwater", "reef", "coral", "seabed", "seafloor"], "underwater", "under the sea"),
    (&["harbor", "harbour", "port", "docks", "dock", "pier", "wharf", "quay"], "seaport", "a harbor"),
    (&["spaceport", "starship", "spaceship"], "spaceport", "a spaceport"),
    (&["sea", "ocean", "lake", "river", "waves", "sailing", "ship", "shore", "beach", "coast", "bay"], "watersurface", "the water"),
    (&["sky", "clouds", "flying", "air", "heavens"], "air", "the open sky"),
    (&["jungle", "rainforest"], "jungle", "a jungle"),
    (&["swamp", "marsh", "bog", "fen", "bayou", "mire"], "swamp", "a swamp"),
    (&["desert", "dunes", "dune", "sand", "sands", "oasis", "wasteland"], "desert", "a desert"),
    (&["mountain", "mountains", "peak", "peaks", "summit", "alps", "volcano"], "mountains", "the mountains"),
    (&["hill", "hills", "downs", "moor", "moors", "highlands"], "hills", "the hills"),
    (&["rocky", "rocks", "crags", "badlands", "canyon", "boulders", "cliff", "cliffs"], "rocky", "rocky ground"),
    (&["forest", "woods", "wood", "grove", "glade", "trees", "woodland", "thicket"], "woods", "a forest"),
    (&["plains", "plain", "field", "fields", "meadow", "meadows", "grassland", "farm", "prairie", "steppe", "countryside", "road"], "plains", "the plains"),
    (&["city", "town", "village", "street", "streets", "market", "square", "rooftops", "castle", "fortress", "citadel"], "city", "a town"),
    (&["cavern", "cave", "caves", "grotto", "mine", "mines", "tunnel", "tunnels", "underground"], "cave", "a cave"),
    (&["chasm", "abyss", "gap", "pit", "ravine"], "gap", "a chasm"),
    (&["tavern", "inn", "cabin", "house", "hut", "shop", "cottage", "barn", "attic", "library", "bedroom", "kitchen"], "wooden", "a wooden room"),
    (&["vault", "bunker", "machine", "machines", "engine", "workshop", "laboratory", "lab", "metal", "steel", "ship's"], "metal", "a metal room"),
    (&["enchanted", "arcane", "sanctum", "wizard", "wizard's", "mage", "magical", "portal"], "magic", "a magical room"),
    (&["dungeon", "hall", "temple", "chamber", "crypt", "tomb", "throne", "cellar", "prison", "cell", "keep", "church", "cathedral", "room", "corridor", "stone"], "stone", "a stone hall"),
    (&["indoors", "inside", "interior"], "stone", "a stone hall"),
];

/// Weather words, as `coupler.weather` names the kinds (`ambient.rs`).
const WEATHER: [(&[&str], &str, &str); 12] = [
    (&["thunderstorm", "storm", "stormy", "thunder", "lightning", "tempest"], "thunderstorm", "a thunderstorm"),
    (&["blizzard"], "blizzard", "a blizzard"),
    (&["hail", "hailstorm"], "hail", "hail"),
    (&["sleet"], "sleet", "sleet"),
    (&["snow", "snowy", "snowing", "snowfall", "winter"], "snow", "snow"),
    (&["rain", "rainy", "raining", "drizzle", "showers", "downpour", "wet"], "rain", "rain"),
    (&["fog", "foggy", "mist", "misty", "haze", "hazy"], "fog", "fog"),
    (&["dust", "dusty", "sandstorm"], "dust", "dust"),
    (&["cloudy", "overcast", "grey", "gray", "gloomy"], "cloudy", "clouds"),
    (&["wind", "windy", "gale", "gusty", "breezy"], "windy", "wind"),
    (&["hot", "scorching", "heat", "sweltering", "summer"], "heat", "heat"),
    (&["cold", "freezing", "icy", "frozen", "frost", "frosty"], "cold", "cold"),
];

/// Time words: the part of the day, where the sun or moon is, and the name said.
const TIMES: [(&[&str], &str, u16, &str); 4] = [
    (&["dawn", "sunrise", "daybreak", "morning"], "dawn", 90, "at dawn"),
    (&["dusk", "sunset", "evening", "twilight", "sundown"], "dusk", 910, "at dusk"),
    (&["night", "midnight", "moon", "moonlight", "moonlit", "stars", "starry", "nighttime", "dark"], "night", 500, "at night"),
    (&["day", "noon", "midday", "afternoon", "daylight", "sunny", "sun", "daytime"], "day", 500, "by day"),
];

/// Words that ask for high fantasy: towers on the horizon, a second moon, auroras.
const HIGH: [&str; 14] = ["epic", "high", "castle", "castles", "towers", "tower", "aurora", "auroras", "majestic", "legendary", "heroic", "kingdom", "banners", "moons"];

/// The words, lower case, letters, digits and apostrophes only.
fn words(prompt: &str) -> Vec<String> {
    prompt.split(|c: char| !(c.is_alphanumeric() || c == '\'')).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// The first of the words any entry names, by where it's named.
fn first<'a, T>(words: &[String], table: &'a [T], names: impl Fn(&T) -> &[&str]) -> Option<&'a T> {
    words.iter().find_map(|w| table.iter().find(|entry| names(entry).contains(&w.as_str())))
}

/// What the painter makes of the words: the scene, the style, and what
/// it understood in words ("A forest at night, in rain, high fantasy.").
pub fn scene_from(prompt: &str, take: u32) -> (Scene, Style, String) {
    let words = words(prompt);
    let terrain = first(&words, &TERRAINS, |t| t.0);
    let weather = first(&words, &WEATHER, |w| w.0);
    let time = first(&words, &TIMES, |t| t.0);
    let style = if words.iter().any(|w| HIGH.contains(&w.as_str())) || words.windows(2).any(|p| p[0] == "high" && p[1] == "fantasy") { Style::High } else { Style::Fantasy };
    let scene = Scene {
        world: "created".into(),
        // The words are the room: the same words, the same picture.
        room: words.join(" "),
        name: prompt.trim().into(),
        zone: String::new(),
        terrain: terrain.map_or("plains", |t| t.1).into(),
        weather: weather.map(|w| w.1.to_string()),
        time: time.map(|t| t.1.to_string()),
        arc: Some(time.map_or(500, |t| t.2)),
        look: take,
        art: None,
    };
    let mut about = terrain.map_or("The plains", |t| t.2).to_string();
    if let Some(first) = about.get(..1) {
        about = first.to_uppercase() + &about[1..];
    }
    let indoors = crate::ambient::room_type(&scene.terrain) == Some("indoors") || scene.terrain == "underwater";
    if !indoors {
        about.push(' ');
        about.push_str(time.map_or("by day", |t| t.3));
        if let Some(w) = weather {
            about.push_str(", in ");
            about.push_str(w.2);
        }
    }
    if style == Style::High {
        about.push_str(", high fantasy");
    }
    if terrain.is_none() {
        about.push_str(" (no place was named, so the plains)");
    }
    about.push('.');
    (scene, style, about)
}

/// A color's SGR code, foreground or background.
fn sgr(color: Option<Color>, background: bool) -> String {
    match color {
        Some(Color::Index { index }) if index < 8 => format!("{}", if background { 40 } else { 30 } + u16::from(index)),
        Some(Color::Index { index }) if index < 16 => format!("{}", if background { 100 } else { 90 } + u16::from(index - 8)),
        Some(Color::Index { index }) => format!("{};5;{index}", if background { 48 } else { 38 }),
        Some(Color::Rgb { r, g, b }) => format!("{};2;{r};{g};{b}", if background { 48 } else { 38 }),
        None => (if background { "49" } else { "39" }).into(),
    }
}

/// A picture as an ANSI art file: UTF-8, each row from a reset, and a
/// SAUCE record with its width and height.
pub fn to_ansi(art: &Art, title: &str) -> Vec<u8> {
    let mut text = String::new();
    for line in &art.lines {
        text.push_str("\x1b[0m");
        for span in line {
            let s = span.style;
            let mut codes = vec!["0".to_string(), sgr(s.fg, false), sgr(s.bg, true)];
            for (on, code) in [(s.bold, "1"), (s.italic, "3"), (s.underline, "4"), (s.inverse, "7")] {
                if on {
                    codes.push(code.into());
                }
            }
            text.push_str(&format!("\x1b[{}m", codes.join(";")));
            text.push_str(&span.text);
        }
        text.push_str("\x1b[0m\r\n");
    }
    let mut out = text.into_bytes();
    out.push(0x1a);
    out.extend(sauce(title, art.columns, art.rows));
    out
}

/// A SAUCE record: an ANSi file (character data) of this many columns and rows.
fn sauce(title: &str, columns: usize, rows: usize) -> Vec<u8> {
    let mut record = vec![b' '; 128];
    record[..7].copy_from_slice(b"SAUCE00");
    let field = |record: &mut Vec<u8>, at: usize, len: usize, text: &str| {
        for (i, b) in text.bytes().filter(u8::is_ascii).take(len).enumerate() {
            record[at + i] = b;
        }
    };
    field(&mut record, 7, 35, title);
    field(&mut record, 42, 20, "Coupler's painter");
    field(&mut record, 82, 8, "        ");
    record[90..94].fill(0); // the file's size: left unsaid
    record[94] = 1; // character data
    record[95] = 1; // ANSi
    record[96..98].copy_from_slice(&(columns.min(u16::MAX as usize) as u16).to_le_bytes());
    record[98..100].copy_from_slice(&(rows.min(u16::MAX as usize) as u16).to_le_bytes());
    record[100..106].fill(0);
    record[106..128].fill(0);
    record
}

/// A file name from the words: letters, digits, spaces, hyphens and
/// apostrophes, at most 40 characters cut at a word, or `fallback`.
pub fn file_stem(prompt: &str, fallback: &str) -> String {
    let kept: String = prompt.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '\'' { c } else { ' ' }).collect();
    let mut stem = String::new();
    for word in kept.split_whitespace() {
        if stem.chars().count() + word.chars().count() + 1 > 40 {
            break;
        }
        if !stem.is_empty() {
            stem.push(' ');
        }
        stem.push_str(word);
    }
    let stem = stem.trim_matches(|c| c == '-' || c == '\'').to_string();
    if stem.is_empty() { fallback.into() } else { stem }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_become_what_the_painter_draws() {
        let (scene, style, about) = scene_from("A stormy night out on the sea", 0);
        assert_eq!((scene.terrain.as_str(), scene.weather.as_deref(), scene.time.as_deref(), style), ("watersurface", Some("thunderstorm"), Some("night"), Style::Fantasy));
        assert_eq!(about, "The water at night, in a thunderstorm.");
        let (scene, style, about) = scene_from("an epic castle at dawn", 0);
        assert_eq!((scene.terrain.as_str(), scene.time.as_deref(), scene.arc, style), ("city", Some("dawn"), Some(90), Style::High));
        assert_eq!(about, "A town at dawn, high fantasy.");
        // The first place named wins, and indoors there's no sky to speak of.
        let (scene, _, about) = scene_from("a tavern in the misty woods", 0);
        assert_eq!(scene.terrain, "wooden");
        assert_eq!(about, "A wooden room.");
        let (scene, _, about) = scene_from("something lovely", 3);
        assert_eq!((scene.terrain.as_str(), scene.look), ("plains", 3));
        assert_eq!(about, "The plains by day (no place was named, so the plains).");
    }

    #[test]
    fn the_same_words_paint_the_same_picture_and_a_take_another() {
        let paint = |prompt: &str, take| {
            let (scene, style, _) = scene_from(prompt, take);
            crate::painter::paint(&scene, style, 40, 12, take)
        };
        assert_eq!(paint("the woods at night", 0), paint("The woods, at night!", 0));
        assert_ne!(paint("the woods at night", 0), paint("the woods at night", 1));
    }

    /// A cell: its character, its background, and its foreground unless it's a space.
    type Cell = (char, Option<Color>, Option<Color>);

    fn cells(art: &Art) -> Vec<Vec<Cell>> {
        art.lines
            .iter()
            .map(|line| line.iter().flat_map(|s| s.text.chars().map(move |c| (c, s.style.bg, if c == ' ' { None } else { s.style.fg }))).collect())
            .collect()
    }

    #[test]
    fn a_painting_written_as_ansi_reads_back_cell_for_cell() {
        for (prompt, columns, rows) in [("a forest at dawn", 40, 12), ("a stormy sea at night", 80, 25), ("a dungeon", 60, 18)] {
            let (scene, style, _) = scene_from(prompt, 0);
            let art = crate::painter::paint(&scene, style, columns, rows, 0);
            let file = to_ansi(&art, prompt);
            let back = crate::ansi_art::render(&file);
            assert_eq!((back.columns, back.rows), (columns, rows), "{prompt}");
            assert_eq!(cells(&back), cells(&art), "{prompt}");
        }
    }

    #[test]
    fn the_sauce_record_says_the_size() {
        let art = Art { columns: 3, rows: 1, lines: vec![vec![crate::ansi::Span { text: "abc".into(), style: Default::default() }]] };
        let file = to_ansi(&art, "A title that is far too long to fit in its thirty-five");
        let record = &file[file.len() - 128..];
        assert!(record.starts_with(b"SAUCE00A title"));
        assert_eq!(u16::from_le_bytes([record[96], record[97]]), 3);
        assert_eq!(file[file.len() - 129], 0x1a);
    }

    #[test]
    fn a_file_name_comes_from_the_words() {
        assert_eq!(file_stem("A stormy night: on the sea!", "picture"), "A stormy night on the sea");
        assert_eq!(file_stem("../../etc/passwd", "picture"), "etc passwd");
        assert_eq!(file_stem("   ", "music"), "music");
        assert_eq!(file_stem("...", "music"), "music");
        assert_eq!(file_stem("the long and winding road through the misty mountains", "x"), "the long and winding road through the");
    }
}

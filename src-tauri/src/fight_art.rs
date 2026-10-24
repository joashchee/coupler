//! **The Fight panel's picture** by Coupler's painter: the room the
//! fight is in, as its picture shows it, across the panel's width and a
//! shade darker, with the two fighters standing in it: the player on
//! the left, mirrored to face right, the foe on the right. Each is drawn
//! by `face.rs` as it is now (its mood, its hurt, what's on it) on a
//! clear canvas, so both stand in the one place and its light falls on
//! both (`figure.rs`). An animal foe's back runs off the right edge.
//! Pure and unit-tested. All ART is Coupler's painter's first
//! (CLAUDE.md rule 11).
//!
//! The panel's words (the names, the bars, the clock) are the
//! frontend's (`FightPanel.tsx`), in bands of their own: no text sits on
//! the picture.
//!
//! Its moments are frames painted in one go (`frames`): a big blow jolts
//! the one hit a cell away from the other and flashes it once, a new foe
//! slides in from the right a cell at a time. The frontend plays them
//! and keeps to its own limits (a flash at most once a blow, never more
//! than three a second, no jolt or slide with reduced motion).

use crate::ansi_art::Art;
use crate::face::{self, Moment, Sitter, SIZE};
use crate::figure::{self, lighter, CLEAR};
use crate::paint::{Scene, Style};
use crate::painter::*;
use serde::Deserialize;

/// The fewest columns: the two faces side by side, overlapping in the
/// middle where the panel is narrower than both (the foe in front).
pub const LEAST_COLUMNS: usize = 40;
/// The most: the stage's width.
const MOST_COLUMNS: usize = 160;
/// The picture's rows: a face's.
pub const ROWS: usize = SIZE.1;

/// Both fighters, as they are now.
pub struct Fighters<'a> {
    pub you: (&'a Sitter, &'a Moment),
    pub foe: (&'a Sitter, &'a Moment),
}

/// Where a fighter stands in a frame, in cells from its place (the
/// player's at the left edge, the foe's at the right; more is further
/// right), and whether it flashes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pose {
    pub shift: i32,
    pub flash: bool,
}

/// One frame: the two fighters' poses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Frame {
    pub you: Pose,
    pub foe: Pose,
}

/// The picture, `columns` wide and a face tall, both standing still.
/// Without a room (not yet known), a plain dark ground.
#[cfg(test)]
pub fn paint(scene: Option<&Scene>, style: Style, fighters: &Fighters, columns: usize) -> Art {
    frames(scene, style, fighters, columns, &[Frame::default()]).remove(0)
}

/// The picture once for each frame, the room and the faces painted once
/// for them all. No frames, one still.
pub fn frames(scene: Option<&Scene>, style: Style, fighters: &Fighters, columns: usize, frames: &[Frame]) -> Vec<Art> {
    let columns = columns.clamp(LEAST_COLUMNS, MOST_COLUMNS);
    let (room, light) = match scene {
        Some(scene) => {
            let (c, light) = figure::room(scene, style, columns, ROWS);
            (c, Some(light))
        }
        None => {
            let mut c = Canvas::new(columns, ROWS);
            c.gradient(0, c.h, &[BLACK, BLACK, DARKGRAY]);
            (c, None)
        }
    };
    let you = face::standing(fighters.you.0, fighters.you.1).mirrored();
    let foe = face::standing(fighters.foe.0, fighters.foe.1);
    let (you_lit, foe_lit) = (flashed(&you), flashed(&foe));
    let still = [Frame::default()];
    let frames = if frames.is_empty() { &still[..] } else { frames };
    frames
        .iter()
        .map(|frame| {
            let mut canvas = room.clone();
            canvas.stamp(if frame.you.flash { &you_lit } else { &you }, frame.you.shift, CLEAR);
            canvas.stamp(if frame.foe.flash { &foe_lit } else { &foe }, (columns - SIZE.0) as i32 + frame.foe.shift, CLEAR);
            if let Some(light) = &light {
                figure::light(&mut canvas, light);
            }
            canvas.art()
        })
        .collect()
}

/// A fighter in the flash of a blow: every pixel a shade lighter, and
/// giving its own light, so neither the rim nor the night touches it.
fn flashed(figure: &Canvas) -> Canvas {
    let mut lit = figure.clone();
    for y in 0..lit.h {
        for x in 0..lit.w {
            let c = lit.get(x, y);
            if c != CLEAR {
                let part = lit.part(x, y);
                lit.put(x, y, lighter(c), true, part);
            }
        }
    }
    lit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::Mood;

    fn person(name: &str, race: &str) -> Sitter {
        face::player(name, Some(race))
    }

    fn scene(terrain: &str) -> Scene {
        Scene { world: "test".into(), room: "1".into(), terrain: terrain.into(), ..Scene::default() }
    }

    #[test]
    fn two_faces_in_the_room_at_any_width() {
        let (you, foe) = (person("Ada", "Human"), face::sitter_named("a large wolf"));
        let (calm, angry) = (Moment::default(), Moment { mood: Mood::Angry, hurt: 2, ..Moment::default() });
        let font = crate::ansi_art::CP437;
        for columns in [10, 50, 64, 73, 200] {
            let art = paint(Some(&scene("woods")), Style::Fantasy, &Fighters { you: (&you, &calm), foe: (&foe, &angry) }, columns);
            assert_eq!(art.columns, columns.clamp(LEAST_COLUMNS, 160));
            assert_eq!((art.rows, art.lines.len()), (ROWS, ROWS));
            assert!(art.lines.iter().flatten().all(|s| s.text.chars().all(|c| font.contains(c))));
        }
    }

    #[test]
    fn the_player_faces_the_foe() {
        let you = person("Ada", "Human");
        let calm = Moment::default();
        let alone = face::standing(&you, &calm);
        let mirrored = alone.mirrored();
        // The same face, left for right.
        for y in 0..alone.h {
            for x in 0..alone.w {
                assert_eq!(alone.get(x, y), mirrored.get(alone.w - 1 - x, y));
            }
        }
        // Laid on the place, the foe's side is the foe's.
        let foe = face::sitter_named("a goblin");
        let art = paint(None, Style::Fantasy, &Fighters { you: (&you, &calm), foe: (&foe, &calm) }, 64);
        let only_you = paint(None, Style::Fantasy, &Fighters { you: (&you, &calm), foe: (&you, &calm) }, 64);
        assert_eq!(art.lines.len(), only_you.lines.len());
        assert_ne!(art, only_you);
    }

    #[test]
    fn hurt_and_mood_change_the_picture() {
        let (you, foe) = (person("Ada", "Elf"), face::sitter_named("an orc"));
        let calm = Moment::default();
        let at = |m: &Moment| paint(Some(&scene("plains")), Style::Fantasy, &Fighters { you: (&you, &calm), foe: (&foe, m) }, 60);
        let fresh = at(&calm);
        assert_ne!(fresh, at(&Moment { hurt: 4, ..Moment::default() }));
        assert_ne!(fresh, at(&Moment { mood: Mood::Dead, ..Moment::default() }));
        // The same moment, the same picture.
        assert_eq!(fresh, at(&calm));
    }

    #[test]
    fn frames_jolt_flash_and_slide() {
        let (you, foe) = (person("Ada", "Human"), face::sitter_named("a large wolf"));
        let calm = Moment::default();
        let fighters = Fighters { you: (&you, &calm), foe: (&foe, &calm) };
        let room = scene("woods");
        let still = paint(Some(&room), Style::Fantasy, &fighters, 64);
        // No frames is the one still.
        assert_eq!(frames(Some(&room), Style::Fantasy, &fighters, 64, &[]), vec![still.clone()]);
        let hit = |you: Pose, foe: Pose| frames(Some(&room), Style::Fantasy, &fighters, 64, &[Frame { you, foe }]).remove(0);
        let jolted = hit(Pose::default(), Pose { shift: 1, flash: false });
        let flashed = hit(Pose { shift: 0, flash: true }, Pose::default());
        assert_ne!(jolted, still);
        assert_ne!(flashed, still);
        // A flash only lightens: the same picture's size, in the font and the 16 colors.
        assert_eq!((flashed.rows, flashed.columns), (still.rows, still.columns));
        // Slid all the way out, the foe's gone: the same as no foe at all.
        let gone = hit(Pose::default(), Pose { shift: SIZE.0 as i32, flash: false });
        let alone = {
            let none = Sitter { name: "nothing".into(), ..Sitter::default() };
            let f = Fighters { you: (&you, &calm), foe: (&none, &calm) };
            frames(Some(&room), Style::Fantasy, &f, 64, &[Frame { foe: Pose { shift: SIZE.0 as i32, flash: false }, ..Frame::default() }]).remove(0)
        };
        assert_eq!(gone, alone);
        // Each frame its own picture, in order.
        let slide: Vec<Frame> = (0..=4).rev().map(|shift| Frame { foe: Pose { shift, flash: false }, ..Frame::default() }).collect();
        let played = frames(Some(&room), Style::Fantasy, &fighters, 64, &slide);
        assert_eq!(played.len(), 5);
        assert_eq!(played[4], still);
        assert_ne!(played[0], played[4]);
    }

    #[test]
    fn a_name_alone_gives_its_race() {
        let wolf = face::sitter_named("a large wolf");
        assert_eq!(wolf.race.as_deref(), Some("Wolf"));
        assert_eq!(face::sitter_named("Grumble the merchant").race, None);
        assert_eq!(face::player("Ada", Some("Human")).category.as_deref(), Some("Humanoid"));
    }

    /// Writes a sheet of fights as a PPM to `OUT` to look at while
    /// tuning: a few fighters in a few rooms, fresh, then hurt, then the
    /// end. `OUT=/tmp/fight.ppm cargo test fight_art -- --ignored`.
    #[test]
    #[ignore]
    fn show() {
        let out = std::env::var("OUT").unwrap_or_else(|_| "fight.ppm".into());
        let fights = [
            ("Human", "a large wolf", "woods", Some("day")),
            ("Elf", "an orc", "hills", Some("dusk")),
            ("Dwarf", "a giant spider", "cave", None),
            ("Halfling", "a city guard", "city", Some("night")),
            ("Human", "a black bear", "mountains", Some("day")),
            ("Gnome", "a skeleton", "stone", None),
        ];
        let mut arts = Vec::new();
        for (race, foe, terrain, time) in fights {
            let you = face::player("Ada", Some(race));
            let foe = face::sitter_named(foe);
            let scene = Scene { time: time.map(str::to_string), ..scene(terrain) };
            for (mine, theirs) in [
                (Moment { mood: Mood::Angry, ..Moment::default() }, Moment { mood: Mood::Angry, ..Moment::default() }),
                (Moment { mood: Mood::Angry, hurt: 2, ..Moment::default() }, Moment { mood: Mood::Pain, hurt: 4, ..Moment::default() }),
                (Moment { hurt: 2, ..Moment::default() }, Moment { mood: Mood::Dead, hurt: 5, ..Moment::default() }),
            ] {
                arts.push(paint(Some(&scene), Style::Fantasy, &Fighters { you: (&you, &mine), foe: (&foe, &theirs) }, 72));
            }
        }
        face::look::sheet(&arts, 3, &out);
    }
}

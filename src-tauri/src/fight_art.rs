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

use crate::ansi_art::Art;
use crate::face::{self, Moment, Sitter, SIZE};
use crate::figure::{self, CLEAR};
use crate::paint::{Scene, Style};
use crate::painter::*;

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

/// The picture, `columns` wide and a face tall.
/// Without a room (not yet known), a plain dark ground.
pub fn paint(scene: Option<&Scene>, style: Style, fighters: &Fighters, columns: usize) -> Art {
    let columns = columns.clamp(LEAST_COLUMNS, MOST_COLUMNS);
    let (mut canvas, light) = match scene {
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
    canvas.stamp(&you, 0, CLEAR);
    canvas.stamp(&foe, (columns - SIZE.0) as i32, CLEAR);
    if let Some(light) = &light {
        figure::light(&mut canvas, light);
    }
    canvas.art()
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

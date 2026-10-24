//! **Figures on a place**, shared by the painter's faces (`face.rs`)
//! and items (`item_art.rs`): the place behind them, painted by the
//! room painter (`painter.rs`) at the picture's size and a shade darker
//! so what's in front stands out; and what's in front, drawn on a clear
//! canvas of its own and laid on it, its outline lit by the place's own
//! colors (a rim light), so the two look like one picture. Pure and
//! unit-tested. All ART is Coupler's painter's first (CLAUDE.md rule 11).

use crate::ambient;
use crate::paint::{Scene, Style};
use crate::painter::*;

/// No paint: what a figure's canvas starts as, so its shape is known.
pub(crate) const CLEAR: u8 = 255;

/// A canvas for a figure: all clear.
pub(crate) fn clear(columns: usize, rows: usize) -> Canvas {
    let mut c = Canvas::new(columns, rows);
    c.rect(0, 0, c.w, c.h, CLEAR);
    c
}

/// The light of a place painted behind a figure.
pub(crate) struct Place {
    indoors: bool,
    night: bool,
}

/// The place where something was seen, as the room painter paints it at
/// a size in cells, darkened, and its light. `key` seeds it, so the same
/// creature or item stands in the same spot of the same kind of place
/// each time.
pub(crate) fn place(key: &str, scene: &Scene, columns: usize, rows: usize) -> (Canvas, Place) {
    // Without the hour, the sun stands to one side, never over a head.
    let arc = scene.arc.or_else(|| Some([150, 250, 750, 850][(crate::paint::seed("sun", key, 0) % 4) as usize]));
    let scene = Scene { world: "bestiary".into(), room: key.into(), arc, ..scene.clone() };
    let mut canvas = draw(&scene, Style::Fantasy, columns, rows, 0);
    canvas.recede();
    let terrain = scene.terrain.to_ascii_lowercase();
    (canvas, Place { indoors: !ambient::has_sky(&terrain), night: scene.time.as_deref() == Some("night") })
}

/// The room itself as its picture shows it (the same look), darkened:
/// the Fight panel's scene (`fight_art.rs`).
pub(crate) fn room(scene: &Scene, style: Style, columns: usize, rows: usize) -> (Canvas, Place) {
    let mut canvas = draw(scene, style, columns, rows, scene.look);
    canvas.recede();
    let terrain = scene.terrain.to_ascii_lowercase();
    (canvas, Place { indoors: !ambient::has_sky(&terrain), night: scene.time.as_deref() == Some("night") })
}

/// A place from the terrain alone, by day.
pub(crate) fn terrain(key: &str, terrain: &str, columns: usize, rows: usize) -> (Canvas, Place) {
    place(key, &Scene { terrain: terrain.into(), ..Scene::default() }, columns, rows)
}

/// What a terrain looks like to an animal: where it lives, never a
/// room or a street. Outdoors stays; anything else becomes the wild.
pub(crate) fn wild(terrain: &str, key: &str) -> String {
    let t = terrain.to_ascii_lowercase();
    if ambient::room_type(&t) == Some("outdoors") && t != "city" && t != "spaceport" && t != "seaport" {
        return t;
    }
    let wilds = ["woods", "plains", "hills"];
    wilds[(crate::paint::seed("wild", key, 0) % wilds.len() as u64) as usize].to_string()
}

/// Lays `figure` on `back`, moved by `offset`, skipping the pixels
/// `hide` says (a figure fading, half invisible). Characters under the
/// figure go: it stands in front of the rain. Where the figure is moved
/// in from past its right edge, its body runs on (an animal's back
/// running off the picture's side, `face.rs`).
pub(crate) fn lay(back: &mut Canvas, figure: &Canvas, offset: (i32, i32), hide: &dyn Fn(i32, i32) -> bool) {
    for y in 0..back.h {
        for x in 0..back.w {
            let (fx, fy) = (x + offset.0, y - offset.1);
            let (sx, part) = if fx >= figure.w && figure.part(figure.w - 1, fy) == Part::Body { (figure.w - 1, Part::Body) } else { (fx, figure.part(fx, fy)) };
            if !figure.inside(sx, fy) {
                continue;
            }
            let c = figure.get(sx, fy);
            if c == CLEAR || hide(x, y) {
                continue;
            }
            back.put(x, y, c, figure.is_lit(sx, fy), part);
            back.unglyph(x, y);
        }
    }
}

/// Whether a pixel of `c` is the figure's (laid on it): anything not
/// the place's own.
pub(crate) fn of_figure(c: &Canvas, x: i32, y: i32) -> bool {
    c.inside(x, y) && c.part(x, y) != Part::Back
}

/// The place's light on the figure: outdoors by day, the sky lights its
/// top edge (each pixel with the place above it, a shade lighter); at
/// night outdoors, what doesn't give its own light goes dark, as the
/// place did. Indoors the room's own light is the figure's.
pub(crate) fn light(c: &mut Canvas, place: &Place) {
    if !place.indoors && !place.night {
        let mut rims = Vec::new();
        for y in 1..c.h {
            for x in 0..c.w {
                if of_figure(c, x, y) && !of_figure(c, x, y - 1) && c.part(x, y) != Part::Eye && !c.is_lit(x, y) {
                    rims.push((x, y, lighter(c.get(x, y))));
                }
            }
        }
        for (x, y, color) in rims {
            let part = c.part(x, y);
            c.put(x, y, color, false, part);
        }
    }
    if place.night && !place.indoors {
        for y in 0..c.h {
            for x in 0..c.w {
                if of_figure(c, x, y) && !c.is_lit(x, y) {
                    let part = c.part(x, y);
                    c.put(x, y, dim(c.get(x, y), x, y), false, part);
                }
            }
        }
    }
}

/// A lighter shade of a color, for a highlight or a rim.
pub(crate) fn lighter(c: u8) -> u8 {
    match c {
        BLACK => DARKGRAY,
        DARKGRAY => LIGHTGRAY,
        LIGHTGRAY => WHITE,
        BROWN => YELLOW,
        c if c < 8 => c + 8,
        _ => WHITE,
    }
}

/// A darker one, for a shadow, a brow, a dent.
pub(crate) fn shade(c: u8) -> u8 {
    match c {
        DARKGRAY => BLACK,
        LIGHTGRAY => DARKGRAY,
        WHITE => LIGHTGRAY,
        LIGHTRED => RED,
        YELLOW => BROWN,
        BROWN => RED,
        c if c > 8 => c - 8,
        BLACK => DARKGRAY,
        _ => DARKGRAY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_is_the_room_painters_and_darker() {
        let (c, _) = terrain("a wolf", "woods", 25, 12);
        let bright = |c: &Canvas| (0..c.h).flat_map(|y| (0..c.w).map(move |x| (x, y))).filter(|&(x, y)| c.get(x, y) >= 9).count();
        let painted = draw(&Scene { world: "bestiary".into(), room: "awolf".into(), terrain: "woods".into(), arc: Some(500), ..Scene::default() }, Style::Fantasy, 25, 12, 0);
        assert!(bright(&c) < bright(&painted));
        // The same key, the same place; another, another.
        assert_eq!(terrain("a wolf", "woods", 25, 12).0.art(), c.art());
        assert_ne!(terrain("a fox", "woods", 25, 12).0.art(), c.art());
    }

    #[test]
    fn animals_live_in_the_wild() {
        assert_eq!(wild("jungle", "a cat"), "jungle");
        assert!(["woods", "plains", "hills"].contains(&wild("city", "a cat").as_str()));
        assert!(["woods", "plains", "hills"].contains(&wild("stone", "a cat").as_str()));
    }

    #[test]
    fn a_figure_is_laid_on_the_place_and_its_outline_lit() {
        let (mut c, p) = terrain("a thing", "plains", 25, 12);
        let mut f = clear(25, 12);
        f.pen = Part::Body;
        f.rect(8, 8, 16, 20, RED);
        lay(&mut c, &f, (0, 0), &|_, _| false);
        assert_eq!(c.part(10, 10), Part::Body);
        assert_eq!(c.part(2, 2), Part::Back);
        light(&mut c, &p);
        // The sky lit its top edge; the inside kept its color.
        assert!((8..16).all(|x| c.get(x, 8) == LIGHTRED));
        assert_eq!(c.get(12, 14), RED);
        // Moved in from past its edge, the body runs on.
        let (mut q, _) = terrain("a thing", "plains", 25, 12);
        let mut g = clear(25, 12);
        g.pen = Part::Body;
        g.rect(20, 20, 25, 24, BROWN);
        lay(&mut q, &g, (3, 0), &|_, _| false);
        assert_eq!(q.part(24, 21), Part::Body);
    }
}

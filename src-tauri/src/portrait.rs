//! **Portraits** by Coupler's painter (`painter.rs`): a race or a class
//! drawn in ANSI, two square pixels a cell in the 16 VGA colors, for the
//! guide to making a character (components/CreationDialog.tsx). Pure and
//! unit-tested.
//!
//! All ART is Coupler's painter's first (CLAUDE.md): CoffeeMUD's own
//! pictures (`public/creation-art.json`) are the player's to choose
//! instead (gear → Pictures…).
//!
//! **A race** stands as itself beside a measuring stick marked every
//! foot up to six, so its size shows against the others: its height and
//! build from its Java (`SIZES`: every player race CoffeeMUD documents,
//! the ones a game turns on too, like Pixie), its skin and hair, pointed
//! ears (elves), a beard and a helmet (dwarves), a tall hat (gnomes),
//! bare feet (halflings), tusks (half ogres, orcs), wings (pixies,
//! faeries), and red eyes where it sees in the dark. A race the painter
//! doesn't know is made from its name, the same each time.
//!
//! **A class** is a person in its clothes with its tools: armor and a
//! sword, a robe and a glowing staff, a hood and a dagger, a holy symbol,
//! leaves, a lute, a bare-handed monk, a craftsman's hammer. Each of
//! CoffeeMUD's classes belongs to one of those kinds and has its own
//! colors; one the painter doesn't know is made from its name.

use crate::ansi_art::Art;
use crate::paint::seed;
use crate::painter::*;

/// The picture's size in cells, at most and at least.
const MOST: (usize, usize) = (80, 40);
const LEAST: (usize, usize) = (8, 4);

/// How a race looks.
#[derive(Clone, Copy)]
struct Build {
    /// Height in inches.
    inches: i32,
    /// Width against a human's, in tenths.
    bulk: i32,
    skin: u8,
    hair: Option<u8>,
    clothes: u8,
    feature: Feature,
    /// Sees in the dark: its eyes glow.
    infravision: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Feature {
    None,
    PointedEars,
    BeardAndHelmet,
    TallHat,
    BareFeet,
    Tusks,
    /// Wings behind the shoulders, in this color.
    Wings(u8),
}

/// Every player race's average height (inches: `shortestMale` and half
/// `heightVariance`) and weight (pounds: `lightestWeight` and half
/// `weightVariance`), and whether it sees in the dark, from its Java and
/// what it extends. `scripts/creation-art.py` prints this table again
/// from a new snapshot.
const SIZES: &[(&str, i32, i32, bool)] = &[
    ("aarakocran", 65, 200, false),
    ("beastkin", 74, 175, true),
    ("centaur", 66, 1350, true),
    ("drow", 65, 120, true),
    ("duergar", 56, 200, true),
    ("dwarf", 56, 200, true),
    ("elf", 65, 120, true),
    ("faerie", 21, 12, false),
    ("ghast", 70, 150, true),
    ("ghost", 70, 150, true),
    ("ghoul", 70, 150, true),
    ("giant", 96, 400, false),
    ("githyanki", 70, 135, true),
    ("gnoll", 76, 220, true),
    ("gnome", 43, 85, true),
    ("goblin", 48, 95, true),
    ("halfelf", 70, 135, true),
    ("halfling", 43, 105, true),
    ("halfogre", 101, 350, false),
    ("human", 74, 175, false),
    ("lich", 70, 150, true),
    ("lizardman", 73, 225, false),
    ("merfolk", 96, 205, true),
    ("mindflayer", 70, 135, true),
    ("mummy", 70, 150, true),
    ("mustie", 48, 72, true),
    ("naga", 65, 120, true),
    ("ogre", 80, 335, false),
    ("orc", 66, 200, true),
    ("pixie", 26, 32, false),
    ("spectre", 70, 150, true),
    ("svirfneblin", 43, 85, true),
    ("tabaxi", 73, 160, true),
    ("tiefling", 69, 190, true),
    ("vampire", 70, 150, true),
    ("zombie", 70, 150, true),
];

/// Width against a human's, in tenths, from pounds an inch of height
/// against a human's (175 pounds, 74 inches): a pixie's slight, a
/// dwarf's stocky. A centaur's horse half is left out.
fn bulk(inches: i32, pounds: i32) -> i32 {
    let against_human = (pounds as f64 / inches as f64) / (175.0 / 74.0);
    ((10.0 * against_human.sqrt()).round() as i32).clamp(6, 15)
}

/// How each player race looks: its skin, hair, clothes and feature; its
/// size from `SIZES`.
fn race(name: &str) -> Build {
    use Feature::*;
    let k = key(name);
    let look: Option<(u8, Option<u8>, u8, Feature)> = match k.as_str() {
        "human" => Some((LIGHTRED, Some(BROWN), BLUE, None)),
        "elf" => Some((LIGHTGRAY, Some(YELLOW), GREEN, PointedEars)),
        "halfelf" => Some((LIGHTRED, Some(BROWN), CYAN, PointedEars)),
        "dwarf" => Some((LIGHTRED, Some(RED), BROWN, BeardAndHelmet)),
        "gnome" => Some((LIGHTRED, Some(WHITE), MAGENTA, TallHat)),
        "halfling" => Some((LIGHTRED, Some(BROWN), GREEN, BareFeet)),
        "halfogre" => Some((GREEN, Option::None, BROWN, Tusks)),
        "pixie" => Some((LIGHTRED, Some(LIGHTGREEN), GREEN, Wings(LIGHTCYAN))),
        "faerie" => Some((LIGHTRED, Some(LIGHTMAGENTA), MAGENTA, Wings(YELLOW))),
        "aarakocran" => Some((BROWN, Some(YELLOW), BLUE, Wings(BROWN))),
        "tiefling" => Some((RED, Some(DARKGRAY), MAGENTA, Wings(RED))),
        "drow" => Some((DARKGRAY, Some(WHITE), MAGENTA, PointedEars)),
        "duergar" => Some((LIGHTGRAY, Some(DARKGRAY), BROWN, BeardAndHelmet)),
        "svirfneblin" => Some((LIGHTGRAY, Option::None, BROWN, None)),
        "goblin" => Some((GREEN, Option::None, RED, PointedEars)),
        "orc" => Some((GREEN, Some(DARKGRAY), BROWN, Tusks)),
        "ogre" => Some((BROWN, Option::None, RED, Tusks)),
        "gnoll" => Some((BROWN, Some(YELLOW), RED, None)),
        "giant" => Some((LIGHTRED, Some(BROWN), CYAN, None)),
        "lizardman" => Some((GREEN, Option::None, BROWN, None)),
        "merfolk" => Some((CYAN, Some(GREEN), BLUE, None)),
        "mindflayer" => Some((MAGENTA, Option::None, RED, None)),
        "naga" => Some((GREEN, Option::None, YELLOW, None)),
        "tabaxi" => Some((YELLOW, Some(BROWN), GREEN, None)),
        "beastkin" => Some((BROWN, Some(BROWN), GREEN, None)),
        "centaur" => Some((LIGHTRED, Some(BROWN), BROWN, None)),
        "githyanki" => Some((YELLOW, Some(DARKGRAY), RED, PointedEars)),
        "mustie" => Some((BROWN, Some(BROWN), GREEN, None)),
        "lich" => Some((WHITE, Option::None, MAGENTA, None)),
        "vampire" => Some((WHITE, Some(DARKGRAY), RED, None)),
        "mummy" => Some((WHITE, Option::None, WHITE, None)),
        "zombie" => Some((GREEN, Option::None, BROWN, None)),
        "ghast" | "ghoul" => Some((LIGHTGRAY, Option::None, BROWN, None)),
        "ghost" | "spectre" => Some((LIGHTCYAN, Option::None, LIGHTGRAY, None)),
        _ => Option::None,
    };
    let size = SIZES.iter().find(|(n, ..)| *n == k).map(|&(_, inches, pounds, dark)| (inches, bulk(inches, pounds), dark));
    // Made from the name: the same each time.
    let mut rng = Rng::new(seed("race", &k, 0));
    let (inches, bulk, infravision) = size.unwrap_or_else(|| (rng.between(40, 96), rng.between(8, 14), rng.chance(50)));
    let (skin, hair, clothes, feature) = look.unwrap_or_else(|| {
        let features = [None, PointedEars, TallHat, Tusks];
        let skins = [LIGHTRED, BROWN, GREEN, LIGHTGRAY, CYAN];
        let clothes = [BLUE, GREEN, BROWN, MAGENTA, RED, CYAN];
        (
            skins[rng.below(skins.len() as i32) as usize],
            Some([BROWN, YELLOW, WHITE, RED, DARKGRAY][rng.below(5) as usize]),
            clothes[rng.below(clothes.len() as i32) as usize],
            features[rng.below(features.len() as i32) as usize],
        )
    });
    Build { inches, bulk, skin, hair, clothes, feature, infravision }
}

/// What a class wears and carries.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Warrior,
    Caster,
    Rogue,
    Priest,
    Nature,
    Bard,
    Monk,
    Crafter,
}

/// CoffeeMUD's classes (`CharClasses/*.java`), each its kind and colors:
/// its clothes, then its trim (a robe's hem, a staff's glow, a symbol).
fn class(name: &str) -> (Kind, u8, u8) {
    use Kind::*;
    match key(name).as_str() {
        "fighter" => (Warrior, LIGHTGRAY, WHITE),
        "barbarian" => (Warrior, BROWN, LIGHTGRAY),
        "paladin" => (Warrior, WHITE, YELLOW),
        "cavalier" => (Warrior, LIGHTBLUE, YELLOW),
        "templar" => (Warrior, DARKGRAY, LIGHTRED),
        "ranger" => (Nature, GREEN, BROWN),
        "mage" | "wizard" => (Caster, BLUE, LIGHTCYAN),
        "abjurer" => (Caster, CYAN, WHITE),
        "alterer" | "transmuter" => (Caster, MAGENTA, LIGHTMAGENTA),
        "conjurer" => (Caster, BLUE, YELLOW),
        "diviner" | "skywatcher" | "oracle" => (Caster, LIGHTBLUE, WHITE),
        "enchanter" | "illusionist" => (Caster, MAGENTA, LIGHTCYAN),
        "evoker" => (Caster, RED, YELLOW),
        "arcanist" => (Caster, DARKGRAY, LIGHTCYAN),
        "necromancer" => (Caster, DARKGRAY, LIGHTGREEN),
        "thief" | "burglar" => (Rogue, DARKGRAY, LIGHTGRAY),
        "assassin" => (Rogue, DARKGRAY, RED),
        "charlatan" => (Rogue, MAGENTA, YELLOW),
        "trapper" | "delver" => (Rogue, BROWN, LIGHTGRAY),
        "pirate" => (Rogue, RED, WHITE),
        "cleric" | "healer" | "missionary" | "reliquist" => (Priest, WHITE, YELLOW),
        "purist" => (Priest, LIGHTGRAY, YELLOW),
        "doomsayer" => (Priest, DARKGRAY, RED),
        "druid" | "gaian" | "shaman" | "beastmaster" | "mer" => (Nature, GREEN, LIGHTGREEN),
        "bard" | "minstrel" | "prancer" => (Bard, MAGENTA, YELLOW),
        "jester" => (Bard, RED, YELLOW),
        "gypsy" => (Bard, RED, YELLOW),
        "monk" => (Monk, BROWN, YELLOW),
        "artisan" | "apprentice" | "gaoler" => (Crafter, BROWN, LIGHTGRAY),
        "scholar" => (Crafter, BLUE, WHITE),
        "sailor" => (Crafter, BLUE, WHITE),
        other => {
            let mut rng = Rng::new(seed("class", other, 0));
            let kinds = [Warrior, Caster, Rogue, Priest, Nature, Bard, Monk, Crafter];
            let colors = [BLUE, GREEN, BROWN, MAGENTA, RED, CYAN, DARKGRAY, LIGHTGRAY];
            (kinds[rng.below(8) as usize], colors[rng.below(8) as usize], [YELLOW, WHITE, LIGHTCYAN, LIGHTGREEN][rng.below(4) as usize])
        }
    }
}

/// Letters only, lower case: "Half Elf" is "halfelf".
fn key(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphabetic).map(|c| c.to_ascii_lowercase()).collect()
}

/// Where a drawn figure's parts are, for what's added to it.
struct Body {
    cx: i32,
    head_top: i32,
    head_bottom: i32,
    head_half: i32,
    /// The shoulders' row and half their width.
    shoulders: i32,
    half: i32,
    hips: i32,
    feet: i32,
    /// Each hand, left then right.
    hands: [(i32, i32); 2],
}

/// A figure's colors: its skin, its clothes, its legs (breeches, a robe, bare).
struct Colors {
    skin: u8,
    clothes: u8,
    legs: u8,
}

/// A person `tall` pixels high and `bulk` tenths of a human's width,
/// standing on `ground` at `cx`.
fn figure(canvas: &mut Canvas, (cx, ground): (i32, i32), tall: i32, bulk: i32, Colors { skin, clothes, legs }: Colors) -> Body {
    let tall = tall.max(5);
    let head = (tall / 6).max(2);
    let head_half = ((head * bulk + 9) / 20).max(1);
    let head_top = ground - tall;
    let head_bottom = head_top + head;
    let half = ((tall * bulk) / 50).max(head_half + 1);
    let shoulders = head_bottom + 1;
    let hips = head_top + tall * 11 / 20;
    let feet = ground;
    // Neck, head (its corners round), the torso narrowing to the hips.
    canvas.rect(cx - head_half, head_top + 1, cx + head_half + 1, head_bottom, skin);
    canvas.rect(cx - head_half + 1, head_top, cx + head_half, head_top + 1, skin);
    canvas.set(cx, head_bottom, skin);
    for y in shoulders..hips {
        let narrowing = (half - (half * 3 / 4)) * (y - shoulders) / (hips - shoulders).max(1);
        canvas.rect(cx - half + narrowing, y, cx + half - narrowing + 1, y + 1, clothes);
    }
    // Legs, a gap between.
    let hip_half = (half * 3 / 4).max(1);
    canvas.rect(cx - hip_half, hips, cx, feet, legs);
    canvas.rect(cx + 1, hips, cx + hip_half + 1, feet, legs);
    // Arms down the sides, the hands skin.
    let arm = (half / 3).max(1);
    let hand_y = hips + (feet - hips) / 4;
    for x0 in [cx - half - arm, cx + half + 1] {
        canvas.rect(x0, shoulders, x0 + arm, hand_y, clothes);
        canvas.rect(x0, hand_y, x0 + arm, hand_y + 1, skin);
    }
    Body {
        cx,
        head_top,
        head_bottom,
        head_half,
        shoulders,
        half,
        hips,
        feet,
        hands: [(cx - half - arm, hand_y), (cx + half + arm, hand_y)],
    }
}

/// Two eyes on a head, glowing red for a race that sees in the dark.
fn eyes(canvas: &mut Canvas, b: &Body, glow: bool) {
    let y = b.head_top + (b.head_bottom - b.head_top) / 2;
    let gap = (b.head_half / 2).max(1);
    for x in [b.cx - gap, b.cx + gap] {
        if glow {
            canvas.light(x, y, LIGHTRED);
        } else {
            canvas.set(x, y, BLACK);
        }
    }
}

/// Black behind, so every color of clothes shows; the ground below.
fn backdrop(canvas: &mut Canvas, ground: i32) {
    canvas.rect(0, ground, canvas.w, canvas.h, BROWN);
    for x in (0..canvas.w).step_by(2) {
        canvas.set(x, ground, GREEN);
    }
}

/// A race's portrait: itself beside a stick marked every foot up to six.
fn paint_race(canvas: &mut Canvas, name: &str) {
    let build = race(name);
    let ground = canvas.h - (canvas.h / 10).max(2);
    backdrop(canvas, ground);
    // Six feet is most of the picture's height above the ground, so the
    // tallest (a half ogre, over eight) still fits.
    let foot = ((ground - 1) * 12 / 101).max(1);
    let stick = canvas.w / 6;
    canvas.rect(stick, ground - foot * 6, stick + 1, ground, DARKGRAY);
    for n in 1..=6 {
        canvas.rect(stick - 1, ground - foot * n, stick + 2, ground - foot * n + 1, LIGHTGRAY);
    }
    let tall = (build.inches * foot / 12).min(ground - 1);
    let cx = canvas.w * 9 / 16;
    let legs = if build.feature == Feature::Tusks { build.skin } else { DARKGRAY };
    if let Feature::Wings(color) = build.feature {
        // Behind the figure: from the shoulders out and up, then down to the hips.
        let top = ground - tall;
        let span = (tall / 2).max(2);
        let half = ((tall * build.bulk) / 50).max(2);
        for i in 0..span {
            let reach = half + 1 + (span - i) * 2 / 3;
            let y = top + tall / 6 + i;
            canvas.rect(cx - reach - 1, y, cx - half, y + 1, color);
            canvas.rect(cx + half + 1, y, cx + reach + 2, y + 1, color);
        }
    }
    let b = figure(canvas, (cx, ground), tall, build.bulk, Colors { skin: build.skin, clothes: build.clothes, legs });
    if let Some(hair) = build.hair {
        canvas.rect(cx - b.head_half, b.head_top, cx + b.head_half + 1, b.head_top + 1, hair);
    }
    eyes(canvas, &b, build.infravision);
    match build.feature {
        Feature::PointedEars => {
            let y = b.head_top + (b.head_bottom - b.head_top) / 2;
            canvas.set(cx - b.head_half - 1, y - 1, build.skin);
            canvas.set(cx + b.head_half + 1, y - 1, build.skin);
        }
        Feature::BeardAndHelmet => {
            canvas.rect(cx - b.head_half, b.head_top, cx + b.head_half + 1, b.head_top + 1, LIGHTGRAY);
            canvas.set(cx, b.head_top - 1, LIGHTGRAY);
            let beard = build.hair.unwrap_or(BROWN);
            canvas.rect(cx - b.head_half, b.head_bottom - 1, cx + b.head_half + 1, b.shoulders + (b.hips - b.shoulders) / 2, beard);
        }
        Feature::TallHat => {
            for i in 0..(b.head_bottom - b.head_top).max(2) {
                let w = (b.head_half - i / 2).max(0);
                canvas.rect(cx - w, b.head_top - 1 - i, cx + w + 1, b.head_top - i, RED);
            }
            canvas.rect(cx - b.head_half, b.head_bottom - 1, cx + b.head_half + 1, b.head_bottom, WHITE);
        }
        Feature::BareFeet => {
            canvas.rect(cx - b.half, b.feet - 1, cx + b.half + 1, b.feet, build.skin);
        }
        Feature::Tusks => {
            canvas.set(cx - 1, b.head_bottom - 1, WHITE);
            canvas.set(cx + 1, b.head_bottom - 1, WHITE);
            canvas.rect(cx - b.half, b.hips - 1, cx + b.half + 1, b.hips + 2, BROWN);
        }
        Feature::Wings(_) | Feature::None => {}
    }
}

/// A class's portrait: a person in its clothes, with its tools.
fn paint_class(canvas: &mut Canvas, name: &str) {
    let (kind, clothes, trim) = class(name);
    let ground = canvas.h - (canvas.h / 10).max(2);
    backdrop(canvas, ground);
    let tall = ground - 3;
    let cx = canvas.w / 2;
    let bulk = match kind {
        Kind::Warrior => 12,
        Kind::Caster | Kind::Rogue => 9,
        _ => 10,
    };
    // A robe falls to the feet; the rest wear breeches.
    let robed = matches!(kind, Kind::Caster | Kind::Priest | Kind::Nature | Kind::Monk);
    let b = figure(canvas, (cx, ground), tall, bulk, Colors { skin: LIGHTRED, clothes, legs: if robed { clothes } else { DARKGRAY } });
    if robed {
        canvas.rect(cx - b.half, b.hips, cx + b.half + 1, b.feet, clothes);
        canvas.rect(cx - b.half, b.feet - 1, cx + b.half + 1, b.feet, trim);
    }
    eyes(canvas, &b, false);
    let [(lx, ly), (rx, ry)] = b.hands;
    let reach = (b.feet - b.head_top) / 2;
    match kind {
        Kind::Warrior => {
            // A helmet, a belt, a sword raised in the right hand.
            canvas.rect(cx - b.head_half, b.head_top, cx + b.head_half + 1, b.head_top + 2, LIGHTGRAY);
            canvas.rect(cx - b.half, b.hips - 1, cx + b.half + 1, b.hips, BROWN);
            canvas.rect(rx, ry - reach, rx + 1, ry, WHITE);
            canvas.rect(rx - 1, ry - 1, rx + 2, ry, YELLOW);
            // And a shield on the left arm, its device in the trim.
            let s = (b.half / 2 + 1).max(2);
            canvas.rect(lx - s, ly - s * 2, lx + 1, ly + 1, clothes);
            canvas.set(lx - s / 2, ly - s, trim);
        }
        Kind::Caster => {
            // A pointed hat, a staff with a glowing head.
            for i in 0..(b.head_bottom - b.head_top + 2) {
                let w = (b.head_half + 1 - i / 2).max(0);
                canvas.rect(cx - w, b.head_top - i, cx + w + 1, b.head_top - i + 1, clothes);
            }
            canvas.rect(rx, ry - reach, rx + 1, b.feet, BROWN);
            canvas.light(rx, ry - reach - 1, trim);
            canvas.light(rx - 1, ry - reach, trim);
            canvas.light(rx + 1, ry - reach, trim);
        }
        Kind::Rogue => {
            // A hood over the head, a dagger low in the right hand.
            canvas.rect(cx - b.head_half - 1, b.head_top - 1, cx + b.head_half + 2, b.head_top + 1, clothes);
            canvas.rect(cx - b.head_half - 1, b.head_top, cx - b.head_half, b.head_bottom, clothes);
            canvas.rect(cx + b.head_half + 1, b.head_top, cx + b.head_half + 2, b.head_bottom, clothes);
            canvas.rect(rx, ry + 1, rx + 1, ry + 1 + (reach / 3).max(2), trim);
        }
        Kind::Priest => {
            // A holy symbol on the chest, a mace in the right hand.
            let (sx, sy) = (cx, b.shoulders + (b.hips - b.shoulders) / 3);
            canvas.light(sx, sy - 1, trim);
            canvas.light(sx, sy, trim);
            canvas.light(sx, sy + 1, trim);
            canvas.light(sx - 1, sy, trim);
            canvas.light(sx + 1, sy, trim);
            canvas.rect(rx, ry - reach / 2, rx + 1, ry, BROWN);
            canvas.rect(rx - 1, ry - reach / 2 - 1, rx + 2, ry - reach / 2 + 1, LIGHTGRAY);
        }
        Kind::Nature => {
            // A hood of leaves, a staff that's still growing.
            canvas.rect(cx - b.head_half, b.head_top, cx + b.head_half + 1, b.head_top + 1, trim);
            canvas.rect(rx, ry - reach, rx + 1, b.feet, BROWN);
            for (dx, dy) in [(-1, 0), (1, -1), (-1, -2), (1, 1)] {
                canvas.set(rx + dx, ry - reach + dy, trim);
            }
        }
        Kind::Bard => {
            // A feathered cap, a lute across the body.
            canvas.rect(cx - b.head_half, b.head_top, cx + b.head_half + 1, b.head_top + 1, clothes);
            canvas.set(cx + b.head_half, b.head_top - 1, trim);
            let (lcx, lcy) = (cx + b.half / 2, b.hips - 1);
            canvas.oval(lcx, lcy, (b.half / 2).max(1), (b.half / 2).max(1), BROWN);
            canvas.set(lcx, lcy, BLACK);
            canvas.rect(lcx - b.half, lcy - b.half, lcx - b.half + 1, lcy, BROWN);
            canvas.set(lcx - b.half, lcy - b.half - 1, trim);
        }
        Kind::Monk => {
            // Bald, a sash, the hands bare and ready.
            canvas.rect(cx - b.half, b.hips - 1, cx + b.half + 1, b.hips, trim);
            canvas.rect(lx, ly - 2, lx + 1, ly, LIGHTRED);
            canvas.rect(rx, ry - 2, rx + 1, ry, LIGHTRED);
        }
        Kind::Crafter => {
            // An apron and a hammer (or a book, a rope: the trim's color).
            canvas.rect(cx - b.half / 2, b.shoulders + 2, cx + b.half / 2 + 1, b.feet - 1, trim);
            canvas.rect(rx, ry - reach / 3, rx + 1, ry + 1, BROWN);
            canvas.rect(rx - 1, ry - reach / 3 - 1, rx + 2, ry - reach / 3 + 1, DARKGRAY);
        }
    }
}

/// Paints a race's or a class's portrait at a size in cells. `kind` is
/// `race` or `class`; the name is the game's ("Half Elf", "Fighter").
pub fn portrait(kind: &str, name: &str, columns: usize, rows: usize) -> Art {
    let (columns, rows) = (columns.clamp(LEAST.0, MOST.0), rows.clamp(LEAST.1, MOST.1));
    let mut canvas = Canvas::new(columns, rows);
    if kind == "class" {
        paint_class(&mut canvas, name);
    } else {
        paint_race(&mut canvas, name);
    }
    canvas.art()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Color;

    const RACES: [&str; 7] = ["Human", "Elf", "Half Elf", "Dwarf", "Gnome", "Halfling", "Half Ogre"];
    const CLASSES: [&str; 12] = ["Fighter", "Mage", "Thief", "Cleric", "Druid", "Bard", "Monk", "Artisan", "Paladin", "Necromancer", "Ranger", "Jester"];

    /// How many pixel rows from the top the first non-sky pixel is in the figure's column.
    fn colors(art: &Art) -> Vec<u8> {
        art.lines
            .iter()
            .flat_map(|line| line.iter())
            .flat_map(|span| [span.style.fg, span.style.bg])
            .filter_map(|c| match c {
                Some(Color::Index { index }) => Some(index),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_race_and_class_paints_at_every_size() {
        for (columns, rows) in [(8, 4), (32, 16), (40, 16), (80, 40), (1, 1), (500, 500)] {
            for name in RACES.iter().chain(&["Something New"]) {
                let art = portrait("race", name, columns, rows);
                assert_eq!(art.lines.len(), art.rows);
                assert!(art.columns >= LEAST.0 && art.rows >= LEAST.1);
            }
            for name in CLASSES.iter().chain(&["Something New"]) {
                portrait("class", name, columns, rows);
            }
        }
    }

    #[test]
    fn a_portrait_is_the_same_each_time_and_another_differs() {
        assert_eq!(portrait("race", "Elf", 32, 16), portrait("race", "Elf", 32, 16));
        assert_ne!(portrait("race", "Elf", 32, 16), portrait("race", "Dwarf", 32, 16));
        assert_ne!(portrait("class", "Fighter", 32, 16), portrait("class", "Mage", 32, 16));
        assert_eq!(portrait("race", "Unknownling", 32, 16), portrait("race", "unknownling", 32, 16));
    }

    #[test]
    fn a_half_ogre_stands_taller_than_a_gnome() {
        // The figure's top: the first row (from the top) its skin shows in.
        let top = |name: &str| {
            let b = race(name);
            let mut canvas = Canvas::new(32, 16);
            paint_race(&mut canvas, name);
            (0..canvas.h).find(|&y| (0..canvas.w).any(|x| canvas.get(x, y) == b.skin)).unwrap()
        };
        assert!(top("Half Ogre") < top("Human"));
        assert!(top("Human") < top("Gnome"));
        assert!(top("Gnome") < top("Pixie"));
    }

    #[test]
    fn every_player_race_has_its_size_and_a_look() {
        for (name, inches, ..) in SIZES {
            assert_eq!(race(name).inches, *inches, "{name}");
        }
        assert!(race("Pixie").inches < 30 && race("Pixie").bulk < race("Human").bulk);
    }

    #[test]
    fn races_that_see_in_the_dark_have_red_eyes() {
        // An elf's skin is pale: red is only its eyes.
        assert!(colors(&portrait("race", "Elf", 32, 16)).contains(&LIGHTRED));
        assert!(!colors(&portrait("race", "Half Ogre", 32, 16)).contains(&LIGHTRED));
    }

    #[test]
    fn only_the_fonts_characters() {
        let font = crate::ansi_art::CP437;
        for name in RACES {
            for line in portrait("race", name, 32, 16).lines {
                for span in line {
                    assert!(span.text.chars().all(|c| font.contains(c)), "{name}: {:?}", span.text);
                }
            }
        }
    }
}

#[cfg(test)]
mod look {
    use super::*;
    use crate::ansi::Color;

    /// Writes the races and some classes as a PPM to `OUT`, side by
    /// side, to look at while tuning: `OUT=/tmp/p.ppm cargo test portrait::look -- --ignored`.
    #[test]
    #[ignore]
    fn show() {
        const VGA: [[u8; 3]; 16] = [[0, 0, 0], [170, 0, 0], [0, 170, 0], [170, 85, 0], [0, 0, 170], [170, 0, 170], [0, 170, 170], [170, 170, 170], [85, 85, 85], [255, 85, 85], [85, 255, 85], [255, 255, 85], [85, 85, 255], [255, 85, 255], [85, 255, 255], [255, 255, 255]];
        let arts: Vec<Art> = ["Human", "Elf", "Half Elf", "Dwarf", "Gnome", "Halfling", "Half Ogre", "Pixie", "Faerie", "Drow", "Duergar", "Orc", "Tiefling", "Aarakocran"].iter().map(|n| portrait("race", n, 32, 16))
            .chain(["Fighter", "Mage", "Thief", "Cleric", "Druid", "Bard", "Monk", "Artisan", "Paladin", "Necromancer"].iter().map(|n| portrait("class", n, 32, 16)))
            .collect();
        let (w, h) = (arts.len() * 33, 32);
        let mut px = vec![40u8; w * h * 3];
        for (i, art) in arts.iter().enumerate() {
            for (row, line) in art.lines.iter().enumerate() {
                let mut x = 0;
                for span in line {
                    let idx = |c: Option<Color>| match c { Some(Color::Index { index }) => index as usize, _ => 0 };
                    let (fg, bg) = (idx(span.style.fg), idx(span.style.bg));
                    for ch in span.text.chars() {
                        let (top, bottom) = if ch == '▀' { (fg, bg) } else { (bg, bg) };
                        for (dy, c) in [(0, top), (1, bottom)] {
                            let at = ((row * 2 + dy) * w + i * 33 + x) * 3;
                            px[at..at + 3].copy_from_slice(&VGA[c]);
                        }
                        x += 1;
                    }
                }
            }
        }
        let mut out = format!("P6 {w} {h} 255\n").into_bytes();
        out.extend(px);
        std::fs::write(std::env::var("OUT").unwrap_or_else(|_| "portraits.ppm".into()), out).unwrap();
    }
}

//! **Faces** by Coupler's painter (`painter.rs`): a close-up of a
//! creature in the Bestiary, 25 by 24 square pixels (25 cells across, 12
//! down, two pixels a cell: 200 by 192 on the screen, nothing cut off):
//! the head and the shoulders, in the 16 VGA colors.
//! Pure and unit-tested. All ART is Coupler's painter's first (CLAUDE.md
//! rule 11).
//!
//! **What it reads**: all the Bestiary knows of the creature
//! (`bestiary::MobEntry`). Its race's category picks the head's shape: a
//! person's (with the race's own skin, hair, ears, tusks or wings from
//! `portrait::race`), a beast's muzzle and ears (canine, feline, ursine,
//! rodent, equine, bovine…), a bird's beak, a reptile's or a fish's
//! scales, an insect's or a spider's eyes, a skull (the undead but the
//! fleshed ones), or a being of stuff (elementals, golems, slimes,
//! plants). Its description and its name give the colors ("long red
//! hair", "a black wolf", "pale skin", "blue eyes") and what's on the
//! face (a beard, a scar, an eye patch, freckles, horns, tusks, fangs,
//! glowing eyes, a halo, old age). What it wears on its head, neck,
//! ears, eyes, mouth, torso and back is drawn, in the item's colors and
//! metals ("a steel helmet", "a gold amulet", "a black hood").
//!
//! The same creature always gets the same face (its name seeds the
//! rest: the backdrop, a man's beard, the length of the hair); learning
//! more of it changes the face to match.

use crate::ansi_art::Art;
use crate::bestiary::MobEntry;
use crate::paint::seed;
use crate::painter::*;
use crate::portrait::{self, Feature};

/// A picture's size in cells (the faces' and the items', `item_art.rs`),
/// and the square of pixels drawn in it.
pub const SIZE: (usize, usize) = (25, 12);
pub(crate) const PIXELS: (i32, i32) = (25, 24);

/// What the painter needs of a creature.
#[derive(Debug, Clone, Default)]
pub struct Sitter {
    pub name: String,
    pub race: Option<String>,
    pub category: Option<String>,
    pub sex: Option<String>,
    pub description: Option<String>,
    /// "has 2 leg(s), 2 arm(s), one eye and a tail": what a long look says.
    pub body: Option<String>,
    /// What it wears: where ("worn on head") and what ("a steel helmet").
    pub wears: Vec<(String, String)>,
}

impl From<&MobEntry> for Sitter {
    fn from(m: &MobEntry) -> Sitter {
        Sitter {
            name: m.name.clone(),
            race: m.race.clone(),
            category: m.category.clone(),
            sex: m.sex.clone(),
            description: m.description.clone(),
            body: m.body.clone(),
            wears: m.wears.iter().map(|c| (c.what.clone().unwrap_or_default(), c.name.clone())).collect(),
        }
    }
}

/// The head's shape, from the race's category.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Person,
    /// A muzzle, and its ears.
    Beast(Ears),
    Bird,
    /// Scales: a lizard, a snake, a frog, a fish, a dragon.
    Scaled(Scales),
    Bug(Bug),
    Skull,
    /// A being of some stuff: fire, water, stone, a slime, a plant.
    Stuff(Stuff),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Ears {
    Pointed,
    Round,
    Long,
    Horned,
    Curled,
    Wide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Scales {
    Lizard,
    Snake,
    Frog,
    Fish,
    Dragon,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Bug {
    Insect,
    Spider,
    Worm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Stuff {
    Fire,
    Water,
    Air,
    Earth,
    Golem,
    Slime,
    Plant,
}

/// Letters only, lower case, as `portrait.rs` keys a race.
pub(crate) fn key(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphabetic).map(|c| c.to_ascii_lowercase()).collect()
}

/// The shape a race's category (`races.tsv`) and name give.
fn shape(category: &str, race: &str) -> Shape {
    let race = key(race);
    let has = |w: &str| race.contains(w);
    match key(category).as_str() {
        "canine" | "mustelid" | "feline" => Shape::Beast(Ears::Pointed),
        "ursine" | "rodent" | "primate" | "seamammal" | "animal" | "pachyderm" => Shape::Beast(if has("elephant") || has("mammoth") { Ears::Wide } else { Ears::Round }),
        "pteropine" => Shape::Beast(Ears::Wide),
        "equine" => Shape::Beast(Ears::Long),
        "bovine" | "porcine" => Shape::Beast(Ears::Horned),
        "ovine" => Shape::Beast(Ears::Curled),
        "avian" => Shape::Bird,
        "reptile" => Shape::Scaled(Scales::Lizard),
        "ophidian" => Shape::Scaled(Scales::Snake),
        "amphibian" => Shape::Scaled(Scales::Frog),
        "fish" => Shape::Scaled(Scales::Fish),
        "dragon" => Shape::Scaled(Scales::Dragon),
        "insect" => Shape::Bug(Bug::Insect),
        "arachnid" => Shape::Bug(Bug::Spider),
        "worm" => Shape::Bug(Bug::Worm),
        "undead" if !(has("zombie") || has("ghoul") || has("ghast") || has("vampire") || has("ghost") || has("spectre") || has("wraith") || has("banshee")) => Shape::Skull,
        "elemental" => Shape::Stuff(if has("fire") {
            Stuff::Fire
        } else if has("water") {
            Stuff::Water
        } else if has("air") {
            Stuff::Air
        } else {
            Stuff::Earth
        }),
        "stonegolem" | "woodgolem" | "metalgolem" | "golem" => Shape::Stuff(Stuff::Golem),
        "slime" => Shape::Stuff(Stuff::Slime),
        "vegetation" => Shape::Stuff(Stuff::Plant),
        _ => Shape::Person,
    }
}

/// A color word and the color it is; the second is for eyes and glows,
/// which want the bright half.
pub(crate) fn color_word(word: &str) -> Option<(u8, u8)> {
    Some(match word {
        "black" | "jet" | "ebony" | "raven" | "dark" | "inky" => (DARKGRAY, DARKGRAY),
        "white" | "snowy" | "ivory" | "pearly" | "bone" => (WHITE, WHITE),
        "grey" | "gray" | "silver" | "silvery" | "ashen" | "grizzled" | "steel" | "iron" | "mithril" => (LIGHTGRAY, LIGHTGRAY),
        "red" | "crimson" | "scarlet" | "ruby" | "auburn" | "ginger" | "bloody" => (RED, LIGHTRED),
        "orange" | "copper" | "bronze" | "rust" | "rusty" | "brown" | "chestnut" | "tan" | "tawny" | "brunette" | "hazel" | "leather" | "wooden" | "wood" => (BROWN, BROWN),
        "blond" | "blonde" | "golden" | "gold" | "yellow" | "flaxen" | "sandy" | "straw" | "amber" | "brass" => (YELLOW, YELLOW),
        "green" | "emerald" | "jade" | "mossy" => (GREEN, LIGHTGREEN),
        "blue" | "azure" | "sapphire" | "cobalt" => (BLUE, LIGHTBLUE),
        "purple" | "violet" | "lavender" | "mauve" | "amethyst" => (MAGENTA, LIGHTMAGENTA),
        "pink" | "rosy" => (LIGHTMAGENTA, LIGHTMAGENTA),
        "teal" | "turquoise" | "aqua" | "cyan" => (CYAN, LIGHTCYAN),
        _ => return None,
    })
}

/// The words of a text, lower case, hyphens and punctuation cut.
pub(crate) fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// What the description and the name say.
struct Reading {
    words: Vec<String>,
}

impl Reading {
    fn has(&self, any: &[&str]) -> bool {
        self.words.iter().any(|w| any.iter().any(|a| w == a))
    }

    /// Two words one after the other ("long hair", "one eye").
    fn pair(&self, first: &[&str], second: &[&str]) -> bool {
        self.words.windows(2).any(|w| first.contains(&w[0].as_str()) && second.contains(&w[1].as_str()))
    }

    /// One of `first` up to three words before one of `nouns` ("long, flowing red hair").
    fn near(&self, first: &[&str], nouns: &[&str]) -> bool {
        self.words.iter().enumerate().any(|(i, w)| nouns.contains(&w.as_str()) && (1..=3.min(i)).any(|back| first.contains(&self.words[i - back].as_str())))
    }

    /// The color said of one of `nouns` (up to three words before it:
    /// "long, flowing red hair"; or "red-haired"), the first such.
    fn color_of(&self, nouns: &[&str], bright: bool) -> Option<u8> {
        for (i, w) in self.words.iter().enumerate() {
            if !nouns.contains(&w.as_str()) {
                continue;
            }
            for back in 1..=3.min(i) {
                if let Some((plain, light)) = color_word(&self.words[i - back]) {
                    return Some(if bright { light } else { plain });
                }
            }
        }
        None
    }
}

const HAIR: &[&str] = &["hair", "haired", "beard", "bearded", "mane", "locks", "curls", "braids", "tresses", "mustache", "moustache"];
const SKIN: &[&str] = &["skin", "skinned", "complexion", "flesh", "face"];
const EYES: &[&str] = &["eyes", "eye", "eyed"];
const COAT: &[&str] = &["fur", "furred", "coat", "pelt", "feathers", "feathered", "plumage", "scales", "scaled", "hide", "carapace", "shell", "body", "wool", "bark", "leaves"];

/// The first color named in an item's name: "a black wool hood", "a steel helm".
pub(crate) fn item_color(item: &str) -> Option<u8> {
    let ws = words(item);
    if ws.iter().any(|w| ["plate", "chain", "chainmail", "mail", "steel", "iron", "metal", "mithril", "silver", "scale"].contains(&w.as_str())) && !ws.iter().any(|w| color_word(w).is_some_and(|(c, _)| c != LIGHTGRAY)) {
        return Some(LIGHTGRAY);
    }
    ws.iter().find_map(|w| color_word(w).map(|(c, _)| c))
}

/// A darker shade of a color, for a nose, a shadow, a brow.
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

/// A lighter one, for a muzzle, a chest, a highlight.
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

/// What it wears, by where.
#[derive(Default)]
struct Worn {
    head: Option<String>,
    neck: Option<String>,
    ears: Option<String>,
    eyes: Option<String>,
    mouth: Option<String>,
    torso: Option<String>,
    back: Option<String>,
}

fn worn(wears: &[(String, String)], said: &Reading) -> Worn {
    let mut w = Worn::default();
    for (place, item) in wears {
        let place = place.to_lowercase();
        let slot = if place.contains("head") {
            &mut w.head
        } else if place.contains("neck") {
            &mut w.neck
        } else if place.contains("ear") {
            &mut w.ears
        } else if place.contains("eye") {
            &mut w.eyes
        } else if place.contains("mouth") {
            &mut w.mouth
        } else if place.contains("torso") {
            &mut w.torso
        } else if place.contains("back") || place.contains("about body") {
            &mut w.back
        } else {
            continue;
        };
        slot.get_or_insert_with(|| item.to_lowercase());
    }
    // What the description has it wear, where nothing's known worn.
    let described = |things: &[&str]| things.iter().find(|t| said.has(&[t])).map(|t| t.to_string());
    if w.head.is_none() {
        w.head = described(&["helmet", "helm", "hood", "cowl", "crown", "tiara", "circlet", "hat", "cap", "turban", "headband", "bandana"]);
    }
    if w.neck.is_none() {
        w.neck = described(&["amulet", "necklace", "pendant", "medallion", "torc", "collar", "scarf"]);
    }
    if w.ears.is_none() {
        w.ears = described(&["earring", "earrings"]);
    }
    if w.eyes.is_none() {
        w.eyes = described(&["spectacles", "glasses", "goggles", "monocle", "blindfold"]);
    }
    if w.back.is_none() {
        w.back = described(&["cloak", "cape", "mantle"]);
    }
    w
}

/// Everything about a face that the painting follows.
struct Look {
    shape: Shape,
    skin: u8,
    hair: Option<u8>,
    eyes: u8,
    glow: bool,
    clothes: u8,
    feature: Feature,
    beard: bool,
    mustache: bool,
    long_hair: bool,
    scar: bool,
    patch: bool,
    freckles: bool,
    old: bool,
    horns: bool,
    tusks: bool,
    fangs: bool,
    halo: bool,
    tattoo: bool,
    backdrop: u8,
    worn: Worn,
}

fn read(s: &Sitter) -> Look {
    let race = s.race.clone().unwrap_or_default();
    let category = s.category.clone().unwrap_or_default();
    let mut text = s.description.clone().unwrap_or_default();
    text.push(' ');
    text.push_str(s.body.as_deref().unwrap_or(""));
    let said = Reading { words: words(&text) };
    let named = Reading { words: words(&s.name) };
    let shape = shape(&category, &race);
    let build = portrait::race(if race.is_empty() { &s.name } else { &race });
    let mut rng = Rng::new(seed("face", &key(&s.name), 0));
    let female = s.sex.as_deref().is_some_and(|x| x.eq_ignore_ascii_case("female"));
    let male = s.sex.as_deref().is_some_and(|x| x.eq_ignore_ascii_case("male"));
    // The colors the words give: the description first, the name after
    // ("a black wolf"), then the race's or the shape's own.
    let named_color = named.words.iter().find_map(|w| color_word(w).map(|(c, _)| c));
    let coat = said.color_of(COAT, false).or_else(|| said.color_of(SKIN, false)).or(named_color);
    let shape_skin = match shape {
        Shape::Person => build.skin,
        Shape::Beast(Ears::Pointed) => [BROWN, DARKGRAY, LIGHTGRAY, YELLOW][rng.below(4) as usize],
        Shape::Beast(Ears::Curled) => WHITE,
        Shape::Beast(Ears::Horned) if key(&category) == "porcine" => LIGHTMAGENTA,
        Shape::Beast(Ears::Wide) => DARKGRAY,
        Shape::Beast(_) => BROWN,
        Shape::Bird => [BROWN, WHITE, DARKGRAY, BLUE, RED][rng.below(5) as usize],
        Shape::Scaled(Scales::Fish) => CYAN,
        Shape::Scaled(Scales::Dragon) => RED,
        Shape::Scaled(_) => GREEN,
        Shape::Bug(_) => [BROWN, RED, GREEN][rng.below(3) as usize],
        Shape::Skull => WHITE,
        Shape::Stuff(Stuff::Fire) => RED,
        Shape::Stuff(Stuff::Water) => BLUE,
        Shape::Stuff(Stuff::Air) => LIGHTGRAY,
        Shape::Stuff(Stuff::Earth) => BROWN,
        Shape::Stuff(Stuff::Golem) => {
            let r = key(&race);
            if key(&category).starts_with("wood") || r.contains("wood") {
                BROWN
            } else if key(&category).starts_with("metal") || r.contains("iron") || r.contains("metal") {
                LIGHTGRAY
            } else if r.contains("flesh") {
                LIGHTRED
            } else if r.contains("clay") {
                BROWN
            } else {
                DARKGRAY
            }
        }
        Shape::Stuff(Stuff::Slime) => GREEN,
        Shape::Stuff(Stuff::Plant) => GREEN,
    };
    let skin = match shape {
        Shape::Person => {
            if said.has(&["pale", "pallid", "ghostly", "ashen", "wan"]) {
                LIGHTGRAY
            } else if said.has(&["tanned", "swarthy", "dusky", "bronzed"]) {
                BROWN
            } else {
                said.color_of(SKIN, false).unwrap_or(shape_skin)
            }
        }
        _ => coat.unwrap_or(shape_skin),
    };
    let old = said.has(&["old", "elderly", "aged", "ancient", "wrinkled", "wizened", "grizzled", "venerable"]);
    let bald = said.has(&["bald", "hairless", "shaven", "shaved"]) && !said.pair(&["shaven", "shaved"], &["face", "chin"]);
    let hair = if bald || shape != Shape::Person {
        None
    } else {
        said.color_of(HAIR, false)
            .or_else(|| named.color_of(HAIR, false))
            .or(if old { Some(LIGHTGRAY) } else { None })
            .or(build.hair)
            .or_else(|| (!build.feature.eq(&Feature::Tusks)).then(|| [BROWN, DARKGRAY, YELLOW, RED][rng.below(4) as usize]))
    };
    let clean = said.has(&["beardless"]) || said.pair(&["clean"], &["shaven", "shaved"]) || said.pair(&["shaven", "shaved"], &["face", "chin"]);
    let dwarvish = build.feature == Feature::BeardAndHelmet;
    let beard = shape == Shape::Person && !female && !clean && (said.has(&["beard", "bearded", "whiskered"]) || dwarvish || (male && rng.chance(30)));
    let mustache = shape == Shape::Person && !female && !clean && (beard || said.has(&["mustache", "moustache"]));
    let glow = build.infravision && shape == Shape::Person && !said.has(&["kind", "gentle"])
        || said.pair(&["glowing", "burning", "blazing", "fiery", "luminous", "shining"], EYES)
        || said.pair(EYES, &["glow", "glowing", "burn", "blaze", "shine", "gleam"]);
    let eyes = said.color_of(EYES, true).unwrap_or(match shape {
        Shape::Person => [BLUE, BROWN, GREEN, DARKGRAY][rng.below(4) as usize],
        Shape::Beast(Ears::Pointed) => [YELLOW, LIGHTGREEN, BROWN][rng.below(3) as usize],
        Shape::Scaled(_) => YELLOW,
        Shape::Bug(_) => LIGHTRED,
        Shape::Skull => LIGHTRED,
        Shape::Stuff(Stuff::Fire) => YELLOW,
        Shape::Stuff(Stuff::Golem) => LIGHTCYAN,
        Shape::Stuff(_) => WHITE,
        _ => DARKGRAY,
    });
    let backdrops = [BLUE, MAGENTA, CYAN, GREEN, RED, BROWN, DARKGRAY];
    let mut backdrop = backdrops[rng.below(backdrops.len() as i32) as usize];
    if backdrop == skin || shade(backdrop) == skin {
        backdrop = if skin == BLUE { DARKGRAY } else { BLUE };
    }
    let r = key(&race);
    Look {
        shape,
        skin,
        hair,
        eyes,
        glow,
        clothes: build.clothes,
        feature: build.feature,
        beard,
        mustache,
        long_hair: hair.is_some() && (said.near(&["long", "flowing", "braided", "lustrous"], HAIR) || said.has(&["braid", "braids", "ponytail", "tresses"]) || (female && !said.near(&["short", "cropped"], HAIR) && rng.chance(80))),
        scar: said.has(&["scar", "scars", "scarred", "scarring"]),
        patch: said.has(&["eyepatch"]) || said.pair(&["eye"], &["patch"]) || said.pair(&["one"], &["eye", "eyed"]),
        freckles: said.has(&["freckles", "freckled"]),
        old,
        horns: said.has(&["horn", "horns", "horned"]) || ["tiefling", "demon", "devil", "minotaur", "satyr", "imp"].iter().any(|h| r.contains(h)) || key(&category) == "demon",
        tusks: said.has(&["tusk", "tusks", "tusked"]) || build.feature == Feature::Tusks,
        fangs: said.has(&["fang", "fangs", "fanged"]) || r.contains("vampire"),
        halo: said.has(&["halo"]) || key(&category) == "celestial",
        tattoo: said.has(&["tattoo", "tattoos", "tattooed", "warpaint"]),
        backdrop,
        worn: worn(&s.wears, &said),
    }
}

/// The picture in pixels: 25 by 24, every one of them shown.
pub(crate) const W: i32 = PIXELS.0;
pub(crate) const H: i32 = PIXELS.1;
pub(crate) const CX: i32 = W / 2;

/// A line of pixels from one point to another.
pub(crate) fn line(c: &mut Canvas, (x0, y0): (i32, i32), (x1, y1): (i32, i32), color: u8) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        c.set(x0 + (x1 - x0) * i / steps, y0 + (y1 - y0) * i / steps, color);
    }
}

/// A pixel and its mirror across the picture's middle.
pub(crate) fn both(c: &mut Canvas, x: i32, y: i32, color: u8) {
    c.set(x, y, color);
    c.set(2 * CX - x, y, color);
}

/// The same, lit (eyes that glow).
fn both_lit(c: &mut Canvas, x: i32, y: i32, color: u8) {
    c.light(x, y, color);
    c.light(2 * CX - x, y, color);
}

/// Whether (x, y) is inside an ellipse.
pub(crate) fn within(x: i32, y: i32, (cx, cy, rx, ry): (i32, i32, i32, i32)) -> bool {
    let (dx, dy) = (x - cx, y - cy);
    dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry
}

/// The part of an ellipse from row `y0` to `y1`, filled.
pub(crate) fn cap(c: &mut Canvas, oval: (i32, i32, i32, i32), y0: i32, y1: i32, color: u8) {
    for y in y0..y1 {
        for x in 0..W {
            if within(x, y, oval) {
                c.set(x, y, color);
            }
        }
    }
}

/// A triangle standing on its base: the peak at (x, top), `half` each side at `bottom`.
pub(crate) fn peak(c: &mut Canvas, x: i32, top: i32, bottom: i32, half: i32, color: u8) {
    let tall = (bottom - top).max(1);
    for y in top..=bottom {
        let reach = half * (y - top) / tall;
        c.rect(x - reach, y, x + reach + 1, y + 1, color);
    }
}

/// The backdrop: its color fading to black at the top.
pub(crate) fn backdrop(c: &mut Canvas, color: u8) {
    c.gradient(0, H, &[BLACK, shade(color), color]);
}

/// The person's head: an oval.
const HEAD: (i32, i32, i32, i32) = (CX, 11, 6, 8);

/// Shoulders and the chest, in the clothes, and what's worn there.
fn shoulders(c: &mut Canvas, look: &Look, color: u8) {
    let (clothes, mail) = match look.worn.torso.as_deref() {
        Some(item) => (item_color(item).unwrap_or(color), words(item).iter().any(|w| ["chain", "chainmail", "mail", "ringmail", "scale"].contains(&w.as_str()))),
        None => (color, false),
    };
    c.oval(CX, 25, 12, 4, clothes);
    c.rect(0, 23, W, H, clothes);
    if mail {
        for y in 21..H {
            for x in 0..W {
                if c.get(x, y) == clothes && (x + y) % 3 == 0 {
                    c.set(x, y, shade(clothes));
                }
            }
        }
    }
    if let Some(cloak) = look.worn.back.as_deref() {
        let color = item_color(cloak).unwrap_or(shade(look.clothes));
        for y in 21..H {
            let reach = (y - 20) / 2;
            c.rect(0, y, 1 + reach, y + 1, color);
            c.rect(W - 1 - reach, y, W, y + 1, color);
        }
        c.light(CX - 5, 22, YELLOW);
        c.light(CX + 5, 22, YELLOW);
    }
}

/// Things worn around the neck, from the row under the chin.
fn neck(c: &mut Canvas, item: &str, bottom: i32) {
    let ws = words(item);
    let color = item_color(item).unwrap_or(YELLOW);
    if ws.iter().any(|w| ["collar", "scarf", "gorget", "choker", "torc", "torque", "mantle", "stole"].contains(&w.as_str())) {
        c.rect(CX - 3, bottom, CX + 4, bottom + 2, color);
        c.rect(CX - 3, bottom + 1, CX + 4, bottom + 2, shade(color));
        return;
    }
    // A chain hanging in a V, the pendant at its bottom.
    for i in 0..=2 {
        both(c, CX - 3 + i, bottom + i, color);
    }
    let gem = ws.iter().find_map(|w| match w.as_str() {
        "ruby" | "garnet" | "blood" => Some(LIGHTRED),
        "emerald" | "jade" => Some(LIGHTGREEN),
        "sapphire" => Some(LIGHTBLUE),
        "holy" | "symbol" | "silver" => Some(WHITE),
        _ => None,
    });
    c.light(CX, bottom + 3, gem.unwrap_or(color));
}

/// Things worn on the head of `oval`, its brow at `brow`.
fn hat(c: &mut Canvas, look: &Look, item: &str, oval: (i32, i32, i32, i32), brow: i32) {
    let ws = words(item);
    let has = |any: &[&str]| ws.iter().any(|w| any.contains(&w.as_str()));
    let (cx, cy, rx, ry) = oval;
    let top = cy - ry;
    if has(&["helmet", "helm", "coif", "casque", "basinet", "bascinet", "sallet", "barbute", "greathelm"]) {
        let metal = item_color(item).unwrap_or(LIGHTGRAY);
        cap(c, (cx, cy, rx + 1, ry + 1), top - 1, brow, metal);
        c.rect(cx - rx - 1, brow - 1, cx + rx + 2, brow, shade(metal));
        c.rect(cx, brow, cx + 1, brow + 3, metal);
        if has(&["horned", "viking"]) || look.horns {
            for i in 0..3 {
                both(c, cx - rx - 1 - i / 2, top + 1 - i, WHITE);
            }
        }
        c.light(cx - 2, top, WHITE);
    } else if has(&["crown", "tiara", "circlet", "diadem", "coronet"]) {
        let gold = item_color(item).unwrap_or(YELLOW);
        let band = top + 2;
        c.rect(cx - rx + 1, band, cx + rx, band + 1, gold);
        if has(&["crown", "coronet"]) {
            for x in [cx - rx + 1, cx - rx / 2, cx, cx + rx / 2, cx + rx - 1] {
                c.rect(x, band - 2, x + 1, band, gold);
            }
        }
        c.light(cx, band, LIGHTRED);
    } else if has(&["hood", "cowl"]) {
        let cloth = item_color(item).unwrap_or(shade(look.clothes));
        cap(c, (cx, cy, rx + 2, ry + 2), top - 2, brow - 1, cloth);
        for y in brow - 1..cy + ry - 1 {
            c.rect(cx - rx - 2, y, cx - rx + 1, y + 1, cloth);
            c.rect(cx + rx, y, cx + rx + 3, y + 1, cloth);
        }
        c.rect(cx - rx - 2, cy + ry - 2, cx + rx + 3, cy + ry, cloth);
    } else if has(&["headband", "bandana", "bandanna", "band", "sweatband"]) {
        let cloth = item_color(item).unwrap_or(RED);
        c.rect(cx - rx, brow - 2, cx + rx + 1, brow - 1, cloth);
    } else if has(&["pointed", "wizard", "witch", "conical"]) {
        let cloth = item_color(item).unwrap_or(look.clothes);
        peak(c, cx, 0, top + 3, rx + 1, cloth);
        c.rect(cx - rx - 3, top + 3, cx + rx + 4, top + 4, cloth);
        c.light(cx, 2, YELLOW);
    } else {
        // A hat or a cap: a crown and a brim.
        let cloth = item_color(item).unwrap_or(look.clothes);
        let brim = has(&["hat", "fedora", "tricorn", "tricorne", "sombrero", "bonnet"]);
        cap(c, (cx, cy, rx + 1, ry + 1), top - 1, top + 3, cloth);
        if brim {
            c.rect(cx - rx - 3, top + 3, cx + rx + 4, top + 4, cloth);
            c.rect(cx - rx + 1, top - 2, cx + rx, top, cloth);
        } else {
            c.rect(cx - rx - 1, top + 3, cx + rx + 2, top + 4, shade(cloth));
        }
        if has(&["feathered", "feather", "plumed"]) {
            line(c, (cx + rx - 1, top), (cx + rx + 3, top - 3), WHITE);
        }
    }
}

/// Things worn on the eyes, at row `y`, the eyes `gap` from the middle.
fn eyewear(c: &mut Canvas, item: &str, y: i32, gap: i32, eyes: &dyn Fn(&mut Canvas)) {
    let ws = words(item);
    if ws.iter().any(|w| ["blindfold", "bandage", "veil"].contains(&w.as_str())) {
        let cloth = item_color(item).unwrap_or(DARKGRAY);
        c.rect(CX - gap - 3, y - 1, CX + gap + 4, y + 1, cloth);
        return;
    }
    if ws.iter().any(|w| ["patch", "eyepatch"].contains(&w.as_str())) {
        c.rect(CX - gap - 1, y - 1, CX - gap + 2, y + 1, BLACK);
        line(c, (CX - gap - 4, y - 3), (CX + gap + 4, y - 2), BLACK);
        return;
    }
    let frame = item_color(item).unwrap_or(LIGHTGRAY);
    for side in [-1, 1] {
        let x = CX + side * gap;
        c.rect(x - 2, y - 1, x + 3, y, frame);
        c.rect(x - 2, y + 1, x + 3, y + 2, frame);
        c.set(x - 2, y, frame);
        c.set(x + 2, y, frame);
    }
    c.rect(CX - gap + 3, y, CX + gap - 2, y + 1, frame);
    eyes(c);
}

/// A person's face.
fn paint_person(c: &mut Canvas, look: &Look) {
    let (cx, cy, rx, ry) = HEAD;
    let skin = look.skin;
    let hair = look.hair;
    let dark = shade(skin);
    // Wings behind (pixies, faeries), long hair behind the shoulders.
    if let Feature::Wings(color) = look.feature {
        for i in 0..9 {
            let reach = 3 + (9 - i) / 2;
            c.rect(0, 12 + i, reach, 13 + i, color);
            c.rect(W - reach, 12 + i, W, 13 + i, color);
        }
    }
    if let (Some(h), true) = (hair, look.long_hair) {
        c.rect(cx - rx - 2, cy - 4, cx + rx + 3, 23, h);
    }
    shoulders(c, look, look.clothes);
    // The neck, its shadow under the chin.
    c.rect(cx - 2, cy + ry - 2, cx + 3, 23, skin);
    c.rect(cx - 2, cy + ry, cx + 3, cy + ry + 1, dark);
    // The head and its ears.
    c.oval(cx, cy, rx, ry, skin);
    for y in cy - 1..cy + 2 {
        both(c, cx - rx - 1, y, skin);
    }
    both(c, cx - rx - 1, cy, dark);
    if look.feature == Feature::PointedEars {
        both(c, cx - rx - 2, cy - 2, skin);
        both(c, cx - rx - 2, cy - 3, skin);
    }
    // The cheeks' shading at the edges.
    for y in cy - 1..cy + ry {
        for x in [cx - rx, cx + rx] {
            if c.get(x, y) == skin {
                c.set(x, y, Canvas::mix(x, y, skin, dark, 8));
            }
        }
    }
    // Hair over the top, down the sides to the ears.
    if let Some(h) = hair {
        cap(c, (cx, cy, rx + 1, ry + 1), cy - ry - 1, cy - 4, h);
        for y in cy - 4..cy - 1 {
            both(c, cx - rx, y, h);
        }
        both(c, cx - rx + 1, cy - 4, h);
        if look.long_hair {
            for y in cy - 4..cy + ry - 1 {
                both(c, cx - rx - 1, y, h);
                both(c, cx - rx, y, h);
            }
        }
        c.set(cx - 2, cy - ry, lighter(h));
        c.set(cx - 1, cy - ry, lighter(h));
    }
    // Brows, eyes, nose, mouth.
    let brow = cy - 2;
    let ey = cy - 1;
    let gap = 3;
    let brows = hair.map(shade).unwrap_or(dark);
    c.rect(cx - gap - 1, brow, cx - gap + 2, brow + 1, brows);
    c.rect(cx + gap - 1, brow, cx + gap + 2, brow + 1, brows);
    let eyes = |c: &mut Canvas| {
        for side in [-1, 1] {
            let x = cx + side * gap;
            c.set(x - side, ey, WHITE);
            if look.glow {
                c.light(x, ey, if look.eyes == DARKGRAY || look.eyes == BROWN { LIGHTRED } else { look.eyes });
            } else {
                c.set(x, ey, look.eyes);
            }
        }
    };
    eyes(c);
    if look.old {
        both(c, cx - gap - 2, ey + 1, dark);
        c.rect(cx - 2, cy - 4, cx + 3, cy - 3, dark);
    }
    c.rect(cx, cy, cx + 1, cy + 2, dark);
    both(c, cx - 1, cy + 2, dark);
    let mouth_y = cy + 4;
    c.rect(cx - 2, mouth_y, cx + 3, mouth_y + 1, if skin == RED || skin == LIGHTRED { RED } else { shade(dark) });
    if look.freckles {
        for (x, y) in [(cx - 4, cy + 1), (cx - 2, cy + 2), (cx + 2, cy + 2), (cx + 4, cy + 1)] {
            c.set(x, y, BROWN);
        }
    }
    if look.tattoo {
        for i in 0..3 {
            c.set(cx + gap + 1 - i % 2, cy + 1 + i, BLUE);
        }
    }
    if look.scar {
        line(c, (cx + gap - 1, brow - 1), (cx + gap + 1, cy + 3), if skin == LIGHTRED { WHITE } else { dark });
        eyes(c);
    }
    // A beard over the jaw, the mouth left showing; a mustache over it.
    let beard_color = hair.unwrap_or(LIGHTGRAY);
    if look.beard {
        for y in cy + 2..cy + ry + 3 {
            for x in 0..W {
                if within(x, y, (cx, cy + 1, rx + 1, ry + 1)) && (y > cy + 3 || (x - cx).abs() > 2) {
                    c.set(x, y, beard_color);
                }
            }
        }
        c.rect(cx - 2, mouth_y, cx + 3, mouth_y + 1, shade(beard_color));
    }
    if look.mustache {
        c.rect(cx - 2, mouth_y - 1, cx + 3, mouth_y, beard_color);
        both(c, cx - 3, mouth_y, beard_color);
    }
    if look.tusks {
        both(c, cx - 2, mouth_y - 1, WHITE);
    }
    if look.fangs {
        both(c, cx - 1, mouth_y + 1, WHITE);
    }
    if look.horns {
        for i in 0..4 {
            both(c, cx - rx + 2 - i / 2, cy - ry - i, if i < 2 { DARKGRAY } else { LIGHTGRAY });
        }
    }
    // What's worn.
    if let Some(item) = look.worn.eyes.as_deref() {
        eyewear(c, item, ey, gap, &eyes);
    } else if look.patch {
        eyewear(c, "eyepatch", ey, gap, &eyes);
    }
    if let Some(item) = look.worn.mouth.as_deref() {
        if words(item).iter().any(|w| w == "pipe") {
            c.rect(cx + 2, mouth_y, cx + 5, mouth_y + 1, BROWN);
            c.rect(cx + 5, mouth_y - 1, cx + 6, mouth_y + 1, BROWN);
            c.light(cx + 5, mouth_y - 2, LIGHTRED);
        } else {
            // A mask or a veil over the face below the eyes.
            let cloth = item_color(item).unwrap_or(DARKGRAY);
            cap(c, (cx, cy, rx, ry), ey + 2, cy + ry + 1, cloth);
        }
    }
    if let Some(item) = look.worn.ears.as_deref() {
        both_lit(c, cx - rx - 1, cy + 2, item_color(item).unwrap_or(YELLOW));
    }
    if let Some(item) = look.worn.neck.as_deref() {
        neck(c, item, cy + ry + 1);
    }
    match look.worn.head.as_deref() {
        Some(item) => hat(c, look, item, HEAD, brow),
        None if look.feature == Feature::TallHat => hat(c, look, "a tall pointed red hat", HEAD, brow),
        None => {}
    }
    if look.halo {
        for x in cx - 4..=cx + 4 {
            c.light(x, cy - ry - 2 + i32::from((x - cx).abs() > 2), YELLOW);
        }
    }
}

/// A beast's face: a muzzle, its ears, its fur.
fn paint_beast(c: &mut Canvas, look: &Look, ears: Ears) {
    let fur = look.skin;
    let light = lighter(fur);
    let dark = shade(fur);
    let (cx, cy, rx, ry) = (CX, 12, 7, 7);
    let top = cy - ry;
    // The chest.
    c.oval(cx, 25, 11, 5, fur);
    c.rect(0, 23, W, H, fur);
    c.oval(cx, 25, 4, 4, light);
    // Ears behind the head's top.
    match ears {
        Ears::Pointed => {
            for side in [-1, 1] {
                peak(c, cx + side * 4, top - 4, top + 3, 2, fur);
                peak(c, cx + side * 4, top - 2, top + 2, 1, LIGHTRED);
            }
        }
        Ears::Round => {
            for side in [-1, 1] {
                c.oval(cx + side * 5, top + 1, 2, 2, fur);
                c.set(cx + side * 5, top + 1, dark);
            }
        }
        Ears::Wide => {
            for side in [-1, 1] {
                c.oval(cx + side * 8, cy - 2, 3, 5, fur);
                c.oval(cx + side * 8, cy - 2, 1, 3, dark);
            }
        }
        Ears::Long => {
            for side in [-1, 1] {
                c.rect(cx + side * 4 - 1, top - 4, cx + side * 4 + 1, top + 2, fur);
            }
        }
        Ears::Horned => {
            for side in [-1, 1] {
                for i in 0..5 {
                    c.set(cx + side * (rx - 1 + i), top + 2 - i / 2, LIGHTGRAY);
                }
                c.oval(cx + side * (rx - 1), cy - 4, 2, 1, fur);
            }
        }
        Ears::Curled => {
            for side in [-1, 1] {
                c.oval(cx + side * (rx - 1), cy - 3, 3, 3, DARKGRAY);
                c.oval(cx + side * (rx - 1), cy - 3, 1, 1, LIGHTGRAY);
            }
        }
    }
    c.oval(cx, cy, rx, ry, fur);
    if ears == Ears::Curled {
        for y in top..cy - 2 {
            for x in 0..W {
                if within(x, y, (cx, cy, rx, ry)) && (x + y) % 2 == 0 {
                    c.set(x, y, light);
                }
            }
        }
    }
    // The muzzle, the nose and the mouth.
    let pig = fur == LIGHTMAGENTA;
    c.oval(cx, cy + 4, 4, 3, if pig { LIGHTRED } else { light });
    let nose_y = cy + 2;
    if pig || ears == Ears::Horned {
        both(c, cx - 1, nose_y + 1, BLACK);
    } else {
        c.rect(cx - 1, nose_y, cx + 2, nose_y + 1, BLACK);
        c.set(cx, nose_y + 1, BLACK);
        c.set(cx, nose_y + 2, dark);
        both(c, cx - 1, nose_y + 3, dark);
    }
    if look.fangs || look.tusks {
        both(c, cx - 2, nose_y + 3, WHITE);
    }
    // Whiskers on the cats and the small ones.
    if matches!(ears, Ears::Pointed | Ears::Round) {
        for side in [-1, 1] {
            line(c, (cx + side * 3, nose_y + 1), (cx + side * 8, nose_y), LIGHTGRAY);
            line(c, (cx + side * 3, nose_y + 2), (cx + side * 8, nose_y + 3), LIGHTGRAY);
        }
    }
    // Eyes, dark at their inner corner.
    let ey = cy - 2;
    for side in [-1, 1] {
        let x = cx + side * 3;
        c.set(x, ey, look.eyes);
        c.set(x + side, ey, look.eyes);
        c.set(x - side, ey, BLACK);
        if look.glow {
            c.light(x, ey, LIGHTRED);
        }
    }
    if look.horns && ears != Ears::Horned {
        for i in 0..4 {
            both(c, cx - 3 - i / 2, top - i, WHITE);
        }
    }
    if look.scar {
        line(c, (cx + 2, cy - 4), (cx + 5, cy + 1), dark);
    }
    if let Some(item) = look.worn.neck.as_deref() {
        c.rect(cx - 5, 20, cx + 6, 21, item_color(item).unwrap_or(RED));
        c.light(cx, 21, YELLOW);
    }
}

/// A bird's: round head, a beak, feathers.
fn paint_bird(c: &mut Canvas, look: &Look) {
    let f = look.skin;
    let (cx, cy, rx, ry) = (CX, 10, 7, 7);
    c.oval(cx, 25, 11, 6, f);
    c.rect(0, 22, W, H, f);
    c.oval(cx, 25, 5, 5, lighter(f));
    c.oval(cx, cy, rx, ry, f);
    // A crest, the feathers' edge.
    for i in 0..3 {
        line(c, (cx - 1 + i, cy - ry), (cx - 2 + i * 2, cy - ry - 3), f);
    }
    for y in (cy + 2..cy + ry + 4).step_by(2) {
        both(c, cx - rx + 1, y, shade(f));
    }
    // The beak, hooked, down the middle.
    peak(c, cx, cy - 1, cy + 4, 2, YELLOW);
    c.rect(cx - 1, cy + 4, cx + 2, cy + 6, YELLOW);
    c.set(cx, cy + 6, BROWN);
    both(c, cx - 1, cy + 1, BROWN);
    // Eyes on the sides, ringed.
    let ey = cy - 2;
    for side in [-1, 1] {
        let x = cx + side * 4;
        c.set(x, ey, YELLOW);
        c.set(x + side, ey, if look.glow { LIGHTRED } else { BLACK });
    }
}

/// A scaled head: a lizard's, a snake's, a frog's, a fish's, a dragon's.
fn paint_scaled(c: &mut Canvas, look: &Look, kind: Scales) {
    let s = look.skin;
    let dark = shade(s);
    let belly = lighter(s);
    let (cx, cy, rx, ry) = match kind {
        Scales::Frog => (CX, 13, 9, 6),
        Scales::Fish => (CX, 12, 7, 8),
        Scales::Snake => (CX, 11, 5, 8),
        _ => (CX, 11, 6, 8),
    };
    if kind == Scales::Fish {
        // Water, and fins out to the sides and on top.
        c.gradient(0, H, &[BLACK, BLUE, CYAN]);
        for side in [-1, 1] {
            for i in 0..4 {
                let x = cx + side * (rx + i);
                c.rect(x, cy - 2 + i, x + 1, cy + 4 - i, dark);
            }
        }
        peak(c, cx, cy - ry - 4, cy - ry + 2, 3, dark);
    } else {
        // The neck and the body's scales, the belly paler.
        let half = if kind == Scales::Snake { 5 } else { 10 };
        c.oval(cx, 25, half, 6, s);
        c.rect(cx - half, 22, cx + half + 1, H, s);
        c.oval(cx, 25, 3, 5, belly);
    }
    if kind == Scales::Dragon {
        for side in [-1, 1] {
            for i in 0..6 {
                c.set(cx + side * (rx - 1 + i / 2), cy - ry + 2 - i, LIGHTGRAY);
            }
        }
    }
    c.oval(cx, cy, rx, ry, s);
    // The scales' pattern.
    for y in cy - ry..cy + ry {
        for x in 0..W {
            if c.get(x, y) == s && x % 3 == 0 && y % 3 == (x / 3) % 2 + 1 {
                c.set(x, y, dark);
            }
        }
    }
    // A pale jaw, nostrils, a long mouth.
    let mouth_y = cy + ry - 3;
    cap(c, (cx, cy, rx - 1, ry), mouth_y, cy + ry, belly);
    c.rect(cx - rx + 2, mouth_y, cx + rx - 1, mouth_y + 1, shade(dark));
    both(c, cx - 1, mouth_y - 2, BLACK);
    if kind == Scales::Fish {
        c.oval(cx, mouth_y + 1, 1, 1, shade(dark));
    }
    if kind == Scales::Snake {
        c.rect(cx, mouth_y + 1, cx + 1, mouth_y + 3, LIGHTRED);
        both(c, cx - 1, mouth_y + 3, LIGHTRED);
    }
    if kind == Scales::Dragon || look.fangs {
        for x in (cx - rx + 3..cx + rx - 2).step_by(2) {
            c.set(x, mouth_y + 1, WHITE);
        }
    }
    // Eyes, slit; a frog's bulge on top.
    let (ey, gap) = match kind {
        Scales::Frog => (cy - ry + 1, 5),
        _ => (cy - 2, 3),
    };
    for side in [-1, 1] {
        let x = cx + side * gap;
        if kind == Scales::Frog {
            c.oval(x, ey, 2, 2, s);
        }
        c.rect(x - 1, ey, x + 2, ey + 1, look.eyes);
        c.set(x, ey, BLACK);
        if look.glow {
            c.light(x - 1, ey, look.eyes);
        }
    }
}

/// An insect's, a spider's or a worm's face.
fn paint_bug(c: &mut Canvas, look: &Look, kind: Bug) {
    let s = look.skin;
    let (cx, cy, rx, ry) = (CX, 12, 7, 7);
    c.oval(cx, 25, 10, 5, s);
    c.rect(1, 23, W - 1, H, s);
    if kind == Bug::Insect {
        for side in [-1, 1] {
            line(c, (cx + side * 2, cy - ry + 1), (cx + side * 6, 0), shade(s));
        }
    }
    c.oval(cx, cy, rx, ry, s);
    c.set(cx - 3, cy - ry + 2, lighter(s));
    c.set(cx - 2, cy - ry + 2, lighter(s));
    match kind {
        Bug::Insect => {
            // Compound eyes, and mandibles.
            for side in [-1, 1] {
                c.oval(cx + side * 4, cy - 1, 2, 2, look.eyes);
                c.set(cx + side * 4, cy - 2, lighter(look.eyes));
                line(c, (cx + side * 3, cy + ry - 2), (cx + side, cy + ry + 1), BROWN);
            }
        }
        Bug::Spider => {
            // Eight eyes, two big; fangs.
            for side in [-1, 1] {
                c.light(cx + side * 2, cy - 2, look.eyes);
                c.light(cx + side * 2, cy - 1, look.eyes);
                for (dx, dy) in [(3, -4), (4, -1), (5, -3)] {
                    c.light(cx + side * dx, cy + dy, look.eyes);
                }
                c.rect(cx + side * 2, cy + ry - 2, cx + side * 2 + 1, cy + ry + 1, BLACK);
            }
        }
        Bug::Worm => {
            // A round mouth of teeth.
            c.oval(cx, cy + 1, 4, 4, BLACK);
            for (dx, dy) in [(0, -3), (2, -2), (3, 0), (2, 2), (0, 3), (-2, 2), (-3, 0), (-2, -2)] {
                c.set(cx + dx, cy + 1 + dy, WHITE);
            }
        }
    }
}

/// A skull: the undead with no flesh to speak of.
fn paint_skull(c: &mut Canvas, look: &Look, race: &str) {
    let bone = look.skin;
    let mummy = key(race).contains("mummy");
    let (cx, cy, rx, ry) = (CX, 10, 7, 8);
    shoulders(c, look, if key(race).contains("lich") { MAGENTA } else { DARKGRAY });
    // The spine for a neck.
    for y in cy + ry..23 {
        c.set(cx, y, LIGHTGRAY);
        if y % 2 == 0 {
            c.rect(cx - 1, y, cx + 2, y + 1, LIGHTGRAY);
        }
    }
    c.oval(cx, cy, rx, ry, bone);
    c.rect(cx - 4, cy + 3, cx + 5, cy + ry + 1, bone);
    if mummy {
        for y in (cy - ry + 1..cy + ry).step_by(3) {
            for x in 0..W {
                if c.get(x, y) == bone {
                    c.set(x, y, LIGHTGRAY);
                }
            }
        }
    }
    // Sockets with a light in them, the nose's hole, teeth.
    for side in [-1, 1] {
        c.oval(cx + side * 3, cy - 1, 1, 1, BLACK);
        c.light(cx + side * 3, cy - 1, look.eyes);
    }
    c.set(cx, cy + 2, BLACK);
    c.rect(cx - 1, cy + 3, cx + 2, cy + 4, BLACK);
    c.rect(cx - 3, cy + 5, cx + 4, cy + 6, BLACK);
    for x in (cx - 3..cx + 4).step_by(2) {
        c.set(x, cy + 6, BLACK);
    }
    if !mummy {
        c.rect(cx - 3, cy + 7, cx + 4, cy + 8, shade(bone));
    }
    if let Some(item) = look.worn.head.as_deref() {
        hat(c, look, item, (cx, cy, rx, ry), cy - 3);
    }
    if let Some(item) = look.worn.neck.as_deref() {
        neck(c, item, cy + ry + 2);
    }
}

/// A being of stuff: fire, water, air, earth, a golem, a slime, a plant.
fn paint_stuff(c: &mut Canvas, look: &Look, stuff: Stuff, rng: &mut Rng) {
    let s = look.skin;
    let light = lighter(s);
    let (cx, cy, rx, ry) = (CX, 11, 7, 9);
    match stuff {
        Stuff::Golem => {
            // Blocks: shoulders, a square head, a heavy brow.
            c.rect(0, 21, W, H, s);
            c.rect(cx - 2, 18, cx + 3, 21, shade(s));
            c.rect(cx - 6, 2, cx + 7, 19, s);
            c.rect(cx - 6, cy - 3, cx + 7, cy - 1, shade(s));
            for _ in 0..10 {
                let (x, y) = (rng.between(cx - 5, cx + 6), rng.between(3, 18));
                if c.get(x, y) == s {
                    c.set(x, y, shade(s));
                }
            }
            for side in [-1, 1] {
                c.light(cx + side * 3, cy, look.eyes);
                c.light(cx + side * 3 + side, cy, look.eyes);
            }
            c.rect(cx - 3, cy + 5, cx + 4, cy + 6, BLACK);
        }
        Stuff::Slime => {
            c.oval(cx, 17, 10, 10, s);
            c.rect(0, 22, W, H, s);
            c.oval(cx - 4, 12, 1, 2, light);
            for side in [-1, 1] {
                c.oval(cx + side * 3, 15, 1, 1, WHITE);
                c.set(cx + side * 3, 15, BLACK);
            }
        }
        Stuff::Plant => {
            // Bark for a face, leaves about it.
            for _ in 0..70 {
                let (x, y) = (rng.between(0, W - 1), rng.between(0, 16));
                if within(x, y, (cx, 8, rx + 4, ry)) {
                    c.set(x, y, if rng.chance(50) { GREEN } else { LIGHTGREEN });
                }
            }
            c.rect(cx - 4, 6, cx + 5, H, BROWN);
            for y in (6..H).step_by(2) {
                c.set(cx - 2 + (y % 5), y, shade(BROWN));
            }
            for side in [-1, 1] {
                c.light(cx + side * 2, 10, YELLOW);
            }
            c.oval(cx, 15, 1, 1, BLACK);
        }
        _ => {
            // Fire, water, air or earth: a body of it, moving.
            let (a, b) = match stuff {
                Stuff::Fire => (RED, YELLOW),
                Stuff::Water => (BLUE, LIGHTCYAN),
                Stuff::Air => (LIGHTGRAY, WHITE),
                _ => (BROWN, DARKGRAY),
            };
            c.oval(cx, 25, 11, 5, a);
            c.rect(0, 23, W, H, a);
            c.oval(cx, cy, rx, ry, a);
            if stuff == Stuff::Fire {
                for x in [cx - 5, cx - 2, cx + 2, cx + 5] {
                    peak(c, x, cy - ry - rng.between(2, 4), cy - ry + 2, 1, a);
                }
            }
            for _ in 0..50 {
                let (x, y) = (rng.between(0, W - 1), rng.between(0, H - 1));
                if c.get(x, y) == a {
                    c.light(x, y, b);
                }
            }
            for side in [-1, 1] {
                c.light(cx + side * 3, cy - 1, WHITE);
                c.light(cx + side * 3 + side, cy - 1, WHITE);
            }
            c.rect(cx - 2, cy + 4, cx + 3, cy + 5, shade(a));
        }
    }
}

/// A creature's face, 25 by 24 pixels in 25 by 12 cells.
pub fn face(sitter: &Sitter) -> Art {
    let look = read(sitter);
    let mut canvas = Canvas::new(SIZE.0, SIZE.1);
    backdrop(&mut canvas, look.backdrop);
    let mut rng = Rng::new(seed("face-stuff", &key(&sitter.name), 0));
    match look.shape {
        Shape::Person => paint_person(&mut canvas, &look),
        Shape::Beast(ears) => paint_beast(&mut canvas, &look, ears),
        Shape::Bird => paint_bird(&mut canvas, &look),
        Shape::Scaled(kind) => paint_scaled(&mut canvas, &look, kind),
        Shape::Bug(kind) => paint_bug(&mut canvas, &look, kind),
        Shape::Skull => paint_skull(&mut canvas, &look, sitter.race.as_deref().unwrap_or("")),
        Shape::Stuff(stuff) => paint_stuff(&mut canvas, &look, stuff, &mut rng),
    }
    canvas.art()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Color;

    fn sitter(name: &str, race: &str, category: &str, description: &str) -> Sitter {
        Sitter { name: name.into(), race: Some(race.into()), category: Some(category.into()), description: Some(description.into()), ..Sitter::default() }
    }

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
    fn every_shape_paints_at_its_size_in_the_fonts_characters() {
        let font = crate::ansi_art::CP437;
        let mut seen = Vec::new();
        for line in include_str!("races.tsv").lines().skip(1) {
            let mut cols = line.split('\t');
            let (Some(race), Some(category)) = (cols.next(), cols.next()) else { continue };
            let art = face(&sitter(&format!("a {race}"), race, category, ""));
            assert_eq!((art.columns, art.rows, art.lines.len()), (SIZE.0, SIZE.1, SIZE.1), "{race}");
            for line in &art.lines {
                for span in line {
                    assert!(span.text.chars().all(|c| font.contains(c)), "{race}: {:?}", span.text);
                }
            }
            seen.push(shape(category, race));
        }
        for wanted in [Shape::Person, Shape::Bird, Shape::Skull, Shape::Beast(Ears::Pointed), Shape::Scaled(Scales::Fish), Shape::Bug(Bug::Spider), Shape::Stuff(Stuff::Golem)] {
            assert!(seen.contains(&wanted), "{wanted:?}");
        }
        // Nothing known at all still paints.
        face(&Sitter { name: "something".into(), ..Sitter::default() });
    }

    #[test]
    fn the_same_creature_the_same_face_and_another_differs() {
        let orc = sitter("an orc", "Orc", "Orc", "An ugly brute.");
        assert_eq!(face(&orc), face(&orc));
        assert_ne!(face(&orc), face(&sitter("a city guard", "Human", "Human", "A guard.")));
    }

    #[test]
    fn the_description_gives_the_colors() {
        let read = |d: &str| read(&sitter("a woman", "Human", "Human", d));
        assert_eq!(read("She has long, flowing red hair and blue eyes.").hair, Some(RED));
        assert_eq!(read("She has long, flowing red hair and blue eyes.").eyes, LIGHTBLUE);
        assert!(read("She has long, flowing red hair.").long_hair);
        assert_eq!(read("A blond-haired youth.").hair, Some(YELLOW));
        assert_eq!(read("He is completely bald.").hair, None);
        assert_eq!(read("Her skin is pale as milk.").skin, LIGHTGRAY);
        assert!(read("A scarred veteran with an eye patch.").scar);
        assert!(read("A scarred veteran with an eye patch.").patch);
        assert!(read("A bearded old man.").beard && read("A bearded old man.").old);
        assert!(!read("A clean-shaven man.").beard);
        // The name gives a beast its coat.
        assert_eq!(super::read(&sitter("a black wolf", "Wolf", "Canine", "")).skin, DARKGRAY);
    }

    #[test]
    fn what_it_wears_shows() {
        let bare = sitter("a guard", "Human", "Human", "A guard.");
        let mut helmed = bare.clone();
        helmed.wears = vec![("worn on head".into(), "a gold helmet".into()), ("worn on neck".into(), "a ruby amulet".into())];
        let (plain, worn) = (face(&bare), face(&helmed));
        assert_ne!(plain, worn);
        // The helmet's gold over where the hair was.
        let top = |art: &Art| colors(&Art { columns: art.columns, rows: 6, lines: art.lines[1..4].to_vec() });
        assert!(top(&worn).contains(&YELLOW));
        assert!(colors(&worn).contains(&LIGHTRED));
        // Worn things in the description count where nothing's known worn.
        let hooded = read(&sitter("a monk", "Human", "Human", "A figure in a grey hood."));
        assert_eq!(hooded.worn.head.as_deref(), Some("hood"));
    }

    #[test]
    fn a_dwarf_has_a_beard_and_an_elf_doesnt() {
        assert!(read(&sitter("a dwarf", "Dwarf", "Dwarf", "")).beard);
        let mut elf = sitter("an elf", "Elf", "Elf", "");
        elf.sex = Some("female".into());
        assert!(!read(&elf).beard);
    }

    fn read(s: &Sitter) -> Look {
        super::read(s)
    }
}

#[cfg(test)]
mod look {
    use super::*;
    use crate::ansi::Color;

    /// Writes a row of faces as a PPM to `OUT` to look at while tuning:
    /// `OUT=/tmp/f.ppm cargo test face::look -- --ignored`.
    #[test]
    #[ignore]
    fn show() {
        const VGA: [[u8; 3]; 16] = [[0, 0, 0], [170, 0, 0], [0, 170, 0], [170, 85, 0], [0, 0, 170], [170, 0, 170], [0, 170, 170], [170, 170, 170], [85, 85, 85], [255, 85, 85], [85, 255, 85], [255, 255, 85], [85, 85, 255], [255, 85, 255], [85, 255, 255], [255, 255, 255]];
        let s = |name: &str, race: &str, category: &str, description: &str, sex: &str, wears: &[(&str, &str)]| Sitter {
            name: name.into(),
            race: Some(race.into()),
            category: Some(category.into()),
            description: Some(description.into()),
            sex: Some(sex.into()),
            body: None,
            wears: wears.iter().map(|(p, i)| (p.to_string(), i.to_string())).collect(),
        };
        let sitters = [
            s("a city guard", "Human", "Human", "A stern guard.", "male", &[("worn on head", "a steel helmet"), ("worn on torso", "a chainmail shirt")]),
            s("a barmaid", "Human", "Human", "She has long red hair and green eyes, and freckles.", "female", &[("worn on neck", "a silver necklace")]),
            s("an old wizard", "Human", "Human", "A bearded old man.", "male", &[("worn on head", "a pointed blue hat")]),
            s("a dwarf", "Dwarf", "Dwarf", "", "male", &[]),
            s("an elf", "Elf", "Elf", "", "female", &[("worn on head", "a gold circlet")]),
            s("an orc", "Orc", "Orc", "A scarred brute.", "male", &[]),
            s("a thief", "Human", "Human", "", "male", &[("worn on head", "a black hood")]),
            s("a black wolf", "Wolf", "Canine", "", "male", &[]),
            s("a cat", "Cat", "Feline", "", "female", &[]),
            s("a bear", "Bear", "Ursine", "", "male", &[]),
            s("a horse", "Horse", "Equine", "", "male", &[]),
            s("a cow", "Cow", "Bovine", "", "female", &[]),
            s("a hawk", "Hawk", "Avian", "", "male", &[]),
            s("a snake", "Snake", "Ophidian", "", "male", &[]),
            s("a red dragon", "Red Dragon", "Dragon", "", "male", &[]),
            s("a fish", "Fish", "Fish", "", "male", &[]),
            s("a frog", "Frog", "Amphibian", "", "male", &[]),
            s("a spider", "Spider", "Arachnid", "", "male", &[]),
            s("an ant", "Ant", "Insect", "", "male", &[]),
            s("a skeleton", "Skeleton", "Undead", "", "male", &[]),
            s("a fire elemental", "Fire Elemental", "Elemental", "", "male", &[]),
            s("a stone golem", "Stone Golem", "Stone Golem", "", "male", &[]),
            s("a green slime", "Slime", "Slime", "", "male", &[]),
            s("a treant", "Treant", "Vegetation", "", "male", &[]),
        ];
        let arts: Vec<Art> = sitters.iter().map(face).collect();
        let (w, h) = (arts.len() * 26, PIXELS.1 as usize);
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
                            if row * 2 + dy >= h {
                                continue;
                            }
                            let at = ((row * 2 + dy) * w + i * 26 + x) * 3;
                            px[at..at + 3].copy_from_slice(&VGA[c]);
                        }
                        x += 1;
                    }
                }
            }
        }
        let mut out = format!("P6 {w} {h} 255\n").into_bytes();
        out.extend(px);
        std::fs::write(std::env::var("OUT").unwrap_or_else(|_| "faces.ppm".into()), out).unwrap();
    }
}

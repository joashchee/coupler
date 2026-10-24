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
//!
//! **In layers** (`figure.rs`): the place behind, the creature drawn on
//! a clear canvas of its own with each pixel's part (`painter::Part`),
//! then the passes that read the parts, then laid on the place and lit by
//! it:
//!
//! - **The place**: where it's been seen the most (its area's commonest
//!   terrain on the map), by the room painter, a shade darker; a
//!   creature before the player now, the room itself (`Moment::scene`).
//!   An animal stands in the wild, never a room or a street; a fish, in
//!   the water. Nowhere known, a backdrop of one color.
//! - **A thinking creature or an animal**: people and the races that
//!   think (a gnoll, a lizard man, a minotaur, a werewolf) have whites
//!   to their eyes, brows, and shoulders in clothes; an animal's head is
//!   low in the frame, its back running off the picture's side, no
//!   brows. A beast's head with two signs of a mind (it wears things,
//!   it has skills, it has arms) thinks too.
//! - **The mood** (`Mood`), at each shape's eyes, brows and mouth
//!   (`Anchors`): calm (a sneer for the evil), angry, in pain, afraid,
//!   asleep, dead.
//! - **Hurt** in five steps (`Moment::hurt`), as CoffeeMUD's ten health
//!   lines go (`healthText`, from "misplaced hairs" to "covered in
//!   blood"): scratches, cuts, bruises, bleeding, near death. Placed by
//!   the name, so the wounds stay put and only add up. Flesh bleeds,
//!   fur mats, stone and bone crack, ghosts and elementals fade, a slime
//!   shrinks.
//! - **Effects** on it (`Moment::effects`, the names `char.effects`
//!   gives, `effect_looks.tsv` from `scripts/painter-tables.py`): a halo,
//!   poison's green, burning, frost, blind eyes, half seen, faerie fire's
//!   outline, a web, stone skin, bark skin, a glow; sleep, fear and rage
//!   set the mood.

use crate::ansi_art::Art;
use crate::bestiary::MobEntry;
use crate::figure::{self, CLEAR};
use crate::paint::{seed, Scene};
use crate::painter::*;
use crate::portrait::{self, Feature};
use serde::Deserialize;

pub(crate) use crate::figure::{lighter, shade};

/// A picture's size in cells (the faces' and the items', `item_art.rs`),
/// and the square of pixels drawn in it.
pub const SIZE: (usize, usize) = (25, 12);
pub(crate) const PIXELS: (i32, i32) = (25, 24);
/// A small face's size in cells (`small`): 16 by 16 pixels.
pub const SMALL: (usize, usize) = (16, 8);

/// The size a face is drawn at: never one scaled to the other.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Size {
    /// The Bestiary's and The Fight panel's, `SIZE`.
    #[default]
    Full,
    /// The phone's Fight strip's, `SMALL`: drawn by its own strokes.
    Small,
}

impl Size {
    /// Its size in cells.
    pub const fn cells(self) -> (usize, usize) {
        match self {
            Size::Full => SIZE,
            Size::Small => SMALL,
        }
    }
}

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
    /// The terrain where it's been seen the most, from the map.
    pub terrain: Option<String>,
    /// "evil", "good": what its aura said.
    pub alignment: Option<String>,
    /// It's been seen using skills of its own (a spell, a fighter's).
    pub skilled: bool,
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
            terrain: None,
            alignment: m.areas.iter().max_by_key(|a| a.seen).and_then(|a| a.alignment.clone()),
            skilled: m.abilities.iter().any(|a| a.what.as_deref().is_some_and(|k| k != "power" && k != "disease")),
        }
    }
}

/// The creature now: how it feels, how hurt, what's on it, where. The
/// Bestiary's still picture is the default: calm, unhurt, nothing on it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Moment {
    pub mood: Mood,
    /// 0 unhurt to 5 near death.
    pub hurt: u8,
    /// The names of the effects on it ("Poison", "Sanctuary").
    pub effects: Vec<String>,
    /// The room it's in, painted behind it instead of where it's most seen.
    pub scene: Option<Scene>,
}

/// How it looks out of its face.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mood {
    #[default]
    Calm,
    Angry,
    Pain,
    Afraid,
    Asleep,
    Dead,
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

/// Races in a beast's category with a person's head and shoulders: an
/// orc (CoffeeMUD files it under porcine), a centaur, a naga, a harpy.
const PEOPLE: &[&str] = &["orc", "centaur", "naga", "harpy", "manscorpion", "merfolk", "selkie"];

/// Races with a beast's head that think: they're drawn as people are,
/// with whites to their eyes, brows and clothes.
const THINKING: &[&str] = &[
    "gnoll", "werewolf", "wererat", "werebear", "werebat", "wereamphibian", "wereanimal", "kobold", "lizardman", "minotaur", "tabaxi", "wemic", "aarakocran", "birdman", "bullywug", "draconian", "beastkin", "mustie", "sasquatch",
];

/// The shape a race's category (`races.tsv`) and name give.
fn shape(category: &str, race: &str) -> Shape {
    let race = key(race);
    let has = |w: &str| race.contains(w);
    if PEOPLE.contains(&race.as_str()) {
        return Shape::Person;
    }
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
    /// It thinks: whites to its eyes, brows, a bust in clothes.
    sapient: bool,
    /// An animal: drawn low in the frame, its back running off the side.
    animal: bool,
    /// A ghost, a spectre: hurt, it fades.
    ghostly: bool,
    evil: bool,
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
    let signs = [!s.wears.is_empty(), s.skilled, said.has(&["arm", "arms"])].iter().filter(|&&b| b).count();
    let sapient = shape == Shape::Person || THINKING.contains(&r.as_str()) || (matches!(shape, Shape::Beast(_) | Shape::Bird | Shape::Scaled(_)) && signs >= 2);
    let animal = !sapient && matches!(shape, Shape::Beast(_) | Shape::Bird | Shape::Scaled(Scales::Lizard | Scales::Snake | Scales::Frog | Scales::Dragon));
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
        sapient,
        animal,
        ghostly: ["ghost", "spectre", "specter", "spirit", "shadow", "wraith", "banshee", "phantom"].iter().any(|g| r.contains(g)),
        evil: s.alignment.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("evil")),
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

/// Where a face's eyes, brows and mouth are, as its shape drew them:
/// the moods and the wounds go there.
struct Anchors {
    /// Each eye's middle, the left first.
    eyes: Vec<(i32, i32)>,
    /// The brows' row, where the face has brows.
    brow: Option<i32>,
    /// The mouth's middle and its half width.
    mouth: Option<(i32, i32, i32)>,
    /// The top of the head, for a halo, a sleeper's z's.
    crown: (i32, i32),
    /// A closed eye's color: the eyelid's.
    lid: u8,
    /// Whites to the eyes: a thinking creature's.
    whites: bool,
}

/// The person's head: an oval.
const HEAD: (i32, i32, i32, i32) = (CX, 11, 6, 8);

/// Shoulders and the chest, in the clothes, and what's worn there.
fn shoulders(c: &mut Canvas, look: &Look, color: u8) {
    c.pen = Part::Worn;
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
    c.pen = Part::Worn;
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
    c.pen = Part::Worn;
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
    c.pen = Part::Worn;
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

/// A thinking creature's brows over eyes `gap` from the middle at row `ey`.
fn brows(c: &mut Canvas, ey: i32, gap: i32, color: u8) {
    c.pen = Part::Brow;
    c.rect(CX - gap - 1, ey - 1, CX - gap + 2, ey, color);
    c.rect(CX + gap - 1, ey - 1, CX + gap + 2, ey, color);
}

/// An animal's back, running off the picture's right side from under
/// its head (`figure::lay` carries it on past the edge).
fn back(c: &mut Canvas, color: u8, top: i32) {
    c.pen = Part::Body;
    c.oval(CX + 6, top + 3, 12, 4, color);
    c.rect(CX, top + 2, W, H, color);
}

/// A person's face.
fn paint_person(c: &mut Canvas, look: &Look) -> Anchors {
    let (cx, cy, rx, ry) = HEAD;
    let skin = look.skin;
    let hair = look.hair;
    let dark = shade(skin);
    // Wings behind (pixies, faeries), long hair behind the shoulders.
    if let Feature::Wings(color) = look.feature {
        c.pen = Part::Detail;
        for i in 0..9 {
            let reach = 3 + (9 - i) / 2;
            c.rect(0, 12 + i, reach, 13 + i, color);
            c.rect(W - reach, 12 + i, W, 13 + i, color);
        }
    }
    if let (Some(h), true) = (hair, look.long_hair) {
        c.pen = Part::Hair;
        c.rect(cx - rx - 2, cy - 4, cx + rx + 3, 23, h);
    }
    shoulders(c, look, look.clothes);
    // The neck, its shadow under the chin.
    c.pen = Part::Body;
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
        c.pen = Part::Hair;
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
    brows(c, ey, gap, hair.map(shade).unwrap_or(dark));
    let eyes = |c: &mut Canvas| {
        c.pen = Part::Eye;
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
    c.pen = Part::Body;
    if look.old {
        both(c, cx - gap - 2, ey + 1, dark);
        c.rect(cx - 2, cy - 4, cx + 3, cy - 3, dark);
    }
    c.rect(cx, cy, cx + 1, cy + 2, dark);
    both(c, cx - 1, cy + 2, dark);
    let mouth_y = cy + 4;
    c.pen = Part::Mouth;
    c.rect(cx - 2, mouth_y, cx + 3, mouth_y + 1, if skin == RED || skin == LIGHTRED { RED } else { shade(dark) });
    c.pen = Part::Body;
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
        c.pen = Part::Hair;
        for y in cy + 2..cy + ry + 3 {
            for x in 0..W {
                if within(x, y, (cx, cy + 1, rx + 1, ry + 1)) && (y > cy + 3 || (x - cx).abs() > 2) {
                    c.set(x, y, beard_color);
                }
            }
        }
        c.pen = Part::Mouth;
        c.rect(cx - 2, mouth_y, cx + 3, mouth_y + 1, shade(beard_color));
    }
    if look.mustache {
        c.pen = Part::Hair;
        c.rect(cx - 2, mouth_y - 1, cx + 3, mouth_y, beard_color);
        both(c, cx - 3, mouth_y, beard_color);
    }
    c.pen = Part::Detail;
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
        c.pen = Part::Worn;
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
        c.pen = Part::Worn;
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
        c.pen = Part::Detail;
        for x in cx - 4..=cx + 4 {
            c.light(x, cy - ry - 2 + i32::from((x - cx).abs() > 2), YELLOW);
        }
    }
    Anchors { eyes: vec![(cx - gap, ey), (cx + gap, ey)], brow: Some(brow), mouth: Some((cx, mouth_y, 2)), crown: (cx, cy - ry), lid: skin, whites: true }
}

/// A beast's face: a muzzle, its ears, its fur. A thinking one (a gnoll,
/// a werewolf) has a bust in clothes, brows and whites to its eyes; an
/// animal, its back running off the side.
fn paint_beast(c: &mut Canvas, look: &Look, ears: Ears) -> Anchors {
    let fur = look.skin;
    let light = lighter(fur);
    let dark = shade(fur);
    let (cx, cy, rx, ry) = (CX, 12, 7, 7);
    let top = cy - ry;
    if look.sapient {
        shoulders(c, look, look.clothes);
    } else {
        back(c, fur, 18);
    }
    c.pen = Part::Body;
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
                c.pen = Part::Detail;
                for i in 0..5 {
                    c.set(cx + side * (rx - 1 + i), top + 2 - i / 2, LIGHTGRAY);
                }
                c.pen = Part::Body;
                c.oval(cx + side * (rx - 1), cy - 4, 2, 1, fur);
            }
        }
        Ears::Curled => {
            c.pen = Part::Detail;
            for side in [-1, 1] {
                c.oval(cx + side * (rx - 1), cy - 3, 3, 3, DARKGRAY);
                c.oval(cx + side * (rx - 1), cy - 3, 1, 1, LIGHTGRAY);
            }
            c.pen = Part::Body;
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
    c.pen = Part::Detail;
    if pig || ears == Ears::Horned {
        both(c, cx - 1, nose_y + 1, BLACK);
    } else {
        c.rect(cx - 1, nose_y, cx + 2, nose_y + 1, BLACK);
        c.set(cx, nose_y + 1, BLACK);
        c.pen = Part::Mouth;
        c.set(cx, nose_y + 2, dark);
        both(c, cx - 1, nose_y + 3, dark);
    }
    c.pen = Part::Detail;
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
    // Eyes: an animal's dark at their inner corner, a thinking one's with whites and brows.
    let ey = cy - 2;
    if look.sapient {
        brows(c, ey - 1, 3, dark);
    }
    c.pen = Part::Eye;
    for side in [-1, 1] {
        let x = cx + side * 3;
        c.set(x, ey, look.eyes);
        c.set(x + side, ey, look.eyes);
        c.set(x - side, ey, if look.sapient { WHITE } else { BLACK });
        if look.glow {
            c.light(x, ey, LIGHTRED);
        }
    }
    c.pen = Part::Detail;
    if look.horns && ears != Ears::Horned {
        for i in 0..4 {
            both(c, cx - 3 - i / 2, top - i, WHITE);
        }
    }
    c.pen = Part::Body;
    if look.scar {
        line(c, (cx + 2, cy - 4), (cx + 5, cy + 1), dark);
    }
    if let Some(item) = look.worn.neck.as_deref() {
        if look.sapient {
            neck(c, item, cy + ry + 1);
        } else {
            // A collar: it's someone's.
            c.pen = Part::Worn;
            c.rect(cx - 5, 20, cx + 6, 21, item_color(item).unwrap_or(RED));
            c.light(cx, 21, YELLOW);
        }
    }
    if let (true, Some(item)) = (look.sapient, look.worn.head.as_deref()) {
        hat(c, look, item, (cx, cy, rx, ry), ey - 1);
    }
    Anchors { eyes: vec![(cx - 3, ey), (cx + 3, ey)], brow: look.sapient.then_some(ey - 2), mouth: Some((cx, nose_y + 3, 1)), crown: (cx, top), lid: fur, whites: look.sapient }
}

/// A bird's: round head, a beak, feathers.
fn paint_bird(c: &mut Canvas, look: &Look) -> Anchors {
    let f = look.skin;
    let (cx, cy, rx, ry) = (CX, 10, 7, 7);
    if look.sapient {
        shoulders(c, look, look.clothes);
    } else {
        back(c, f, 17);
    }
    c.pen = Part::Body;
    c.oval(cx, cy, rx, ry, f);
    // A crest, the feathers' edge.
    for i in 0..3 {
        line(c, (cx - 1 + i, cy - ry), (cx - 2 + i * 2, cy - ry - 3), f);
    }
    for y in (cy + 2..cy + ry + 4).step_by(2) {
        both(c, cx - rx + 1, y, shade(f));
    }
    // The beak, hooked, down the middle.
    c.pen = Part::Mouth;
    peak(c, cx, cy - 1, cy + 4, 2, YELLOW);
    c.rect(cx - 1, cy + 4, cx + 2, cy + 6, YELLOW);
    c.set(cx, cy + 6, BROWN);
    both(c, cx - 1, cy + 1, BROWN);
    // Eyes on the sides, ringed.
    let ey = cy - 2;
    if look.sapient {
        brows(c, ey, 4, shade(f));
    }
    c.pen = Part::Eye;
    for side in [-1, 1] {
        let x = cx + side * 4;
        c.set(x, ey, YELLOW);
        c.set(x + side, ey, if look.glow { LIGHTRED } else { BLACK });
    }
    Anchors { eyes: vec![(cx - 4, ey), (cx + 4, ey)], brow: look.sapient.then_some(ey - 1), mouth: Some((cx, cy + 5, 1)), crown: (cx, cy - ry - 2), lid: f, whites: false }
}

/// A scaled head: a lizard's, a snake's, a frog's, a fish's, a dragon's.
fn paint_scaled(c: &mut Canvas, look: &Look, kind: Scales) -> Anchors {
    let s = look.skin;
    let dark = shade(s);
    let belly = lighter(s);
    let (cx, cy, rx, ry) = match kind {
        Scales::Frog => (CX, 13, 9, 6),
        Scales::Fish => (CX, 12, 7, 8),
        Scales::Snake => (CX, 11, 5, 8),
        _ => (CX, 11, 6, 8),
    };
    c.pen = Part::Body;
    if kind == Scales::Fish {
        // Fins out to the sides and on top.
        for side in [-1, 1] {
            for i in 0..4 {
                let x = cx + side * (rx + i);
                c.rect(x, cy - 2 + i, x + 1, cy + 4 - i, dark);
            }
        }
        peak(c, cx, cy - ry - 4, cy - ry + 2, 3, dark);
    } else if look.sapient {
        shoulders(c, look, look.clothes);
        c.pen = Part::Body;
        c.rect(cx - 3, cy + ry - 2, cx + 4, 23, s);
    } else if look.animal {
        back(c, s, cy + ry - 3);
    }
    if kind == Scales::Dragon {
        c.pen = Part::Detail;
        for side in [-1, 1] {
            for i in 0..6 {
                c.set(cx + side * (rx - 1 + i / 2), cy - ry + 2 - i, LIGHTGRAY);
            }
        }
    }
    c.pen = Part::Body;
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
    c.pen = Part::Mouth;
    c.rect(cx - rx + 2, mouth_y, cx + rx - 1, mouth_y + 1, shade(dark));
    c.pen = Part::Detail;
    both(c, cx - 1, mouth_y - 2, BLACK);
    if kind == Scales::Fish {
        c.pen = Part::Mouth;
        c.oval(cx, mouth_y + 1, 1, 1, shade(dark));
    }
    if kind == Scales::Snake {
        c.pen = Part::Detail;
        c.rect(cx, mouth_y + 1, cx + 1, mouth_y + 3, LIGHTRED);
        both(c, cx - 1, mouth_y + 3, LIGHTRED);
    }
    if kind == Scales::Dragon || look.fangs {
        c.pen = Part::Detail;
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
            c.pen = Part::Body;
            c.oval(x, ey, 2, 2, s);
        }
        c.pen = Part::Eye;
        c.rect(x - 1, ey, x + 2, ey + 1, look.eyes);
        c.set(x, ey, BLACK);
        if look.glow {
            c.light(x - 1, ey, look.eyes);
        }
    }
    Anchors { eyes: vec![(cx - gap, ey), (cx + gap, ey)], brow: None, mouth: Some((cx, mouth_y, (rx - 3).max(1))), crown: (cx, cy - ry), lid: s, whites: false }
}

/// An insect's, a spider's or a worm's face.
fn paint_bug(c: &mut Canvas, look: &Look, kind: Bug) -> Anchors {
    let s = look.skin;
    let (cx, cy, rx, ry) = (CX, 12, 7, 7);
    c.pen = Part::Body;
    c.oval(cx, 25, 10, 5, s);
    c.rect(1, 23, W - 1, H, s);
    if kind == Bug::Insect {
        c.pen = Part::Detail;
        for side in [-1, 1] {
            line(c, (cx + side * 2, cy - ry + 1), (cx + side * 6, 0), shade(s));
        }
        c.pen = Part::Body;
    }
    c.oval(cx, cy, rx, ry, s);
    c.set(cx - 3, cy - ry + 2, lighter(s));
    c.set(cx - 2, cy - ry + 2, lighter(s));
    let eyes = match kind {
        Bug::Insect => {
            // Compound eyes, and mandibles.
            for side in [-1, 1] {
                c.pen = Part::Eye;
                c.oval(cx + side * 4, cy - 1, 2, 2, look.eyes);
                c.set(cx + side * 4, cy - 2, lighter(look.eyes));
                c.pen = Part::Mouth;
                line(c, (cx + side * 3, cy + ry - 2), (cx + side, cy + ry + 1), BROWN);
            }
            vec![(cx - 4, cy - 1), (cx + 4, cy - 1)]
        }
        Bug::Spider => {
            // Eight eyes, two big; fangs.
            for side in [-1, 1] {
                c.pen = Part::Eye;
                c.light(cx + side * 2, cy - 2, look.eyes);
                c.light(cx + side * 2, cy - 1, look.eyes);
                for (dx, dy) in [(3, -4), (4, -1), (5, -3)] {
                    c.light(cx + side * dx, cy + dy, look.eyes);
                }
                c.pen = Part::Mouth;
                c.rect(cx + side * 2, cy + ry - 2, cx + side * 2 + 1, cy + ry + 1, BLACK);
            }
            vec![(cx - 2, cy - 2), (cx + 2, cy - 2)]
        }
        Bug::Worm => {
            // A round mouth of teeth.
            c.pen = Part::Mouth;
            c.oval(cx, cy + 1, 4, 4, BLACK);
            c.pen = Part::Detail;
            for (dx, dy) in [(0, -3), (2, -2), (3, 0), (2, 2), (0, 3), (-2, 2), (-3, 0), (-2, -2)] {
                c.set(cx + dx, cy + 1 + dy, WHITE);
            }
            Vec::new()
        }
    };
    let mouth = if kind == Bug::Worm { (cx, cy + 1, 3) } else { (cx, cy + ry, 1) };
    Anchors { eyes, brow: None, mouth: Some(mouth), crown: (cx, cy - ry), lid: s, whites: false }
}

/// A skull: the undead with no flesh to speak of.
fn paint_skull(c: &mut Canvas, look: &Look, race: &str) -> Anchors {
    let bone = look.skin;
    let mummy = key(race).contains("mummy");
    let (cx, cy, rx, ry) = (CX, 10, 7, 8);
    shoulders(c, look, if key(race).contains("lich") { MAGENTA } else { DARKGRAY });
    c.pen = Part::Body;
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
    c.pen = Part::Eye;
    for side in [-1, 1] {
        c.oval(cx + side * 3, cy - 1, 1, 1, BLACK);
        c.light(cx + side * 3, cy - 1, look.eyes);
    }
    c.pen = Part::Detail;
    c.set(cx, cy + 2, BLACK);
    c.rect(cx - 1, cy + 3, cx + 2, cy + 4, BLACK);
    c.pen = Part::Mouth;
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
    Anchors { eyes: vec![(cx - 3, cy - 1), (cx + 3, cy - 1)], brow: None, mouth: Some((cx, cy + 5, 3)), crown: (cx, cy - ry), lid: BLACK, whites: false }
}

/// A being of stuff: fire, water, air, earth, a golem, a slime, a plant.
fn paint_stuff(c: &mut Canvas, look: &Look, stuff: Stuff, rng: &mut Rng) -> Anchors {
    let s = look.skin;
    let light = lighter(s);
    let (cx, cy, rx, ry) = (CX, 11, 7, 9);
    c.pen = Part::Body;
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
            c.pen = Part::Eye;
            for side in [-1, 1] {
                c.light(cx + side * 3, cy, look.eyes);
                c.light(cx + side * 3 + side, cy, look.eyes);
            }
            c.pen = Part::Mouth;
            c.rect(cx - 3, cy + 5, cx + 4, cy + 6, BLACK);
            Anchors { eyes: vec![(cx - 3, cy), (cx + 3, cy)], brow: None, mouth: Some((cx, cy + 5, 3)), crown: (cx, 2), lid: s, whites: false }
        }
        Stuff::Slime => {
            c.oval(cx, 17, 10, 10, s);
            c.rect(0, 22, W, H, s);
            c.oval(cx - 4, 12, 1, 2, light);
            c.pen = Part::Eye;
            for side in [-1, 1] {
                c.oval(cx + side * 3, 15, 1, 1, WHITE);
                c.set(cx + side * 3, 15, BLACK);
            }
            Anchors { eyes: vec![(cx - 3, 15), (cx + 3, 15)], brow: None, mouth: Some((cx, 19, 2)), crown: (cx, 7), lid: s, whites: false }
        }
        Stuff::Plant => {
            // Bark for a face, leaves about it.
            c.pen = Part::Detail;
            for _ in 0..70 {
                let (x, y) = (rng.between(0, W - 1), rng.between(0, 16));
                if within(x, y, (cx, 8, rx + 4, ry)) {
                    c.set(x, y, if rng.chance(50) { GREEN } else { LIGHTGREEN });
                }
            }
            c.pen = Part::Body;
            c.rect(cx - 4, 6, cx + 5, H, BROWN);
            for y in (6..H).step_by(2) {
                c.set(cx - 2 + (y % 5), y, shade(BROWN));
            }
            c.pen = Part::Eye;
            for side in [-1, 1] {
                c.light(cx + side * 2, 10, YELLOW);
            }
            c.pen = Part::Mouth;
            c.oval(cx, 15, 1, 1, BLACK);
            Anchors { eyes: vec![(cx - 2, 10), (cx + 2, 10)], brow: None, mouth: Some((cx, 15, 1)), crown: (cx, 0), lid: BROWN, whites: false }
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
            c.pen = Part::Eye;
            for side in [-1, 1] {
                c.light(cx + side * 3, cy - 1, WHITE);
                c.light(cx + side * 3 + side, cy - 1, WHITE);
            }
            c.pen = Part::Mouth;
            c.rect(cx - 2, cy + 4, cx + 3, cy + 5, shade(a));
            Anchors { eyes: vec![(cx - 3, cy - 1), (cx + 3, cy - 1)], brow: None, mouth: Some((cx, cy + 4, 2)), crown: (cx, cy - ry), lid: a, whites: false }
        }
    }
}

/// Paints over a pixel of the figure, never the air around it or what's worn.
fn on(c: &mut Canvas, x: i32, y: i32, color: u8, part: Part) {
    if c.inside(x, y) && c.get(x, y) != CLEAR && c.part(x, y) != Part::Worn {
        c.put(x, y, color, false, part);
    }
}

/// A color that shows on `base`: `want`, else a lighter or darker one of it.
fn against(want: u8, base: u8) -> u8 {
    [want, lighter(want), shade(want)].into_iter().find(|&c| c != base).unwrap_or(want)
}

/// Closes the eyes: what was eye is eyelid.
fn close(c: &mut Canvas, a: &Anchors) {
    for &(x, y) in &a.eyes {
        for dy in -2..=2 {
            for dx in -2..=2 {
                if c.part(x + dx, y + dy) == Part::Eye {
                    c.put(x + dx, y + dy, a.lid, false, Part::Body);
                }
            }
        }
    }
}

/// Brows moved: each eye's brow from its inner end to its outer, rows
/// up from the eye (1 is just over it).
fn brows_to(c: &mut Canvas, a: &Anchors, inner: i32, middle: i32, outer: i32) {
    let Some(row) = a.brow else { return };
    let mut color = shade(a.lid);
    for y in 0..c.h {
        for x in 0..c.w {
            if c.part(x, y) == Part::Brow {
                color = c.get(x, y);
                c.put(x, y, a.lid, false, Part::Body);
            }
        }
    }
    let above = a.eyes.first().map_or(1, |&(_, ey)| ey - row);
    for (i, &(x, ey)) in a.eyes.iter().enumerate() {
        let toward = if i == 0 { 1 } else { -1 };
        for (dx, up) in [(toward, inner), (0, middle), (-toward, outer)] {
            on(c, x + dx, ey - above + 1 - up, color, Part::Brow);
        }
    }
}

/// The mood, at the face's eyes, brows and mouth.
fn mood(c: &mut Canvas, a: &Anchors, mood: Mood, look: &Look) {
    let dark = against(shade(a.lid), a.lid);
    match mood {
        Mood::Calm => {
            // The evil smile on one side.
            if let (true, true, Some((mx, my, half))) = (look.evil, a.whites, a.mouth) {
                on(c, mx + half + 1, my - 1, dark, Part::Mouth);
                on(c, mx + half, my, a.lid, Part::Body);
            }
        }
        Mood::Asleep => {
            close(c, a);
            for &(x, y) in &a.eyes {
                for dx in -1..=1 {
                    on(c, x + dx, y, dark, Part::Body);
                }
            }
        }
        Mood::Pain => {
            close(c, a);
            for (i, &(x, y)) in a.eyes.iter().enumerate() {
                let out = if i == 0 { -1 } else { 1 };
                for (dx, dy) in [(out, -1), (0, 0), (out, 1)] {
                    on(c, x + dx, y + dy, dark, Part::Body);
                }
            }
            brows_to(c, a, 2, 1, 1);
            if let Some((mx, my, half)) = a.mouth {
                for x in mx - (half - 1).max(0)..=mx + (half - 1).max(0) {
                    on(c, x, my, BLACK, Part::Mouth);
                    on(c, x, my + 1, BLACK, Part::Mouth);
                }
                if half >= 2 {
                    on(c, mx, my + 1, RED, Part::Mouth);
                }
            }
        }
        Mood::Angry => {
            if a.whites {
                brows_to(c, a, 0, 1, 2);
            } else {
                // A glare: the brow's fur drawn down over the eyes' inner corners.
                for (i, &(x, y)) in a.eyes.iter().enumerate() {
                    let inner = if i == 0 { 1 } else { -1 };
                    on(c, x + inner, y - 1, dark, Part::Body);
                    on(c, x, y - 1, dark, Part::Body);
                }
            }
            if let Some((mx, my, half)) = a.mouth {
                if a.whites {
                    // A frown: the mouth's corners down.
                    on(c, mx - half, my, a.lid, Part::Body);
                    on(c, mx + half, my, a.lid, Part::Body);
                    on(c, mx - half, my + 1, dark, Part::Mouth);
                    on(c, mx + half, my + 1, dark, Part::Mouth);
                } else {
                    // Fangs bared.
                    for x in mx - half..=mx + half {
                        on(c, x, my, WHITE, Part::Mouth);
                    }
                    on(c, mx - half - 1, my, dark, Part::Mouth);
                    on(c, mx + half + 1, my, dark, Part::Mouth);
                }
            }
        }
        Mood::Afraid => {
            for &(x, y) in &a.eyes {
                let ring = if a.whites { WHITE } else { c.get(x, y) };
                on(c, x, y - 1, ring, Part::Eye);
                if a.whites {
                    on(c, x - 1, y, WHITE, Part::Eye);
                    on(c, x + 1, y, WHITE, Part::Eye);
                }
            }
            brows_to(c, a, 3, 3, 2);
            if let Some((mx, my, _)) = a.mouth {
                on(c, mx, my, BLACK, Part::Mouth);
                on(c, mx, my + 1, BLACK, Part::Mouth);
            }
        }
        Mood::Dead => {
            close(c, a);
            for &(x, y) in &a.eyes {
                for (dx, dy) in [(-1, -1), (1, 1), (1, -1), (-1, 1), (0, 0)] {
                    on(c, x + dx, y + dy, if a.lid == BLACK { DARKGRAY } else { against(BLACK, a.lid) }, Part::Body);
                }
            }
            if let Some((mx, my, half)) = a.mouth {
                for x in mx - half..=mx + half {
                    on(c, x, my, dark, Part::Mouth);
                }
            }
            // The color going out of it.
            for y in 0..c.h {
                for x in 0..c.w {
                    if c.part(x, y) == Part::Body {
                        let here = c.get(x, y);
                        c.put(x, y, Canvas::mix(x, y, here, shade(here), 6), false, Part::Body);
                    }
                }
            }
        }
    }
}

/// How a shape shows its hurt.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Wounds {
    /// Flesh: cuts, bruises, blood of this color.
    Bleed(u8),
    /// Fur and feathers: torn, then matted with blood.
    Mat,
    /// Stone, bone, wood: cracks, then a piece gone.
    Crack,
    /// A ghost, fire, water, air: less and less of it there.
    Fade,
    /// A slime: smaller.
    Shrink,
}

fn wounds(look: &Look) -> Wounds {
    match look.shape {
        Shape::Person if look.ghostly => Wounds::Fade,
        Shape::Person | Shape::Scaled(_) => Wounds::Bleed(RED),
        Shape::Bug(_) => Wounds::Bleed(GREEN),
        Shape::Beast(_) | Shape::Bird => Wounds::Mat,
        Shape::Skull | Shape::Stuff(Stuff::Golem | Stuff::Earth | Stuff::Plant) => Wounds::Crack,
        Shape::Stuff(Stuff::Slime) => Wounds::Shrink,
        Shape::Stuff(_) => Wounds::Fade,
    }
}

/// The hurt, in five steps: 1 scratches, 2 cuts, 3 bruises (fur
/// matted), 4 bleeding, 5 near death. Where each wound goes comes from
/// the name alone, so a step only adds to the one before; only where
/// it'll be seen once the figure's laid at `offset` (`figure::lay`).
fn hurt(c: &mut Canvas, look: &Look, name: &str, step: u8, offset: (i32, i32)) {
    if step == 0 {
        return;
    }
    let step = step.min(5);
    let mut rng = Rng::new(seed("wounds", &key(name), 0));
    let bottom = c.h - 2;
    let seen = |x: i32, y: i32| x - offset.0 >= 1 && y + offset.1 < bottom;
    let body: Vec<(i32, i32)> = (0..c.h).flat_map(|y| (0..c.w).map(move |x| (x, y))).filter(|&(x, y)| c.part(x, y) == Part::Body && seen(x, y)).collect();
    if body.is_empty() {
        return;
    }
    let spots: Vec<(i32, i32)> = (0..8).map(|_| body[rng.below(body.len() as i32) as usize]).collect();
    let mark = |c: &mut Canvas, x: i32, y: i32, color: u8| {
        if matches!(c.part(x, y), Part::Body | Part::Hair) || (c.part(x, y) == Part::Detail && c.get(x, y) != CLEAR) {
            c.put(x, y, color, false, Part::Detail);
        }
    };
    let style = wounds(look);
    match style {
        Wounds::Bleed(_) | Wounds::Mat => {
            let fur = style == Wounds::Mat;
            let blood = if let Wounds::Bleed(b) = style { b } else { RED };
            // Scratches; on fur, tufts out of place.
            for &(x, y) in &spots[0..2] {
                let base = c.get(x, y);
                mark(c, x, y, if fur { lighter(base) } else { shade(base) });
                mark(c, x + 1, y + 1, shade(base));
            }
            let cut = |c: &mut Canvas, (x, y): (i32, i32)| {
                let base = c.get(x, y);
                for i in 0..3 {
                    mark(c, x + i, y - i, against(blood, base));
                }
            };
            if step >= 2 {
                for &s in &spots[2..4] {
                    cut(c, s);
                }
            }
            if step >= 3 {
                for &(x, y) in &spots[4..6] {
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0)] {
                        let base = c.get(x + dx, y + dy);
                        let bruise = if fur { Canvas::mix(x + dx, y + dy, against(RED, base), shade(base), 8) } else { Canvas::mix(x + dx, y + dy, MAGENTA, BLUE, 6) };
                        mark(c, x + dx, y + dy, bruise);
                    }
                }
            }
            if step >= 4 {
                cut(c, spots[6]);
                // Blood running down from the cuts, over clothes too.
                for (i, &(x, y)) in spots[2..4].iter().chain(&spots[6..7]).enumerate() {
                    for dy in 1..=2 + i as i32 + i32::from(step) - 3 {
                        let (px, py) = (x, y + dy);
                        if c.inside(px, py) && c.get(px, py) != CLEAR && c.part(px, py) != Part::Eye {
                            c.put(px, py, against(blood, c.get(px, py)), false, Part::Detail);
                        }
                    }
                }
            }
            if step >= 5 {
                cut(c, spots[7]);
                for y in 0..c.h {
                    for x in 0..c.w {
                        if c.part(x, y) == Part::Body {
                            let here = c.get(x, y);
                            c.put(x, y, Canvas::mix(x, y, here, shade(here), 5), false, Part::Body);
                        }
                    }
                }
            }
        }
        Wounds::Crack => {
            for &(x0, y0) in spots.iter().take(step as usize) {
                let (mut x, mut y) = (x0, y0);
                for _ in 0..3 + step {
                    let base = c.get(x, y);
                    if c.part(x, y) == Part::Body {
                        c.put(x, y, if base == BLACK { DARKGRAY } else { BLACK }, false, Part::Detail);
                    }
                    x += if rng.chance(50) { 1 } else { -1 };
                    y += 1;
                }
            }
            if step >= 5 {
                let (x, y) = spots[7];
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if c.part(x + dx, y + dy) == Part::Body {
                            c.put(x + dx, y + dy, CLEAR, false, Part::Back);
                        }
                    }
                }
            }
        }
        Wounds::Fade => {
            for y in 0..c.h {
                for x in 0..c.w {
                    let part = c.part(x, y);
                    if c.get(x, y) != CLEAR && part != Part::Eye && Canvas::mix(x, y, 0, 1, i32::from(step) * 2) == 1 {
                        c.put(x, y, CLEAR, false, Part::Back);
                    }
                }
            }
        }
        Wounds::Shrink => {
            for _ in 0..step {
                let edge: Vec<(i32, i32)> = (0..c.h)
                    .flat_map(|y| (0..c.w).map(move |x| (x, y)))
                    .filter(|&(x, y)| c.part(x, y) == Part::Body && [(x - 1, y), (x + 1, y), (x, y - 1)].iter().any(|&(nx, ny)| c.inside(nx, ny) && c.get(nx, ny) == CLEAR))
                    .collect();
                for (x, y) in edge {
                    c.put(x, y, CLEAR, false, Part::Back);
                }
            }
        }
    }
}

/// What an effect on a creature looks like.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Effect {
    Halo,
    Poisoned,
    Sick,
    Burning,
    Frozen,
    Blind,
    Invisible,
    Outlined,
    Webbed,
    Stone,
    Bark,
    Asleep,
    Afraid,
    Raging,
    Glowing,
}

/// An effect's look, by its name as `char.effects` gives it ("Poison")
/// or as AFFECTS shows it ("(Sleep spell)"): `effect_looks.tsv`.
fn effect(name: &str) -> Option<Effect> {
    let plain = name.trim().trim_start_matches('(').trim_end_matches(')').trim().to_lowercase();
    let plain = plain.strip_suffix(" spell").unwrap_or(&plain);
    let look = include_str!("effect_looks.tsv").lines().skip(1).find_map(|l| l.split_once('\t').filter(|(n, _)| *n == plain).map(|(_, look)| look))?;
    Some(match look {
        "halo" => Effect::Halo,
        "poisoned" => Effect::Poisoned,
        "sick" => Effect::Sick,
        "burning" => Effect::Burning,
        "frozen" => Effect::Frozen,
        "blind" => Effect::Blind,
        "invisible" => Effect::Invisible,
        "outlined" => Effect::Outlined,
        "webbed" => Effect::Webbed,
        "stone" => Effect::Stone,
        "bark" => Effect::Bark,
        "asleep" => Effect::Asleep,
        "afraid" => Effect::Afraid,
        "raging" => Effect::Raging,
        "glowing" => Effect::Glowing,
        _ => return None,
    })
}

/// The effects that change the figure's own pixels: a tint, a skin, its eyes.
fn on_figure(c: &mut Canvas, effects: &[Effect]) {
    for y in 0..c.h {
        for x in 0..c.w {
            let (here, part) = (c.get(x, y), c.part(x, y));
            if here == CLEAR {
                continue;
            }
            let mut color = here;
            if part == Part::Body {
                for e in effects {
                    color = match e {
                        Effect::Poisoned | Effect::Sick => Canvas::mix(x, y, color, if color == GREEN { LIGHTGREEN } else { GREEN }, if *e == Effect::Sick { 4 } else { 6 }),
                        Effect::Frozen => Canvas::mix(x, y, color, LIGHTCYAN, 5),
                        Effect::Stone => Canvas::mix(x, y, LIGHTGRAY, DARKGRAY, 6),
                        Effect::Bark => if x % 3 == 0 { RED } else { BROWN },
                        _ => color,
                    };
                }
            }
            if part == Part::Eye {
                if effects.contains(&Effect::Blind) {
                    color = LIGHTGRAY;
                } else if effects.contains(&Effect::Raging) {
                    c.put(x, y, LIGHTRED, true, part);
                    continue;
                }
            }
            if color != here {
                c.put(x, y, color, false, part);
            }
        }
    }
    // Frost rimes the top of it.
    if effects.contains(&Effect::Frozen) {
        for y in 1..c.h {
            for x in 0..c.w {
                if c.get(x, y) != CLEAR && c.get(x, y - 1) == CLEAR && x % 2 == 0 {
                    let part = c.part(x, y);
                    c.put(x, y, WHITE, true, part);
                }
            }
        }
    }
}

/// The effects drawn around the figure once it's laid on its place.
fn around(c: &mut Canvas, effects: &[Effect], crown: (i32, i32), rng: &mut Rng) {
    let figure = |c: &Canvas, x: i32, y: i32| figure::of_figure(c, x, y);
    if effects.contains(&Effect::Outlined) || effects.contains(&Effect::Glowing) {
        let (color, outside) = if effects.contains(&Effect::Outlined) { (LIGHTMAGENTA, false) } else { (YELLOW, true) };
        let mut lit = Vec::new();
        for y in 0..c.h {
            for x in 0..c.w {
                let near = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)].iter().any(|&(nx, ny)| c.inside(nx, ny) && figure(c, nx, ny) != figure(c, x, y));
                if near && figure(c, x, y) != outside && (outside || (x + y) % 2 == 0) {
                    lit.push((x, y));
                }
            }
        }
        for (x, y) in lit {
            let part = c.part(x, y);
            c.put(x, y, color, true, part);
        }
    }
    let (w, h) = (c.w, c.h);
    if effects.contains(&Effect::Halo) {
        let (cx, cy) = crown;
        let half = w / 6;
        for x in cx - half..=cx + half {
            let y = (cy - 2 + i32::from((x - cx).abs() > half / 2)).max(0);
            c.put(x, y, YELLOW, true, Part::Detail);
        }
    }
    if effects.contains(&Effect::Webbed) {
        c.pen = Part::Detail;
        for (from, to) in [((0, 3 * h / 24), (w - 1, h - 7 * h / 24)), ((w - 1, h / 12), (0, h - 9 * h / 24)), ((0, h - h / 6), (w - 1, h / 4))] {
            line(c, from, to, LIGHTGRAY);
        }
    }
    if effects.contains(&Effect::Burning) {
        // Flames up from the bottom, lit.
        let mut x = rng.between(0, 2);
        while x < w {
            let tall = rng.between(3, 6) * h / 24;
            for y in h - tall..h {
                let tip = y < h - tall + 2;
                c.put(x, y, if tip { YELLOW } else { RED }, true, Part::Detail);
                if !tip && x + 1 < w {
                    c.put(x + 1, y, LIGHTRED, true, Part::Detail);
                }
            }
            x += rng.between(3, 5);
        }
    }
}

/// A creature's face, 25 by 24 pixels in 25 by 12 cells: the Bestiary's
/// still picture, calm and unhurt.
#[cfg(test)]
pub fn face(sitter: &Sitter) -> Art {
    face_in(sitter, &Moment::default())
}

/// A creature drawn on a clear canvas of its own, as it feels and is
/// hurt now, with what's on it: everything but the place behind it.
struct Drawn {
    figure: Canvas,
    /// Where it's laid: an animal low in the frame, its back running off
    /// the side.
    offset: (i32, i32),
    crown: (i32, i32),
    feeling: Mood,
    effects: Vec<Effect>,
    animal: bool,
    fish: bool,
    /// The color behind it when it's nowhere known.
    backdrop: u8,
    rng: Rng,
}

impl Drawn {
    fn invisible(&self) -> bool {
        self.effects.contains(&Effect::Invisible)
    }

    /// Lays it on `c` (its own size), then what shows around it.
    fn lay_on(&mut self, c: &mut Canvas, light: Option<&figure::Place>) {
        let invisible = self.invisible();
        figure::lay(c, &self.figure, self.offset, &|x, y| invisible && (x + y) % 2 == 1);
        if let (Some(light), false) = (light, invisible) {
            figure::light(c, light);
        }
        let crown = (self.crown.0 - self.offset.0, self.crown.1 + self.offset.1);
        around(c, &self.effects, crown, &mut self.rng);
        if self.feeling == Mood::Asleep {
            let (x, y) = ((crown.0 + 6).min(c.w - 3), (crown.1 - 2).max(0));
            c.glyph(x, y, 'z', WHITE);
            c.glyph(x + 2, (y - 2).max(0), 'Z', WHITE);
        }
    }
}

fn drawn(sitter: &Sitter, moment: &Moment, size: Size) -> Drawn {
    let look = read(sitter);
    let name = key(&sitter.name);
    let effects: Vec<Effect> = moment.effects.iter().filter_map(|e| effect(e)).collect();
    let feeling = match moment.mood {
        Mood::Calm if effects.contains(&Effect::Asleep) => Mood::Asleep,
        Mood::Calm if effects.contains(&Effect::Afraid) => Mood::Afraid,
        Mood::Calm if effects.contains(&Effect::Raging) => Mood::Angry,
        m => m,
    };
    let (columns, rows) = size.cells();
    let mut figure = figure::clear(columns, rows);
    let mut rng = Rng::new(seed("face-stuff", &name, 0));
    let race = sitter.race.as_deref().unwrap_or("");
    let anchors = match (size, look.shape) {
        (Size::Small, _) => small::paint(&mut figure, &look, race, &mut rng),
        (_, Shape::Person) => paint_person(&mut figure, &look),
        (_, Shape::Beast(ears)) => paint_beast(&mut figure, &look, ears),
        (_, Shape::Bird) => paint_bird(&mut figure, &look),
        (_, Shape::Scaled(kind)) => paint_scaled(&mut figure, &look, kind),
        (_, Shape::Bug(kind)) => paint_bug(&mut figure, &look, kind),
        (_, Shape::Skull) => paint_skull(&mut figure, &look, race),
        (_, Shape::Stuff(stuff)) => paint_stuff(&mut figure, &look, stuff, &mut rng),
    };
    mood(&mut figure, &anchors, feeling, &look);
    on_figure(&mut figure, &effects);
    let offset = match (look.animal, size) {
        (false, _) => (0, 0),
        (true, Size::Full) => (3, 2),
        (true, Size::Small) => (2, 1),
    };
    hurt(&mut figure, &look, &sitter.name, moment.hurt, offset);
    Drawn { figure, offset, crown: anchors.crown, feeling, effects, animal: look.animal, fish: look.shape == Shape::Scaled(Scales::Fish), backdrop: look.backdrop, rng }
}

/// A creature's face as it is now: its mood, its hurt, what's on it and
/// where it is.
pub fn face_in(sitter: &Sitter, moment: &Moment) -> Art {
    let name = key(&sitter.name);
    let mut drawn = drawn(sitter, moment, Size::Full);
    let fish = drawn.fish;
    // The place: where it is now, else where it's most seen (the wild for
    // an animal), else a fish's water, else a backdrop of one color.
    let place = if let Some(scene) = &moment.scene {
        Some(figure::place(&name, scene, SIZE.0, SIZE.1))
    } else if let Some(t) = sitter.terrain.as_deref().filter(|t| !t.is_empty()) {
        let t = if drawn.animal { figure::wild(t, &name) } else { t.to_string() };
        Some(figure::terrain(&name, if fish { "underwater" } else { &t }, SIZE.0, SIZE.1))
    } else if fish {
        Some(figure::terrain(&name, "underwater", SIZE.0, SIZE.1))
    } else {
        None
    };
    let (mut canvas, light) = match place {
        Some((canvas, light)) => (canvas, Some(light)),
        None => {
            let mut c = Canvas::new(SIZE.0, SIZE.1);
            backdrop(&mut c, drawn.backdrop);
            (c, None)
        }
    };
    drawn.lay_on(&mut canvas, light.as_ref());
    canvas.art()
}

/// A creature as it is now on a clear canvas of the face's `size`, with
/// nothing behind it, to stand in a picture of its own (the Fight panel,
/// `fight_art.rs`). Unlit: the picture it stands in lights it.
pub(crate) fn standing(sitter: &Sitter, moment: &Moment, size: Size) -> Canvas {
    let (columns, rows) = size.cells();
    let mut c = figure::clear(columns, rows);
    drawn(sitter, moment, size).lay_on(&mut c, None);
    c
}

/// A creature known by its name alone (one the Bestiary hasn't met): the
/// race its name says, else a person, plain rather than made up.
pub fn sitter_named(name: &str) -> Sitter {
    let race = crate::bestiary::race_in_name(name);
    Sitter { name: name.to_string(), race: race.map(|r| r.name.to_string()), category: race.map(|r| r.category.to_string()), ..Sitter::default() }
}

/// The player's character: their race from `char.base`, nothing else
/// asked of the game for it.
pub fn player(name: &str, race: Option<&str>) -> Sitter {
    let known = race.and_then(crate::bestiary::race_named);
    Sitter {
        name: name.to_string(),
        race: race.map(str::to_string),
        category: known.map(|r| r.category.to_string()),
        skilled: true,
        ..Sitter::default()
    }
}

/// **Small faces**, 16 by 16 pixels in 16 by 8 cells, for the phone's
/// Fight strip (`fight_art.rs`, `Size::Small`): the same creatures, read
/// the same way (`Look`), each shape drawn again by its own few strokes
/// at this size, never the big face scaled down (a scaled face loses its
/// eyes). What they give back are the same anchors, so the moods, the
/// hurt and the effects go on them as on the big ones. Less is drawn:
/// what's worn on the head, eyes and neck, the beard, horns, tusks and
/// fangs, but no freckles, tattoo or earrings.
mod small {
    use super::*;

    const W: i32 = SMALL.0 as i32;
    const H: i32 = SMALL.1 as i32 * 2;
    /// The middle: a face is drawn the same either side of it.
    const CX: i32 = W / 2;

    /// A pixel and its mirror across the middle.
    fn both(c: &mut Canvas, x: i32, y: i32, color: u8) {
        c.set(x, y, color);
        c.set(2 * CX - x, y, color);
    }

    /// The face of `look`'s shape; `race` tells a mummy and a lich.
    pub(super) fn paint(c: &mut Canvas, look: &Look, race: &str, rng: &mut Rng) -> Anchors {
        match look.shape {
            Shape::Person => person(c, look),
            Shape::Beast(ears) => beast(c, look, ears),
            Shape::Bird => bird(c, look),
            Shape::Scaled(kind) => scaled(c, look, kind),
            Shape::Bug(kind) => bug(c, look, kind),
            Shape::Skull => skull(c, look, race),
            Shape::Stuff(stuff) => stuff_of(c, look, stuff, rng),
        }
    }

    /// Shoulders in the clothes (or what's worn on the torso), a cloak's edges.
    fn shoulders(c: &mut Canvas, look: &Look, color: u8) {
        c.pen = Part::Worn;
        let clothes = look.worn.torso.as_deref().and_then(item_color).unwrap_or(color);
        c.oval(CX, 16, 7, 3, clothes);
        c.rect(0, 15, W, H, clothes);
        if let Some(cloak) = look.worn.back.as_deref() {
            let color = item_color(cloak).unwrap_or(shade(look.clothes));
            for y in 13..H {
                let reach = (y - 12) / 2;
                c.rect(0, y, 1 + reach, y + 1, color);
                c.rect(2 * CX - reach, y, W, y + 1, color);
            }
        }
    }

    /// An animal's back, running off the right side from under its head.
    fn back(c: &mut Canvas, color: u8, top: i32) {
        c.pen = Part::Body;
        c.oval(CX + 4, top + 2, 8, 3, color);
        c.rect(CX, top + 1, W, H, color);
    }

    /// Something worn around the neck: a glint under the chin.
    fn neck(c: &mut Canvas, item: &str, y: i32) {
        c.pen = Part::Worn;
        c.light(CX, y, item_color(item).unwrap_or(YELLOW));
    }

    /// A thinking creature's brows over eyes `gap` from the middle at row `ey`.
    fn brows(c: &mut Canvas, ey: i32, gap: i32, color: u8) {
        c.pen = Part::Brow;
        c.rect(CX - gap - 1, ey - 1, CX - gap + 2, ey, color);
        c.rect(CX + gap - 1, ey - 1, CX + gap + 2, ey, color);
    }

    /// Things worn on the head of `oval`, its brow at `brow`.
    fn hat(c: &mut Canvas, look: &Look, item: &str, oval: (i32, i32, i32, i32), brow: i32) {
        c.pen = Part::Worn;
        let ws = words(item);
        let has = |any: &[&str]| ws.iter().any(|w| any.contains(&w.as_str()));
        let (cx, cy, rx, ry) = oval;
        let top = cy - ry;
        if has(&["helmet", "helm", "coif", "casque", "basinet", "bascinet", "sallet", "barbute", "greathelm"]) {
            let metal = item_color(item).unwrap_or(LIGHTGRAY);
            cap(c, (cx, cy, rx + 1, ry + 1), top - 1, brow, metal);
            c.rect(cx - rx - 1, brow - 1, cx + rx + 2, brow, shade(metal));
            if has(&["horned", "viking"]) || look.horns {
                both(c, cx - rx - 1, top, WHITE);
                both(c, cx - rx - 1, top - 1, WHITE);
            }
            c.light(cx - 1, top, WHITE);
        } else if has(&["crown", "tiara", "circlet", "diadem", "coronet"]) {
            let gold = item_color(item).unwrap_or(YELLOW);
            let band = top + 1;
            c.rect(cx - rx + 1, band, cx + rx, band + 1, gold);
            if has(&["crown", "coronet"]) {
                for x in [cx - rx + 1, cx, cx + rx - 1] {
                    c.set(x, band - 1, gold);
                }
            }
            c.light(cx, band, LIGHTRED);
        } else if has(&["hood", "cowl"]) {
            let cloth = item_color(item).unwrap_or(shade(look.clothes));
            cap(c, (cx, cy, rx + 1, ry + 1), top - 1, brow - 1, cloth);
            for y in brow - 1..cy + ry {
                both(c, cx - rx - 1, y, cloth);
            }
        } else if has(&["headband", "bandana", "bandanna", "band", "sweatband"]) {
            let cloth = item_color(item).unwrap_or(RED);
            c.rect(cx - rx, brow - 2, cx + rx + 1, brow - 1, cloth);
        } else if has(&["pointed", "wizard", "witch", "conical"]) {
            let cloth = item_color(item).unwrap_or(look.clothes);
            peak(c, cx, 0, top + 2, rx, cloth);
            c.rect(cx - rx - 2, top + 2, cx + rx + 3, top + 3, cloth);
        } else {
            // A hat or a cap: a crown and a brim.
            let cloth = item_color(item).unwrap_or(look.clothes);
            cap(c, (cx, cy, rx + 1, ry + 1), top - 1, top + 2, cloth);
            if has(&["hat", "fedora", "tricorn", "tricorne", "sombrero", "bonnet"]) {
                c.rect(cx - rx - 2, top + 2, cx + rx + 3, top + 3, cloth);
            } else {
                c.rect(cx - rx - 1, top + 2, cx + rx + 2, top + 3, shade(cloth));
            }
        }
    }

    /// Things worn on the eyes at row `y`, the eyes `gap` from the middle.
    fn eyewear(c: &mut Canvas, item: &str, y: i32, gap: i32) {
        c.pen = Part::Worn;
        let ws = words(item);
        if ws.iter().any(|w| ["blindfold", "bandage", "veil"].contains(&w.as_str())) {
            c.rect(CX - gap - 2, y, CX + gap + 3, y + 1, item_color(item).unwrap_or(DARKGRAY));
        } else if ws.iter().any(|w| ["patch", "eyepatch"].contains(&w.as_str())) {
            c.rect(CX - gap - 1, y, CX - gap + 1, y + 1, BLACK);
            line(c, (CX - gap - 3, y - 2), (CX + gap + 3, y - 1), BLACK);
        } else {
            // Rims over the eyes and the bridge between.
            let frame = item_color(item).unwrap_or(LIGHTGRAY);
            c.rect(CX - gap - 1, y - 1, CX - gap + 2, y, frame);
            c.rect(CX + gap - 1, y - 1, CX + gap + 2, y, frame);
            c.set(CX, y, frame);
        }
    }

    /// A person's face.
    fn person(c: &mut Canvas, look: &Look) -> Anchors {
        let (cx, cy, rx, ry) = (CX, 7, 4, 5);
        let (skin, hair) = (look.skin, look.hair);
        let dark = shade(skin);
        if let Feature::Wings(color) = look.feature {
            c.pen = Part::Detail;
            for i in 0..6 {
                let reach = 2 + (6 - i) / 2;
                c.rect(0, 8 + i, reach, 9 + i, color);
                c.rect(2 * CX + 1 - reach, 8 + i, W, 9 + i, color);
            }
        }
        if let (Some(h), true) = (hair, look.long_hair) {
            c.pen = Part::Hair;
            c.rect(cx - rx - 1, cy - 2, cx + rx + 2, 15, h);
        }
        shoulders(c, look, look.clothes);
        c.pen = Part::Body;
        c.rect(cx - 1, cy + ry - 1, cx + 2, 15, skin);
        c.rect(cx - 1, cy + ry, cx + 2, cy + ry + 1, dark);
        c.oval(cx, cy, rx, ry, skin);
        both(c, cx - rx - 1, cy, skin);
        if look.feature == Feature::PointedEars {
            both(c, cx - rx - 2, cy - 1, skin);
        }
        if let Some(h) = hair {
            c.pen = Part::Hair;
            cap(c, (cx, cy, rx + 1, ry + 1), cy - ry - 1, cy - 2, h);
            both(c, cx - rx, cy - 2, h);
            if look.long_hair {
                for y in cy - 2..cy + ry - 1 {
                    both(c, cx - rx - 1, y, h);
                }
            }
            c.set(cx - 1, cy - ry, lighter(h));
        }
        let (ey, gap) = (cy - 1, 2);
        brows(c, ey, gap, hair.map(shade).unwrap_or(dark));
        let eyes = |c: &mut Canvas| {
            c.pen = Part::Eye;
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
        c.pen = Part::Body;
        if look.old {
            both(c, cx - gap - 1, ey + 1, dark);
        }
        c.set(cx, cy + 1, dark);
        let mouth_y = cy + 3;
        c.pen = Part::Mouth;
        c.rect(cx - 1, mouth_y, cx + 2, mouth_y + 1, if skin == RED || skin == LIGHTRED { RED } else { shade(dark) });
        if look.scar {
            c.pen = Part::Body;
            line(c, (cx + gap, ey - 2), (cx + gap + 1, cy + 2), if skin == LIGHTRED { WHITE } else { dark });
            eyes(c);
        }
        let beard_color = hair.unwrap_or(LIGHTGRAY);
        if look.beard {
            c.pen = Part::Hair;
            for y in cy + 2..cy + ry + 2 {
                for x in 0..W {
                    if within(x, y, (cx, cy + 1, rx, ry)) && (y > mouth_y || (x - cx).abs() > 1) {
                        c.set(x, y, beard_color);
                    }
                }
            }
            c.pen = Part::Mouth;
            c.rect(cx - 1, mouth_y, cx + 2, mouth_y + 1, shade(beard_color));
        }
        if look.mustache {
            c.pen = Part::Hair;
            c.rect(cx - 1, mouth_y - 1, cx + 2, mouth_y, beard_color);
        }
        c.pen = Part::Detail;
        if look.tusks {
            both(c, cx - 2, mouth_y - 1, WHITE);
        }
        if look.fangs {
            both(c, cx - 1, mouth_y + 1, WHITE);
        }
        if look.horns {
            for i in 0..3 {
                both(c, cx - rx + 1 - i / 2, cy - ry - i, if i < 2 { DARKGRAY } else { LIGHTGRAY });
            }
        }
        if let Some(item) = look.worn.eyes.as_deref() {
            eyewear(c, item, ey, gap);
        } else if look.patch {
            eyewear(c, "eyepatch", ey, gap);
        }
        if let Some(item) = look.worn.mouth.as_deref() {
            c.pen = Part::Worn;
            if words(item).iter().any(|w| w == "pipe") {
                c.rect(cx + 1, mouth_y, cx + 4, mouth_y + 1, BROWN);
                c.light(cx + 3, mouth_y - 1, LIGHTRED);
            } else {
                cap(c, (cx, cy, rx, ry), ey + 2, cy + ry + 1, item_color(item).unwrap_or(DARKGRAY));
            }
        }
        if let Some(item) = look.worn.neck.as_deref() {
            neck(c, item, cy + ry + 1);
        }
        match look.worn.head.as_deref() {
            Some(item) => hat(c, look, item, (cx, cy, rx, ry), ey - 1),
            None if look.feature == Feature::TallHat => hat(c, look, "a tall pointed red hat", (cx, cy, rx, ry), ey - 1),
            None => {}
        }
        if look.halo {
            c.pen = Part::Detail;
            for x in cx - 2..=cx + 2 {
                c.light(x, (cy - ry - 2 + i32::from((x - cx).abs() > 1)).max(0), YELLOW);
            }
        }
        Anchors { eyes: vec![(cx - gap, ey), (cx + gap, ey)], brow: Some(ey - 1), mouth: Some((cx, mouth_y, 1)), crown: (cx, cy - ry), lid: skin, whites: true }
    }

    /// A beast's face: a muzzle, its ears; a thinking one in clothes, an
    /// animal's back running off the side.
    fn beast(c: &mut Canvas, look: &Look, ears: Ears) -> Anchors {
        let fur = look.skin;
        let (light, dark) = (lighter(fur), shade(fur));
        let (cx, cy, rx, ry) = (CX, 8, 5, 5);
        let top = cy - ry;
        if look.sapient {
            shoulders(c, look, look.clothes);
        } else {
            back(c, fur, 11);
        }
        c.pen = Part::Body;
        for side in [-1, 1] {
            match ears {
                Ears::Pointed => {
                    peak(c, cx + side * 3, top - 3, top + 2, 1, fur);
                    c.set(cx + side * 3, top - 1, LIGHTRED);
                }
                Ears::Round => {
                    c.oval(cx + side * 4, top, 1, 1, fur);
                    c.set(cx + side * 4, top, dark);
                }
                Ears::Wide => {
                    c.oval(cx + side * 6, cy - 2, 2, 3, fur);
                    c.rect(cx + side * 6, cy - 3, cx + side * 6 + 1, cy, dark);
                }
                Ears::Long => {
                    c.rect(cx + side * 3, top - 4, cx + side * 3 + 1, top + 1, fur);
                }
                Ears::Horned => {
                    c.pen = Part::Detail;
                    for i in 0..3 {
                        c.set(cx + side * (rx - 1 + i), top + 1 - i / 2, LIGHTGRAY);
                    }
                    c.pen = Part::Body;
                }
                Ears::Curled => {
                    c.pen = Part::Detail;
                    c.oval(cx + side * (rx - 1), cy - 3, 2, 2, DARKGRAY);
                    c.set(cx + side * (rx - 1), cy - 3, LIGHTGRAY);
                    c.pen = Part::Body;
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
        let pig = fur == LIGHTMAGENTA;
        c.oval(cx, cy + 3, 3, 2, if pig { LIGHTRED } else { light });
        let nose_y = cy + 1;
        c.pen = Part::Detail;
        if pig || ears == Ears::Horned {
            both(c, cx - 1, nose_y + 1, BLACK);
        } else {
            c.rect(cx - 1, nose_y, cx + 2, nose_y + 1, BLACK);
            c.pen = Part::Mouth;
            c.set(cx, nose_y + 1, dark);
            both(c, cx - 1, nose_y + 2, dark);
        }
        c.pen = Part::Detail;
        if look.fangs || look.tusks {
            both(c, cx - 2, nose_y + 2, WHITE);
        }
        if matches!(ears, Ears::Pointed | Ears::Round) {
            for side in [-1, 1] {
                line(c, (cx + side * 3, nose_y + 1), (cx + side * 6, nose_y), LIGHTGRAY);
            }
        }
        let (ey, gap) = (cy - 2, 2);
        if look.sapient {
            brows(c, ey, gap, dark);
        }
        c.pen = Part::Eye;
        for side in [-1, 1] {
            let x = cx + side * gap;
            c.set(x, ey, look.eyes);
            c.set(x + side, ey, look.eyes);
            c.set(x - side, ey, if look.sapient { WHITE } else { BLACK });
            if look.glow {
                c.light(x, ey, LIGHTRED);
            }
        }
        c.pen = Part::Detail;
        if look.horns && ears != Ears::Horned {
            for i in 0..3 {
                both(c, cx - 2 - i / 2, top - i, WHITE);
            }
        }
        if look.scar {
            c.pen = Part::Body;
            line(c, (cx + 1, cy - 3), (cx + 3, cy), dark);
        }
        if let Some(item) = look.worn.neck.as_deref() {
            if look.sapient {
                neck(c, item, cy + ry + 1);
            } else {
                // A collar: it's someone's.
                c.pen = Part::Worn;
                c.rect(cx - 3, 13, cx + 4, 14, item_color(item).unwrap_or(RED));
                c.light(cx, 13, YELLOW);
            }
        }
        if let (true, Some(item)) = (look.sapient, look.worn.head.as_deref()) {
            hat(c, look, item, (cx, cy, rx, ry), ey - 1);
        }
        Anchors { eyes: vec![(cx - gap, ey), (cx + gap, ey)], brow: look.sapient.then_some(ey - 1), mouth: Some((cx, nose_y + 2, 1)), crown: (cx, top), lid: fur, whites: look.sapient }
    }

    /// A bird's: a round head, a hooked beak, eyes on the sides.
    fn bird(c: &mut Canvas, look: &Look) -> Anchors {
        let f = look.skin;
        let (cx, cy, rx, ry) = (CX, 7, 5, 5);
        if look.sapient {
            shoulders(c, look, look.clothes);
        } else {
            back(c, f, 10);
        }
        c.pen = Part::Body;
        c.oval(cx, cy, rx, ry, f);
        for i in 0..2 {
            line(c, (cx + i, cy - ry), (cx - 1 + i * 2, cy - ry - 2), f);
        }
        for y in (cy + 2..cy + ry + 2).step_by(2) {
            both(c, cx - rx + 1, y, shade(f));
        }
        c.pen = Part::Mouth;
        peak(c, cx, cy - 1, cy + 2, 1, YELLOW);
        c.set(cx, cy + 3, YELLOW);
        c.set(cx, cy + 4, BROWN);
        let (ey, gap) = (cy - 2, 3);
        if look.sapient {
            brows(c, ey, gap, shade(f));
        }
        c.pen = Part::Eye;
        for side in [-1, 1] {
            let x = cx + side * gap;
            c.set(x, ey, YELLOW);
            c.set(x + side, ey, if look.glow { LIGHTRED } else { BLACK });
        }
        Anchors { eyes: vec![(cx - gap, ey), (cx + gap, ey)], brow: look.sapient.then_some(ey - 1), mouth: Some((cx, cy + 3, 1)), crown: (cx, cy - ry - 2), lid: f, whites: false }
    }

    /// A scaled head: a lizard's, a snake's, a frog's, a fish's, a dragon's.
    fn scaled(c: &mut Canvas, look: &Look, kind: Scales) -> Anchors {
        let s = look.skin;
        let (dark, belly) = (shade(s), lighter(s));
        let (cx, cy, rx, ry) = match kind {
            Scales::Frog => (CX, 9, 6, 4),
            Scales::Fish => (CX, 8, 5, 5),
            Scales::Snake => (CX, 7, 3, 5),
            _ => (CX, 7, 4, 5),
        };
        c.pen = Part::Body;
        if kind == Scales::Fish {
            for side in [-1, 1] {
                for i in 0..3 {
                    let x = cx + side * (rx + i);
                    c.rect(x, cy - 1 + i, x + 1, cy + 3 - i, dark);
                }
            }
            peak(c, cx, cy - ry - 3, cy - ry + 1, 2, dark);
        } else if look.sapient {
            shoulders(c, look, look.clothes);
            c.pen = Part::Body;
            c.rect(cx - 2, cy + ry - 1, cx + 3, 15, s);
        } else if look.animal {
            back(c, s, cy + ry - 2);
        }
        if kind == Scales::Dragon {
            c.pen = Part::Detail;
            for side in [-1, 1] {
                for i in 0..4 {
                    c.set(cx + side * (rx - 1 + i / 2), cy - ry + 1 - i, LIGHTGRAY);
                }
            }
        }
        c.pen = Part::Body;
        c.oval(cx, cy, rx, ry, s);
        for y in cy - ry..cy + ry {
            for x in 0..W {
                if c.get(x, y) == s && x % 3 == 0 && y % 3 == (x / 3) % 2 + 1 {
                    c.set(x, y, dark);
                }
            }
        }
        let mouth_y = cy + ry - 2;
        cap(c, (cx, cy, rx - 1, ry), mouth_y, cy + ry, belly);
        c.pen = Part::Mouth;
        c.rect(cx - rx + 2, mouth_y, cx + rx - 1, mouth_y + 1, shade(dark));
        c.pen = Part::Detail;
        both(c, cx - 1, mouth_y - 2, BLACK);
        if kind == Scales::Fish {
            c.pen = Part::Mouth;
            c.set(cx, mouth_y + 1, shade(dark));
        }
        if kind == Scales::Snake {
            c.pen = Part::Detail;
            c.set(cx, mouth_y + 1, LIGHTRED);
            both(c, cx - 1, mouth_y + 2, LIGHTRED);
        }
        if kind == Scales::Dragon || look.fangs {
            c.pen = Part::Detail;
            for x in (cx - rx + 2..cx + rx - 1).step_by(2) {
                c.set(x, mouth_y + 1, WHITE);
            }
        }
        let (ey, gap) = match kind {
            Scales::Frog => (cy - ry + 1, 4),
            _ => (cy - 2, 2),
        };
        for side in [-1, 1] {
            let x = cx + side * gap;
            if kind == Scales::Frog {
                c.pen = Part::Body;
                c.oval(x, ey, 1, 1, s);
            }
            c.pen = Part::Eye;
            c.set(x - side, ey, look.eyes);
            c.set(x, ey, BLACK);
            if look.glow {
                c.light(x - side, ey, look.eyes);
            }
        }
        Anchors { eyes: vec![(cx - gap, ey), (cx + gap, ey)], brow: None, mouth: Some((cx, mouth_y, (rx - 3).max(1))), crown: (cx, cy - ry), lid: s, whites: false }
    }

    /// An insect's, a spider's or a worm's face.
    fn bug(c: &mut Canvas, look: &Look, kind: Bug) -> Anchors {
        let s = look.skin;
        let (cx, cy, rx, ry) = (CX, 8, 5, 5);
        c.pen = Part::Body;
        c.oval(cx, 16, 7, 3, s);
        c.rect(1, 15, W - 1, H, s);
        if kind == Bug::Insect {
            c.pen = Part::Detail;
            for side in [-1, 1] {
                line(c, (cx + side, cy - ry + 1), (cx + side * 4, 0), shade(s));
            }
            c.pen = Part::Body;
        }
        c.oval(cx, cy, rx, ry, s);
        c.set(cx - 2, cy - ry + 1, lighter(s));
        let eyes = match kind {
            Bug::Insect => {
                for side in [-1, 1] {
                    c.pen = Part::Eye;
                    c.oval(cx + side * 3, cy - 1, 1, 1, look.eyes);
                    c.set(cx + side * 3, cy - 2, lighter(look.eyes));
                    c.pen = Part::Mouth;
                    line(c, (cx + side * 2, cy + ry - 1), (cx + side, cy + ry + 1), BROWN);
                }
                vec![(cx - 3, cy - 1), (cx + 3, cy - 1)]
            }
            Bug::Spider => {
                for side in [-1, 1] {
                    c.pen = Part::Eye;
                    c.light(cx + side, cy - 2, look.eyes);
                    c.light(cx + side, cy - 1, look.eyes);
                    for (dx, dy) in [(3, -3), (3, -1)] {
                        c.light(cx + side * dx, cy + dy, look.eyes);
                    }
                    c.pen = Part::Mouth;
                    c.rect(cx + side * 2, cy + ry - 1, cx + side * 2 + 1, cy + ry + 1, BLACK);
                }
                vec![(cx - 1, cy - 2), (cx + 1, cy - 2)]
            }
            Bug::Worm => {
                c.pen = Part::Mouth;
                c.oval(cx, cy + 1, 3, 3, BLACK);
                c.pen = Part::Detail;
                for (dx, dy) in [(0, -2), (2, 0), (0, 2), (-2, 0)] {
                    c.set(cx + dx, cy + 1 + dy, WHITE);
                }
                Vec::new()
            }
        };
        let mouth = if kind == Bug::Worm { (cx, cy + 1, 2) } else { (cx, cy + ry, 1) };
        Anchors { eyes, brow: None, mouth: Some(mouth), crown: (cx, cy - ry), lid: s, whites: false }
    }

    /// A skull: the undead with no flesh to speak of.
    fn skull(c: &mut Canvas, look: &Look, race: &str) -> Anchors {
        let bone = look.skin;
        let mummy = key(race).contains("mummy");
        let (cx, cy, rx, ry) = (CX, 6, 5, 5);
        shoulders(c, look, if key(race).contains("lich") { MAGENTA } else { DARKGRAY });
        c.pen = Part::Body;
        for y in cy + ry..15 {
            c.set(cx, y, LIGHTGRAY);
            if y % 2 == 0 {
                c.rect(cx - 1, y, cx + 2, y + 1, LIGHTGRAY);
            }
        }
        c.oval(cx, cy, rx, ry, bone);
        c.rect(cx - 3, cy + 2, cx + 4, cy + ry + 1, bone);
        if mummy {
            for y in (cy - ry + 1..cy + ry).step_by(3) {
                for x in 0..W {
                    if c.get(x, y) == bone {
                        c.set(x, y, LIGHTGRAY);
                    }
                }
            }
        }
        c.pen = Part::Eye;
        for side in [-1, 1] {
            c.oval(cx + side * 2, cy - 1, 1, 1, BLACK);
            c.light(cx + side * 2, cy - 1, look.eyes);
        }
        c.pen = Part::Detail;
        c.set(cx, cy + 1, BLACK);
        c.pen = Part::Mouth;
        c.rect(cx - 2, cy + 3, cx + 3, cy + 4, BLACK);
        if !mummy {
            c.rect(cx - 2, cy + 5, cx + 3, cy + 6, shade(bone));
        }
        if let Some(item) = look.worn.head.as_deref() {
            hat(c, look, item, (cx, cy, rx, ry), cy - 3);
        }
        if let Some(item) = look.worn.neck.as_deref() {
            neck(c, item, cy + ry + 2);
        }
        Anchors { eyes: vec![(cx - 2, cy - 1), (cx + 2, cy - 1)], brow: None, mouth: Some((cx, cy + 3, 2)), crown: (cx, cy - ry), lid: BLACK, whites: false }
    }

    /// A being of stuff: fire, water, air, earth, a golem, a slime, a plant.
    fn stuff_of(c: &mut Canvas, look: &Look, stuff: Stuff, rng: &mut Rng) -> Anchors {
        let s = look.skin;
        let (cx, cy, rx, ry) = (CX, 7, 5, 6);
        c.pen = Part::Body;
        match stuff {
            Stuff::Golem => {
                c.rect(0, 14, W, H, s);
                c.rect(cx - 1, 12, cx + 2, 14, shade(s));
                c.rect(cx - 4, 1, cx + 5, 13, s);
                c.rect(cx - 4, cy - 2, cx + 5, cy - 1, shade(s));
                for _ in 0..6 {
                    let (x, y) = (rng.between(cx - 3, cx + 4), rng.between(2, 12));
                    if c.get(x, y) == s {
                        c.set(x, y, shade(s));
                    }
                }
                c.pen = Part::Eye;
                for side in [-1, 1] {
                    c.light(cx + side * 2, cy, look.eyes);
                }
                c.pen = Part::Mouth;
                c.rect(cx - 2, cy + 3, cx + 3, cy + 4, BLACK);
                Anchors { eyes: vec![(cx - 2, cy), (cx + 2, cy)], brow: None, mouth: Some((cx, cy + 3, 2)), crown: (cx, 1), lid: s, whites: false }
            }
            Stuff::Slime => {
                c.oval(cx, 11, 7, 7, s);
                c.rect(0, 14, W, H, s);
                c.set(cx - 3, 7, lighter(s));
                c.set(cx - 3, 8, lighter(s));
                c.pen = Part::Eye;
                for side in [-1, 1] {
                    c.oval(cx + side * 2, 10, 1, 1, WHITE);
                    c.set(cx + side * 2, 10, BLACK);
                }
                Anchors { eyes: vec![(cx - 2, 10), (cx + 2, 10)], brow: None, mouth: Some((cx, 13, 1)), crown: (cx, 4), lid: s, whites: false }
            }
            Stuff::Plant => {
                c.pen = Part::Detail;
                for _ in 0..40 {
                    let (x, y) = (rng.between(0, W - 1), rng.between(0, 11));
                    if within(x, y, (cx, 5, rx + 3, ry - 1)) {
                        c.set(x, y, if rng.chance(50) { GREEN } else { LIGHTGREEN });
                    }
                }
                c.pen = Part::Body;
                c.rect(cx - 3, 4, cx + 4, H, BROWN);
                for y in (4..H).step_by(2) {
                    c.set(cx - 2 + (y % 5), y, shade(BROWN));
                }
                c.pen = Part::Eye;
                for side in [-1, 1] {
                    c.light(cx + side * 2, 7, YELLOW);
                }
                c.pen = Part::Mouth;
                c.rect(cx, 10, cx + 1, 12, BLACK);
                Anchors { eyes: vec![(cx - 2, 7), (cx + 2, 7)], brow: None, mouth: Some((cx, 10, 1)), crown: (cx, 0), lid: BROWN, whites: false }
            }
            _ => {
                let (a, b) = match stuff {
                    Stuff::Fire => (RED, YELLOW),
                    Stuff::Water => (BLUE, LIGHTCYAN),
                    Stuff::Air => (LIGHTGRAY, WHITE),
                    _ => (BROWN, DARKGRAY),
                };
                c.oval(cx, 16, 7, 3, a);
                c.rect(0, 15, W, H, a);
                c.oval(cx, cy, rx, ry, a);
                if stuff == Stuff::Fire {
                    for x in [cx - 3, cx, cx + 3] {
                        peak(c, x, cy - ry - rng.between(1, 3), cy - ry + 1, 1, a);
                    }
                }
                for _ in 0..30 {
                    let (x, y) = (rng.between(0, W - 1), rng.between(0, H - 1));
                    if c.get(x, y) == a {
                        c.light(x, y, b);
                    }
                }
                c.pen = Part::Eye;
                for side in [-1, 1] {
                    c.light(cx + side * 2, cy - 1, WHITE);
                    c.light(cx + side * 3, cy - 1, WHITE);
                }
                c.pen = Part::Mouth;
                c.rect(cx - 1, cy + 3, cx + 2, cy + 4, shade(a));
                Anchors { eyes: vec![(cx - 2, cy - 1), (cx + 2, cy - 1)], brow: None, mouth: Some((cx, cy + 3, 1)), crown: (cx, cy - ry), lid: a, whites: false }
            }
        }
    }
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

    /// Every pixel's color index, top and bottom halves.
    fn indexes(art: &Art) -> Vec<u8> {
        colors(art)
    }

    const MOODS: [Mood; 6] = [Mood::Calm, Mood::Angry, Mood::Pain, Mood::Afraid, Mood::Asleep, Mood::Dead];
    const EFFECTS: [&str; 15] = ["Sanctuary", "Poison", "Cold", "Burning", "Freeze", "Blind", "Invisibility", "Faerie Fire", "Web", "Stoneskin", "Barkskin", "Sleep", "Fear", "Berzerk", "Light"];

    #[test]
    fn every_shape_mood_hurt_and_effect_paints_in_the_16_colors_and_the_fonts_characters() {
        let font = crate::ansi_art::CP437;
        for (race, category) in [("Human", "Humanoid"), ("Wolf", "Canine"), ("Gnoll", "Canine"), ("Hawk", "Avian"), ("Snake", "Ophidian"), ("Fish", "Fish"), ("Spider", "Arachnid"), ("Worm", "Worm"), ("Skeleton", "Undead"), ("Ghost", "Undead"), ("Fire Elemental", "Elemental"), ("Stone Golem", "Stone Golem"), ("Slime", "Slime"), ("Treant", "Vegetation")] {
            let mut who = sitter(&format!("a {race}"), race, category, "");
            for terrain in [None, Some("woods"), Some("stone")] {
                who.terrain = terrain.map(str::to_string);
                let moments = MOODS.iter().map(|&mood| Moment { mood, ..Moment::default() }).chain((0..=5).map(|hurt| Moment { hurt, ..Moment::default() })).chain(EFFECTS.iter().map(|e| Moment { effects: vec![e.to_string()], hurt: 3, ..Moment::default() }));
                for m in moments {
                    let art = face_in(&who, &m);
                    assert_eq!((art.columns, art.rows, art.lines.len()), (SIZE.0, SIZE.1, SIZE.1), "{race} {m:?}");
                    assert!(indexes(&art).iter().all(|&c| c < 16), "{race} {m:?}: a color past the 16");
                    assert!(art.lines.iter().flatten().all(|s| s.text.chars().all(|c| font.contains(c))), "{race} {m:?}");
                }
            }
        }
    }

    #[test]
    fn each_mood_each_hurt_and_each_effect_changes_the_face() {
        for (race, category) in [("Human", "Humanoid"), ("Wolf", "Canine"), ("Skeleton", "Undead"), ("Slime", "Slime"), ("Ghost", "Undead")] {
            let who = sitter(&format!("a {race}"), race, category, "");
            let calm = face(&who);
            for mood in &MOODS[1..] {
                assert_ne!(face_in(&who, &Moment { mood: *mood, ..Moment::default() }), calm, "{race} {mood:?}");
            }
            let mut before = calm.clone();
            for hurt in 1..=5 {
                let now = face_in(&who, &Moment { hurt, ..Moment::default() });
                assert_ne!(now, before, "{race} hurt {hurt}");
                before = now;
            }
        }
        let who = sitter("a guard", "Human", "Humanoid", "");
        for e in EFFECTS {
            assert_ne!(face_in(&who, &Moment { effects: vec![e.into()], ..Moment::default() }), face(&who), "{e}");
        }
        // An effect no look is known for changes nothing.
        assert_eq!(face_in(&who, &Moment { effects: vec!["Detect Magic".into()], ..Moment::default() }), face(&who));
    }

    /// A small face (`Size::Small`) on a black ground, as the strip's picture lays it.
    fn small_face(who: &Sitter, m: &Moment) -> Art {
        let mut c = Canvas::new(SMALL.0, SMALL.1);
        c.stamp(&standing(who, m, Size::Small), 0, CLEAR);
        c.art()
    }

    #[test]
    fn small_faces_every_shape_mood_hurt_and_effect_at_their_own_size() {
        let font = crate::ansi_art::CP437;
        for (race, category) in [("Human", "Humanoid"), ("Elf", "Elf"), ("Dwarf", "Dwarf"), ("Wolf", "Canine"), ("Cat", "Feline"), ("Bear", "Ursine"), ("Rabbit", "Rodent"), ("Cow", "Bovine"), ("Goat", "Goat"), ("Gnoll", "Canine"), ("Hawk", "Avian"), ("Lizard Man", "Reptile"), ("Snake", "Ophidian"), ("Frog", "Amphibian"), ("Fish", "Fish"), ("Dragon", "Dragon"), ("Ant", "Insect"), ("Spider", "Arachnid"), ("Worm", "Worm"), ("Skeleton", "Undead"), ("Mummy", "Undead"), ("Ghost", "Undead"), ("Fire Elemental", "Elemental"), ("Water Elemental", "Elemental"), ("Stone Golem", "Stone Golem"), ("Slime", "Slime"), ("Treant", "Vegetation")] {
            let who = sitter(&format!("a {race}"), race, category, "");
            let calm = small_face(&who, &Moment::default());
            assert_eq!((calm.columns, calm.rows, calm.lines.len()), (SMALL.0, SMALL.1, SMALL.1), "{race}");
            let moments = MOODS.iter().map(|&mood| Moment { mood, ..Moment::default() }).chain((0..=5).map(|hurt| Moment { hurt, ..Moment::default() })).chain(EFFECTS.iter().map(|e| Moment { effects: vec![e.to_string()], hurt: 3, ..Moment::default() }));
            for m in moments {
                let art = small_face(&who, &m);
                assert_eq!((art.columns, art.rows), (SMALL.0, SMALL.1), "{race} {m:?}");
                assert!(indexes(&art).iter().all(|&c| c < 16), "{race} {m:?}: a color past the 16");
                assert!(art.lines.iter().flatten().all(|s| s.text.chars().all(|c| font.contains(c))), "{race} {m:?}");
            }
        }
    }

    #[test]
    fn a_small_face_shows_each_mood_hurt_and_effect() {
        for (race, category) in [("Human", "Humanoid"), ("Wolf", "Canine"), ("Gnoll", "Canine"), ("Hawk", "Avian"), ("Snake", "Ophidian"), ("Spider", "Arachnid"), ("Skeleton", "Undead"), ("Slime", "Slime"), ("Ghost", "Undead"), ("Stone Golem", "Stone Golem")] {
            let who = sitter(&format!("a {race}"), race, category, "");
            let calm = small_face(&who, &Moment::default());
            for mood in &MOODS[1..] {
                assert_ne!(small_face(&who, &Moment { mood: *mood, ..Moment::default() }), calm, "{race} {mood:?}");
            }
            let mut before = calm.clone();
            for hurt in 1..=5 {
                let now = small_face(&who, &Moment { hurt, ..Moment::default() });
                assert_ne!(now, before, "{race} hurt {hurt}");
                before = now;
            }
        }
        let who = sitter("a guard", "Human", "Humanoid", "");
        for e in EFFECTS {
            assert_ne!(small_face(&who, &Moment { effects: vec![e.into()], ..Moment::default() }), small_face(&who, &Moment::default()), "{e}");
        }
        // Drawn small, not the big one cut down; and what's worn shows here too.
        let mut big = Canvas::new(SMALL.0, SMALL.1);
        big.stamp(&standing(&who, &Moment::default(), Size::Full), 0, CLEAR);
        assert_ne!(big.art(), small_face(&who, &Moment::default()));
        let helmeted = Sitter { wears: vec![("worn on head".into(), "a steel helmet".into())], ..who.clone() };
        assert_ne!(small_face(&helmeted, &Moment::default()), small_face(&who, &Moment::default()));
    }

    #[test]
    fn wounds_stay_put_and_only_add_up() {
        let who = sitter("a guard", "Human", "Humanoid", "");
        let look = read(&who);
        let wounded = |step: u8| {
            let mut c = figure::clear(SIZE.0, SIZE.1);
            paint_person(&mut c, &look);
            hurt(&mut c, &look, &who.name, step, (0, 0));
            (0..c.h).flat_map(|y| (0..c.w).map(move |x| (x, y))).filter(|&(x, y)| c.part(x, y) == Part::Detail).map(|(x, y)| (x, y, c.get(x, y))).collect::<Vec<_>>()
        };
        let (two, three) = (wounded(2), wounded(3));
        assert!(!two.is_empty());
        assert!(two.iter().all(|w| three.contains(w)), "step 3 kept step 2's wounds");
        assert!(three.len() > two.len());
    }

    #[test]
    fn people_and_thinking_beasts_and_animals() {
        assert_eq!(shape("Porcine", "Orc"), Shape::Person);
        assert_eq!(shape("Equine", "Centaur"), Shape::Person);
        let gnoll = read(&sitter("a gnoll", "Gnoll", "Canine", ""));
        assert!(gnoll.sapient && !gnoll.animal);
        let wolf = read(&sitter("a wolf", "Wolf", "Canine", ""));
        assert!(!wolf.sapient && wolf.animal);
        // Two signs of a mind make a beast think.
        let mut odd = sitter("a strange hound", "Dog", "Canine", "It has 2 leg(s), 2 arm(s).");
        odd.skilled = true;
        assert!(read(&odd).sapient);
        // An animal in a city is painted in the wild; a person stays.
        let mut cat = sitter("a cat", "Cat", "Feline", "");
        cat.terrain = Some("city".into());
        let mut guard = sitter("a guard", "Human", "Humanoid", "");
        guard.terrain = Some("city".into());
        assert_ne!(face(&cat), face(&Sitter { terrain: None, ..cat.clone() }));
        assert_ne!(face(&guard), face(&Sitter { terrain: None, ..guard.clone() }));
    }

    #[test]
    fn an_effects_look_by_its_name_or_how_affects_shows_it() {
        assert_eq!(effect("Poison"), Some(Effect::Poisoned));
        assert_eq!(effect("(Sleep spell)"), Some(Effect::Asleep));
        assert_eq!(effect("Sanctuary"), Some(Effect::Halo));
        assert_eq!(effect("Detect Magic"), None);
        // Sleep closes the eyes as the mood would.
        let who = sitter("a guard", "Human", "Humanoid", "");
        assert_eq!(face_in(&who, &Moment { effects: vec!["Sleep".into()], ..Moment::default() }), face_in(&who, &Moment { mood: Mood::Asleep, ..Moment::default() }));
    }

    #[test]
    fn a_moment_comes_from_the_frontends_json() {
        let m: Moment = serde_json::from_str(r#"{"mood":"pain","hurt":4,"effects":["Poison"]}"#).unwrap();
        assert_eq!((m.mood, m.hurt, m.effects.len()), (Mood::Pain, 4, 1));
        let calm: Moment = serde_json::from_str("{}").unwrap();
        assert_eq!(calm.mood, Mood::Calm);
    }
}

#[cfg(test)]
pub(crate) mod look {
    use super::*;
    use crate::ansi::Color;

    const VGA: [[u8; 3]; 16] = [[0, 0, 0], [170, 0, 0], [0, 170, 0], [170, 85, 0], [0, 0, 170], [170, 0, 170], [0, 170, 170], [170, 170, 170], [85, 85, 85], [255, 85, 85], [85, 255, 85], [255, 255, 85], [85, 85, 255], [255, 85, 255], [85, 255, 255], [255, 255, 255]];

    /// Writes `arts` as a PPM, `per` to a row, each with a pixel between.
    pub(crate) fn sheet(arts: &[Art], per: usize, out: &str) {
        let (cw, ch) = (arts[0].columns + 1, arts[0].rows * 2 + 1);
        let rows = arts.len().div_ceil(per);
        let (w, h) = (per * cw, rows * ch);
        let mut px = vec![40u8; w * h * 3];
        for (i, art) in arts.iter().enumerate() {
            let (ox, oy) = ((i % per) * cw, (i / per) * ch);
            for (row, line) in art.lines.iter().enumerate() {
                let mut x = 0;
                for span in line {
                    let idx = |c: Option<Color>| match c {
                        Some(Color::Index { index }) => index as usize,
                        _ => 0,
                    };
                    let (fg, bg) = (idx(span.style.fg), idx(span.style.bg));
                    for ch in span.text.chars() {
                        // A character other than the half block shows as its color, solid.
                        let (top, bottom) = match ch {
                            '▀' => (fg, bg),
                            ' ' => (bg, bg),
                            _ => (fg, bg),
                        };
                        for (dy, c) in [(0, top), (1, bottom)] {
                            let at = ((oy + row * 2 + dy) * w + ox + x) * 3;
                            px[at..at + 3].copy_from_slice(&VGA[c]);
                        }
                        x += 1;
                    }
                }
            }
        }
        let mut file = format!("P6 {w} {h} 255\n").into_bytes();
        file.extend(px);
        std::fs::write(out, file).unwrap();
    }

    /// Writes the faces' contact sheet as a PPM to `OUT` to look at while
    /// tuning: each creature in a row, calm in its place, then each mood,
    /// each step of hurt, and the effects.
    /// `OUT=/tmp/f.ppm cargo test face::look -- --ignored`.
    #[test]
    #[ignore]
    fn show() {
        let s = |name: &str, race: &str, category: &str, description: &str, sex: &str, wears: &[(&str, &str)], terrain: &str| Sitter {
            name: name.into(),
            race: Some(race.into()),
            category: Some(category.into()),
            description: Some(description.into()),
            sex: Some(sex.into()),
            wears: wears.iter().map(|(p, i)| (p.to_string(), i.to_string())).collect(),
            terrain: (!terrain.is_empty()).then(|| terrain.to_string()),
            ..Sitter::default()
        };
        let sitters = [
            s("a city guard", "Human", "Humanoid", "A stern guard.", "male", &[("worn on head", "a steel helmet"), ("worn on torso", "a chainmail shirt")], "city"),
            s("a barmaid", "Human", "Humanoid", "She has long red hair and green eyes, and freckles.", "female", &[("worn on neck", "a silver necklace")], "wooden"),
            s("an old wizard", "Human", "Humanoid", "A bearded old man.", "male", &[("worn on head", "a pointed blue hat")], "stone"),
            s("a dwarf", "Dwarf", "Dwarf", "", "male", &[], "cave"),
            s("an elf", "Elf", "Elf", "", "female", &[("worn on head", "a gold circlet")], "woods"),
            s("an orc", "Orc", "Porcine", "A scarred brute.", "male", &[], "hills"),
            s("a gnoll", "Gnoll", "Canine", "", "male", &[("worn on torso", "a leather jerkin")], "plains"),
            s("a black wolf", "Wolf", "Canine", "", "male", &[], "city"),
            s("a cat", "Cat", "Feline", "", "female", &[], "woods"),
            s("a bear", "Bear", "Ursine", "", "male", &[], "mountains"),
            s("a hawk", "Hawk", "Avian", "", "male", &[], "plains"),
            s("a lizard man", "Lizard Man", "Reptile", "", "male", &[], "swamp"),
            s("a snake", "Snake", "Ophidian", "", "male", &[], "desert"),
            s("a fish", "Fish", "Fish", "", "male", &[], ""),
            s("a spider", "Spider", "Arachnid", "", "male", &[], "cave"),
            s("a skeleton", "Skeleton", "Undead", "", "male", &[], "stone"),
            s("a ghost", "Ghost", "Undead", "A pale ghost.", "female", &[], "stone"),
            s("a fire elemental", "Fire Elemental", "Elemental", "", "male", &[], "rocky"),
            s("a stone golem", "Stone Golem", "Stone Golem", "", "male", &[], "cave"),
            s("a green slime", "Slime", "Slime", "", "male", &[], "cave"),
            s("a treant", "Treant", "Vegetation", "", "male", &[], "woods"),
        ];
        let moments: Vec<Moment> = [Mood::Calm, Mood::Angry, Mood::Pain, Mood::Afraid, Mood::Asleep, Mood::Dead]
            .into_iter()
            .map(|mood| Moment { mood, ..Moment::default() })
            .chain((1..=5).map(|hurt| Moment { hurt, ..Moment::default() }))
            .chain(["Poison", "Sanctuary", "Freeze", "Blind", "Faerie Fire", "Web", "Stoneskin", "Burning", "Invisibility", "Light"].iter().map(|e| Moment { effects: vec![e.to_string()], ..Moment::default() }))
            .collect();
        let mut arts = Vec::new();
        for sitter in &sitters {
            arts.push(face(&Sitter { terrain: None, ..sitter.clone() }));
            for m in &moments {
                arts.push(face_in(sitter, m));
            }
        }
        sheet(&arts, moments.len() + 1, &std::env::var("OUT").unwrap_or_else(|_| "faces.ppm".into()));
        // The same, small (the phone's Fight strip), beside it.
        let mut small = Vec::new();
        for sitter in &sitters {
            let mut c = Canvas::new(SMALL.0, SMALL.1);
            c.gradient(0, c.h, &[BLACK, DARKGRAY]);
            c.stamp(&standing(sitter, &Moment::default(), Size::Small), 0, CLEAR);
            small.push(c.art());
            for m in &moments {
                let mut c = Canvas::new(SMALL.0, SMALL.1);
                c.gradient(0, c.h, &[BLACK, DARKGRAY]);
                c.stamp(&standing(sitter, m, Size::Small), 0, CLEAR);
                small.push(c.art());
            }
        }
        sheet(&small, moments.len() + 1, &std::env::var("OUT_SMALL").unwrap_or_else(|_| "faces-small.ppm".into()));
    }
}

//! **Item pictures** by Coupler's painter (`painter.rs`): an item in the
//! Bestiary drawn alone, 25 by 24 square pixels in 25 by 12 cells (200
//! by 192 on the screen, the same frame as a creature's face, `face.rs`),
//! in the 16 VGA colors. Pure and unit-tested. All ART is Coupler's painter's first
//! (CLAUDE.md rule 11).
//!
//! **What it reads**: the item's name picks its kind, by the first word
//! of it that names one ("a pair of leather boots": boots; "a bag of
//! gold coins": a bag): a sword, a dagger, an axe, a mace or hammer, a
//! spear, a staff or wand, a bow, arrows, a shield, a helmet, a crown, a
//! hood or hat, body armor or clothes, leggings, boots, gloves, a cloak,
//! a belt, a ring, an amulet, a bracelet, a potion, a scroll, a book, a
//! bag, a chest, a barrel, a cup or goblet, food, coins, a gem, a key, a
//! torch, a lantern or candle, an instrument, a tool, a rock; anything
//! else is a bundle (a container, a sack). Its colors come from the
//! color words in its name, then what EXAMINE says it's made of ("It is
//! mostly made of oak"), then its kind's own. The same item always gets
//! the same picture.
//!
//! **What's on it** (what the game writes after its name, the Bestiary's
//! `auras`): glowing (a halo, in its color for glowing red, blue, purple
//! or white), a magical aura (sparks), a metallic or mithril aura
//! (glints), invisible (half of it seen through), hidden (in shadow),
//! floating or flying (lifted, its shadow below), bound (a rope).
//!
//! **Its slot**, in words on the bottom row, centered: where it was seen
//! worn ("Worn on head", "Wielded"), else where its kind goes, else
//! "Carried".
//!
//! **In layers** (`figure.rs`), as the faces are:
//!
//! - **Its shape from numbers**: a sword's length, breadth, curve and
//!   guard, a dagger's, an axe's head, a helmet's dome and visor, a
//!   shield's outline and device, a potion's bottle; set by the words of
//!   its name ("scimitar": curved; "rapier": thin, a basket guard;
//!   "claymore": long and broad), the rest by its name's seed.
//! - **Its material as a texture**, not only a color: what EXAMINE says
//!   it's made of, else a material in its name, else its kind's own, by
//!   class (`materials.tsv`, from CoffeeMUD's `RawMaterial` by
//!   `scripts/painter-tables.py`): cloth woven, leather stitched, wood
//!   grained, metal with a band of light, mithril and gems sparkling,
//!   glass the backdrop through it, stone speckled. A handle keeps its
//!   wood and a blade its metal.
//! - **Its wear**, from the words of its name and description: worn,
//!   then damaged (dents in metal, a crack in wood or stone, a frayed
//!   edge of cloth), battered (notches out of its edge, holes), broken
//!   (split in two); rust on iron; crude (a rough outline) or fine (gilt
//!   trim and a gem). **To confirm**: what LOOK or EXAMINE says of an
//!   item's condition, which would say it outright.
//! - **The place behind**: where it's been found the most (its area's
//!   commonest terrain on the map), a shade darker; else a backdrop of
//!   one color over a table. Its shadow under it.

use crate::ansi_art::Art;
use crate::bestiary::ItemEntry;
use crate::face::{both, cap, color_word, key, lighter, line, peak, shade, within, words, CX, H, SIZE, W};
use crate::figure::{self, CLEAR};
use crate::paint::{seed, Colors as GameColors};
use crate::painter::*;

/// What the painter needs of an item.
#[derive(Debug, Clone, Default)]
pub struct Thing {
    pub name: String,
    pub material: Option<String>,
    pub description: Option<String>,
    pub container: bool,
    /// What's been seen after its name: "glowing", "magical aura".
    pub auras: Vec<String>,
    /// Where it was seen worn, the most often: "worn on head", "wielded".
    pub worn: Option<String>,
    /// The terrain where it's been found the most, from the map.
    pub terrain: Option<String>,
}

impl From<&ItemEntry> for Thing {
    fn from(i: &ItemEntry) -> Thing {
        let worn = i.worn_by.iter().filter_map(|c| c.what.as_ref().map(|w| (w, c.count))).max_by_key(|(_, n)| *n).map(|(w, _)| w.clone());
        Thing { name: i.name.clone(), material: i.material.clone(), description: i.description.clone(), container: i.container, auras: i.auras.clone(), worn, terrain: None }
    }
}

/// Where a kind goes on the body, in CoffeeMUD's words.
fn slot_of(kind: Kind) -> &'static str {
    match kind {
        Kind::Sword | Kind::Dagger | Kind::Axe | Kind::Mace | Kind::Hammer | Kind::Spear | Kind::Staff | Kind::Bow | Kind::Pick | Kind::Shovel => "Wielded",
        Kind::Wand | Kind::Shield | Kind::Torch | Kind::Lantern | Kind::Candle | Kind::Lute | Kind::Drum | Kind::Horn | Kind::Flute => "Held",
        Kind::Helmet | Kind::Crown | Kind::Hood | Kind::Hat => "Worn on head",
        Kind::Armor | Kind::Robe => "Worn on torso",
        Kind::Legs => "Worn on legs",
        Kind::Boots => "Worn on feet",
        Kind::Gloves => "Worn on hands",
        Kind::Cloak => "Worn about body",
        Kind::Belt => "Worn on waist",
        Kind::Ring => "Worn on finger",
        Kind::Amulet => "Worn on neck",
        Kind::Bracelet => "Worn on wrist",
        _ => "Carried",
    }
}

/// The slot's words for the bottom row: where it was seen worn, else its kind's.
fn slot(thing: &Thing, kind: Kind) -> String {
    let said = thing.worn.as_deref().map(str::trim).filter(|w| !w.is_empty());
    let mut text = said.map_or_else(|| slot_of(kind).to_string(), |w| {
        let mut cs = w.chars();
        cs.next().map(|c| c.to_uppercase().chain(cs).collect()).unwrap_or_default()
    });
    text.truncate(SIZE.0);
    text
}

/// What an item is, by the word in its name.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Sword,
    Dagger,
    Axe,
    Mace,
    Hammer,
    Spear,
    Staff,
    Wand,
    Bow,
    Arrow,
    Shield,
    Helmet,
    Crown,
    Hood,
    Hat,
    Armor,
    Robe,
    Legs,
    Boots,
    Gloves,
    Cloak,
    Belt,
    Ring,
    Amulet,
    Bracelet,
    Potion,
    Scroll,
    Book,
    Bag,
    Chest,
    Barrel,
    Cup,
    Goblet,
    Bread,
    Fruit,
    Meat,
    Cheese,
    Fish,
    Coins,
    Gem,
    Key,
    Torch,
    Lantern,
    Candle,
    Lute,
    Drum,
    Horn,
    Flute,
    Pick,
    Shovel,
    Rock,
    Bundle,
}

/// The words that name each kind.
const KINDS: &[(Kind, &[&str])] = &[
    (Kind::Sword, &["sword", "longsword", "shortsword", "broadsword", "greatsword", "blade", "scimitar", "sabre", "saber", "rapier", "katana", "cutlass", "falchion", "claymore", "gladius", "estoc"]),
    (Kind::Dagger, &["dagger", "knife", "dirk", "stiletto", "kris", "kukri", "shiv", "sickle"]),
    (Kind::Axe, &["axe", "battleaxe", "handaxe", "hatchet", "cleaver", "tomahawk"]),
    (Kind::Mace, &["mace", "club", "morningstar", "flail", "cudgel", "baton", "truncheon", "sap"]),
    (Kind::Hammer, &["hammer", "warhammer", "maul", "mallet"]),
    (Kind::Spear, &["spear", "pike", "halberd", "lance", "trident", "javelin", "glaive", "polearm", "harpoon", "scythe"]),
    (Kind::Staff, &["staff", "quarterstaff", "stave", "cane", "crook", "walkingstick"]),
    (Kind::Wand, &["wand", "rod", "scepter", "sceptre"]),
    (Kind::Bow, &["bow", "longbow", "shortbow", "crossbow", "sling"]),
    (Kind::Arrow, &["arrow", "arrows", "bolt", "bolts", "quiver", "dart", "darts"]),
    (Kind::Shield, &["shield", "buckler", "targe", "aegis"]),
    (Kind::Helmet, &["helmet", "helm", "coif", "casque", "basinet", "bascinet", "sallet", "barbute"]),
    (Kind::Crown, &["crown", "tiara", "circlet", "diadem", "coronet"]),
    (Kind::Hood, &["hood", "cowl", "mask", "veil"]),
    (Kind::Hat, &["hat", "cap", "turban", "beret", "fedora", "bonnet", "headband", "bandana"]),
    (Kind::Armor, &["armor", "armour", "breastplate", "cuirass", "hauberk", "chainmail", "platemail", "mail", "jerkin", "shirt", "tunic", "vest", "jacket", "coat", "doublet", "blouse", "brigandine", "corset", "tabard"]),
    (Kind::Robe, &["robe", "robes", "gown", "dress", "habit", "vestments", "frock", "smock", "apron"]),
    (Kind::Legs, &["pants", "leggings", "greaves", "trousers", "breeches", "skirt", "kilt", "shorts", "leggings", "chaps"]),
    (Kind::Boots, &["boots", "boot", "shoes", "shoe", "sandals", "slippers", "moccasins", "sabatons", "clogs"]),
    (Kind::Gloves, &["gloves", "glove", "gauntlets", "gauntlet", "mittens"]),
    (Kind::Cloak, &["cloak", "cape", "mantle", "shawl", "poncho"]),
    (Kind::Belt, &["belt", "girdle", "sash", "baldric"]),
    (Kind::Ring, &["ring", "band", "signet"]),
    (Kind::Amulet, &["amulet", "necklace", "pendant", "medallion", "locket", "torc", "talisman", "charm", "symbol", "chain", "collar"]),
    (Kind::Bracelet, &["bracelet", "bracer", "bracers", "armband", "wristband", "bangle", "anklet", "vambrace", "vambraces"]),
    (Kind::Potion, &["potion", "vial", "flask", "bottle", "elixir", "philter", "philtre", "tonic", "draught", "salve", "phial", "decanter", "jug", "jar"]),
    (Kind::Scroll, &["scroll", "parchment", "map", "letter", "note", "paper", "deed", "contract", "page", "recipe"]),
    (Kind::Book, &["book", "tome", "journal", "spellbook", "manual", "grimoire", "codex", "diary", "ledger", "bible", "prayerbook"]),
    (Kind::Bag, &["bag", "sack", "pouch", "backpack", "pack", "purse", "satchel", "knapsack", "rucksack", "haversack", "wallet"]),
    (Kind::Chest, &["chest", "box", "crate", "coffer", "trunk", "casket", "strongbox", "lockbox", "footlocker", "cabinet"]),
    (Kind::Barrel, &["barrel", "keg", "cask", "bucket", "pail", "tub"]),
    (Kind::Cup, &["cup", "mug", "tankard", "stein", "flagon", "waterskin", "skin", "canteen", "bowl"]),
    (Kind::Goblet, &["goblet", "chalice", "grail"]),
    (Kind::Bread, &["bread", "loaf", "ration", "rations", "biscuit", "cake", "pie", "bun", "roll", "pastry", "muffin", "cookie", "tart", "waybread"]),
    (Kind::Fruit, &["apple", "pear", "peach", "orange", "plum", "fruit", "berry", "berries", "grape", "grapes", "melon", "lemon", "tomato", "turnip", "potato", "onion", "egg", "mushroom", "nut", "nuts"]),
    (Kind::Meat, &["meat", "steak", "drumstick", "ham", "jerky", "sausage", "bacon", "leg", "haunch", "chop", "mutton", "venison", "beef", "pork", "chicken"]),
    (Kind::Cheese, &["cheese", "wedge"]),
    (Kind::Fish, &["fish", "trout", "salmon", "herring", "cod", "eel", "carp"]),
    (Kind::Coins, &["coins", "coin", "money", "gold", "silver", "copper", "platinum", "pennies", "nobles", "crowns"]),
    (Kind::Gem, &["gem", "gems", "jewel", "jewels", "ruby", "diamond", "emerald", "sapphire", "opal", "pearl", "amethyst", "topaz", "garnet", "jade", "onyx", "crystal", "agate", "jasper", "quartz", "moonstone", "beryl"]),
    (Kind::Key, &["key", "keys", "skeleton"]),
    (Kind::Torch, &["torch", "brand"]),
    (Kind::Lantern, &["lantern", "lamp"]),
    (Kind::Candle, &["candle", "candles", "taper"]),
    (Kind::Lute, &["lute", "guitar", "mandolin", "harp", "lyre", "fiddle", "violin", "banjo"]),
    (Kind::Drum, &["drum", "tambourine", "gong"]),
    (Kind::Horn, &["horn", "trumpet", "bugle", "tuba"]),
    (Kind::Flute, &["flute", "pipe", "pipes", "whistle", "recorder", "ocarina", "panpipes"]),
    (Kind::Pick, &["pick", "pickaxe", "mattock", "hoe"]),
    (Kind::Shovel, &["shovel", "spade", "rake"]),
    (Kind::Rock, &["rock", "stone", "ore", "pebble", "boulder", "nugget", "chunk", "lump", "brick", "ingot", "bar"]),
];

/// Words that would name a kind but say what it's made of, when another word names one ("a gold ring", "a silver key").
const MATERIALS_TOO: &[&str] = &["gold", "silver", "copper", "platinum", "crystal", "jade", "stone", "pearl", "onyx", "bone", "skin"];

/// The kind the name gives: its first word that names one, a material
/// word only if nothing else does.
fn kind(thing: &Thing) -> Kind {
    let ws = words(&thing.name);
    let of = |w: &str| KINDS.iter().find(|(_, names)| names.contains(&w)).map(|(k, _)| *k);
    // "a potion of healing", "a bag of coins": the word before "of" names it.
    let first = ws.iter().filter(|w| !MATERIALS_TOO.contains(&w.as_str())).find_map(|w| of(w));
    first.or_else(|| ws.iter().find_map(|w| of(w))).unwrap_or(if thing.container { Kind::Bag } else { Kind::Bundle })
}

/// What it's made of, as a color: "oak", "iron", "wool", "glass".
fn material_color(material: &str) -> Option<u8> {
    // By whole words, from their start: "golden" is gold, "helmet" isn't elm.
    let ws = words(material);
    let is = |any: &[&str]| any.iter().any(|a| ws.iter().any(|w| w.starts_with(a)));
    Some(if is(&["gold", "brass", "electrum"]) {
        YELLOW
    } else if is(&["silver", "platinum", "mithril", "pearl"]) {
        WHITE
    } else if is(&["copper", "bronze"]) {
        BROWN
    } else if is(&["iron", "steel", "tin", "lead", "adamant", "metal", "chrome", "pewter", "nickel"]) {
        LIGHTGRAY
    } else if is(&["glass", "crystal", "diamond", "quartz", "ice"]) {
        LIGHTCYAN
    } else if is(&["bone", "ivory", "paper", "parchment", "chalk", "marble"]) {
        WHITE
    } else if is(&["stone", "granite", "rock", "slate", "obsidian", "flint", "coal"]) {
        DARKGRAY
    } else if is(&["ruby", "garnet"]) {
        LIGHTRED
    } else if is(&["emerald", "jade"]) {
        LIGHTGREEN
    } else if is(&["sapphire"]) {
        LIGHTBLUE
    } else if is(&["amethyst"]) {
        LIGHTMAGENTA
    } else if is(&["wood", "oak", "pine", "ash", "elm", "maple", "yew", "birch", "cedar", "ebony", "teak", "willow", "leather", "hide", "fur", "straw", "wicker", "clay"]) {
        BROWN
    } else {
        return None;
    })
}

/// The item's colors: its main color, an accent (a handle, trim) and a
/// metal (a blade, a buckle, a band).
struct Colors {
    main: u8,
    metal: u8,
    wood: u8,
    /// A color the name or description gives outright ("a red potion").
    said: Option<u8>,
    /// The bright color of what's said: a potion's liquid, a gem.
    bright: Option<u8>,
}

fn colors(thing: &Thing, kind: Kind, rng: &mut Rng) -> Colors {
    let named = words(&thing.name);
    // "Rusty" is its wear (`wear`), not its color: the steel stays steel under the rust.
    let colored = |w: &String| if ["rust", "rusty", "rusted"].contains(&w.as_str()) { None } else { color_word(w) };
    let said_plain = named.iter().find_map(colored).or_else(|| words(thing.description.as_deref().unwrap_or("")).iter().find_map(colored));
    let named_metal = named.iter().find_map(|w| material_color(w));
    let made = thing.material.as_deref().and_then(material_color).or(named_metal);
    let metal = match made {
        Some(c) if [YELLOW, WHITE, BROWN, LIGHTGRAY, LIGHTCYAN].contains(&c) => c,
        _ => LIGHTGRAY,
    };
    let fabric = [BLUE, RED, GREEN, BROWN, MAGENTA, CYAN, DARKGRAY][rng.below(7) as usize];
    let armored = named.iter().any(|w| ["mail", "chainmail", "chain", "ringmail", "scale", "hauberk", "plate", "platemail", "breastplate", "cuirass", "greaves"].contains(&w.as_str()));
    let own = match kind {
        Kind::Armor | Kind::Legs if armored => made.unwrap_or(metal),
        Kind::Sword | Kind::Dagger | Kind::Axe | Kind::Spear | Kind::Helmet | Kind::Shield | Kind::Key | Kind::Hammer | Kind::Pick | Kind::Shovel | Kind::Mace => metal,
        Kind::Crown | Kind::Ring | Kind::Amulet | Kind::Bracelet | Kind::Coins => made.unwrap_or(YELLOW),
        Kind::Staff | Kind::Bow | Kind::Arrow | Kind::Chest | Kind::Barrel | Kind::Torch | Kind::Lute | Kind::Drum | Kind::Flute | Kind::Belt | Kind::Boots | Kind::Gloves | Kind::Bag => made.unwrap_or(BROWN),
        Kind::Wand => made.unwrap_or(BROWN),
        Kind::Potion => LIGHTRED,
        Kind::Scroll => WHITE,
        Kind::Book => fabric,
        Kind::Cup | Kind::Goblet => made.unwrap_or(LIGHTGRAY),
        Kind::Bread => BROWN,
        Kind::Fruit => RED,
        Kind::Meat => LIGHTRED,
        Kind::Cheese => YELLOW,
        Kind::Fish => LIGHTGRAY,
        Kind::Gem => LIGHTRED,
        Kind::Lantern | Kind::Horn => made.unwrap_or(YELLOW),
        Kind::Candle => WHITE,
        Kind::Rock => made.unwrap_or(DARKGRAY),
        Kind::Armor | Kind::Robe | Kind::Legs | Kind::Cloak | Kind::Hood | Kind::Hat | Kind::Bundle => made.unwrap_or(fabric),
    };
    // A gem's or a fruit's name is its color.
    let gem = named.iter().find_map(|w| match w.as_str() {
        "ruby" | "garnet" => Some(LIGHTRED),
        "emerald" | "jade" => Some(LIGHTGREEN),
        "sapphire" => Some(LIGHTBLUE),
        "amethyst" => Some(LIGHTMAGENTA),
        "diamond" | "crystal" | "quartz" | "moonstone" | "pearl" => Some(WHITE),
        "topaz" | "amber" => Some(YELLOW),
        "onyx" | "obsidian" => Some(DARKGRAY),
        "opal" | "beryl" | "turquoise" => Some(LIGHTCYAN),
        "lemon" | "egg" => Some(YELLOW),
        "orange" | "potato" | "nut" | "nuts" | "mushroom" => Some(BROWN),
        "pear" => Some(LIGHTGREEN),
        "plum" | "grape" | "grapes" => Some(MAGENTA),
        "onion" | "turnip" => Some(WHITE),
        _ => None,
    });
    let bright = said_plain.map(|(_, b)| b).or(gem);
    let main = match kind {
        Kind::Gem | Kind::Fruit | Kind::Potion => bright.unwrap_or(own),
        _ => said_plain.map(|(c, _)| c).unwrap_or(own),
    };
    let wood = match thing.material.as_deref().and_then(material_color) {
        Some(BROWN) | None => BROWN,
        Some(c) if [Kind::Staff, Kind::Bow, Kind::Chest, Kind::Barrel, Kind::Torch].contains(&kind) => c,
        Some(_) => BROWN,
    };
    Colors { main, metal, wood, said: said_plain.map(|(c, _)| c), bright }
}

/// A diagonal bar two pixels thick, from the bottom left to the top right.
fn bar(c: &mut Canvas, from: (i32, i32), to: (i32, i32), color: u8) {
    line(c, from, to, color);
    line(c, (from.0 + 1, from.1), (to.0 + 1, to.1), color);
}

/// A ring: the pixels inside the outer ellipse and not the inner.
fn hoop(c: &mut Canvas, outer: (i32, i32, i32, i32), inner: (i32, i32, i32, i32), color: u8) {
    for y in 0..H {
        for x in 0..W {
            if within(x, y, outer) && !within(x, y, inner) {
                c.set(x, y, color);
            }
        }
    }
}

/// A glint of light on something shiny.
fn glint(c: &mut Canvas, x: i32, y: i32) {
    c.light(x, y, WHITE);
}

/// A blade from its hilt to its tip, up to the right: `width` pixels
/// across, its last one the lit edge, narrowing to a point; bowed
/// `bend` pixels at its middle (a sabre's curve).
fn blade(c: &mut Canvas, from: (i32, i32), tip: (i32, i32), bend: i32, width: i32, color: u8, edge: u8) {
    let steps = (tip.0 - from.0).abs().max((tip.1 - from.1).abs()).max(1);
    for i in 0..=steps {
        let t = f64::from(i) / f64::from(steps);
        let bow = (f64::from(bend) * (std::f64::consts::PI * t).sin()).round() as i32;
        let (x, y) = (from.0 + (tip.0 - from.0) * i / steps + bow, from.1 + (tip.1 - from.1) * i / steps + bow);
        let w = if steps - i < 2 { 1 } else { width };
        for j in 0..w {
            c.set(x + j, y, if j == w - 1 && w > 1 { edge } else { color });
        }
    }
    c.set(tip.0 + 1, tip.1 - 1, edge);
}

fn paint(c: &mut Canvas, kind: Kind, k: &Colors, thing: &Thing, rng: &mut Rng) {
    let main = k.main;
    let dark = shade(main);
    let light = lighter(main);
    let ws = words(&thing.name);
    let has = |any: &[&str]| ws.iter().any(|w| any.contains(&w.as_str()));
    match kind {
        Kind::Sword => {
            // The blade up to the top right, a guard across, the grip and
            // pommel: how long, broad and curved, and the guard, from the name.
            let long = has(&["long", "longsword", "claymore", "greatsword", "bastard", "great", "zweihander", "flamberge", "broadsword", "twohanded"]);
            let short = has(&["short", "shortsword", "gladius", "falchion"]);
            let thin = has(&["rapier", "estoc", "foil", "epee", "smallsword"]);
            let broad = has(&["broad", "broadsword", "claymore", "greatsword", "great", "falchion"]);
            let curved = has(&["scimitar", "sabre", "saber", "cutlass", "katana", "tulwar", "shamshir", "falchion", "curved"]);
            let reach = if long { 15 } else if short { 9 } else { 11 + rng.between(0, 2) };
            blade(c, (7, 17), (7 + reach, 17 - reach), if curved { 2 } else { 0 }, if thin { 1 } else if broad { 3 } else { 2 }, main, light);
            let gilt = if k.metal == YELLOW { LIGHTGRAY } else { YELLOW };
            if thin {
                // A basket about the hand.
                hoop(c, (6, 18, 3, 3), (6, 18, 2, 2), gilt);
            } else if has(&["katana"]) {
                c.oval(7, 17, 2, 2, DARKGRAY);
                c.set(7, 17, gilt);
            } else {
                line(c, (3, 15), (9, 21), gilt);
                line(c, (4, 15), (10, 21), BROWN);
                if rng.chance(50) {
                    // Quillons curled toward the blade.
                    c.set(3, 14, gilt);
                    c.set(10, 22, gilt);
                }
            }
            bar(c, (3, 22), (6, 19), k.wood);
            c.light(2, 23, YELLOW);
            if has(&["glowing", "flaming", "magic", "enchanted", "holy", "rune", "runed"]) {
                for i in [6, 9, 12] {
                    let at = i.min(reach - 1);
                    c.light(7 + at, 17 - at, LIGHTCYAN);
                }
            }
        }
        Kind::Dagger => {
            let thin = has(&["stiletto", "dirk"]);
            let curved = has(&["kukri", "sickle", "kris"]);
            let reach = 7 + rng.between(0, 2);
            blade(c, (10, 15), (10 + reach, 15 - reach), if curved { 2 } else { 0 }, if thin { 1 } else { 2 }, main, light);
            line(c, (6, 13), (11, 18), YELLOW);
            bar(c, (6, 19), (9, 16), k.wood);
            c.light(5, 20, YELLOW);
        }
        Kind::Axe => {
            bar(c, (6, 23), (16, 3), k.wood);
            // The head: a broad crescent off the haft near the top, its size
            // and whether it's bearded (hanging lower) from the seed.
            let (rx, ry) = (3 + rng.between(0, 1), 4 + rng.between(0, 2));
            c.oval(19, 7, rx, ry, main);
            if rng.chance(35) {
                c.oval(20, 10, rx - 1, ry, main);
            }
            c.rect(14, 5, 18, 9, dark);
            for y in 7 - ry + 1..7 + ry {
                c.set(19 + rx, y, light);
            }
            bar(c, (12, 11), (16, 3), k.wood);
            if has(&["battleaxe", "great", "double"]) {
                c.oval(9, 7, 3, 4, main);
                bar(c, (12, 11), (16, 3), k.wood);
            }
        }
        Kind::Mace => {
            let club = has(&["club", "cudgel", "baton", "truncheon", "sap"]);
            bar(c, (6, 22), (14, 9), k.wood);
            if club {
                c.oval(16, 7, 4, 5, k.wood);
                c.set(15, 4, lighter(k.wood));
            } else {
                c.oval(16, 7, 4, 4, main);
                for (x, y) in [(16, 1), (22, 7), (16, 13), (10, 7), (20, 3), (12, 3), (20, 11)] {
                    c.set(x, y, light);
                }
                glint(c, 15, 5);
            }
        }
        Kind::Hammer => {
            bar(c, (7, 23), (14, 8), k.wood);
            c.rect(8, 3, 21, 9, main);
            c.rect(8, 3, 21, 4, light);
            c.rect(8, 8, 21, 9, dark);
        }
        Kind::Spear => {
            bar(c, (3, 23), (18, 8), k.wood);
            // The head along the shaft.
            for i in 0..6 {
                let (x, y) = (18 + i / 2 + 1, 7 - i);
                c.rect(x - 1, y, x + 2, y + 1, main);
            }
            c.set(22, 1, light);
            if has(&["halberd", "glaive", "scythe"]) {
                c.oval(20, 10, 3, 3, main);
            }
            if has(&["trident"]) {
                line(c, (16, 6), (16, 1), main);
                line(c, (22, 9), (24, 9), main);
            }
        }
        Kind::Staff => {
            bar(c, (8, 24), (15, 2), k.wood);
            line(c, (8, 24), (15, 2), lighter(k.wood));
            if has(&["magic", "magi", "wizard", "mage", "power", "glowing", "crystal", "fire", "frost", "lightning", "of"]) {
                let orb = k.bright.unwrap_or(LIGHTCYAN);
                c.oval(16, 2, 2, 2, orb);
                c.light(16, 2, WHITE);
                c.light(15, 1, WHITE);
            } else {
                c.oval(15, 2, 2, 2, k.wood);
            }
        }
        Kind::Wand => {
            line(c, (7, 20), (17, 6), k.wood);
            line(c, (8, 20), (18, 6), shade(k.wood));
            let tip = k.bright.unwrap_or(LIGHTCYAN);
            c.light(18, 5, tip);
            c.light(19, 4, WHITE);
            for (x, y) in [(21, 3), (17, 2), (21, 7)] {
                c.light(x, y, tip);
            }
            if has(&["scepter", "sceptre"]) {
                c.oval(18, 5, 2, 2, YELLOW);
                c.light(18, 5, tip);
            }
        }
        Kind::Bow => {
            if has(&["crossbow"]) {
                c.rect(3, 12, 22, 14, k.wood);
                for x in 6..19i32 {
                    let y = 6 + (x - 12).abs() / 2;
                    c.set(x, y, main);
                    c.set(x, 18 - y + 6, main);
                }
                line(c, (6, 9), (18, 9), LIGHTGRAY);
                c.rect(11, 6, 13, 20, k.wood);
            } else if has(&["sling"]) {
                line(c, (4, 4), (12, 18), BROWN);
                line(c, (20, 4), (12, 18), BROWN);
                c.oval(12, 19, 2, 1, BROWN);
                c.set(12, 19, DARKGRAY);
            } else {
                // The bow's arc, its string, an arrow on it.
                for y in 1..24 {
                    let t = (y - 12) as f64 / 11.0;
                    let x = 7 + (9.0 * (1.0 - t * t).sqrt()) as i32;
                    c.set(x, y, main);
                    c.set(x + 1, y, shade(main));
                }
                line(c, (7, 1), (7, 23), LIGHTGRAY);
                line(c, (4, 12), (20, 12), BROWN);
                c.set(21, 12, LIGHTGRAY);
                c.set(20, 11, LIGHTGRAY);
                c.set(20, 13, LIGHTGRAY);
                both(c, 4, 11, WHITE);
                c.set(4, 13, WHITE);
            }
        }
        Kind::Arrow => {
            let quiver = has(&["quiver"]);
            if quiver {
                c.rect(8, 8, 17, 24, BROWN);
                c.rect(8, 8, 17, 9, shade(BROWN));
            }
            for (dx, n) in [(0, 0), (3, 1), (6, 2)] {
                let x = 8 + dx;
                let top = 2 + n;
                line(c, (x, top + 2), (x, if quiver { 10 } else { 22 }), BROWN);
                c.set(x, top + 1, main);
                c.set(x, top, light);
                c.set(x - 1, top + 2, main);
                c.set(x + 1, top + 2, main);
                if !quiver {
                    c.set(x - 1, 21, WHITE);
                    c.set(x + 1, 21, WHITE);
                    c.set(x - 1, 22, WHITE);
                    c.set(x + 1, 22, WHITE);
                }
            }
        }
        Kind::Shield => {
            // Round, a heater, a kite or a tower: by the name, else the seed.
            let outline = if has(&["buckler", "targe", "round"]) {
                0
            } else if has(&["kite"]) {
                2
            } else if has(&["tower", "pavise"]) {
                3
            } else {
                rng.below(3)
            };
            if outline == 0 {
                c.oval(CX, 12, 10, 10, main);
                hoop(c, (CX, 12, 10, 10), (CX, 12, 9, 9), dark);
                c.oval(CX, 12, 2, 2, k.metal);
                glint(c, CX - 1, 11);
            } else {
                match outline {
                    // A kite: rounded on top, long to its point.
                    2 => {
                        cap(c, (CX, 7, 8, 6), 1, 8, main);
                        for y in 8..24 {
                            let half = 8 - (y - 8) * 8 / 16;
                            c.rect(CX - half, y, CX + half + 1, y + 1, main);
                        }
                    }
                    // A tower: tall and straight, its top rounded.
                    3 => {
                        cap(c, (CX, 4, 8, 3), 1, 4, main);
                        c.rect(4, 4, 21, 24, main);
                    }
                    // A heater: square at the top, to a point.
                    _ => {
                        c.rect(4, 2, 21, 12, main);
                        for y in 12..24 {
                            let half = 8 - (y - 12) * 8 / 12;
                            c.rect(CX - half, y, CX + half + 1, y + 1, main);
                        }
                    }
                }
                // Its device in a second color, on the face only.
                let device = if main == YELLOW || main == WHITE { RED } else { YELLOW };
                let mut charge = Canvas::new(SIZE.0, SIZE.1);
                charge.rect(0, 0, W, H, CLEAR);
                match rng.below(5) {
                    0 => {
                        charge.rect(CX - 1, 4, CX + 2, 19, device);
                        charge.rect(6, 9, 19, 11, device);
                    }
                    1 => {
                        for t in 0..2 {
                            line(&mut charge, (4, 17 + t), (CX, 9 + t), device);
                            line(&mut charge, (CX, 9 + t), (21, 17 + t), device);
                        }
                    }
                    2 => {
                        for t in 0..3 {
                            line(&mut charge, (4 + t, 2), (21, 19 - t), device);
                        }
                    }
                    3 => {
                        for y in 0..H {
                            for x in 0..W {
                                if (x < CX) == (y < 11) {
                                    charge.set(x, y, device);
                                }
                            }
                        }
                    }
                    _ => {
                        for t in 0..2 {
                            line(&mut charge, (4 + t, 2), (21, 19 - t), device);
                            line(&mut charge, (21 - t, 2), (4, 19 - t), device);
                        }
                    }
                }
                for y in 0..H {
                    for x in 0..W {
                        if c.get(x, y) == main && charge.get(x, y) != CLEAR {
                            c.set(x, y, device);
                        }
                    }
                }
                // The rim: lit along the top, dark down the left.
                for y in 0..H {
                    for x in 0..W {
                        if c.get(x, y) != CLEAR && (c.get(x, y - 1) == CLEAR || y == 0 || c.get(x - 1, y) == CLEAR) {
                            c.set(x, y, dark);
                        }
                    }
                }
            }
        }
        Kind::Helmet => {
            // A full helm with its face plate, a nasal over an open face, or
            // a cap: by the name, else the seed; the dome's height from it too.
            let style = if has(&["great", "greathelm", "full", "closed", "barbute", "bascinet", "basinet", "visored", "sallet"]) {
                0
            } else if has(&["nasal", "norman", "spangenhelm"]) {
                1
            } else if has(&["cap", "coif", "skullcap", "open"]) {
                2
            } else {
                rng.below(3)
            };
            let top = 2 + rng.between(0, 3);
            cap(c, (CX, 14, 9, 14 - top), top, 15, main);
            c.rect(3, 14, 22, 16, dark);
            match style {
                0 => {
                    c.rect(5, 15, 20, 22, main);
                    c.rect(6, 17, 19, 18, BLACK);
                    c.rect(CX, 15, CX + 1, 22, dark);
                    for x in [8, 10, 15, 17] {
                        c.set(x, 20, BLACK);
                    }
                }
                1 => {
                    c.rect(CX, 14, CX + 1, 21, main);
                    c.rect(5, 15, 8, 20, main);
                    c.rect(17, 15, 20, 20, main);
                }
                _ => {}
            }
            glint(c, 8, top + 2);
            if has(&["horned", "viking"]) {
                for i in 0..5 {
                    both(c, 4 - i / 2, 8 - i, WHITE);
                }
            }
            if has(&["plumed", "crested", "feathered"]) {
                for i in 0..6 {
                    c.set(CX + i / 2, 2 - i / 3, RED);
                    c.set(CX + i / 2, 3 - i / 3, RED);
                }
            }
        }
        Kind::Crown => {
            let low = has(&["circlet", "tiara", "diadem"]);
            c.rect(4, 14, 21, if low { 17 } else { 19 }, main);
            if !low {
                for x in [4, 8, 12, 16, 20] {
                    peak(c, x, 6, 14, 1, main);
                    c.light(x, 6, light);
                }
            } else {
                peak(c, CX, 10, 14, 2, main);
            }
            c.rect(4, 14, 21, 15, light);
            let gem = k.bright.filter(|g| *g != main).unwrap_or(LIGHTRED);
            for x in [8, 12, 16] {
                c.light(x, 16, if x == 12 { gem } else { LIGHTBLUE });
            }
        }
        Kind::Hood => {
            if has(&["mask"]) {
                c.oval(CX, 12, 8, 10, main);
                c.rect(7, 9, 11, 11, BLACK);
                c.rect(14, 9, 18, 11, BLACK);
                c.rect(10, 17, 15, 18, dark);
            } else {
                // A hood's shape: rounded on top, open at the face, falling to the shoulders.
                c.oval(CX, 11, 9, 10, main);
                c.rect(1, 18, 24, 25, main);
                c.oval(CX, 13, 5, 7, BLACK);
                hoop(c, (CX, 13, 6, 8), (CX, 13, 5, 7), dark);
            }
        }
        Kind::Hat => {
            let brim = has(&["hat", "fedora", "bonnet"]);
            let pointed = has(&["pointed", "wizard", "witch", "conical"]);
            if pointed {
                peak(c, CX, 1, 17, 7, main);
                c.rect(2, 17, 23, 20, main);
                c.light(CX, 6, YELLOW);
            } else if has(&["headband", "bandana", "band"]) {
                hoop(c, (CX, 13, 10, 5), (CX, 13, 8, 3), main);
            } else {
                cap(c, (CX, 17, 8, 10), 7, 17, main);
                if brim {
                    c.rect(1, 16, 24, 19, main);
                    c.rect(4, 14, 21, 16, dark);
                } else {
                    c.rect(4, 16, 21, 18, dark);
                }
            }
            if has(&["feathered", "feather", "plumed"]) {
                line(c, (18, 13), (22, 4), WHITE);
                line(c, (19, 13), (23, 5), LIGHTGRAY);
            }
        }
        Kind::Armor | Kind::Robe => {
            let robe = kind == Kind::Robe;
            // The body, the sleeves, the neck's opening.
            c.rect(7, 3, 18, if robe { 25 } else { 21 }, main);
            if robe {
                for y in 14..25 {
                    let out = (y - 14) / 3;
                    c.rect(7 - out, y, 18 + out, y + 1, main);
                }
            }
            c.rect(2, 4, 7, 15, main);
            c.rect(18, 4, 23, 15, main);
            c.rect(2, 4, 7, 5, light);
            c.rect(18, 4, 23, 5, light);
            c.oval(CX, 3, 3, 2, BLACK);
            c.rect(9, 0, 16, 3, BLACK);
            let mail = has(&["mail", "chainmail", "chain", "ringmail", "scale", "hauberk"]);
            let plate = has(&["plate", "platemail", "breastplate", "cuirass"]);
            if mail {
                for y in 4..21 {
                    for x in 2..23 {
                        if c.get(x, y) == main && (x + y) % 2 == 0 {
                            c.set(x, y, dark);
                        }
                    }
                }
            }
            if plate {
                c.rect(CX, 5, CX + 1, 20, light);
                glint(c, 9, 7);
                glint(c, 15, 7);
                c.rect(7, 13, 18, 14, dark);
            }
            if !mail && !plate {
                c.rect(7, 13, 18, 14, if robe { YELLOW } else { BROWN });
                for y in (6..12).step_by(2) {
                    c.set(CX, y, k.metal);
                }
            }
        }
        Kind::Legs => {
            let skirt = has(&["skirt", "kilt"]);
            c.rect(6, 2, 19, 6, main);
            c.rect(6, 2, 19, 3, BROWN);
            if skirt {
                for y in 6..20 {
                    let out = (y - 6) / 3;
                    c.rect(6 - out, y, 19 + out, y + 1, main);
                }
                for x in (5..21).step_by(3) {
                    line(c, (x, 8), (x, 19), dark);
                }
            } else {
                c.rect(6, 6, 12, 24, main);
                c.rect(13, 6, 19, 24, main);
                line(c, (12, 6), (12, 23), dark);
                if has(&["greaves"]) {
                    glint(c, 8, 15);
                    glint(c, 15, 15);
                }
            }
        }
        Kind::Boots => {
            for (x0, dx) in [(3, 0), (12, 1)] {
                let x = x0 + dx;
                c.rect(x, 4, x + 6, 18, main);
                c.rect(x, 16, x + 10, 21, main);
                c.oval(x + 9, 19, 1, 2, main);
                c.rect(x, 21, x + 10, 22, shade(dark));
                c.rect(x, 4, x + 6, 5, light);
            }
        }
        Kind::Gloves => {
            for (x0, flip) in [(2, false), (13, true)] {
                c.rect(x0 + 1, 10, x0 + 9, 19, main);
                for f in 0..4 {
                    let fx = x0 + 1 + f * 2;
                    c.rect(fx, 4 + (f % 3), fx + 2, 10, main);
                    line(c, (fx + 1, 6 + (f % 3)), (fx + 1, 9), dark);
                }
                let tx = if flip { x0 + 9 } else { x0 - 1 };
                c.rect(tx, 11, tx + 2, 15, main);
                c.rect(x0, 19, x0 + 10, 23, if has(&["gauntlets", "gauntlet"]) { k.metal } else { dark });
            }
        }
        Kind::Cloak => {
            peak(c, CX, 2, 24, 11, main);
            c.oval(CX, 4, 5, 3, main);
            for x in (5..21).step_by(4) {
                line(c, (CX, 6), (x, 23), dark);
            }
            c.light(CX - 3, 5, YELLOW);
            c.light(CX + 3, 5, YELLOW);
        }
        Kind::Belt => {
            hoop(c, (CX, 13, 11, 7), (CX, 13, 10, 5), main);
            c.rect(9, 17, 16, 22, k.metal);
            c.rect(11, 18, 14, 21, BLACK);
            c.rect(12, 19, 13, 20, k.metal);
            glint(c, 9, 17);
        }
        Kind::Ring => {
            hoop(c, (CX, 15, 7, 7), (CX, 15, 5, 5), main);
            hoop(c, (CX, 15, 7, 7), (CX, 15, 6, 6), dark);
            let gem = k.bright.filter(|g| *g != main).unwrap_or(if has(&["signet", "band"]) { main } else { LIGHTRED });
            c.oval(CX, 7, 2, 2, gem);
            c.light(CX - 1, 6, WHITE);
            glint(c, CX - 5, 12);
        }
        Kind::Amulet => {
            let chain = k.metal;
            line(c, (3, 1), (CX, 12), chain);
            line(c, (21, 1), (CX, 12), chain);
            if has(&["collar", "torc"]) {
                hoop(c, (CX, 8, 10, 8), (CX, 8, 8, 6), main);
            } else {
                c.oval(CX, 16, 4, 5, main);
                hoop(c, (CX, 16, 4, 5), (CX, 16, 3, 4), dark);
                let gem = k.bright.filter(|g| *g != main).unwrap_or(LIGHTRED);
                if has(&["symbol", "holy"]) {
                    c.rect(CX, 13, CX + 1, 20, WHITE);
                    c.rect(CX - 2, 15, CX + 3, 16, WHITE);
                } else {
                    c.oval(CX, 16, 1, 2, gem);
                    c.light(CX, 15, WHITE);
                }
            }
        }
        Kind::Bracelet => {
            hoop(c, (CX, 12, 10, 6), (CX, 12, 7, 3), main);
            hoop(c, (CX, 12, 10, 6), (CX, 12, 9, 5), dark);
            c.light(CX, 17, k.bright.unwrap_or(LIGHTCYAN));
            glint(c, 5, 10);
        }
        Kind::Potion => {
            let glass = LIGHTCYAN;
            // A round flask, a tall vial or a square bottle: by the name,
            // else the seed. Its liquid, a neck and a cork.
            let bottle = if has(&["vial", "phial", "tube"]) {
                1
            } else if has(&["bottle", "decanter", "jar", "jug"]) {
                2
            } else if has(&["flask"]) {
                0
            } else {
                rng.below(3)
            };
            match bottle {
                1 => {
                    c.rect(CX - 3, 6, CX + 4, 24, glass);
                    c.rect(CX - 2, 7, CX + 3, 23, BLACK);
                    c.rect(CX - 2, 13, CX + 3, 23, main);
                    c.rect(CX - 3, 3, CX + 4, 6, BROWN);
                    glint(c, CX - 2, 9);
                }
                2 => {
                    c.rect(6, 11, 19, 24, glass);
                    c.rect(7, 12, 18, 23, BLACK);
                    c.rect(7, 16, 18, 23, main);
                    c.rect(CX - 2, 5, CX + 3, 11, glass);
                    c.rect(CX - 1, 5, CX + 2, 12, BLACK);
                    c.rect(CX - 2, 2, CX + 3, 5, BROWN);
                    glint(c, 8, 13);
                    glint(c, 8, 15);
                }
                _ => {
                    c.oval(CX, 16, 7, 7, glass);
                    c.oval(CX, 16, 6, 6, BLACK);
                    cap(c, (CX, 16, 6, 6), 14, 23, main);
                    c.rect(CX - 2, 4, CX + 3, 10, glass);
                    c.rect(CX - 1, 4, CX + 2, 10, BLACK);
                    c.rect(CX - 2, 2, CX + 3, 5, BROWN);
                    glint(c, CX - 3, 12);
                    glint(c, CX - 4, 14);
                }
            }
            if has(&["glowing", "bubbling", "magic", "magical"]) || k.bright.is_some() {
                for _ in 0..4 {
                    let (x, y) = (rng.between(CX - 4, CX + 4), rng.between(16, 21));
                    c.light(x, y, lighter(main));
                }
            }
        }
        Kind::Scroll => {
            let paper = if has(&["parchment", "map", "old"]) { YELLOW } else { k.said.unwrap_or(WHITE) };
            c.rect(6, 5, 19, 21, paper);
            c.rect(4, 3, 21, 6, shade(paper));
            c.rect(4, 20, 21, 23, shade(paper));
            c.rect(4, 3, 5, 6, BROWN);
            c.rect(20, 3, 21, 6, BROWN);
            for y in (8..19).step_by(2) {
                let len = 9 - (y % 3);
                c.rect(8, y, 8 + len, y + 1, if has(&["map"]) { BROWN } else { DARKGRAY });
            }
            if has(&["map"]) {
                c.set(15, 12, LIGHTRED);
            }
            c.light(CX, 21, LIGHTRED);
        }
        Kind::Book => {
            c.rect(5, 3, 20, 22, main);
            c.rect(5, 3, 8, 22, dark);
            c.rect(19, 4, 21, 22, WHITE);
            for y in (5..21).step_by(2) {
                c.set(20, y, LIGHTGRAY);
            }
            c.rect(10, 9, 17, 14, YELLOW);
            c.rect(11, 10, 16, 13, main);
            c.light(13, 11, if has(&["spellbook", "grimoire", "magic", "arcane"]) { LIGHTCYAN } else { YELLOW });
        }
        Kind::Bag => {
            if has(&["backpack", "pack", "knapsack", "rucksack", "haversack"]) {
                c.rect(5, 6, 20, 23, main);
                cap(c, (CX, 9, 8, 6), 3, 9, main);
                c.rect(5, 9, 20, 13, dark);
                c.rect(11, 12, 14, 14, k.metal);
                line(c, (7, 3), (7, 6), dark);
                line(c, (17, 3), (17, 6), dark);
            } else {
                c.oval(CX, 16, 8, 7, main);
                peak(c, CX, 4, 10, 3, main);
                c.rect(CX - 3, 9, CX + 4, 11, dark);
                c.set(CX + 4, 11, dark);
                c.set(CX + 5, 12, dark);
                glint(c, CX - 4, 13);
                if ws.iter().any(|w| ["coins", "gold", "money", "purse"].contains(&w.as_str())) {
                    c.light(CX + 2, 17, YELLOW);
                }
            }
        }
        Kind::Chest => {
            let wood = main;
            c.rect(3, 10, 22, 22, wood);
            cap(c, (CX, 10, 9, 5), 5, 10, wood);
            c.rect(3, 10, 22, 11, dark);
            for x in [5, 19] {
                c.rect(x, 6, x + 1, 22, k.metal);
            }
            c.rect(3, 21, 22, 22, shade(dark));
            c.rect(CX - 1, 10, CX + 2, 14, YELLOW);
            c.set(CX, 12, BLACK);
            for x in (7..19).step_by(4) {
                c.set(x, 16, dark);
            }
        }
        Kind::Barrel => {
            c.oval(CX, 13, 8, 11, main);
            c.rect(4, 3, 21, 23, main);
            cap(c, (CX, 13, 8, 11), 0, 3, main);
            for y in [5, 12, 19] {
                c.rect(4, y, 21, y + 1, k.metal);
            }
            for x in (6..20).step_by(3) {
                line(c, (x, 3), (x, 22), dark);
            }
            c.oval(CX, 3, 8, 1, dark);
        }
        Kind::Cup => {
            if has(&["waterskin", "skin", "canteen"]) {
                c.oval(CX, 15, 7, 8, main);
                c.rect(CX - 1, 3, CX + 2, 8, dark);
                c.rect(CX - 2, 2, CX + 3, 3, BLACK);
                glint(c, CX - 3, 11);
            } else if has(&["bowl"]) {
                cap(c, (CX, 12, 10, 8), 12, 21, main);
                c.rect(2, 12, 23, 13, light);
                c.rect(CX - 3, 20, CX + 4, 22, dark);
            } else {
                c.rect(6, 7, 17, 22, main);
                hoop(c, (17, 14, 4, 4), (17, 14, 2, 2), main);
                c.rect(6, 7, 17, 8, light);
                c.rect(6, 21, 17, 22, dark);
                c.rect(7, 8, 16, 9, if has(&["ale", "beer", "mead"]) { WHITE } else { dark });
                glint(c, 8, 11);
            }
        }
        Kind::Goblet => {
            cap(c, (CX, 5, 7, 8), 5, 13, main);
            c.rect(5, 5, 20, 6, light);
            c.rect(CX - 1, 12, CX + 2, 20, main);
            c.rect(CX - 5, 20, CX + 6, 22, main);
            c.light(CX, 9, k.bright.unwrap_or(LIGHTRED));
            glint(c, 8, 7);
        }
        Kind::Bread => {
            if has(&["pie", "tart"]) {
                c.oval(CX, 15, 10, 5, BROWN);
                c.oval(CX, 13, 9, 3, YELLOW);
                for x in (6..20).step_by(4) {
                    line(c, (x, 11), (x + 2, 15), BROWN);
                }
            } else {
                c.oval(CX, 15, 10, 6, main);
                c.oval(CX, 13, 9, 4, lighter(main));
                for x in (6..19).step_by(4) {
                    line(c, (x, 12), (x + 2, 15), main);
                }
            }
        }
        Kind::Fruit => {
            c.oval(CX, 14, 7, 7, main);
            c.oval(CX - 2, 11, 2, 2, lighter(main));
            line(c, (CX, 7), (CX + 1, 4), BROWN);
            c.oval(CX + 3, 5, 2, 1, GREEN);
            c.oval(CX, 20, 6, 1, dark);
        }
        Kind::Meat => {
            c.oval(10, 12, 7, 6, main);
            c.oval(9, 11, 5, 4, if main == LIGHTRED { RED } else { shade(main) });
            c.oval(10, 12, 7, 6, main);
            c.oval(8, 10, 3, 2, lighter(main));
            bar(c, (15, 16), (19, 20), WHITE);
            c.oval(20, 21, 2, 2, WHITE);
        }
        Kind::Cheese => {
            for y in 8..20 {
                let reach = (y - 8) * 10 / 12;
                c.rect(3, y, 4 + reach + 8, y + 1, main);
            }
            c.rect(3, 19, 22, 21, BROWN);
            for (x, y) in [(7, 12), (11, 16), (15, 15), (6, 17)] {
                c.set(x, y, BROWN);
            }
        }
        Kind::Fish => {
            c.oval(11, 12, 8, 4, main);
            c.oval(11, 13, 6, 2, lighter(main));
            for i in 0..5 {
                c.rect(19 + i, 12 - i, 20 + i, 13 + i, main);
            }
            c.set(6, 11, BLACK);
            line(c, (8, 9), (8, 15), dark);
        }
        Kind::Coins => {
            for (x, y) in [(7, 18), (17, 18), (12, 19), (9, 14), (15, 14), (12, 10)] {
                c.oval(x, y, 4, 2, main);
                c.rect(x - 4, y + 1, x + 5, y + 2, dark);
                c.set(x - 1, y, light);
            }
            glint(c, 11, 9);
            glint(c, 17, 13);
        }
        Kind::Gem => {
            // A cut stone: a table on top, a point below.
            c.rect(6, 7, 19, 11, main);
            c.rect(8, 5, 17, 7, main);
            peak(c, CX, 11, 21, 6, dark);
            for y in 11..21 {
                let reach = 6 - (y - 11) * 6 / 10;
                c.rect(CX - reach, y, CX + reach + 1, y + 1, if y < 15 { main } else { dark });
            }
            line(c, (8, 11), (CX, 21), light);
            c.rect(9, 6, 13, 7, WHITE);
            glint(c, 9, 8);
            c.light(15, 5, WHITE);
        }
        Kind::Key => {
            hoop(c, (6, 8, 4, 4), (6, 8, 2, 2), main);
            c.rect(9, 7, 22, 9, main);
            c.rect(17, 9, 19, 13, main);
            c.rect(20, 9, 22, 12, main);
            c.rect(9, 7, 22, 8, light);
        }
        Kind::Torch => {
            bar(c, (9, 23), (13, 10), k.wood);
            c.rect(11, 9, 16, 12, DARKGRAY);
            c.oval(14, 6, 3, 4, RED);
            c.oval(14, 7, 2, 2, YELLOW);
            c.light(14, 7, WHITE);
            peak(c, 14, 0, 4, 1, RED);
            c.light(14, 3, YELLOW);
        }
        Kind::Lantern => {
            c.rect(7, 6, 18, 21, main);
            c.rect(8, 8, 17, 19, YELLOW);
            c.light(CX, 13, WHITE);
            c.light(CX - 1, 14, WHITE);
            for x in [8, CX, 16] {
                c.rect(x, 8, x + 1, 19, main);
            }
            c.rect(6, 21, 19, 23, dark);
            peak(c, CX, 2, 6, 6, dark);
            hoop(c, (CX, 2, 3, 2), (CX, 2, 2, 1), main);
        }
        Kind::Candle => {
            c.rect(9, 9, 16, 22, main);
            c.rect(9, 9, 16, 10, light);
            c.set(14, 11, light);
            c.set(14, 12, light);
            c.rect(5, 21, 20, 23, LIGHTGRAY);
            line(c, (CX, 7), (CX, 8), DARKGRAY);
            c.oval(CX, 5, 1, 2, YELLOW);
            c.light(CX, 5, WHITE);
        }
        Kind::Lute => {
            if has(&["harp", "lyre"]) {
                line(c, (5, 23), (5, 3), main);
                line(c, (6, 23), (6, 3), main);
                for x in 6..20 {
                    c.set(x, 3 + (x - 6) * (x - 6) / 18, main);
                }
                line(c, (19, 13), (6, 23), main);
                for x in (8..18).step_by(2) {
                    line(c, (x, 5 + (x - 6) * (x - 6) / 18), (x, 22 - (x - 6) * 10 / 13), LIGHTGRAY);
                }
            } else {
                c.oval(9, 16, 6, 7, main);
                c.oval(9, 16, 2, 2, BLACK);
                bar(c, (13, 11), (20, 3), shade(main));
                c.rect(19, 1, 23, 4, dark);
                line(c, (5, 19), (20, 3), LIGHTGRAY);
            }
        }
        Kind::Drum => {
            c.rect(4, 8, 21, 21, main);
            c.oval(CX, 8, 8, 3, WHITE);
            c.oval(CX, 21, 8, 2, main);
            for x in (5..21).step_by(3) {
                line(c, (x, 11), (x + 2, 19), YELLOW);
            }
        }
        Kind::Horn => {
            for i in 0..14 {
                let x = 4 + i;
                let y = 18 - (i * i) / 14;
                let r = 1 + i / 5;
                c.oval(x, y, 1, r, main);
            }
            c.oval(19, 6, 3, 5, main);
            c.oval(19, 6, 1, 3, dark);
            glint(c, 10, 15);
        }
        Kind::Flute => {
            bar(c, (3, 20), (21, 4), main);
            for i in 0..5 {
                c.set(9 + i * 2, 15 - i * 2 + 1, BLACK);
            }
        }
        Kind::Pick => {
            bar(c, (8, 24), (13, 7), k.wood);
            for x in 3..23 {
                let y = 6 + (x - 13) * (x - 13) / 25;
                c.rect(x, y, x + 1, y + 2, k.metal);
            }
            c.set(3, 8, light);
            c.set(22, 8, light);
        }
        Kind::Shovel => {
            bar(c, (CX, 1), (CX, 15), k.wood);
            c.rect(CX - 2, 1, CX + 4, 2, k.wood);
            cap(c, (CX, 15, 5, 8), 15, 24, k.metal);
            c.rect(CX - 5, 15, CX + 7, 18, k.metal);
            glint(c, CX - 2, 18);
        }
        Kind::Rock => {
            let ingot = has(&["ingot", "bar", "brick"]);
            if ingot {
                c.rect(4, 11, 21, 19, main);
                c.rect(6, 8, 19, 11, light);
                c.rect(4, 18, 21, 19, dark);
                glint(c, 8, 9);
            } else {
                c.oval(CX, 15, 9, 7, main);
                c.oval(CX - 3, 12, 4, 3, light);
                for _ in 0..10 {
                    let (x, y) = (rng.between(4, 20), rng.between(9, 21));
                    if within(x, y, (CX, 15, 9, 7)) {
                        c.set(x, y, if has(&["ore", "nugget"]) { YELLOW } else { dark });
                    }
                }
            }
        }
        Kind::Bundle => {
            // Something wrapped and tied: what the painter has no shape for.
            c.oval(CX, 14, 9, 8, main);
            c.oval(CX - 3, 11, 3, 3, light);
            line(c, (CX, 6), (CX, 22), dark);
            line(c, (3, 14), (21, 14), dark);
            c.oval(CX, 6, 2, 1, dark);
        }
    }
}

/// A material's class, as CoffeeMUD sorts its resources
/// (`RawMaterial.Material`): what its texture is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Cloth,
    Leather,
    Metal,
    Mithril,
    Wooden,
    Glass,
    Rock,
    Precious,
    /// Vegetation, flesh, paper, liquid, energy, gas, plastic: smooth.
    Plain,
}

/// The class of a material by its name as EXAMINE says it ("oak",
/// "white gold"; "a kind of wood" to a dull character): `materials.tsv`.
fn class(material: &str) -> Option<Class> {
    let m = material.trim().to_lowercase();
    let m = m.strip_prefix("a kind of ").unwrap_or(&m);
    let found = include_str!("materials.tsv").lines().skip(1).find_map(|l| l.split_once('\t').filter(|(n, _)| *n == m).map(|(_, c)| c))?;
    Some(match found {
        "cloth" => Class::Cloth,
        "leather" => Class::Leather,
        "metal" => Class::Metal,
        "mithril" => Class::Mithril,
        "wooden" => Class::Wooden,
        "glass" => Class::Glass,
        "rock" => Class::Rock,
        "precious" => Class::Precious,
        _ => Class::Plain,
    })
}

/// What the item's main part is made of: what EXAMINE said, else a
/// material its name gives before any "of" ("a steel helmet", not "a bag
/// of gold"), else what its kind is mostly made of.
fn class_of(thing: &Thing, kind: Kind) -> Class {
    if let Some(c) = thing.material.as_deref().and_then(|m| class(m).or_else(|| words(m).iter().find_map(|w| class(w)))) {
        return c;
    }
    let named: Vec<String> = words(&thing.name).into_iter().take_while(|w| w != "of").collect();
    let pairs = named.windows(2).map(|p| format!("{} {}", p[0], p[1]));
    if let Some(c) = pairs.chain(named.iter().cloned()).find_map(|w| class(&w)) {
        return c;
    }
    let has = |any: &[&str]| named.iter().any(|w| any.contains(&w.as_str()));
    match kind {
        Kind::Sword | Kind::Dagger | Kind::Axe | Kind::Spear | Kind::Helmet | Kind::Key | Kind::Hammer | Kind::Pick | Kind::Shovel | Kind::Lantern | Kind::Horn | Kind::Cup | Kind::Goblet => Class::Metal,
        Kind::Mace if !has(&["club", "cudgel", "baton", "truncheon", "sap"]) => Class::Metal,
        Kind::Crown | Kind::Ring | Kind::Amulet | Kind::Bracelet | Kind::Coins => Class::Metal,
        Kind::Armor if has(&["mail", "chainmail", "chain", "ringmail", "scale", "hauberk", "plate", "platemail", "breastplate", "cuirass"]) => Class::Metal,
        Kind::Armor if has(&["jerkin", "jacket", "vest", "brigandine"]) => Class::Leather,
        Kind::Mace | Kind::Staff | Kind::Wand | Kind::Bow | Kind::Arrow | Kind::Shield | Kind::Chest | Kind::Barrel | Kind::Torch | Kind::Lute | Kind::Drum | Kind::Flute => Class::Wooden,
        Kind::Boots | Kind::Gloves | Kind::Belt | Kind::Book => Class::Leather,
        Kind::Armor | Kind::Robe | Kind::Legs | Kind::Cloak | Kind::Hood | Kind::Hat | Kind::Bag | Kind::Bundle => Class::Cloth,
        Kind::Gem => Class::Precious,
        Kind::Rock => Class::Rock,
        _ => Class::Plain,
    }
}

/// Every pixel of a canvas, row by row.
fn pixels(c: &Canvas) -> impl Iterator<Item = (i32, i32)> + '_ {
    (0..c.h).flat_map(move |y| (0..c.w).map(move |x| (x, y)))
}

/// Whether a pixel of the item is on its outline: next to the clear.
fn edge(c: &Canvas, x: i32, y: i32) -> bool {
    c.get(x, y) != CLEAR && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)].iter().any(|&(nx, ny)| !c.inside(nx, ny) || c.get(nx, ny) == CLEAR)
}

/// A material's texture on the pixels of one color: cloth woven,
/// leather stitched, wood grained, metal with a band of light (mithril
/// and gems sparkling too), glass showing the backdrop through, stone
/// speckled.
fn texture(c: &mut Canvas, color: u8, class: Class, rng: &mut Rng) {
    let px: Vec<(i32, i32)> = pixels(c).filter(|&(x, y)| c.get(x, y) == color && !c.is_lit(x, y)).collect();
    if px.len() < 6 {
        return;
    }
    let (dark, light) = (shade(color), lighter(color));
    match class {
        Class::Cloth => {
            for &(x, y) in &px {
                if (x + 2 * y) % 4 == 0 {
                    c.set(x, y, dark);
                }
            }
        }
        Class::Leather => {
            // Stitches a pixel in from the edge, and the grain of the hide.
            let inner: Vec<(i32, i32)> = px.iter().copied().filter(|&(x, y)| !edge(c, x, y) && [(x - 2, y), (x + 2, y), (x, y - 2), (x, y + 2)].iter().any(|&(nx, ny)| c.get(nx, ny) == CLEAR)).collect();
            for &(x, y) in &px {
                if (x * 7 + y * 3) % 13 == 0 {
                    c.set(x, y, dark);
                }
            }
            for (x, y) in inner {
                if (x + y) % 3 == 0 {
                    c.set(x, y, light);
                }
            }
        }
        Class::Wooden if px.len() >= 30 => {
            let wave = rng.between(0, 2);
            for &(x, y) in &px {
                if (y + (x / 5 + wave) % 2) % 3 == 0 && (x + y) % 5 != 0 {
                    c.set(x, y, dark);
                }
            }
        }
        Class::Metal | Class::Mithril | Class::Precious => {
            let sums: Vec<i32> = px.iter().map(|&(x, y)| x + y).collect();
            let (low, high) = (*sums.iter().min().unwrap_or(&0), *sums.iter().max().unwrap_or(&0));
            let band = low + (high - low) / 3;
            for &(x, y) in &px {
                if (x + y == band || x + y == band + 1) && light != color {
                    c.set(x, y, light);
                } else if [(x + 1, y), (x, y + 1)].iter().any(|&(nx, ny)| c.get(nx, ny) == CLEAR) {
                    c.set(x, y, dark);
                }
            }
            let sparkle = match class {
                Class::Mithril => Some((LIGHTCYAN, 3)),
                Class::Precious => Some((WHITE, 2)),
                _ => None,
            };
            if let Some((spark, n)) = sparkle {
                for _ in 0..n {
                    let (x, y) = px[rng.below(px.len() as i32) as usize];
                    c.light(x, y, spark);
                }
            }
        }
        Class::Glass => {
            for &(x, y) in &px {
                if !edge(c, x, y) && (x + y) % 2 == 0 {
                    c.set(x, y, CLEAR);
                }
            }
        }
        Class::Rock => {
            for &(x, y) in &px {
                match rng.below(10) {
                    0 | 1 => c.set(x, y, dark),
                    2 => c.set(x, y, light),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// How worn it is, from the words of its name and description.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Wear {
    /// 0 sound, 1 worn, 2 damaged, 3 battered, 4 broken.
    step: u8,
    rust: bool,
    crude: bool,
    fine: bool,
}

fn wear(thing: &Thing) -> Wear {
    let mut text = thing.name.clone();
    text.push(' ');
    text.push_str(thing.description.as_deref().unwrap_or(""));
    let ws = words(&text);
    let has = |any: &[&str]| ws.iter().any(|w| any.contains(&w.as_str()));
    let step = if has(&["broken", "shattered", "ruined", "splintered", "destroyed", "snapped", "smashed", "busted", "sundered"]) {
        4
    } else if has(&["battered", "notched", "tattered", "torn", "ragged", "pitted", "shoddy", "mangled", "holed"]) {
        3
    } else if has(&["dented", "chipped", "cracked", "frayed", "damaged", "nicked", "scratched", "bent", "scarred"]) {
        2
    } else if has(&["old", "worn", "used", "faded", "scuffed", "weathered", "dull", "tarnished", "threadbare", "aged"]) {
        1
    } else {
        0
    };
    Wear {
        step,
        rust: has(&["rusty", "rusted", "rust", "corroded", "rusting"]),
        crude: has(&["crude", "crudely", "rough", "makeshift", "primitive", "shoddy"]),
        fine: has(&["fine", "masterwork", "masterful", "exquisite", "ornate", "superb", "elegant", "gilded", "jeweled", "jewelled", "engraved", "filigreed"]),
    }
}

/// Its wear on the item's canvas, by what it's made of.
fn damage(c: &mut Canvas, w: Wear, class: Class, k: &Colors, rng: &mut Rng) {
    let shape: Vec<(i32, i32)> = pixels(c).filter(|&(x, y)| c.get(x, y) != CLEAR && !c.is_lit(x, y)).collect();
    if shape.is_empty() {
        return;
    }
    let pick = |rng: &mut Rng, from: &[(i32, i32)]| from[rng.below(from.len() as i32) as usize];
    let hard = matches!(class, Class::Metal | Class::Mithril | Class::Precious);
    let soft = matches!(class, Class::Cloth | Class::Leather | Class::Plain);
    if w.crude {
        for &(x, y) in &shape {
            if edge(c, x, y) && (x * 3 + y) % 3 == 0 {
                c.set(x, y, CLEAR);
            }
        }
    }
    if w.step >= 1 {
        for &(x, y) in &shape {
            if (x * 5 + y * 7) % 9 == 0 && c.get(x, y) != CLEAR {
                let here = c.get(x, y);
                c.set(x, y, shade(here));
            }
        }
    }
    if w.step >= 2 {
        for _ in 0..2 {
            let (x, y) = pick(rng, &shape);
            if hard {
                // A dent: dark, lit on its far side.
                let here = c.get(x, y);
                c.set(x, y, shade(here));
                if c.get(x - 1, y - 1) != CLEAR {
                    c.set(x - 1, y - 1, lighter(here));
                }
            } else if soft {
                // Frayed: bites out of its lower edge.
                let bites: Vec<(i32, i32)> = shape.iter().copied().filter(|&(ex, ey)| ey >= y && edge(c, ex, ey) && (ex + ey) % 2 == 0).take(4).collect();
                for (ex, ey) in bites {
                    c.set(ex, ey, CLEAR);
                }
            } else {
                // A crack.
                let (mut cx, mut cy) = (x, y);
                for _ in 0..5 {
                    if c.get(cx, cy) != CLEAR {
                        c.set(cx, cy, BLACK);
                    }
                    cx += if rng.chance(50) { 1 } else { -1 };
                    cy += 1;
                }
            }
        }
    }
    if w.step >= 3 {
        let outline: Vec<(i32, i32)> = shape.iter().copied().filter(|&(x, y)| edge(c, x, y)).collect();
        if !outline.is_empty() {
            // Notches out of its edge.
            for _ in 0..3 {
                let (x, y) = pick(rng, &outline);
                c.set(x, y, CLEAR);
                c.set(x - 1, y + 1, CLEAR);
            }
        }
        if soft {
            // Holes.
            let inside: Vec<(i32, i32)> = shape.iter().copied().filter(|&(x, y)| !edge(c, x, y)).collect();
            for _ in 0..2.min(inside.len()) {
                let (x, y) = pick(rng, &inside);
                c.set(x, y, CLEAR);
                c.set(x + 1, y, CLEAR);
            }
        }
    }
    if w.step >= 4 {
        // Split in two across its middle, the upper part knocked aside.
        let n = shape.len() as i32;
        let (mx, my) = (shape.iter().map(|p| p.0).sum::<i32>() / n, shape.iter().map(|p| p.1).sum::<i32>() / n);
        let cut = mx + my;
        let mut moved = Vec::new();
        for &(x, y) in &shape {
            let here = c.get(x, y);
            if here == CLEAR {
                continue;
            }
            if x + y == cut || x + y == cut + 1 {
                c.set(x, y, CLEAR);
            } else if x + y < cut {
                moved.push((x, y, here, c.is_lit(x, y)));
                c.set(x, y, CLEAR);
            }
        }
        for (x, y, here, lit) in moved {
            if lit {
                c.light(x - 1, y - 1, here);
            } else {
                c.set(x - 1, y - 1, here);
            }
        }
    }
    if w.rust && ![YELLOW, WHITE, BROWN].contains(&k.metal) {
        for (x, y) in pixels(c).collect::<Vec<_>>() {
            let here = c.get(x, y);
            if (here == k.metal || (hard && here == k.main)) && !c.is_lit(x, y) {
                match (x * 3 + y * 5) % 7 {
                    0 | 3 => c.set(x, y, BROWN),
                    5 => c.set(x, y, RED),
                    _ => {}
                }
            }
        }
    }
    if w.fine {
        // Gilt along its top, a gem set in it.
        let gilt = if k.main == YELLOW { WHITE } else { YELLOW };
        for (x, y) in pixels(c).collect::<Vec<_>>() {
            if c.get(x, y) == k.main && c.get(x, y - 1) == CLEAR && x % 2 == 0 {
                c.set(x, y, gilt);
            }
        }
        let body: Vec<(i32, i32)> = pixels(c).filter(|&(x, y)| c.get(x, y) == k.main && !edge(c, x, y)).collect();
        if !body.is_empty() {
            let (x, y) = body[body.len() / 2];
            c.light(x, y, k.bright.filter(|&b| b != k.main).unwrap_or(LIGHTRED));
        }
    }
}

/// The row a table's top is at.
const TABLE: i32 = 17;

/// A backdrop of one color fading up to black, on a table: boards, their
/// grain, the front edge dark.
fn table(c: &mut Canvas, color: u8) {
    c.gradient(0, TABLE, &[BLACK, color, color]);
    c.rect(0, TABLE, W, LABEL, BROWN);
    for y in TABLE + 1..LABEL {
        for x in 0..W {
            if (x + y * 3) % 7 == 0 {
                c.set(x, y, RED);
            }
        }
    }
    c.rect(0, LABEL - 1, W, LABEL, RED);
}

/// The rows the slot's words take: the picture's last row of cells.
const LABEL: i32 = H - 2;

/// The glow an aura's words give: "glowing", "glowing red".
fn glow(aura: &str) -> Option<u8> {
    let rest = aura.strip_prefix("glowing")?.trim();
    Some(match rest {
        "red" => LIGHTRED,
        "blue" => LIGHTBLUE,
        "purple" => LIGHTMAGENTA,
        "white" => WHITE,
        _ => color_word(rest).map_or(YELLOW, |(_, bright)| bright),
    })
}

/// An item's picture and its slot in words (written on it, and for the
/// screen reader beside it).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Picture {
    pub art: Art,
    pub slot: String,
}

/// The picture with its slot.
pub fn painted(thing: &Thing, game: GameColors) -> Picture {
    Picture { art: picture_in(thing, game), slot: slot(thing, kind(thing)) }
}

/// The picture in the 16 colors.
#[cfg(test)]
fn picture(thing: &Thing) -> Art {
    picture_in(thing, GameColors::Sixteen)
}

/// An item's picture, 25 by 24 pixels in 25 by 12 cells: the item, what's
/// on it, and its slot in words on the bottom row; in the `game`'s colors.
fn picture_in(thing: &Thing, game: GameColors) -> Art {
    let kind = kind(thing);
    let mut rng = Rng::new(seed("item", &key(&thing.name), 0));
    let colors = colors(thing, kind, &mut rng);
    // A backdrop the item stands out from: where it's found, else one color over a table.
    let backdrops = [BLUE, MAGENTA, CYAN, GREEN, RED, BROWN, DARKGRAY];
    let mut back = backdrops[rng.below(backdrops.len() as i32) as usize];
    if back == colors.main || shade(back) == colors.main || back == shade(colors.main) {
        back = if colors.main == BLUE || colors.main == DARKGRAY { MAGENTA } else { BLUE };
    }
    let (mut canvas, light) = match thing.terrain.as_deref().filter(|t| !t.is_empty()) {
        Some(t) => {
            let (c, l) = figure::terrain(&key(&thing.name), t, SIZE.0, SIZE.1);
            (c, Some(l))
        }
        None => {
            let mut c = Canvas::new(SIZE.0, SIZE.1);
            table(&mut c, back);
            (c, None)
        }
    };
    // The item on a canvas of its own, its material and wear on it, then
    // laid on the backdrop as what's on it says.
    let mut item = figure::clear(SIZE.0, SIZE.1);
    paint(&mut item, kind, &colors, thing, &mut rng);
    let mut grain = Rng::new(seed("item-wear", &key(&thing.name), 0));
    let made = class_of(thing, kind);
    texture(&mut item, colors.main, made, &mut grain);
    if colors.wood != colors.main {
        texture(&mut item, colors.wood, Class::Wooden, &mut grain);
    }
    if colors.metal != colors.main && colors.metal != colors.wood {
        texture(&mut item, colors.metal, Class::Metal, &mut grain);
    }
    damage(&mut item, wear(thing), made, &colors, &mut grain);
    let has = |any: &[&str]| thing.auras.iter().any(|a| any.contains(&a.as_str()));
    let lifted = if has(&["floating", "flying", "hovering", "levitating"]) { 3 } else { 0 };
    let invisible = has(&["invisible"]);
    let hidden = has(&["hidden"]);
    let at = |x: i32, y: i32| item.get(x, y + lifted);
    let shaped = |x: i32, y: i32| item.inside(x, y + lifted) && at(x, y) != CLEAR;
    // Its glow, under it: the backdrop next to it lit.
    for aura in &thing.auras {
        let Some(color) = glow(aura) else { continue };
        for y in 0..LABEL {
            for x in 0..W {
                if shaped(x, y) {
                    continue;
                }
                let near = |d: i32| (-d..=d).any(|dy| (-d..=d).any(|dx| shaped(x + dx, y + dy)));
                if near(1) || (near(2) && (x + y) % 2 == 0) {
                    canvas.light(x, y, color);
                }
            }
        }
    }
    // Its shadow: under it where it stands, below it where it floats.
    let low = (0..LABEL).rev().find(|&y| (0..W).any(|x| shaped(x, y))).unwrap_or(LABEL - 1);
    if lifted > 0 {
        for y in low + 1..(low + 3).min(LABEL) {
            for x in CX - 6..=CX + 6 {
                if (x + y) % 2 == 0 {
                    canvas.set(x, y, BLACK);
                }
            }
        }
    } else {
        for x in 0..W {
            if let Some(bottom) = (0..LABEL).rev().find(|&y| shaped(x, y)) {
                if bottom + 1 < LABEL && bottom + 1 >= low - 1 && (x + bottom) % 2 == 0 {
                    canvas.set(x, bottom + 1, BLACK);
                }
            }
        }
    }
    canvas.pen = Part::Body;
    for y in 0..H {
        for x in 0..W {
            if !shaped(x, y) || (invisible && (x + y) % 2 == 1) {
                continue;
            }
            let c = at(x, y);
            if item.is_lit(x, y + lifted) {
                canvas.light(x, y, c);
            } else {
                canvas.set(x, y, if hidden { shade(c) } else { c });
            }
            canvas.unglyph(x, y);
        }
    }
    canvas.pen = Part::Back;
    if let (Some(light), false) = (&light, invisible) {
        figure::light(&mut canvas, light);
    }
    if has(&["bound"]) {
        let ys: Vec<i32> = (0..LABEL).filter(|&y| (0..W).any(|x| shaped(x, y))).collect();
        if let Some(&mid) = ys.get(ys.len() / 2) {
            for x in 0..W {
                if shaped(x, mid) || shaped(x, mid + 1) {
                    canvas.set(x, mid, if x % 2 == 0 { LIGHTGRAY } else { DARKGRAY });
                }
            }
        }
    }
    // Sparks of magic and glints of metal: characters by it, on the backdrop.
    let sparks: Vec<(char, u8)> = [("magical aura", '*', LIGHTMAGENTA), ("metallic aura", '+', WHITE), ("mithril aura", '+', LIGHTCYAN)]
        .iter()
        .filter(|(aura, ..)| has(&[aura]))
        .map(|&(_, c, color)| (c, color))
        .collect();
    for (c, color) in sparks {
        let mut placed = 0;
        for _ in 0..200 {
            if placed == 4 {
                break;
            }
            let (x, y) = (rng.between(0, W - 1), rng.between(0, LABEL / 2 - 1) * 2);
            let by = (-3..=3).any(|dy| (-3..=3).any(|dx| shaped(x + dx, y + dy)));
            if by && !shaped(x, y) && !shaped(x, y + 1) {
                canvas.glyph(x, y, c, color);
                placed += 1;
            }
        }
    }
    // The slot, in words on the bottom row, centered.
    canvas.rect(0, LABEL, W, H + 1, BLACK);
    let words = slot(thing, kind);
    let start = (W - words.chars().count() as i32) / 2;
    for (i, ch) in words.chars().enumerate() {
        canvas.glyph(start + i as i32, LABEL, ch, LIGHTGRAY);
    }
    canvas.art_in(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Color;

    fn thing(name: &str) -> Thing {
        Thing { name: name.into(), ..Thing::default() }
    }

    fn colors_in(art: &Art) -> Vec<u8> {
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
    fn the_name_says_what_it_is() {
        assert_eq!(kind(&thing("a pair of leather boots")), Kind::Boots);
        assert_eq!(kind(&thing("a bag of gold coins")), Kind::Bag);
        assert_eq!(kind(&thing("a potion of healing")), Kind::Potion);
        assert_eq!(kind(&thing("a gold ring")), Kind::Ring);
        assert_eq!(kind(&thing("a silver key")), Kind::Key);
        assert_eq!(kind(&thing("12 gold coins")), Kind::Coins);
        assert_eq!(kind(&thing("a pile of gold")), Kind::Coins);
        assert_eq!(kind(&thing("a ruby")), Kind::Gem);
        assert_eq!(kind(&thing("a rusty scimitar")), Kind::Sword);
        assert_eq!(kind(&thing("a leather jerkin")), Kind::Armor);
        assert_eq!(kind(&thing("a strange thing")), Kind::Bundle);
        assert_eq!(kind(&Thing { container: true, ..thing("a strange thing") }), Kind::Bag);
    }

    #[test]
    fn every_kind_paints_at_its_size_in_the_fonts_characters() {
        let font = crate::ansi_art::CP437;
        for (k, names) in KINDS {
            let art = picture(&thing(&format!("a {}", names[0])));
            assert_eq!((art.columns, art.rows, art.lines.len()), (SIZE.0, SIZE.1, SIZE.1), "{k:?}");
            for line in &art.lines {
                for span in line {
                    assert!(span.text.chars().all(|c| font.contains(c)), "{k:?}: {:?}", span.text);
                }
            }
        }
        picture(&Thing::default());
    }

    /// The text of a picture's row of cells.
    fn row_text(art: &Art, row: usize) -> String {
        art.lines[row].iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn the_slot_is_written_on_the_bottom_row_centered() {
        let art = picture(&thing("an iron helmet"));
        assert_eq!(row_text(&art, 11), format!("{:^25}", "Worn on head"));
        assert_eq!(row_text(&picture(&thing("a potion")), 11).trim(), "Carried");
        let worn = Thing { worn: Some("held in hands".into()), ..thing("a strange thing") };
        assert_eq!(row_text(&picture(&worn), 11).trim(), "Held in hands");
    }

    #[test]
    fn what_is_on_it_shows() {
        let plain = picture(&thing("a long sword"));
        for aura in ["glowing", "glowing red", "magical aura", "invisible", "hidden", "floating", "bound", "metallic aura"] {
            let with = Thing { auras: vec![aura.into()], ..thing("a long sword") };
            assert_ne!(picture(&with), plain, "{aura}");
        }
        let red = Thing { auras: vec!["glowing red".into()], ..thing("a long sword") };
        assert!(colors_in(&picture(&red)).contains(&LIGHTRED));
        let magic = Thing { auras: vec!["magical aura".into()], ..thing("a long sword") };
        assert!(picture(&magic).lines.iter().flatten().any(|s| s.text.contains('*')));
    }

    #[test]
    fn the_same_item_the_same_picture_and_its_colors_show() {
        assert_eq!(picture(&thing("a long sword")), picture(&thing("a long sword")));
        assert_ne!(picture(&thing("a long sword")), picture(&thing("a dagger")));
        assert!(colors_in(&picture(&thing("a green potion"))).contains(&LIGHTGREEN));
        assert!(colors_in(&picture(&thing("a gold crown"))).contains(&YELLOW));
        // What EXAMINE says it's made of colors it.
        let oak = Thing { material: Some("oak".into()), ..thing("a staff") };
        assert!(colors_in(&picture(&oak)).contains(&BROWN));
        assert_eq!(material_color("helmet"), None);
        assert_eq!(material_color("golden"), Some(YELLOW));
    }
    #[test]
    fn every_kind_material_and_wear_paints_in_the_16_colors() {
        let font = crate::ansi_art::CP437;
        let materials = [None, Some("wool"), Some("leather"), Some("oak"), Some("iron"), Some("mithril"), Some("glass"), Some("granite"), Some("diamond"), Some("a kind of wood")];
        let wear = ["", "old ", "dented ", "battered ", "broken ", "rusty ", "crude ", "fine "];
        for (k, names) in KINDS {
            for (i, m) in materials.iter().enumerate() {
                let t = Thing { material: m.map(str::to_string), terrain: (i % 3 == 0).then(|| "woods".to_string()), ..thing(&format!("a {}{}", wear[i % wear.len()], names[0])) };
                let art = picture(&t);
                assert_eq!((art.columns, art.rows), (SIZE.0, SIZE.1), "{k:?}");
                assert!(colors_in(&art).iter().all(|&c| c < 16), "{k:?} {m:?}: a color past the 16");
                assert!(art.lines.iter().flatten().all(|s| s.text.chars().all(|c| font.contains(c))), "{k:?} {m:?}");
            }
        }
    }

    #[test]
    fn its_material_wear_and_place_show() {
        let plain = picture(&thing("a robe"));
        // A robe is cloth already: wool changes nothing; the rest do.
        assert_eq!(picture(&Thing { material: Some("wool".into()), ..thing("a robe") }), plain);
        for m in ["leather", "oak", "iron", "mithril", "glass", "granite", "diamond"] {
            assert_ne!(picture(&Thing { material: Some(m.into()), ..thing("a robe") }), plain, "{m}");
        }
        for w in ["old", "dented", "cracked", "frayed", "notched", "broken", "rusty", "crude", "fine"] {
            assert_ne!(picture(&thing(&format!("a {w} sword"))), picture(&thing("a sword")), "{w}");
        }
        assert_ne!(picture(&Thing { terrain: Some("cave".into()), ..thing("a chest") }), picture(&thing("a chest")));
        // Rust is wear: the blade stays steel under it.
        assert_eq!(colors(&thing("a rusty sword"), Kind::Sword, &mut Rng::new(1)).main, LIGHTGRAY);
    }

    #[test]
    fn what_its_made_of_by_examine_its_name_or_its_kind() {
        assert_eq!(class("oak"), Some(Class::Wooden));
        assert_eq!(class("white gold"), Some(Class::Metal));
        assert_eq!(class("a kind of wood"), Some(Class::Wooden));
        assert_eq!(class("helmet"), None);
        assert_eq!(class_of(&Thing { material: Some("silk".into()), ..thing("a sword") }, Kind::Sword), Class::Cloth);
        assert_eq!(class_of(&thing("a steel helmet"), Kind::Helmet), Class::Metal);
        assert_eq!(class_of(&thing("a bag of gold coins"), Kind::Bag), Class::Cloth);
        assert_eq!(class_of(&thing("a leather jerkin"), Kind::Armor), Class::Leather);
        assert_eq!(class_of(&thing("a chainmail shirt"), Kind::Armor), Class::Metal);
        assert_eq!(wear(&thing("a rusty, battered sword")), Wear { step: 3, rust: true, crude: false, fine: false });
    }

    #[test]
    fn the_name_shapes_it() {
        for (a, b) in [("a sword", "a scimitar"), ("a sword", "a rapier"), ("a short sword", "a long sword"), ("a dagger", "a kukri"), ("a great helm", "an iron cap"), ("a round shield", "a kite shield"), ("a vial", "a bottle")] {
            assert_ne!(picture(&thing(a)), picture(&thing(b)), "{a} and {b}");
        }
    }
}

#[cfg(test)]
mod look {
    use super::*;

    fn thing(name: &str) -> Thing {
        Thing { name: name.into(), ..Thing::default() }
    }

    /// Writes the items' contact sheet as a PPM to `OUT` to look at while
    /// tuning: every kind; the shapes from numbers; the materials; the
    /// wear; what's on it; in a place.
    /// `OUT=/tmp/i.ppm cargo test item_art::look -- --ignored`.
    #[test]
    #[ignore]
    fn show() {
        let per = 13;
        let mut arts: Vec<Art> = KINDS.iter().map(|(_, names)| picture(&thing(&format!("a {}", names[0])))).chain([picture(&thing("a strange thing"))]).collect();
        let pad = |arts: &mut Vec<Art>| {
            while !arts.len().is_multiple_of(per) {
                arts.push(picture(&thing("a strange thing")));
            }
        };
        pad(&mut arts);
        let names = [
            "a long sword", "a short sword", "a scimitar", "a rapier", "a claymore", "a katana", "a sword", "a dagger", "a kukri", "a stiletto", "a battleaxe", "a hand axe", "an axe",
            "a great helm", "a nasal helmet", "an iron cap", "a helmet", "a round shield", "a kite shield", "a tower shield", "a shield", "a heater shield", "a vial", "a bottle", "a flask", "a potion",
        ];
        arts.extend(names.iter().map(|n| picture(&thing(n))));
        pad(&mut arts);
        let made = [("a robe", "wool"), ("a jerkin", "leather"), ("a chest", "oak"), ("a helmet", "iron"), ("a ring", "mithril"), ("a goblet", "glass"), ("a rock", "granite"), ("a cloak", "silk"), ("a crown", "gold"), ("a gem", "diamond"), ("a staff", "ebony"), ("a shield", "steel"), ("a pair of boots", "leather")];
        arts.extend(made.iter().map(|(n, m)| picture(&Thing { material: Some(m.to_string()), ..thing(n) })));
        let worn = ["an old sword", "a dented helmet", "a cracked staff", "a frayed cloak", "a notched axe", "a tattered robe", "a broken sword", "a rusty sword", "a crude club", "a fine sword", "a masterwork shield", "a broken shield", "a rusty chainmail shirt"];
        arts.extend(worn.iter().map(|n| picture(&thing(n))));
        let placed = [("a long sword", "woods"), ("a chest", "cave"), ("a gold ring", "city"), ("a potion", "stone"), ("a helmet", "plains"), ("a book", "wooden"), ("a fish", "watersurface"), ("a torch", "cave"), ("an amulet", "magic"), ("a lute", "hills"), ("a bag", "desert"), ("a key", "metal"), ("a gem", "mountains")];
        arts.extend(placed.iter().map(|(n, t)| picture(&Thing { terrain: Some(t.to_string()), ..thing(n) })));
        let auras = [("a long sword", "glowing"), ("a potion", "invisible"), ("a gold ring", "floating"), ("a staff", "magical aura"), ("a key", "hidden"), ("a chest", "bound"), ("an amulet", "glowing blue"), ("a steel helmet", "metallic aura")];
        arts.extend(auras.iter().map(|(n, a)| picture(&Thing { auras: vec![a.to_string()], ..thing(n) })));
        pad(&mut arts);
        crate::face::look::sheet(&arts, per, &std::env::var("OUT").unwrap_or_else(|_| "items.ppm".into()));
    }
}

#!/usr/bin/env python3
"""Turns CoffeeMUD's materials and effects into the painter's tables.

Reads the snapshot's `Items/interfaces/RawMaterial.java` and
`Abilities/**/*.java` and writes two tables the painter reads
(`item_art.rs` and `face.rs`):

- `src-tauri/src/materials.tsv`: every resource by its name as EXAMINE
  says it ("It is mostly made of oak.": the resource's name in lower
  case, `_` a space, `CommonMsgs`), and its material class (`cloth`,
  `leather`, `metal`, `mithril`, `wooden`, `glass`, `vegetation`,
  `flesh`, `paper`, `rock`, `liquid`, `precious`, `energy`,
  `synthetic`, `gas`), each `RESOURCE_X = MATERIAL_Y | n`. A character
  of little intelligence is told only the class's noun ("a kind of
  wood"), so those nouns are rows too.
- `src-tauri/src/effect_looks.tsv`: the effects that show on a face,
  by the name `char.effects` and AFFECTS give (each ability's
  `localizedName`), and the look: `halo`, `poisoned`, `sick`,
  `burning`, `frozen`, `blind`, `invisible`, `outlined`, `webbed`,
  `stone`, `bark`, `asleep`, `afraid`, `raging`, `glowing`. Poisons and
  diseases by their folder; the rest by the words of their names.

Run it again whenever the CoffeeMUD snapshot changes:

    python3 scripts/painter-tables.py
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GAME = ROOT / "reference" / "CoffeeMud" / "com" / "planet_ink" / "coffee_mud"
MATERIALS_OUT = ROOT / "src-tauri" / "src" / "materials.tsv"
EFFECTS_OUT = ROOT / "src-tauri" / "src" / "effect_looks.tsv"

# The class nouns a dull character is told (`RawMaterial.Material`).
NOUNS = {
    "cloth": "cloth",
    "leather": "leather",
    "metal": "metal",
    "wood": "wooden",
    "glass": "glass",
    "vegetation": "vegetation",
    "flesh": "flesh",
    "paper": "paper",
    "rock": "rock",
    "liquid": "liquid",
    "gemstone": "precious",
    "energy": "energy",
    "plastic": "synthetic",
    "gas": "gas",
}

# The words of an effect's name and the look they give, the first match
# winning. Whole words, lower case.
LOOKS = [
    ("afraid", ["fear", "terror", "scare", "cowardice"]),
    ("halo", ["sanctuary", "bless", "blessed", "sanctity", "holy", "angelic"]),
    ("blind", ["blind", "blindness", "blinded"]),
    ("invisible", ["invisibility", "invisible", "vanish", "hide", "hidden"]),
    ("outlined", ["faerie"]),
    ("webbed", ["web", "webbed", "entangle", "hold", "paralysis", "paralyze", "snare", "net"]),
    ("asleep", ["sleep", "slumber"]),
    ("raging", ["berzerk", "berserk", "frenzy", "rage", "fury", "bloodlust"]),
    ("burning", ["fire", "flame", "flames", "flaming", "burning", "burn", "immolation", "inferno"]),
    ("frozen", ["frost", "ice", "freeze", "frozen", "chill", "cold", "glacial"]),
    ("stone", ["stoneskin", "stone", "granite"]),
    ("bark", ["barkskin", "bark"]),
    ("glowing", ["light", "glow", "radiance"]),
]

# Words that make an effect about the look's thing without being it: a
# cure, a ward, an attack that lasts no time, a craft, an item's charm.
NOT = {
    "cure", "detect", "sense", "resist", "protection", "remove", "purge", "ward", "weakness", "summon", "wall",
    "fighting", "immunity", "craft", "endow", "clanenchant", "feign", "placebo", "snuff", "control", "building",
    "spinning", "healing", "ink", "ship", "mark", "friend", "spying", "liquor", "line", "set", "trap", "bomb",
    "breathing", "produce", "weapon", "sword", "arrows", "lance", "storm", "sheet", "burst", "hands", "wounds",
    "hammer", "sensitivity", "snatch", "item", "hearth", "land", "moon", "undead", "animal", "other", "metal",
    "strike", "wind", "word", "day", "mount", "empower",
}

LOCALIZED = re.compile(r'localizedName\s*=\s*CMLib\.lang\(\)\.L\("([^"]+)"\)')


def materials() -> list[tuple[str, str]]:
    text = (GAME / "Items" / "interfaces" / "RawMaterial.java").read_text(encoding="utf-8", errors="replace")
    rows = {}
    for name, cls in re.findall(r"RESOURCE_([A-Z_]+)\s*=\s*MATERIAL_([A-Z]+)\s*\|", text):
        if cls == "UNKNOWN":
            continue
        rows[name.replace("_", " ").lower()] = cls.lower()
    for noun, cls in NOUNS.items():
        rows.setdefault(noun, cls)
    return sorted(rows.items())


def look_of(name: str, folder: str) -> str | None:
    if folder == "Poisons":
        return "poisoned"
    if folder == "Diseases":
        return "sick"
    words = re.findall(r"[a-z]+", name.lower())
    if NOT.intersection(words):
        return None
    for look, keys in LOOKS:
        if any(w in keys for w in words):
            return look
    return None


def effects() -> list[tuple[str, str]]:
    rows = {}
    for path in sorted((GAME / "Abilities").rglob("*.java")):
        m = LOCALIZED.search(path.read_text(encoding="utf-8", errors="replace"))
        if not m:
            continue
        name = m.group(1).strip()
        look = look_of(name, path.relative_to(GAME / "Abilities").parts[0])
        if look and name.lower() not in rows:
            rows[name.lower()] = look
    return sorted(rows.items())


def write(path: Path, head: str, rows: list[tuple[str, str]]) -> None:
    path.write_text(head + "\n" + "".join(f"{a}\t{b}\n" for a, b in rows), encoding="utf-8")
    print(f"{path.relative_to(ROOT)}: {len(rows)} rows")


def main() -> None:
    write(MATERIALS_OUT, "material\tclass", materials())
    write(EFFECTS_OUT, "effect\tlook", effects())


if __name__ == "__main__":
    main()

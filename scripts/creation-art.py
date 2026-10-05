#!/usr/bin/env python3
"""Makes the pictures and facts the character-creation guide shows.

Reads CoffeeMUD's own pictures (Apache-2.0, Bo Zimmerman; the snapshot in
`reference/CoffeeMud/`): each race's portrait (`web/pub/images/mxp/
race_*.jpg`, 32 by 32) and each class's (`web/pub/images/classes/*.jpg`,
320 by 320), and turns each into ANSI art as Coupler draws it: two square
pixels a cell (the upper half block), in the 16 VGA colors, a
near-white background made black. Reads each player race's Java
(`Races/*.java`, those a theme makes available) for its facts: stat
changes, senses, height, lifespan, what it knows and can do, and what
sets it apart from the other player races.

Writes `public/creation-art.json`, which components/CreationDialog.tsx
reads when the guide opens (lib/creationArt.ts): the facts always, the
pictures only when the player chooses CoffeeMUD's over Coupler's own
painter (gear → Pictures…; the painter's come first, CLAUDE.md rule 11). Needs Pillow (the
maintainer's, never shipped). Run it again when the snapshot changes:

    python3 scripts/creation-art.py
"""

import base64
import json
import re
from pathlib import Path

from PIL import Image, ImageEnhance

ROOT = Path(__file__).resolve().parent.parent
CM = ROOT / "reference" / "CoffeeMud"
RACE_IMAGES = CM / "web" / "pub" / "images" / "mxp"
CLASS_IMAGES = CM / "web" / "pub" / "images" / "classes"
RACES = CM / "com" / "planet_ink" / "coffee_mud" / "Races"
OUT = ROOT / "public" / "creation-art.json"

# The VGA palette, as src/lib/mud.ts's BASE16.
VGA = [
    (0, 0, 0), (170, 0, 0), (0, 170, 0), (170, 85, 0), (0, 0, 170), (170, 0, 170), (0, 170, 170), (170, 170, 170),
    (85, 85, 85), (255, 85, 85), (85, 255, 85), (255, 255, 85), (85, 85, 255), (255, 85, 255), (85, 255, 255), (255, 255, 255),
]

# The most each kind of picture is drawn at, in pixels across (one a
# cell) and down (two a cell): a race's portrait whole, a class's figure
# as tall as the guide has room for.
RACE_SIZE = (32, 32)
CLASS_SIZE = (40, 32)

STATS = {
    "STRENGTH": "Strength", "DEXTERITY": "Dexterity", "CONSTITUTION": "Constitution",
    "INTELLIGENCE": "Intelligence", "WISDOM": "Wisdom", "CHARISMA": "Charisma",
}
SAVES = {"MAGIC": "magic", "POISON": "poison", "DISEASE": "disease", "FIRE": "fire", "COLD": "cold", "ELECTRIC": "lightning", "ACID": "acid", "MIND": "mind attacks", "PARALYSIS": "paralysis", "TRAPS": "traps", "UNDEAD": "undead"}


def palette_image():
    p = Image.new("P", (1, 1))
    flat = [c for rgb in VGA for c in rgb]
    p.putpalette(flat + [0] * (768 - len(flat)))
    return p


def ansi(path: Path, most: tuple, white_is_background: bool) -> dict:
    """A picture as rows of cells, each a byte: top pixel's color << 4 | bottom's."""
    img = Image.open(path).convert("RGB")
    if white_is_background:
        # The figure alone, without the white around it.
        mask = img.point(lambda v: 255 if v < 232 else 0).convert("L").point(lambda v: 255 if v > 0 else 0)
        box = mask.getbbox()
        if box:
            img = img.crop(box)
    img = ImageEnhance.Color(ImageEnhance.Contrast(img).enhance(1.25)).enhance(1.4)
    scale = min(most[0] / img.width, most[1] / img.height)
    width = max(1, round(img.width * scale))
    height = max(2, round(img.height * scale))
    height += height % 2
    img = img.resize((width, height), Image.LANCZOS)
    background = set()
    if white_is_background:
        # Lighter, so the figure's dark tones don't sink into the black.
        img = ImageEnhance.Brightness(img).enhance(1.35)
        px = img.load()
        for y in range(height):
            for x in range(width):
                r, g, b = px[x, y]
                if min(r, g, b) > 232:
                    px[x, y] = (0, 0, 0)
                    background.add((x, y))
    q = img.quantize(palette=palette_image(), dither=Image.Dither.NONE)
    data = q.load()
    # What's left black inside the figure is its darkest gray: it stays a figure, not an outline.
    for y in range(height):
        for x in range(width):
            if white_is_background and data[x, y] == 0 and (x, y) not in background:
                data[x, y] = 8
    cells = bytearray()
    for row in range(0, height, 2):
        for x in range(width):
            cells.append((data[x, row] & 15) << 4 | (data[x, row + 1] & 15))
    return {"columns": width, "rows": height // 2, "cells": base64.b64encode(bytes(cells)).decode()}


def key(name: str) -> str:
    """How a race or class is looked up: letters only, lower case ("Half Elf" is "halfelf")."""
    return re.sub(r"[^a-z]", "", name.lower())


def words_of(ability: str) -> str:
    """`Skill_Keenvision` is "keen vision"; `Prayer_x`'s and the like lose their kind."""
    name = re.sub(r"^[A-Z][a-z]+_", "", ability)
    name = re.sub(r"(?<=[a-z])(?=[A-Z])", " ", name).lower()
    return {"keenvision": "keen vision"}.get(name, name)


def method_body(src: str, name: str) -> str:
    m = re.search(r"\b" + name + r"\s*\([^)]*\)\s*\{", src)
    if not m:
        return ""
    depth, i = 1, m.end()
    while depth and i < len(src):
        depth += {"{": 1, "}": -1}.get(src[i], 0)
        i += 1
    return src[m.end():i]


def strings(src: str, field: str) -> list:
    m = re.search(field + r"\s*=\s*\{([^}]*)\}", src)
    return re.findall(r'"([^"]+)"', m.group(1)) if m else []


def number(src: str, method: str):
    m = re.search(r"\b" + method + r"\s*\(\)\s*\{\s*return\s+(\d+)\s*;", src)
    return int(m.group(1)) if m else None


def race_facts(path: Path):
    src = path.read_text(errors="replace")
    code = re.search(r"int availabilityCode\(\)\s*\{\s*return ([^;]+);", src)
    if not code or "THEME_" not in code.group(1) or "SKILLONLY" in code.group(1):
        return None
    name = re.search(r'localizedStaticName\s*=\s*CMLib\.lang\(\)\.L\("([^"]+)"\)', src)
    name = name.group(1) if name else path.stem
    affect = method_body(src, "affectCharStats")
    stats = {STATS[s]: int(n) for s, n in re.findall(r"adjStat\(CharStats\.STAT_(\w+)\s*,\s*([+-]?\d+)\)", affect) if s in STATS}
    resists = [SAVES[s] for s, n in re.findall(r"STAT_SAVE_(\w+)\s*\)\s*([+-]\s*\d+)", affect) if s in SAVES and int(n.replace(" ", "")) > 0]
    senses = []
    phy = method_body(src, "affectPhyStats")
    if "CAN_SEE_INFRARED" in phy:
        senses.append("infravision")
    if "CAN_SEE_DARK" in phy:
        senses.append("sees in the dark")
    shortest, variance = number(src, "shortestMale"), number(src, "heightVariance")
    height = shortest + (variance or 0) // 2 if shortest else None
    aging = re.search(r"agingChart\s*=\s*\{([^}]*)\}", src)
    lifespan = int(aging.group(1).split(",")[-1]) if aging else None
    knows = strings(src, "culturalAbilityNames")
    effects = [words_of(a) for a in strings(src, "racialEffectNames")]
    abilities = [words_of(a) for a in strings(src, "racialAbilityNames")]
    return {
        "name": name,
        "stats": stats,
        "resists": resists,
        "senses": senses,
        "height": height,
        "lifespan": lifespan,
        "knows": [words_of(k) for k in knows],
        "gifts": effects + abilities,
    }


def feet(inches: int) -> str:
    return f"{inches // 12} ft {inches % 12} in" if inches % 12 else f"{inches // 12} ft"


def describe(race: dict, all_races: list) -> dict:
    """Facts in few words, and what sets the race apart from the others."""
    facts = []
    if race["stats"]:
        facts.append(", ".join(f"{s} {n:+d}" for s, n in race["stats"].items()) + ".")
    else:
        facts.append("No stat changes.")
    sense = race["senses"] + race["gifts"]
    if sense:
        facts.append(f"{', '.join(sense).capitalize()}.")
    if race["resists"]:
        facts.append(f"Resists {', '.join(race['resists'])}.")
    if race["height"]:
        facts.append(f"About {feet(race['height'])} tall.")
    if race["lifespan"]:
        facts.append(f"Lives up to {race['lifespan']} years.")
    if race["knows"]:
        facts.append(f"Knows {', '.join(race['knows'])}.")

    others = [r for r in all_races if r is not race]
    unlike = []
    for stat in STATS.values():
        mine = race["stats"].get(stat, 0)
        theirs = [r["stats"].get(stat, 0) for r in others]
        if mine > 0 and mine > max(theirs):
            unlike.append(f"the most {stat}")
        elif mine < 0 and mine < min(theirs):
            unlike.append(f"the least {stat}")
    heights = [r["height"] for r in others if r["height"]]
    if race["height"] and heights:
        if race["height"] > max(heights):
            unlike.append("the tallest")
        elif race["height"] < min(heights):
            unlike.append("the smallest")
    lives = [r["lifespan"] for r in others if r["lifespan"]]
    if race["lifespan"] and lives:
        if race["lifespan"] > max(lives):
            unlike.append("the longest lived")
        elif race["lifespan"] < min(lives):
            unlike.append("the shortest lived")
    theirs = {g for r in others for g in r["senses"] + r["gifts"]}
    unlike += [f"alone with {g}" for g in race["senses"] + race["gifts"] if g not in theirs]
    if not race["stats"] and all(r["stats"] for r in others):
        unlike.append("the only one with no stat changes: good at anything")
    return {"facts": " ".join(facts), "unlike": f"Of the races: {', '.join(unlike)}." if unlike else ""}


def main():
    races = [f for f in (race_facts(p) for p in sorted(RACES.glob("*.java"))) if f]
    described = {key(r["name"]): {"name": r["name"], **describe(r, races)} for r in races}
    out = {"races": {}, "classes": {}}
    for path in sorted(RACE_IMAGES.glob("race_*.jpg")):
        k = key(path.stem[len("race_"):])
        out["races"][k] = {"art": ansi(path, RACE_SIZE, False), **described.get(k, {})}
    for k, d in described.items():
        out["races"].setdefault(k, d)
    for path in sorted(CLASS_IMAGES.glob("*.jpg")):
        out["classes"][key(path.stem)] = {"name": path.stem, "art": ansi(path, CLASS_SIZE, True)}
    OUT.write_text(json.dumps(out, separators=(",", ":")) + "\n")
    print(f"{OUT.relative_to(ROOT)}: {len(out['races'])} races ({len(described)} with facts), {len(out['classes'])} classes, {OUT.stat().st_size} bytes")
    for k, d in described.items():
        print(f"  {d['name']}: {d['facts']} {d['unlike']}")


if __name__ == "__main__":
    main()

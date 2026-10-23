#!/usr/bin/env python3
"""Turns CoffeeMUD's races and abilities into the Bestiary's tables.

Reads the snapshot's `Races/*.java` and `Abilities/**/*.java` and writes
two tables `src-tauri/src/bestiary.rs` reads:

- `src-tauri/src/races.tsv`: every race by its name as the game says it
  ("Orc", "Black Dragon") and its racial category ("Goblinoid",
  "Dragon"), a race with no category of its own taking its parent
  class's. CoffeeMUD never tells a player an NPC's race (LOOK says it
  for players only, `CommonMsgs.handleBeingMobLookedAt`), so the
  Bestiary reads it from the name ("an orc") and the long look's
  "than the average Orc".
- `src-tauri/src/abilities.tsv`: the lines an ability prints when a
  creature uses it, as someone watching or its target sees them, the
  one using it left off the front: `<S-NAME> point(s) at <T-NAMESELF>,
  shooting forth a magic missile!` is ` points at *, shooting forth a
  magic missile!`, `*` a name, a pronoun or a value. Only lines one
  ability prints (none another's would match the same), with enough
  fixed words to tell, and never a fizzle (`maliciousFizzle`'s lines
  are shared), and only from the ability's `invoke` (the one using it,
  not the one under it). The game names no ability it's using; this is how the
  Bestiary learns what a creature can do.

Run it again whenever the CoffeeMUD snapshot changes:

    python3 scripts/coffeemud-bestiary.py
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GAME = ROOT / "reference" / "CoffeeMud" / "com" / "planet_ink" / "coffee_mud"
RACES_OUT = ROOT / "src-tauri" / "src" / "races.tsv"
ABILITIES_OUT = ROOT / "src-tauri" / "src" / "abilities.tsv"

# The folders under Abilities/ and what the Bestiary calls each kind.
# Left out: Archon (staff commands), Properties (builders' effects, never
# used), Languages (speaking one isn't a skill shown), Traps, Tech.
KINDS = {
    "Spells": "spell", "Prayers": "prayer", "Druid": "chant", "Songs": "song",
    "Fighter": "fighter skill", "Thief": "thief skill", "Ranger": "ranger skill",
    "Paladin": "paladin skill", "Skills": "skill", "Specializations": "weapon skill",
    "Misc": "power", "SuperPowers": "power", "Common": "common skill",
    "Diseases": "disease", "Poisons": "poison",
}

NAME = re.compile(r'localizedStaticName\s*=\s*CMLib\.lang\(\)\.L\("([^"]*)"\)')
CATEGORY = re.compile(r'localizedStaticRacialCat\s*=\s*CMLib\.lang\(\)\.L\("([^"]*)"\)')
EXTENDS = re.compile(r"public\s+class\s+(\w+)\s+extends\s+(\w+)")
ABILITY_NAME = re.compile(r'localizedName\s*=\s*CMLib\.lang\(\)\.L\("([^"]*)"\)')
STRING = re.compile(r'L\("((?:[^"\\]|\\.)*)"')
# Colors and MXP: ^S, ^?, ^<FIGHT^>.
COLOR = re.compile(r"\^<[^>]*\^>|\^.")
TAG = re.compile(r"<([STO])-([A-Z-]+)>")
# Words a race name shares with ordinary words a mob's name has, so
# they're never read as its race ("a tree spirit" isn't a Tree).
NOT_FROM_NAMES = {"Unique", "Unknown", "StdRace", "GenRace"}


def races() -> list[tuple[str, str]]:
    files = {}
    for f in sorted((GAME / "Races").glob("*.java")):
        text = f.read_text(encoding="latin-1")
        m = EXTENDS.search(text)
        name = NAME.search(text)
        cat = CATEGORY.search(text)
        files[f.stem] = (m.group(2) if m else None, name.group(1) if name else None, cat.group(1) if cat else None)

    def category(cls: str, seen=()) -> str | None:
        parent, _, cat = files.get(cls, (None, None, None))
        if cat:
            return cat
        if parent and parent in files and parent not in seen:
            return category(parent, seen + (cls,))
        return None

    rows = {}
    for cls, (_, name, _) in files.items():
        cat = category(cls)
        if not name or not cat or name in NOT_FROM_NAMES or cat in ("Unique", "Unknown"):
            continue
        rows[name] = cat
    return sorted(rows.items())


def invoked(text: str) -> str:
    """The body of a class's `invoke` methods: what's printed by the one
    using it. Elsewhere (`unInvoke`, `tick`, `okMessage`) it's mostly what
    the one under it shows ("begins to feel a bit stupid"), no sign the
    creature can use it."""
    bodies = []
    for m in re.finditer(r"public\s+boolean\s+invoke\s*\(", text):
        start = text.find("{", m.end())
        if start < 0:
            continue
        depth, i = 0, start
        while i < len(text):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        bodies.append(text[start:i])
    return "\n".join(bodies)


def render(text: str) -> str | None:
    """A line as someone else sees it, after the one doing it: `*` a wildcard."""
    text = COLOR.sub("", text).replace("\\n", " ").replace("\\r", " ").strip()
    if not text.startswith("<S-NAME>"):
        return None
    text = text[len("<S-NAME>"):]
    text = re.sub(r"y\(ys\)", "ies", text, flags=re.I)
    text = re.sub(r"\((e?s)\)", r"\1", text, flags=re.I)

    def tag(m: re.Match) -> str:
        who, what = m.group(1), m.group(2)
        if who == "S" and what == "HAS-HAVE":
            return "has"
        if who == "S" and what == "IS-ARE":
            return "is"
        return "*"

    text = TAG.sub(tag, text)
    text = re.sub(r"@x\d+", "*", text)
    if "<" in text or ">" in text:
        return None
    text = re.sub(r"\*(\s*\*)+", "*", text)
    text = " ".join(text.split())
    return " " + text if text and not text.startswith("'") else text


def matches(pattern: str, line: str) -> bool:
    """Whether a pattern takes a line, as bestiary.rs `glob` does."""
    rx = "^" + ".+".join(re.escape(part) for part in pattern.split("*")) + "$"
    return re.match(rx, line, re.I) is not None


def abilities() -> list[tuple[str, str, str]]:
    found: dict[str, set[tuple[str, str]]] = defaultdict(set)
    for folder, kind in KINDS.items():
        for f in sorted((GAME / "Abilities" / folder).rglob("*.java")):
            text = f.read_text(encoding="latin-1")
            name = ABILITY_NAME.search(text)
            if not name:
                continue
            for raw in STRING.findall(invoked(text)):
                if "fizzle" in raw.lower():
                    continue
                line = render(raw)
                if not line:
                    continue
                letters = sum(c.isalpha() for c in line.replace("*", ""))
                if letters < 12 or len(line.replace("*", "").split()) < 3:
                    continue
                found[line].add((name.group(1), kind))
    unique = {line: next(iter(a)) for line, a in found.items() if len(a) == 1}
    # One pattern's wildcards may swallow another's line: keep only those
    # no other ability's pattern also takes.
    by_first: dict[str, list[str]] = defaultdict(list)
    for line in unique:
        by_first[line.split()[0].lower() if line.split() else ""].append(line)
    rows = []
    for line, (name, kind) in unique.items():
        sample = line.replace("*", "Bob")
        rivals = by_first.get(line.split()[0].lower(), []) + by_first.get("*", [])
        if any(other != line and unique[other][0] != name and matches(other, sample) for other in rivals):
            continue
        rows.append((name, kind, line))
    return sorted(rows)


def main() -> int:
    if not (GAME / "Races").exists():
        print("missing the CoffeeMUD snapshot in reference/CoffeeMud", file=sys.stderr)
        return 1
    race_rows = races()
    with RACES_OUT.open("w", encoding="utf-8") as f:
        f.write("race\tcategory\n")
        for name, cat in race_rows:
            f.write(f"{name}\t{cat}\n")
    ability_rows = abilities()
    with ABILITIES_OUT.open("w", encoding="utf-8") as f:
        f.write("ability\tkind\tline\n")
        for name, kind, line in ability_rows:
            f.write(f"{name}\t{kind}\t{line}\n")
    print(f"{len(race_rows)} races, {len(ability_rows)} ability lines ({len({r[0] for r in ability_rows})} abilities)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

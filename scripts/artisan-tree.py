#!/usr/bin/env python3
"""Turns CoffeeMUD's Artisan class into Coupler's table of its skills.

Reads `reference/CoffeeMud/com/planet_ink/coffee_mud/CharClasses/Artisan.java`
(every `addCharAbilityMapping`: the skill, the level it's had at, a
base stat it needs, the skills it needs and how well) and each skill's
own class under `Abilities/` (its name as the game says it, and the
first word that uses it), and writes `src/lib/artisanSkills.ts`, which
`src/lib/artisan.ts` (the tree and the mentor) reads.

A skill named without a number in brackets is needed at any
proficiency (`CMAbleMap.addCharAbilityMapping` reads it as 0). Run it
again whenever the CoffeeMUD snapshot changes:

    python3 scripts/artisan-tree.py
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GAME = ROOT / "reference" / "CoffeeMud" / "com" / "planet_ink" / "coffee_mud"
SOURCE = GAME / "CharClasses" / "Artisan.java"
OUT = ROOT / "src" / "lib" / "artisanSkills.ts"

MAPPING = re.compile(
    r'addCharAbilityMapping\(ID\(\),(\d+),"(\w+)",(?:(\d+),)?(true|false)'
    r'(?:,(?:"([^"]*)"|CMParms\.parseSemicolons\("([^"]*)",true\)))?\)'
)
NAME = re.compile(r'localizedName\s*=\s*CMLib\.lang\(\)\.L\("([^"@]+)')
# A Master skill builds its name in name(): "Master Baking@x1".
NAMED = re.compile(r'String name\(\)\s*\{\s*return L\("([^"@]+)')
TRIGGERS = re.compile(r'triggerStrings\s*=\s*I\(new String\[\]\s*\{\s*"([^"]+)"')
STATS = {"STR": "Strength", "INT": "Intelligence", "WIS": "Wisdom", "DEX": "Dexterity", "CON": "Constitution", "CHA": "Charisma"}
# The kind of skill, from the folder its class is in.
KINDS = {"Common": "craft", "Specializations": "weapon"}


def main() -> int:
    if not SOURCE.exists():
        print(f"No CoffeeMUD source at {SOURCE}", file=sys.stderr)
        return 1
    files = {p.stem.lower(): p for p in (GAME / "Abilities").rglob("*.java")}
    rows = []
    for m in MAPPING.finditer(SOURCE.read_text()):
        level, aid, _prof, auto, mask, pre = m.groups()
        path = files.get(aid.lower())
        if path is None:
            print(f"No class for {aid}", file=sys.stderr)
            return 1
        text = path.read_text()
        name = NAME.search(text) or NAMED.search(text)
        trigger = TRIGGERS.search(text)
        stat = None
        if mask:
            s = re.fullmatch(r"\+(\w+) (\d+)", mask.strip())
            if not s or s.group(1) not in STATS:
                print(f"Unknown mask on {aid}: {mask}", file=sys.stderr)
                return 1
            stat = (STATS[s.group(1)], int(s.group(2)))
        needs = []
        for part in (pre or "").split(";"):
            part = part.strip()
            if not part:
                continue
            n = re.fullmatch(r"(\w+)(?:\((\d+)\))?", part)
            needs.append((n.group(1), int(n.group(2) or 0)))
        rows.append(
            {
                "id": aid,
                "name": (name.group(1) if name else aid).strip(),
                "level": int(level),
                "kind": KINDS.get(path.parent.name, "skill"),
                "word": trigger.group(1).strip() if trigger else None,
                "auto": auto == "true",
                "stat": stat,
                "needs": needs,
            }
        )
    # The game matches a needed skill's ID without case ("Leatherworking").
    ids = {r["id"].lower(): r["id"] for r in rows}
    for r in rows:
        fixed = []
        for nid, prof in r["needs"]:
            if nid.lower() not in ids:
                print(f"{r['id']} needs {nid}, which isn't an Artisan skill", file=sys.stderr)
                return 1
            fixed.append((ids[nid.lower()], prof))
        r["needs"] = fixed

    def ts(s):
        return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'

    out = [
        "// Written by scripts/artisan-tree.py from CoffeeMUD's Artisan.java",
        "// (snapshot c1e556f): don't edit by hand, run the script again.",
        "// CoffeeMUD is Copyright Bo Zimmerman, Apache-2.0.",
        'import type { ArtisanSkill } from "./artisan";',
        "",
        "export const ARTISAN_SKILLS: ArtisanSkill[] = [",
    ]
    for r in rows:
        fields = [
            f"id: {ts(r['id'])}",
            f"name: {ts(r['name'])}",
            f"level: {r['level']}",
            f"kind: {ts(r['kind'])}",
        ]
        if r["word"]:
            fields.append(f"word: {ts(r['word'])}")
        if r["auto"]:
            fields.append("auto: true")
        if r["stat"]:
            fields.append(f"stat: [{ts(r['stat'][0])}, {r['stat'][1]}]")
        needs = ", ".join(f"[{ts(n)}, {p}]" for n, p in r["needs"])
        fields.append(f"needs: [{needs}]")
        out.append("  { " + ", ".join(fields) + " },")
    out.append("];")
    OUT.write_text("\n".join(out) + "\n")
    print(f"{len(rows)} skills written to {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

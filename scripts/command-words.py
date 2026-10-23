#!/usr/bin/env python3
"""Turns CoffeeMUD's commands, skills and spells into Coupler's tips.

Reads, in `reference/CoffeeMud/`:

- `com/planet_ink/coffee_mud/Commands/*.java`: each command's access
  words (`getAccessWords`), leaving out the staff's (a `securityCheck`
  that asks `CMSecurity` whether the player is allowed);
- `com/planet_ink/coffee_mud/Abilities/**/*.java`: each skill used by a
  word of its own (`triggerStrings`, BUTCHER) and each spell, prayer,
  chant, song, tune and dance by its name after CAST, PRAY, CHANT, SING,
  PLAY or DANCE (`localizedName`);
- `resources/help/*.ini` (all but the Archon's): each one's Usage and
  Examples, and its first sentence; one with no entry is left out
  (the game's own, or the staff's).

and writes `public/command-words.json`, which `src/lib/commandWords.ts`
reads when the Triggers, Gags, Aliases and Tickers dialog first wants a
tip. Run it again whenever the CoffeeMUD snapshot changes:

    python3 scripts/command-words.py
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GAME = ROOT / "reference" / "CoffeeMud"
CODE = GAME / "com" / "planet_ink" / "coffee_mud"
HELP = GAME / "resources" / "help"
OUT = ROOT / "public" / "command-words.json"

HELP_FILES = ["help.ini", "common_help.ini", "skill_help.ini", "spell_help.ini", "prayer_help.ini", "chant_help.ini", "songs_help.ini", "misc_help.ini"]
ACCESS = re.compile(r'(?:access|accessWords?)\s*=\s*I\(\s*new String\[\]\s*\{([^}]*)\}')
TRIGGERS = re.compile(r'triggerStrings\s*=\s*I\(\s*new String\[\]\s*\{([^}]*)\}')
NAME = re.compile(r'localizedName\s*=\s*CMLib\.lang\(\)\.L\("([^"@]+)"')
WORD = re.compile(r'"([^"]+)"')
STAFF = re.compile(r'boolean securityCheck\(final MOB mob\)\s*\{[^}]*CMSecurity\.isAllowed', re.S)
# The abilities used by a word of their kind, by their classes' prefix.
INVOKED = {"Spell_": "CAST", "Prayer_": "PRAY", "Chant_": "CHANT", "Song_": "SING", "Play_": "PLAY", "Dance_": "DANCE"}
# Not used by the player at all: the staff's, and what afflicts or is built in.
LEFT_OUT = {"Archon", "Diseases", "Poisons", "Properties", "Traps", "Languages"}
ABOUT_LENGTH = 200


def read_help() -> dict:
    """Every help entry, its key in upper case, its text as one string."""
    entries: dict = {}
    for name in HELP_FILES:
        key, text = None, []
        for raw in (HELP / name).read_text(encoding="latin-1").splitlines():
            if key is None:
                if raw.startswith("#") or "=" not in raw:
                    continue
                key, rest = raw.split("=", 1)
                key = key.strip().upper()
                text = [rest]
            else:
                text.append(raw)
            if not text[-1].endswith("\\"):
                entries.setdefault(key, "\n".join(t.rstrip("\\") for t in text))
                key = None
            else:
                text[-1] = text[-1][:-1]
    return entries


def entry(help: dict, *keys: str):
    """The entry for the first key there, following `KEY=OTHER` redirects."""
    for key in keys:
        seen = set()
        key = key.upper()
        while key in help and key not in seen:
            seen.add(key)
            text = help[key].strip()
            if re.fullmatch(r"[A-Z0-9_ ]+", text) and text != key:
                key = text
                continue
            return text
    return None


def parse(text: str) -> dict:
    """Usage, examples and the first sentence of an entry."""
    text = text.replace("<ABILITY>", "").replace("\\n\\r", "\n").replace("\\n", "\n").replace("\\r", "")
    usage, examples, rest = "", [], []
    for line in text.split("\n"):
        label, _, value = line.partition(":")
        label = label.strip().lower()
        if value and label in ("usage", "syntax"):
            usage = usage or " ".join(value.split())
        elif value and label == "example":
            examples.append(" ".join(value.split()))
        elif value and label in ("command", "short(s)", "shorts", "short", "alts(s)", "alts", "alias", "aliases", "skill", "spell", "prayer", "chant", "song", "domain", "domains", "allows", "allows(s)", "availability", "chant domain", "prayer domain", "spell domain"):
            continue
        else:
            rest.append(line)
    words = " ".join(" ".join(rest).split())
    words = re.sub(r"\^[A-Za-z?]", "", words)
    sentence = re.split(r"(?<=[.!?])\s", words, maxsplit=1)[0] if words else ""
    if len(sentence) > ABOUT_LENGTH:
        sentence = sentence[: ABOUT_LENGTH - 1].rsplit(" ", 1)[0] + "…"
    return {"usage": usage, "examples": examples[:3], "about": sentence}


def words_in(match) -> list:
    return [w.upper() for w in WORD.findall(match.group(1)) if w.strip()]


def main() -> int:
    if not CODE.is_dir():
        print(f"No CoffeeMUD source at {GAME}", file=sys.stderr)
        return 1
    help = read_help()
    commands = []
    for path in sorted((CODE / "Commands").glob("*.java")):
        source = path.read_text(encoding="latin-1")
        found = ACCESS.search(source)
        if not found or STAFF.search(source):
            continue
        words = words_in(found)
        if not words:
            continue
        text = entry(help, words[0], path.stem)
        # No help for the player: the staff's or the game's own.
        if text:
            commands.append({"words": words, "kind": "command", **parse(text)})
    abilities: dict = {}
    for path in sorted((CODE / "Abilities").rglob("*.java")):
        if LEFT_OUT & set(path.relative_to(CODE / "Abilities").parts):
            continue
        source = path.read_text(encoding="latin-1")
        name = NAME.search(source)
        if not name:
            continue
        name = name.group(1)
        text = entry(help, path.stem, name.replace(" ", "_"), name.replace(" ", ""))
        if not text:
            continue
        about = parse(text)
        by = next((word for prefix, word in INVOKED.items() if path.stem.startswith(prefix)), None)
        own = TRIGGERS.search(source)
        if by:
            abilities.setdefault(by, []).append({"name": name, **about})
        elif own and words_in(own):
            commands.append({"words": words_in(own), "kind": "skill", "name": name, **about})
    seen = set()
    unique = []
    for c in commands:
        key = tuple(c["words"])
        if key not in seen:
            seen.add(key)
            unique.append(c)
    for list_ in abilities.values():
        list_.sort(key=lambda a: a["name"].lower())
    OUT.write_text(json.dumps({"commands": unique, "abilities": abilities}, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"{len(unique)} commands and skills, {sum(len(v) for v in abilities.values())} spells and the like: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

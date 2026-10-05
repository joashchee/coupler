#!/usr/bin/env python3
"""Turns CoffeeMUD's socials into the lines a player sees, to match.

Reads `reference/CoffeeMud/resources/socials.txt` (the game's own list)
and writes `docs/coffeemud-socials.tsv`: one row per line a social can
print, for each player who sees it (the one doing it, the target, the
others in the room, or the one doing it when the target isn't there).
Each row has the line with wildcards for people's names and pronouns,
and the same as a regular expression (Rust `regex` syntax) for a whole
line, its color removed and a wrapped line joined back up.

How the server builds those lines, and why the wildcards are where they
are, is in `docs/coffeemud-socials.md`. Run it again whenever the
CoffeeMUD snapshot changes:

    python3 scripts/socials-patterns.py
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "reference" / "CoffeeMud" / "resources" / "socials.txt"
OUT = ROOT / "docs" / "coffeemud-socials.tsv"

# A person or thing named by the game: a name, "a goblin", "someone"
# (can't be seen), a description on the role-playing port.
NAME = r"(.+?)"
# His/her/its/their, himself/herself/themself...: a gender's own word.
WORD = r"(\S+)"

# What each tag becomes, by who's reading (`DefaultSocial.invoke` sends
# it, `CoffeeFilter.fullOutFilter` fills it in). None: a name or a
# pronoun, a wildcard. The one doing it is S, the target T, an item I, E
# or V.
YOU_FORMS = {
    "NAME": "you", "NAMESELF": "you", "YOUPOSS": "your",
    "HIS-HER": "your", "HIM-HER": "you", "HE-SHE": "you",
    "HIM-HERSELF": "yourself", "HIS-HERSELF": "yourself",
}


def tag(who: str, what: str, reader: str, self_form: bool) -> tuple[str, str]:
    """A tag as (wildcard text, regex) for this reader."""
    me = (reader in ("you", "fail") and who == "S") or (reader == "target" and who == "T")
    if me:
        word = YOU_FORMS[what]
        return word, re.escape(word)
    if who != "S" and (self_form or reader == "fail"):
        # No target (a SELF social, or one not here): the tag is empty.
        return "", ""
    if what in ("NAME", "NAMESELF"):
        return "*", NAME
    if what == "YOUPOSS":
        return "*'s", NAME + "'s"
    return "*", WORD


TAG = re.compile(r"<([STIEV])-([A-Z-]+)>", re.IGNORECASE)
COLOR = re.compile(r"\^.")  # ^r, ^?, ^w: colors, gone from the line matched


def verb_endings(text: str, reader_is_subject: bool) -> str:
    """CoffeeFilter's (s)/(es)/(ys): dropped after "you", kept otherwise."""
    if reader_is_subject:
        text = re.sub(r"\(ys\)", "y", text, flags=re.I)
        return re.sub(r"\((e?s)\)", "", text, flags=re.I)
    text = re.sub(r"y\(ys\)", "ies", text, flags=re.I)
    return re.sub(r"\((e?s)\)", r"\1", text, flags=re.I)


def render(text: str, reader: str, self_form: bool) -> tuple[str, str]:
    """The line as wildcard text and as a whole-line regex."""
    text = COLOR.sub("", text).strip()
    subject_is_you = reader == "you" or (reader == "fail")
    text = verb_endings(text, subject_is_you)
    plain, rx, pos = [], [], 0
    for m in TAG.finditer(text):
        lit = text[pos:m.start()]
        plain.append(lit)
        rx.append(re.escape(lit))
        p, r = tag(m.group(1).upper(), m.group(2).upper(), reader, self_form)
        plain.append(p)
        rx.append(r)
        pos = m.end()
    plain.append(text[pos:])
    rx.append(re.escape(text[pos:]))
    plain_s = " ".join("".join(plain).split())
    rx_s = "".join(rx)
    # The game collapses nothing, but a wrapped line is joined with one
    # space: any run of spaces matches any run.
    rx_s = re.sub(r"(\\ )+", r"\\s+", rx_s)
    # The first letter is capitalized by the game ("A goblin smiles."),
    # so the match ignores case.
    return plain_s, "(?i)^" + rx_s + "$"


def main() -> int:
    if not SOURCE.exists():
        print(f"missing {SOURCE.relative_to(ROOT)}", file=sys.stderr)
        return 1
    rows = []
    for line in SOURCE.read_text(encoding="latin-1").splitlines():
        cols = line.split("\t")
        if len(cols) < 3 or not cols[1].strip():
            continue
        cols += [""] * (9 - len(cols))
        name = cols[1].strip().upper()
        verb, _, tail = name.partition(" ")
        target = tail.split(" ")[0] if tail else ""
        form = {
            "": "alone", "SELF": "self", "<T-NAME>": "someone",
            "<I-NAME>": "item in room", "<V-NAME>": "item carried",
            "<E-NAME>": "item worn",
        }.get(target, target.lower())
        self_form = target == "SELF"
        targeted = target.endswith("-NAME>")
        sound = cols[6].strip()
        seen = [("you", cols[2]), ("others", cols[3])]
        if target == "<T-NAME>":
            seen.append(("target", cols[4]))
        if targeted:
            seen.append(("fail", cols[5]))
        for reader, text in seen:
            if not text.strip():
                continue
            plain, rx = render(text, reader, self_form)
            rows.append([verb, name, form, reader, sound, plain, rx])

    # Where a line could be another social's too: the same line, or
    # another's pattern taking this one's text (its wildcards swallow
    # the difference). Prefer the pattern with the most fixed text.
    compiled = [re.compile(r[6]) for r in rows]
    header = ["social", "full name", "form", "seen by", "sound", "text", "regex", "also matches"]
    overlapping = 0
    with OUT.open("w", encoding="utf-8") as f:
        f.write("\t".join(header) + "\n")
        for r in rows:
            sample = r[5].replace("*", "Bob")
            also = sorted({o[1] for o, c in zip(rows, compiled) if o[1] != r[1] and c.match(sample)})
            overlapping += bool(also)
            f.write("\t".join(r + ["; ".join(also)]) + "\n")
    socials = len({r[1] for r in rows})
    verbs = len({r[0] for r in rows})
    print(f"{len(rows)} lines from {socials} socials ({verbs} verbs), {overlapping} could be another's")
    return 0


if __name__ == "__main__":
    sys.exit(main())

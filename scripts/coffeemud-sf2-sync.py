#!/usr/bin/env python3
"""Copies coffeemud.sf2's map and credits from ansiapps' Neumetik
repository, at the bank's release tag (coffeemud-sf2-fetch.py's TAG), so
they match the bank Coupler ships:
- coffeemud/map.tsv to src-tauri/src/soundbank.tsv, the slots
  soundbank.rs reads (program, key, what triggers it, its MSP names,
  once or a loop, and its source; three sources joined by " | " are its
  velocity tiers);
- coffeemud/sources.tsv to src-tauri/resources/coffeemud/CREDITS.txt,
  every recording in the bank with its author, page and licence, shipped
  beside the bank.
Run it after changing the fetch script's TAG, and commit both.

Usage: scripts/coffeemud-sf2-sync.py [neumetik folder]
  (default: ../neumetik beside Coupler, or $NEUMETIK)
"""
import csv
import io
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from importlib import import_module  # noqa: E402

TAG = import_module("coffeemud-sf2-fetch").TAG
MAP = ROOT / "src-tauri" / "src" / "soundbank.tsv"
CREDITS = ROOT / "src-tauri" / "resources" / "coffeemud" / "CREDITS.txt"


def show(repo: Path, path: str) -> str:
    out = subprocess.run(["git", "-C", str(repo), "show", f"{TAG}:{path}"], capture_output=True, text=True, encoding="utf-8")
    if out.returncode != 0:
        sys.exit(f"Can't read {path} at {TAG} in {repo}: {out.stderr.strip()}")
    return out.stdout


def rows(text: str) -> "list[dict]":
    return list(csv.DictReader(io.StringIO(text), delimiter="\t"))


def credits(slots: "list[dict]", sources: "list[dict]") -> str:
    names = {(s["prog"], s["key"]): s["name"] for s in slots}
    licences = {s["licence"] for s in sources}
    if licences != {"CC0-1.0"}:
        sys.exit(f"sources.tsv has licences other than CC0: {sorted(licences - {'CC0-1.0'})}.")
    lines = [
        "coffeemud.sf2: CoffeeMUD's sounds",
        "==================================",
        "",
        f"The SoundFont in this folder ({TAG}) is by ansiapps, made with",
        "Neumetik, and dedicated to the public domain under CC0 1.0",
        "(creativecommons.org/publicdomain/zero/1.0/). Its ambience and",
        "weather loops and its synthesized sound effects are Neumetik's own.",
        "",
        f"Its {len(sources)} recordings are CC0 1.0 too, each one's licence read at",
        "its source. CC0 asks for no credit; they are listed here anyway, by",
        "the slot they play in.",
        "",
    ]
    for s in sources:
        tier = f", {s['tier']}" if s["tier"] else ""
        slot = names.get((s["prog"], s["key"]), s["slug"])
        lines.append(f"{slot}{tier}: \"{s['title']}\" by {s['author']}")
        lines.append(f"  {s['page']}")
    return "\n".join(lines) + "\n"


def main():
    repo = Path(sys.argv[1] if len(sys.argv) > 1 else os.environ.get("NEUMETIK", ROOT.parent / "neumetik")).resolve()
    map_text = show(repo, "coffeemud/map.tsv")
    slots = rows(map_text)
    sources = rows(show(repo, "coffeemud/sources.tsv"))
    CREDITS.parent.mkdir(parents=True, exist_ok=True)
    for path, text in ((MAP, map_text), (CREDITS, credits(slots, sources))):
        with path.open("w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    print(f"Wrote {MAP.relative_to(ROOT)} ({len(slots)} slots) and {CREDITS.relative_to(ROOT)} ({len(sources)} recordings), from {TAG}.")


if __name__ == "__main__":
    main()

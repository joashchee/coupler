#!/usr/bin/env python3
"""Fetches coffeemud.sf2, CoffeeMUD's sounds as a SoundFont (CC0, made by
ansiapps' Neumetik: src-tauri/src/soundbank.rs), checks it, and puts it
where it ships in the app: src-tauri/resources/coffeemud/coffeemud.sf2
(not in git: 77 MB). CREDITS.txt beside it is committed.

This is build tooling, run on the developer's machine and by CI: the app
itself never downloads anything (CLAUDE.md rule 2), it reads the bank
from its own Resources folder. build-release.sh runs this before every
release build; run it once yourself after cloning, before `npm run tauri
dev`.

It comes from Coupler's own release of the bank (RELEASE), pinned by its
SHA-256. If that can't be reached and the GitHub CLI is signed in to an
account that can read ansiapps' Neumetik repository, it's taken from
Neumetik's release of the same tag instead. The download is kept in
src-tauri/target/coffeemud-sf2-download/ so a second run is quick.

A new version of the bank: change TAG and SHA256, then run
scripts/coffeemud-sf2-sync.py for its map and credits.

Usage: scripts/coffeemud-sf2-fetch.py [--check]
  --check  only say whether the bank is there and current.
"""
import hashlib
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "src-tauri" / "resources" / "coffeemud" / "coffeemud.sf2"
CACHE = ROOT / "src-tauri" / "target" / "coffeemud-sf2-download"

TAG = "coffeemud-sf2-v0.1.0"
SHA256 = "5ba53593e2662414c21796c11da22e4a270b5f4b7e111acf50e972c563195ed2"
RELEASE = f"https://github.com/joashchee/coupler/releases/download/{TAG}/coffeemud.sf2"
NEUMETIK = "joashchee/neumetik"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def current(path: Path) -> bool:
    return path.is_file() and sha256(path) == SHA256


def download(dest: Path):
    part = dest.with_suffix(".part")
    part.unlink(missing_ok=True)
    # A connection can end early without an error, so each try is checked
    # against the bank's whole length and a short one carries on from
    # where it stopped.
    total = 0
    print(f"Downloading {RELEASE}")
    for attempt in range(1, 9):
        have = part.stat().st_size if part.is_file() else 0
        request = urllib.request.Request(RELEASE, headers={"Range": f"bytes={have}-"} if have else {})
        try:
            with urllib.request.urlopen(request) as r:
                if have and r.status != 206:
                    have = 0  # the server sent it all again
                if not total:
                    total = int((r.headers.get("Content-Range") or "").rpartition("/")[2] or r.headers.get("Content-Length") or 0)
                with part.open("ab" if have else "wb") as f:
                    shutil.copyfileobj(r, f)
        except OSError as e:
            print(f"  try {attempt}: {e}")
            continue
        got = part.stat().st_size
        if total and got == total:
            part.replace(dest)
            return
        print(f"  try {attempt}: {got} of {total or '?'} bytes, carrying on")
    part.unlink(missing_ok=True)
    print("  not reached; trying Neumetik's release with the GitHub CLI")
    if shutil.which("gh") is None:
        sys.exit("The GitHub CLI (gh) isn't installed, so the bank can't be fetched.")
    subprocess.run(["gh", "release", "download", TAG, "-R", NEUMETIK, "-p", "coffeemud.sf2", "-D", str(dest.parent), "--clobber"], check=True)


def main():
    if "--check" in sys.argv[1:]:
        if current(OUT):
            print(f"coffeemud.sf2 is there and current ({TAG}).")
            return
        sys.exit(f"coffeemud.sf2 is missing or not {TAG}: run scripts/coffeemud-sf2-fetch.py.")
    if current(OUT):
        print(f"coffeemud.sf2 is current ({TAG}).")
        return
    CACHE.mkdir(parents=True, exist_ok=True)
    cached = CACHE / "coffeemud.sf2"
    if not current(cached):
        download(cached)
        if not current(cached):
            got = sha256(cached) if cached.is_file() else "nothing"
            cached.unlink(missing_ok=True)
            sys.exit(f"coffeemud.sf2's SHA-256 is {got}, not {SHA256}: not used.")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(cached, OUT)
    print(f"Wrote {OUT.relative_to(ROOT)} ({TAG}, {OUT.stat().st_size // (1 << 20)} MB).")


if __name__ == "__main__":
    main()

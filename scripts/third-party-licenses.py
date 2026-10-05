#!/usr/bin/env python3
"""Writes public/third-party-licenses.txt: the license texts of everything
Coupler ships, as the licenses ask (BSD and MIT: "the above copyright
notice and this permission notice shall be included"). Vite copies it
into the app, and About -> Open-Source Licenses shows it.

What's included, for one target (the host's, or the one given):
- every Rust crate the app links: normal dependencies, followed from
  Coupler's own crate (not build tools or dev dependencies, which don't
  ship), each with the license files in its folder;
- the C libraries some crates build in (libopus, WavPack, SQLite), whose
  own license files sit deeper in those crates, and Flite's and Pocket
  TTS's speech data;
- every npm production package, from node_modules;
- the VGA font's license (CC BY-SA 4.0), shipped beside the font too.

Identical texts are written once, with every package that uses them.
Some crates leave their license file out of the package they publish.
For those, a sibling crate's file by the same authors (NO_FILE), else
the standard text of the license they declare (scripts/licenses/,
choosing MIT, then BSD-3-Clause, Apache-2.0, MPL-2.0 from an OR) with
their listed authors as the copyright holders, and the entry says so.
A crate whose license has no template fails the script.

build-release.sh runs this before every release build. Run it yourself
whenever Cargo.lock or package-lock.json changes (with license-scan.py),
and commit the result.

Usage: scripts/third-party-licenses.py [rust-target]
"""
import json
import re
import subprocess
import sys
from pathlib import Path

# Coupler's own crates, vendored from ansiapps' private repositories
# (vendor/neumetik/README.md): Coupler's code under Coupler's license,
# not a third party's.
OWN = {"neumetik"}

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "public" / "third-party-licenses.txt"

LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|copyright|notice|unlicense)([-_.].*)?$", re.I)

# Crates that build a C library in, and where its own license is.
BUNDLED_C = {
    "opusic-sys": [("libopus", "opus/COPYING")],
    "wavpack-sys": [("WavPack", "WavPack/COPYING")],
    # Not C, but the same need: Flite's voice and models are CMU's data,
    # under CMU's license, in a file the name pattern above doesn't catch.
    "flite-rs": [("Flite's voice and language data (CMU, University of Edinburgh)", "THIRD-PARTY-LICENSES.md")],
    # Pocket TTS's model and voices (CC BY 4.0, CC0): data in the app's
    # Resources, credited in the file that ships beside them.
    "pocket-tts": [("Pocket TTS's model and voices (Kyutai, VCTK, Alba MacKenna, LibriVox readers)", "../../resources/pocket-tts/CREDITS.txt")],
}

# Crates with no license file in their package, and a sibling crate by the
# same authors whose file is theirs too (checked by hand).
NO_FILE = {
    # Dropbox's brotli allocator pair; alloc-no-stdlib carries the license.
    "alloc-stdlib": "alloc-no-stdlib",
}

# Standard texts, in the order a choice ("MIT OR Apache-2.0") is taken.
TEMPLATES = ["MIT", "BSD-3-Clause", "Apache-2.0", "MPL-2.0"]


def template(spdx: str, authors: list[str]):
    """The standard text of a license `spdx` offers, and which it is."""
    offered = {o.strip() for o in re.split(r"\s+OR\s+|/", spdx.replace("(", " ").replace(")", " ")) if o.strip()}
    for name in TEMPLATES:
        if name in offered:
            f = ROOT / "LICENSE" if name == "Apache-2.0" else ROOT / "scripts/licenses" / f"{name}.txt"
            holders = ", ".join(re.sub(r"\s*<[^>]*>", "", a).strip() for a in authors) or "the authors"
            return name, read(f).replace("{holders}", holders)
    return None

# Built in with no license file anywhere: what to say instead.
PUBLIC_DOMAIN = {
    "libsqlite3-sys": ("SQLite", "SQLite is in the public domain (sqlite.org/copyright.html): no license is required."),
}


def host_target() -> str:
    out = subprocess.run(["rustc", "-vV"], check=True, capture_output=True, text=True, encoding="utf-8").stdout
    return next(line.split(": ", 1)[1] for line in out.splitlines() if line.startswith("host: "))


def license_files(folder: Path) -> list[Path]:
    return sorted((f for f in folder.iterdir() if f.is_file() and LICENSE_FILE.match(f.name)), key=lambda f: f.name.lower())


def read(f: Path) -> str:
    text = f.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n").replace("\r", "\n")
    return "\n".join(line.rstrip() for line in text.split("\n")).strip("\n")


def rust(target: str):
    """(name version (license), [texts]) for each crate the app links."""
    command = ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(ROOT / "src-tauri/Cargo.toml"), "--filter-platform", target]
    # Offline first: every crate the build needs is already fetched.
    run = subprocess.run(command + ["--offline"], capture_output=True, text=True, encoding="utf-8")
    if run.returncode != 0:
        run = subprocess.run(command, check=True, capture_output=True, text=True, encoding="utf-8")
    out = run.stdout
    meta = json.loads(out)
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    pkgs = {p["id"]: p for p in meta["packages"]}
    seen, stack = set(), [meta["resolve"]["root"]]
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            # Only what's linked into the app: not build scripts' tools, not dev.
            if any(k["kind"] is None for k in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    missing = []
    for pid in seen:
        p = pkgs[pid]
        if pid == meta["resolve"]["root"] or p["name"] in OWN:
            continue  # Coupler itself; a vendored crate (vendor/) is listed like any other
        folder = Path(p["manifest_path"]).parent
        label = f"{p['name']} {p['version']} ({p.get('license') or 'see text'})"
        files = license_files(folder)
        if p.get("license_file"):
            extra = folder / p["license_file"]
            if extra.is_file() and extra not in files:
                files.append(extra)
        texts = [read(f) for f in files]
        if not texts and p["name"] in NO_FILE:
            sibling = next((q for q in meta["packages"] if q["name"] == NO_FILE[p["name"]]), None)
            if sibling:
                texts = [read(f) for f in license_files(Path(sibling["manifest_path"]).parent)]
                label += f": its package has no license file; this is {sibling['name']}'s, by the same authors"
        if not texts:
            standard = template(p.get("license") or "", p.get("authors") or [])
            if standard:
                texts = [standard[1]]
                label += f": its package has no license file; the standard {standard[0]} text, its authors as holders"
        if not texts:
            missing.append(label)
        yield label, texts
        for lib, rel in BUNDLED_C.get(p["name"], []):
            f = folder / rel
            if not f.is_file():
                missing.append(f"{lib} in {p['name']} ({rel} not found)")
                continue
            yield f"{lib}, built into {p['name']} {p['version']}", [read(f)]
        if p["name"] in PUBLIC_DOMAIN:
            lib, text = PUBLIC_DOMAIN[p["name"]]
            yield f"{lib}, built into {p['name']} {p['version']}", [text]
    if missing:
        sys.exit("No license text found for:\n  " + "\n  ".join(sorted(missing)) + "\nRead each one's license and add it to NO_FILE or BUNDLED_C.")


def npm():
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    missing = []
    for path, info in lock.get("packages", {}).items():
        if not path or info.get("dev"):
            continue
        folder = ROOT / path
        name = path.split("node_modules/")[-1]
        label = f"{name} {info.get('version', '?')} ({info.get('license') or 'see text'})"
        texts = [read(f) for f in license_files(folder)] if folder.is_dir() else []
        if not texts:
            missing.append(label)
        yield label, texts
    if missing:
        sys.exit("No license text found for (run npm install?):\n  " + "\n  ".join(missing))


def main():
    target = sys.argv[1] if len(sys.argv) > 1 else host_target()
    entries = list(rust(target)) + list(npm())
    entries.append(("IBM VGA 8x16 font by VileR, The Ultimate Oldschool PC Font Pack (CC BY-SA 4.0)", [read(ROOT / "public/fonts/ansiapps/LICENSE.TXT")]))
    # Each distinct text once (texts differing only in spacing and line
    # breaks are the same), with everything under it.
    users: dict[str, list[str]] = {}
    first: dict[str, str] = {}
    for label, texts in entries:
        for text in texts:
            key = " ".join(text.split())
            first.setdefault(key, text)
            users.setdefault(key, []).append(label)
    groups = sorted(((sorted(set(u), key=str.lower), first[k]) for k, u in users.items()), key=lambda g: g[0][0].lower())
    rule = "=" * 72
    parts = [
        "Coupler: third-party software\n"
        f"{rule}\n"
        "Coupler includes the software below, each used under the license\n"
        "that follows its name. Coupler itself is licensed under the Apache\n"
        "License 2.0.\n\n"
        f"{len(entries)} packages, {len(groups)} license texts. Built for {target}.\n"
    ]
    for names, text in groups:
        parts.append(f"{rule}\n" + "\n".join(names) + f"\n{'-' * 72}\n\n{text}\n")
    OUT.write_text("\n".join(parts), encoding="utf-8", newline="\n")
    home = str(Path.home())
    if home in OUT.read_text(encoding="utf-8"):
        sys.exit(f"{OUT} contains {home}: a license text named a local path.")
    print(f"Wrote {OUT.relative_to(ROOT)}: {len(entries)} packages, {len(groups)} license texts, {OUT.stat().st_size // 1024} KB.")


if __name__ == "__main__":
    main()

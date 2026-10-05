#!/usr/bin/env python3
"""License scan for Coupler's dependencies (copied from Stylus's). Run it
whenever Cargo.lock or package-lock.json changes (CLAUDE.md rule 5), with
third-party-licenses.py, which puts the texts in the app.

Coupler takes no GPL/AGPL/LGPL dependencies, so it can ship closed source.
This lists every license in the Rust trees (src-tauri, src-web) and the npm
production tree, and fails on anything copyleft-only, missing, or
not yet reviewed. A license that offers a permissive choice ("MIT OR
LGPL-2.1") passes: we take the permissive option.

REVIEWED holds crates checked by hand, with why they're allowed. Add to it
only after reading the crate's actual license.

Usage: scripts/license-scan.py   (needs the crates fetched: cargo fetch)
"""
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

# Coupler's own crates, vendored from ansiapps' private repositories
# (vendor/neumetik/README.md): Coupler's code under Coupler's license,
# not a third party's.
OWN = {"neumetik"}

ROOT = Path(__file__).resolve().parent.parent

COPYLEFT = re.compile(r"\b(A?GPL|LGPL|EUPL|OSL|CDDL|CC-BY-SA|SSPL)", re.I)
# MPL-2.0 is file-level copyleft: fine to link unmodified, and Tauri itself
# brings some in (cssparser, selectors, option-ext), as in every ansiapps
# app. Listed in the counts, not failed. Modifying an MPL file would mean
# publishing that file's changes.

# name -> why it's allowed. Empty so far: every crate declares its license.
REVIEWED: dict[str, str] = {}


def allowed(lic: str) -> bool:
    # Any OR-alternative without copyleft is a permissive choice we can take.
    for option in re.split(r"\s+OR\s+|/", lic.replace("(", " ").replace(")", " ")):
        if option.strip() and not COPYLEFT.search(option):
            return True
    return False


def cargo_tree(manifest="src-tauri/Cargo.toml"):
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(ROOT / manifest)],
        check=True, capture_output=True, text=True,
    ).stdout
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
            # Normal and build dependencies ship or run in the build; dev ones don't.
            if any(k["kind"] in (None, "build") for k in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    for pid in seen:
        p = pkgs[pid]
        if pid == meta["resolve"]["root"] or p["name"] in OWN:
            continue  # Coupler itself; a vendored crate (vendor/) is listed like any other
        yield p["name"], p["version"], p.get("license") or ""


def npm_tree():
    lock = json.loads((ROOT / "package-lock.json").read_text())
    for path, info in lock.get("packages", {}).items():
        if not path or info.get("dev"):
            continue
        name = path.split("node_modules/")[-1]
        lic = info.get("license") or ""
        if not lic:
            pj = ROOT / path / "package.json"
            if pj.exists():
                lic = json.loads(pj.read_text()).get("license", "")
        yield name, info.get("version", "?"), lic if isinstance(lic, str) else json.dumps(lic)


def scan(label, entries):
    counts, problems = Counter(), []
    for name, version, lic in entries:
        counts[lic or "(none)"] += 1
        if name in REVIEWED:
            continue
        if not lic or not allowed(lic):
            problems.append(f"  {name} {version}: {lic or '(no license declared)'}")
    print(f"{label}: {sum(counts.values())} packages")
    for lic, n in counts.most_common():
        print(f"  {n:4}  {lic}")
    return problems


def main():
    problems = scan("Rust (src-tauri)", cargo_tree())
    # The web build's core (src-web/). Its crates are a few of
    # src-tauri's, so third-party-licenses.txt already covers them; this
    # catches one that drifts or arrives on its own.
    problems += scan("Rust (src-web)", cargo_tree("src-web/Cargo.toml"))
    problems += scan("npm (production)", npm_tree())
    if REVIEWED:
        print("Reviewed by hand:")
        for name, why in REVIEWED.items():
            print(f"  {name}: {why}")
    if problems:
        print("Needs review (copyleft-only, missing, or unknown):", file=sys.stderr)
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print("License scan passed.")


if __name__ == "__main__":
    main()

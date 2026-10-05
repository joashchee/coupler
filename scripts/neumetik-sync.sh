#!/usr/bin/env bash
# Copies Neumetik's source from ansiapps' private Neumetik repository
# into Coupler's copy, src-tauri/vendor/neumetik/src
# (vendor/neumetik/README.md), replacing it. Only src/ moves: Coupler's copy
# keeps its own Cargo.toml and README (Apache-2.0, as Coupler), the
# private one its own license. Then shows what changed and runs the
# crate's tests; commit it yourself.
#   [folder]   the private checkout (default: ../neumetik beside Coupler,
#              or $NEUMETIK)
set -euo pipefail
cd "$(dirname "$0")/.."

from="${1:-${NEUMETIK:-../neumetik}}"
to=src-tauri/vendor/neumetik/src
if [[ ! -f "$from/src/lib.rs" || ! -f "$from/Cargo.toml" ]] || ! grep -q '^name = "neumetik"' "$from/Cargo.toml"; then
  echo "No Neumetik checkout at $from (clone the private repository there, or pass its folder)." >&2
  exit 1
fi

rm -rf "$to"
cp -R "$from/src" "$to"
git status --short -- "$to"
cargo test --quiet --manifest-path src-tauri/vendor/neumetik/Cargo.toml --target-dir src-tauri/target

#!/usr/bin/env bash
# Copies Neumetik MIDI (the neumetik-midi crate, MIT) from ansiapps'
# Neumetik repository into Coupler's copy, src-tauri/vendor/neumetik-midi
# (vendor/neumetik-midi/README.md), replacing it: its src/, Cargo.toml and
# LICENSE. Coupler's copy keeps its own README. Only the MIDI crate
# moves: nothing else in that repository (BGN, the app) comes to Coupler.
# Then shows what changed and runs the crate's tests; commit it yourself.
#   [folder]   the Neumetik checkout (default: ../neumetik beside Coupler,
#              or $NEUMETIK)
set -euo pipefail
cd "$(dirname "$0")/.."

from="${1:-${NEUMETIK:-../neumetik}}/midi"
to=src-tauri/vendor/neumetik-midi
if [[ ! -f "$from/src/lib.rs" || ! -f "$from/Cargo.toml" ]] || ! grep -q '^name = "neumetik-midi"' "$from/Cargo.toml"; then
  echo "No Neumetik MIDI at $from (clone the Neumetik repository beside Coupler, or pass its folder)." >&2
  exit 1
fi

rm -rf "$to/src"
cp -R "$from/src" "$to/src"
cp "$from/Cargo.toml" "$from/LICENSE" "$to/"
git status --short -- "$to"
cargo test --quiet --manifest-path "$to/Cargo.toml" --target-dir src-tauri/target

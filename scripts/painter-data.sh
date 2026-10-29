#!/usr/bin/env bash
# Regenerates painter-data/ for the Painter engine (docs/coupler-handoff.md):
# B1 the reference pictures drawn by the Painter engine (.ans, manifest, PNGs, kits report), B2 the races,
# classes and portrait rules, B3 faces, B4 items, B5 figure and fight, B6 the painter's tables. The folder is
# gitignored: this script is what's kept. Run from anywhere.
set -euo pipefail
cd "$(dirname "$0")/.."
# B1 is drawn by the Painter engine (../painter), never from the Rust painter's output.
node scripts/painter-engine-reference.mjs --painter "${PAINTER_DIR:-../painter}" --to painter-data/engine
# The tests run in src-tauri/, so OUT is relative to it.
OUT=../painter-data cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored portrait::export::painter_data painter::show::painter_tables face::export::painter_data item_art::export::painter_data figure::export::painter_data fight_art::export::painter_data
ls painter-data

#!/usr/bin/env bash
# Builds the web build (coupler.ansiapps.com) into
# dist-web/: the WebAssembly core from src-web/, then the frontend in
# Vite's web mode, then the headers with the inline blocks' hashes, and
# the mirror if there is one (web-mirror/, from scripts/web-publish.sh).
# Fails on anything that mustn't ship: a file Pages can't serve, App
# Testing, or a path from this machine.
#   --wasm-only   only build the core into src/web/coupler.wasm (npm run dev:web)
set -euo pipefail
cd "$(dirname "$0")/.."

# Panic messages carry source paths: the builder's home and the repo's
# place are rewritten, as scripts/build-release.sh does for the desktop.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=~ --remap-path-prefix=$PWD=."
cargo build --release --target wasm32-unknown-unknown --manifest-path src-web/Cargo.toml
cp src-web/target/wasm32-unknown-unknown/release/coupler_web.wasm src/web/coupler.wasm
echo "WebAssembly core: $(wc -c < src/web/coupler.wasm | tr -d ' ') bytes"
node src-web/tests/core.mjs src/web/coupler.wasm
[[ "${1:-}" == "--wasm-only" ]] && exit 0

npx tsc --noEmit
npx vite build --mode web

# The desktop's licenses file covers every crate the web build compiles in
# (src-web's are a few of src-tauri's) and the same npm packages.
node - <<'JS'
const fs = require("fs");
const crypto = require("crypto");
const html = fs.readFileSync("dist-web/index.html", "utf8");
const hashes = (tag) =>
  [...html.matchAll(new RegExp(`<${tag}(?![^>]*\\bsrc=)[^>]*>([\\s\\S]*?)</${tag}>`, "g"))]
    .map((m) => `'sha256-${crypto.createHash("sha256").update(m[1]).digest("base64")}'`)
    .join(" ");
const headers = fs
  .readFileSync("web/_headers", "utf8")
  .replace("{{SCRIPT_HASHES}}", hashes("script"))
  .replace("{{STYLE_HASHES}}", hashes("style"));
fs.writeFileSync("dist-web/_headers", headers);
JS

if [[ -d web-mirror ]]; then
  mkdir -p dist-web/mirror
  cp web-mirror/hooks.json dist-web/mirror.json
  find web-mirror -type f ! -name hooks.json -exec cp {} dist-web/mirror/ \;
  echo "Mirror: $(ls dist-web/mirror | wc -l | tr -d ' ') files"
fi

# Pages refuses a file of 25 MiB or more.
big=$(find dist-web -type f -size +24M)
[[ -z "$big" ]] || { echo "Too big for Cloudflare Pages: $big" >&2; exit 1; }
# App Testing is dev-only.
! grep -l "App Testing" dist-web/assets/*.js >/dev/null 2>&1 || { echo "App Testing is in the web build" >&2; exit 1; }
# No path from this machine.
! grep -a -rl "$HOME" dist-web >/dev/null 2>&1 || { echo "dist-web holds a path from this machine" >&2; exit 1; }
echo "dist-web/: $(du -sh dist-web | cut -f1)"

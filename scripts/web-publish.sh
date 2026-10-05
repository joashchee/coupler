#!/usr/bin/env bash
# Publishes the web build to coupler.ansiapps.com,
# with the web's hooks and assets made to mirror this Mac's Coupler:
#   1. src-tauri/examples/web_mirror.rs reads this Mac's hooks (a copy of
#      hooks.sqlite: only the triggers) and the assets they use, into
#      web-mirror/ (gitignored), replacing what was there.
#   2. scripts/web-build.sh builds dist-web/ with that mirror.
#   3. After you say yes, wrangler uploads dist-web/ to the Pages project
#      `coupler` (wrangler.toml), with wrangler's own login on this Mac.
# For the maintainer only. Anything published is public: publish only
# assets you may share.
#   --no-deploy       mirror and build, then stop (preview: npx vite preview --outDir dist-web)
#   --from <folder>   another Coupler app-data folder to mirror
#   --allow-private   include triggers on pairs that may name a person
set -euo pipefail
cd "$(dirname "$0")/.."

deploy=1
mirror_args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-deploy) deploy=0 ;;
    --from) mirror_args+=(--from "$2"); shift ;;
    --allow-private) mirror_args+=(--allow-private) ;;
    *) echo "Unknown argument $1" >&2; exit 1 ;;
  esac
  shift
done

echo "== Mirroring this Mac's hooks and their assets"
cargo run --release --manifest-path src-tauri/Cargo.toml --example web_mirror -- --to web-mirror ${mirror_args[@]+"${mirror_args[@]}"}

echo "== Building the web build"
scripts/web-build.sh

[[ $deploy == 1 ]] || { echo "Built dist-web/; not deployed."; exit 0; }

echo
read -r -p "Publish this to coupler.ansiapps.com, where anyone can load it? [y/N] " answer
[[ "$answer" == [yY] ]] || { echo "Not published."; exit 0; }
npx --yes wrangler@4 pages deploy dist-web --project-name coupler --branch main --commit-dirty=true

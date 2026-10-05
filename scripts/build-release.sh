#!/bin/sh
# Builds a release Coupler.app (and DMG) without the build machine's paths
# in it. rustc bakes source file paths into panic messages, including those
# of every dependency under ~/.cargo/registry, which would publish the
# builder's home folder (and user name). --remap-path-prefix rewrites them.
# When several prefixes match, rustc applies the last one, so the more
# specific project path comes after $HOME.
#
# Arguments are passed on to `tauri build`, e.g. `--bundles app`.
# Afterwards the built app is searched for $HOME, and the script fails if
# it's still there. Always build releases with this script. Finally the
# built Coupler.app is copied to ~/Applications (see the end).
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

# Every third-party license goes into the app (About -> Open-Source
# Licenses), written fresh from what this build links.
# Pocket TTS's weights and voices (~250 MB, not in git): fetched once,
# checked against their pinned SHA-256s, and shipped in Resources.
python3 scripts/pocket-tts-fetch.py
python3 scripts/third-party-licenses.py

export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=~ --remap-path-prefix=$ROOT=."
# The C libraries the crates compile in (libopus, WavPack) bake their source
# paths in too, through __FILE__; clang's -ffile-prefix-map is the same
# rewrite for them. The cc and cmake crates pass CFLAGS on to the compiler.
export CFLAGS="${CFLAGS:-} -ffile-prefix-map=$HOME=~ -ffile-prefix-map=$ROOT=."
export CXXFLAGS="${CXXFLAGS:-} -ffile-prefix-map=$HOME=~ -ffile-prefix-map=$ROOT=."

npm run tauri build -- "$@"

# The DMG is compressed, but it's made from this .app, so checking the app,
# the bare binary, and the frontend bundle covers everything shipped.
TARGET="$ROOT/src-tauri/target/release"
LEAKS=$(grep -r -l -a -F "$HOME" "$TARGET/app" "$TARGET/bundle/macos" "$ROOT/dist" 2>/dev/null || true)
if [ -n "$LEAKS" ]; then
  echo "The build still contains $HOME in:" >&2
  echo "$LEAKS" >&2
  # Cargo doesn't rebuild a C library when CFLAGS change, so objects built
  # without the prefix map above survive until cleaned.
  echo "If the paths are a C library's (e.g. opusic-sys), clean it and rebuild:" >&2
  echo "  cargo clean --manifest-path src-tauri/Cargo.toml --release -p opusic-sys -p wavpack-sys" >&2
  exit 1
fi
echo "Checked: no $HOME paths in the release build."

# Every local release build replaces the copy in ~/Applications, so the
# installed app is always the latest build. Skipped on CI, or with
# COUPLER_NO_INSTALL=1.
APP="$TARGET/bundle/macos/Coupler.app"
if [ -d "$APP" ] && [ -z "${CI:-}" ] && [ -z "${COUPLER_NO_INSTALL:-}" ]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/Coupler.app"
  ditto "$APP" "$HOME/Applications/Coupler.app"
  echo "Installed: ~/Applications/Coupler.app"
fi

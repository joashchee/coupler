#!/bin/sh
# Builds a release Coupler.app (and DMG) that runs on any Mac someone
# downloads it to: universal (Apple Silicon and Intel), signed with the
# maintainer's Developer ID and the hardened runtime, notarized by Apple
# and stapled, then zipped for the release. Without the build machine's
# paths in it: rustc bakes source file paths into panic messages,
# including those of every dependency under ~/.cargo/registry, which would
# publish the builder's home folder (and user name). --remap-path-prefix
# rewrites them. When several prefixes match, rustc applies the last one,
# so the more specific project path comes after $HOME.
#
# Arguments are passed on to `tauri build`, e.g. `--bundles app`.
# Afterwards the built app is searched for $HOME, and the script fails if
# it's still there. Always build releases with this script. Finally the
# built Coupler.app is copied to ~/Applications (see the end).
#
# Signing and notarization need, once:
#   1. A "Developer ID Application" certificate in the login keychain
#      (Xcode > Settings > Accounts > Manage Certificates > +, or the
#      developer site). Found by itself; with more than one, name it in
#      APPLE_SIGNING_IDENTITY.
#   2. notarytool's credentials in the keychain, under a profile:
#        xcrun notarytool store-credentials coupler \
#          --apple-id you@example.com --team-id TEAMID
#      (it asks for an app-specific password from account.apple.com).
#      Another profile name: COUPLER_NOTARY_PROFILE.
# COUPLER_UNSIGNED=1 builds without them (signed ad hoc, not notarized):
# for trying the build only, never for a release, as Gatekeeper blocks it
# on any other Mac.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

PROFILE="${COUPLER_NOTARY_PROFILE:-coupler}"
UNSIGNED="${COUPLER_UNSIGNED:-}"

# The signing identity, checked before the long build.
if [ -n "$UNSIGNED" ]; then
  # "-" is an ad-hoc signature over the whole bundle.
  APPLE_SIGNING_IDENTITY="-"
  echo "COUPLER_UNSIGNED: signed ad hoc and not notarized; not for release."
elif [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  APPLE_SIGNING_IDENTITY=$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -n 1)
  if [ -z "$APPLE_SIGNING_IDENTITY" ]; then
    echo "No Developer ID Application certificate in the keychain. See this script's header, or set COUPLER_UNSIGNED=1 to try the build unsigned." >&2
    exit 1
  fi
fi
export APPLE_SIGNING_IDENTITY
if [ -z "$UNSIGNED" ]; then
  echo "Signing as: $APPLE_SIGNING_IDENTITY"
  # Notarization is done below with the keychain profile, so the password
  # never sits in the environment; these would make Tauri notarize too.
  unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH 2>/dev/null || true
  if ! xcrun notarytool history --keychain-profile "$PROFILE" >/dev/null 2>&1; then
    echo "No notarytool credentials under the profile \"$PROFILE\". See this script's header." >&2
    exit 1
  fi
fi

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

# Both architectures, built one after the other and joined by lipo.
# Tauri warns that it skips notarization: this script notarizes below.
npm run tauri build -- --target universal-apple-darwin "$@"

TARGETS="$ROOT/src-tauri/target"
TARGET="$TARGETS/universal-apple-darwin/release"
APP="$TARGET/bundle/macos/Coupler.app"
VERSION=$(node -p "require('./src-tauri/tauri.conf.json').version")

# The DMG is compressed, but it's made from this .app, so checking the app,
# each architecture's bare binary, and the frontend bundle covers
# everything shipped.
LEAKS=$(grep -r -l -a -F "$HOME" "$TARGET/bundle/macos" "$TARGETS/aarch64-apple-darwin/release/coupler" "$TARGETS/x86_64-apple-darwin/release/coupler" "$ROOT/dist" 2>/dev/null || true)
if [ -n "$LEAKS" ]; then
  echo "The build still contains $HOME in:" >&2
  echo "$LEAKS" >&2
  # Cargo doesn't rebuild a C library when CFLAGS change, so objects built
  # without the prefix map above survive until cleaned.
  echo "If the paths are a C library's (e.g. opusic-sys), clean it and rebuild:" >&2
  echo "  cargo clean --manifest-path src-tauri/Cargo.toml --release --target aarch64-apple-darwin -p opusic-sys -p wavpack-sys" >&2
  echo "  cargo clean --manifest-path src-tauri/Cargo.toml --release --target x86_64-apple-darwin -p opusic-sys -p wavpack-sys" >&2
  exit 1
fi
echo "Checked: no $HOME paths in the release build."

ARCHS=$(lipo -archs "$APP/Contents/MacOS/coupler")
case "$ARCHS" in
  *arm64*x86_64* | *x86_64*arm64*) echo "Checked: universal ($ARCHS)." ;;
  *) echo "Not universal: $ARCHS" >&2; exit 1 ;;
esac

codesign --verify --deep --strict "$APP"
if [ -z "$UNSIGNED" ]; then
  codesign -dvv "$APP" 2>&1 | grep -q "Authority=Developer ID Application" || { echo "Not signed with a Developer ID." >&2; exit 1; }
  codesign -dvv "$APP" 2>&1 | grep -q "flags=.*runtime" || { echo "Not signed with the hardened runtime." >&2; exit 1; }
  echo "Checked: signed with the Developer ID and the hardened runtime."

  # Notarize: Apple scans a zip of the app, then the ticket is stapled to
  # the app itself so it opens even offline.
  notarize() {
    OUT=$(xcrun notarytool submit "$1" --keychain-profile "$PROFILE" --wait --output-format json)
    STATUS=$(printf '%s' "$OUT" | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);console.log(j.status+" "+j.id)})')
    case "$STATUS" in
      Accepted*) echo "Notarized: $1" ;;
      *)
        echo "Notarization failed ($STATUS). Apple's log:" >&2
        xcrun notarytool log "${STATUS#* }" --keychain-profile "$PROFILE" >&2 || true
        exit 1
        ;;
    esac
  }
  SUBMIT="$TARGET/bundle/macos/Coupler-notarize.zip"
  rm -f "$SUBMIT"
  ditto -c -k --keepParent "$APP" "$SUBMIT"
  notarize "$SUBMIT"
  rm -f "$SUBMIT"
  xcrun stapler staple "$APP"
  spctl --assess --type execute -vv "$APP" 2>&1 | grep -q "source=Notarized Developer ID" || { echo "Gatekeeper doesn't accept the app." >&2; exit 1; }
  echo "Checked: Gatekeeper accepts it as notarized."

  # A DMG, when one was asked for, is notarized and stapled on its own.
  for DMG in "$TARGET"/bundle/dmg/*.dmg; do
    [ -f "$DMG" ] || continue
    notarize "$DMG"
    xcrun stapler staple "$DMG"
  done
fi

# The zip to upload to the version's release, the stapled app inside.
# ditto keeps the signature, symlinks and extended attributes intact.
ZIP="$TARGET/bundle/macos/Coupler_${VERSION}_universal.app.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
echo "For the release: $ZIP"

# Every local release build replaces the copy in ~/Applications, so the
# installed app is always the latest build. Skipped on CI, or with
# COUPLER_NO_INSTALL=1.
if [ -d "$APP" ] && [ -z "${CI:-}" ] && [ -z "${COUPLER_NO_INSTALL:-}" ]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/Coupler.app"
  ditto "$APP" "$HOME/Applications/Coupler.app"
  echo "Installed: ~/Applications/Coupler.app"
fi

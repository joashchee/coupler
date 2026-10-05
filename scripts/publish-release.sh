#!/usr/bin/env bash
# Publishes a GitHub Actions build to a release on GitHub, so every
# successful Windows and Linux build can be downloaded
# (.github/workflows/windows.yml, linux.yml; run by their `publish` job,
# with GH_TOKEN, GITHUB_REPOSITORY and GITHUB_SHA set).
#
# Which release:
# - A `v*` tag, or a commit that is (or will become) the tag of the
#   version in tauri.conf.json: that version's release, "Coupler 0.27.0",
#   made if it's missing (tagging this commit), its notes the version's
#   CHANGELOG.md section. This is the release Coupler's update check sees.
# - Any later commit, while the version's tag already marks an earlier
#   one: the `dev` prerelease, moved to this commit. A build in between
#   versions never replaces a version's files, and the update check
#   never offers it (GitHub's "latest" skips prereleases).
#
# Windows and Linux both publish to the same release, so either may make
# it first: a failed make is tolerated, and the upload that follows says
# whether the release is really there. Files of the same name are
# replaced (--clobber).
#   files...   the installers to upload
set -euo pipefail
cd "$(dirname "$0")/.."

[[ $# -gt 0 ]] || { echo "Nothing to publish." >&2; exit 1; }
repo="$GITHUB_REPOSITORY"
sha="$GITHUB_SHA"
version=$(node -p "require('./src-tauri/tauri.conf.json').version")

# The commit a tag marks, or nothing when there's no such tag
# (an annotated tag's ^{} line names its commit).
tagged() {
  git ls-remote --tags origin "refs/tags/$1" "refs/tags/$1^{}" | sort -k2 | tail -n 1 | cut -f1
}

# A version's section of CHANGELOG.md, without its heading.
notes() {
  awk -v v="$1" '
    index($0, "## " v " ") == 1 || $0 == "## " v { on = 1; next }
    on && /^## / { exit }
    on { print }
  ' CHANGELOG.md
}

if [[ "${GITHUB_REF:-}" == refs/tags/v* ]]; then
  tag="${GITHUB_REF#refs/tags/}"
  version="${tag#v}"
else
  tag="v$version"
  at=$(tagged "$tag")
  [[ -n "$at" && "$at" != "$sha" ]] && tag=dev
fi

if [[ "$tag" == dev ]]; then
  # Move (or make) the dev tag to this commit, then its release.
  gh api -X PATCH "repos/$repo/git/refs/tags/dev" -f sha="$sha" -F force=true >/dev/null 2>&1 ||
    gh api -X POST "repos/$repo/git/refs" -f ref=refs/tags/dev -f sha="$sha" >/dev/null 2>&1 || true
  body="A build between versions, from ${sha:0:7}: everything since Coupler $version (CHANGELOG.md's Unreleased). Unsigned, and not offered by Coupler's update check."
  gh release create dev --repo "$repo" --prerelease --title "Coupler development build" --notes "$body" >/dev/null 2>&1 ||
    gh release edit dev --repo "$repo" --prerelease --notes "$body" >/dev/null
else
  body=$(notes "$version")
  [[ -n "$body" ]] || body="Coupler $version."
  body="$body"$'\n\n'"Unsigned: Windows' SmartScreen warns until the installers are code-signed."
  gh release create "$tag" --repo "$repo" --target "$sha" --title "Coupler $version" --notes "$body" >/dev/null 2>&1 || true
fi

gh release upload "$tag" --repo "$repo" --clobber "$@"
echo "Published to $tag: $*"

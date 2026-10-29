#!/usr/bin/env bash
# Copies the Painter engine (`art-sprites`, ansiapps' own, the `painter`
# repo beside this one) into Coupler's copy, src/vendor/art-sprites: only
# the modules the room picture (`recipes/vista.js`) and `toSpans` reach
# (about 100 KB), replacing them. Coupler's copy keeps
# its `engine.js` entry, which this writes. The engine is ansiapps' own,
# Apache-2.0 like Coupler (rule 5): this refuses a Painter whose
# package.json doesn't say so, and copies its LICENSE and NOTICE. The
# original is the painter repo; change it there and copy, never only here.
#
#   scripts/painter-sync.sh [../painter]
set -euo pipefail
src="$(cd "${1:-../painter}" && pwd)"
grep -q '"license": *"Apache-2.0"' "$src/package.json" || { echo "painter/package.json doesn't say Apache-2.0: not copying it into Coupler (rule 5)" >&2; exit 1; }
for f in LICENSE NOTICE; do [ -f "$src/$f" ] || { echo "painter has no $f" >&2; exit 1; }; done
dest="$(cd "$(dirname "$0")/.." && pwd)/src/vendor/art-sprites"
mkdir -p "$dest"
find "$dest" -type f \( -name '*.js' -o -name VERSION -o -name LICENSE -o -name NOTICE \) -delete
cp "$src/LICENSE" "$src/NOTICE" "$dest/"
node --input-type=module - "$src" "$dest" <<'JS'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
const [src, dest] = process.argv.slice(2);
const seen = new Set();
const walk = (f) => {
  if (seen.has(f)) return;
  seen.add(f);
  for (const m of readFileSync(f, 'utf8').matchAll(/(?:from|import)\s*['"](\.[^'"]+)['"]/g)) walk(resolve(dirname(f), m[1]));
};
for (const entry of ['recipes/vista.js', 'ansi.js']) walk(join(src, entry));
for (const f of seen) {
  const to = join(dest, relative(src, f));
  mkdirSync(dirname(to), { recursive: true });
  writeFileSync(to, readFileSync(f));
}
const { version } = JSON.parse(readFileSync(join(src, 'package.json'), 'utf8'));
writeFileSync(join(dest, 'VERSION'), `${version}\n`);
writeFileSync(join(dest, 'engine.js'), `// Written by scripts/painter-sync.sh: the engine's entry for Coupler.\nexport { drawVista, roomSeed, TERRAINS, TIMES, WEATHER_KINDS } from './recipes/vista.js';\nexport { toSpans } from './ansi.js';\nexport const VERSION = '${version}';\n`);
console.log(`art-sprites ${version}: ${seen.size} modules`);
JS

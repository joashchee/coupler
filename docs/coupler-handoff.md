# Handoff: Coupler's painter to the Painter engine (`art-sprites`)

**Who reads this:** a Claude Code session working in the Coupler repo, with
the user. **Who wrote it:** the Painter session (the `painter` repo,
npm name `art-sprites`), on 2026-10-29, at the end of Phase 7's first step.
The Painter session cannot edit Coupler and Coupler's session cannot edit
Painter, so this file lists what must be produced and decided on Coupler's
side. Everything below is either **(A) done in Painter already**, **(B) for
Coupler to produce as data**, or **(C) for Coupler to decide or change**.

Painter's rules that matter here: same recipe, params and seed gives the same
cells; no dependencies; the main entry has no Node built-ins (it runs in the
webview, a worker, Node or Deno); no network, no AI at draw time; DB32 is the
pixel palette and the ANSI colours are VGA indexes; the font is IBM VGA 8x16.
Pictures are **not** byte-identical to `painter.rs` and are not meant to be.

---

## 1. What Painter already does (A)

Package: `art-sprites` (this repo, folder `painter`). Entry `art-sprites`
exports `draw`, `Cells`, `writeAnsi`, `readAnsi`, `toSpans`, `tones`,
`resolveTones`, `roomSeed`, `TERRAINS`, `hasSky`, `lintAnsi` and more; types
are in `index.d.ts`. The CLI is `node cli.js` (`sprites`).

### The room picture: recipe `vista` (ANSI only)

```js
import { draw, roomSeed, toSpans } from 'art-sprites';
const art = draw('vista', {
  terrain: 'woods',            // one of CoffeeMUD's 24 (see below), lower case
  time: 'dusk',                // dawn | day | dusk | night
  weather: 'rain',             // clear cloudy windy rain thunderstorm snow blizzard hail sleet fog dust heat drought cold
  style: 'fantasy',            // fantasy | high
  arc: 640,                    // sun/moon position, thousandths across the sky (Scene.arc); optional
  colors: 16,                  // 16 | 256
  seed: roomSeed(world, roomId, look),   // decimal string; Coupler's FNV-1a
  columns: 80, rows: 24,       // clamped to 8..160 by 4..45
}, { style: 'ansi' });
art.frames[0];                 // Cells (palette indexes, glyphs)
toSpans(art.frames[0]);        // { columns, rows, lines: [[{text, fg, bg, blink}]] } for AnsiPicture.tsx
art.says;                      // "Outdoors: woods, dusk, rain." for the words beside the picture
```

Terrains: outdoors `city woods rocky plains underwater air watersurface
jungle swamp desert hills mountains spaceport seaport`; indoors `stone wooden
cave magic in_underwater gap cavelakesurface metal innerseaport caveseaport`.
Indoors and underwater ignore time and weather (`hasSky(terrain)`).

### Mapping `paint::Scene` to the params (C decides, this is the proposal)

| `paint::Scene` | recipe param |
|---|---|
| `terrain` | `terrain` (already lower case) |
| `time` (`None` means day) | `time` |
| `weather` (`None` means `clear`) | `weather` (`heat`/`drought` are accepted) |
| `arc` | `arc` |
| `world`, `room`, `look` | `seed: roomSeed(world, room, look)` |
| `Style` | `style: 'fantasy' \| 'high'` |
| `Colors` | `colors: 16 \| 256` (`True` reads `tones(cells)` as hex) |
| `name`, `zone`, `art` | **never passed** (paint.rs's rule: only the game's own words about the room) |

### Other things Coupler's `create.rs` and viewer can use now

* `writeAnsi(cells, { target: 'ice' | 'modern' | 'classic', colors: 16 | 256, sauce })`: `.ans` bytes with SAUCE, CRLF, CP437 or UTF-8, warnings never silent.
* `readAnsi(bytes)`: the inverse (Coupler's `ansi_art.rs` role).
* `resolveTones(cells, { depth: 256 })` and `tones(cells)`: the in-between colours for 256 and true colour (Coupler's `art_in`).
* `lintAnsi`, `lintAnsiFrames` (flash rule: no more than three flashes a second, as Coupler holds).
* `writeAnsimation`/`readAnsimation`, `light`, `gradient`, `bayer`, `fromImage` (study only).
* Styles `pixel`, `ansi`, `text` (7-bit ASCII for plain terminals).

---

## 2. What Coupler must produce (B)

Painter has **no recipe yet** for portraits, faces, item pictures, the figure
layer or the Fight panel, because they read data only Coupler has. To port
them, produce the following into a folder `painter-data/` (suggested; any
folder works) and give it to the Painter session. Write a Rust example or
script (Coupler's `src-tauri/examples/` and `scripts/painter-tables.py` are
the natural homes), not hand-edited files, so the data regenerates.

All files are JSON (UTF-8, stable key order, no player data, no names of real
players, nothing a player typed). Record the CoffeeMUD snapshot and date each
was made from in a `"source"` field.

### B1. Reference pictures (for judging by eye; highest value, cheapest)

`painter-data/reference/` holds the Rust painter's own output for a fixed
matrix, as `.ans` (CP437 or UTF-8, say which in the name) **and** a PNG render,
so someone can put them beside Painter's. Matrix:

* every one of the 24 terrains x {day clear, night thunderstorm, dusk snow} x {fantasy, high} at 80 by 24, and a few at 25 by 12, 8 by 4 and 160 by 45;
* `arc` 0, 500, 1000 for `plains` day and night;
* the same scene at `Colors` 16, 256 and true;
* each of `paint::seed("standard", "Midgaard#3001", 0)` and a second room, with the seed value printed in a `seeds.json` so the FNV-1a can be cross-checked (Painter's `roomSeed` returns it as a decimal string; the u64 must match exactly).

### B2. Portraits (`portrait.rs`)

* `races.json`: for every race in `SIZES`: `{ name, inches, pounds, darkvision }` plus its `Build` from `portrait::race` (skin, hair or none, clothes colour as VGA indexes, `Feature` and its colour: `PointedEars`, `BeardAndHelmet`, `TallHat`, `BareFeet`, `Tusks`, `Wings(colour)`, none) and the fallback rule for a race the painter does not know ("made from its name, the same each time": give the exact hash and how it picks colours).
* `classes.json`: every CoffeeMUD class with its kind (armor and sword, robe and glowing staff, hood and dagger, holy symbol, leaves, lute, bare-handed monk, craftsman's hammer, ...) and its colours, plus the unknown-class fallback rule.
* The measuring stick's rule (marks every foot up to six) and the portrait sizes (`LEAST`, `MOST`).

### B3. Faces (`face.rs`, 3080 lines)

* `shapes.json`: the head shapes by race category (`races.tsv` is the category table; also export the category to shape rule: person, canine, feline, ursine, rodent, equine, bovine, bird, reptile, fish, insect, spider, skull, elemental, golem, slime, plant, ...), with the numbers each shape uses (oval radii, ear, muzzle, beak, eye positions as `Anchors`).
* `words.json`: the colour-word table (`color_word`), the item colour table (`item_color`), the stop-words list (`words`, `key`), and what description words add (beard, scar, eye patch, freckles, horns, tusks, fangs, glowing eyes, halo, old age).
* `moods.json`: for each `Mood` (calm, angry, in pain, afraid, asleep, dead) the eye, brow and mouth strokes per shape, and the evil-calm sneer.
* `hurt.json`: the five hurt steps and how each material class (flesh, fur, stone and bone, ghost and elemental, slime) shows them; the CoffeeMUD `healthText` mapping to 0..5.
* `effects.json`: `effect_looks.tsv` (already in `src-tauri/src/`, 2.7 KB) as JSON, plus what each look (`halo`, `poison`, `burning`, `frost`, `blind`, `half-seen`, `outline`, `web`, `stone`, `bark`, `glow`) draws.
* `small-face.json`: the 16 by 8 face's own strokes (never the big face scaled).
* `sitters.json`: 20 to 40 real `Sitter` examples from the Bestiary (name, race, category, sex, description, body, wears, terrain, alignment, skilled), chosen to cover every shape, as fixtures. **Strip anything a player wrote** (descriptions are the game's own).

### B4. Item pictures (`item_art.rs`, 1938 lines)

* `kinds.json`: the first-word to `Kind` table (sword, dagger, axe, mace, hammer, spear, staff, wand, bow, arrows, shield, helmet, crown, hood, hat, armor, robe, legs, boots, gloves, cloak, belt, ring, amulet, bracelet, potion, scroll, book, bag, chest, barrel, cup, food, coins, gem, key, torch, lantern, candle, instruments, tool, rock, bundle) and each kind's default colours and slot text (`slot_of`).
* `materials.json`: `materials.tsv` (3.3 KB, material to class) as JSON and the texture each class draws (cloth woven, leather stitched, wood grained, metal with a band of light, mithril and gems sparkling, glass see-through, stone speckled).
* `shape-numbers.json`: the shape parameters per kind and per name word (scimitar curved, rapier thin, claymore long and broad, ...), and how the name's seed fills the rest.
* `wear.json`: the wear words (worn, damaged, battered, broken, rusty, crude, fine) and what each draws; the aura words (glowing and its colours, magical, metallic or mithril, invisible, hidden, floating or flying, bound).
* `things.json`: 30 to 50 real `Thing` examples (name, material, description, container, auras, worn, terrain) covering every kind, as fixtures.

### B5. Figure layer and the Fight panel (`figure.rs`, `fight_art.rs`)

* `figure.json`: the `Part` classes (Back, Body, Hair, Eye, Mouth, Worn, Brow, Detail), the `lay` rule (offset, hide), the rim `light`/`lighter`/`shade` colour pairs (`lighter(c)` and `shade(c)` as a 16-entry table each), and `wild(terrain, key)`'s rule for an animal's place.
* `fight.json`: the layout (columns, face size, the strip's 8 rows, mirroring), the `Frame` list the Rust code builds for a big blow (jolt shift, flash) and a new foe (slide in), and the frontend's limits that must still hold (a flash at most once a blow, never more than three a second, no jolt or slide with reduced motion).

### B6. Painter internals worth a data dump

`painter.rs` has tables Painter currently guesses at. Export, if it is easy:
the per-terrain horizon shares (45 percent, mountains 60, watersurface and
seaport 50, air all), the gradient stops per time of day and sky kind, the
`dim` function's 16-entry mapping, the `Bayer` thresholds (Painter uses the
same 4 by 4), the `Material` table for indoor rooms (wall, joint, side, floor,
floor_joint, ceiling, laid), and the weather rules (which glyphs, densities,
which kinds are overcast).

---

## 3. What Coupler decides and changes (C)

1. **How Coupler runs the engine.** Painter is browser-safe JavaScript. Options: (a) call it in the webview with Rust passing the scene, (b) a worker, (c) embed a JS runtime in Rust. Constraint from Coupler: the web build and the phone must draw the same pictures. Painter's recommendation: (a), since it already runs in the webview and `toSpans` matches `AnsiPicture.tsx`'s shape.
2. **Rule edits in Coupler's own docs** (never from Painter): rule 11 ("all ART is Coupler's painter's first") becomes "all ART is the Painter engine's first"; rule 6 (pure Rust, unit-tested) needs a carve-out or a Rust-side check that the scene reaching the engine contains only the game's own words; rule 12 (sound and pictures never break) needs a fallback when the engine throws (draw nothing and say so in words, never block sound).
3. **Delete or keep `painter.rs`** and the other six files once parity is accepted; until then keep them as the reference.
4. **Accessibility:** a picture's facts stay in words beside it. `art.says` is the engine's words; Coupler's UI shows them.
5. **Storage:** `create.rs` keeps three files per picture (16, 256, true). The engine writes each with `writeAnsi(..., { colors })`; true colour reads `tones(cells)` hex.
6. **Parity sign-off:** someone looks at `reference/` beside Painter's output (`sprites sheet vista --style ansi --out sheet.png`, or one terrain at a time with `sprites draw vista terrain=woods time=dusk --style ansi --out a.png`) and accepts or lists differences.
7. **Which gaps matter first:** faces are the biggest (3080 lines); items next; portraits are small. Say the order.

---

## 4. Checklist for the Coupler session

- [ ] Build the matrix in B1 and save the references.
- [ ] Write the exporter for B2 to B6 (an example or `scripts/` Python, regenerable), run it, check no player-typed text is in any file.
- [ ] Hand `painter-data/` to the Painter session (copy it into `painter/docs/coupler-data/` or give the path).
- [ ] Decide C1 and C7.
- [ ] Wire `vista` into the picture path behind a flag, compare, then switch.
- [ ] Make the rule and doc edits (C2) in Coupler's repo.

## 5. How Painter will use the data

Each gap becomes a recipe taking **data, not Coupler types** (rule: the engine
never imports Coupler), registered with `variants()` and a kit row, drawn on
`Cells` in the artists' order (`paint`: focal, outline, base, highlights,
shadows, blend), linted (`lintAnsi`), and covered by golden hashes. The
`coupler-painter` kit (`node cli.js kits`) is the live migration list; its
five gap rows are exactly B2 to B5.

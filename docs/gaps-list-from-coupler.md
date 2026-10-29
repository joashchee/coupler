# Gaps list: what Painter must research and draw next

**For:** the Painter session (`art-sprites`, the `painter` repo).
**From:** the Coupler session, 2026-10-29, after drawing the B1 reference matrix
with the engine (`art-sprites` 1.3.0, painter commit `9150457`, recipe `vista`)
and reading `sprites kits`.
**Companion files:** `docs/coupler-handoff.md` (the first handoff: sections A to C),
and the data in `painter-data/` (below).

This file lists what the engine does not draw yet, or draws thinly, in the order
Coupler suggests (Coupler's decision C7 is still open, so the order is a proposal).
Each item says what Coupler's painter does (read for ideas, never copied), what the
engine does now, the evidence, the data Coupler has already given or still owes,
and how to tell it is done.

Posture, as in the handoff: pictures are **not** byte-identical to Coupler's and
are judged by eye; the engine imports nothing of Coupler's; each gap becomes a
recipe taking **data, not Coupler types**; the game's colours are VGA indexes and
the font IBM VGA 8x16.

---

## Status check, 2026-10-29, after `art-sprites` 1.7.0 (commit `eab9c0e`)

Coupler re-checked the claim that 1.7.0 "closed all gaps". **It closed all gaps in the
`coffeemud` kit** (26 subjects: creature bodies and races, class loadouts, item shapes,
props, exits, locales, status icons, spell effects, `bar`, `mappane`, `sheet`, `sigil`).
That is a different kit from `coupler-painter`, which is the one that tracks replacing
Coupler's painter. Evidence, read at painter HEAD `eab9c0e`:

- `node cli.js kits`, kit `coupler-painter`: **1 covered, 9 partial, 1 built-in, 5 gap**.
  The five gaps are still `portraits`, `faces`, `item-pictures`, `figure-on-place` and
  `fight-panel`. Eight kit gaps are listed overall.
- The 1.7.0 changelog says of `vista`: "unchanged (its 24 terrains are Coupler's list)".
  Every `vista` observation in Priority 2 below still stands.
- No recipe takes Coupler's inputs: a race's inches and bulk (portrait), a mood or a hurt
  step (face), a material class or a wear word (item), part-tagged pixels laid on a place
  (figure), or a blow's frames with a reduced-motion switch (fight). The nearest recipes
  (`bust`, `creature` with `race`, `item`, `character`) are generic and do not read
  `painter-data/`.
- Looked at by eye, `city_night-thunderstorm_fantasy_80x24_c16` (engine, `painter-data/engine/`)
  against the same scene in Coupler's painter (`painter-data/reference/`): the engine's is a
  flat grey slab with a black lower half and a few lit windows; Coupler's has dithered
  paving, lit buildings, a road, clouds and lightning across the whole picture.
- Lint on `vista#0` still reports `invisible` cells (a glyph in its own background colour)
  on 74 of 260 pictures.

**Verdict: the engine cannot replace Coupler's painter yet.** Replacing it means all of
G1 to G5 below, plus G2's fidelity, reaching parity by eye. Until then Coupler keeps
`painter.rs`, `portrait.rs`, `face.rs`, `item_art.rs`, `figure.rs` and `fight_art.rs`
(about 8,400 lines) and does not wire the engine in as the default.

**Why parity matters to Coupler (so Painter knows the bar).**
- Coupler's rule 12: sound and pictures never break. A change that touches pictures is
  checked against every picture it could reach, so Coupler will not swap a picture for
  one that shows less. The test is "as informative as Coupler's", not "the same".
- Every picture has its facts in words beside it (`says`), kept as now.
- The same pictures must draw on the desktop, the web build (including Safari on the
  iPhone) and the phone. Painter runs in the webview, so Coupler's plan is the
  recommended route (a): Rust passes the scene, the webview draws. No Node built-ins,
  no network, no AI at draw time, as Painter already holds.
- The phone and the Fight strip need the **16 by 8 face drawn on its own strokes**, never
  the big face scaled (G3), and the Fight panel's reduced-motion switch (G5).

**Order Coupler asks for (decision C7, settled here).**
1. **G2, the room picture's fidelity.** It is the one recipe Coupler could switch to
   today, and it is the picture seen most. Coupler will wire `vista` in behind a flag and
   compare once G2's "done when" is met; not before.
2. **G3, faces** (the biggest, and the Fight and the Bestiary need them), with the small
   face in the same piece.
3. **G4, item pictures.**
4. **G1, portraits** (small, data ready; can be done in parallel with any of the above).
5. **G5, figure on a place and the Fight panel** (needs G2 and G3).

**Decided by the maintainer, 2026-10-29:** the order above stands (C7), and the engine
runs **in the webview** (C1), so the web build and the phone draw the same pictures.

**The handover bundle** (`coupler-handover.zip`, unzip it into `painter/docs/coupler-data/`):
- `docs/`: this file and `coupler-handoff.md`.
- `painter-data/reference/`: **Coupler's own Rust painter's** output, 260 pictures as
  `.ans` and `.png`, with `manifest.json` and `seeds.json`. This is the target to match
  by eye. (Section 0's note that the Rust set is "not to be used" meant not for the B1
  matrix of engine pictures; for judging parity it is exactly what to look at.)
- `painter-data/engine/`: **the engine's** output for the same matrix at 1.3.0, with its
  manifest and kits report. Compare it beside `reference/` by file name; the names match.
- `painter-data/*.json`: B2 to B6 data (races, classes, portrait, shapes, words, moods,
  hurt, effects, small-face, sitters, faces, kinds, materials, shape-numbers, wear,
  things, figure, fight, painter-tables).

**What Coupler wants confirmed when Painter hands back.** For each gap: the kit row
moved from `gap` or `partial` to `covered`; `lintAnsi` clean on the matrix; any changed
existing output named in `CHANGELOG.md` with the file names; and a fresh contact sheet
beside Coupler's reference.

---

## 0. Data Coupler has given, and what it still owes

| Handoff | Content | Status | Where |
|---|---|---|---|
| B1 | Reference pictures | **Done, drawn by the engine** (260 `vista` pictures, `.ans` UTF-8 and CP437, PNGs, manifest, kits report, contact sheet). The earlier set drawn by Coupler's Rust painter is not to be used. | `painter-data/engine/` |
| B2 | Races, classes, portrait rules | **Done** | `painter-data/races.json`, `classes.json`, `portrait.json` |
| B6 | The Rust painter's tables | **Done** (VGA values, Bayer, RNG with test vectors, horizon shares, sky stops, `dim`, `recede`, weather marks, indoor materials) | `painter-data/painter-tables.json` |
| B3 | Faces | **Done** (`shapes`, `words`, `moods`, `hurt`, `effects`, `small-face`, `sitters`, plus `faces.json`: every fixture drawn full and small) | `painter-data/` |
| B4 | Item pictures | **Done** (`kinds`, `materials`, `shape-numbers`, `wear`, `things`, with finished pictures) | `painter-data/` |
| B5 | Figure layer and the Fight panel | **Done** (`figure.json`, `fight.json`) | `painter-data/` |

`painter-data/` is gitignored and made by `scripts/painter-data.sh`; ask Coupler to
rerun it if a file is missing. All of B1 to B6 now exist. The fixtures in B3 and B4
are **synthetic** (written by the exporters from the race table and kind table, one
per shape or kind plus described variants), not real Bestiary entries, so no game text
or player data is in them. Each file also holds drawn grids (colour and part rows, or
`char,fg,bg` cells) as ground truth to match by eye.

---

## Priority 1: portraits (G1). Data is ready; smallest piece

**What Coupler draws** (`portrait.rs`, 617 lines): a *race* standing beside a
measuring stick marked every foot up to six, so its size shows against the others
(height from the game's Java, bulk from weight per inch, skin, hair or none,
clothes, a feature: pointed ears, beard and helmet, tall hat, bare feet, tusks,
wings; red glowing eyes where it sees in the dark), and a *class*: a person in its
clothes with its tools (armor, sword and shield; robe, pointed hat, glowing staff;
hood and dagger; holy symbol and mace; leaves; lute; bare-handed monk; craftsman's
hammer). Sizes 8 by 4 up to 80 by 40 cells, two square pixels a cell.

**What the engine does now:** nothing for portraits. `bust` (a front-facing bust,
no expressions or angles) and `characters` are the nearest, and neither stands on
ground beside a stick.

**Needed:**
- A recipe taking `{ kind: 'race' | 'class', name, inches, bulk, skin, hair, clothes, feature, darkvision }` or `{ ..., classKind, clothes, trim }`, drawing from `races.json` and `classes.json`. `portrait.json` gives the layout rules (ground row, foot size, stick, `cx`, how the figure is built).
- The two fallbacks: a race or class the painter does not know is made from its name (`fallback` in both files, with the exact FNV-1a seed, the xorshift64* RNG and the order of draws; check against the `rngFirstSix…` vectors).
- Words beside the picture (`says`): "Elf, 5 feet 5, pointed ears, sees in the dark."

**Done when:** all 186 races and 49 classes draw at 32 by 16 and at 80 by 40 with
no lint errors; the Half Ogre stands taller than the Human, the Human taller than
the Gnome, the Gnome taller than the Pixie (Coupler's own test); an unknown name
draws the same every time; a race that sees in the dark shows red eyes and one
that does not does not.

---

## Priority 2: the room picture's fidelity (G2). `vista` exists; it is thin

The kit rows `terrains-outdoor`, `terrains-indoor`, `time-of-day`, `weather`,
`high-fantasy` and `colour-depths` are all **partial**. Looking at the 260 pictures
in `painter-data/engine/` (and `vista-sheet.png`), these are the gaps that show.
They are listed as observations; where Coupler's painter does something the engine
could learn from, the source is named.

**Outdoors**
- **The ground below the horizon is one flat band, usually grey, ending in a black strip.** On `city`, `woods`, `rocky`, `plains`, `jungle`, `hills`, `swamp` and `spaceport` the lower third is a grey slab, not terrain: no grass, paving, sand, mud or rock texture, no perspective. Coupler's painter dithers and detail-fills the whole lower picture and takes the road in perspective (a wedge that widens toward the viewer).
- **`city` at night is a black lower half.** In `city_night-thunderstorm` (fantasy and high) the picture below the skyline is solid black and 560 cells hold a character in the colour of their own background (lint `invisible`). Coupler draws lit windows, streets and rain over the whole picture.
- **`jungle` reads as `woods`** (no canopy), **`swamp`** has no water, reeds or mist, **`air`** is sky only (no clouds below, no sense of height), **`underwater`** is a blue gradient and a black floor (no sea floor, weed, bubbles or light shafts), **`seaport`** has one jetty and no ship detail, **`spaceport`** is a sketch (no pads, ships or lights).
- **`mountains`** are one ridge in a single grey; Coupler's are layered ranges with snow, shaded by depth. The `biomes-environments` kit's `mountain-cave-dungeon` and `desert-ruins` rows are partial for the same reason.
- **The sky is striped.** Every sky is bands of dithered cyan/blue with a row of a vertical-hatch glyph at each band's join; at 80 by 24 this reads as scan lines. Coupler's bands are solid and dither only where one meets the next, with smooth gradients between colours at 256 and true colour.
- **Weather the kit already admits is missing:** `windy`, `heat`, `drought` and `cold` change nothing visible. Rain, snow, hail, sleet, dust, fog, a storm's lightning and a blizzard draw; check that snow lies on the land and not on water or stone as Coupler's does (`weather_marks` and the snow rule are in `painter-tables.json`).
- **No hour-by-hour sky.** Night is lit down with `light` and stars; Coupler's night is the day drawn in its night colours (`dim` table in `painter-tables.json`), with the moon on the same arc as the sun.

**Indoors**
- **Every wall is planks.** `stone`, `metal`, `innerseaport`, `caveseaport`, `wooden`, `magic` and `in_underwater` all draw the same vertical-stripe back wall. Coupler's rooms are laid per material: bricks (stone, with mortar joints), planks (wooden), panels (metal), runes (magic), with their own wall, joint, side, floor and ceiling colours and a floor of laid stones or boards. The table is `materials` in `painter-tables.json`. The kit row says "no per-material floors".
- **`innerseaport`** should show a channel of water across the floor and a jetty; **`in_underwater`** bubbles; **`magic`** a glowing circle on the floor. None draws.
- **Caves** (`cave`, `cavelakesurface`, `caveseaport`) and **`gap`** (a chasm) draw stalactites but look alike; the lake and the quay do not show.
- **Light:** torches are two cells. Coupler adds a flame, a glow on the walls and, in high fantasy, banners and braziers.

**High fantasy** has one castle and one aurora band (kit: partial). Coupler's adds a
second small red moon on its own arc, banners and braziers indoors and castle
towers on every landscape except `air`, `city` and `spaceport`.

**Colour depths:** 256 and true colour draw as one picture; true colour is the 256
drawing written with `colors: 'true'`. Coupler's `art_in` smooths gradients and
dithers into the colours in between; confirm the engine's 256 version really uses
colours between the 16 and not only the 16 again.

**Lint findings from the B1 run** (`manifest.json`, `lint`): 106 of 260 pictures
have findings, all warnings.
- `invisible`: 74 pictures, up to 560 cells. Real: a glyph drawn in the colour of its own background wastes cells and makes the file lie about its contents. Worst on `city` night, `woods` day (357) and mountains.
- `shade-noise`: 4 pictures, lone shade glyphs among solid cells.
- `width` and `height`: 16 each, only the 160 by 45 pictures. **Not an engine fault**: the run linted with the classic target's 80 by 24 limit.
- `bg-limit`: 30, only the 256 and true pictures. **Not an engine fault**: they were linted as a classic file (8 backgrounds); a modern file has 16 or more.
So only `invisible` and `shade-noise` are findings to fix.

**Done when:** each terrain looks like its terrain at 80 by 24 and 160 by 45; a
stone, a wooden, a metal and a magic room are told apart at a glance without
reading their names; a night city and a night plain are lit, not black; the sky
is solid bands; the four missing weathers show; `invisible` and `shade-noise`
fall to zero on the B1 matrix; the contact sheet (`sprites sheet vista --style ansi`)
is accepted by eye beside Coupler's picture of the same scene (Coupler will render
a fresh Rust reference on request, from `painter_reference`).

---

## Priority 3: faces (G3). The biggest gap; B3 data is ready

**What Coupler draws** (`face.rs`, 3080 lines): a creature's face, 25 by 24 pixels,
and a small 16 by 8 face that is its own drawing (never the big face scaled).
- Head shape by race category: person, canine, feline, ursine, rodent, equine, bovine, bird, reptile, fish, insect, spider, skull, elemental, golem, slime, plant and more (`races.tsv` is the category table).
- Colours from the description's words (the colour-word table), and what a description adds: beard, scar, eye patch, freckles, horns, tusks, fangs, glowing eyes, halo, old age.
- **Six moods** (calm, angry, in pain, afraid, asleep, dead), each an eye, brow and mouth stroke per shape, and an evil-calm sneer.
- **Five hurt steps** from the game's health text, shown differently on flesh, fur, stone and bone, ghost and elemental, and slime.
- **Effects** on the face (`effect_looks.tsv`): halo, poison, burning, frost, blind, half-seen, outline, web, stone, bark, glow.
- The face stands in its area's terrain (the room painter's picture, a shade back).

**What the engine does now:** `faces`, `creatures`, `bust` exist for pixel art and
a front-facing ANSI bust; the `faceart` kit rows are partial (four skin ramps, no
nose or eye-shape variation, no profile or back view, no 8 by 8 face). Nothing
reads a race category, a mood or a hurt step.

**From Coupler (B3, done):** `shapes.json`, `words.json`, `moods.json`,
`hurt.json`, `effects.json`, `small-face.json`, `sitters.json` (33 synthetic sitters, one per shape plus variants).
**Needed from Painter:** one `face` recipe over those; a `small` size drawn from
its own strokes; moods and hurt as params; the effects as overlays.

**Done when:** the 33 synthetic fixtures each draw at 25 by 24 and 16 by 8 with every
mood and hurt step; a hurt flesh face and a hurt stone face are told apart;
`lintAnsi` passes; the flash rule (no more than three flashes a second) holds for
any animated effect.

---

## Priority 4: item pictures (G4). B4 data is ready

**What Coupler draws** (`item_art.rs`, 1938 lines): an item alone, 25 by 24, from
its name, material, wear and auras. The kind comes from the first word of the name
(sword, dagger, axe, mace, hammer, spear, staff, wand, bow, arrows, shield, helmet,
crown, hood, hat, armor, robe, legs, boots, gloves, cloak, belt, ring, amulet,
bracelet, potion, scroll, book, bag, chest, barrel, cup, food, coins, gem, key,
torch, lantern, candle, instruments, tool, rock, bundle); shape numbers per kind
and per name word (scimitar curved, rapier thin, claymore long and broad); the
material's texture (cloth woven, leather stitched, wood grained, metal with a band
of light, mithril and gems sparkling, glass see-through, stone speckled); wear
(worn, damaged, battered, broken, rusty, crude, fine); auras (glowing in its colours,
magical, metallic, invisible, hidden, floating, bound); a slot in words.

**What the engine does now:** `items` and `icons` recipes for pixel art; the
`items-equipment` kit is partial (no spear or polearm, no gauntlets or greaves, no
jars or herbs, no maps or relics, torch only for camping, no wands or runes).
Nothing reads a material or a wear word.

**From Coupler (B4, done):** `kinds.json`, `materials.json` (from
`materials.tsv`), `shape-numbers.json`, `wear.json`, `things.json` (one synthetic thing per kind plus variants).
**Needed from Painter:** the missing kinds named above (spear and polearms first:
Coupler draws them); an `item` recipe taking name, material class, wear and auras;
texture by material class; aura overlays.

**Done when:** every kind draws at 25 by 24 in each material class and wear step;
a rusty iron sword, a fine mithril sword and a glowing wooden staff are told apart
by eye; an unknown first word falls back to a bundle as Coupler's does.

---

## Priority 5: the figure on a place, and the Fight panel (G5). Needs G2 and G3; B5 data is ready

**What Coupler draws** (`figure.rs` 198 lines, `fight_art.rs` 334 lines): a face or
item drawn on a clear canvas, each pixel tagged with its part (back, body, hair,
eye, mouth, worn, brow, detail), laid on its place (the room, a shade back) and
lit by it with a rim light (`light`, `lighter`, `shade` colour pairs). The Fight
panel is the room with two faces in it, the player mirrored; frames for a big blow
(a jolt shift and a flash) and a new foe (a slide in); on the phone a strip of eight
rows with its own 16 by 8 faces and the words between them.

**What the engine does now:** `light` and `overlay` exist; the layering of a figure
over a vista is not written (kit row `figure-on-place`); no Fight recipe (`fight-panel`).
Clips and `lintAnsiFrames` (the flash rule) exist.

**From Coupler (B5, done):** `figure.json` (parts, the `lay` rule, the
`lighter`/`shade` tables, how an animal's place is chosen) and `fight.json`
(layout, frames, the frontend's limits).
**Needed from Painter:** the part-tagged layer; a `fight` recipe with the frames as
clips (jolt, flash at most once a blow and never more than three a second, slide in)
and **no jolt or slide when the player has reduced motion** (a param, since Coupler
cannot post-filter a clip).

**Done when:** a face laid on `woods` dusk and on `stone` shows the place through
its rim light; the Fight's clips pass `lintAnsiFrames`; the reduced-motion version
draws a still.

---

## Priority 6: the kit rows that are not Coupler's (G6)

Painter's own kits list more gaps; they are not Coupler's to prioritise, but the
vista and the faces lean on some of them. From `sprites kits` (2026-10-29):

- **Gaps (nothing draws these):** `pose-foreshortening`, `turn-three-quarter`, `motion-robes` (hands, poses and motion kit).
- **Partial that help Coupler:** `village-castle` (one house; no walls, gates, markets or castle: wanted by `city` and high fantasy), `coast-river-swamp` (no docks, boats or bridges: `seaport`, `swamp`), `desert-ruins`, `mountain-cave-dungeon`, `magical` (no floating islands or portals), `readability` (no foreground, midground and background layers: the sky and the flat ground in G2 are this), `interior-architecture` (no stairs, ceilings or beams: the room's one-point perspective), `shaded-landscape` (one composition), `ansimation` (falling weather only: a storm or a fire would help the Fight and rooms).
- Partial kit rows about faces: `faces-small` (no 8 by 8), `range-angle`, `range-phenotype`, `historical-monsters` (dragon and bat only; no demon, chimera or harpy body plan: Coupler's face shapes cover many more).

---

## What Coupler will do meanwhile

- Keep `painter-data/` regenerable (`scripts/painter-data.sh`; B1 to B6 are all there now) and rerun it on request.
- Keep Coupler's own painter as the reference and the default until each gap's "done when" is met and Coupler has judged it by eye. Then wire the engine in behind a flag, compare, switch, and only then retire the matching Rust file.
- C1 is settled as the webview (Painter's recommendation) and C7 as the order in the status check above. The rule edits in Coupler's own docs (rules 6, 11 and 12) wait until a gap reaches parity. None of those is Painter's.
- Render a fresh Rust reference of any scene Painter asks for to compare by eye (`cargo run --release --example painter_reference`), without making it part of the engine's tests.
- Judge parity by eye on the contact sheets; a picture is accepted when it is as informative as Coupler's, not when it is the same.

## How to check your work

- `node cli.js kits` should move rows from `gap` to `partial` or `covered`; add the row to the kit when a recipe lands.
- `node cli.js sheet <recipe> --style ansi --out s.png` for a contact sheet; `node cli.js draw vista terrain=woods time=dusk --style ansi --out a.png` for one.
- Re-run `scripts/painter-engine-reference.mjs` in Coupler for the 260-picture matrix and its manifest; the manifest's `lint` and `says` fields are what to read.
- **Rule 12 (Coupler's):** a change touching pictures must be checked against every picture it could reach. A recipe change that alters existing output says so in `CHANGELOG.md`, with the manifest's file names that changed.

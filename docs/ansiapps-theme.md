# The ANSIapps theme (every ansiapps app)

**Convention, decided 2026-09-26.** Every ansiapps app (Diskette,
Crunchy, Floppy, Stylus, and whatever comes next) ships two UI themes:

1. **Modern**: the default (except in Stylus, below), the dark
   token-based design the apps share today (IBM Plex, `--bg`/`--panel`/`--line`/`--text`/`--muted`/
   `--accent`).
2. **ANSIapps**: an alternative, old-school DOS text-mode look. It is
   switched from the **gear menu** with a checkbox, "ANSIapps theme
   (old-school DOS look)", remembered across launches, and applied
   before first paint so it never flashes the modern theme.

**Stylus flips the default** (decided 2026-09-29): it opens in the
ANSIapps theme, with modern as the option, because it's the theme's own
tool. Its no-flash script and splash assume ANSIapps when nothing is
stored. Every other app keeps modern as the default.

Reference for the look: int10h.org's
[Ultimate Oldschool PC Font Pack font list](https://int10h.org/oldschool-pc-fonts/fontlist/),
Norton Commander's blue file panels, and Turbo Vision's gray dialogs.

Diskette is the reference implementation of the theme's **code** (CSS,
toggle, splash). Copy its structure into each new app rather than
reinventing it.

**Stylus is the theme's authoring tool** (decided 2026-09-29,
`docs/stylus-notes.md`). The ANSIapps look is what sets the family
apart, and Stylus is its second hero app, after Diskette: open source,
always free, built on icy_tools. Text mode doesn't mean no graphics.
The theme's graphics (the app mark, splash art, icons, empty states,
spinners) and the drawn parts of its controls (button faces, dialog and
panel frames, title bars, checkboxes, scroll bars, the progress fill)
are **ANSI art and animation**, drawn and touched up in Stylus and
shipped to every app as a shared theme pack (see "ANSI art assets"
below).

## How it's built (Diskette's implementation)

| Piece | Where | What |
|---|---|---|
| Theme state | `src/lib/theme.ts` | `Theme = "modern" \| "ansiapps"`. `data-theme="ansiapps"` on `<html>` when on, stored in `localStorage` under `<app>.theme` |
| No-flash apply | `index.html` inline `<script>` | Reads the same key before the bundle loads, and the inline splash styles have an ansiapps background |
| Styles | `src/ansiapps-theme.css` | Everything scoped to `:root[data-theme="ansiapps"]`, imported after the modern CSS. The modern stylesheets stay untouched |
| Toggle | Gear menu checkbox (`data-testid="ansiapps-theme-toggle"`) | Next to the app's other preferences |
| Font + license | `public/fonts/ansiapps/` | The `.woff`, the pack's `LICENSE.TXT`, and a `README.txt` crediting it |
| Credit | About dialog, "Credits" section | Names the font, VileR, int10h.org, and CC BY-SA 4.0 |

The theme is overwhelmingly **token overrides**. The modern CSS already
routes every color through `--bg`, `--panel`, `--text` and so on, so the
ANSIapps stylesheet re-points them and then adds the structural changes
below. Re-pointing tokens inside a container (menus, dialogs, a
selected row) recolors everything in it without touching components.
Keep new modern UI on the tokens and the ANSIapps theme mostly follows
for free.

## Design rules

- **Palette**: only the 16 CGA/VGA text-mode colors, defined as
  `--dos-*` tokens: black `#000000`, blue `#0000AA`, green `#00AA00`,
  cyan `#00AAAA`, red `#AA0000`, magenta `#AA00AA`, brown `#AA5500`,
  light gray `#AAAAAA`, dark gray `#555555`, light blue `#5555FF`,
  light green `#55FF55`, light cyan `#55FFFF`, light red `#FF5555`,
  light magenta `#FF55FF`, yellow `#FFFF55`, white `#FFFFFF`. No other
  colors, no gradients except a solid-block progress fill, no
  transparency except modal dimming.
- **Contrast**: every text/background pair comes from the ranked list
  in **`docs/ansiapps-color-contrast.md`** (4.5:1 or better, WCAG AA;
  the font's 16px with no bold, so there's no large-text allowance).
  Only 32 of the palette's pairs pass. Light red on blue and red on
  light gray don't, for example. A colored mark that can land on several
  row colors (selected, busy, tinted) gets its own black character cell.
- **Status marks are characters**: a dot, light or flag in the ANSIapps
  theme is a glyph from the VGA font (`•`, `○`, `■`, all in CP437), not
  a CSS circle. Diskette's Catalog location dot is the example.
- **Surfaces**: the "desktop" and panels are blue with light-cyan text
  frames. Menus and dialogs are light-gray windows with black text, in
  the Turbo Vision style.
- **Type**: one bitmap font, **IBM VGA 8x16, at 16px only**, since a
  bitmap font is only crisp at its native size. There's no bold or
  italic (synthesized bold smears it), and font smoothing is off.
  Hierarchy comes from color and position, not size or weight.
- **The grid** (the rule, decided 2026-09-30, first built in Coupler):
  **every character sits in a cell of the 8 by 16 grid**, counted from
  the screen's top left, the way a text-mode screen works. No text is
  ever a few pixels off its cell. What that takes:
  - Every width, padding, margin and gap is a multiple of **8px across
    and 16px down**, and every line of text is **16px tall** (no looser
    leading anywhere).
  - **Nothing that holds text has a border**: a border pushes the text
    off by its own width. A frame is drawn by `::before` inside a ring
    of padding one cell wide and one row tall, so the line runs through
    the middle of its own cells, as `║` and `═` do.
  - **Buttons and text fields are one row tall**, with a cell of
    padding each side and no border. Stacked buttons get a blank row
    between them, which also keeps small targets apart.
  - **The browser never centers text**: centering lands it half a cell
    off whenever the space left over is odd. A centered title is placed
    by whole cells (`left: 50%` of an even number of cells, then back by
    half its width rounded to a cell with CSS `round()`). Button text
    stays at the left. A row of things lines up along its **top**, not
    its middle.
  - **Dialogs have a fixed place and width** in cells rather than being
    centered by their height.
  - **Scrolling rests on a whole row** (`lib/grid.ts`, `installRowSnap`).
    A frame never scrolls; a body inside it does.
  - **Every character shown is one the font has**, or it falls back to
    a font of another width and shifts the rest of its line. The font
    has arrows, box drawing, `…`, `×`, `·` and `•`, but **no Mac key
    symbols** (`⌘ ⇧ ⌃ ⌥`): write keys as words (`Cmd+Shift+L`).
  - Pictures and icons are whole cells too (16×16 is 2×1 cells).
  - **Check it**: Coupler's dev-only gear → **Check the Grid**
    (`checkGrid` in `lib/grid.ts`) measures everything showing and
    outlines what's off. Run it on each screen and dialog after any
    change to the theme's CSS.
  - An app whose window resizes must also keep the window's content to
    whole cells for this to hold; Coupler's screen is a fixed 160 by 45.
  - Not covered: text the app doesn't write (in Coupler, the game's
    output, if the game sends a character the font lacks).
- **Shapes**: square corners everywhere. Panels get a double-line frame
  (drawn in the panel's outer ring of cells) with the title set into
  the top edge, centered. Dialog titles sit in the frame's top edge the
  same way (`═══ Title ═══`). Tiles and boxes get single-line frames.
- **Depth**: hard black drop shadows with no blur, for **windows
  only** (menus, dialogs: 16px) and message bars (8px).
- **Buttons are just clickable** (convention, decided 2026-09-30, first
  built in Coupler; it replaces "a pressed button shifts into its
  shadow"). A button is a flat bar: no shadow, and it never moves when
  pressed. It shows its state by color alone among the listed pairs:
  white under the pointer, black on cyan while the mouse is down, dark
  gray text when disabled, the dashed ring when focused. A button that
  moves can lose the click (the hit box moves with it) and shifts what
  should have a fixed place. Port this to the sister apps, dropping
  Diskette's `::after` cover for the old press.
- **Selection**: list rows are borderless lines, and the selected one
  is a cyan bar with black text, as in a file panel. Menus highlight
  in green.
- **Controls**: checkboxes render as `[ ]` / `[X]`. Text inputs are
  black command-line strips. Focus is a dashed white outline.
- **No easing, but ANSI animation is welcome**: no fades, slides or
  easing, because a text-mode screen just redraws. Motion is
  **frame-by-frame redrawing of whole character cells**, the way an
  ANSImation plays: an indeterminate progress bar that steps, a spinner,
  an animated splash. Every animation follows the accessibility rules
  under "ANSI art assets" (no more than 3 flashes a second, a pause for
  anything longer than 5 seconds, a still frame under reduced motion).
- **App-specific colors keep their meaning.** Map each reserved color to
  its nearest text-mode equivalent rather than dropping it. Diskette:
  Handlers' gold becomes yellow on brown (still reserved for Handlers
  only), and the file-container tint becomes light magenta.
- **Graphics are ANSI art** (changed 2026-09-29; this replaces the old
  "keep the SVG icons" rule). Icons, the app mark, splash art and
  decorative frames come from the theme pack, drawn in Stylus on the
  VGA character grid. Until an icon has an ANSI version, the modern
  inline SVG (`currentColor`) stays as a stand-in. Replace it rather
  than restyle it.

## Shared UI conventions (every ansiapps app)

**Decided 2026-09-27.** Beyond the two themes, every ansiapps app shares
the behaviors below. Diskette is again the reference, and each
must look right in both themes.

### An in-development warning on first run

**Decided 2026-09-30.** On first run, before the window or its loading
screen exists, a native OS dialog says the app is still in development
and used at the user's own risk. Two buttons: **OK** opens the window
and starts loading (and is remembered, so it never asks again);
**I'll Be Back.** quits cleanly, having created nothing but what the OS
itself makes, and the warning shows again next launch.

- Diskette: `src-tauri/src/first_run.rs`. The main window is
  `"create": false` in `tauri.conf.json`, and `setup` builds it (and
  opens the catalog) only once the warning is accepted. The answer is a
  marker file in the app-data dir. The dialog is native, so it doesn't
  follow the ANSIapps theme; nothing of the app is drawn yet.
- Web trials can't remember anything, so a trial would ask on every
  visit (see `web-trial.md`).

### A loading screen from the first paint

The window is never blank or half-drawn at launch.

- `index.html` holds a static splash: app mark, name, an empty
  progress slot and an empty label. Its styles are inline in `<head>`
  so it paints before any JS or bundled CSS loads. The no-flash theme
  script runs first, and the inline styles include the ANSIapps
  background (`#0000AA`), so the ANSIapps splash never flashes the
  modern one.
- `components/StartupScreen.tsx` renders the same markup once React
  mounts. It adds a determinate progress bar over the launch-time
  calls (`STARTUP_STEPS` in `App.tsx`) and labels the step still
  running. Once every call has answered it fades out and unmounts; in
  the ANSIapps theme it just disappears.
- A failed step counts as finished, and its error shows in the app.
  New launch-time work (a migration, an index build) becomes a labeled
  step rather than running unseen.
- ANSIapps look: blue screen, white name, light-cyan accent, the 16px
  block progress bar, and a light-gray label (`ansiapps-theme.css`,
  "Launch screen"). Keep the splash markup and `StartupScreen` in step.

### Feedback for every user activity

Anything the user sets off shows at once that it's underway, and ends
with a visible result: a status message on success, an error on failure.

- Backend work goes through `runActivity` (`src/lib/activity.ts`), which
  shows a label and progress bar in the activity area
  (`components/ActivityStatus.tsx`) until the work settles. A row
  appears only after 150 ms, so instant work doesn't flash one.
- The bar fits the work. It's **determinate** when the work can be
  counted, which means the backend emits progress events for it.
  It's **indeterminate** only when nothing can be counted, and
  **timed** when the duration is known.
- The control the user acted on shows it's busy too (`isBusy(key)`).
  It's disabled, or its row is grayed. In the ANSIapps theme a busy row
  turns dark gray rather than transparent, since the palette allows no
  transparency.

### Estimated completion on every counting progress bar

Every determinate or timed bar shows an estimate under itself, like
"About 4 min left, done around 14:32". `ProgressBar` does this itself
through `useCompletionEstimate` (`src/lib/estimate.ts`), so apps copy
both files together.

- The rate is measured from the first value the bar sees, so a bar that
  appears partway through isn't skewed.
- The estimate is redone only when progress crosses 20, 40, 60, 80 or
  90%, or every 10 seconds, so the text holds steady. The 10-second
  check also runs while progress stalls, so the estimate grows then.
- A value that drops (a scan moving on to its next pass) starts it
  over. Nothing shows in the first second.
- A bar with no room for the line passes `hideEstimate` and shows the
  estimate elsewhere. The startup screen puts it in its label.
- ANSIapps look: the estimate is light-gray 16px text like other status
  lines. Progress in this theme doesn't ease. A determinate fill jumps
  to each new width, and the indeterminate sweep moves in 12 whole
  steps. That's the one animation the "no motion" rule allows, because
  a stopped bar would look like a hang.

## ANSI art assets (the theme pack, authored in Stylus)

**Decided 2026-09-29; the pack and its tooling aren't built yet.** Every
graphic the ANSIapps theme shows is ANSI art, and the drawn parts of
every control are too. They're made in Stylus and shared by every app.

### The canvas every asset uses

So an asset lines up with the UI's text exactly:

| Setting | Value | Why |
|---|---|---|
| Font | IBM VGA 8×16 (the theme font) | Same glyphs as the UI text |
| Cell | 8×16 px, **8-px letter spacing** (not 9) | The UI's text grid is 8 px wide |
| Aspect | Square pixels (SAUCE aspect "modern") | CSS pixels are square |
| Palette | The 16 `--dos-*` colors, nothing else | Design rules |
| Background colors | All 16 (iCE colors on) | The UI already uses bright backgrounds (cyan, light gray) |
| Format | `.XB` source with SAUCE (title, author, part name) | Self-contained, keeps the font and palette with the art |

### What's in the pack

- **Control parts**, each a **9-slice** in whole cells (corners, edges,
  fill) with one frame per state: normal, hover, pressed, focused,
  disabled. Every state's frame is the same size in the same place
  (buttons are just clickable: a pressed button doesn't move). Buttons (default, primary, danger), dialog window and its
  title rule, panel frame with its centered title, menu, text field,
  checkbox `[ ]`/`[X]`, radio, scroll bar, tabs, the progress bar's
  block fill, the selection bar.
- **Graphics:** each app's mark (the About box, splash, and the itch.io
  page), splash art, empty-state art, icons in fixed cell sizes (1×1,
  2×1, 2×2 cells), status glyphs.
- **Animations:** spinners, the indeterminate bar's sweep, animated
  splash art. Frames plus a per-frame hold time.
- **`manifest.json`:** every part's name, file, slice insets, states,
  frame timings, **alt text** (or `decorative`), and the
  **contrast role** of every colored text cell (below).

### How apps use it

- **Rendered at build time**, not at run time: a Stylus command-line
  tool (from `stylus-core`) renders every part to PNG at 1× and 2×
  and generates the CSS (`border-image` 9-slices,
  `image-rendering: pixelated`, integer scales only) for each app's
  `ansiapps-theme.css`. The apps ship PNGs and CSS, never Stylus code,
  so a closed-source app takes on no dependency. The generated files
  are committed to each app, with a note of the pack version.
- **A redraw in Stylus reaches every app:** touch up the button face
  once, re-run the render, and every app picks it up on its next
  build.
- **Text stays text.** A button's label, a dialog's title and every
  word in the UI is still HTML text in the VGA font, so it can be
  read, translated and read aloud. The art is the frame around it,
  never a picture of words (WCAG 1.4.5). The one exception is the app's
  name in its logo art, which gets alt text.

### Accessibility rules for art

These extend `docs/ansiapps-color-contrast.md` to art. **Stylus checks
them as you draw** (see its notes, "Contrast checking"), and the render
tool refuses a pack that fails.

- **Text cells** (letters, digits, and any character that carries
  meaning on its own, like a status dot): the foreground/background
  pair must be on the **ranked list of 32 passing pairs** (4.5:1, AA).
  Prefer the AAA pairs (ranks 1–14) for anything longer than a label.
- **Meaningful graphics** (icons, borders that mark a control, focus
  indicators): **3:1** against what's next to them (WCAG 1.4.11). The
  "icons and borders only" table lists the extra 3:1 pairs.
- **Decorative art** (shading, splash art, texture) has no contrast
  requirement. Mark it `decorative` in the manifest so it gets
  `aria-hidden`/empty alt text.
- **Shade characters** (░ ▒ ▓) mix their two colors. For a meaningful
  graphic, check both colors of the shade against the neighboring
  cells, not just one.
- **A colored mark that can land on several row colors** (selected,
  busy, tinted) gets its own black cell, as the location dot does.
- **Color is never the only cue** (1.4.1): a state change also changes
  a glyph or shape, and a meaningful icon has alt text or a tooltip.
- **Animation:** nothing flashes more than **3 times a second** (2.3.1),
  so an ANSImation "blink" runs at 3 Hz or slower. Anything that moves
  for **more than 5 seconds** can be paused (2.2.2). Under
  `prefers-reduced-motion`, show one still frame (the last, or a
  chosen "rest" frame).

## Licensing (each app must stay within its own license)

The only third-party asset is the font:

- **IBM VGA 8x16** (`WebPlus_IBM_VGA_8x16.woff`, the "Plus" variant for
  wide Unicode coverage, since file names aren't ASCII) from **The
  Ultimate Oldschool PC Font Pack v2.2 by VileR**,
  https://int10h.org/oldschool-pc-fonts/, **CC BY-SA 4.0**. Downloaded
  from the official `oldschool_pc_font_pack_v2.2_web.zip` on 2026-09-26.
- **How it stays compatible with a closed-source paid app** (Diskette,
  Crunchy): the font is shipped **unmodified, byte for byte**, as its
  own file, with the license text beside it and a visible credit.
  Using a font to display an app's text doesn't make the app an
  adaptation of the font, so ShareAlike covers only the font file,
  which stays under CC BY-SA. CLAUDE.md rule 7 bars copyleft *code*
  linked into the binary (GPL/AGPL/LGPL). This is a separate, unmodified
  asset file, which is a different situation.
- **Open-source apps** (Floppy, GPLv2-or-later; Stylus): the same
  unmodified file with credit ships alongside GPL code as a separate
  work, which is fine.
- **Never** subset, convert (e.g. to woff2), rename glyphs, merge the
  font into another font, or inline it as a data URI inside the app's
  bundle. Any of those creates an adapted font that would itself have
  to be CC BY-SA and marked as modified. If a modified font is ever
  needed, keep it a separate CC BY-SA file with a "modified from"
  notice, never compiled into closed code.
- **Always** keep `LICENSE.TXT` and `README.txt` next to the font, and
  keep the About credit: the font name, "VileR", the int10h.org URL,
  and "CC BY-SA 4.0".
- Any **new** theme asset must be checked the same way before use:
  permissive (MIT/Apache/OFL/CC BY/CC0) or CC BY-SA used unmodified as
  a separate file, **never** GPL/AGPL/LGPL code in a closed-source app.
  Record it in this section.
- The theme's CSS is ansiapps' own work, so each app may copy it freely
  under that app's own license.
- **The theme pack's art** is ansiapps' own work too. It lives in the
  open-source Stylus repo, so give it a license every app can ship:
  **MIT** (decided 2026-09-29, like Stylus itself). Keep the MIT notice
  with the rendered files and credit the pack in About. Never
  CC BY-SA for the art, which would make every app that ships it follow
  ShareAlike.
- **Rendered PNGs of the VGA font's glyphs** are fine to ship in any
  app: VileR's readme says images rendered from the fonts aren't covered
  by ShareAlike, and he claims no rights to the original IBM raster
  data. What stays off-limits is shipping a modified or converted
  **font file**. Stylus rasterizes the unmodified font at run time and
  never ships a converted copy.

## Per-app checklist when adding the theme

0. Build on **the grid** (Design rules): copy Coupler's
   `ansiapps-theme.css` structure and `lib/grid.ts`, not Diskette's
   older off-grid CSS, until Diskette is brought over.
1. Copy `src/lib/theme.ts`, `src/ansiapps-theme.css`, the `index.html`
   no-flash script, and `public/fonts/ansiapps/` (all three files).
2. Rename the storage key to `<app>.theme`.
3. Add the gear-menu checkbox and the About "Credits" section.
4. Map the app's own reserved colors (see Design rules), and check
   every pair against `docs/ansiapps-color-contrast.md`.
5. Web builds (Diskette's web trial) can't persist anything, so the
   choice lasts the session only there.
6. Once the theme pack exists: render it for the app, commit the PNGs
   and generated CSS, and replace the app's stand-in SVG icons with the
   pack's ANSI icons.
6. Copy the shared UI conventions' pieces too: the `index.html` splash
   and `StartupScreen.tsx`, `ProgressBar.tsx`, `ActivityStatus.tsx`,
   `lib/activity.ts` and `lib/estimate.ts`. Check each one in both themes.
7. Add App Testing checklist items and a CHANGELOG bullet.

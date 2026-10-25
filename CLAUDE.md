# Coupler

A dedicated MUD client for **CoffeeMUD** and nothing else, named for the
acoustic coupler. Tauri 2 + React/TypeScript + Rust, macOS first. Its
core behavior follows **Sip**, CoffeeMUD's official client (Bo
Zimmerman, Apache-2.0, github.com/bozimmerman/Sip). This file is the
condensed operating rules and a map of the code. The design record,
roadmap and research are the maintainer's private notes, pointed to by
a gitignored `CLAUDE.local.md` where they're checked out; without it,
work from this file, the code and `docs/`.

- **Docs**: `docs/accessibility.md` (rule 10), `docs/platform-parity.md`
  (rule 9), `docs/ansiapps-theme.md` and
  `docs/ansiapps-color-contrast.md` (the look); the study of the
  server: `docs/coffeemud-gmcp.md`, `coffeemud-ports.md`,
  `coffeemud-socials.md` (+ `.tsv`), `coffeemud-commands.md`.
  `CONTRIBUTING.md` is these rules for contributors; keep the two in
  step.
- **CoffeeMUD's source** is in `reference/CoffeeMud/` (gitignored,
  snapshot `720aec6` of 2026-10-03, CoffeeMUD 5.11.0.4; the docs'
  studies were made at `c1e556f`, nothing they read has changed). Read
  it before guessing what the server sends. To update: download the
  tarball of `bozimmerman/CoffeeMud`'s `master` and unpack it in place
  (the sandbox can't write a `.git` there); bump `BUILT_FOR` in
  `mssp.rs` if `MUD.HOST_VERSION` changed, and rerun the scripts made
  from it (`creation-art.py`, `coffeemud-bestiary.py`,
  `command-words.py`, `artisan-tree.py`, `painter-tables.py`).
- **The wiki is the players' source of truth** (decided 2026-10-24):
  check anything read from the source against
  `https://coffeemud.net:8443/wiki/index.php?title=<Page>`
  (`&action=raw` for wikitext). It has coffeemud.net's own settings and
  what players are told. Where they differ, say so, follow the live
  game, and cite the page and date read in a comment. Coupler itself
  never fetches it (rule 2).
- **`CHANGELOG.md`**: add a bullet whenever a change lands.

## Current milestone: phase 2, hear the game (started 2026-10-03)

Phase 0 (connect and play) is done. Phase 1 (a button per port, the
auto-mapper, the accessibility baseline) is built but **not yet run
against the live game**; it closes with the App Testing pass and the
"to confirm" list in `docs/coffeemud-gmcp.md`. Phase 2 is under way:
say keys, quiet, Combat mode and Immersive's voice (0.9.0), then review
mode (`reviewStep` in `App.tsx`), the Latin-1 fallback and asking for
`Room.Info` (`ask_where` in `session.rs`). What's left is in the
roadmap's phase 2.

## Non-negotiable rules

1. **No server, no telemetry, no accounts of ours.** Nothing about the
   user, their characters or their play leaves the machine except what
   they send the game.
2. **CoffeeMUD only.** The address is a constant in
   `src-tauri/src/ports.rs` (`HOST`, `PORTS`); no command, setting or
   file takes another host or port. The frontend passes a port's ID and
   Rust looks it up. The web build opens only `web_socket_url`'s
   WebSocket, and its CSP (`web/_headers`) allows nothing else.
   Anything the game asks Coupler to fetch (Client.Media sounds,
   WebView pages; never MXP images) comes only from CoffeeMUD's own
   hosts, and only once that feature is built and in the design notes.
   The launch greeting asks player counts by MSSP (`mssp.rs`), never
   logging in. Picture model files are the player's to download, never
   Coupler's. **The one exception** (decided 2026-10-06): the update
   check (`update.rs`), off until the player turns it on or asks; one
   GET of GitHub's latest release of Coupler's own repo, sending only
   Coupler's version as User-Agent; it downloads and installs nothing.
3. **Credentials stay local.** Saved passwords go in the OS keychain,
   never a plain file, localStorage or a log. A password typed while
   the server echoes is never echoed, logged or kept in history
   (`secret` in `App.tsx`).
4. **Bundle ID `com.scorelessmusic.coupler`** on Mac and iPhone (decided
   2026-10-19); Windows and Linux keep `com.ansiapps.coupler`
   (`tauri.windows.conf.json`, `tauri.linux.conf.json`). The Mac shipped
   up to 0.28.0 as `com.ansiapps.coupler`; `moved.rs` migrates its data.
   Never change it again: it keys the app-data folder.
5. **Free and open source, Apache-2.0** (`LICENSE`, `NOTICE`). So the
   global "closed source" rule doesn't apply; its other half (no
   user-visible text names a repo file) does. **No GPL/AGPL/LGPL
   dependencies.** A bundled library's notice goes in `NOTICE`; every
   third-party license ships in the app via
   `scripts/third-party-licenses.py` → `public/third-party-licenses.txt`
   (committed). A C library a crate compiles in needs an entry in its
   `BUNDLED_C`. Run it and `scripts/license-scan.py` whenever
   `Cargo.lock` or `package-lock.json` changes. Code copied from Sip
   keeps its copyright notice and is credited in About.
6. **The protocol lives in Rust** (`telnet.rs`, `ansi.rs`: pure,
   unit-tested); the frontend only draws finished lines. A garbled-output
   bug is a byte-level regression test there first.
7. **The game output shows the game's own colors.** The theme's 16-color
   rule covers Coupler's own screens only. Every UI text/background pair
   comes from `docs/ansiapps-color-contrast.md`, in both themes.
8. **All user data lives under Tauri's app-data dir**, never a hardcoded
   path.
9. **macOS first.** Add a `docs/platform-parity.md` row for every
   macOS-specific mechanism.
10. **Accessible first: an immersive game for every player**, sighted or
    not, heard as well as read. Every UI element and feature passes
    `docs/accessibility.md`'s six questions before it's done: keyboard
    alone; named and announced for VoiceOver; nothing said only by a
    picture, color or position; not too much speech; readable at low
    vision; focus goes somewhere sensible. A picture (the map) always
    has its facts in words beside it. New UI adds checklist items,
    tested with VoiceOver on.
11. **All ART is Coupler's painter's first** (decided 2026-10-04). Every
    picture Coupler makes or shows on its own is drawn by default by its
    ANSI painter (`painter.rs`, `portrait.rs`). Other sources
    (CoffeeMUD's pictures, a brought model) are the player's choice in
    gear → Pictures…, never the default. New picture work starts in the
    painter.

## ansiapps conventions (shared with Diskette, Crunchy, Floppy, Stylus)

- **In-development warning on first run**: `first_run.rs`; the main
  window is `"create": false`, built in `setup` once accepted. Desktop
  only (decided 2026-10-19): on the iPhone `start` opens the window at
  once (App Review rejects "unfinished").
- **Loading screen from the first paint**: `index.html`'s splash and
  `StartupScreen.tsx`, kept in step. Launch work joins `STARTUP_STEPS`
  in `App.tsx` with its own label.
- **Feedback for every user activity**: `runActivity`
  (`lib/activity.ts`): a busy key, a status on success, an error on
  failure.
- **Two themes**: **ANSIapps is Coupler's default** (unlike the sister
  apps), modern from the gear menu. `coupler.theme`; only `"modern"`
  means modern (`lib/theme.ts` and `index.html` alike). Accent amber
  `#facc15` (provisional).
- **Font: IBM VGA 8x16 by VileR**, CC BY-SA 4.0, in
  `public/fonts/ansiapps/` with its license, credited in About. Ship it
  unmodified: never subset, convert or inline it.
- **The app never mentions its development docs** (`docs/`,
  `CLAUDE.md`, `CHANGELOG.md`) in user-visible text. Comments may.
- **A fixed screen**: 1280 by 720 (160 by 45 cells), not resizable; full
  screen scales the stage. Every control has a fixed place; lay out in
  stage pixels (no `vw`, `vh`, media queries). Messages go in the status
  bar, never a row that pushes the screen. `lib/stage.ts`.
- **The ANSIapps grid is the rule**: every character in a cell of the
  8 by 16 grid; spacing in multiples of 8px across and 16px down; no
  borders on anything holding text (frames are `::before` in a ring of
  padding); nothing centered by the browser; rows top-aligned; only
  characters the font has (no `⌘ ⇧ ⌃`: write `Cmd+Shift+L`). After any
  change to `ansiapps-theme.css` or UI markup, run dev gear → **Check
  the Grid** on every screen and dialog (`lib/grid.ts`).
- **Buttons are just clickable**: flat, no shadow, no press-in (not yet
  ported to the sister apps).
- **A row's own buttons come first** (decided 2026-10-24): a button
  acting on one entry of a list (Show…, Play, Edit…, Delete) sits at the
  row's left edge, before the name and its box; the head's blank cell
  and the grid's column go first too (`docs/ansiapps-theme.md`).
- **A panel's frame never scrolls**: scroll a body inside (`.map-body`).
- **Everything on screen is a `Movable`** (`lib/layout.ts`,
  `Movable.tsx`) with an entry in `defaultLayout` in `App.tsx`, per
  theme, in stage pixels. Workshop can move and resize them (gear →
  Move and resize everything); placement kept in `coupler.layout`.
- **The iPhone** (`lib/formFactor.ts` `PHONE`): its own fixed screen,
  48 cells across, rows as fit above the keyboard (`phoneFit`), upright
  only. One component tree: `phoneLayout` in `App.tsx`, `src/phone.css`;
  touch targets at least 48 px. A carousel of sections
  (`lib/phoneSections.ts`: Game, Map, Here, Heard; edge swipes,
  `lib/swipe.ts`, and `PhoneBar.tsx`'s ◄ ►), a pull-down sheet
  (`PhoneSheet.tsx`), the section's keys above the command line
  (`phoneKeys`, `PhoneKeys.tsx`, `TouchPad.tsx`). iOS project:
  `src-tauri/gen/apple` (`project.yml`; run `xcodegen generate` after
  changing it). `coupler.phone` = `on` tries it on a tablet or the dev
  Mac build.
- **The tablet** (`TABLET_SCREEN`; also an iPhone in the web build,
  `WEB_IPHONE`): two stages (`tabletFit`): Workshop's desk above
  (`tabletLayout`, `tablet:` layout scope) in cells of the player's
  text size (`lib/tablet.ts`), and `.tablet-lower` below, 96 by 23: the
  command line, Coupler's own keyboard (`TabletKeyboard.tsx`,
  `lib/softKeys.ts`) or the touch controls (`TabletControls.tsx`), the
  messages. Fields get `inputmode="none"`; no zoom; upright only
  (`src/tablet.css`). `coupler.tablet` = `on` tries it on the dev Mac
  build. **`TABLET_SMALL`** (the web build's iPhone; `coupler.tablet` =
  `iphone` in dev): the lower part in the desk's own cells, its keyboard
  `SmallKeyboard.tsx` (`smallKeyRows`), fewer controls, a one-row bar
  (`html[data-size="small"]`).
- **Anything new says whether the phone and the tablet have it.**
- **Name**: 7 characters (DOS 8.3), unused by any well-known DOS program.
- **Before any commit and push**, check for security issues: paths
  leaking this machine, anything that should stay local (game logs and
  captures hold other players' names and chat), network code reaching
  anywhere but CoffeeMUD. Report, then fix or proceed.
- **GitHub Actions only where free and unlimited** (decided 2026-10-06):
  workflows only in Coupler's **public** repo (never `coupler-notes` or
  `neumetik`), only standard runners by their plain labels (`ubuntu-*`,
  `windows-*`, `macos-*`); never a larger runner (cores, `-large`,
  `-xlarge`, GPU, runner groups) or self-hosted; no paid action or
  outside service; cache at the free 10 GB default. If the repo goes
  private, the workflows stop first. Check a new workflow or label
  against GitHub's Actions billing page before it lands.
- **Suggest a checkpoint** (version bump, commit and push, `/clear`) once
  a meaningful batch lands. Never automatic.
- **Shared with the sister apps, kept identical**: `ProgressBar`,
  `ActivityStatus`, `lib/activity.ts`, `lib/estimate.ts`,
  `StartupScreen`, the gear menu, `AppTesting`, `index.css`'s tokens,
  `ansiapps-theme.css`'s shared part. Coupler's `Dialog` adds `inert`
  while closed, focus in and back, Esc, and stacking (`covered`): port
  that to the sister apps rather than reverting it here.
- Dev server port **1460** (web build **1462**; Diskette 1420, Floppy
  1430, Stylus 1450).

## Where things live

Most Rust modules are pure and unit-tested; read a module's header for
the why. Game data (`hooks.sqlite`, `journal.sqlite`,
`bestiary.sqlite`, maps, cast, fights) stays local. **New data in app data
joins `KEPT` (`backup.rs`) and `KINDS` (`storage.rs`).** **Anything new
on the desktop works in `src-web` too or joins `DESKTOP_ONLY`**
(`lib/web.ts`), with a row in the web build's table (private notes).

### Connection and protocol
- `telnet.rs`: option negotiation (Sip's differences in the header),
  TTYPE/MTTS, NAWS, NEW-ENVIRON, GMCP, MSDP (one REPORT; not
  CoffeeMUD's broken MSDP-over-GMCP), MSP, MCCP2.
- `ansi.rs`: UTF-8 across reads, SGR to spans, lines; drops MCP `#$#`
  lines. Colors stay indexes; `spanColors` in `lib/mud.ts` picks RGB.
- `session.rs`: the socket, reader thread and every event; feeds all the
  readers below; saves `maps/<world>.json`; walks routes one confirmed
  step at a time. `lib.rs`: the commands. `lib/mud.ts`: typed wrappers.
- `ports.rs` (`HOST`, `PORTS`, `web_socket_url`), `trigger.rs`,
  `clock.rs`: split out so the web build can share them.
- `mssp.rs`: the launch greeting (players online, version vs
  `BUILT_FOR`, times played from `played.json`).
- `update.rs`: the update check (rule 2), desktop only; also
  `community_open`'s fixed Discord and Reddit pages.
- `who.rs`: WHO once a minute, its reply hidden; **never for a player
  idle 9 minutes** (it would defeat the game's idle timers).
  `Who::gentle` is when Coupler's own commands go unnoticed (WHO, and
  WEATHER for the painter's sky at most every 15 minutes per area).

### Reading the game
- `mapper.rs`: the auto-mapper from GMCP `room.info`; no stored
  coordinates (header says why). `MapPanel.tsx`, `lib/terrain.ts`.
- `ways.rs`: portals, ships, caravans read from the exits text.
- `speech.rs`: each line's kind (game, talk, time, combat, prompt).
- `echo.rs`: the commands' one-line answers, said by the narrator
  (gear → Narrator's Answers…).
- `hidden.rs`: hidden details found by a long look's HIGHLIGHT color.
- `combat.rs`: Combat or Explore mode and the opponent; MSDP's exact
  figures preferred over GMCP's `enemypct`.
- `fight_log.rs`: every fight recorded for **Combat Summary**: blows
  read from CoffeeMUD's hit and miss strings, damage exact (`(N)`) or
  measured from the health that fell, shared by the hit words' ranges;
  the page in words; kept in `fights/<world>.json` (200 whole per
  character). `CombatSummaryDialog.tsx`, Cmd+Shift+Y. Both builds.
- `senses.rs`: vitals and talk addressed to the player.
- `effects.rs`: what's on the player (GMCP `char.effects.add`/`remove`,
  times from a hidden AFFECTS, gentle as WHO), cooldowns from the game's
  "You must wait", the group (GMCP `group`). The Fight's rows,
  Cmd+Shift+X, the `effectEnds` and `cooldownReady` cues, Combat
  Summary's Effects. Both builds.
- `daytime.rs`: the time of day (no time GMCP: MSDP `WORLD_TIME` plus
  the game's dawn/dusk lines), Coupler's pair `coupler.time`.
- `ambient.rs`: BGN and BGW loops, room type, weather from the text
  (`coupler.room.type`, `coupler.weather`), the place's music.
- `foley.rs`: Coupler's own events (`coupler.event`: comings and goings,
  the player's doings), placed and panned by `playEffect`.
- `character.rs`: telling a LOGOUT or SWITCH.
- `creation.rs`: character creation as questions; `CreationDialog.tsx`,
  portraits by `portrait.rs`, facts from `public/creation-art.json`
  (`scripts/creation-art.py`). `account.rs`: the account menu
  (`AccountMenuDialog.tsx`).
- `bestiary.rs`: every creature and item met (`bestiary_store.rs`;
  tables by `scripts/coffeemud-bestiary.py`); faces by `face.rs`
  (moods, hurt, effects: `face::Moment`), item pictures by
  `item_art.rs` (materials, wear), both standing in their area's
  terrain (`Map::terrain_of`); optional quiet auto-look
  (`coupler.bestiaryLook`, off by default). `BestiaryDialog.tsx`.
- `cast.rs`: every character met, each with a voice from its name
  (`cast/<world>.json`). `CastDialog.tsx`; the narrator's voice in
  `lib/voice.ts`.
- `journal.rs`: talk kept per world and character (`journal_store.rs`):
  says, tells, group in the journal, other channels in the log.
  `lib/journal.ts`, `JournalDialog.tsx`, `SpeakingPopup.tsx`.
- `channels.rs`: the game's channels on and off by its own commands
  (`NO<NAME>`, `<NAME> ON`, `CHANNELS ON|OFF`), each followed by a hidden
  `CHANNELS` read into the list (`channels` event); WHO and the tickers
  wait for its answer. The Log's Mute Channel and `ChannelsDialog.tsx`
  (`lib/channels.ts`). Both builds.
- `automation.rs`: triggers, gags, aliases, tickers (rules in
  `lib/automation.ts`, `coupler.automation`). **Gentle to the game**:
  only `Who::acting`, tickers at least `MIN_EVERY` 15 s and only
  `Who::present`, one `BURST` allowance; a ticker due while WHO's reply
  is awaited (`Who::awaiting`) waits for it rather than skip its turn.
  `AutomationDialog.tsx`; tips
  from `public/command-words.json` (`scripts/command-words.py`).
- `songbook.rs`: songs as TMF in clan journals (desktop only).

### Sound
- `hooks.rs`: every GMCP pair once, in SQLite, with triggers naming
  assets; `HooksDialog.tsx`, `HookEditor.tsx`. `lib/assets.ts` plays
  them on the SFX and BGM buses (`MixerDialog.tsx`).
- `assets.rs`: the Assets folder in app data (SFX, BGM, ART,
  SoundFonts); renders what the WebView can't play to WAV; offers a
  smaller copy of a WAV (`ShrinkDialog.tsx`). A frontend path is only
  ever `type/name` inside it. `codecs.rs`: WAV, Opus, WavPack (WavPack
  needs the root `.cargo/config.toml`'s CMake policy line).
- `soundbank.rs`: **coffeemud.sf2**, CoffeeMUD's sounds as a SoundFont
  (CC0, from Neumetik's `coffeemud/`). Not in git:
  `scripts/coffeemud-sf2-fetch.py` (pinned tag and SHA-256; run once
  after cloning) writes `src-tauri/resources/coffeemud/`. A new bank
  changes its `TAG` and `SHA256`; `coffeemud-sf2-sync.py` copies the
  map and credits. Renders cached in `soundbank-cache/<VERSION>-<MADE>/`;
  a new way of making them bumps `MADE`. The bank is the default for
  MSP, events, creatures and BGN/BGW where the player set nothing.
  Desktop only (the web plays the mirror's renders).
- `msp.rs`: MSP sounds cut from the text, played from the bank.
- `vendor/neumetik-midi/`: **Neumetik MIDI**, ansiapps' synthesizer
  (MIT, own `LICENSE`). **The original is `midi/` in the `neumetik`
  repo: change it there and copy with `scripts/neumetik-sync.sh`,
  never only here.** **No maker's trademark in any name a player sees**
  (patches, kits); `after` notes and comments may name them. Also
  `score.rs` and `tmf.rs` (the Text Music Format).
- `music.rs`, `lib/music.ts`, `MusicEditorDialog.tsx`: the Music Editor
  (desktop only); `lib/musicEditor.ts` keeps its song and player.
  Coupler's own `#` commands: `lib/commands.ts`, `runCommand`.
- Create Asset (`CreateAssetDialog.tsx`): `create.rs` paints ANSI art
  from words; `compose.rs` is Coupler's composer (rule-based, its own).
  Desktop only.
- `earcons.ts` (cues, `CuesDialog.tsx`), `lib/captions.ts` (every sound
  in words, in Heard), `lib/priority.ts` (Priority Audio).

### Voice
- `lib/voice.ts`: one queue over the system's voices and the bundled
  ones; `pronounce` for keys and initialisms.
- `synth.rs`: bundled engines, Flite (`flite-rs`) and **Pocket TTS**
  (`vendor/pocket-tts`, trimmed Candle port: **no voice cloning, never
  add the encoder back**). Its weights aren't in git:
  `scripts/pocket-tts-fetch.py` writes `src-tauri/resources/pocket-tts/`
  (~250 MB). `stretch.rs`: pitch and speed.
- `voicecache.rs`: talk and narrator lines rendered ahead into
  `voice-cache/`, sized to the disk's free space.

### Pictures
- `paint.rs` (the `Scene`, `Style`, seed), `painter.rs` (**Coupler's
  painter**, 16 VGA colors, `▀` cells; its ignored `show` test writes a
  PPM to look at; `COLORS=256` or `true`), `RoomPicture.tsx`,
  `PicturesDialog.tsx`. Looks kept in `pictures/<world>.json`. **In the
  game's colors** (`paint::Colors`, read from its `ANSI 16|256|TRUE`
  answers; `lib/palette.ts`, `coupler.palette`): with more than 16,
  `Canvas::art_in` shows gradients, night and Bayer dithers as the
  colors between; every painter command takes `colors`. Not yet: the cache, the worker, brought
  models.
- `fight_art.rs`: The Fight's picture, the room with the two faces in
  it (`face::standing`, the player mirrored), and its moments' frames (a
  blow's jolt and flash, a foe sliding in); `FightPanel.tsx`, the bout,
  the moods and the frames in `lib/fight.ts`. On the phone a strip of
  eight rows: the picture small (`face::Size::Small`, the faces drawn
  again at 16 by 8 cells in `face.rs`'s `small`, never scaled), the
  words between the faces; four rows of words with pictures off.
- `figure.rs`: a face or an item drawn on a clear canvas, each pixel's
  part (`painter::Part`), laid on its place (the room painter's, a
  shade back) and lit by it. Tables by `scripts/painter-tables.py`:
  `materials.tsv`, `effect_looks.tsv`. Contact sheets: `face::look` and
  `item_art::look`'s ignored `show` tests.
- `ansi_art.rs`, `AnsiPicture.tsx`: ANSI art for ART triggers.

### Screens and UI
- **Three ways to play** (`lib/ux.ts`): Terminal (with
  `ControlPanel.tsx`), Immersive and Workshop (the default). **They
  differ only in what's on the screen** (decided 2026-10-23): cues,
  voice, hooks and sounds work the same in each; what a layout has no
  button for is in the gear menu. Immersive's director is
  `lib/immersive.ts` (fights: `lib/fightTalk.ts`); its goal is **fewer
  spoken words**: what can be a sound is one.
- `App.tsx`: the screen, layouts (`uxLayout`), input, history, password
  mode, `SHORTCUTS`. `Terminal.tsx`: output view and NAWS.
- `lib/display.ts` (gear → Display…): readable colors, big text.
- `scrollbars.ts`: drawn scroll bars on touch screens.
- `ArtisanDialog.tsx`, `lib/artisan.ts` (`artisanSkills.ts` from
  `scripts/artisan-tree.py`).
- `TutorialDialog.tsx`, `lib/tutorial.ts`: Before You Play, offline.
- `backup.rs` (`backup_format.rs`): Coupler Backup (`.coupler`, Brotli),
  restored at launch. `storage.rs`: gear → Storage….

### The web build (coupler.ansiapps.com)
`src-web/` includes the pure modules by `#[path]` and adds its own
`session.rs`, stores, `ffi.rs` (C-style, no wasm-bindgen). `src/web/`
stands in for Tauri's modules in Vite's `--mode web` (`core.ts` owns
the WebSockets and IndexedDB). Installable (`web/manifest.webmanifest`),
no service worker. `examples/web_mirror.rs` mirrors this Mac's hooks and
assets for it.

## Testing

- Before calling a change done: `cargo test --manifest-path
  src-tauri/Cargo.toml`, `cargo clippy --manifest-path
  src-tauri/Cargo.toml --all-targets`, `npx tsc --noEmit`. When
  Neumetik MIDI changed, also `cargo test --manifest-path
  src-tauri/vendor/neumetik-midi/Cargo.toml`. When a shared module or
  `src-web/` changed, also `cargo test` and `cargo clippy --all-targets`
  in `src-web/`, and `npm run build:web` (runs `src-web/tests/core.mjs`).
- **App Testing checklist** (dev-only, gear menu):
  `src/lib/testChecklist.ts`. Add items when a feature lands, drop them
  once confirmed. Production builds must not contain it: grep
  `dist/assets/*.js` for "App Testing".
- Testing against the live game means a real character on a shared
  server: don't script floods of commands at coffeemud.net. For protocol
  work, run a local CoffeeMUD and point a dev build at it by editing
  `HOST` locally, never committed.

## Build

- **"Build the macOS app" means the `.app` only**:
  `scripts/build-release.sh --bundles app`. DMG only when asked. Never a
  bare `tauri build`. The script builds universal, signs with the
  Developer ID and hardened runtime, notarizes with the `scorelessmusic`
  notarytool profile (`COUPLER_NOTARY_PROFILE` overrides), staples,
  strips `$HOME` paths, verifies all of it, zips
  `Coupler_<version>_universal.app.zip`, and installs to
  `~/Applications` (skipped with `CI` or `COUPLER_NO_INSTALL`). Setup is
  in its header; `COUPLER_UNSIGNED=1` for local tries only. No Apple
  credential is ever in the repo or the environment.
- **Windows and Linux**: GitHub Actions, `windows.yml` (NSIS, MSI) and
  `linux.yml` (.deb, .rpm; never an AppImage, it bundles LGPL
  WebKitGTK), unsigned. Successful runs on `main` or a `v*` tag publish
  via `scripts/publish-release.sh`. **Releasing is pushing the tag**
  (a push to a branch builds only when the workflow file changed): bump,
  commit, push, then `git tag -a v<version>` on that commit and push it.
  The Mac's zip is uploaded to the release by hand.
- **The web build**: `npm run build:web` makes `dist-web/`;
  `scripts/web-publish.sh` mirrors, builds and uploads to Cloudflare
  Pages after a yes. Only when asked: it publishes the mirror to anyone.

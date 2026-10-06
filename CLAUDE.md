# Coupler

A dedicated MUD client for **CoffeeMUD** and nothing else. Named for the
acoustic coupler, the cradle a phone handset sat in to carry a modem's
data. Tauri 2 + React/TypeScript + Rust, macOS first. Its core behavior
follows **Sip**, CoffeeMUD's official client (Bo Zimmerman, Apache-2.0,
Electron, github.com/bozimmerman/Sip). This file is the condensed
operating rules. The design record, the roadmap and the research
behind them (the plan, the feature list, the speech engines and
picture makers by license, the web build's design) are the
maintainer's private notes: a `CLAUDE.local.md` (gitignored) points to
them where they're checked out. Without it, work from this file, the
code and the docs here.

Coupler's docs: `docs/accessibility.md` (rule 10),
`docs/coffeemud-gmcp.md`, `docs/coffeemud-ports.md` and
`docs/coffeemud-socials.md` (the study of the server; socials' lines to
match in `docs/coffeemud-socials.tsv`), `docs/coffeemud-commands.md`
(every command: a block or one line), `docs/platform-parity.md`
(rule 9), `docs/ansiapps-theme.md` and
`docs/ansiapps-color-contrast.md` (the look). `CONTRIBUTING.md` is
these rules for contributors; keep the two in step.
CoffeeMUD's source is unpacked in `reference/CoffeeMud/` (gitignored,
Apache-2.0, snapshot `720aec6` of 2026-10-03, CoffeeMUD 5.11.0.4; the
studies in `docs/` were made at `c1e556f`, and nothing they read changed
since): read it before guessing what the server sends. To update it,
download the tarball of `bozimmerman/CoffeeMud`'s `master` (the sandbox
can't write a `.git` there) and unpack it in its place. A new snapshot
changes `BUILT_FOR` in `mssp.rs` when `MUD.HOST_VERSION` changed.

**`CHANGELOG.md`**: add a bullet whenever a change lands.

## Current milestone: phase 2, hear the game (started 2026-10-03)

Phase 0 (connect and play) is done: telnet the way Sip does it (with
its bugs fixed, see `telnet.rs`'s header), MCCP2, ANSI to truecolor,
hidden password input, history, NAWS, GMCP received. Phase 1 is built
and **not yet run against the live game**: a button per CoffeeMUD port,
the auto-mapper from GMCP `room.info`, the accessibility baseline. It
closes with the App Testing pass and the "to confirm" list in
`docs/coffeemud-gmcp.md`. Phase 2, "hear the game", is under way: say
keys, quiet, Combat mode and Immersive's voice (0.9.0), then review
mode (Option+Up/Down, `reviewStep` in `App.tsx`), the Latin-1 fallback
and asking for `Room.Info` (`ask_where` in `session.rs`). What's left
and its order is in the roadmap's phase 2.

## Non-negotiable rules

1. **No server, no telemetry, no accounts of ours.** Coupler talks to the
   game and to nothing else, but for the update check the player turns
   on (rule 2). Nothing about the user, their characters or their play
   leaves the machine except what they send the game.
2. **CoffeeMUD only.** The address is a constant in
   `src-tauri/src/ports.rs` (`HOST`, and `PORTS`: the games
   coffeemud.net runs, one per port); no command, setting or file takes
   another host or a port number. The web build opens only
   `web_socket_url`'s WebSocket on the same host, and its CSP
   (`web/_headers`) allows nothing else. The frontend passes a port's ID, and
   Rust looks it up. Anything the game asks Coupler to fetch
   (Client.Media sounds, WebView pages; never MXP images, decided
   2026-10-03) is fetched only from CoffeeMUD's own hosts, and only
   once that feature is built and listed in the design notes. The greeting at
  launch asks the game how many are online by MSSP on the same host
  and port list (`mssp.rs`), never logging in. Room pictures
   are made locally; their model files are the player's to download,
   never Coupler's. The one exception, decided 2026-10-06: **the update
   check** (`update.rs`), off until the player turns it on or asks (gear
   menu), one GET of GitHub's latest release of Coupler's own
   repository, sending only Coupler's version as its User-Agent; it
   downloads and installs nothing. This is Coupler's only network code.
3. **Credentials stay local and protected.** When saved logins arrive,
   passwords go in the macOS Keychain (the OS store elsewhere), never in
   a plain file, localStorage or a log. A password typed while the
   server echoes is never echoed, logged or kept in history (today:
   `secret` in `App.tsx`).
4. **Bundle ID `com.ansiapps.coupler`.** Never change it once a build
   ships: it keys the app-data folder.
5. **Coupler is free and open source, Apache-2.0** (`LICENSE`, `NOTICE`;
   decided 2026-10-02, to match CoffeeMUD and Sip). So the global rule's
   "the app is closed source" doesn't apply here; its other half (no
   user-visible text names a repo file) still does. Still **no
   GPL/AGPL/LGPL dependencies**, so the license stays a choice. A
   bundled library's notice goes in `NOTICE`. Every third-party license
   text ships in the app (About → Open-Source Licenses):
   `scripts/third-party-licenses.py` writes
   `public/third-party-licenses.txt` (committed; `build-release.sh`
   rewrites it each release). A C library a crate compiles in needs its
   own entry in the script's `BUNDLED_C`. Run it and
   `scripts/license-scan.py` whenever `Cargo.lock` or `package-lock.json`
   changes. Sip is Apache-2.0: porting its logic is fine, and any code
   copied from it keeps its copyright notice and is credited in About.
6. **The protocol lives in Rust** (`telnet.rs`, `ansi.rs`: pure and
   unit-tested); the frontend only draws finished lines. A bug report
   about garbled output is a byte-level regression test there first.
7. **The game output shows the game's own colors.** The theme's 16-color
   rule and the contrast list cover Coupler's own screens only. Every UI
   text/background pair comes from `docs/ansiapps-color-contrast.md`, in
   both themes.
8. **All user data lives under Tauri's app-data dir**, never a hardcoded
   path.
9. **macOS first.** Add a `docs/platform-parity.md` row for every
   macOS-specific mechanism.
10. **Accessible first: an immersive game for every player.** Coupler
    should be the most immersive way into CoffeeMUD for everyone,
    sighted or not: a game heard as well as read, where what serves a
    screen reader, low vision or play by ear makes it richer for all.
    Every UI element and feature is run through `docs/accessibility.md`'s
    six questions before it's called done: keyboard alone; named and
    announced for VoiceOver; nothing said only by a picture, color or
    position; not too much speech; readable at low vision (contrast,
    size, focus rings); focus goes somewhere sensible. A picture (the
    map) always has the same facts in words beside it. New UI adds
    items to the checklist's Accessibility section, tested with
    VoiceOver on.
11. **All ART is Coupler's painter's first.** Every picture Coupler
    makes or shows on its own is drawn by default by Coupler's ANSI
    painter (`painter.rs`, `portrait.rs`): room pictures, race and class
    portraits, and anything new that shows a picture. Every other source
    (CoffeeMUD's own pictures, a model the player brings, an image made
    elsewhere) is the player's to choose in gear → Pictures…, never the
    default; the player's own ART on a hook is their choice already.
    New picture work starts in the painter (decided 2026-10-04).

## ansiapps conventions (shared with Diskette, Crunchy, Floppy, Stylus)

- **In-development warning on first run**: `src-tauri/src/first_run.rs`.
  The main window is `"create": false` and built in `setup` only once
  it's accepted.
- **Loading screen from the first paint**: the `index.html` splash and
  `components/StartupScreen.tsx`, kept in step. Launch-time work joins
  `STARTUP_STEPS` in `App.tsx` with its own label.
- **Feedback for every user activity**: `runActivity` (`lib/activity.ts`)
  with a key for the control's busy state, a status on success, an error
  on failure. Connecting has no count, so its bar is indeterminate.
- **Two themes**: **ANSIapps is Coupler's default** (unlike the sister
  apps), modern from the gear menu. Key `coupler.theme`; only `"modern"`
  stored means modern, in `lib/theme.ts` and `index.html`'s inline
  script alike. Coupler's accent is amber `#facc15`, yellow in the
  ANSIapps theme (provisional). Follow `docs/ansiapps-theme.md`.
- **The theme's font is IBM VGA 8x16 by VileR**, CC BY-SA 4.0, in
  `public/fonts/ansiapps/` with its license, credited in About. Ship it
  unmodified as its own file: never subset, convert or inline it.
- **The app never mentions its development docs**: no user-visible text
  names a file in `docs/`, `CLAUDE.md` or `CHANGELOG.md`. Code comments may.
- **A fixed screen**: the window is 1280 by 720 (160 by 45 of the
  theme's 8 by 16 characters), not resizable; full screen scales the
  same stage up. Every control has a fixed place. Lay out in stage
  pixels: no `vw`, `vh` or media queries. Messages go in the status bar
  (bottom, fixed height), never in a row that appears and pushes the
  screen. `src/lib/stage.ts`.
- **The ANSIapps grid is the rule**: every character sits in a cell of
  the 8 by 16 grid. Spacing in multiples of 8px across and 16px down,
  16px lines, no borders on anything holding text (frames are `::before`
  in a ring of padding), nothing centered by the browser, rows aligned
  to the top, only characters the font has (no `⌘ ⇧ ⌃`: write
  `Cmd+Shift+L`). `docs/ansiapps-theme.md`, "The grid". After any change
  to `ansiapps-theme.css` or to UI markup, run dev gear → **Check the
  Grid** on every screen and dialog (`src/lib/grid.ts`).
- **ANSIapps buttons are just clickable**: flat, no shadow, no press-in
  movement (`docs/ansiapps-theme.md`, "Depth"). Decided here; the sister
  apps still need it ported.
- **A panel's frame never scrolls**: its title sits in the top edge and
  a scrolling box clips it. Scroll a body inside (`.map-body`,
  `.ports-body`).
- **Name**: Coupler is 7 characters (DOS 8.3), and no well-known DOS
  program used it.
- **Before any commit and push**, check the changes for security issues:
  paths leaking this machine, anything that should stay local (game logs
  and captures hold other players' names and chat), and whether the
  network code could reach anywhere but CoffeeMUD. Report, then fix or
  proceed.
- **GitHub Actions only where it's free and unlimited** (decided
  2026-10-06): GitHub bills nothing for a **public** repository on
  **standard GitHub-hosted runners** (`ubuntu-*`, `windows-*`,
  `macos-*` by their plain labels), artifacts and caches included. So:
  workflows live only in Coupler's public repository, never in a private
  one (`coupler-notes`, `neumetik`: their minutes are metered); never a
  larger runner (any label with cores, `-large`, `-xlarge`, GPU, or an
  organization's runner group: billed even in public repositories) nor a
  self-hosted one (GitHub announced a per-minute charge for those in
  December 2025 and only postponed it); no paid marketplace action or
  outside service; the cache stays at the free 10 GB default, never
  raised (more is billed). If the repository ever goes private, the
  workflows stop first. A new workflow or runner label is checked
  against GitHub's Actions billing page before it lands.
- **Suggest a checkpoint** (version bump, commit and push, `/clear`) once
  a meaningful batch lands. Never automatic.

## Where things live

- `src-tauri/src/telnet.rs`: the telnet state machine: option
  negotiation (what Coupler accepts and why, Sip's differences listed in
  the header), TTYPE/MTTS, NAWS, NEW-ENVIRON, GMCP (Core.Hello and
  Supports), MSDP (one REPORT, `WORLD_TIME`; not CoffeeMUD's broken
  MSDP-over-GMCP), MCCP2 inflate (`flate2`, pure Rust).
- `src-tauri/src/ansi.rs`: UTF-8 decoding across reads, SGR to styled
  spans (16, 256, 24-bit), line splitting; MCP's out-of-band lines
  (`#$#`, CoffeeMUD's greeting on every connect) dropped. Colors stay
  indexes; the view
  picks RGB (`src/lib/mud.ts`, `spanColors`).
- `src-tauri/src/mapper.rs`: the auto-mapper, pure and unit-tested:
  rooms keyed by the game's room ID, exits with destinations, doors,
  landmarks, the layout around the player, routes and directions in
  words. No stored coordinates (the header says why).
- `src-tauri/src/hooks.rs`: the hooks database, unit-tested: every
  GMCP key and value received, flattened to dotted keys
  (`room.info.zone`), each unique pair once, in SQLite (`rusqlite`,
  bundled), plus the keys filtered out of the list (still recorded). `session.rs` feeds it and opens it at `hooks.sqlite` in app
  data (game data: it stays local).
- `src-tauri/src/assets.rs`: the **Assets** folder (`Assets` in app
  data, made at launch): dropped or chosen files copied into a folder
  per file type; the three kinds (SFX `.wav .ogg .opus .wv`, BGM `.mid .mp3
  .mod .xm .s3m .it`, ART `.png .jpg .ans .asc`) and SoundFonts (`.sf2`, for
  MIDI); a frontend path is only ever `type/name` inside it. An import
  reports progress over a Tauri `Channel` (bytes read, each file's
  step) and what became of each file. Renders
  what the WebView can't play to WAV: Ogg (`lewton`), Opus and WavPack
  (`codecs.rs`), MIDI through Neumetik (below) unless the player
  chose one of their SoundFonts for that file (`set_font`,
  `.soundfonts.json` in Assets; a SoundFont gone falls back to
  Neumetik; `rustysynth`), a SoundFont as a sample tune
  (`compose::sample`), modules (`xmrsplayer`). Unit-tested. A WAV coming in is also made as
  Opus and WavPack and the smallest waits in `Assets/.smaller/`
  (cleared at launch); the import's `smaller` offers it, and
  `components/ShrinkDialog.tsx` asks: `assets_compress` puts it in the
  WAV's place and deletes the WAV, `assets_keep` forgets it. The
  Hooks dialog's Assets tab tries each (Play and Stop, one at a time:
  `playPreview` in `lib/assets.ts`; Show… for pictures,
  `components/AssetPreview.tsx`) and chooses each MIDI file's
  SoundFont. `add_made` saves what Create Asset makes.
- **Create Asset** (`components/CreateAssetDialog.tsx`, beside Add
  Assets): an asset made from the player's words, on this computer.
  `src-tauri/src/create.rs`, pure and unit-tested: the words read for
  what Coupler's painter draws (a terrain, the weather, the time of
  day, high fantasy; the rest only the seed) and the painting written
  as UTF-8 ANSI art with SAUCE (`asset_create_art`, rule 11).
  `src-tauri/src/compose.rs`, pure and unit-tested: **Coupler's
  composer**, the words read for a mood (`MOODS`, 22: mode, tempo,
  meter, instruments, drums, swing, sevenths, length), an instrument,
  a key, a scale (13), slow or fast, a tempo, bars; then `Options`
  (Create Asset's twelve menus, each Automatic or a choice that wins
  over the words): key, scale, length, loop or piece, arc, parts,
  brightness, drive and tension (leaning on the mood), swing, timing,
  fills. Drums first, the bass after them (on the kick for some), held
  chords, a tune in two-bar phrases, a broken chord and a second line
  in counterpoint, each part its own seeded hand; written as MIDI with
  its time and key signatures for Neumetik, a loop exact
  (`asset_create_music`). `Piece::rules` says every decision in words,
  listed under the music made. Its features follow notebin.fm's
  rule-based generator (its code isn't published; this is Coupler's
  own). The same words make the same asset; again, the next take
  (`name (2)`).
- `src-tauri/vendor/neumetik/`: **Neumetik**, Coupler's own synthesizer,
  its own crate (standard library only), pure and unit-tested: GM and
  GS MIDI without a SoundFont, no samples. **The original is ansiapps'
  private `neumetik` repository** (proprietary, so ansiapps' closed
  apps can use it too); this is Coupler's copy, Apache-2.0 as Coupler.
  Change it there and copy it here with `scripts/neumetik-sync.sh`
  (`../neumetik` by default), never only here; the license scripts
  skip it as Coupler's own (`OWN`). Three engines (subtractive with
  supersaw unison and sync, two-operator FM with a "tine" pair,
  Karplus-Strong plucked strings), an SVF filter, envelopes, LFO,
  chorus and reverb sends (`voice.rs`, `lib.rs`); `smf.rs` reads the
  file. Banks by Bank Select MSB: GM's 128 (`gm.rs`), the SC-55's
  variation tones falling back to GM's (`gs.rs`), and the classic
  synth sounds, 80 the eighties, 81 the nineties, 82 the 2000s to now
  (`classics.rs`, each patch's `after` names what it's after). Drums
  (`drums.rs`): every key 27 to 87, GS's kits and the drum machines at
  the SC-88's program numbers. **No maker's trademark in a name a
  player sees** (patches, kits: "Ladder Lead", "Analog Boom", not Moog
  or TR-808); the `after` notes and comments may name them.
  `assets.rs` renders MIDI through it unless the player chose a
  SoundFont for the file. Its ignored `neumetik_demo` test writes a
  tour of the banks (`OUT`).
- `src-tauri/src/codecs.rs`: WAV reading, and Opus (libopus via
  `opusic-sys`, Ogg pages by `ogg`, `rubato` to 48 kHz) and WavPack
  (`wavpack-sys`) both ways, pure and unit-tested. Both C libraries
  build from bundled source with cmake; WavPack needs
  the root `.cargo/config.toml`'s CMake policy line (found
  from the repo root or any folder in it).
  The hooks' triggers (`hooks.rs`, table `triggers`) name these files;
  `assets_list` first clears any asset whose file is gone from every
  trigger (`Hooks::clear_missing`) and counts each asset's uses (the
  Hooks dialog's Assets tab, a green bullet per one in use).
  `session.rs` emits `hook-fired` each time a pair with one comes, and
  `src/lib/assets.ts` plays and shows them: two buses (SFX, BGM) at the
  Mixer's volumes (`components/MixerDialog.tsx`, `coupler.volume.*`),
  each trigger's volume a share of its bus; rendered music is
  normalized to just under full scale (`louder`).
- `src-tauri/src/backup.rs`: **Coupler Backup**, unit-tested: gear →
  Export Coupler Backup… writes `KEPT` (the app-data items backed up:
  Assets but `.smaller`, maps, pictures, cast, hooks and journal through
  `VACUUM INTO`, played.json) and the frontend's `coupler.` settings
  (`lib/backup.ts`) to one `.coupler` file, Brotli (strongest setting,
  largest window; what's compressed already stored), each block
  CRC-checked. Restore Coupler Backup… (or a backup dropped on the
  window, `RestoreDialog.tsx`) unpacks it to `.restoring` and restarts;
  `apply` swaps it in at launch before anything opens (the old in
  `.before-restore`), and `start` in `lib.rs` puts the settings back by
  the window's initialization script. **New data in app data joins
  `KEPT`.** Desktop only.
- `src-tauri/src/music.rs`: **the Music Editor's song**, pure and
  unit-tested: a MIDI asset read into parts (a channel of a track),
  bars and notes, every event it doesn't show kept and written back
  (format 1, each track ending at the last bar); `preview` the bars to
  play. `lib/music.ts` is the edits, `components/MusicEditorDialog.tsx`
  the screen (Workshop's BGM button beside the Mixer): the pattern, the
  piano roll, Play Bar and Play Song looping (`playPreviewBytes`)
  through Neumetik unless a SoundFont is picked there (Plays through),
  Save… and Save as… (`music_save`, `music_save_as`, never over another
  file). Desktop only.
- `src-tauri/src/ambient.rs`: ambience, pure and unit-tested: BGN
  (background noise) and BGW (background weather), two more trigger
  slots that loop while they apply. Works out the room type (indoors or
  outdoors) from `room.info.terrain`, and reads the weather from the
  game's text (CoffeeMUD has no weather GMCP; every line is from its
  `lists.ini`), both recorded as Coupler's own pairs, `coupler.room.type`
  and `coupler.weather`. Picks the BGN (room id, then terrain, then room
  type) and the BGW (under the sky); `session.rs` emits `ambient` when
  that changes and `lib/assets.ts` `setLoop` fades between them on the
  BGN and BGW buses.
- `src-tauri/src/daytime.rs`: the time of day, pure and unit-tested:
  CoffeeMUD has no time GMCP, so it takes the game's hour from MSDP's
  `WORLD_TIME` and the part of the day from where each starts (the
  stock clock's to begin with), corrected by the `lists.ini` lines the
  game says at dawn, dusk and night (a line within seconds of a new
  hour shows where its part starts) and by the TIME command's. The
  game never says when day starts. `session.rs` records it as
  Coupler's own pair `coupler.time` (`dawn`, `day`, `dusk`, `night`,
  all four listed from the start by `Hooks::add_own`). `arc` is where
  the sun (or the moon by night) is across the sky, left to right, for
  the painter (`Scene::arc`); `about_time` tells the lines about the
  time (the changes and all TIME says), `speech.rs`'s `time` kind,
  which Immersive's narrator says instead of writing.
- `src-tauri/src/ansi_art.rs`: ANSI art for ART triggers, pure and
  unit-tested: classic CP437 (SAUCE width and iCE colors, cursor
  moves) or UTF-8 with Coupler's own SGR (`ansi::apply_sgr`, shared
  with the terminal), drawn onto a grid and cut into the same styled
  lines as the game output. `components/AnsiPicture.tsx` shows it.
- `src-tauri/src/paint.rs` and `src-tauri/src/painter.rs`: **room
  pictures** (phase 1 under way). `paint.rs`
  is what a picture is made from: the `Scene` (world, room ID, name,
  zone, terrain, weather, time; never talk or anything typed), the
  `Style` (fantasy, high) and the room's `seed` (FNV, so a room keeps
  its look). `painter.rs` is **Coupler's painter**, pure and
  unit-tested: an ANSI scene at any size in cells, two square pixels a
  cell (`▀`) in the 16 VGA colors as indexes, CP437 characters over
  them for stars, rain, snow, waves, leaves. Its ignored `show` test
  prints pictures and writes a PPM (`OUT`) to look at while tuning.
  `session.rs` gathers the scene after each read, after the sounds,
  and emits `picture` when it changes; `picture_paint` paints it at the
  size the view asks (`components/RoomPicture.tsx`, in Here in
  Immersive and in Workshop's `PicturePanel.tsx`); gear → Pictures…
  (`PicturesDialog.tsx`, `lib/pictures.ts`, `coupler.pictures`) picks
  the engine, the style and when (every room, landmarks, on a key:
  `pictureShows`). Indoors and underwater nothing of the sky shows.
  Cmd+Shift+P (`paintRoom` in `App.tsx`,
  `picture_again`) paints a new look the room keeps (`paint::Looks`,
  `pictures/<world>.json` in app data; the Scene's `look`); the
  player's ART on `room.info.id` is the Scene's `art` and shows in
  place of the painting. Not yet: the picture cache, the worker,
  brought models.
- **Seeing the game** (low vision, deaf and sighted players):
  `src/lib/display.ts` (gear → Display…, `DisplayDialog.tsx`,
  `coupler.display`): game colors as sent, lifted to 4.5:1
  (`readable`, used by `spanColors`) or none; 2x/3x text in Terminal
  (`bigTextLayout` in `App.tsx`, `.desk.text-2x`/`3x` in both
  stylesheets); sound captions. `src/lib/captions.ts`: every earcon,
  hook sound, music and loop says itself in words, written in Heard
  (`HeardPanel.tsx`), never spoken. `components/Gauges.tsx`: the bars
  (Here, and Workshop's You panel `VitalsPanel.tsx`). Review mode's
  split view is `Terminal.tsx`'s `.terminal-live`, laid over the held
  view so the game's size never changes.
- `src-tauri/src/character.rs`: telling a LOGOUT or a SWITCH while
  still connected, pure and unit-tested (the game's logout question
  and a yes; `char.base` naming someone else). `session.rs` then stops
  the walk (a logout also leaves the room, as disconnecting does) and
  emits `character-left`; the frontend fades the sounds out
  (`fadeAll`) and closes the pictures.
- `src-tauri/src/combat.rs`: **Combat mode or Explore mode** (in a
  fight or not) and the opponent, pure and unit-tested. GMCP
  `char.status`'s `enemy` (pushed at once when a fight starts) and
  MSDP's `OPPONENT_HEALTH` (within a second, both ways) each say
  whether there's a fight; the latest change wins. Not `state` 8
  (sitting or sleeping hides it). The menu bar's mode chip (`App.tsx`,
  `.play-mode`) shows it. **MSDP's exact figures are preferred**
  (`OPPONENT_HEALTH`, `_MAX`, `_RANGE`, REPORTed by `telnet.rs`); GMCP
  `char.status` gives the name, and its `enemypct` only when the server
  sends no MSDP. The header says why the other `OPPONENT_` variables
  are left out. `session.rs` emits `combat` (`onCombat` in
  `lib/mud.ts`) when it changes, null when the fight's over.
- `src-tauri/src/speech.rs`: what kind each output line is (game,
  talk, time, combat, prompt), pure and unit-tested: talk matched to
  `comm.channel` (sent before the text, unwrapped, so wrapped lines
  count; a say found by its speaker, since a mood or a language changes
  it after its GMCP, and kept as printed or translated by `heard`; a
  whisper or a yell from afar read from the text alone), the finished
  prompt, a fight from `combat.rs`. Sent with
  `mud-output` as `kinds`. The game output isn't a live region: App
  gives the screen reader the kinds the player keeps on (gear →
  Speech…, `lib/speech.ts`, `coupler.speech`) through a hidden one,
  and an unchanged prompt never.
- `src-tauri/src/echo.rs`: **the commands' one-line answers**, pure and
  unit-tested: from `docs/coffeemud-commands.md`, the commands that
  answer in one line (`COMMANDS`, by their exact access words) and what
  the narrator says for each line (`ECHOES`, `*` a name, `{1}` its
  value). `session.rs` tells it each line sent (`typed`) and, after
  `speech.rs`, marks the next reply's answer `echo` and sends it with
  `mud-output` as `echoes`. Immersive doesn't write them (Terminal's
  `hide.echo`) and `lib/immersive.ts` has the narrator say them;
  Workshop's gear → Narrator's Answers… (`components/EchoesDialog.tsx`,
  `lib/echoes.ts`, `coupler.echoes`, the table from `echo_list`)
  changes the words.
- `src-tauri/src/hidden.rs`: **hidden details**, pure and
  unit-tested: a long look at the room (`LL`, `LONGLOOK`, `EXAMINE`
  alone) colors the words of the room's hidden items in HIGHLIGHT
  (`CommonMsgs.getFullRoomView`, `LOOK_LONG`); without MXP the color is
  all that marks them. `session.rs` tells it each line sent and, after
  `echo.rs`, finds the room (`Map::here_name`), its description and the
  words in another color, sent with `mud-output` as `hidden`. Immersive's
  narrator says them (`lib/immersive.ts`); the screen reader hears
  them after the room (`App.tsx`).
- `src-tauri/src/creation.rs`: **making a character**, pure and
  unit-tested: CoffeeMUD's character creation (`CharCreation.java`) as
  questions (`Step`: its kind, the choices with the game's words about
  each, the stats table, a problem, what's settled), found from the
  prompt and the text since the last one. Guiding starts at a new
  account or character and ends in the game (`room.info`), at a login's
  password or on hanging up; a typed line is never kept. `session.rs`
  tells it each line sent and emits `creation` (null when over) before
  `mud-output`. `components/CreationDialog.tsx` is the guide (every way
  to play; Type Instead hides it, Cmd+Shift+G shows it), its words and
  the narrator's in `lib/creation.ts`; while it's open the game's lines
  aren't read or said, and Immersive cues each question. A mode, the
  guide or CoffeeMUD's own way (`coupler.creationMode`, the gear menu).
  Races and classes show a portrait by Coupler's painter
  (`src-tauri/src/portrait.rs`, `portrait_paint`: a race at its size
  beside a six-foot stick, a class in its clothes with its tools), or
  CoffeeMUD's own pictures if the player chooses them (gear →
  Pictures…, `portraits`), and the player races' facts
  (`lib/creationArt.ts`, `public/creation-art.json`, made by
  `scripts/creation-art.py` from the snapshot's web images and
  `Races/*.java`; Pillow, the maintainer's; run it again with a new
  snapshot); alignment and inclination say what they do
  (`creation.factionImpact`). WHO waits while a character's made
  (`Who::creating`): the stats prompt would pass for the usual one.
- `src-tauri/src/account.rs`: **the account menu**, pure and
  unit-tested: CoffeeMUD's (`CharCreation.java`'s `acctmenu*`) read
  from its letters and the Account command's list of characters, open
  while "Command or Name (?)" is the prompt; `wants_list` has
  `session.rs` send `L` once. Emits `account-menu` (null when gone).
  `components/AccountMenuDialog.tsx` is the dialog (in place of the
  guide's account menu), its words in `lib/account.ts`; a y/N it
  already asked is answered for the player (`pendingConfirm` in
  `App.tsx`). Both builds.
- `src-tauri/src/senses.rs`: the character's vitals (`char.vitals`,
  `char.maxstats`) and the talk addressed to them (`comm.channel`:
  tell, group, say, channel; their own lines marked), pure and
  unit-tested. `session.rs` emits `vitals` on a change and `talk` per
  line, for Immersive mode.
- `src-tauri/src/journal.rs`: **the journal and the log**, unit-tested:
  every `comm.channel` line kept in SQLite (`journal.sqlite` in app
  data, game data: it stays local) per world and character, by who said
  it and when. Says, tells and the group go in the journal; every other
  channel in the log. A typed channel LAST (`OOC LAST 10`) isn't sent:
  `journal_last` answers it from the log (`last_asked`, `Journal::last`).
  A repeat (same speaker, same words, numbers and
  punctuation ignored) from anyone not a player isn't kept again.
  `heard` only once Coupler's voice finished a line (`lib/voice.ts`
  `speak`'s `onDone`). `session.rs` records each line (`talk` carries
  its `entry`, or `repeat`) and emits `journal-changed` with what's
  unheard. `src/lib/journal.ts` speaks the journal as it comes (one
  queue, Cmd+Period flushes it), blips the log (`earcons.logLine`, at
  most one per `LOG_COOLDOWN`; a held-back line and 10 minutes unnoticed
  plays `earcons.logReminder` with the unread count), and
  knocks when the journal has lines nothing will say;
  `components/JournalDialog.tsx` shows either book. In Immersive, talk
  isn't written in the game output (`TermLine.talk`, from `speech.rs`'s
  kinds; Terminal's `hide`, and review skips it), nor the player's
  prompt once in the game (`playerPrompt`: a question still shows):
  `components/SpeakingPopup.tsx` shows each line at the top while it's
  said (`voice.onSpeaking`).
- `src-tauri/src/voicecache.rs`: **the voice cache**, unit-tested:
  every line of talk rendered in a bundled voice the moment it arrives
  (`voice_prerender`, a worker thread in `lib.rs`, the journal's lines
  before the log's), kept as `<hash>.pcm` in `voice-cache/` in app data
  with an `index.sqlite` of when each was made, last played and how
  often. `voice_synth` with a `book` reads it first. Its size follows
  the disk's free space (`budget`, `free_space`; checked after each
  render and every five minutes); `trim` flushes lines played once
  first (the log's before the journal's, oldest first), then the
  fewest plays, then never-played. The system's voices can't be
  rendered ahead (they speak through the WebView).
- **The three ways to play** (`src/lib/ux.ts`, `coupler.ux`): Terminal
  (the terminal, 80 wide and as tall as the screen, at the left; on the
  right its Control Panel, `components/ControlPanel.tsx` and
  `lib/controlPanel.ts`, what shows beside it and arranged under the
  `terminal:` layout scope, `LayoutScopeContext`), Immersive
  (sound first, self-voiced) and Workshop (everything, arranged by the
  player; the default). Layouts in `uxLayout` (`App.tsx`); only
  Workshop's arrangement is kept (`KeptContext`). Immersive's director
  is `src/lib/immersive.ts`, its cues `src/lib/earcons.ts` (Web Audio,
  the CUE bus, no files; each cue's sound and caption the player's to
  change in Workshop's gear → Cues…, `components/CuesDialog.tsx`,
  `CUES`, `coupler.cues`), its voice `src/lib/voice.ts` (the system's
  speech, behind one module so a bundled engine can replace it;
  `pronounce` says keys and initialisms as words, "Esc" as "escape",
  "OOC" as "O O C", for every engine and the screen reader's feed);
  in a fight, `src/lib/fightTalk.ts` fills every silence with the
  freshest fact and says what matters at once, cutting the queue;
  `lib/output.ts` tells the prompt the game wrote on after and the
  exits' lines, which Immersive leaves out; `voice.ts`'s "Let's see…"
  (`asked`, `onThinking`) covers a reply still rendering;
  `components/ScenePanel.tsx` (Here) and `HeardPanel.tsx` (Heard) are
  its panels, `WorkshopDialog.tsx` Workshop's Customize the Screen.
  The grand goal: **fewer spoken words**; what can be a sound is one.
- **Artisan Skills** (`src/lib/artisan.ts`,
  `components/ArtisanDialog.tsx`, gear menu and Cmd+Shift+A): the
  Artisan class's skill tree and its mentor. `scripts/artisan-tree.py`
  writes `src/lib/artisanSkills.ts` from CoffeeMUD's `Artisan.java`
  (each skill's level, base stat and the skills it needs, how well;
  run it again when the snapshot changes). `artisan.ts` is pure: the
  steps (`tier`), the chart's columns, `find` (a name, ID or the game's
  word, inside a question or misspelt) and `advice`, the mentor's few
  words (what to learn first, in order, how well, the stats, the
  level). The chart is buttons on the grid with one Tab stop, its lines
  an SVG hidden from screen readers; the answer is a live region and
  Coupler's voice says it.
- **Before You Play** (`src/lib/tutorial.ts`,
  `components/TutorialDialog.tsx`): the optional tutorial, a practice
  game in Midgaard that teaches Immersive in ten lessons, never
  connecting. `Stage` simulates the rooms, body and fight and plays
  Immersive's own cues (`arrivalCues`, `bodyCues`, `fightCues`,
  exported from `lib/immersive.ts` for it); `LESSONS` is the script,
  each a lesson said and written, a scene, and one key or command to
  try. The game's lines are CoffeeMUD's own from its source. Offered
  beside the ways to play and in the gear menu while not connected.
- `src-tauri/src/cast.rs`: **the cast**, pure and unit-tested:
  every character met (NPCs from `room.mobiles`, players from
  `room.players`, anyone heard in `comm.channel`; not the player),
  keyed by the name's letters and digits as `comm.channel`'s `player`
  is, each with a voice made from the name (`auto_voice`: gender,
  pitch and speed from its words, a seed from its FNV hash) until the
  player edits it. `session.rs` keeps one per world at
  `cast/<world>.json` in app data, sends each `talk` with its speaker's
  `voice`, and emits `cast-changed` when someone new is met.
  A voice names an `engine` (`system` or a bundled one) and a
  `voiceName`, or neither (Automatic across every engine).
  `lib/voice.ts` `characterVoice` resolves it (system voices by
  gender, never the narrator's, no novelty voices unless chosen; every
  bundled voice), and one queue speaks lines from either engine;
  gear → Characters' Voices… (`components/CastDialog.tsx`, its editor
  `VoiceEditor.tsx`) lists and edits them. **The narrator** (its row
  above the list, `lib/voice.ts`, `coupler.narrator`) is the voice of
  every line given no voice: the login, the say keys' answers, the
  time of day, and who's talking (`speakTalk`: "Hassan says" in the
  narrator's voice, the quoted words in Hassan's; `splitTalk`).
- `src-tauri/src/synth.rs`: **the bundled speech engines**, for the
  characters' voices: `VOICES` (Flite's Kal and Kal 16 kHz, from
  `flite-rs`, pure Rust; and Pocket TTS's 19 English presets, each with
  its speaker's kind) and `Engines::say`, raw 16-bit audio at 22 kHz or
  more (the `voice_synth` command; `voice_engines` lists only what this
  copy has), played by `lib/voice.ts` through Web Audio at the VOX
  volume. Unit-tested; Pocket's tests skip without its files.
  **Pocket TTS** is `src-tauri/vendor/pocket-tts`, the Candle port
  vendored and trimmed (no downloader, no MKL, **no voice cloning**:
  never add the encoder back; its `README.md` lists the changes). Its
  weights and voices aren't in git: `scripts/pocket-tts-fetch.py`
  (pinned revision and SHA-256s; run once after cloning, and
  `build-release.sh` runs it) writes `src-tauri/resources/pocket-tts/`,
  shipped as the app's `Resources/pocket-tts/` (`tauri.conf.json`
  `bundle.resources`) beside the committed `CREDITS.txt`. ~250 MB.
  `src-tauri/src/stretch.rs` gives it pitch and speed (WSOLA), pure and
  unit-tested. Licenses: the data's credits reach the licenses file
  through the script's `BUNDLED_C` entries (Flite's CMU data,
  Pocket's CREDITS.txt), and both license scripts include vendored
  crates.
- `src-tauri/src/who.rs`: **who's online**, pure and unit-tested:
  CoffeeMUD has no GMCP for it, so `session.rs` sends WHO once a minute
  when `Who::due` says (in the game, at the usual prompt, no password,
  not just after a command, **never for a player idle 9 minutes**: it
  would defeat the game's idle timers) and hides its reply
  (`Who::line`, the header's bracket width telling rows), and hides the
  login and logout announcements (friends' and the LOGINS/LOGOFFS
  channels', also kept out of talk via `Who::heard`). Emits `who` with
  each change; `lib/earcons.ts` `loggedOnOff` rings for it, and
  Cmd+Shift+W (`sayWho`, `who_now`, `mud.describeWho`) says the list,
  names alone (`bare_name` takes the titles off); before any WHO was
  read it asks one at once (`who_ask`, `Who::ask_now`). `Who::gentle` is
  when a command of Coupler's own goes unnoticed: WHO, and WEATHER
  when the painter's sky is unknown (`Ambient::weather_wanted`, an area
  at most every 15 minutes, its answer hidden).
  The web build ticks it from `core.ts` (`who_tick`).
- `src-tauri/src/update.rs`: **the update check**, the version
  reading unit-tested: gear → Check for Updates Now, and Check for
  Updates Automatically (`lib/updates.ts`, `coupler.updates`, off by
  default; once a day at most, with the greeting at launch). GitHub's
  `releases/latest` for `joashchee/coupler` (prereleases never count)
  over reqwest with the system's TLS (`native-tls`); a newer version
  puts Get Coupler X… in the gear menu, which opens the release page in
  the browser by the system's opener (`open`). Desktop only. About's
  Discord and Reddit buttons open `COMMUNITY`'s fixed pages the same
  way (`community_open`; the web build's `core.ts` keeps the same two).
- `src-tauri/src/mssp.rs`: **the greeting at launch**, pure and
  unit-tested: MSSP's `PLAYERS` (how many are online) and `CODEBASE`
  (the game's CoffeeMUD version), read by a short probe
  (`session::game_status`) that never logs in. A version other than
  `BUILT_FOR` (the snapshot's, 5.11.0.4; `fit`) is a warning in the
  status bar and said (`mud.describeVersionFit`). Beside it the
  times played on this Mac (`played.json` in app data, counted when a
  character comes into the game). `launch_counts` gives both;
  `App.tsx` says them once the launch screen is gone
  (`mud.describeLaunchCounts`). Desktop only.
- `src-tauri/src/ports.rs`: `HOST`, `PORTS` and the web build's
  `web_socket_url`. `src-tauri/src/trigger.rs`: `Trigger`, `Fired` and
  how a GMCP message becomes pairs (`hooks.rs` re-exports them).
  `src-tauri/src/clock.rs`: the `Instant` `speech.rs` and `daytime.rs`
  read. All three exist so the web build can share them.
- **The web build** (coupler.ansiapps.com):
  `src-web/`, a crate that includes the pure modules above by `#[path]`
  and adds its own `session.rs` (no socket, journal, hooks database or
  pictures), `hooks.rs` (the mirror's triggers), `clock.rs`, `synth.rs`
  (no bundled voices) and `ffi.rs` (a C-style interface, no
  wasm-bindgen). `src/web/` stands in for Tauri's modules in Vite's
  `--mode web` (`vite.config.ts` aliases): `core.ts` loads the module,
  owns the WebSocket, keeps maps and casts in IndexedDB and answers the
  desktop's commands by name. `src/lib/web.ts` (`WEB`, `DESKTOP_ONLY`)
  leaves out the journal, log, pictures, hooks list and Characters'
  Voices. `src-tauri/examples/web_mirror.rs`, the maintainer's tool,
  makes the web's hooks and assets mirror this Mac's Coupler
  (`scripts/web-publish.sh`). Anything new on the desktop either works
  in `src-web` too or joins `DESKTOP_ONLY`, with a row in the web build's table (the private notes).
- `src-tauri/src/session.rs`: the socket, the reader thread,
  the events (`mud-output`, `mud-echo`, `mud-gmcp`, `mud-closed`,
  `character-left`, `creation`, `vitals`, `talk`, `journal-changed`, `cast-changed`, `map-changed`, `map-walk`, `hooks-changed`, `hook-fired`, `ambient`, `combat`, `picture`, `who`). It feeds the mapper, saves one map per
  world to `maps/<world>.json` in app data, and walks a route one step
  at a time, each sent only after the game confirms the last.
- `src-tauri/src/lib.rs`: the commands (`server_info`, `mud_connect`
  (takes a port ID), `launch_counts` (takes a port ID), `update_check`,
  `update_open`, `community_open`, `mud_send`, `mud_disconnect`, `mud_resize`,
  `map_snapshot`, `map_find`, `map_directions`, `map_walk`, `map_stop`,
  `map_set_landmark`, `map_clear`, `who_now`, `who_ask`, `echo_list`, `cast_list`, `cast_set_voice`,
  `cast_reset`, `cast_forget`, `journal_list`, `journal_last`, `journal_unheard`,
  `journal_unheard_entries`, `journal_set_heard`, `journal_heard_all`,
  `voice_engines`, `voice_synth`, `voice_prerender`, `hooks_count`, `hooks_list`, `hooks_set_trigger`, `ambience_now`, `picture_now`, `picture_again`, `picture_forget_looks`, `picture_paint`, `assets_list`,
  `assets_import`, `assets_compress`, `assets_keep`, `asset_set_font`,
  `asset_create_art`, `asset_create_music`, `asset_audio`, `asset_picture`, `asset_ansi`,
  `music_open`, `music_new`, `music_save`, `music_save_as`, `music_render`,
  `music_instruments`, `backup_export`, `backup_inspect`, `backup_restore`,
  `app_restart`, `window_set_fullscreen`,
  `window_is_fullscreen`).
- `src/lib/mud.ts`: typed wrappers over the commands and events, and the
  palette. `src/components/Terminal.tsx`: the output view (capped
  scrollback, follows the bottom, measures cells for NAWS).
  `src/components/MapPanel.tsx`: where you are, the picture (fixed
  size, hidden from screen readers, its facts in words beside it; each
  room a tile in its terrain's color from `src/lib/terrain.ts`, the
  table of all 24 terrains, with a key of those in view), then
  landmark and find a room. `src/lib/layout.ts` and
  `src/components/Movable.tsx`: **everything on the screen goes in a
  `Movable`** on the desk (`.desk`, the whole stage). In Workshop, with
  gear → Move and resize everything (`coupler.arrange`, off by default), its top left corner
  moves it and its bottom right corner resizes it (not the game
  output: 80 wide, and 25 tall but in Terminal); the corners are laid over it, never drawn,
  and take no room. Its place, size and order are kept
  (`coupler.layout`). The default layout is `defaultLayout` in
  `App.tsx`, per theme, in stage pixels; anything new on the screen
  gets an entry there and a `Movable`. Gear → Reset to Default Layout
  forgets the arrangement; dev-only Copy the Layout exports it.
  `src/lib/stage.ts` and
  `src/components/ScreenReadout.tsx`: the fixed screen and the status
  bar's size and pointer readout. `src/components/HooksDialog.tsx`: the
  hooks list behind the status bar's total, each row a summary and an
  Edit button that opens `HookEditor.tsx` over it. `src/App.tsx`: the
  screen, the ways to play, input line, history, password mode, the
  keyboard shortcuts (`SHORTCUTS`, shown in gear → Keyboard).
- Shared with the other ansiapps apps and kept identical: `ProgressBar`,
  `ActivityStatus`, `lib/activity.ts`, `lib/estimate.ts`,
  `StartupScreen`, the gear menu, `AppTesting`, the tokens in
  `index.css`, `ansiapps-theme.css`'s shared part.
- `Dialog` was shared too; Coupler's now adds `inert` while closed,
  focus in and back, and Esc (rule 10), and one can open over another
  (`covered`; Esc closes only the top). Port that to the sister apps
  rather than reverting it here.
- Dev server port **1460** (Diskette 1420, Floppy 1430, Stylus 1450);
  the web build's **1462**.

## Testing

- Before calling a change done: `cargo test --manifest-path
  src-tauri/Cargo.toml`, `cargo clippy --manifest-path
  src-tauri/Cargo.toml --all-targets`, `npx tsc --noEmit`. When
  Neumetik changed, also `cargo test --manifest-path
  src-tauri/vendor/neumetik/Cargo.toml` (a dependency's tests don't run
  with Coupler's). When a
  shared module or `src-web/` changed, also `cargo test` and `cargo
  clippy --all-targets` in `src-web/`, and `npm run build:web` (it runs
  `src-web/tests/core.mjs`).
- **App Testing checklist** (dev-only, gear menu): `src/lib/testChecklist.ts`.
  Add items when a feature lands, drop them once confirmed. Production
  builds must not contain it: grep `dist/assets/*.js` for "App Testing".
- Testing against the live game means a real character on a shared
  server: don't script floods of commands at coffeemud.net. For protocol
  work, run a local CoffeeMUD (Java, Apache-2.0) and point a dev-only
  build at it by editing `HOST` locally, never committed.

## Build

- **"Build the macOS app" means the `.app` only**:
  `scripts/build-release.sh --bundles app`. DMG only when asked. The
  script builds universal (Apple Silicon and Intel), signs with the
  maintainer's Developer ID and the hardened runtime, notarizes with
  notarytool's keychain profile (`coupler`, or
  `COUPLER_NOTARY_PROFILE`) and staples, so a download opens on any
  Mac; it strips `$HOME` paths, fails if any survive or if any of
  universal, signed, notarized or Gatekeeper's yes is missing, zips the
  app for the release (`Coupler_<version>_universal.app.zip`, beside
  the app) and installs to `~/Applications/Coupler.app` (skipped with
  `CI` or `COUPLER_NO_INSTALL`). The one-time setup is in its header;
  `COUPLER_UNSIGNED=1` tries a build without it, never for a release.
  No Apple credential is ever in the repo or the environment. Never a
  bare `tauri build`.
- **Windows and Linux**: GitHub Actions, `.github/workflows/windows.yml`
  (NSIS and MSI) and `linux.yml` (.deb and .rpm; never an AppImage, it
  bundles LGPL WebKitGTK), by Run workflow or a `v*` tag, each run's
  artifact, unsigned. Every successful run on `main` or a `v*` tag is
  published to a release by `scripts/publish-release.sh`: the version's
  own (`v` + `tauri.conf.json`'s version, made at that commit if
  missing), or
  the `dev` prerelease once that tag marks an earlier commit. The Mac's
  notarized `.app.zip` is uploaded to the version's release by hand.
  **Releasing is pushing the tag**: a push to a branch runs a workflow
  only when its own file changed (the path filter), so a version bump
  on `main` builds nothing. Bump, commit, push, then push `v<version>`
  on that commit (`git tag -a v0.27.0 -m ...`); both workflows build
  and publish the release, and the Mac's zip joins it.
- **The web build**: `npm run build:web` (`scripts/web-build.sh`) makes
  `dist-web/`; `scripts/web-publish.sh` mirrors this Mac's hooks and
  assets, builds, and uploads to Cloudflare Pages after a yes. Only when
  asked: it publishes the mirror to anyone.

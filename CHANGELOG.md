# Changelog

## Unreleased

- **Priority Audio** (2026-10-06): Workshop's gear → Priority Audio… is
  a list of the cues and speech that never wait behind the speech queue
  (`lib/priority.ts`): any cue but the heartbeat, tells, the group, a
  say, the time of day, a long look, a command's answer, or a game line
  holding the player's words. If something's being said, it pauses, the
  priority bell rings (a bell, a chime, ding-dong or a ping; its volume,
  pitch and caption a cue in Cues), the priority audio plays, and the
  queue picks up: a bundled voice where it stopped, the system's from
  the line's start (`voice.ts`'s `interrupt`, `LineInfo.priority`).
  Starts with tells and a fight starting.

## 0.28.0 (2026-10-06)

- **The account menu is a dialog** (2026-10-06): `account.rs` reads
  CoffeeMUD's account menu (`CharCreation.java`'s `acctmenu*`, the
  Account command's list) and asks `L` once so the characters are
  known; `AccountMenuDialog.tsx` shows each as a button to play them,
  New Character, Retire, Quit (asked in words, the game's y/N answered),
  and Change Password, E-mail, Import and Export on the command line.
  The narrator says the menu in a few words, not its screen of letters.
- **All the talk, as it was seen** (2026-10-06): a say's GMCP is sent
  before a mood rewrites it or a language scrambles it, so its printed
  line is found by its speaker and kept instead (`speech.rs` `heard`);
  a known language's "(translated from …)" line is kept as the line;
  WHISPER and a yell heard from the next room, which have no GMCP, are
  read from the text. Say, yell, shout and ask were already captured.
- **A channel's LAST from the log** (2026-10-06): `OOC LAST 10` (or
  `GOSSIP LAST`, `ooc prev 5`) isn't sent: the channel's newest lines in
  Coupler's log are written in the output, said, and marked heard
  (`journal.rs` `last`). The stock channels and any heard; SAY LAST
  still goes to the game. Desktop only.
- **Terminal's Control Panel** (2026-10-06): the game output sits against
  the left edge, and the right of the screen is the Control Panel:
  checkboxes for the map, You, Picture, Heard and the Journal, Log and
  Hooks buttons, help at its top, and Arrange to move and resize them
  (dragging or the arrow keys), kept apart from Workshop's arrangement.

- **CoffeeMUD's source brought up to date** (2026-10-06):
  `reference/CoffeeMud/` is now `master` at `720aec6` (2026-10-03),
  still CoffeeMUD 5.11.0.4, so `BUILT_FOR` stands. The 15 commits since
  `c1e556f` touch items, quests, a new common skill (Snake Oil Selling)
  and the channel backlog's settings; nothing Coupler reads (the
  protocols, commands, socials, races, character creation, Artisan), so
  the studies and the files made from the source stand.

- **The narrator, more at hand** (2026-10-06):
  - Connecting in Immersive, the narrator says "Connecting…", then
    nothing of the intro until the game's welcome (said), or its first
    question if that comes first.
  - Typing a command where nothing takes it (focus on the output or a
    button) says "Press escape to return to the command line", at most
    every 5 seconds.
  - "Let's see…": when a reply to a command or a say key has to be
    rendered in a built-in voice (not in the voice cache), the narrator
    says it at once and the status bar shows it while the render runs.
    The narrator's own is rendered at launch and kept, so it never waits.
  - The say keys answer in the narrator's voice in Workshop too, whatever
    its voice layer (Terminal still leaves it to the screen reader).
  - "CoffeeMUD closed the connection." is said in Immersive and Workshop
    when the game hangs up, also when a QUIT ends with a reset (it was
    "the connection dropped", with the system's error).
  - Immersive leaves the player's prompt out also where the game wrote
    on after it on the same line, and never says it; and the room's
    exits lines (`[Exits: …]`, `Obvious exits:`) are left out, the cues
    playing them.
  - Fight talk says the opponent's exact hit points ("Rat at 37 of 90")
    when MSDP sends them, the percentage otherwise; it fills a silence
    after a quarter second (was 0.9).
  - Cmd+Shift+W before any WHO was read asks WHO at once (its reply
    hidden) and says the list. WHO's prompt counting starts with the
    character, so the account menu's prompt no longer holds back the
    background WHO.
  - About links Coupler's Discord and r/coupler_app on Reddit, opened in
    the browser by fixed address.

- **Accessibility, for every player** (2026-10-06): the README,
  `CLAUDE.md`, `CONTRIBUTING.md` and `docs/accessibility.md` no longer
  speak of blind and visually impaired players alone. Accessible first
  now means the most immersive game for everyone, sighted or not:
  heard as well as read, what serves a screen reader making it richer
  for all.

- **The Mac app opens on any Mac** (2026-10-06):
  `scripts/build-release.sh` builds it universal (Apple Silicon and
  Intel), signs it with the Developer ID and the hardened runtime, has
  Apple notarize it and staples the ticket, then checks each of those
  and Gatekeeper's verdict before zipping it for the release. Before,
  the app was Apple Silicon only and signed ad hoc, so Gatekeeper
  blocked a download as damaged. The Apple credentials stay in the
  keychain (a notarytool profile); `COUPLER_UNSIGNED=1` builds without
  them, for trying only.

- **The licenses file stops changing by itself** (2026-10-06):
  `scripts/third-party-licenses.py` walked the crates in a set's order,
  which Python changes each run, so a license text many crates share
  was written from a different crate's copy each build. Now in name
  order: the same build writes the same file.

## 0.27.0 (2026-10-06)

- **The composer learns a generator's controls** (2026-10-06): Create
  Asset → BGM gets twelve menus, each Automatic (the words, then the
  mood) or a choice: key, scale (13, from major to double harmonic),
  length (4 to 32 bars), a loop or a piece that ends on home, an arc
  (rise, fade away, arch, two swells), the parts (a bed with no tune,
  bass and drums, the tune and a second line...), brightness, drive
  and tension leaning on the mood, swing, timing off the grid, and
  drum fills. The words understand them too ("in D minor", "dorian",
  "16 bars", "with an ending", "rising", "swing", "no drums", "duet").
  New: a second line written against the tune by counterpoint rules, a
  bass that follows the kick, eight fills, a jazz kit and half-time
  drums, sevenths, and eight moods (boss, court, chiptune, celtic,
  blues with its twelve bars, jazz, suspense, cave). Every piece lists
  how it was written, a sentence a decision, under it. The files now
  carry their time and key signatures, so the Music Editor shows a
  jig's bars in 6/8. Modeled on notebin.fm's generator, studied for its
  features; its code isn't published, and none of it is used.
- **GitHub Actions stays free** (2026-10-06): a convention in
  `CLAUDE.md` and `CONTRIBUTING.md`: workflows only in this public
  repository, on standard GitHub-hosted runners, no larger or
  self-hosted runners, paid actions or raised cache. Both workflows
  already qualify.
- **Check for updates** (2026-10-06): gear → Check for Updates Now,
  and Check for updates automatically (off until turned on; then once a
  day at most, with the greeting at launch). Coupler asks GitHub for
  its latest release (`update.rs`, one GET, only Coupler's version in
  the User-Agent; prereleases never count), says what it found in the
  status bar and by the narrator, and Get Coupler X… in the gear menu
  opens the release's page. Nothing is downloaded or installed. The one
  exception to talking only to CoffeeMUD, recorded in rules 1 and 2.
  reqwest (already in the tree) with the system's own TLS.
- **Every successful Windows and Linux build is published to a
  release** (2026-10-06): a `publish` job in both workflows, on `main`
  and `v*` tags, runs `scripts/publish-release.sh`: the version's own
  release (made at that commit if missing, its notes this file's
  section) or, once the version's tag marks an earlier commit, the
  `dev` prerelease, moved along. The two workflows share a concurrency
  group so they don't race.
- **`.cargo/config.toml` moved to the repo's root** (2026-10-06), so
  `cargo test --manifest-path src-tauri/Cargo.toml` from the root finds
  its CMake policy line for WavPack too.
- **Windows and Linux builds on GitHub Actions** (2026-10-05):
  `.github/workflows/windows.yml` builds the NSIS installer and the MSI
  on `windows-latest`, and `.github/workflows/linux.yml` the .deb and
  .rpm on Ubuntu 22.04 (no AppImage: it would bundle WebKitGTK and GTK,
  LGPL). Each runs by hand, on a `v*` tag, or when its workflow
  changes, fetching Pocket TTS's files and writing the licenses file
  for what that build links, as `build-release.sh` does on the Mac.
  Unsigned for now. `third-party-licenses.py` reads and writes UTF-8
  everywhere (Windows' Python defaulted to cp1252).
- **Ready for a public repository** (2026-10-05): the maintainer's
  planning and research docs moved to a private notes repository
  (linked back in as a gitignored `CLAUDE.local.md`), and every comment
  that cited them now says what it meant. A README for players and
  contributors, `CONTRIBUTING.md` (the rules a change keeps, the
  checks, sign-off by the Developer Certificate of Origin),
  `SECURITY.md`, and issue and pull request templates. Two comments
  that still named a private notes file by name (`telnet.rs`,
  `index.css`) and a test character named after the maintainer
  (`journal.rs`) fixed (2026-10-06).
- **Neumetik is its own crate** (2026-10-05): moved from
  `src-tauri/src/neumetik/` to `src-tauri/vendor/neumetik/`, standard
  library only. The original is now ansiapps' private `neumetik`
  repository, proprietary so ansiapps' closed-source apps can use it;
  Coupler's copy stays Apache-2.0 and is updated by
  `scripts/neumetik-sync.sh`. The license scripts treat it as
  Coupler's own. Patches and kits named after makers' trademarks have
  generic names: Ladder Lead and Ladder Bass (Moog), Chorus Strings and
  Pulse Sub Bass (Juno), Warm Poly Pad and Poly Arp (Jupiter); the kits
  Analog Boom (TR-808), Rhythm Box (CR-78), Analog Tick (TR-606),
  Digital Machine (TR-707), Analog Punch (TR-909), Studio Machine
  (LinnDrum) and Hex Pads (Simmons). Create Asset's dance music says
  "an analog drum machine".
- **Coupler Backup** (2026-10-04): gear → Export Coupler Backup… writes
  everything Coupler keeps to one `.coupler` file: the settings, the
  Assets folder, the hooks, the maps, the casts and their voices, the
  rooms' looks, the journal and the log, the times played
  (`src-tauri/src/backup.rs`). As small as it can be made: the settings
  and small files together in Brotli's strongest setting and largest
  window, files of a type side by side; sounds and SoundFonts each
  Brotli too; what's compressed already (Ogg, Opus, MP3, WavPack, PNG,
  JPEG) stored as it is. The databases are read through `VACUUM INTO`,
  consistent and compacted, even mid-game. Not the voice cache (made
  again) nor the first-run answer. Gear → Restore Coupler Backup…, or a
  backup dropped on the window, asks first (what's in it, when, by which
  Coupler), checks every block's CRC while unpacking it beside the data,
  and starts Coupler again; the backup takes the place of what was there
  before anything is opened (what's replaced is kept aside until the next
  restore), and the settings go back into the WebView before the page
  runs. Not while connected. Desktop only.
- **Create Asset's music loops** (2026-10-04): Play plays it over and
  over, an exact loop, until Stop (or closing).
- **The Music Editor** (2026-10-04): Workshop's new BGM button, beside
  the Mixer. Opens a MIDI file from the Assets folder (or New: a piano
  and the drums, four bars) into a pattern (a row a part, a column a
  bar, how full each is; copy, paste and clear a bar) and a piano roll
  (a row a key, or a drum by name on the drums; a column a sixteenth;
  notes put in, taken out, made longer, shorter, louder, softer). Tempo,
  beats a bar, the number of bars, each part's name and instrument
  (Neumetik's banks) or drum kit; parts and drums added and removed.
  Play Bar and Play Song loop until Stop, through Neumetik unless a
  SoundFont is picked beside them (Plays through). Save… writes over the file
  after a yes; Save as… makes a new one in the Assets folder. Every
  controller, bend and other event the editor doesn't show is kept and
  written back (`src-tauri/src/music.rs`). Desktop only.

## 0.26.0 (2026-10-04)

- **Try every asset** (2026-10-04): the Hooks dialog's Assets tab ends
  each row with Play, which turns to Stop while it plays (one asset at
  a time; closing the dialog stops it), or Show… for a picture, opened
  over the list with its kind and size in words. A SoundFont's Play
  plays a sample tune through it.
- **MIDI plays through Neumetik by default** (2026-10-04), even with
  SoundFonts in the Assets folder. Each MIDI file's row chooses what
  plays it: Neumetik, or one of the player's SoundFonts (kept in
  `.soundfonts.json` in Assets). If that SoundFont leaves the folder,
  the file plays through Neumetik again.
- **Create Asset** (2026-10-04), beside Add Assets: an asset made on
  this computer from a few words. ART is a picture by Coupler's painter
  (`src-tauri/src/create.rs`: a place, the weather, the time of day and
  "epic" read from the words), saved as ANSI art at 40 by 12, 60 by 18
  or 80 by 25. BGM is a short loop by **Coupler's composer**
  (`src-tauri/src/compose.rs`): fourteen moods (peaceful, merry, dark,
  fierce, heroic, mysterious, sad, seafaring, woodland, desert, wintry,
  nighttime, electronic, holy), each with its mode, tempo, meter,
  instruments and drums; an instrument named leads; slow, fast, soft,
  loud, major, minor, waltz, jig, march and a tempo ("90 bpm") change
  it. Eight bars of chords, bass, pad, tune and broken chord, written as
  MIDI that loops with no gap. The same words make the same asset;
  again, another take. What was made is said in words.

## 0.25.0 (2026-10-04)

- **Neumetik, Coupler's own synthesizer** (2026-10-04,
  `src-tauri/src/neumetik/`): MIDI music now plays without a SoundFont.
  General MIDI's 128 instruments and its drum map, Roland GS's variation
  tones (the SC-55's, by Bank Select, falling back to GM's as the SC-55
  did), GS's drum kits and the classic drum machines (TR-808, TR-909,
  CR-78, TR-606, TR-707, LinnDrum, Simmons), all made by three small
  engines (analog, FM, plucked string) with no samples, so nothing to
  download, rendered many times faster than real time. GM and GS resets, GS's drum part on any channel,
  sustain, pitch bend range, mod wheel, brightness and resonance, reverb
  and chorus. Three banks of the synth sounds everyone knows, rebuilt:
  bank 80 the eighties (the DX7's piano and bass, the Prophet's sync,
  OB-Xa brass, Juno strings, the 303, the D-50's Fantasia, the M1's
  house piano and organ...), 81 the nineties (the hoover, the supersaw,
  the Reese, Mentasm, trance plucks, the 808 boom, G-funk's whistle...),
  82 the 2000s to now (the wobble, growls, future bass, trap's 808,
  synthwave, lo-fi keys, chiptune...). A SoundFont the player adds is
  still used instead.

## 0.24.0 (2026-10-04)

- **All ART is Coupler's painter's first** (2026-10-04), a rule now
  (CLAUDE.md rule 11): every picture Coupler makes or shows on its own
  is drawn by default by its ANSI painter, and any other source is the
  player's choice. Room pictures already were. The guide to making a
  character now paints its own portraits (`src-tauri/src/portrait.rs`,
  `portrait_paint`, the web build too): each race at its size beside a
  stick marked every foot up to six, with its skin and hair, pointed
  ears, a dwarf's beard and helmet, a gnome's tall hat, a halfling's
  bare feet, a half ogre's tusks, and red eyes for those that see in
  the dark; each class as a person in its clothes with its tools
  (armor, sword and shield; a robe, pointed hat and glowing staff; a
  hood and dagger; a holy symbol and mace; leaves; a lute; a monk's
  sash; an apron and hammer), CoffeeMUD's 49 classes each in their own
  colors. CoffeeMUD's own pictures are a choice in gear → Pictures…
  (Race and class portraits, `coupler.pictures`'s `portraits`).

- **Sixteen fixes from playing** (2026-10-04):
  - **Spoken right**: Coupler's voice and the screen reader's feed say
    keys as words ("Esc" is "escape", "Cmd+Shift+L" is "command shift
    L", "Ctrl" is "control") and spell out initialisms a voice would
    say as a word ("OOC" is "O O C", not "ock"; also IC, AFK, PK, PVP,
    NPC, XP, HP). Only what's said changes (`voice.pronounce`).
  - **The tutorial's refusals** ("That way leads out of this practice
    game", "You can't go that way", a closed door, "Huh?!") play the
    problem buzz and are said, as well as written.
  - **Fight talk** (`lib/fightTalk.ts`): in a fight, Immersive's
    narrator never leaves a silence. When nothing's said for a moment
    it says the freshest fact: the opponent's health and its trend
    ("Rat at 40 percent, falling fast."), yours, the newest line of the
    fight, or by turns both healths, the range and who it is. What
    matters cuts the queue and is said at once: the fight starting and
    ending, a death or someone fleeing, a blow of 15 percent or more,
    health under 30 and 15 percent. The tutorial's fight too, waiting
    behind its lesson instead of cutting it.
  - **Making a character**: a mode, Coupler's guide or CoffeeMUD's own
    way (`coupler.creationMode`; the guide's Standard Mode (CoffeeMUD)
    button and the gear menu's pair of choices). Races and classes show
    CoffeeMUD's own portraits as ANSI art in the 16 VGA colors, beside
    the list (`scripts/creation-art.py` makes `public/creation-art.json`
    from its web images), and each player race its facts in few words
    (stats, senses, gifts, height, lifespan, what it knows) and what
    sets it apart from the others ("the most Constitution", "the
    longest lived"), from its Java; the narrator says them as each is
    reached. Moving through alignment or inclination says what each
    does, in few words, from CoffeeMUD's factions. **The guide no
    longer vanishes at the stats**: the stats question came back so
    often it passed for the usual prompt, so the background WHO went
    out mid-creation, the game took it for an answer and the guide
    closed. WHO now waits while a character's made, and the creation's
    prompts don't count toward the usual one (`Who::creating`).
  - **Lone lines said**: in Immersive, a read that's a single line of
    the game's ("A rat arrives from the north.") with no hook's sound
    for it is said by the narrator.
  - **The Log's channel tabs**: Every Channel, then a tab a channel,
    most lines first, each with its unheard count; Left and Right move
    between them.
  - **The painter's sky**: indoors and underwater show nothing of it
    now (a wooden wall's window has its shutters closed; no sun rays
    under the water). Outdoors, the weather shows once it's known, and
    Coupler now asks: on coming under the sky in an area whose weather
    it hasn't read, it sends WEATHER when it's gentle to (as WHO:
    never while typing, making a character, a password, or for a
    player away 9 minutes; each area at most every 15 minutes) and
    hides the answer (`Ambient::weather_wanted`, `Who::gentle`).
  - **Who's online by name**: Cmd+Shift+W says the names alone; a
    title without a comma ("Bob the Brave", "Lord Bob of Midgaard",
    "Sous-Chef de Cuisine Bob") no longer gives the title's word as the
    name.
  - **Try It** beside Edit… in Characters' Voices, for each character
    and the narrator.

- **A warning when the game's CoffeeMUD isn't Coupler's** (2026-10-04):
  the greeting at launch already asks the game by MSSP how many are
  online; it now also reads `CODEBASE` ("CoffeeMUD v5.11.0.4") and
  compares it with the version of the source Coupler was made from
  (5.11.0.4, snapshot `c1e556f`; `mssp::BUILT_FOR`). Another version,
  newer or older, shows a warning in the status bar with both numbers
  (an alert, so VoiceOver says it), and the narrator says it after the
  greeting in Immersive. The same version, or a game that doesn't say,
  shows nothing. Desktop only, as the greeting.

## 0.23.0 (2026-10-04)

- **Artisan Skills and the mentor** (2026-10-04): the Artisan's skill
  tree, all 150 skills, is hard to take in from QUALIFY. Gear → Artisan
  Skills and Mentor… (Cmd+Shift+A) shows it as a chart: a column a step
  from the start (step 1 needs nothing), the lines between each skill
  and what it needs. Choosing a skill marks what it needs (◄) and what
  it opens up (►), and draws only their lines. The mentor answers in a
  few words for anyone who'd rather not read a chart: name a skill, or
  ask "how do I get blacksmithing?" (a typo or the game's own word
  works too), and it says what to learn first, in order and how well,
  the base stats and the level ("First Fire Building and Mining to
  75%. Then Smelting to 75%. Then gain Blacksmithing."). A screen
  reader reads each answer, and Coupler's voice says it when it's on.
  `scripts/artisan-tree.py` makes `src/lib/artisanSkills.ts` from
  CoffeeMUD's `Artisan.java`; `lib/artisan.ts`,
  `components/ArtisanDialog.tsx`. The web build too.

## 0.22.0 (2026-10-04)

- **Making a character, guided** (2026-10-04): CoffeeMUD's character
  creation asks each question after screens of text. Now, as soon as
  the game asks about a new account or character, a guide opens over
  every way to play and takes it a question at a time: the steps
  (Account, Name, Race, Gender, Stats, Class, Beliefs, Begin; World
  where a game has themes) with the current one marked, what's settled
  so far, the question in a few plain words with a line of help, and
  the answers as buttons, each with the game's own line about it
  (races, classes, themes and stats from the intro the game printed
  once). The classes that suit the character's best stat, which the
  game shows only by color, say so in words. The stats are a table with
  Raise and Lower per stat, the points left and the classes they'd
  qualify for; a random roll and Done are buttons. Passwords go in a
  hidden field and are written as stars, never kept; the command line
  hides the account's password too, which CoffeeMUD itself echoes. A
  problem with the
  last answer is shown first ("The game says: …"). The game's full text
  is a button away, and the terminal keeps all of it. In Immersive the
  narrator says the question and how many answers instead of reading
  every line, then each answer as the keyboard reaches it, and after a
  point spent only what changed; two new cues (Create: a page turn for
  the next question, a fanfare as the new character comes into the
  game) in Cues. Type Instead or Esc hides the guide, Cmd+Shift+G shows
  it again. `src-tauri/src/creation.rs` (every question CoffeeMUD's
  own, from `CharCreation.java`; unit-tested), `lib/creation.ts`,
  `components/CreationDialog.tsx`. The web build too.

## 0.21.0 (2026-10-04)

- **Before You Play, a tutorial** (2026-10-04): an optional practice
  game that teaches Immersive before the real one, from a Before You
  Play button beside the ways to play and the gear menu (while not
  connected). Ten short lessons in CoffeeMUD's Midgaard: the quiet key;
  the Temple of Mota and its exits as notes; walking north and back
  (footstep, new room, explored and new exits); down to the Temple
  Square (a closed door's knock) and the Market Square (a landmark's
  bell); the body's sounds and Cmd+Shift+V; a fight with the beastly
  fido and Cmd+Shift+E; talk (a say, a tell's ping, an OOC blip) and
  answering with say; the narrator's dusk line and its short answers to
  sit and stand; reading back with Option+Up. Each lesson teaches in the
  narrator's voice, plays its scene, then asks for one key or command,
  and the practice game answers as CoffeeMUD would: its own prompt,
  starting vitals, damage words, death and experience lines, says,
  tells and channels, from its source. Nothing connects. The sounds are
  Immersive's own (`arrivalCues`, `bodyCues` and `fightCues`, now
  shared from `lib/immersive.ts`), the say keys answer in the game's
  sentences, and everything is written too: the lesson, the practice
  output in the game's colors, and a Heard list of each sound's caption
  and each line said. The last lesson offers Play in Immersive.
  `lib/tutorial.ts`, `components/TutorialDialog.tsx`. The web build too.

## 0.20.0 (2026-10-04)

- **Hidden details said in words** (2026-10-04): CoffeeMUD's long look
  at a room (LL, LONGLOOK, EXAMINE alone) marks the scenery you can look
  at or get only by color: the words of each hidden item's name in the
  description, in HIGHLIGHT. Blind players never heard them. Now, after
  a long look, `src-tauri/src/hidden.rs` finds the room's description
  (by the current room's name from `room.info`) and the words in a color
  not its own, joined when side by side ("marble fountain"), each named
  once, and sends them with `mud-output` as `hidden`. In Immersive the
  narrator says "Hidden details found.", the description, then
  "Hidden: marble fountain and statue." ("No hidden details." after the
  description when there are none); in Terminal and Workshop the screen
  reader hears "Hidden details: …" after the room. The web build too.

## 0.19.0 (2026-10-04)

- **The narrator says the commands' answers** (2026-10-04): a study of
  all 310 of CoffeeMUD's commands (docs/coffeemud-commands.md) sorts
  them into those that print a block (lists, tables, boxes, columns, a
  room, help text, a menu; 115 for players, sorted by how many lines,
  TOPICS's ~700 first) and those that answer in one line (110). In
  Immersive, a one-line answer isn't written in the game output; the
  narrator says it in short ("You sit down and take a rest." is
  "Sitting.", "You get a sword from a chest." is "Got a sword."): 210
  answers to 109 commands, wrapped lines joined back up, GET ALL a line
  each, and any other lone answer said as the game wrote it. Talk stays
  the journal's. `src-tauri/src/echo.rs`, a new line kind `echo` (its
  own row in gear → Speech… for the screen reader). **Narrator's
  Answers…**, Workshop's gear menu: every answer with its command, the
  game's line and the narrator's words, a search, Edit to change the
  words or say nothing, Hear It, Back to Coupler's Own, Reset All
  Answers, and one checkbox to turn it off. Kept as `coupler.echoes`.
  On the web too.

## 0.18.0 (2026-10-04)

- **Immersive has no prompt to look at** (2026-10-04): once a
  character is in the game, the game's prompt (`<100hp 50m 80mv>`)
  isn't written in the game output, unfinished or finished, nor met in
  review mode. A question waiting for an answer still shows (ending in
  `?` or `:`, a y/n, the pager's `<pause - enter>`), and so does every
  line of the login. Terminal and Workshop show it as always.
- **Cues…** (2026-10-04), Workshop's gear menu and Customize the
  Screen: every cue Coupler makes, 22 of them in five groups, each
  with its sound on or off, its volume (a share of CUE) and pitch, and
  its caption (the visual cue in Heard) on or off and in the player's
  own words. Play It plays one with made-up details, even while it's
  off; Back to Coupler's Own and Reset All Cues undo. Kept as
  `coupler.cues`. Captions now read as words then detail (`Exits:
  north, east (new)`, `A heartbeat: fast, health under a quarter`,
  `Logged on: Bob`), and the log's blip has a caption.
- **The log's blip has a cooldown** (2026-10-04): at most one blip
  (and caption) every 15 seconds, so a busy channel isn't a stream of
  them; with the Log open, every line blips. If a line came in without
  a blip and the log then goes 10 minutes unnoticed (no blip heard, the
  Log not opened or played, no Cmd+Shift+U), **a reminder** plays: three
  falling blips, captioned `Unread in the log: N lines`, and the
  narrator says the count with the voice on. It doesn't remind again
  until another blip is held back. A new cue, The log reminds.

## 0.17.0 (2026-10-04)

- **Who's online** (2026-10-04), built, not yet run against the live
  game (`who.rs`, shared with the web build).
  - **WHO, asked quietly once a minute**, and its reply (with the
    prompt line it finishes) taken out of the game output; a WHO the
    player types shows as always and updates the list too. It asks
    only with a character in the game, at the player's usual prompt
    (never into a question, menu or editor), never while a password is
    typed, not within 2 s of a command, and **not once the player has
    been idle 9 minutes**: every line sent resets CoffeeMUD's idle
    clock (away at 10 minutes, logged out at 90), so asking for an
    away player would keep them "here" forever.
  - **Logins and logouts are a sound, not a line**: the game's
    announcements (a friend's `Bob has logged on.`/`off.`, and the
    LOGINS and LOGOFFS channels' `[CLANTALK] 'Bob has logged on.'`/
    `logged out`) aren't written, spoken, or kept in the log; a muted
    doorbell rises for someone logging on and falls for someone logging
    off (`earcons.loggedOnOff`), captioned with the name, in every way
    to play. Someone who came or went between two WHOs rings it too.
    The same login heard twice (a friend's notice and a channel's)
    rings once.
  - **Cmd+Shift+W says who's online** in the narrator's voice (and the
    status bar): "3 others online: Ann, Bo (idle) and Cy. Last, Dee
    logged off 2 minutes ago."

## 0.16.0 (2026-10-04)

- **The greeting, the narrator, and a sky that keeps time**
  (2026-10-04), built, not yet run against the live game.
  - **At launch** the status bar (and in Immersive the narrator) says
    "You've played CoffeeMUD 12 times with Coupler. 37 players are
    online now." The times are this Mac's own (`played.json` in app
    data, one each time a character comes into the game): with no
    server of ours (rule 1) Coupler can't count anyone else's. The
    players online are the game's own figure, asked by MSSP
    (`mssp.rs`): a short connection that never logs in, takes MSSP,
    refuses everything else and hangs up once the table comes.
  - **A narrator**: the voice of everything Coupler says itself, set in
    Characters' Voices (any voice of any engine, its pitch and speed;
    the system's own until then). Talk is introduced by it: "Hassan
    says", then Hassan's own voice says the words (`speakTalk`, the
    journal's lines played back too). Automatic never gives a character
    the narrator's voice. The voice cache renders only the words now.
  - **The time of day, narrated**: the change lines at dawn, dusk and
    night and everything TIME says (the hour, the date, the season,
    the moon) are a new kind of line, `time` (`daytime::about_time`,
    `speech.rs`). In Immersive the narrator says them and the game
    output leaves them out; the Speech dialog has them as their own
    kind for the screen reader.
  - **The sun and the moon cross the sky by the clock**: left to right,
    low at rising, highest halfway, low at setting; the moon the same
    by night (`Scene::arc` from `Daytime::arc`: the hour, the day's
    length from TIME, and quarters of the hour once two new hours show
    how long one lasts). Every room's sky agrees, and each room's land
    keeps its look.

- **Coupler on the web** (2026-10-04), built,
  not yet deployed or tried against the live game: the same app in a
  browser, for coupler.ansiapps.com on Cloudflare Pages, made as slim as
  it can be (a 150 KB gzipped WebAssembly core, no wasm-bindgen).
  - It plays through CoffeeMUD's own `/WebSock`, which carries the raw
    telnet stream, so there's no relay of ours (rule 1) and the page's
    CSP allows coffeemud.net's WebSocket alone (rule 2).
  - The desktop's own Rust (`src-web/` includes the pure modules), the
    desktop's own UI (`src/web/` stands in for Tauri). Maps and the
    cast are kept in the browser. Left out: the journal and the log,
    room pictures, the hooks list and asset import, Characters' Voices,
    Flite and Pocket TTS (the browser's voices speak).
  - **The web's hooks and assets mirror the maintainer's Coupler**:
    `scripts/web-publish.sh` runs `src-tauri/examples/web_mirror.rs`,
    which reads a copy of the hooks database's triggers (never what was
    recorded, and no trigger on a pair that may name a person), renders
    every sound the triggers use to Ogg Opus with the desktop's own
    code, draws the ANSI art, then builds and, after a yes, uploads.
  - The desktop gained `ports.rs`, `trigger.rs` and `clock.rs`, moved
    out of `session.rs`, `hooks.rs` and the standard clock so the web
    build can share them; nothing it does changed.
  - Fixed: a site with no mirror showed "the hooks couldn't be read"
    and no port buttons. Cloudflare Pages answers a missing
    `mirror.json` with the page itself, the core refused to start on
    it, and every command failed after. Now only real hooks JSON
    counts, a bad mirror means no hooks rather than no Coupler, a
    failed start is retried, and a missing mirror file isn't played as
    the page.

## 0.15.0 (2026-10-03)

- **Seeing the game, for every player** (2026-10-03): Immersive's
  sounds can now be seen, and the screen read at low vision.
  - **Sound captions**: every sound Coupler plays is written in the
    Heard panel as it plays (a footstep, the exits and what's behind
    them, a fight starting, the heartbeat quickening, a hook's sound or
    music, the place's and the sky's loops), so a player who can't hear
    them misses nothing and anyone can learn the cues by seeing them.
    Never spoken. On by default; gear → Display… turns them off.
  - **Gear → Display…**: the game's colors as sent, made readable (a
    color too dark or light for its background is brightened or
    darkened to 4.5:1, hue kept), or none; and text twice or three
    times as big in Terminal, the game output, command line and
    messages filling the screen (78 or 52 columns, the game told so).
  - **Gauges in color**: health, mana and movement bars green, yellow
    or red by level (the numbers beside them still say it), and in
    Workshop a **You** panel with them and the fight above the game
    output (Customize the Screen hides it).
  - **The split view in review mode**: while Option+Up holds the output
    still, its bottom rows show the live lines under a rule, so what's
    coming in is seen without losing your place.
  - **Room pictures, when and again**: gear → Pictures… shows them in
    every room, at landmarks only, or only when asked; Cmd+Shift+P
    paints the room (or paints it again in a new look the room keeps,
    in `pictures/<world>.json` in app data; First Looks Back undoes
    them all). A picture you set for a room in the hooks list (ART on
    its `room.info.id`) shows in place of the painting.

## 0.14.0 (2026-10-03)

- **A picture of every room, by Coupler's painter** (2026-10-03): the
  first part of room pictures. Coupler draws the room you're in, in
  characters and the 16 VGA colors, from its terrain (plains, woods,
  hills, mountains, desert, swamp, the sea, a city's roofs, a port;
  indoors a hall of stone, wood, metal or magic with its torches, a
  cave, a chasm), the time of day (sun, dawn and dusk low on the
  horizon, moon and stars, lit windows at night) and the weather (clouds,
  rain, storms with lightning, snow lying on the land, hail, fog, dust).
  Instant, made on this computer, and the same each time you come back.
  It shows under the room's name in Immersive's Here panel, and in a
  Picture panel of its own in Workshop (gear → Customize the Screen),
  painted to fill it at any size. Gear → Pictures… picks the style,
  Fantasy or High fantasy (castles on the horizon, a second moon and an
  aurora at night, banners and braziers indoors), or turns pictures
  off. Hidden from screen readers and never announced; it's painted
  after each room's sounds start, never before.

## 0.13.0 (2026-10-03)

- **No home-folder paths from the C libraries** (2026-10-03): the release
  build now maps libopus's and WavPack's source paths the way rustc's
  already were (`-ffile-prefix-map` in `build-release.sh`), so the leak
  check passes again.
- **Voices ready before they're wanted** (2026-10-03): every line of
  talk, the journal's and the log's, is rendered in its speaker's
  built-in voice (Flite or Pocket TTS) the moment it arrives, heard now
  or not, and kept in a voice cache on this computer, so it starts at
  once when it's played. The cache watches the disk's free space: at
  most 1 GB or 5% of the free space, and it gives room back when under
  2 GB is free. Lines played once go first (channels before the game's),
  then the least played, oldest first; lines not yet heard go last.
  The Mac's own voices speak directly and aren't cached.
- **Talk in a pop-up, in Immersive** (2026-10-03): says, tells and
  channel lines are no longer written in the game output in Immersive
  (they're in the Journal and the Log). Instead the line being said
  shows in a pop-up at the top of the screen, titled with who's saying
  it, gone when the line ends or when it's clicked. Review mode and
  Cmd+Shift+O skip them there too. The other ways to play still show
  them in the output.
- **The journal** (2026-10-03): everything said to you and around you
  in the game (says, NPCs' lines, tells, the group) is kept by who said
  it and when, per character, on this computer only. Cmd+Shift+J (or
  the Journal button by the message bar) shows it oldest first,
  searchable and narrowed to one speaker. A green bullet marks a line
  not yet heard to the end. In Immersive each line is spoken as it
  comes, at once if nothing else is, else right after; Cmd+Period cuts
  it off and flushes the queue (now over a dialog too), and what's cut
  stays unheard, with a low knock-knock to say so. A line someone who
  isn't a player has said before (a shopkeeper's welcome) is kept and
  spoken once. Cmd+Shift+U says what's not yet heard; Cmd+Shift+N
  plays it; Play in the Journal says any line again in its speaker's
  voice.
- **The log** (2026-10-03): OOC, INFO and every other channel go in a
  log of their own (Cmd+Shift+K, the Log button), never spoken as they
  come so the game isn't interrupted. Each line is a soft blip at its
  channel's own note, and unheard lines have the green bullet too.

## 0.12.0 (2026-10-03)

- **A voice for everyone you meet** (2026-10-03): every NPC and player
  you meet (in a room, or talking) gets a voice of their own, made from
  their name: feminine or masculine, lower for a giant, higher for a
  pixie, slower when old or undead, and a little different for each
  name. In Immersive, tells and says are spoken in the speaker's voice,
  never the narrator's. Gear → Characters' Voices… lists everyone met
  in the game, the most recent first, and Edit changes a voice: its
  kind, the system voice, pitch, speed, or Quiet (written in Heard, not
  spoken). Your changes are kept; Back to Automatic and Forget undo
  them. Kept per game, on this computer only.
- **A built-in voice engine** (2026-10-03): Flite (Kal, at 8 and 16
  kHz) ships inside Coupler, so characters' voices come from every
  engine: the Mac's own voices and Flite's. Automatic spreads characters
  across all of them for the most different voices, pitching Flite's
  for feminine characters. The voice menu groups the voices by engine,
  with an Automatic in each. Lines in different engines take turns, and
  Cmd+Period stops whichever is speaking.
- **Pocket TTS's neural voices** (2026-10-03): 19 natural English
  voices, feminine and masculine, in a range of accents, ship inside
  Coupler (Kyutai's Pocket TTS, running on this
  Mac's CPU, nothing fetched). Automatic gives characters a voice of
  their own kind from them, Flite and the Mac's voices; pitch and speed
  work on them too. The app is about 250 MB larger for it.

## 0.11.0 (2026-10-03)

- **Speech by kind of line** (2026-10-03): gear → Speech… chooses
  what your screen reader reads as it comes in: the game's lines,
  talk (tells, the group, says, channels, even when the game wraps
  them over several lines), lines during a fight, the prompt, the
  commands you send and Coupler's own messages. Everything still
  shows on the screen. Your own commands aren't read back by default,
  since you heard them as you typed.
- **The prompt isn't read again** (2026-10-03) when it hasn't changed,
  and a prompt the game finishes when it answers you isn't read twice.

## 0.10.0 (2026-10-03)

- **Review mode** (2026-10-03): Option+Up reads the game output back a
  line at a time, Option+Down goes forward, Option+Home jumps to the
  oldest line, and Option+End returns to live. Each line is shown in
  the status bar (read by VoiceOver) and said by Coupler's voice when
  it's on, and marked in the output by a yellow bar. While you review,
  the output holds still and new lines aren't read over you: Coupler
  tells you how many came in, and sending a command takes you back.
- **Accented letters** (2026-10-03): text from the game that isn't
  UTF-8 is read as Latin-1, CoffeeMUD's usual character set, so an é
  shows as é rather than a box.
- **The map knows where you are at once** (2026-10-03): on logging in,
  Coupler asks the game which room you're in, so the map and Immersive's
  sounds start without your having to move or look.

## 0.9.1 (2026-10-03)

- **Voices studied** (2026-10-02): which speech engines Coupler can
  ship as free software. The plan: Coupler's own words in your system
  voice, played through the Mixer, with a small built-in voice where
  there's none; the game's talk in built-in voices, one per speaker, so
  you know who's talking without hearing the name.

## 0.9.0 (2026-10-02)

- **Three ways to play** (2026-10-02): gear menu, or Ctrl+Cmd+1, 2 and
  3. **Terminal** is a plain text terminal, 80 characters wide and as tall
  as the screen, with the command line under it and nothing else.
  **Immersive** tells the game in layers of sound so little has to be
  read: each room's exits play as notes placed left and right and high
  and low, with footsteps, a sparkle for a new room, a heartbeat when
  hurt, and calls for a fight starting, weakening and ending. Coupler
  speaks for itself, no screen reader needed, but only tells, says, the
  login and what you ask for (Cmd+Shift+L, V, E, O; Cmd+Period for
  quiet). **Workshop** is the screen Coupler always had, now with
  Customize the Screen… to show or hide each part and add Immersive's
  cues and voice.
- **Say keys** (2026-10-02): Cmd+Shift+V says your health, mana and
  movement, Cmd+Shift+E the fight, Cmd+Shift+O what the game said since
  your last command, in every way to play.
- **The Mixer** (2026-10-02) has the cues' volume and the voice's volume
  and speed.

## 0.8.0 (2026-10-02)

- **Terrain colors on the map** (2026-10-02): every explored room is
  drawn as a tile in its terrain's color, so woods, water, stone, caves
  and the rest stand apart at a glance, with a key under the map for
  the terrains in view. All 24 of CoffeeMUD's terrains have a color;
  alike ones share (15 colors, every one readable). The terrain is also
  in words, in Find a room's results and each room's tooltip.
- **Free and open source** (2026-10-02): Coupler is licensed under the
  Apache License 2.0, the same as CoffeeMUD and Sip. About says so.
- **Every license in the app** (2026-10-02): About → Open-Source
  Licenses shows the full license of every library Coupler is built
  with (271, among them libopus, WavPack and the VGA font), as their
  licenses ask.
- **Opus and WavPack sounds** (2026-10-02): `.opus` and `.wv` files can
  be dropped in the Assets folder and played by any hook, as sound
  effects, background noise or weather.
- **WAVs made smaller** (2026-10-02): a WAV dropped on Coupler is also
  made as Opus and as WavPack while it's added, and Coupler offers the
  smallest: a dialog lists each file, its smaller size and what's saved.
  Taking it puts the smaller file in the Assets folder in the WAV's
  place and deletes Coupler's copy of the WAV (never the file it came
  from). Opus nearly always wins, often a tenth of the size or less;
  WavPack is lossless, and only made at its slowest, smallest settings
  when it could beat Opus.

## 0.7.0 (2026-10-02)

- **No more MCP line on connecting** (2026-10-02): CoffeeMUD starts
  every connection with a line of code meant for MOO clients
  (`#$#mcp version: 2.1 to: 2.1`). Coupler no longer shows it, so a
  screen reader doesn't read it out each time.
- **Combat mode and Explore mode** (2026-10-02): the menu bar says
  which you're in while you play: Combat mode the moment a fight
  starts, even sitting or asleep in it, and Explore mode within about a
  second of it ending. VoiceOver says the change, once.
- **Exact opponent figures for Combat mode** (2026-10-02): Coupler now
  asks the game by MSDP for the opponent's hit points, maximum and
  range, checked every second, and keeps them ahead of GMCP's rounded
  percentage. Nothing shows them yet: they're ready for Combat mode.
- **Every protocol CoffeeMUD offers, studied** (2026-10-02): what each
  would bring, and which come next: MSP sounds (176 game events, from
  spells to doors to the weather) as hooks, then MXP's item, creature
  and exit menus.

## 0.6.0 (2026-10-02)

- **Hooks for the time of day** (2026-10-02): the Hooks list has
  `coupler.time`, with a row each for dawn, day, dusk and night from the
  start, so a sound, music or a picture can be set on each right away.
  CoffeeMUD sends no time by GMCP, so Coupler now takes the game's hour
  by MSDP (only that: nothing else is asked for) and knows the time of
  day from the moment it connects, changing on the game's own hour,
  asleep, blind or indoors. The game's own lines (the sun rising and
  setting, night falling) and what TIME says check it, and teach
  Coupler where dawn, day, dusk and night start if the server's clock
  isn't the usual one. Day, which the game never announces, comes on
  time.
- **Socials studied** (2026-10-02): what a player sees when a social is
  used, confirmed to come with no GMCP, and every line CoffeeMUD's
  socials print, ready to match for social hooks.

## 0.5.0 (2026-10-02)

- **Leaving the game stops what it set off** (2026-10-02): Hang Up, or
  the game closing the connection, stops every sound (music, effects,
  background noise and weather) and closes every picture, and **Choose
  how to play** comes back in front of the game output. Quitting hangs
  up first too, so the map is saved and the game sees the line close.
- **Logging out or switching character fades what was playing**
  (2026-10-02): LOGOUT (once you answer yes) and SWITCH fade every
  sound out and close every picture while you stay connected. A
  refused switch or a "no" stops nothing. After a switch, the new
  character's room's music and background noise come back in.
- **ART Fade** (2026-10-02): a picture's hook has a Fade, in seconds.
  0 (the default) keeps it until it's clicked away or replaced; any
  other number fades it away after that long, starting over each time
  its pair comes again.

## 0.4.0 (2026-10-02)

- **Background noise and weather** (2026-10-02):
  - Two more kinds of hook sound, both looping while they apply and
    fading in and out: **BGN**, background noise, and **BGW**,
    background weather. Each takes a sound effect's files (.wav, .ogg).
  - BGN is set on a room's pairs: its id (just that room; **This Room**
    in the Hooks list finds it), its terrain, or its room type, indoors
    or outdoors, which Coupler works out from the terrain and records as
    `coupler.room.type`. The room's own wins, then the terrain's, then
    the room type's.
  - BGW is set on `coupler.weather`. CoffeeMUD sends no weather by
    GMCP, so Coupler reads it from the game's weather lines (all 14
    kinds, the lines saying one ended, and a description wrapped onto
    two lines), kept per area and heard only under the sky. Words a
    player says don't count.
  - The Mixer has BGN and BGW volumes, says what's looping, and Stop All
    Sound stops them too.
- **The hook editor** (2026-10-02): a Hooks list row now shows only what
  the pair sets off, in words ("SFX door 40%, BGM town, loop, 75%", or
  Nothing), and an **Edit** button. Edit opens a dialog over the list
  with every choice for that pair: SFX, BGM (with Loop), ART (with its
  column and row), and BGN or BGW where they apply, each with its
  volume, plus Set Off Nothing. Escape closes only the dialog on top
  when one is open over another.

- **Assets: progress, what's in use, missing files cleared** (2026-10-02):
  - Adding assets shows a progress bar that fills by bytes read, its
    label naming the file (2 of 5), and whether it's reading, checking
    for a copy already there, or saving. After, the status says what
    became of each file: added as what, renamed because its name was
    taken, or already there.
  - The Hooks dialog has a third tab, **Assets**: every file in the
    folder, its kind and how many hooks use it, with a green bullet
    beside each one in use.
  - An asset whose file has left the Assets folder is cleared from every
    hook that names it (a hook left with nothing is removed), and the
    status bar says so: at launch, when the Hooks dialog reads the
    folder, and when a trigger fails to play.

- **Fixed: typing and clicking went sluggish after opening Hooks**
  (2026-10-02). The closed dialog kept its rows (up to 500, three asset
  menus each) and redrew them on every key typed and every line of
  output. Its rows are now drawn only while it's open, and it isn't
  redrawn when nothing it shows has changed.

- **Mixer, volumes, pictures that stack** (2026-10-01):
  - A **Mixer** button on the menu bar opens the master volumes for
    sound effects (SFX) and music (BGM), kept for next time, with Stop
    All Sound.
  - Each hook's SFX and BGM has its own volume (0 to 100, a share of
    the Mixer's), set beside it in the Hooks list, which now lists
    assets by name, without folder or file type.
  - While music plays, its name shows on the menu bar (♪ name);
    clicking it opens the Mixer.
  - A new ART picture at the place of one already showing replaces it;
    anywhere else it opens as another picture in front of the rest.
    Each closes when clicked; Cmd+Shift+S closes them all.
  - MIDI and tracker music are brought up to full level when rendered:
    MIDI through a quiet SoundFont played too softly to hear.
  - Music that failed to load (a MIDI before its SoundFont was added)
    is tried again the next time its hook comes, instead of never.
- **Assets and triggers** (2026-10-01): Coupler keeps an **Assets**
  folder in its app data. Files dropped anywhere on the window (or
  chosen with Add Assets in the Hooks list) are copied into it, sorted
  into a folder per file type (`wav/`, `mid/`, `png/`...); a taken name
  gets a number, and the same file dropped twice is kept once. On the
  right of every pair in the Hooks list, choose what it sets off each
  time the game sends it, any or all of three: **SFX**, a sound played
  once (`.wav`, `.ogg`); **BGM**, music played once or in a loop
  (`.mid`, `.mp3`, and the tracker modules `.mod`, `.xm`, `.s3m`,
  `.it`), which replaces other music but carries on if it's already
  playing; **ART**, a picture shown with its corner at a chosen column
  and row until it's clicked or replaced: an image (`.png`, `.jpg`) or
  **ANSI art** (`.ans`, `.asc`), drawn in the IBM VGA font on black in
  either theme. ANSI art can be the classic kind (CP437, 80 columns or
  its SAUCE width, cursor moves, iCE colors) or the kind Coupler reads
  from the game (UTF-8 with 16, 256 and 24-bit colors). MIDI plays
  through a SoundFont (`.sf2`) the player adds to the Assets folder;
  Coupler ships none. Ogg, MIDI and modules are rendered in pure Rust
  (lewton, rustysynth, xmrsplayer). Cmd+Shift+S stops the sounds and
  music and hides the picture.

## 0.3.0 (2026-10-01)

- **An 80 by 25 game output** (2026-09-30): the game output is always
  80 characters wide (its scroll bar beside them, not among them) and
  25 tall, starting at row 15 of the screen in the ANSIapps theme. It
  no longer changes size with the theme or the map.
- **A screen you arrange** (2026-10-01): gear → **Workshop mode** lets
  everything on the screen be moved and resized: the panels (the game
  output, the ways to play, the map), the command line and its hint,
  the message bar, the Hooks button, the screen readout, the menu bar
  and each button on it. With the mode on, a thing's top left corner
  moves it and its bottom right corner resizes it; nothing is drawn
  for them and nothing shifts, so the screen looks the same with the
  mode on or off, and the pointer's shape shows which corner it's on.
  A character at a time in the ANSIapps theme. Only the game output
  can't be resized: it's always 80 by 25. Buttons and other small
  controls are always in front of the bars (command line, message
  bar), and the bars in front of the panels, so a button put on top of
  something can't end up underneath it. From the keyboard, the arrow
  keys move or resize and Home puts it back. Coupler keeps every
  place, size and order. Gear → **Reset to Default Layout** puts
  everything back where it was before it could be moved. Anything
  added to the screen later gets the same.
- **Hooks** (2026-09-30): a database of every key and value the game
  sends by GMCP, each unique pair kept once (a SQLite database, `hooks.sqlite` in app
  data, one for all the games). The status bar shows the total; clicking it
  (or Cmd+Shift+H) opens a scrolling list with a search. Each row has
  a box: ticking it moves that key to a Filtered list (a second tab),
  out of the hooks list and its total; unticking it there brings it
  back. Filtered keys are still recorded.
## 0.2.0 (2026-09-30, unreleased)

- **A fixed screen** (2026-09-30): the window is 1280 by 720 and can't
  be resized, so everything has a fixed place; that's 160 by 45
  characters of the theme's font, and it fits every common desktop.
  Full screen (gear menu, ⌃⌘F, remembered) scales the same screen up.
  Status, errors and progress moved to a status bar along the bottom
  that's always there, so messages no longer push the screen down. Its
  right end shows the screen's size in characters and the character
  under the pointer.
- **ANSIapps buttons are just clickable** (2026-09-30): flat bars, no
  shadow, no press-in movement; white under the pointer, cyan while
  pressed. Now a convention of the theme.
- **The ANSIapps grid** (2026-09-30): every character of Coupler's own
  screens now sits in a cell of the 8 by 16 grid, and that's the theme's
  rule. One-row buttons and fields, frames drawn inside their own
  cells, titles centered by whole cells, dialogs at a fixed place,
  scrolling that rests on a row, and keys written as words
  (Cmd+Shift+L) because the font has no ⌘. The game output is 39 rows
  with a permanent scroll bar. Dev-only: gear → Check the Grid.
- **Map panel** (2026-09-30): the Exits list is gone (Where am I? and
  the picture cover it; a screen reader still gets the exits under
  "Where you are"). The picture is a fixed size with you in the middle
  (11 rooms by 7), and Landmark and Find a room sit below it. Fixed: the
  titles "Choose how to play" and "Map" were cut off at the top in the
  ANSIapps theme.

## 0.1.0 (2026-09-30, unreleased)

- **Scaffold** (2026-09-30): Tauri 2 + React/TypeScript + Rust on the
  ansiapps conventions: the in-development warning on first run, the
  loading screen from the first paint, activity feedback, the modern and
  ANSIapps themes (amber accent, yellow in ANSIapps), About with credits,
  the dev-only App Testing checklist, the release script with its `$HOME`
  check, and the license scan. Stand-in icon: two handset cups.
- **Connect and play** (2026-09-30): Connect dials coffeemud.net:23, the
  only address Coupler knows. Telnet negotiation follows Sip, CoffeeMUD's
  own client, with its bugs fixed (16-bit escaped NAWS, no negotiation
  loops, no user name in NEW-ENVIRON); MCCP2 compression; ANSI colors
  up to truecolor; the prompt shows as soon as it arrives; the input
  hides passwords while the server echoes and keeps them out of history;
  Up and Down recall commands; the window size reaches the game; GMCP
  is received (Core.Hello sent, not shown yet). MXP, MSP and MSDP are
  refused until Coupler can show them.
- **ANSIapps is the default theme** (2026-09-30): Coupler opens in the
  blue DOS look from the first frame; modern is the gear-menu choice.
- **Ways to play** (2026-09-30): six connection buttons, one per
  documented game coffeemud.net runs (Standard on 23 and 2323, Player
  vs Player 2324, Hardcore 2325, Role-Playing 2326, Classic 2327), each
  with what's different about it. The list is a constant in Rust and
  the window passes an ID, never a port number. The one played last is
  first. The server's Tech (2328), Heroics (2329) and NO (2330) ports
  are undocumented and have no button.
- **Auto-mapper** (2026-09-30): builds a map from the game's GMCP
  `room.info` and `room.exits`: rooms by ID, exits with the room behind
  each (known before you walk there), doors and locks, up and down. The
  map panel shows where you are, the exits as buttons, landmarks you
  name, a room finder, Walk (one step at a time, each after the game
  confirms the last; opens closed doors; stops when you type) and
  Directions in words. One map per world, saved in app data.
- **Accessibility is now a rule** (2026-09-30): every feature is
  designed for a blind or visually impaired player first. This pass:
  game output is read as it arrives; Where am I? (⌘⇧L) says the room,
  area and exits; Esc returns to the command line; the map's picture
  has all its facts in words above it; closed dialogs can't be reached
  by a screen reader, open ones take and return focus; focus rings are
  visible on every surface; gear → Keyboard lists the shortcuts.
- **Docs** (2026-09-30): the study of CoffeeMUD's GMCP, mapping
  features and ports; every known MUD client; the master feature list;
  the development plan and UI design.

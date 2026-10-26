# Changelog

## Unreleased

- **Sound comes back after the narrator on the iPad** (2026-10-26): in
  the web build on an iPad, the loops, music and sound effects could go
  silent for good once the narrator first spoke. iOS stops the sound
  for the system's voice; Coupler now starts it again a moment after
  each line ends (iOS takes a moment to let go), and a touch anywhere
  starts it too (it listened for the start of a touch, which iOS
  doesn't count, and Coupler's own keyboard on the iPad sends no keys).
- **The colors badge follows ANSI 256 and ANSI TRUE** (2026-10-26):
  the game writes its answer on the prompt's line (`<20Hp> ANSI 256
  colour enabled.`), and Coupler read it only at the start of a line,
  so gear → Pictures… and the pictures stayed at 16 colors. Both builds.
- **Reminders when you're hungry, thirsty, fatigued or stinky**
  (2026-10-26): Coupler says "You're hungry and thirsty." as soon as a
  need begins, and again every 5 minutes while any lasts, never in a
  fight. The game says hunger and thirst only now and then, and never
  says fatigue or smell unasked. Spoken by the narrator, or written as
  Coupler's own line. Both builds, the phone and the tablet.
- **A Stats table** (2026-10-26): gear → Tables… → Stats holds
  everything the game's STAT can tell of each character (hit points
  and their most, the six abilities and their bases, the saving throws,
  armor, attack and damage in words, experience, quest points, trains,
  practices, alignment, and more), read by Coupler quietly when you
  come in, when your level changes and every quarter of an hour,
  without showing in the output. `#check stats` says the main ones and
  reads them again. Both builds.
- **The Mixer says a level once it's set** (2026-10-26): let go of a
  slider (or stop pressing the arrows) and 2 seconds later "Sound
  effects at 40 percent." is said and shown, not every step of a drag.
- **#help lists Coupler's commands** (2026-10-26): every command with
  its words and what it does, a line each, in the output; only the
  names are said. `#help check` says one command.
- **Disconnections** (2026-10-26): when the connection ends and you
  didn't leave, Coupler says the likely cause (the game shut down, logged
  you out for being idle, this device was asleep, the network dropped,
  or the game closed it without a word) and offers what it saw in the
  last moments: how it ended, when the game was last heard from, the
  lines sent lately (never a password), the GMCP lately and the game's
  last lines, to copy into a report. Every ending is kept in gear →
  Tables… → Disconnections, each row's Show… opening its findings. A
  QUIT or Hang Up offers nothing. Both builds.

## 0.47.0 (2026-10-26)

- **The game's colors as a badge** (2026-10-26): gear → Pictures… opens
  with a badge saying ANSI 16, ANSI 256 or ANSI TRUE beside a strip of
  those colors: the 16's eight and bright eight, the 256's cube steps
  and grays, true color's smooth rainbow and gray. It changes as soon as
  the game does. The words under it say the same for screen readers.
  Both builds, the phone and the tablet.

## 0.46.0 (2026-10-26)

- **Aliases work as soon as you're in the game** (2026-10-26): an alias
  typed was sent as it was ("Huh?") until the game had told Coupler
  your character's name, which it does only every 16 seconds or so, and
  never again after a LOGOUT and the same character back in. Coupler
  now asks for the name at once. Triggers and tickers had the same
  wait. Both builds.
- **Every currency sounds like coins** (2026-10-26): Coins played only
  for gold coins, notes, bits and pieces. Now it plays for every coin
  of every currency in the game: Alramus's crowns, Atlantis's
  sanddollars, shells and conches, Old Troya's drachmas, Cannabis's
  buds, Character Coin and dark credits, whether picked up, looted,
  dropped, given, banked or paid. The Bestiary keeps them by their kind,
  not how many. Both builds.
- **LOOK LOOT goes to the game again** (2026-10-26): Coupler no longer
  answers it. #check loot says what the last fight brought, shown and
  said. Both builds.

## 0.45.0 (2026-10-26)

- **Open and Close on the tablet's pad** (2026-10-26): the two empty
  corners at the pad's left are Open (top) and Close (bottom). Press
  one, then a way: Open then W sends OPEN WEST. Pressed, it's yellow and
  says so to VoiceOver; pressed again it lets go.
- **Loot collected** (2026-10-26): when you win a fight and AUTOLOOT
  (or AUTOGOLD) takes things off the body, the narrator says "Loot
  collected." once instead of every item, and Immersive leaves those
  lines out of the output. Each thing goes in the Loot table. LOOK LOOT
  or #check loot says just what the last fight brought. Both builds.
- **Tables** (2026-10-26): gear → Tables… (Cmd+Shift+T; the phone's
  Here sheet) shows Coupler's tables (Loot) and your own: make one with
  a name and up to five columns, add rows there or with #add, delete
  rows, clear Loot. #check and a table's name says it. Kept for each
  world on this machine, in a Coupler Backup and in gear → Storage….
  Both builds.
- **Coins sound like coins** (2026-10-26): money got, looted, dropped,
  given, put away, deposited, withdrawn, borrowed, paid or bid plays
  CoffeeMUD's Coins, as buying and selling do. It's its own event, "you
  handle coins", in the hooks list, for a sound of your own. Both
  builds.

## 0.44.0 (2026-10-26)

- **The game's sound ducks under a voice** (2026-10-26): while the
  narrator or a character speaks, sound effects, music, the place's
  sounds and the weather go quieter, never stopping, and come back up
  a moment after the last line; Coupler's cues stay as they are. Both
  builds.
- **Sound comes back after the narrator speaks** (2026-10-26): on the
  iPhone's Safari the system's speech stops the page's sound and never
  starts it again, so the place's sounds (BGN), the weather and the
  music went quiet once the narrator spoke in another voice. Coupler
  starts it again when the narrator's line ends, and at any key or
  touch.
- **Pictures in each of the game's colors** (2026-10-26): a picture
  made with Create Asset is painted in the 16, 256 and true color, and
  the one shown follows the colors you have the game send you (a pair's
  ART, the room's, the preview). A picture made before is painted again
  in the others when the game's colors change, if it's still the
  painter's own; other ANSI art shows as it is. The race and class
  portraits paint again too when the colors change. Both builds.
- **A newer Coupler is a dialog** (2026-10-26): Check for Updates Now,
  or the automatic check at launch, says a newer version in a dialog
  you can't miss, with Get It to open its page. While playing, the
  automatic check says it in the status bar instead.
- **The narrator waits for the game** (2026-10-26): Coupler's narrator
  is quiet before a character is in the game (the first screen, logging
  in, the account menu, making a character), so the screen reader reads
  all of it. Gear → Speech… → Coupler's narrator before the game too
  turns it on there.

## 0.43.0 (2026-10-25)

- **Pictures in more colors** (2026-10-25): when you have the game send
  you 256 colors or true color (ANSI 256, ANSI TRUE), Coupler's painter
  follows: skies and land are smooth gradients, night is shadow instead
  of a checkerboard, and the dithers in rooms, faces, items and The
  Fight show as the colors they stand for. Sparse textures (cobbles,
  grass) are kept. 16 until the game says otherwise; remembered across
  launches; gear → Pictures… says which. Both builds, the phone and the
  tablet.
- **The web build fetches a sound again on failure** (2026-10-25): a
  sound from the mirror that fails (seen on the iPhone with the bank's
  wind) is fetched once more past the browser's cache, and the message
  says why it failed if it still does.

## 0.42.0 (2026-10-25)

- **The iPhone's web screen, made for its size** (2026-10-25): in the
  web build on an iPhone the menu bar is one row (no icon or name;
  Explore or Combat, a dot for online or offline, Quit, Where, Map,
  Mixer), Choose how to play is one column, and You reads HP, MN and MV
  with bars five characters wide, each character a quarter step. The
  lower part is now in the game output's own text size: Coupler's
  phone keyboard (numbers, three rows of letters with Backspace, Shift,
  the arrows and Return, then Esc, Ctrl, Opt, Cmd, Space and the
  command prefix), whose keys show bigger above the finger and type on
  lifting it, so a finger can slide to the right key; and fewer touch
  controls, the size of the text. The dev build tries it from the gear
  menu (iPhone Web Screen, for Testing). The phone app's Choose how to
  play is one column too.
- **The Fight at the top left** (2026-10-25): in every way to play, and
  on the tablet, The Fight now comes up at the screen's top left.

## 0.41.0 (2026-10-24)

- **Faces in The Fight on the iPhone** (2026-10-24): the strip above
  the touch pad is now the room you're fighting in with the two of you
  in it, your face at the left and the foe's at the right, drawn small
  for the phone (not shrunk: each kind of face has its own small
  drawing, so the eyes still show), angry, hurt and in pain as the
  fight goes, with the jolt, the flash and a new foe sliding in. The
  words sit between the faces: your health, bar, mana and moves, the
  clock or the result, what's on you, the foe's hurt, bar and health.
  It's eight rows; with pictures off (gear → Pictures…) it stays the
  four rows of words.

## 0.40.0 (2026-10-24)

- **The Fight moves, and comes to the iPhone** (2026-10-24): a big
  blow (15 percent or more of the health at once) knocks the face hit a
  cell back and flashes it once, never more than three flashes a
  second; a new foe slides in from the right. With reduced motion,
  only the flash. On the iPhone, The Fight is a strip of four rows
  above the touch pad in the Game section: both names and health with
  the clock (then the result) between them, the two health bars, your
  mana and moves, and what's on you, scrolling round when it's long.
  No faces in the strip yet.

## 0.39.0 (2026-10-24)

- **What's on you, and your group** (2026-10-24): Coupler now knows
  the spells and other effects on your character, each with its time
  left, and your group. In a fight, The Fight shows them under the
  faces ("On you Shield spell 3:20", counting down), your face takes
  their look (a halo for Sanctuary, poison's green), a row for each
  ally gives their health, mana and moves, and ◄ marks whoever the foe
  is hitting. Cmd+Shift+X (the phone: On you) says it all, with the
  times in words; a spell wearing off is a soft falling shimmer, a
  command the game made you wait for (You must wait…) two taps when
  it's ready again, both in Gear → Cues…. Combat Summary has Effects:
  what was on you at the start, and what came and wore off. The times
  come from the game's own AFFECTS, asked now and then and never shown,
  only after something changed and never while you're typing or away;
  on the web too.
- **Combat Summary** (2026-10-24): every fight is kept, blow by blow,
  to study afterwards (Cmd+Shift+Y, gear → Combat Summary…, or the
  button under The Fight's picture at its end). A fight's page says who
  you fought, how it ended and how long it took, where, the experience
  and what you took; the damage you dealt in all and by weapon (hits,
  misses, the biggest, the average) and by the spells and skills you
  used; the damage taken by attacker and by attack, what you dodged and
  healed; mana and moves spent, by command; your allies' blows; both
  healths over time, in figures and as charts; every moment by the
  second; and how it compares with your fights with the same foe
  before. Damage is the game's own figure when it shows one, otherwise
  measured from the health that fell and shared among the blows by the
  game's hit words ("cut" is 11 to 15), marked "about". The last 200
  fights are kept whole per character, older ones in brief; Copy as
  Text copies a page. Kept in Storage and backups; on the web too.

## 0.38.0 (2026-10-24)

- **The Fight** (2026-10-24): in a fight, a panel of its own faces you
  off against your foe over the room you're fighting in, painted by
  Coupler's painter: you on the left (your character's race, turned to
  face them), the foe on the right (as the Bestiary knows it, else from
  its name), each angry, in pain, afraid or dead and hurt in five steps
  as your health and theirs fall. Health bars meet in the middle (a
  loss leaves a strip that drains away), with your mana and moves, the
  fight's clock, the range and, at the end, KO. Over the picture in
  Workshop and Immersive, over the Control Panel in Terminal; also on
  the tablet and the web. Cmd+Shift+F or gear → Pictures… hides it.
- **Faces in their place, with moods, wounds and effects** (2026-10-24):
  a creature's face in the Bestiary stands in the area it's seen in
  most, painted by the room painter a shade darker (an animal in the
  wild, a fish in the water), lit by its sky. People and the races that
  think (a gnoll, a lizard man, a minotaur, a werewolf) have whites to
  their eyes, brows and clothes; an animal sits low in the frame, its
  back running off the side. An orc, a centaur, a naga and a harpy now
  have a person's face (CoffeeMUD files the orc as porcine, which drew
  a pig). Ready for the fight panel to come: six moods (calm, with a
  sneer for the evil; angry, in pain, afraid, asleep, dead), five steps
  of hurt as CoffeeMUD's health lines go (flesh bleeds, fur mats, stone
  and bone crack, ghosts and elementals fade, slimes shrink), and the
  look of about 160 effects by name (a halo for Sanctuary, poison's
  green, frost, burning, blindness, faerie fire, webs, stone and bark
  skin, a glow; sleep, fear and rage set the mood).
- **Items in their material, worn or fine** (2026-10-24): an item's
  material is a texture, not only a color: cloth woven, leather
  stitched, wood grained, metal with a band of light, mithril and gems
  sparkling, glass showing what's behind it, stone speckled; from what
  EXAMINE says, its name, or its kind (CoffeeMUD's own material classes).
  Its name's words show its wear (old, dented, cracked, frayed,
  notched, tattered, broken), rust on iron, a crude outline or gilt
  trim. Swords, daggers, axes, helmets, shields and potions take their
  shape from their names (a scimitar curves, a rapier is thin with a
  basket guard, a great helm has a face plate, a kite shield is long)
  and their seed: shields carry one of five devices. Each stands on a
  table, or in the place it's found most, with its shadow.

## 0.37.0 (2026-10-24)

- **Mute a channel in the Log, by the game's own commands** (2026-10-24):
  on a channel's tab (OOC, ZCHAT…), Mute Channel, left of Mark All Heard,
  sends CoffeeMUD's NO<channel>, so the game stops sending it to the
  character, in Coupler or any client; Unmute Channel sends <channel> ON.
  The tab says (off) as the game says. On Every Channel, **Channels…**
  opens a dialog of every channel the character may use, each on or off
  to turn, with Turn All On and Turn All Off (CHANNELS ON and OFF). The
  game's answers (its CHANNELS list, read when the Log opens and after
  each change) aren't shown in the output; a refusal (QUIET mode) is
  told in the status bar.
- **The web build's clock counts whole microseconds** (2026-10-24): it
  kept fractions of a millisecond, so a ticker's next time could fall a
  hair after the second it was due and wait one more.
- **A row's own buttons come first** (2026-10-24): every list's buttons
  for one entry are at the left of its rows, before the name, so they
  sit in one column however long the names: the Bestiary's Show…, the
  Journal's and Log's Play, the hooks' Edit…, the Assets' Play or Show…,
  CoffeeMUD's Sounds' Play and Edit…, Characters' Voices, Cues,
  Narrator's Answers, the triggers, gags, aliases and tickers (and their
  last lines), Priority Audio and Storage.
- **Tickers keep their time** (2026-10-24): a ticker that came due while
  Coupler's WHO waited for its answer (about 10 seconds of every minute)
  skipped its whole turn, so one every 15 seconds missed about one in
  twenty, and one every 61 seconds could fall in step with WHO and go
  six minutes unsent. Now it waits for the answer and goes then, within
  12 seconds of its time. A two-hour test of tickers from 15 seconds to
  8 minutes beside WHO's clock checks it.
- **`CLAUDE.md` trimmed** (2026-10-24): from about 1,050 lines to 380.
  The rules and conventions are kept whole; "Where things live" is now
  a short map by area, keeping each module's do's and don'ts and leaving
  the details to the code and its headers.
- **The Bestiary's names in the game's colors** (2026-10-24): the Name
  column is black, as the game output is, and each name is drawn in the
  colors the game gave it (CoffeeMUD's `^` codes, which showed as
  letters before). An item named in colors and the same item printed
  plainly are now one entry.
- **Bundles are one item** (2026-10-24): "a 25# oak bundle" and "a 3#
  oak bundle" (and pools of a liquid) are kept as "an oak bundle", and
  the entries kept before are joined when the Bestiary opens.
- **The say keys answer on the screen too** (2026-10-24): Who, HP,
  Where, Fight and the unheard count (the tablet's buttons and their
  keyboard shortcuts) write their answer in the game output as well as
  saying it, so whatever the player asks always shows and sounds.
- **Scroll bars that show** (2026-10-24): on an iPad or iPhone, where
  the browser shows its own only while scrolling, every box with more
  than it shows has a bar at its right edge (the ways to play after
  hanging up, the lists, the game output). On the desktop the modern
  theme's bars show all the time, as the ANSIapps theme's did.

- **Install the web build as an app** (2026-10-24): coupler.ansiapps.com
  has a manifest and icons, so Chrome and Edge offer Install, Safari
  Add to Dock, and an iPhone or iPad's Add to Home Screen opens it
  without the browser's bars, with Coupler's icon and name.
- **The web build on an iPhone plays on the tablet screen**
  (2026-10-24): the game above and the command line with Coupler's own
  keyboard and the controls below, never zooming, as on an iPad. Only
  the smallest text fits an iPhone's width, so Larger Text is greyed
  there. The iPhone app keeps the phone screen.

## 0.36.0 (2026-10-24)

- **Portals, ships and caravans** (2026-10-24): what the room's exits
  text lists after the directions (a portal's name, a ship's or a
  caravan's gangplank) is read (`ways.rs`), kept on the map's room, and
  shown in Here and the Map's Where you are. Immersive's narrator says
  "There's a ship here." on arriving, and Where am I? names each with
  the command that goes through it (ENTER gangplank). A portal's row
  with no door name (`: a swirling vortex`) is no longer shown or said
  in Immersive.
- **The Music Editor won't save the same music twice without asking**
  (2026-10-24): Save… and Save as… look for a MIDI file in the Assets
  folder with the same notes, instruments, tempo and meter, whatever the
  names, and ask first. `#save` and `#hook` use that file instead.
- **The narrator's phrases are rendered once** (2026-10-24): in a
  bundled voice, every line the narrator says goes in the voice cache
  the first time, so "Command line." and the like play at once ever
  after, every connection and launch. Phrases said more than once are
  the last thing the cache ever gives up.
- **Storage** (2026-10-24): gear → Storage… shows what Coupler keeps on
  this computer, kind by kind, and the disk's free space. Clear the
  Caches gives back what's made again as it's wanted; the assets no
  hook uses, the maps, the rooms' looks, the journal, the Bestiary and
  the copy from before a restore can be deleted, each after a warning
  that offers a backup first. Export the Smallest Backup… uses Brotli's
  strongest setting on every file.
- **Where each race starts** (2026-10-24): the guide's facts about a
  race begin with where it starts on coffeemud.net (Mud School, Cora
  Isle, Magthere Wa`q or Tzbrakas Training Academy), shown beside the
  list and said as the race is reached. From the game's wiki, since the
  game's start rooms aren't in CoffeeMUD's source.
- **The stats to spend points on show again** (2026-10-24): a read
  that ended mid-line ("Enter a Stat to add or remove po") was taken
  for the guide's question, and the table went with it, so the Stats
  step showed nothing. A question now has to end as CoffeeMUD's do.
- **The narrator no longer waits for ever after making a character**
  (2026-10-24): the bundled voices rendered the journal's and the log's
  lines ahead one after another, and a line wanted now waited behind
  all of them ("Getting the answer ready…" on and on). A line wanted
  now goes first.
- **Coupler's own look at a creature is hidden from its first line**
  (2026-10-24): the Bestiary's unobtrusive look showed and said its
  first lines when the game's health line didn't start with the name
  room.mobiles gives (a race's own words, a name in colors). Every line
  to the prompt is hidden now, and a command typed meanwhile no longer
  lets it through.
- **Immersive leaves out all the exits' lines** (2026-10-24): a long
  exit row the game wrapped, a door's row, and a wrapped `[Exits: …]`
  no longer show or get said.
- **`#play` plays as the music** (2026-10-24): on the BGM channel in
  place of what was playing, which fades out; the game's music waits
  till the song stops (the place's comes back then). The sound layers,
  the menu bar and the Mixer call it "Music Editor - " and its name.
- **QUIT and hanging up, said in few words** (2026-10-24): at QUIT's
  question the narrator says "Ready to quit?"; when the connection
  ends, however it did, "CoffeeMUD disconnected." and nothing else.

## 0.35.0 (2026-10-24)

- **Music in the game's journals** (2026-10-24): the Music Editor's
  song written into a journal entry as TMF, the Text Music Format
  (`#write journal`), and read back by anyone with Coupler who READs
  it: the song opens in the Music Editor (`#load` if it had changes
  unsaved), the code itself neither shown nor said. A clan's journal
  (Clan Crafting's) is its forum, so a clan shares music there. TMF
  is Neumetik's (`neumetik_midi::tmf`): only letters, digits, `-` and
  `_`, wrapped any way the game likes, checked by a CRC, a 16-bar song
  of about 200 notes in about 180 characters. Coupler answers WRITE's
  questions one at a time, and stops if the WRITE fails.
- **Coupler's own commands, after `#`** (2026-10-24): `#play`, `#pause`,
  `#rewind`, `#loop`, `#stop` and `#save` for the Music Editor's song,
  open or closed; `#hook room` and `#hook area` make it the music of
  where you are (`#unhook` takes it off); `#prefix` picks another
  character; `#help` lists them. They're never sent to the game or
  written in its output: a soft tick says one was taken, the status bar
  what it did, two notes when a slow one's done. `##` sends a `#` to
  the game.
- **Music for a room or an area** (2026-10-24): a BGM hooked to a room
  or its area plays while you're there and fades out when you leave for
  a place with other music or none, goes quiet in a fight and starts
  again after.

## 0.34.0 (2026-10-24)

- **Triggers, gags, aliases and tickers** (2026-10-24): gear →
  Triggers, Gags, Aliases and Tickers…, in both the desktop and the web
  build. Triggers: the game's last ten lines, gagged ones too, each
  with Create…, which offers patterns for it (the exact line, any
  number, anyone in place of a name, whatever's said, how it starts or
  ends) or your own, with `*` for any text and `%1`, `%2`… for what it
  stood for; then what happens: commands sent, words the narrator says,
  a sound, and the line gagged too. Gags: the last ten lines shown, a
  Gag button each, or a pattern of your own; a gagged line isn't shown,
  said or read. Aliases: a word that stands for commands (`%1`… and
  `%*` for the words after it; `\` first sends a line as typed).
  Tickers: commands sent every so often. As you type an alias's or a
  ticker's commands, tips from CoffeeMUD's own commands, skills and
  spells say what each is, how it's used, and what could finish the
  word. Gentle to the game: nothing's sent without a character in the
  game or into a password, a ticker waits 15 seconds at least and not
  once you've been away 9 minutes, and a trigger answering its own line
  is held back.
- **The Bestiary learns a race by butchering** (2026-10-24): a body
  butchered gives materials named for its race ("a pound of wolf
  meat"), so the creature's race is the game's own word, and its entry
  lists what its body gave.
- **The phone screen in sections** (2026-10-23): Game, Map, Here and
  Heard, one at a time, as a carousel: swipe in from the right edge for
  the next, from the left for the one before (or the bar's ◄ and ►),
  and the screen says which. A swipe down from the top (or ▼) brings
  the options, the general ones and the section's own; a swipe up puts
  them away. The keys above the command line are the section's: the
  touch pad in Game, the ways with Where and Stop in Map, Look, Score,
  Inventory, Equipment, Time, health and who's online in Here, the
  Journal, the Log, the Mixer and Quiet in Heard.
- **The tablet screen** (2026-10-23): on an iPad, held upright, its own
  screen. Above, Workshop's desk to arrange as you like, its text size
  yours (gear → Larger Text and Smaller Text, or Cmd+Plus and
  Cmd+Minus: bigger text, fewer columns and rows). Below, one fixed
  part: the command line, the messages, and between them Coupler's own
  keyboard while you type (function keys, Esc, Tab, Ctrl, Shift,
  Option, Cmd and the arrows), or touch controls while you don't: the
  ways and Look, what's typed most, Say… and Tell… begun for the
  keyboard to finish, the Journal, the Log and the settings. The
  system's keyboard never shows, the page never zooms, and held
  sideways it asks to be turned upright.
- **Try the phone screen on an iPad** (2026-10-23): gear → Phone
  Screen, for Testing shows it phone-shaped, upright whichever way the
  iPad is held; the options' Leave the Phone Screen goes back.

- **Resize the game output in Workshop** (2026-10-23): with Move and
  resize everything on, its bottom right corner resizes it like any
  panel (20 columns by 5 rows at least), and the game is told its new
  size. Home gives it back its 80 by 25.
- **What you type isn't written in the game output** (2026-10-23): the
  command line keeps it in its history, and gear → Speech… can still
  have the screen reader say it.
- **The phone screen in Safari on the iPhone** (2026-10-23): with
  Request Desktop Website on, Safari passes for a Mac, and the web build
  showed the desktop's wide screen. An iPhone is now told by its touch
  screen and size too, as in Chrome and Brave.

## 0.33.1 (2026-10-23)

- **No more blue screen on the iPhone** (2026-10-23): the map panel was
  the one part of the screen not asked whether the layout has it, and
  Workshop (the default) with the map open (also the default) put it on
  the phone, whose layout has no place for it: the screen failed to
  draw past the in-development notice, in the web build on an iPhone
  and in the iPhone app. The map now shows only where the layout has it.

## 0.33.0 (2026-10-23)

- **The web build does what the desktop does** (2026-10-23): on the web
  now too, the journal and the log (Cmd+Shift+J/K/U/N, a typed channel
  LAST answered from the log), the Bestiary and the Items with their
  faces and pictures (Cmd+Shift+B, and the unobtrusive look), room
  pictures by Coupler's painter (the Picture panel, Pictures…,
  Cmd+Shift+P), Characters' Voices with the browser's voices, the
  greeting at launch (times played and who's online, by MSSP over the
  game's own WebSocket, never logging in), and Coupler Backup (a
  download, and a backup chosen or dropped on the page; a desktop
  backup's maps, voices, looks and settings restore on the web, and a
  web backup restores on the desktop). CoffeeMUD's own sounds play where
  no hook gives one, as on the desktop: the mirror now publishes every
  sound of coffeemud.sf2 as Ogg Opus (250 files, about 8 MB, fetched as
  they're wanted). Left on the desktop alone: the hooks list and editor,
  the player's own assets, Create Asset, the Music Editor, the bundled
  voices and the update check. The journal and the Bestiary share their
  code with the desktop: each keeps its SQLite half in a store of its
  own (`journal_store.rs`, `bestiary_store.rs`), and the web's stores
  keep the same rows as JSON in the browser, the same tests run against
  both; Coupler Backup's file is `backup_format.rs`, shared too.

## 0.32.1 (2026-10-23)

- **No more blue screen in Workshop** (2026-10-23): Workshop shows
  everything the player hasn't hidden, and that took in the iPhone's
  touch pad, which no desktop layout places: the screen failed to draw
  past the in-development notice, in the web build (Safari and Chrome)
  and for anyone new to Workshop on the desktop. The phone's own parts
  (`PHONE_ONLY` in `App.tsx`) are never on a desktop screen now.

## 0.32.0 (2026-10-23)

- **The account menu lists your characters** (2026-10-23): CoffeeMUD
  writes the list on the prompt's own line (`Command or Name (?):  Name
  Race …`), so its header was never recognized and the dialog said
  Reading your characters… for good. `account.rs` now reads past the
  prompt, a header cut short by a narrow wrap (`Nam Rac Leve`), a row
  wrapped onto a second line by `(email)` or `(post)`, and a list the
  prompt ends with no total.
- **Coupler's own look is silent** (2026-10-23): once the answer to its
  unobtrusive look begins, every line is gagged until the game's prompt
  comes back, so a line the Bestiary's reader doesn't follow (equipment
  on one line, an odd health line) no longer slips into the terminal.
- **LOOK says the room too** (2026-10-23): a plain LOOK (LOOK, LOO, LO,
  L) has the narrator say the room's description, as LL does, with the
  hidden details the last long look at that room found; the room's
  title is found on the prompt's line too, where it often comes.
  Priority Audio's item is now "A look at the room".
- **Pictures in the Bestiary** (2026-10-23): Show… has the creature's
  face or the item beside the facts, 25 by 12 cells (25 by 24 painter
  pixels, 200 by 192), by Coupler's painter. A face (`face.rs`): a person, a beast,
  a bird, a scaled head, a bug, a skull or a being of fire, stone or
  slime by its race's category; colors, beards, scars, eye patches,
  horns, tusks, glowing eyes and old age from its description and name;
  what it wears on its head, neck, ears, eyes, mouth, torso and back.
  An item (`item_art.rs`): one of 52 kinds by the word in its name
  (weapons, armor and clothes, jewelry, potions, scrolls and books,
  containers, food, coins, gems, keys, lights, instruments, tools,
  rocks; a bundle otherwise), in its name's colors and what EXAMINE
  says it's made of. What the game writes after an item's name is kept
  (`item_auras`) and drawn on it: glowing (a halo, red, blue, purple or
  white for the auras), a magical aura (sparks), a metallic or mithril
  aura (glints), invisible (half seen through), hidden (in shadow),
  floating or flying (lifted over its shadow), bound (a chain). Its
  slot is written on the picture's bottom row, centered (where it was
  seen worn, else where its kind goes, else Carried), and the facts
  say both in words.
- **The release build runs on Python 3.9 again** (2026-10-23):
  `scripts/third-party-licenses.py` no longer passes `newline=` to
  `Path.write_text`, which needs 3.10 (Xcode's `python3` is 3.9).
- **The Bestiary** (2026-10-23, gear → Bestiary…, Cmd+Shift+B): every
  creature and every item you come across in a game, and all you learn
  of each as you play, kept quietly on this Mac. A creature: the areas
  it's found in and, in each, its level (from CONSIDER, beside your
  own), its hit points (from a fight) and its alignment (glowing red
  or blue); its race, read from its name until a long look says it;
  its size, body and description from a LOOK or LL; the abilities it
  uses, known by the lines CoffeeMUD prints for them (about 1,100 from
  the game's source), and from them what it seems to be (a Mage, a
  Cleric…); what it drops and what it wears; how often you've seen it
  die. An item: where it lies, its level, weight and material from
  EXAMINE, its description, and who dropped or wore it. Search finds a
  name, a race or material, or an area; Show… says everything known,
  a fact a line. Kept in Coupler Backup.
- **Every way to play sounds the same** (2026-10-23): the cues,
  Coupler's voice and all of Immersive's audio now play in Terminal and
  Workshop too; the ways differ only in what's on the screen. Workshop's
  sound layers (now on unless you turn one off) are its own choice.
  Cues…, Narrator's Answers… and Priority Audio… are in the gear menu in
  every way. With a screen reader, VOX at 0 in the Mixer gives the
  game's lines back to it.
- **Coupler looks for you, if you like** (2026-10-23): ticked in the
  Bestiary, Coupler looks at each creature you've never looked at, once,
  with LOOK … UNOBTRUSIVELY (the room isn't told), when a command of its
  own would go unnoticed and never in a fight, and keeps what it sees
  without showing it. Off at first.
- **What a body tells** (2026-10-23): EXAMINE at a creature's body gives
  its level (exact at 18 intelligence, a range below) and its weight;
  any look at a body lists what it carried. The game shows no race on a
  body, so that still comes from a long look or the name.
- **Hooks, the same in every way to play** (2026-10-23): Hooks…,
  Mixer… and Music Editor… are in the gear menu in Terminal, Immersive
  and Workshop alike. Triggers already played the same in all three;
  now each can set them, their volumes and their music.
- **Creatures' cries** (2026-10-23): a creature of a kind
  coffeemud.sf2 has a voice for (humanoid, goblinoid, giant-kin,
  canine, beasts, birds, reptiles, insects, the undead, dragons,
  elementals, constructs) cries as it comes into the room, as a fight
  with it starts, and as it dies, by its race in the Bestiary. Each is
  Coupler's own pair `coupler.creature` (`"canine fights"`), all 36 in
  the hooks list from the start and in CoffeeMUD's Sounds' Creatures,
  for a sound of your own.

- **CoffeeMUD's Sounds, a tab of the Hooks list** (2026-10-23): every
  pair CoffeeMUD's sounds play for while you've set none, in groups
  (places, weather, combat, magic, creatures, nature, doors and locks,
  comings and goings, everyday doings, crafting, socials, your
  character): each terrain's loop (the woods by day and by night), each
  weather's, the battle loop, every sound the game names and every one
  of Coupler's own events, with its volume and what's laid under it.
  Where your own sound takes its place (on the pair, or a room type's
  background noise for its terrains), a green bullet and its name;
  how many rooms have their own background noise. Play tries the
  default; Edit… opens the hook editor on its pair.

## 0.31.0 (2026-10-23)

- **CoffeeMUD's sounds for everything else** (2026-10-23): the blows
  and deaths of a fight now play. CoffeeMUD names each by a roll of the
  dice (`punch1.wav` to `punch7.wav`, `death1.wav` to `death7.wav`), and
  until now nothing played for them; now Flesh hit and Monster collapse
  do, and all fourteen are in the hooks list from the start. What the
  game names no sound for at all has one now too, as Coupler's own
  pair `coupler.event`: someone walking into the room or out of it
  (footsteps), sitting, standing, sleeping and waking, getting,
  dropping, giving and putting things, wearing, wielding, holding and
  removing them, and buying and selling (coins), the player's own only.
  All of them are listed from the start for sounds of the player's
  own, and the hook editor says which of CoffeeMUD's plays while one's
  None. In a fight, where the player has set no background noise of
  their own, Battle tension loops in place of the place's. And the
  sounds the bank lays a sub-bass or an accent under now play with
  them: a fire spell's boom and crackle, a lightning bolt's crack and
  buzz, a critical hit's thump, a dragon's roar, the casts' sparkle.
  The comings, goings and doings play the web mirror's hooks on them
  too.
- **Hear where they came from and went** (2026-10-23): someone coming
  into the room or leaving it is heard from that way, as Immersive's
  exits are: from the east in the right ear, the west in the left, the
  north a little higher, the south a little lower, above and below
  further still, and the caption says it ("♪ Footsteps in, from the
  north"). How they move is heard too, from the word the game uses for
  their race, their gait (the GAIT command: amble, tiptoe, thunder…)
  or what they're doing: walking plays footsteps, creeping, sneaking
  or slithering soft ones, thundering or stomping footsteps with a
  heavy thud, flying a whoosh, swimming a splash, falling a fall, and
  fleeing every step. Someone leaving is heard softer than someone
  arriving, going away. Each is its own `coupler.event` value
  ("someone creeps in", "someone flies away", "someone flees"), 13 in
  all, for sounds of the player's own, which are heard from there too.

## 0.30.0 (2026-10-23)

- **CoffeeMUD's places and weather heard** (2026-10-23): wherever the
  player has set no background noise of their own, the place loops
  coffeemud.sf2's sound for its terrain (a town, a forest by day or by
  night, a cave, a tavern, the open plains, underwater, 24 terrains in
  all), and outdoors the weather overhead loops its own (rain, a
  thunderstorm, wind, snow, fog, a dust storm…). They fade from one to
  the next as the player walks, at 60% of the BGN and BGW volumes, and
  the player's own loops on a room, terrain, indoors or outdoors, or a
  weather always come first. The hook editor says which of CoffeeMUD's
  loops plays on a terrain's or a weather's row while it's None. Not on
  the web.

## 0.29.0 (2026-10-23)

- **The game's sounds play** (2026-10-23): Coupler now accepts MSP, so
  CoffeeMUD names a sound in its text each time one happens (a spell
  landing, a blow, a door, a social, crafting, a level gained), and
  Coupler cuts it out of the text and plays CoffeeMUD's sound for it
  from coffeemud.sf2. Each is a pair in the hooks list (`msp.sound` =
  `"fireball.wav"`), every one listed from the start, so any can be
  given a sound of the player's instead; the hook editor says which of
  CoffeeMUD's sounds plays while none is chosen. They're made ready in
  the background when the game's sounds come on, so the first is on
  time. A character whose SOUNDS is off (made before this, or in
  another client) gets one line at login saying to type SOUNDS; a new
  character made in Coupler has it on. Nothing is ever downloaded for
  them. On the web, the sounds come out of the text and play where the
  mirror has a hook on them.
- **CoffeeMUD's sounds in the hook editor** (2026-10-23): every sound
  in coffeemud.sf2 can be chosen for a hook, under "CoffeeMUD's sounds"
  in its menus: the one-shots for SFX (each blow and creature in its
  three kinds, `Flesh hit (light)`, `Canine (pain)`), the ambience and
  weather loops for BGN and BGW. Each plays at the bank's own level
  through the SFX, BGN and BGW buses, captioned by its name. A one-shot
  is rendered by Neumetik MIDI the first time it's played, the loops
  taken from the bank at their exact loop points so they go round with
  no seam; both are kept in app data, so the next time is instant. The
  web mirror publishes those a hook uses.
- **coffeemud.sf2 comes with Coupler** (2026-10-22): CoffeeMUD's
  sounds as a SoundFont, 210 of them, each of the game's events on a
  key of its own (combat, creatures, spells, doors, socials, crafting,
  and 52 ambience and weather loops), made by ansiapps' Neumetik and
  dedicated to the public domain (CC0). It ships in the app
  (`scripts/coffeemud-sf2-fetch.py`, pinned by its SHA-256), with its
  recordings' authors credited in About; `soundbank.rs` reads its map
  (every MSP sound CoffeeMUD sends) and tests that every slot plays.
  Nothing plays it yet. Neumetik MIDI 0.3.0 comes with it, the version
  the bank was mixed on.
- **Neumetik MIDI 0.2.0, and SoundFonts through it** (2026-10-22): MIDI
  through a SoundFont is now played by Neumetik MIDI itself, in place of
  rustysynth (one fewer dependency), its own instruments filling in what
  a SoundFont hasn't. **DLS banks** (`.dls`, levels 1 and 2) can be
  added as SoundFonts are, and chosen in Plays through. A SoundFont or
  DLS bank is read as it comes in, and a damaged one, or a compressed
  `.sf3`, is turned away with the reason instead of failing when a hook
  plays it. The Music Editor's Instrument and Kit menus list what plays:
  the GS variation tones as banks of their own, and, through a
  SoundFont, its presets and kits by their own names.
- **Bundle ID `com.scorelessmusic.coupler`** on the Mac and the iPhone
  (2026-10-19), as every ansiapps app on Apple's platforms is released;
  Windows and Linux keep `com.ansiapps.coupler`. At launch the Mac moves
  what the old ID kept (the app data and the WebView's settings) to the
  new name, once and never over data already there (`moved.rs`).
- **The iPhone, begun** (2026-10-19): a phone screen, upright and played
  by touch, from the same code (`lib/formFactor.ts`, `src/phone.css`).
  48 columns and as many rows as show above the keyboard (`phoneFit` in
  `lib/stage.ts`); the bar at the top (the connection, Hang Up, the
  gear), the game output, a touch pad (`components/TouchPad.tsx`: the six
  ways, an exit the map knows marked in yellow and in words, Look, the
  last command back), the command line and the messages, each touched
  thing 48 px tall. Dialogs and the gear menu fit the phone; full
  screen, Quit, the keyboard list, backups, the update check and
  arranging are left out there. The iOS project (`src-tauri/gen/apple`):
  portrait only, iPhone only, Coupler's icon; no first-run warning
  there (desktop only: App Review turns down an app calling itself
  unfinished). VoiceOver found by UIKit there; About's
  Discord and Reddit left out until the iPhone can open a browser; a
  privacy manifest (`PrivacyInfo.xcprivacy`). Sounds play with the ring
  switch on silent. iOS 16 and later (the iPhone 8 on: Pocket TTS needs
  the A11's half-precision math). WavPack (`vendor/wavpack-sys`)
  is linked static so the iPhone build gets it. In a dev build, `coupler.phone` = `on` shows it on
  the Mac.
- **Keys on every platform** (2026-10-06): on Windows and Linux, Cmd is
  Ctrl, Ctrl+Cmd is Ctrl+Alt and Option is Alt, in what's matched and in
  everything that names a key, shown or spoken (`lib/keys.ts`). Before,
  Cmd+Period, Ctrl+Cmd+1/2/3 and Ctrl+Cmd+F couldn't be pressed there,
  and every hint said Cmd. A letter's found by its key's place when Alt
  changes it.
- **Gear → Quit Coupler** (2026-10-06, desktop): the sounds fade and the
  app closes, hanging up first (`app_quit`).
- **Taking turns with a screen reader** (2026-10-06): `screen_reader.rs`
  says whether one is running (VoiceOver's process; Windows'
  `SPI_GETSCREENREADER`; GNOME's setting or Orca), asked every 5
  seconds (`lib/screenReader.ts`). While one is, a key it reads pauses
  Coupler's voice until a second and a half after the keys stop
  (`voice.yieldTurn`), a say key takes the turn back, and the status bar
  doesn't hand the screen reader what Coupler's voice is saying. Gear →
  Speech… says whether one's running.
- **Every player race described** (2026-10-06): `scripts/creation-art.py`
  now reads every race CoffeeMUD documents as a player race (its
  `misc_help.ini` `<RACE>` entries; a game turns them on with `ENABLE`,
  as coffeemud.net does Pixie), following `extends` for sizes and stat
  changes, with weight and the game's help on each (`help`). The guide
  shows and says that help for a race the game's list gives no words
  for. Without Pillow the script keeps the pictures and remakes the
  facts.
- **Race portraits at their real size** (2026-10-06): the painter knows
  every player race's height and weight (`portrait.rs` `SIZES`), so a
  Pixie stands two feet tall, with wings, instead of a size made up from
  its name; a look for each (drow, duergar, orcs, faeries, the undead…).
- **Stats the game would refuse are dimmed** (2026-10-06): raising past
  a stat's most or spending points not left; pressed, the guide says why
  instead of sending it.
- **The account menu with no characters** (2026-10-06): a list read
  before its prompt came (the game sends them apart) now shows, so an
  empty account says No characters yet instead of Reading your
  characters… for good.
- **The login kept alive** (2026-10-06): while a character's being made
  or the account menu waits, a telnet NOP each quiet minute stops
  CoffeeMUD starting the login over after 3 minutes; at 8 minutes
  without an answer Coupler warns that the game hangs up at 10.
- **A ping, not "Let's see…"** (2026-10-06): while an answer renders,
  a soft high ping each second until it's spoken (the "Getting the
  answer ready" cue).
- **The map's @ blinks** (2026-10-06), swapping to inverted once a
  second; gear → Display… turns it off, and reduced motion never blinks.
- **Ambience always fades** (2026-10-06): stopping the sounds fades the
  BGN and BGW out instead of cutting them, and a loop's new volume
  glides to it.
- **A fight's end in one line** (2026-10-06): the moment it's over,
  whatever was waiting to be said is dropped and the narrator says how
  it ended (killed, fled, you fled, you died), how long it took and your
  health, with what it cost.
- **Neumetik MIDI** (2026-10-06): Coupler's synthesizer is now the
  `neumetik-midi` crate (MIT), ansiapps' synthesizer split out for games
  to use freely, in `src-tauri/vendor/neumetik-midi/` in place of
  `vendor/neumetik/`. `scripts/neumetik-sync.sh` copies it from the
  Neumetik repository's `midi/` (and nothing else from there). MIDI
  plays as before.
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
- The desktop build no longer warns that its chunk is over 500 kB: it
  loads from disk, so its limit is 1 MB (`vite.config.ts`); the web
  build keeps 500 kB.

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

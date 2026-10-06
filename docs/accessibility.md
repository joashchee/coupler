# Accessibility

**A high-priority convention, adopted 2026-09-30 (CLAUDE.md rule 10).**
Coupler should be the most immersive way to play CoffeeMUD, for every
player, sighted or not. MUDs are text, which makes them one of the few
game genres that can be played fully by ear as well as by eye: through
a screen reader, through a voice of the client's own, or on the screen.
Most clients treat anything past the screen as an afterthought. Coupler
treats the game as one experience of sound, speech and words that
carries it whether the player watches, listens or both. What a player
who can't see the screen needs (the facts in words, sounds that tell,
speech that never floods) is what makes the game more immersive for
everyone.

## The rule

**Every UI element and every feature is designed to be heard and used
without sight, before it's drawn for the eye, so it works for every
player.** For each change, ask
the questions below. A feature that fails one isn't done.

1. **Can it be done with the keyboard alone?** Every control is reached
   with Tab, worked with Return or Space, and left with Esc. Nothing
   needs a mouse, a hover or a drag. A mouse-only convenience (clicking
   a room on the map picture) is allowed only beside a keyboard way to
   do the same thing (Find a room).
2. **Does VoiceOver say what it is and what it did?** Real HTML controls
   (`button`, `input`, `label`, headings, lists) before ARIA. An icon
   button has an `aria-label`. Every action ends in words in the status
   line (`role="status"`) or the error line (`role="alert"`), which is
   the existing "feedback for every user activity" convention doing
   double duty.
3. **Is anything said only by a picture, a color or a position?** Then
   say it in words too. The map picture is `aria-hidden` *because* the
   words above it say everything it shows. A gauge has a number. A
   colored state has a label.
4. **Is there too much speech?** A sighted player skims; a screen-reader
   user hears every word, in order. Don't announce what the game text
   already said. Don't put decoration (box-drawing frames, ASCII art,
   repeated labels) where it will be read. Give control over verbosity
   rather than guessing.
5. **Does it survive 200% text size, low vision and high contrast?**
   Text/background pairs come from `docs/ansiapps-color-contrast.md`
   (4.5:1 at least, 7:1 for body text); focus rings are 3:1 against
   what's behind them on *every* surface they can land on; layouts
   reflow rather than clip; motion respects `prefers-reduced-motion`.
6. **Where does focus go?** Opening something moves focus into it;
   closing it puts focus back. Focus never lands on something hidden.
   After connecting, focus is on the command line. Esc always returns
   there.

Test with **VoiceOver** (⌘F5) on macOS before calling a UI change done:
the App Testing checklist has an Accessibility section, and new features
add their own items to it. When Windows arrives: NVDA first, then JAWS.

## What's in place (2026-09-30)

| Area | How it works by ear, by keyboard and at low vision |
|---|---|
| Launch | Focus starts on the first way to play; Return connects. The button's description (what's different about that game) is read with it (`aria-describedby`). |
| Game output | `role="log"`, focusable, so arrow keys scroll it, but not itself live. New lines are read as they arrive, without moving focus from the command line, through a hidden live region of their own (`aria-live="polite"`, additions only, "Game output, as read") that's given only the kinds of line the player keeps on: the game's lines, talk, lines during a fight, the prompt, their own commands, Coupler's messages (gear → Speech…, `lib/speech.ts`; the kinds come from `src-tauri/src/speech.rs`). Own commands are off by default (the screen reader read them as they were typed). The prompt is read only when it changes, and a finished prompt never. Off while Coupler's own voice speaks and while reviewing. |
| Speech dialog | Gear → Speech…: one named box per kind, each change at once and kept (`coupler.speech`), Esc and focus back as in every dialog. |
| Characters' Voices | Gear → Characters' Voices…: in Immersive, each speaker has their own voice, so who's talking is heard without their name being the only clue (the name is still in the line, and in Heard). The list is words only: name, NPC or Player, the voice in short, and an Edit button named for the character and that summary; Search reads how many match; new arrivals aren't read out. The editor's controls each have a visible label; pitch and speed are sliders that say their percent and speak a line in the new voice when let go; Quiet keeps a noisy NPC's lines in Heard but silent. Esc closes only the editor, focus back to the list. The voice menu groups voices by engine (built-in Flite and Pocket TTS, the system's), each group's Automatic naming its pick; a built-in voice is as loud as the Mixer's VOX, and Cmd+Period and urgent speech cut off whichever engine is talking. |
| Three ways to play | Gear and Ctrl+Cmd+1/2/3: **Terminal** (the game output alone, as tall as the screen: the plainest page for VoiceOver), **Immersive** (sound first, Coupler's own voice, no screen reader needed), **Workshop** (everything, arranged by the player). The choice is a radio group; switching says the new one in the status bar (and by voice in Immersive) and puts focus back on the command line or the first way to play. |
| Immersive mode | Once a character is in the game, the game's prompt isn't written (unfinished questions are): it would only repeat what Here, the cues and the say keys give. Self-voicing through the system's speech (WebKit `speechSynthesis`), so a player needs no screen reader: the ways to play are spoken as they get focus, the login is read line by line (art skipped) until the character is in the game, then only tells, the group, says in the room, problems and the say keys (Cmd+Shift+L where, V vitals, E the fight, O what the game said since the last command; Cmd+Period stops it). Everything else is a sound cue (`lib/earcons.ts`): exits as notes placed left and right and high and low, footsteps, a new room, area or landmark, a heartbeat under half health, blows, healing, low mana and movement, the fight starting, weakening and ending, a ping before a tell. The game output isn't a live region here (the voice speaks for itself; VoiceOver would double it). Everything said is also written in Heard, and Here shows the room, a compass and the vitals as words beside the pictures (compass and bars hidden from assistive technology). Limits today: the gear menu and the dialogs aren't self-voiced; they still want sight or VoiceOver. |
| Arranging the screen | Workshop only: gear → Move and resize everything (off by default). With it on, everything on the screen has a button named "Move the …" just before it and "Resize the …" just after it (the game output has no resize). The arrow keys move or resize by one character, Home puts it back, and each press says the result in the status bar (the column and row, or the size in columns and rows). Dragging says it once, on letting go. The corners aren't drawn, so for a sighted mouse user the pointer's shape is the cue; a keyboard user gets the usual focus ring, and a screen reader the names. With the mode off the corner buttons don't exist: no extra Tab stops. The Tab and reading order never changes with the arrangement. Gear → Reset to Default Layout puts everything back, after asking. Nothing depends on arranging: the default layout is complete. |
| Command line | Labelled "Command", or "Password" while the server echoes (and then never echoed, logged or kept). The hint is tied to it. |
| Status and errors | In the status bar along the bottom, always in the same place. A message longer than its two lines is cut on screen but read whole. The pointer's position there is hidden from assistive technology. Status is `role="status"`; errors are `role="alert"`. Connecting, walking, arriving, stopping, saving a landmark all say so. |
| Where am I? | A button and ⌘⇧L. Says the room, the area, and every exit with what's behind it, from the map. Pressing it again says it again. |
| Map | Where you are, then the picture, then landmark and find a room. The exit list was removed from the screen (2026-09-30), so the exits with what's behind each are a sentence under "Where you are" that only a screen reader meets, and Where am I? says them. Walk goes there by itself; Directions reads the way ("3 north, east") for players who'd rather walk it. The picture is hidden from assistive technology. |
| Map colors | Each explored room is a tile in its terrain's color (24 terrains, 15 colors, alike ones sharing), with a key of the terrains in view. The brackets and mark on a tile are black or white, whichever passes on it in the contrast list. Color is never the only cue: the terrain is in words under Where you are, in each Find a room result, and in the tile's tooltip. |
| Dialogs | Closed dialogs are `inert` (unreachable); opening moves focus in, Esc closes, focus returns. |
| Hooks | The total in the status bar is a button named "Hooks: N. Show the list" (also Cmd+Shift+H); it isn't a live region, since it changes all the time while playing. The list is words only, name then value per row, with a labelled search that says how many match; the list takes focus so the arrow keys scroll it. The Hooks list and Filtered list tabs are a real tab list (Left and Right switch; the chosen one has a bullet, not just a color). Each row's box is named for what ticking it does, a status says where the name went, and focus moves to the row that took its place. |
| Assets and triggers | Files can be dropped on the window, and also added from the keyboard: the Hooks list's Add Assets button opens the system's file chooser. What was added, and each file refused with why, is said in the status bar. Each row's choices are named for their pair ("Sound effect for room.info.zone \"Midgaard\""), and the picture's column and row fields say their range; a change is saved at once and said. A picture that appears (an image, or ANSI art, which is a picture made of characters and so is named, not read out character by character) is said once by name in the status bar, since it says nothing to someone who can't see it. Cmd+Shift+S stops every sound and the music and closes the pictures: music can drown out VoiceOver. The Mixer's volumes are sliders named "Sound effects volume" and "Music volume" that say their percent, moved by the arrow keys (1) and Page Up and Down (10); each hook's volumes are number fields that say their range. The music playing shows on the menu bar as a button (it opens the Mixer), not a live region, so it isn't read out each time it changes. |
| Smaller sounds | After WAVs are added, a dialog asks whether to keep them smaller: its title is the question, each file is one line in words (name, both sizes, the format, what it saves), and a sentence says what Opus or WavPack means, so nothing depends on a table or a picture. Make Smaller and Keep are plain buttons; Esc keeps the WAVs; focus goes back where it was. The result goes in the status bar. |
| Trying assets | The Hooks list's Assets tab ends each row with one button: Play, which becomes Stop while it plays (named "Play door", "Stop door", "Play a sample tune through Arachno", and pressed while playing), or Show… for a picture, which opens it over the list with its kind and size in words first ("ANSI art, 80 characters wide and 25 rows tall"). One asset plays at a time, and closing the list stops it. Music that has to be rendered first shows in the activity area. A MIDI file's "plays through" menu is named for the file ("town plays through") and lists Neumetik first, then the SoundFonts; a SoundFont that's gone shows as missing, Neumetik playing in its place, and the change is said in the status bar. Each sound is captioned in Heard ("♪ Trying door"). |
| Create Asset | A dialog over the Hooks list. Make is a radio group (ART, BGM, each with what it is in words); "Describe it" is a labelled field whose hint, tied to it, lists the words it understands; Size is a labelled menu (pictures only). Enter in the field creates, and the activity area shows it working. What was made is said in the status bar with what it is in words ("A forest at dawn, high fantasy." or "A lively merry piece in E Mixolydian, 128 beats a minute..."), and the same words stay beside it; the picture is one image named for them, so nothing is said only by the picture. Music plays once it's made and has a Play/Stop button; closing stops it. |
| Hook editor | A Hooks list row is its box, its name and value, what it sets off in words ("SFX door 40%, BGM town, loop, 75%", or Nothing) and an Edit button named for its pair and that summary. Edit opens the hook editor over the list: the list goes inert, focus moves in, Esc closes only the editor and focus goes back to that row's Edit. Every control has a visible label and a spoken name with its range ("Music volume, 0 to 100 percent"); a change is saved at once and said in the status bar. Background noise (BGN) shows only for a room's pairs and weather sound (BGW) only for coupler.weather, with a line saying where they apply, so nothing offered does nothing. This Room puts the current room's id in Search and says so; it's dimmed, with a tooltip saying why, before the room is known. The loops fade in and out rather than cut, and nothing is said each time one changes (it would talk over the game); the Mixer lists what noise and weather are playing, in words, and Cmd+Shift+S stops them with everything else. |
| Combat or Explore mode | On the menu bar while a character is in the game, in words: "Combat mode" or "Explore mode". The color only repeats it (a white-on-red bar for Combat in the ANSIapps theme, the warning color in modern). It's `role="status"`, so VoiceOver says the new mode when a fight starts or ends, once each, and never mid-fight (`combat.rs` only changes it then). Not a Tab stop: it's read in the menu bar's order, after the app's name. |
| Hidden details | A long look at a room (LL) shows the scenery a player can look at or get only by coloring its words. After a long look Coupler says them in words: in Immersive the narrator says "Hidden details found.", the description, then the details by name ("No hidden details." when there are none); with VoiceOver, "Hidden details: …" is read after the room. The colors stay as the game sent them for sighted players. |
| Room pictures | Immersive's Here and Workshop's Picture panel show a picture of the room (Coupler's painter's, 2026-10-03). It's hidden from assistive technology with its caption, never announced and never a Tab stop: it says nothing the room's own description and Here's words don't (the terrain is in words under the room's name). Who made it and what it shows are in a caption for sighted players and in gear → Pictures… for everyone. Pictures… is two labelled groups of choices (Painted by, Style), each choice read with what it does; Style is disabled, not hidden, with pictures off. A new picture makes no sound and no message: fewer words. |
| Who's online | Cmd+Shift+W says, in the status bar and by the narrator when Coupler's voice is on, how many others are online, their names (idle ones marked), and the last login or logout within 15 minutes. The game's login and logout lines aren't written or read: a doorbell rises for a login and falls for a logout in every way to play (pitch direction, not the name, tells which), captioned with the name in Heard; the name is one keypress away. Not a live region: logins on a busy server would be a stream of speech. |
| Cues | Workshop's gear → Cues… (and a button in Customize the Screen): a list of every cue, each row its name, group and how it is now in words, and an Edit… button named with all three. Its editor is a labelled row per control: Plays and Written are checkboxes, Volume and Pitch sliders read with their values in words, Caption's words a text field whose example reads beside it. A cue's caption is its visual half (Heard, with sound captions on), so a deaf player can turn a cue's sound off and keep the caption, or rename a caption to what it means to them. Play It plays a cue even while its sound is off; changes are silent (nothing said or announced): fewer words. Esc closes the editor back to the list, focus on the Edit… it came from. |
| Before You Play | An optional tutorial for Immersive, offered by a button beside the ways to play (Shift+Tab from the first way to play; Immersive's intro says so) and in the gear menu while not connected. A dialog: the narrator teaches each lesson aloud and its words are written above the practice game's output; focus goes to its labelled command box at each lesson, and the task is tied to the box (`aria-describedby`), so VoiceOver reads it with the box. The say keys, Cmd+Period and Option+Up/Down/End work in it as in the game (App's own shortcuts are off under a dialog, so nothing reaches the real game). Every sound's caption and every line said for the game is written in its Heard list, so a deaf player can follow it. Return on an empty line says the task again, or goes on once it's done; Back, Hear It Again, Next and Close are plain buttons; Esc leaves and focus goes back. Nothing in it is timed against the player: a lesson waits. |
| Account menu | A dialog in place of CoffeeMUD's screen of letters (`AccountMenuDialog.tsx`, the game's menu read by `account.rs`): each character a button named in full ("Play Bob, level 12 Human Fighter, last played…"), focus on the first; New, Retire, Quit ask in words, then answer the game's own y/N; password, e-mail, import and export go to the command line, where a password is hidden. The narrator says the menu in a few words, and the characters once read; the game's menu lines aren't read or said while it's open. Esc or Type Instead hides it until the menu comes back (Cmd+Shift+G shows it again); focus goes to the command line. |
| Terminal's Control Panel | The right of the screen in Terminal (`ControlPanel.tsx`): a checkbox for each thing to show beside the game (the map, You, Picture, Heard, the Journal, Log and Hooks buttons), each named with what it shows; help at its top says how to arrange. Arrange is a checkbox: then each shown thing's Move and Resize corners are buttons, the arrow keys moving or sizing by a character, Home for the default, and the status bar says where it went. The game output and the command line never move. |
| Making a character | CoffeeMUD's character creation is screens of text, then a one-line question. When a new account or character is on the way, a guide opens over every way to play (`creation.rs` finds each question, its choices and the game's text; `CreationDialog.tsx`): the steps in words with the current one in brackets and `aria-current`, what's settled so far, the question in Coupler's few words (a live region, unless Coupler's voice says it) with a line of help, and the answers as buttons, each read with the game's own line about it. Up and Down move through them, Home and End to the ends; focus goes to the first answer at each new question, and back there when the guide is shown again. The classes that suit the character's best stat, which the game marks only by color, say "suits you" in words. The stats are a table with a Raise and a Lower button per stat, named for it, the points left and the classes they qualify for in sentences beside it. A password goes in a hidden field and is written in the output as stars, never kept. The game's full text is behind a button (`aria-expanded`), and the terminal keeps all of it. In Immersive the narrator says the stage, the question and how many answers, then each answer as it's reached; after a point spent only what changed ("Strength 11. 6 points left."); a soft page turn marks each new question and a fanfare the character's first step into the game. While the guide is open the game's lines aren't read or said. Type Instead (or Esc) hides it, Cmd+Shift+G brings it back. |
| Keyboard help | Gear → Keyboard lists every shortcut. |
| Focus rings | Visible on every surface in both themes (black on the light-gray bars of the ANSIapps theme, where the theme's white ring was under 3:1). |
| Theme | The ANSIapps theme is high contrast by construction: every pair is from the contrast list. The game output keeps the game's own colors (rule 7); see "Open problems". |

## Open problems, in priority order

These are scheduled on the roadmap. They're listed here so
nobody mistakes "in place" for "finished".

1. **Not yet tested by players who play by screen reader every day.**
   Everything above is built to the standard and unverified in use.
   Finding two or three CoffeeMUD players who play that way to try it
   is the most valuable thing on this list.
2. **Speech control.** A busy room floods a live region. Needed: a key
   to stop speech (VoiceOver's Control works, but a client-level
   "quiet" is better), a review mode to step back through output line
   by line without losing new lines, and per-kind choices (read
   channels? read combat? read my own commands back?). Built: the
   quiet key, review mode, and per-kind choices (2026-10-03). Still to
   come: room lines as a kind of their own, once it's confirmed live
   whether `room.info` comes before the room's text.
3. **The game's own colors can be unreadable.** Rule 7 says the output
   shows CoffeeMUD's colors, and dark blue on black is 1.6:1. Needed: a
   low-vision option that lifts any foreground below 4.5:1 on black to
   the nearest color that passes, and a plain "no colors" mode. Both
   are the user's choice, so they don't break rule 7.
4. **Text size.** The ANSIapps theme pins 16px for the bitmap font. A
   low-vision player needs 2x and 3x (integer scaling keeps the font
   crisp) and the modern theme needs a size setting.
5. **ASCII art and tables.** The login banner, `SCORE`, `WHO`, maps and
   the in-game `AUTOMAP` read as noise. Needed: detect runs of
   non-letters and offer to skip them; recommend `AUTOMAP OFF`
   (`docs/coffeemud-gmcp.md` section 7); use GMCP to offer the same
   facts as words (vitals, room, group).
6. **Sound as information.** Earcons for tells, low health, a walk
   arriving, a failed move: a sound is quicker than words for every
   player, and anyone playing by ear relies on cues more than speech. Comes with the sound phase.
7. **The prompt.** It's re-rendered as the unfinished last line, and a
   live region may re-read it. With GMCP vitals on a key ("say my
   health"), the prompt can be left out of speech. Built (2026-10-03):
   read only when it changes, and a kind that can be turned off.
8. **Self-voicing.** Many players prefer a client that speaks
   through the system voice itself (as VIP Mud does) over a screen
   reader reading a web view, because it can queue, interrupt and
   prioritize. Prototyped through WebKit's `speechSynthesis`
   (Immersive); next, `AVSpeechSynthesizer` from Rust so the voice goes
   through the mixer (a `docs/platform-parity.md` row).
9. **Braille displays.** They follow the screen reader's cursor; the
   review mode (2) is what makes them usable.
10. **The first-run warning and startup screen** are native and standard
    but haven't been listened to yet.

## What the server gives us

- Nothing screen-reader specific is acted on: CoffeeMUD parses the MTTS
  screen-reader bit (64) but never uses it (`docs/coffeemud-gmcp.md`,
  section 5). So accessibility is the client's job, and GMCP is the
  tool: vitals, status, room, exits, group and channels all arrive as
  data that can be spoken on demand instead of scraped from a prompt.
- Useful in-game settings to recommend at first login (as suggestions
  the player types, never sent for them): `AUTOMAP OFF`,
  `CONFIG BRIEF` for short room descriptions on repeat visits,
  `AUTOEXITS ON`, `CONFIG NOBATTLESPAM`, `LINEWRAP` disabled so the
  screen reader reads whole sentences. Check each against the game's
  `HELP CONFIG` before it goes into any UI text.

## Reference clients for accessibility

From a survey of MUD clients: **VIP Mud** (built to be played
by ear; self-voicing, sound packs), **MUSHclient with the MushReader
plugin** and **Mudlet's screen-reader support** (what most Windows
players with a screen reader use today), **TinTin++** in a terminal with a screen
reader, and **MUDRammer** on iOS with VoiceOver. What they share: speech
interrupt, output review, sound triggers, and staying out of the way.

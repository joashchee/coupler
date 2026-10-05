# CoffeeMUD's socials: what a player sees, and how to match it

Studied 2026-10-02 from CoffeeMUD's source, snapshot `c1e556f`
(`reference/CoffeeMud/`). Paths below are under
`com/planet_ink/coffee_mud/`. Nothing here was checked against the live
server yet; section 6 lists what to confirm.

The list of lines to match is **`docs/coffeemud-socials.tsv`**, made by
`scripts/socials-patterns.py` from the game's `resources/socials.txt`
(Apache-2.0, Bo Zimmerman). Run the script again when the snapshot
changes.

## 1. Socials send no GMCP: they're matched from the text

Confirmed. A social is `Common/DefaultSocial.java`: `invoke` and
`invokeIntern` build one `CMMsg` (source, target and others messages)
and `Room.send` it. Each MOB's `executeMsg` prints its part through
`tell`, as text. Nothing on that path calls `sendInlineCommand`, and
`Libraries/CMProtocols.java` has no social package (its packages are
`char.*`, `room.*`, `comm.tick`, plus `comm.channel` from
`CMChannels`, `room.enter`/`room.leave` from `StdMOB`, and the say
packages from `Commands/Say.java`). `grep -i social` over the GMCP code
finds nothing.

Two things that look like exceptions, and aren't a way in:

- **A social on a channel** (`GOSSIP ,smile`, or `:smile`,
  `Libraries/CMChannels.java`) does come as GMCP: `comm.channel` with
  the line as `msg`, since Coupler asks for `Comm 1`. It says nothing
  about it being a social, though: it's just the line, and it already
  lands in the hooks database as `comm.channel.msg`. The same patterns
  match its text after the `[GOSSIP] ` part.
- **MSP**: a social with a sound file (40 of them, column 7 of
  `socials.txt`) appends ` !!SOUND(bark.wav V=10 P=50) ` to its lines
  (`CMProtocols.msp`). `CoffeeFilter` keeps it only for a session that
  agreed to MSP. Coupler refuses MSP (`telnet.rs`), so the tag never
  comes, even when the player's SOUND setting makes the server offer it
  again (`DefaultSession.initTelnetMode`). The sound names are in
  the list's `sound` column as the game's own suggestion.

So a social hook is a text hook, like the weather (`ambient.rs`).

## 2. Who sees which line

Each line of `socials.txt` is one social form, tab-separated:

| Column | What |
|---|---|
| 1 | Two letters: the doer's action type, then the others' (below) |
| 2 | The name: `SMILE` (alone), `SMILE <T-NAME>` (at someone), `SMILE SELF`; also `<I-NAME>` (an item in the room), `<V-NAME>` (carried), `<E-NAME>` (worn), and a word after the target (`KISS <T-NAME> NECK`) |
| 3 | What the doer sees |
| 4 | What everyone else in the room sees |
| 5 | What the target sees (only when it's someone) |
| 6 | What the doer sees when the target isn't here (`Who's that?`) |
| 7 | Sound file (MSP) |
| 8, 9 | Who may use it (a mask), flags (`CONFIRM`, `TARG_CONFIRM`): none set in the stock list |

The snapshot has **843 forms of 338 socials**, giving **2,225 lines**:
843 seen by the doer, 782 by others, 308 by a target, 292 when the
target is missing. An empty column prints nothing (`BEARHUG` alone
only tells you `Whom do you wish to bearhug?`).

The action type decides **who notices**: `W` words, `S` a noise, `M`
hands, `O` a big movement, `Q` a quiet one, `V` visual, `T` touch. A
blind character misses a visual social and a deaf one a noise (a sleeping
one may miss it too). Someone they can't see becomes **"someone"**
(something, for an item).

## 3. How a line is filled in

`Libraries/CoffeeFilter.java` (`fullOutFilter`) fills the tags for each
reader:

| Tag | The doer reads (S tags) / target reads (T tags) | Everyone else reads |
|---|---|---|
| `<S-NAME>` `<T-NAME>` | you | the name: `Alice`, `a goblin`, `someone`, or on the role-playing port a description until you're introduced |
| `<T-NAMESELF>` | you (yourself when it's you) | the name |
| `<S-YOUPOSS>` `<T-YOUPOSS>` | your | the name's: `Alice's` |
| `<S-HIS-HER>` | your | his, her, its, their (the gender's own word) |
| `<S-HIM-HER>` `<S-HE-SHE>` | you | him/her/it/them, he/she/it/they |
| `<S-HIM-HERSELF>` | yourself | himself, herself, itself, themself |

- **(s), (es), (ys)** after a verb: dropped after "you"
  (`You curl into a ball`), kept for anyone else (`Alice curls`).
- **The first letter is capitalized** (`A goblin smiles.`), and after
  `. ` when two spaces or a tag follow.
- **Colors**: three socials color a word (`BLUSH`, `HOT`, `HOT
  <T-NAME>`). Match the line without color, as Coupler's lines already
  give it.
- **Wrapping**: the server wraps at the width Coupler reports by NAWS,
  less 2 (78 for the 80-column game output). Most social lines are
  shorter; a long one comes as two lines and must be **joined** before
  matching, the way `ambient.rs` joins a wrapped weather line.
- **Where it starts**: at the start of a line, or just after a prompt's
  closing `>` (the same rule as the weather).

## 4. The list: `docs/coffeemud-socials.tsv`

Tab-separated, one row per line a social prints, in `socials.txt`'s
order:

| Column | What |
|---|---|
| `social` | The command: `BARK` |
| `full name` | The form: `BARK <T-NAME>`; the key a hook would use |
| `form` | alone, someone, self, item in room, item carried, item worn, or the word after the target |
| `seen by` | `you` (you did it), `others` (someone did it near you), `target` (someone did it to you), `fail` (you did it at someone not here) |
| `sound` | The game's own sound file for it, if any |
| `text` | The line, `*` for a name or a pronoun: `* barks at * scaring * silly -- SHAME ON YOU!!` |
| `regex` | The whole line as a regular expression (Rust `regex`): `(?i)^…$`, names `(.+?)`, pronouns `(\S+)`, any run of spaces `\s+` (so a joined wrap still matches) |
| `also matches` | Other forms whose pattern also takes this line (below) |

## 5. Matching them well

- **Whole line, ignoring case**, after joining a wrap and from the
  start or after a prompt. A pattern never matches inside a longer line.
- **Lines that are the same**: some socials print the same words
  (`HANDSHAKE` and `HSHAKE`; `AWE <T-NAME>` and `AWE SELF` both give
  `* stands in awe of *.` to others). The `fail` lines are the worst:
  `Sorry, friend, I can't see that person here.` is nine socials'.
  Such a line can only say "one of these".
- **Lines that swallow others**: a wildcard name can take extra words,
  so `* kisses *.` also takes `Alice kisses Bob passionately.`. When
  several match, **the one with the most fixed text wins**. 200 rows
  list another form in `also matches`.
- **Check the name**: the name a pattern caught for an `others` or
  `target` line should be someone in the room (GMCP `room.players`,
  `room.mobiles`) or `someone`. That rules out most false matches,
  including another player's EMOTE copying a social's words, which no
  pattern can tell apart otherwise.
- **The doer's own lines** come right after you send the command, so a
  `you` or `fail` line can also be checked against the last command
  sent (`smile`, `smile bob`).
- A **fail** line on its own (`They aren't here.`) is said by other
  commands too; treat it as a social's only right after sending one.

## 6. To confirm on the live game

- coffeemud.net's socials may differ from the snapshot: archons can
  add and edit them (the web admin, the `SOCIALS` editor), and an item,
  room or area can carry its own (`Abilities/Properties/Prop_Socials.java`).
  `SOCIALS` in the game lists the names there; compare with the list.
- That a long social wraps at 78 columns, joined back the way the
  weather is.
- What a blind, deaf or sleeping character gets (nothing, or
  "someone").
- That no `!!SOUND(` ever reaches Coupler with SOUND turned on.

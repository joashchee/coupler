# CoffeeMUD's commands: which print a block, which answer in one line

Studied 2026-10-04 from CoffeeMUD's source, snapshot `c1e556f`
(`reference/CoffeeMud/`), every file in
`com/planet_ink/coffee_mud/Commands/` (310 classes). Nothing here was
counted on the live server yet; section 6 lists what to confirm.

Coupler uses it for **the narrator's answers**: in Immersive, a
command's one-line answer isn't written in the game output, and the
narrator says it in short ("You sit down and take a rest." is
"Sitting."). `src-tauri/src/echo.rs` holds section 3's list (`COMMANDS`)
and the narrator's words for each line (`ECHOES`); Workshop's gear →
Narrator's Answers… (`components/EchoesDialog.tsx`, `lib/echoes.ts`,
`coupler.echoes`) lets the player change them.

## 1. How a command answers

A command's `execute` prints in one of three ways:

- **`mob.tell(...)`** or **`postCommandFail(...)`**: a line to the
  player alone. Most failures ("Get what?", "You don't see 'x' here.").
- **A `CMMsg`** sent to the room (`CMClass.getMsg(mob, target, tool,
  code, "<S-NAME> sit(s) down.")`): each one in the room sees their
  own version, the doer's with `<S-NAME>` as "You" and the `(s)`
  dropped (`CoffeeFilter.fullOutFilter`, as for socials:
  docs/coffeemud-socials.md). One line, unless something in the room
  answers it too (a full belly after EAT: `StdFood` adds "You are
  full.").
- **A block**: a `StringBuilder` of rows (`padRight`, `CMLib.lister()`'s
  columns, `^H` headers), a room (`postLook`), a help entry, a file
  (`credits.txt`), or a menu that waits for answers.

**Which command a line is.** `EnglishParser.findCommand` takes the
first word (or the first character when it isn't a letter or a digit:
`:waves` is `:`) and tries, in order: an exact access word
(`CMClass.findCommandByTrigger(word, true)`), a skill, a social, a
channel name, a command journal (BUG, IDEA...), then *prefixes* of
skills, commands, socials and channels. The prefix pass walks a
`Hashtable`, so which command an abbreviation lands on isn't knowable
from outside. Coupler only trusts the exact pass: `sit` and `r` are SIT,
`sitt` is nobody's.

**Which lines are the answer.** CoffeeMUD answers each command, then
sends the prompt again. `echo.rs` queues the command each line sent
names and gives the next read with lines of the game's to the oldest
one waiting (a block's command takes its turn too, so the next one's
answer is still the next one's).

## Which list a command goes in

**One line** means the answer is a sentence: one per thing when the
command takes ALL or a number (GET ALL is a sentence per item), and
sometimes a second sentence the thing adds (EAT's "You are full.").
Talk the command says too (AUTOASSIST says "I will now assist in
combat.") is talk, the journal's. **A block** means rows, columns, a
box, a room, help text or a menu, or the command starts something that
prints one (a move shows the room; KILL starts a fight's rounds).

## 2. Commands that print a block, most lines first

Lines are a typical answer on an 80-column screen in the stock game (not the most it can print: a list grows with the game and the character). Read from the source, not counted on the live server.

| # | Command | Words | About | What it prints |
| --- | --- | --- | ---: | --- |
| 1 | Topics | TOPICS | 700 | Every player help topic (2,791 in the stock files), four columns |
| 2 | WillQualify | WILLQUALIFY | 150 | Every skill a class gets, by level |
| 3 | Qualify | QUALIFY, QUAL | 120 | Every skill you can gain now or later, by level, with costs |
| 4 | SocialsCmd | SOCIALS | 85 | All 338 socials, four columns |
| 5 | Commands | COMMANDS | 60 | Every command you can use, four columns |
| 6 | Areas | AREAS | 60 | Every area, in columns, with a header |
| 7 | Achievements | ACHIEVEMENTS | 50 | Achievements, done and not, with progress |
| 8 | Abilities | ABILITIES, ABILITYS, ABLES | 40 | Every ability you have, grouped by kind, with proficiency |
| 9 | Deities | DEITIES, GODS, DEITY | 40 | Every deity and its description |
| 10 | ColorSet | COLORSET | 35 | The color menu: every color slot and its setting |
| 11 | Channels | CHANNELS | 35 | Every channel and whether it's on, a table |
| 12 | Stat | STAT | 30 | Your statistics in full (STAT; staff see anyone's) |
| 13 | Skills | SKILLS, SK | 30 | Your skills, in columns, with proficiency |
| 14 | ListCmd | LIST | 30 | A shop's goods or a banker's accounts, a priced table (LIST) |
| 15 | Config | CONFIG, AUTO, CONFIGURATION | 30 | Every setting (AUTO... and the rest), a table |
| 16 | Help | HELP, ? | 25 | A help entry: usage, examples, text |
| 17 | Score | SCORE, SC | 25 | Your character sheet in a box: stats, level, money, alignment |
| 18 | ClanDetails | CLANDETAILS, CLAN | 25 | A clan's details: members, ranks, rules |
| 19 | Kill | KILL, K, ATTACK, MURDER, FIGHT | 20 | Starts a fight: a round of blows every few seconds until it ends |
| 20 | Assist | ASSIST | 20 | Joins a fight (as KILL) |
| 21 | Equipment | EQUIPMENT, EQ, EQUIP | 20 | Every wear slot and what's in it |
| 22 | Spells | SPELLS, SP | 20 | Your spells, in columns |
| 23 | CalendarCmd | CALENDAR | 20 | The calendar: months, events |
| 24 | MOTD | MOTD, NEWS | 20 | The news and messages of the day |
| 25 | Replay | REPLAY | 20 | A channel's last lines |
| 26 | History | HISTORY | 20 | Your last commands, numbered |
| 27 | HelpList | HELPLIST, HLIST | 20 | Help topics that match a word |
| 28 | Remort | REMORT | 20 | Remort's questions and what it resets |
| 29 | Credits | CREDITS | 20 | The credits file |
| 30 | Who | WHO, WH | 15 | Who's online, a framed table (Coupler's own WHO is hidden) |
| 31 | Where | WHERE | 15 | Who's in your area, and where |
| 32 | ClanList | CLANLIST, CLANS | 15 | Every clan, a table |
| 33 | Top | TOP | 15 | The top players, by category |
| 34 | Questwins | QUESTS, QUESTWINS | 15 | Quests you've won, or the quest list |
| 35 | AutoInvoke | AUTOINVOKE | 15 | Your automatic skills and whether each is on |
| 36 | Email | EMAIL | 15 | The mail menu and your messages |
| 37 | Account | ACCOUNT | 15 | Your account's menu and characters |
| 38 | Report | REPORT | 15 | Your skills and how good you are at them |
| 39 | I3Cmd | I3 | 15 | The Intermud-3 network's lists |
| 40 | IMC2 | IMC2 | 15 | The IMC2 network's lists |
| 41 | ClanCreate | CLANCREATE | 15 | Clan creation, step by step |
| 42 | Inventory | INVENTORY, INV, I | 12 | What you carry, item by item |
| 43 | ClanKills | CLANKILLS | 12 | A clan's kills, a table |
| 44 | ClanPVPKills | CLANPVPKILLS | 12 | A clan's player kills, a table |
| 45 | Look | LOOK, LOO, LO, L | 10 | The room: name, description, exits, things and people; or a thing's description |
| 46 | North | NORTH, N | 10 | Moves, then shows the new room (as LOOK). The same for every direction below |
| 47 | South | SOUTH, S | 10 | As NORTH |
| 48 | East | EAST, E | 10 | As NORTH |
| 49 | West | WEST, W | 10 | As NORTH |
| 50 | Up | UP, U | 10 | As NORTH |
| 51 | Down | DOWN, D | 10 | As NORTH |
| 52 | Northeast | NORTHEAST, NE | 10 | As NORTH |
| 53 | Northwest | NORTHWEST, NW | 10 | As NORTH |
| 54 | Southeast | SOUTHEAST, SE | 10 | As NORTH |
| 55 | Southwest | SOUTHWEST, SW | 10 | As NORTH |
| 56 | Above | ABOVE | 10 | As NORTH |
| 57 | Below | BELOW | 10 | As NORTH |
| 58 | Aft | AFT | 10 | As NORTH |
| 59 | Foreward | FOREWARD, FORE | 10 | As NORTH |
| 60 | Portside | PORTSIDE, PORT | 10 | As NORTH |
| 61 | Starboard | STARBOARD, STB | 10 | As NORTH |
| 62 | Left | LEFT | 10 | As NORTH |
| 63 | Right | RIGHT | 10 | As NORTH |
| 64 | Back | BACK | 10 | As NORTH |
| 65 | Go | GO, WALK | 10 | Moves (GO north, WALK), then the new room |
| 66 | Run | RUN | 10 | Moves several rooms, showing each |
| 67 | Crawl | CRAWL, CR | 10 | Crawls a way, then the new room |
| 68 | Enter | ENTER, EN | 10 | Enters a portal or a vehicle, then the new room |
| 69 | Leave | LEAVE | 10 | Leaves a vehicle or a room, then where you are |
| 70 | Pull | PULL, DRAG | 10 | Drags someone or something a way, then the new room |
| 71 | Push | PUSH | 10 | Pushes something a way, then the new room |
| 72 | Flee | FLEE | 10 | Runs from the fight, then the new room |
| 73 | Wake | WAKE | 10 | Wakes up and looks around (the room) |
| 74 | Read | READ | 10 | What's written on a thing |
| 75 | Affect | AFFECTS, AFFECT, AFF, AF | 10 | What's affecting you, with durations |
| 76 | FactionList | FACTIONS, FAC | 10 | Your standing with each faction |
| 77 | Group | GROUP, GR | 10 | Your group, a table of members and their health |
| 78 | Train | TRAIN, TR, TRA | 10 | What you can train and what it costs |
| 79 | ClanWho | CLANWHO, CLWH | 10 | Your clan's members online |
| 80 | ClanVote | CLANVOTE | 10 | A clan vote and its options |
| 81 | WhoIs | WHOIS, FINGER | 10 | A player's details |
| 82 | ChanWho | CHANWHO | 10 | Who's listening to a channel |
| 83 | Alias | ALIAS | 10 | The alias menu |
| 84 | PollCmd | POLL | 10 | A poll and its answers |
| 85 | Rules | RULES | 10 | The rules file |
| 86 | Grapevine | GRAPEVINE, GV | 10 | The Grapevine network's lists |
| 87 | Prayers | PRAYERS | 10 | Your prayers, in columns |
| 88 | Chants | CHANTS | 10 | Your chants, in columns |
| 89 | Songs | SONGS | 10 | Your songs, in columns |
| 90 | Powers | POWERS | 10 | Your powers, in columns |
| 91 | Expertises | EXPERTISES, EXPS | 10 | Your expertises |
| 92 | Switch | SWITCH | 10 | Your characters, then the switch |
| 93 | WizList | WIZLIST | 10 | The staff |
| 94 | Auction | AUCTION | 10 | What's up for auction |
| 95 | ClanQual | CLANQUAL | 8 | What a clan asks of its members |
| 96 | AutoAffects | AUTOAFFECTS, AUTOAFF, AAF | 8 | Lasting effects you keep |
| 97 | Subscribe | SUBSCRIBE, SUBSCRIPTIONS | 8 | Your subscriptions |
| 98 | Languages | LANGUAGES, LANGS | 6 | The languages you know |
| 99 | Title | TITLE | 6 | Your titles, numbered |
| 100 | Vassals | VASSALS | 6 | Who serves you |
| 101 | Examine | EXAMINE, EXAM, EXA, LONGLOOK, LLOOK, LL, ID | 6 | A thing or person, looked at closely |
| 102 | View | VIEW | 6 | A shop item's description |
| 103 | Exits | EXITS, EX | 6 | Every exit, a line each |
| 104 | OutFit | OUTFIT | 6 | The starting gear handed out, a line each |
| 105 | Ver | VERSION, VER | 5 | The version, the copyright, two links and the uptime |
| 106 | Experience | EXPERIENCE, EXPER, XP, EXP | 4 | Experience, level and what the next takes |
| 107 | Time | TIME, DATE | 4 | The time, the date, the season and your birthday |
| 108 | Wealth | WEALTH | 4 | Your money, a line per currency |
| 109 | Formation | FORMATION | 4 | Your group's battle rows (FORMATION alone) |
| 110 | GConsider | GCONSIDER, GCOS, GCO | 4 | CONSIDER for each member of your group |
| 111 | Description | DESCRIPTION | 4 | Your description and how to change it (DESCRIPTION alone) |
| 112 | Retire | RETIRE | 4 | Retirement's warning and questions |
| 113 | Worth | WORTH | 3 | Your money and its worth, in a block |
| 114 | PlayerKill | PLAYERKILL, PKILL, PVP | 3 | Turning the PK flag on: a warning, a question, the answer |
| 115 | Prompt | PROMPT | 2 | Your prompt as it is (PROMPT alone) |

## 3. Commands that answer in one line

The answer is a sentence (or a sentence a thing, for ALL and numbers), never a block. These are `COMMANDS` in `src-tauri/src/echo.rs`; the narrator's words for their lines are its `ECHOES`.

| Command | Words | Note |
| --- | --- | --- |
| AFK | AFK | AWAY on or off (a second line if you missed tells) |
| ANSI | ANSI, COLOR, COLOUR |  |
| Activate | ACTIVATE, ACT, A, > |  |
| AutoAssist | AUTOASSIST | and a say (talk, the journal's) |
| AutoAttack | AUTOATTACK | and a say |
| AutoDraw | AUTODRAW |  |
| AutoExits | AUTOEXITS |  |
| AutoGold | AUTOGOLD |  |
| AutoGuard | AUTOGUARD, GUARD | and a say |
| AutoImprovement | AUTOIMPROVEMENT |  |
| AutoLoot | AUTOLOOT |  |
| AutoMap | AUTOMAP |  |
| AutoMelee | AUTOMELEE | and a say |
| AutoNotify | AUTONOTIFY |  |
| AutoRun | AUTORUN |  |
| AutoTellNotify | AUTOTELLNOTIFY |  |
| AutoWeather | AUTOWEATHER |  |
| Autoforward | AUTOFORWARD |  |
| Bid | BID |  |
| Borrow | BORROW |  |
| Brief | BRIEF |  |
| Buy | BUY | a line per item bought |
| ClanAccept | CLANACCEPT | then the clan channel's announcement |
| ClanApply | CLANAPPLY |  |
| ClanAssign | CLANASSIGN | then the announcement |
| ClanDeclare | CLANDECLARE | then the announcement |
| ClanDonateSet | CLANDONATESET |  |
| ClanExile | CLANEXILE | then the announcement |
| ClanHomeSet | CLANHOMESET |  |
| ClanMorgueSet | CLANMORGUESET |  |
| ClanReject | CLANREJECT |  |
| Close | CLOSE, CLOS, CLO, CL |  |
| CommandJournal | BUG, IDEA, TYPO, TASK, STUCK | BUG, IDEA, TYPO, TASK, STUCK (the stock journals) |
| Compare | COMPARE, COMP |  |
| Compress | COMPRESS |  |
| Consider | CONSIDER | a second sentence when your skills could help |
| Deactivate | DEACTIVATE, DEACT, DEA, < |  |
| Deposit | DEPOSIT |  |
| DieRoll | DIEROLL, DROLL |  |
| Dig | DIG |  |
| Disembark | DISEMBARK |  |
| Dismount | DISMOUNT |  |
| Display | DISPLAY, SHOW |  |
| Draw | DRAW |  |
| Dress | DRESS |  |
| DrinkCmd | DRINK, DR, DRI | a second line when you're slaked |
| Drop | DROP, DRO | a line per item (DROP ALL) |
| Duel | DUEL |  |
| Eat | EAT | a second line when you're full |
| Emote | EMOTE, ,, ;, : |  |
| Empty | EMPTY, EMP |  |
| Feed | FEED |  |
| Fill | FILL |  |
| Follow | FOLLOW, FOL, FO, F |  |
| Friends | FRIENDS | the list is one line |
| Gait | GAIT, NOGAIT | not implemented |
| Get | GET, G | a line per item (GET ALL) |
| Give | GIVE, GI | a line per item |
| Hold | HOLD, HOL, HO, H |  |
| Ignore | IGNORE | the list is one line |
| Knock | KNOCK |  |
| LineWrap | LINEWRAP |  |
| Lock | LOCK, LOC |  |
| MXP | MXP |  |
| Mend | MEND |  |
| Mood | MOOD | not implemented |
| Mount | MOUNT, BOARD, RIDE, M |  |
| NOMXP | NOMXP |  |
| NoANSI | NOANSI, NOCOLOR, NOCOLOUR |  |
| NoBattleSpam | NOBATTLESPAM |  |
| NoChannel |  | NOGOSSIP and the like; not narrated: its words are the game's channel names, not known ahead |
| NoFollow | NOFOLLOW, NOFOL |  |
| NoSell | NOSELL |  |
| NoSounds | NOSOUNDS, NOMSP |  |
| NoSpam | NOSPAM |  |
| NoTeach | NOTEACH |  |
| Open | OPEN, OP, O |  |
| Order | ORDER | then the follower acts |
| Package | PACKAGE |  |
| PageBreak | PAGEBREAK |  |
| Pay | PAY |  |
| Pose | POSE, NOPOSE |  |
| Pour | POUR |  |
| Practice | PRACTICE, PRAC |  |
| Put | PUT, PU, P | a line per item |
| Quiet | QUIET |  |
| Rebuke | REBUKE |  |
| Remove | REMOVE, REM | a line per item |
| Request | REQUEST |  |
| Sell | SELL |  |
| Serve | SERVE |  |
| Sheath | SHEATH |  |
| Sit | SIT, REST, R |  |
| Sleep | SLEEP, SL |  |
| Sniff | SNIFF, SMELL | the smell itself may follow |
| Sounds | SOUNDS, MSP |  |
| Split | SPLIT | a line per share |
| Stand | STAND, ST, STA, STAN |  |
| Take | TAKE |  |
| Teach | TEACH |  |
| Throw | THROW, TOSS |  |
| TypeCmd | TYPE, = |  |
| Undress | UNDRESS |  |
| Unlock | UNLOCK, UNL, UN |  |
| Visible | VISIBLE, VIS |  |
| Wear | WEAR | a line per item (WEAR ALL) |
| Weather | WEATHER | WEATHER alone |
| Wield | WIELD |  |
| Wimpy | WIMPY |  |
| Withdraw | WITHDRAW |  |

### Talk: one line, and the journal's

Their echo is talk (`comm.channel` comes first, `speech.rs` marks it): the journal speaks it, so it's never an echo.

| Command | Words | Note |
| --- | --- | --- |
| Say | SAY, `, SA, SAYTO |  |
| Ask | ASK |  |
| Tell | TELL, T |  |
| Whisper | WHISPER |  |
| GTell | GTELL, FTELL, GT |  |
| Reply | REPLY, REP, RE |  |
| Yell | YELL, YELLTO, YELLAT, Y |  |
| Shout | SHOUT, SHOUTTO, SHOUTAT |  |
| Channel |  | GOSSIP and the other channel names |
| Fire | FIRE | a say to the one fired |
| Hire | HIRE | a say |
| Value | VALUE, VAL, V | the shopkeeper's answer is a say |
| LLM | LLM | a chat with the game's language model, where it's on |

### A question, or someone else's answer

| Command | Words | What happens |
| --- | --- | --- |
| Logoff | LOGOFF, LOGOUT | Logout -- are you sure (y/N)? |
| Quit | QUIT, QUI, Q | Quit -- are you sure (y/N)? |
| Password | PASSWORD | asks for the old password, then the new one twice |
| Learn | LEARN | hands over to TRAIN or GAIN |
| Gain | GAIN | hands over to the trainer |
| PreviousCmd | ! | ! repeats your last command: whatever that one prints |
| ClanResign | CLANRESIGN | Resign from ... Are you absolutely SURE (y/N)? |
| ClanMOTD | CLANMOTD | asks for the message |
| ClanPremise | CLANPREMISE | asks for the premise |
| ClanTax | CLANTAX | asks for the rate |
| ClanDues | CLANDUES | the dues and who's behind, two lines, or asks for the amount |
| SetCmd | SET | no SET command for players; the game's "huh?" |

## 4. Staff commands

Behind a security flag (`securityCheck`): an ordinary player never gets them. Not narrated.

| Command | Words | Prints | About |
| --- | --- | --- | ---: |
| ATopics | ARCTOPICS, ATOPICS | a block | 60 |
| Modify | MODIFY, MOD | a block | 40 |
| Deviations | DEVIATIONS | a block | 40 |
| Create | CREATE | a block | 30 |
| Shell | SHELL, CMFS, . | a block | 30 |
| CharGen | CHARGEN | a block | 30 |
| Import | IMPORT | a block | 30 |
| AHelp | ARCHELP, AHELP | a block | 25 |
| Export | EXPORT | a block | 20 |
| Catalog | CATALOG | a block | 20 |
| GModify | GMODIFY | a block | 20 |
| Generate | GENERATE | a block | 20 |
| Merge | MERGE | a block | 20 |
| Reset | RESET | a block | 20 |
| Test | TEST | a block | 20 |
| Template | TEMPLATE | a block | 15 |
| TrailTo | TRAILTO | a block | 15 |
| ProxyCtl | PROXYCTL | a block | 10 |
| JConsole | JCONSOLE | a block | 10 |
| Goto | GOTO | a block | 10 |
| Transfer | TRANSFER | a block | 10 |
| At | AT | a block | 10 |
| As | AS | a block | 10 |
| After | AFTER | a block | 5 |
| Every | EVERY | a block | 5 |
| Copy | COPY | a block | 5 |
| Purge | PURGE | a block | 5 |
| Save | SAVE | a block | 5 |
| Shutdown | SHUTDOWN | a block | 5 |
| Load | LOAD, RELOAD | a block | 5 |
| Unload | UNLOAD | a block | 5 |
| Destroy | DESTROY, JUNK, TEAR | a block | 5 |
| ASync | ASYNC | one line | 1 |
| Announce | ANNOUNCE, ANNOUNCETO, ANNOUNCEMSG | one line | 1 |
| Ban | BAN | one line | 1 |
| Beacon | BEACON | one line | 1 |
| Boot | BOOT | one line | 1 |
| Cloak | CLOAK | one line | 1 |
| DumpFile | DUMPFILE | one line | 1 |
| Expire | EXPIRE | one line | 1 |
| JRun | JRUN | one line | 1 |
| Link | LINK | one line | 1 |
| MPCommand | MPCOMMAND | one line | 1 |
| MPRun | MPRUN | one line | 1 |
| NoPurge | NOPURGE | one line | 1 |
| Pause | PAUSE | one line | 1 |
| Poof | POOF | one line | 1 |
| Possess | POSSESS, POSS | one line | 1 |
| Restring | RESTRING | one line | 1 |
| Snoop | SNOOP | one line | 1 |
| SysMsgs | SYSMSGS | one line | 1 |
| TickTock | TICKTOCK | one line | 1 |
| UnLink | UNLINK | one line | 1 |
| WizEmote | WIZEMOTE | one line | 1 |
| WizInv | WIZINVISIBLE, WIZINV, NOWIZINV | one line | 1 |

### No words of their own

| Command | Why |
| --- | --- |
| DeferCmd | no words: stands in for a command a new character can't use yet |
| GenCommand | no words: a command the staff built |
| MetaMsgCommand | METAMSGCOMMAND: internal |
| Xml | XML: switched off (its check always fails) |

## 5. What the narrator says

`echo.rs`'s `ECHOES`: 210 lines from the commands above, as the doer
sees them, each with `*` for a name or a number and the narrator's
words with `{1}`, `{2}` for those. In order; the first that fits wins,
so "You get * from *." comes before "You get *.". Some lines are
shared by every one-line command (`commands` empty: "You don't see '*'
here.", "Illegal * argument...").

The words follow Immersive's goal, fewer spoken words: the fact, no
"you" ("Got a sword.", "Autoloot on.", "Wimpy 20."). A line Coupler
has no words for is said as the game wrote it, when it's the whole
answer. A line the game wrapped is joined back up before it's matched
(up to three rows).

How it shows and sounds:

- **Immersive**: the answer isn't written; the narrator says it (the
  player's words if they changed them, nothing if they turned it off
  for that line). Review mode skips it, as talk.
- **Workshop with the voice layer**: written, and said.
- **Terminal, Workshop**: written. The screen reader gets it as its own
  kind, "Commands' one-line answers" (gear → Speech…).
- **The web build**: the same; `echo_list` comes from the module.

Narrator's Answers turns it all off with one checkbox (Immersive writes
the answers again).

## 6. To confirm on the live server

- That every answer comes with its prompt after it in the same read
  (`echo.rs` waits for the prompt before saying a lone unknown line).
- The wrapping: lines over 80 columns (AUTODRAW's, NOSPAM's) wrapped
  at a space, so joining with one space gives the line back.
- That `(s)`, `(es)` and `(ys)` drop as expected in the doer's view:
  "You empty", "You fly".
- AFK's lines start with a blank line (`println("\n\rYou are now
  listed as AFK.")`): blank lines aren't matched, so it's still one.
- COLOR's lines carry `^H` and `^!` codes inside the words: as plain
  text they read "ANSI 256 colour enabled.".

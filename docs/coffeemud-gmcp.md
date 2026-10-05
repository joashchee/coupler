# CoffeeMUD's GMCP and mapping features

Studied 2026-09-30 from CoffeeMUD's source, snapshot of `master` at
commit `c1e556f` (unpacked in `reference/CoffeeMud/`, not committed).
Paths below are under `com/planet_ink/coffee_mud/`. Where the server's
own guide (`guides/Protocols.html`) and its code disagree, the code is
what's written here and the difference is called out.

Nothing here was checked against the live server yet. Section 9 lists
what to confirm with a real character.

## 1. Transport

- Telnet option **201**. The server offers `IAC WILL GMCP` at connect
  unless `DISABLE=GMCP` is set (`Common/DefaultSession.java`).
- A message is `IAC SB 201 <package> <space> <JSON> IAC SE`. The package
  name is matched **case-insensitively**: the server lowercases it and
  turns dots into underscores to look it up in the `GMCPCommand` enum
  (`Libraries/CMProtocols.java`). An unknown package is ignored silently.
- The server **sends package names in lower case** (`room.info`, not
  `Room.Info`). A client must compare names without regard to case.
- Text is encoded in `Session.MSDP_CHARSET`; JSON strings are escaped
  with `MiniJSON.toJSONString`. Some strings are built by hand without
  escaping (mob names in `room.players` keys, `char.base` name), so a
  client must survive one malformed message without dropping the rest.
- Sending a bare word after the package instead of JSON is accepted: if
  the payload starts with a letter it's wrapped as a JSON string.

## 2. How the server decides what to send

Two maps per session:

- **`gmcpSupports`**: what the client said it supports, lowercased, from
  `Core.Supports.Set` / `.Add` / `.Remove`. `"Room 1"` becomes key
  `room`, version `1.0`.
- **`gmcpPings`**: a hash of the last message sent per package, so a
  package is re-sent only when its content changed.

A pushed event (`sendInlineCommand`, gated by `isInlineAllowed`) goes
out only if the supports map has the **full name** (`room.wrongdir`) or
**everything before the last dot** (`room`). So `"Room 1"` enables every
`room.*` push, and `"Comm 1"` every `comm.*` push. Note the prefix is
one level only: supporting `char` does **not** by that rule enable
`char.effects.add` (its prefix is `char.effects`), but see 4.4.

Polled packages are produced by `pingGmcp`, called on every session
tick, and by `invokeRoomChangeGmcp`, called on a room change. Each has
its own key test, listed below.

| Support key the client sends | What it turns on |
|---|---|
| `char` | `char.vitals` (every ping, when changed), `char.status` (each tick), `char.effects.*` list (when the number of effects changes), and every ~16 s when changed: `char.worth`, `char.maxstats`, `char.base`, `char.statusvars` |
| `char.vitals`, `char.status`, `char.worth`, `char.maxstats`, `char.base`, `char.statusvars`, `char.effects` / `char.effects.get` | The same, one at a time |
| `room` | `room.info`, `room.exits`, `room.mobiles`, `room.players`, `room.items.list` on change, and the pushes `room.enter`, `room.leave`, `room.wrongdir` |
| `room.info`, `room.exits`, `room.mobiles`, `room.players`, `room.items` / `room.items.inv` | The same, one at a time |
| `comm` | `comm.tick` each game tick, and the `comm.channel` pushes |
| `comm.tick`, `comm.channel` | The same, one at a time |
| `group` | `group`, each tick when changed. **Not covered by any other key**: a client must list `Group 1` itself |
| `char.login` | The server answers `Char.Login.Default {"type":["password-credentials"]}` |
| `siplet.input` (or `siplet`) | The server may send `Siplet.Input` to open a text editor |

Coupler sends `["Core 1","Char 1","Room 1","Comm 1"]`. To do when the
features land: add `Group 1` (group panel), `Char.Login 1` (saved
logins), `Siplet 1` (the composer).

## 3. Client to server

| Package | Payload | What the server does |
|---|---|---|
| `Core.Hello` | `{"client":"Coupler","version":"0.1.0"}` | Records the client name as a supports key with the version as a number. A non-numeric version such as `0.1.0` becomes 0. Harmless. |
| `Core.Supports.Set` | `["Room 1","Char 1"]` | Clears the supports map, then adds each. |
| `Core.Supports.Add` | same | Adds. |
| `Core.Supports.Remove` | `["Room"]` | Removes. |
| `Core.KeepAlive` | none | Resets the idle timers. |
| `Core.Ping` | none | Answers `core.ping`. |
| `Core.Goodbye` | none | Ends the session and logs the character off. |
| `Char.Login` | `{"name":"…","password":"…"}` | Only before a character is in: calls `session.autoLogin`. On success answers `char.statusvars`. |
| `Char.Login.Credentials` | `{"account"｜"user"｜"username"｜"name":"…","password":"…"}` | Stores the pair as `LOGIN_ACCOUNT` / `LOGIN_PASSWORD` for the login flow to use. This is the one Sip uses. |
| `Char.Skills.Get`, `Char.Effects.Get` | `{}` / `{"group":"…"}` / `{"group":"…","name":"…"}` | Answers `.groups`, `.list` or `.info` (4.4). |
| `Char.Items.Inv` | none | Answers `char.items.list` for the inventory. |
| `Char.Items.Contents`, `Room.Items.Contents` | `{"id":123}` | Answers the items inside that container. |
| `Room.Info`, `Request.Room`, `Request.Area`, `Request.Sectors` | none | Answers `room.info` for the current room, now. |
| `Room.Exits`, `Request.Exits` | none | Answers `room.exits`. |
| `Room.Mobiles`, `Room.Players`, `Room.Items.Inv` | none | Answer the current lists. |
| `Group`, `Request.Group` | none | Answers `group`. |
| `Request.Char`, `Char.Base` | none | Answers `char.base`. |
| `Comm.Channel.Players` | none | Builds the list of players and channels, then **discards it** (the `return` is missing). No answer. |
| `Request.Quest` | none | Always `comm.quest {"action":"status","status":"ready"}`. A stub. |
| `Request` | `"area"` | Rewritten to `request_area` and run as that. |
| `IRE.Composer.SetBuffer` | `"text"` | Feeds the text to the session as typed input (newlines become `\n`). The answer to `Siplet.Input`. |
| `MSDP` | an MSDP request as JSON | Runs it through the MSDP code and answers as JSON, but with no package name, and a REPORTed variable's later changes are written into the text stream unframed (`pingGmcp`). Unusable: Coupler uses telnet MSDP instead. |
| `Client`, `Client.Version`, `MapLevel`, `RawColor`, `External.Discord.Hello` | any | Accepted, do nothing. |

**The useful one for a mapper:** sending `Room.Info` (no payload) makes
the server answer immediately. The automatic `room.info` is sent only
when the room **object** changes, so after a reconnect, or when the map
lost track, this is how to ask "where am I?".

## 4. Server to client

### 4.1 The room (the mapper's input)

**`room.info`** — sent when the player's room changes.

```json
{"num": 1830469233, "id": "Midgaard#3001", "name": "The Temple of Mota",
 "zone": "Midgaard", "desc": "You are in the southern end…",
 "terrain": "stone", "move": "normal", "details": "", "extradata": {},
 "exits": {"N": 1830469350, "D": 1830469237},
 "idexits": {"N": "Midgaard#3054", "D": "Midgaard#3005"},
 "coord": {"id": 0, "x": -1, "y": -1, "cont": 0}}
```

| Field | Meaning | Use it? |
|---|---|---|
| `id` | The room's ID, from `CMMap.getExtendedRoomID`: `Area#number` for an ordinary room, `Area#number#(x,y)` for a cell of a grid room. **Stable across reboots**: it's the database key. Empty for a room with no ID (some temporary rooms). | **Yes: the map key.** |
| `num` | `abs(id.hashCode())`, a Java string hash squeezed positive. Exists for clients that want a number. Two rooms can collide. | No. Use `id`. |
| `name` | `room.displayText(mob)`: the title as this character sees it (darkness, blindness and some effects change it). May carry CoffeeMUD color codes (`^w`). | Yes, stripped of color codes. Don't treat a change as a different room. |
| `zone` | The area's name. | Yes: groups rooms into areas. |
| `desc` | The description as this character sees it. | Not stored. It's long, and the game already printed it. |
| `terrain` | The locale type, lower case. Outdoors: `city woods rocky plains underwater air watersurface jungle swamp desert hills mountains spaceport seaport`. Indoors: `stone wooden cave magic in_underwater gap cavelakesurface metal innerseaport caveseaport`. | Yes: said in words, and the map's color for the room (`src/lib/terrain.ts`). |
| `move` | How you move in the room: `normal`, `crawl`, `swim`, `fly`. | Yes: a walk should warn before water or air. |
| `details` | Always empty. | No. |
| `extradata` | Builder-set `gmcp_*` values from an `ExtraData` effect on the room; usually `{}`. | Not yet. |
| `exits` | Direction letter to the destination's `num`. | No. |
| `idexits` | Direction letter to the destination's `id`. | **Yes: the map's edges.** |
| `coord` | For a grid cell, its `x`,`y` in the grid (`id` is the grid parent's hash, or 0 for a grid-zone area); otherwise `-1,-1`. `cont` is always 0. | Not yet. Could lay out grid areas exactly. |

Direction letters (`core/Directions.java`, `DIRECTION_CHARS`):
`N S E W U D V NE NW SE SW`. `V` is a gate or vortex ("there"). Whether
the diagonals exist is a server setting (`DIRECTIONS=11`; the stock
default is 7). The letters can be translated on a non-English server;
coffeemud.net is English.

An exit is listed only if the destination exists, the exit exists, and
**this character can see it** or just came through it
(`canBeSeenBy(E2, mob) || mob.lastLocation()==R2`). So hidden and secret
exits appear only once found, and can disappear again. A mapper must
**merge** exits over visits, never replace the list.

**`room.exits`** — sent with `room.info`. The guide leaves out `open`
and `locked`; the code sends them in `exits` (keyed by `num`) but not in
`idexits`:

```json
{"exits": {"N": {"id": 1830469350, "door": "door", "open": false, "locked": true, "move": "normal"}},
 "idexits": {"N": {"id": "Midgaard#3054", "door": "door", "move": "normal"}}}
```

`door` is `"door"` or `""`. `move` here describes the exit or the room
behind it. Door state is true only for the moment it was sent.

**`room.wrongdir "N"`** — pushed when the player tries a direction with
no room behind it ("You can't go that way."). It is **not** sent for a
closed or locked door, or for any other refusal (too tired, too big,
guarded): those only print text.

**`room.enter "name"`**, **`room.leave "name"`** — pushed when another
creature enters or leaves and the player can sense it.

**`room.mobiles`** `{"npcs":[{"an orc":"an orc.2"}]}` — name to the
context name that targets it (`kill orc.2`). **`room.players`**
`{"pcs":[{"Bob":"Bob the Bold"}]}`. **`room.items.list`**
`{"location":"room","items":[{"id":93939,"name":"a sword","attrib":"c"}]}`.
Each is re-sent when its set changes. `attrib` letters: `l` wielded,
`w` worn, `W` wearable but not worn, `c` container.

### 4.2 The character

| Package | Payload | When |
|---|---|---|
| `char.vitals` | `{"hp","mana","moves","maxhp","maxmana","maxmoves"}` | Every ping, if changed |
| `char.status` | `{"level","tnl","xpnl","xppl","hunger","thirst","fatigue","stink_pct","align","faction","state","pos","enemy","enemyrange","enemypct"}` | Each tick, if changed. `state`: 1 not logged in, 3 standing, 4 AFK, 6 sysop messages on, 8 fighting, 9 sleeping, 11 sitting, 12 autorun, the first that applies in the order AFK, sleeping, sitting, fighting (so a player sitting in a fight is 11). `pos`: `Standing`, `Sitting`, `Sleeping`, or the mount. `enemy*` only in a fight (`getVictim()` set): **the reliable combat signal**, and a fight starting pushes a `char.status` at once (`StdMOB.combatStarted`, `GMCP_PING_MED`); its end waits for the next tick. `faction` only where the area has one. |
| `char.maxstats` | `{"maxhp","maxmana","maxmoves"}` | ~16 s |
| `char.worth` | `{"gold","qp","trains","pracs"}` | ~16 s |
| `char.base` | `{"name","class","subclass","race","perlevel","prevlevel","extradata","pretitle","clan"}` (`clan` is a string, or an array for several) | ~16 s |
| `char.statusvars` | `{"level","race","guild"}` (`guild` is the clan, same shape). **Values, not labels**: it differs from IRE's package of the same name. | ~16 s, and after `Char.Login` |
| `char.items.list` | `{"location":"inv"｜"<container id>","items":[…]}` | On request only |

### 4.3 Talk

**`comm.channel`** `{"chan":"GOSSIP","msg":"Bob GOSSIPs 'hi'","player":"Bob"}`
— pushed for every channel line the character can hear
(`Libraries/CMChannels.java`), and also for **tells** (`chan: "tell"`),
**group tells** (`GTELL`) and **says** (`chan` is the say verb), from
`CommonMsgs.java`, `Commands/GTell.java`, `Commands/Say.java`. Colors
are removed and the text is unwrapped. The same line is also printed
in the game text.

**`comm.tick {}`** — once per game tick: a clock.

### 4.4 Skills and effects

`char.skills.groups` / `char.effects.groups` `["skill-…","spell-…"]`,
`.list` `{"group":[names…]}`, `.info` `{"group","skill","info"}` (the
help text). Pushed without asking: `char.effects.add` and
`char.effects.remove` `{"name","group"}` from `Abilities/StdAbility.java`
for "the most common" cases only, so they hint at a change rather than
account for every one. `pingGmcp` also re-sends the effects list when
the count changes.

Gating quirk: the `add`/`remove` pushes pass `isInlineAllowed` only with
`char.effects` or `char.effects.add` in the supports map (2), while the
polled list accepts `char`. A client that wants the pushes should list
`Char.Effects 1` explicitly.

### 4.5 Group

`group {"groupname","leader","status":"Private","count","tank",
"members":[{"name","info":{"hp","mhp","mn","mmn","mv","mmv","lvl","align","tnl"}}]}`.
`tank` only in a fight. Needs the `group` key (2).

### 4.6 The editor

`Siplet.Input {"title":"…","text":"…"}` asks the client to open an
editor on `text` (journals, descriptions, `Libraries/CMJournals.java`,
`CMGenEditor.java`, `Commands/JConsole.java`). The client answers with
`IRE.Composer.SetBuffer "the new text"`. The guide also lists
`ire.composer.edit`, which the code no longer sends.

## 5. What GMCP does not carry

Things a client might expect and must get another way:

- **Door refusals and failed moves** other than "no such direction".
- **Other rooms.** Only the current room is ever described. There is no
  area download, no "map" package, and `MapLevel` does nothing.
- **Sounds.** `Client.Media` exists in Sip but this server code drives
  sound through MSP (`Attrib.SOUND`), not GMCP.
- **The weather.** No package carries it (`CMProtocols.java` has no
  climate; MSDP neither). It's only in the text, every line built from
  `resources/lists.ini`: per kind (`Climate.WEATHER_DESCS`, 14) five
  openings by climate and two endings (windy or not), joined by a space;
  `WEATHER_ENDS` (one per kind) when it ends, sometimes followed on the
  same line by the new one; `WEATHER_NONE` with no sky. Said on a
  change (`DefaultClimate.weatherTick`), by `weather`, on stepping out
  of doors with AUTOWEATHER (`MUDTracker`), by `time` at night, and by
  storms (`WeatherAffects`). Indoors and underwater there's no sky
  (`CMMap.hasASky`). Coupler reads it in `ambient.rs`. A server that
  edits `lists.ini` would need the lists there changed.
- **The time of day.** No GMCP. MSDP's `WORLD_TIME` gives the world
  clock's date and hour (`"12/3/7 HR:2"`, `getShortestTimeDescription`),
  but not the part of the day. Said as text when the area's clock moves
  into a new part of the day (`DefaultTimeClock.handleTimeChange`), from
  `lists.ini`'s `TOD_CHANGE_OUTSIDE` (dawn, three choices; dusk; night,
  three) under the sky, awake and seeing, else `TOD_CHANGE_INSIDE`
  (dawn as "It is now daytime.", night). **The start of day is never
  said**, nor dusk indoors. `time` says "It is dawn (Hour: 0/5)". The
  stock clock (`coffeemud.ini`): six hours a day, dawn 0, day 1, dusk
  4, night 5, ten real minutes an hour. A prompt can show it (`%t`), but
  the prompt is the player's. Coupler puts the hour, the lines and
  `time` together in `daytime.rs`.
- **Indoors or outdoors.** Not a field, but `terrain` is from one of
  two lists (`Room.DOMAIN_OUTDOOR_DESCS`, `DOMAIN_INDOORS_DESCS`), so
  it follows from it.
- **The prompt.** It stays in the text, and usually **unmarked**. The
  server never offers EOR. It sends GA after a prompt only while the
  client hasn't agreed to SGA and either no one is logged in yet or
  the player has `CONFIG TELNET-GA` on (`MOB.Attrib.TELNET_GA`, off
  by default; `DefaultSession.java`, the prompt code). So once
  logged in, a prompt is just the unfinished line, which is how
  Coupler shows it anyway (`Event::Prompt` is ignored). Anything that
  needs to know "this is the prompt" (phase 2's speech and review
  mode) must not wait for GA.
- **A screen-reader flag.** The server parses MTTS and defines
  `MTTS_SCREENREAD` (64) but never reads it. Only `MTTS_TRUECOLOR` is
  used (during character creation). Setting the bit changes nothing
  today.

## 6. The same data by other protocols

- **MSDP** (telnet 69) has `ROOM` (`VNUM`, `NAME`, `AREA`, `TERRAIN`,
  `EXITS`), `ROOM_NAME`, `ROOM_VNUM`, `ROOM_AREA`, `ROOM_TERRAIN`,
  `ROOM_EXITS`. `VNUM` is the same hash as GMCP's `num`; MSDP has no
  string ID. GMCP is strictly better for mapping. Coupler accepts MSDP
  for `WORLD_TIME` and the opponent's exact figures (6.1; one REPORT
  with an array, `telnet.rs`, since several REPORTs in one message
  collide in the server's map): the server answers at once, then
  checks every second (`MSDPPINGINTERVAL`) and sends what changed.
  Only REPORTed variables are ever sent, and accepting MSDP changes
  nothing else the server does.
- **MXP** (`resources/text/mxp.txt`) tags the room name (`<RName>`), the
  description (`<RDesc>`), the exits line (`<RExits>`) and each exit
  (`<Ex>`, a clickable send with look/open/close/lock/unlock). No IDs.
  This is what a client without GMCP would scrape.

### 6.1 Every protocol the server offers

From `Common/DefaultSession.java` (what it offers at connect, and
`mightSupportTelnetMode`, what it agrees to), `Libraries/CMProtocols.java`
and `Libraries/CoffeeFilter.java`. Studied 2026-10-02.

| Protocol | The server | Coupler | Why |
|---|---|---|---|
| GMCP 201, MCCP2 86, NAWS 31, TTYPE/MTTS 24, NEW-ENVIRON 39, ECHO 1 | Offers or asks | Accepted | Phase 0 |
| MSDP 69 | Offers | `WORLD_TIME`, `OPPONENT_HEALTH`, `OPPONENT_HEALTH_MAX`, `OPPONENT_RANGE` | The hour (GMCP has none) and the opponent's exact hit points (GMCP's `enemypct` is a rounded percentage, each tick), checked every second: **preferred for Combat mode** (`combat.rs`). Left out: `OPPONENT_NAME` (only sent when a fight starts or ends: its change check compares the player's own name), `OPPONENT_LEVEL` (sends the player's own level, `M.phyStats()`), `OPPONENT_STRENGTH` (runs CONSIDER, which tells the room "<player> considers <foe>." every time it's sent). `AFFECTS` is a count, or names without durations. `ARACHNOS_*` is intermud, never. |
| **MSP** 90 | Offers | Refused, **planned** (phase 5) | See below. |
| **MXP** 91 | Offers | Refused, planned (phase 3) | See below. |
| **MCP 2.1** (in the text, not telnet) | Sends `#$#mcp version: 2.1 to: 2.1` as the first line of every connection, unless `DISABLE=MCP`. Packages: `mcp-negotiate` and `dns-org-mud-moo-simpleedit` (only in `CMGenEditor`, the builder's field editor, by typing `\#$#`) | **Lines dropped** (`ansi.rs`, `mcp_out_of_band`) | Shown, the greeting is a line of noise a screen reader reads on every connect. simple-edit does what GMCP `Siplet.Input` does (4.6), so MCP itself isn't worth speaking. |
| MSSP 70 | Offers when asked | Refused | Name, players online, uptime, status, ports: only for the world already connected (asking the other ports means connecting to each, load on a shared server). A "players online" line in the ports panel at most. Low. |
| TELNET LOGOUT 18, BINARY 0 | Would agree | Not asked | Nothing gained: Coupler closes the socket itself, and the text is UTF-8 already. |
| LINEMODE 34 | Asks (`DO`) | Refused | Coupler sends whole lines (`telnet.rs` header). |
| EOR 25, SGA 3 | Never offered | Accepted if ever offered | See "The prompt" (5). |
| MPCP 202 | Proxy to server | — | CoffeeMUD's own `MUDProxy` handing over a session, signed with the server's secret key (HMAC-SHA1). Not for clients. |
| ATCP 200, AARD 102, MCCP v1 85 and v3 87 | Constants only, never offered or handled | — | — |
| Sip's WebSocket JSON (`WebMacros/WebSockJSON.java`) | Web port | — | The browser Sip's transport. Coupler is telnet. |

**MSP** is the one to add next. 176 calls to `CMLib.protocol().msp(…)`
put `!!SOUND(name V=volume P=priority U=path)` into the text: spells
landing (`fireball.wav` 16 places, `spelldam1.wav`, `lightning.wav`),
blows, doors (`dooropen`, `doorlock`, `doorunlock`), `levelgain.wav`,
death, the weather (`rain`, `thunder`, `blizzard`), crafting and
instruments (`CommonSkill`, `PlayInstrument`) and socials
(`DefaultSocial`, the sound named in `socials.txt`). `CoffeeFilter`
keeps a token only for a session with MSP on and a character with
`SOUNDS` on, and only when the source is the player or can be heard by
them (`canBeHeardSpeakingBy`); otherwise it cuts it out. So each token
says what just happened, more reliably than matching the text. For
Coupler: take the token out of the text in Rust and record it as a hook
pair (`msp.sound` = `fireball.wav`), so the existing triggers and
Assets give it a sound. Nothing is fetched. `U=` is `SOUNDPATH` from
the ini (stock `http://localhost:27744/sounds/`); the stock pack is
`web/pub/sounds/`, 174 WAVs, license unchecked. Catches:
- The `SOUNDS` flag (`MOB.Attrib.SOUND`, off by default) is set at
  character creation **only if the client accepted MSP then**
  (`CharCreation.charcrANSIDone`). A character made in Coupler before
  MSP is accepted has it off: the player types `SOUNDS` once. Coupler
  should say so, not set it behind their back.
- A character with `SOUNDS` (or `MXP`) on, played in a client that
  refuses the option, gets "MSP sounds have been disabled for this
  session." (or the MXP one) at login: harmless noise.
- With MXP on as well, `CoffeeFilter` turns `!!SOUND(…)` into an MXP
  `<SOUND …>` tag, so whichever is built second must send its sounds
  to the same place as the first.

**MXP** carries what GMCP doesn't: actions on the things named in the
text. `mxp.txt` defines menus for an item in the room (`RItem`: get,
look, drink, read), carried (`MItem`: wear, drop, eat…), worn
(`EItem`), in a container (`CRItem`, `CMItem`), a creature (`RMob`:
consider, look, kill), a shopkeeper (`RShopM`), shop stock (`SHOP`:
view, buy), exits (`EX`, `MEX`), help, journals and clans; who said a
line (`SAY`, `TELL`, `WHISPER`, `GTELL`, `CHANNEL` with the speaker);
`Fight`; vitals (duplicating `char.vitals`); and images from
`MXPIMAGEPATH`. For a keyboard or screen-reader player that's picking
"get" from an item's menu instead of typing `get sword.2`. The cost is
a strict parser in Rust and an accessible link UI (Sip's
`mxpsupport.js` is 2.8k lines). Build only `mxp.txt`'s elements, not
general MXP. The `MXP` flag behaves like `SOUNDS` (set at creation if
the client accepted it; the `MXP` command turns it on later).

## 7. CoffeeMUD's own mapping features (in the game)

What the server offers players, so Coupler complements them rather than
fights them:

| Feature | What it is | For Coupler |
|---|---|---|
| `AUTOMAP` (config flag) | With `AWARERANGE` above 0 in the ini (the stock default is **0, off**), each room look ends with an ASCII mini-map drawn by `Skill_RegionalAwareness`. | If coffeemud.net has it on, a screen reader reads rows of symbols on every move. The accessibility guide should suggest `AUTOMAP OFF`, since Coupler's map panel says the same in words. **Check on the live server.** |
| `Skill_RegionalAwareness`, `Skill_WildernessLore`, `Skill_Map`, `Skill_SeaMapping`, `Skill_ResearchRegionMap`, `Thief_TreasureMap` | In-character map skills: draw the surroundings, or make a map item. | Game content. Nothing to do. |
| `EXITS` / `AUTOEXITS` | Lists exits, optionally after every look. | The map panel's exit list is the same data, with destinations. |
| `AREAS` | Lists areas, filterable (`areas explored`). | A future area index could pair it with the map's own area list. |
| `RUN 4 north 2 east`, `AUTORUN` | Moves several rooms in one command, faster but costing more movement. | Coupler's walk sends single steps instead, each waiting for the game to confirm the last. `RUN` is an option for later (one command, less typing for the server, but it can't stop at a surprise). |
| `TRAILTO` | Server-side pathfinding. **Archons only.** | Not available to players; Coupler's own route finder does the job from what the character has seen. |
| Trails files (`midgaardtrails.txt` and others on the website) | Published walking directions between areas. | Could seed landmarks one day. Would be fetched from CoffeeMUD's web host, so it needs the rule 2 sign-off. |
| `Prop_RoomUnmappable` | A builder property: a room can refuse to be mapped or explored by the in-game skills (`MAPOK`, `NOEXPLORE`). | It affects the in-game skills only. GMCP still reports such rooms. Coupler maps them, since the player did walk there. |
| `WHERE` | Finds mobs, items and rooms. **Archons**, in its full form. | Nothing. |

## 8. What this means for Coupler's mapper (`mapper.rs`)

Decisions taken from the study, all in the code:

1. **Key rooms by `id`**, never `num` or name.
2. **Record the destination of every exit** from `idexits`, and keep a
   stub for each destination not yet visited. The map knows what's
   behind a door before the player opens it.
3. **Merge exits.** A hidden exit seen once stays.
4. **No stored coordinates.** Areas aren't grids; the picture is laid
   out breadth-first around the player each time, and a room whose cell
   is taken is left out rather than misplaced. Words (exit list, route,
   directions) never depend on the layout.
5. **Doors from `room.exits`**: a route avoids a door last seen locked;
   a walk sends `open <direction>` before a closed one.
6. **Walk one confirmed step at a time**: the next command goes only
   when `room.info` reports the expected room. `room.wrongdir`, an
   unexpected room, a typed command or Stop end the walk. A refusal the
   server only prints (5) leaves the walk waiting: the player sees the
   game's message and presses Stop or types.
7. **One map per world**, saved as JSON under the app-data dir
   (`maps/<world>.json`), written to a temporary file and renamed.
8. **Strip color codes** from names.

## 9. To confirm on the live server

1. A real `room.info` and `room.exits` capture, byte for byte, as a
   regression test in `mapper.rs` (rule 6). In particular: are color
   codes present in `name`, and is `idexits` always there?
2. Whether `room.info` and `room.exits` always arrive in the same read.
   The walker assumes door state is fresh when it takes the next step.
3. Whether coffeemud.net enables the diagonals (`DIRECTIONS=11`).
4. Whether `AWARERANGE` is on (the text mini-map), for the accessibility guide.
5. Whether 23, 2324 and 2326 really share room IDs (one map), and
   whether Hardcore and Classic do too.
6. The character set. The stock ini says `CHARSETINPUT=iso-8859-1` and
   `CHARSETOUTPUT=iso-8859-1`, and the wiki says the same of the live
   game. Coupler's `ansi.rs` decodes UTF-8 and, since 2026-10-03, reads
   bytes that aren't UTF-8 as Latin-1 (tested). Still to capture: which
   the game really sends, and whether it reads what Coupler sends
   (UTF-8) right.
7. Leaving a character while connected (`character.rs`). CoffeeMUD
   sends no GMCP on LOGOUT or SWITCH, so Coupler reads a logout from
   the `Logout -- are you sure (y/N)?` prompt and a yes, and a switch
   from `char.base` naming someone else. Confirm: the prompt arrives as
   the unfinished line (no newline after it); an offline SWITCH sends
   `char.base` at once (`GMCP_PING_ALL`) and after the new room's
   `room.info`; a live SWITCH sends it within the ~16 s poll.
8. The time of day (`daytime.rs`): that coffeemud.net offers MSDP
   (`DISABLE=MSDP` off) and `WORLD_TIME` arrives on connecting and each
   hour; whether it runs the stock clock (`time` shows `(Hour: n/5)`,
   and dawn, dusk and night lines come with hours 0, 4 and 5); that the
   change lines are on (`TODNOTIFIES`); and whether any area the players
   reach keeps a clock of its own.
9. Combat (`combat.rs`): that `OPPONENT_HEALTH`, `_MAX` and `_RANGE`
   arrive with the four-variable REPORT, change during a fight, and
   come empty when it ends; and what they do when the player turns on
   another foe mid-fight (the figures should jump to the new one's,
   and `char.status` names them within a tick).
10. MCP: whether coffeemud.net sends the `#$#mcp version: 2.1 to: 2.1`
   greeting (`DISABLE=MCP` off). Coupler drops it either way
   (`ansi.rs`), but a capture makes it a byte-for-byte test.
11. Who's online (`who.rs`): that coffeemud.net's WHO has the stock
   header (`] Character name`) and rows padded to its bracket; whether
   the reply's first line follows the prompt on its line or after a
   newline, and whether a blank line follows the rows (both hidden
   either way, but a capture makes them byte tests); which channels
   carry LOGINS and LOGOFFS there, and the exact line a clan's channel
   shows (`The Knights [CLANTALK] 'Bob has logged on.'`); and that the
   friends' notice comes with AUTONOTIFY on.

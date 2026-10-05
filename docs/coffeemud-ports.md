# CoffeeMUD's ports

Studied 2026-09-30 from three places: CoffeeMUD's source (snapshot of
`master` at commit `c1e556f`, unpacked in `reference/CoffeeMud/`, not
committed), the live server's own phonebook
(`http://coffeemud.net:27744/MudPhonebook`), and the game's wiki
(`wiki.coffeemud.net`, pages *Official CoffeeMUD* and
*Playstyles(CoffeeMUD)*).

## How one server has many ports

A CoffeeMUD process can run several **hosts** at once. Each host has its
own `.ini`, its own rules, and its own telnet port (`MUD.java` opens one
`ServerSocket` per entry of `PORT=`, and one JVM can be started with
several ini files). `WebMacros/MudPhonebook.java` lists every host's
`MUD_NAME`, public port and whether it uses accounts. That list is what
Siplet, the web client, offers as its phonebook, and it is the
authoritative list of what coffeemud.net runs.

The phonebook on 2026-09-30:

```json
{"phonebook":[
 {"name":"CoffeeMud","port":23,"accounts":true},
 {"name":"CoffeeMud","port":2323,"accounts":true},
 {"name":"CoffeeMud NO","port":2330,"accounts":true},
 {"name":"CoffeeMud HardCore","port":2325,"accounts":true},
 {"name":"CoffeeMud Heroics","port":2329,"accounts":true},
 {"name":"CoffeeMud Tech","port":2328,"accounts":true},
 {"name":"CoffeeMud Classic","port":2327,"accounts":false},
 {"name":"CoffeeMud PVP","port":2324,"accounts":true},
 {"name":"CoffeeMud RP","port":2326,"accounts":true}]}
```

## The game ports (what Coupler connects to)

The first six are `PORTS` in `src-tauri/src/session.rs`, one button each.
The last three have **no button** (removed 2026-09-30, the user's call).

| Port | Coupler's button | Server's name | Account | What's different | Source |
|---|---|---|---|---|---|
| 23 | Standard | CoffeeMud | Shared | The main game. Player-kill is a flag each player toggles. Up to 3 characters online per account, 50 characters per account. | Wiki |
| 2323 | Standard, second line | CoffeeMud | Shared | The same host as 23 on a second port, for networks that block 23. | Phonebook, wiki |
| 2324 | Player vs Player | CoffeeMud PVP | Shared | Player-kill always on and can't be toggled. No hunger or thirst. | Wiki |
| 2325 | Hardcore | CoffeeMud HardCore | Separate (hardcore) | Kill experience tripled. Any death deletes the character. Hardcore characters join only hardcore clans. Remort at 31. | Wiki |
| 2326 | Role-Playing | CoffeeMud RP | Shared | Most channels and TELL off (NEWBIE stays, OOC only in inns). You must introduce yourself to appear on WHO. Stats shown as words. Experience is deferred until you TRAIN with a guildmaster, and social play earns role-play experience. One character at a time. | Wiki |
| 2327 | Classic | CoffeeMud Classic | None (each character logs in by name) | The game as it started: old experience tables (much slower), level cap 31, no classes from Artisan on, fewer races, no remort, high-level areas closed. | Wiki |
| 2328 | (none) | CoffeeMud Tech | Yes | **Undocumented.** Not on the wiki. The name suggests CoffeeMUD's technology and space content. | Phonebook only |
| 2329 | (none) | CoffeeMud Heroics | Yes | **Undocumented.** | Phonebook only |
| 2330 | (none) | CoffeeMud NO | Yes | **Undocumented.** | Phonebook only |

The three undocumented ports get no button: there is nothing true to
tell a player about them. To add one later, connect to it once, read
its welcome and `HELP` text (or ask Bo Zimmerman; see the notes' open
question 4), then add a `Port` with a real summary and its own `world`.

Which ports share a world matters for the map. Coupler keeps one map per
world (`Port::world`): 23, 2323, 2324 and 2326 use the shared account
and are treated as one world; every other port gets its own map. If two
"worlds" turn out to have identical rooms the only cost is exploring
twice. Treating different worlds as one would corrupt a map, so the
split errs that way.

## Ports Coupler does not connect to

Defaults from the source; the live server's values are noted where seen.

| Port | What | Where set | Coupler |
|---|---|---|---|
| 5555 | The telnet port of a stock install (`PORT=5555`). coffeemud.net overrides it with the list above. | `coffeemud.ini` | Not used. A dev-only build pointed at a local CoffeeMUD uses it (CLAUDE.md, Testing). |
| 27744, 80 | The public web server: the site, help, Siplet, `/MudPhonebook`, MXP images, the sound pack, and the `/WebSock` WebSocket that carries the game for Siplet. Live on coffeemud.net at both 27744 and 80/443. | `web/pub.ini` | Not used. If sounds (Client.Media, MSP) or MXP images are built, this is the only other host and port they may be fetched from (rule 2). |
| 27777 | The admin web server (MUDGrinder, the area editor). | `web/admin.ini` | Never. |
| 27733 | CM1, a text admin/control protocol. | `web/cm1.ini` | Never. |
| 27766 | InterMud3 (I3) server-to-server chat. | `coffeemud.ini` `I3PORT` | Never. It's between MUDs. |
| (client) | IMC2 server-to-server chat: CoffeeMUD dials out. | `coffeemud.ini` | Never. |
| 25 | The built-in SMTP server, for in-game mail forwarding. | `web/email.ini` | Never. |
| 8080 | The wiki (`wiki.coffeemud.net` redirects to `coffeemud.net:8080`). | Seen live | Never. |

`PROXY=` and `MPCPKEY=` in `coffeemud.ini` are for a load-balancing
proxy in front of several MUD processes; players still connect to the
public port.

## How the rule holds

CLAUDE.md rule 2 says no command, setting or file takes another host.
With six ports that becomes: the frontend passes a port's **ID**
(`"hardcore"`), never a number, and `Session::connect` looks the ID up
in the constant table. There is no code path from the webview to an
arbitrary port.

If CoffeeMUD adds or renumbers a port, the table is edited and a new
build ships. Reading `/MudPhonebook` at launch would keep the list
current without a build, but it's a second network destination and a
server-supplied port list, so it isn't done. It's noted on the
roadmap as a decision for the maintainer.

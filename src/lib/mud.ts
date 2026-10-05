/**
 * The CoffeeMUD session, as the frontend sees it: typed wrappers over the
 * Rust commands and events (src-tauri/src/lib.rs, session.rs). The
 * protocol never runs here; Rust hands over finished, styled lines.
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { parseColor, readable, rgbCss, type GameColors } from "./display";

export type Color = { kind: "index"; index: number } | { kind: "rgb"; r: number; g: number; b: number };

export interface Span {
  text: string;
  fg?: Color;
  bg?: Color;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  inverse?: boolean;
}

export type Line = Span[];

/** One way to play: a game CoffeeMUD runs on its own port (session.rs, `PORTS`). */
export interface Port {
  /** What `connect` takes. The frontend never passes a port number. */
  id: string;
  port: number;
  name: string;
  /** What's different about this game, in plain words. */
  summary: string;
}

export interface ServerInfo {
  host: string;
  ports: Port[];
  connected: boolean;
  /** The port the live connection is on. */
  portId: string | null;
}

/** What kind a line of the game output is (src-tauri/src/speech.rs). */
export type LineKind = "game" | "talk" | "time" | "combat" | "prompt" | "echo";

/** A command's one-line answer in the output (src-tauri/src/echo.rs). */
export interface Echoed {
  /** Its first line in `lines`, and how many it took (the game wraps long ones). */
  line: number;
  lines: number;
  /** Which answer; null for one said as the game wrote it. */
  id: string | null;
  /** What its `*`s stood for. */
  values: string[];
  /** What the narrator says unless the player changed it. */
  says: string;
}

/** A long look's room (src-tauri/src/hidden.rs): its description, and the words in it only color marked. */
export interface Hidden {
  /** The description's first line in `lines`, and how many it took. */
  line: number;
  lines: number;
  /** The description, its lines joined back up. */
  text: string;
  /** The hidden details, in order; empty when there were none. */
  words: string[];
}

/** The hidden details as words: "marble fountain and statue". */
export function hiddenList(words: string[]): string {
  return words.length < 2 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} and ${words[words.length - 1]}`;
}

/** A command that answers in one line, and the words that call it. */
export interface EchoCommand {
  name: string;
  words: string[];
}

/** A one-line answer: the line (`*` a name or number) and Coupler's words for it (`{1}` the first `*`). */
export interface EchoInfo {
  id: string;
  /** None: any of the one-line commands. */
  commands: string[];
  line: string;
  says: string;
}

export interface EchoListing {
  commands: EchoCommand[];
  echoes: EchoInfo[];
}

export interface OutputEvent {
  lines: Line[];
  /** What kind each line is, for speech by kind. */
  kinds: LineKind[];
  /** The commands' one-line answers among them. */
  echoes: Echoed[];
  /** A long look's room and its hidden details, when this read has it. */
  hidden: Hidden | null;
  /** The unfinished last line (usually a prompt); replaces the previous one. */
  partial: Line | null;
}

export interface GmcpEvent {
  package: string;
  data: string;
}

// ---- Making a character (src-tauri/src/creation.rs) ----

/** Which of CoffeeMUD's character-creation questions is being asked. */
export type CreationKind =
  | "accountName" | "loginName" | "newAccount" | "newCharacter" | "accountPassword" | "email" | "emailAgain" | "accountMenu"
  | "characterName" | "confirmName" | "password" | "colors" | "theme"
  | "race" | "confirmRace" | "gender" | "stats" | "reroll" | "statAmount"
  | "class" | "confirmClass" | "faction" | "deity" | "confirmDeity" | "rules"
  | "yesNo" | "other";

/** One of a question's answers. */
export interface CreationChoice {
  /** As the game names it ("Half Elf"). */
  name: string;
  /** What's sent to choose it. */
  send: string;
  /** The game's own words about it, where it gave some. */
  about: string | null;
  /** A class suited to the character's best stat (the game shows it only by color). */
  suggested: boolean;
}

export interface CreationStat {
  name: string;
  value: number;
  most: number;
  /** What the race adds or takes; 0 for none. */
  race: number;
  about: string | null;
}

export interface CreationStats {
  stats: CreationStat[];
  total: number;
  most: number;
  /** Points left to spend; null when the game rolls the stats instead. */
  points: number | null;
  /** The classes these stats qualify for. */
  qualifies: CreationChoice[];
}

/** A question the game is asking while a character is made. */
export interface CreationStep {
  kind: CreationKind;
  /** The game's question, as it asked. */
  asked: string;
  /** The name, race, class or deity it's about, or the faction. */
  subject: string | null;
  choices: CreationChoice[];
  /** The game's text around it: an intro, a pick's help. Lines as the game wrapped them. */
  text: string[];
  /** What the game said was wrong with the last answer. */
  problem: string | null;
  stats: CreationStats | null;
  /** The answer's a password: hidden, never kept. */
  secret: boolean;
  /** What's settled so far, in order ("Race", "Elf"). */
  chosen: { what: string; value: string }[];
}

export const serverInfo = () => invoke<ServerInfo>("server_info");
export const connect = (portId: string) => invoke<void>("mud_connect", { portId });
export const disconnect = () => invoke<void>("mud_disconnect");
/** For the greeting at launch: the times played on this Mac, how many are online now in a game, and its CoffeeMUD version against the one Coupler was made for (null when it didn't say). */
export interface LaunchCounts {
  played: number;
  online: number | null;
  version: string | null;
  builtFor: string;
  fit: "same" | "newer" | "older" | null;
}
export const launchCounts = (portId: string) => invoke<LaunchCounts>("launch_counts", { portId });
/** The greeting at launch, in words: "You've played CoffeeMUD 12 times with Coupler. 37 players are online now." */
export function describeLaunchCounts({ played, online }: LaunchCounts): string {
  const times = played === 0 ? null : played === 1 ? "once" : played === 2 ? "twice" : `${played} times`;
  const first = times ? `You've played CoffeeMUD ${times} with Coupler.` : "Welcome to CoffeeMUD with Coupler.";
  if (online === null) return first;
  return `${first} ${online === 0 ? "No one is" : online === 1 ? "1 player is" : `${online} players are`} online now.`;
}
/**
 * A warning when the game runs another CoffeeMUD than the one Coupler was
 * made for (src-tauri/src/mssp.rs), or null: "CoffeeMUD here is newer
 * than Coupler knows (5.12.0.1; Coupler was made for 5.11.0.4). Some of
 * what Coupler hears and says may be off."
 */
export function describeVersionFit({ version, builtFor, fit }: LaunchCounts): string | null {
  if (!version || !fit || fit === "same") return null;
  return `CoffeeMUD here is ${fit === "newer" ? "newer" : "older"} than Coupler knows (${version}; Coupler was made for ${builtFor}). Some of what Coupler hears and says may be off.`;
}
export const sendLine = (line: string) => invoke<void>("mud_send", { line });
export const resize = (columns: number, rows: number) => invoke<void>("mud_resize", { columns, rows });

export const onOutput = (f: (e: OutputEvent) => void): Promise<UnlistenFn> => listen<OutputEvent>("mud-output", (e) => f(e.payload));
/** The next question while a character is made, or null when that's over (in the game, a login, or hung up). */
export const onCreation = (f: (step: CreationStep | null) => void): Promise<UnlistenFn> => listen<CreationStep | null>("creation", (e) => f(e.payload));
/** The server echoes (true: the input is a password) or has stopped. */
export const onEcho = (f: (serverEchoes: boolean) => void): Promise<UnlistenFn> => listen<boolean>("mud-echo", (e) => f(e.payload));
export const onGmcp = (f: (e: GmcpEvent) => void): Promise<UnlistenFn> => listen<GmcpEvent>("mud-gmcp", (e) => f(e.payload));
/** `reason` is null when the user disconnected. */
export const onClosed = (f: (reason: string | null) => void): Promise<UnlistenFn> =>
  listen<{ reason: string | null }>("mud-closed", (e) => f(e.payload.reason));
/** The player logged out or switched character, still connected (src-tauri/src/character.rs): words for the status line. */
export const onCharacterLeft = (f: (message: string) => void): Promise<UnlistenFn> =>
  listen<{ message: string }>("character-left", (e) => f(e.payload.message));

/**
 * The opponent in a fight (src-tauri/src/combat.rs): MSDP's exact hit
 * points where the server sends them, GMCP's percentage otherwise. For
 * Combat mode. Fields are null when no source has them yet.
 */
export interface Opponent {
  name: string | null;
  health: number | null;
  healthMax: number | null;
  /** Health left out of 100. */
  percent: number | null;
  /** 0 is toe to toe. */
  range: number | null;
}
/** Each change to the fight; null when it's over (or the player left). */
export const onCombat = (f: (opponent: Opponent | null) => void): Promise<UnlistenFn> =>
  listen<Opponent | null>("combat", (e) => f(e.payload));

/** The character's hit points, mana and movement (src-tauri/src/senses.rs); a field is null until the game sends it. */
export interface Vitals {
  hp: number | null;
  maxHp: number | null;
  mana: number | null;
  maxMana: number | null;
  moves: number | null;
  maxMoves: number | null;
}
/** Each change to the vitals; null once the player has left the game or their character. */
export const onVitals = (f: (vitals: Vitals | null) => void): Promise<UnlistenFn> => listen<Vitals | null>("vitals", (e) => f(e.payload));

/** One line of talk (src-tauri/src/senses.rs): a tell, the group, a say in the room, or a channel. */
export interface Talk {
  kind: "tell" | "group" | "say" | "channel";
  /** The game's name for it: `tell`, `GTELL`, `say`, `GOSSIP`. */
  channel: string;
  from: string;
  /** The line as the game prints it. */
  text: string;
  /** The player said it. */
  mine: boolean;
  /** The speaker's voice (src-tauri/src/cast.rs); null for the player's own line. */
  voice: Voice | null;
  /** Where it's kept, in the journal or the log (src-tauri/src/journal.rs); null for a repeat, which isn't. */
  entry: JournalEntry | null;
  /** Said by this speaker before: not kept, and not spoken. */
  repeat: boolean;
}
export const onTalk = (f: (talk: Talk) => void): Promise<UnlistenFn> => listen<Talk>("talk", (e) => f(e.payload));

/** The journal (said in the game) or the log (the channels). */
export type Book = "journal" | "log";
/** One line kept (src-tauri/src/journal.rs). */
export interface JournalEntry {
  id: number;
  book: Book;
  kind: Talk["kind"];
  channel: string;
  speaker: string;
  speakerKey: string;
  text: string;
  /** Unix milliseconds. */
  at: number;
  mine: boolean;
  /** Heard to the end. */
  heard: boolean;
}
/** Someone in the journal, or a channel in the log. */
export interface JournalSpeaker {
  key: string;
  name: string;
  lines: number;
  unheard: number;
}
export interface JournalListing {
  /** The newest that match, oldest first. */
  entries: JournalEntry[];
  matching: number;
  speakers: JournalSpeaker[];
}
/** What's still to be heard. */
export interface Unheard {
  journal: number;
  log: number;
  /** The journal's by speaker, the most first. */
  speakers: { name: string; count: number }[];
  /** The log's by channel, the most first. */
  channels: { name: string; count: number }[];
}
export const NOTHING_UNHEARD: Unheard = { journal: 0, log: 0, speakers: [], channels: [] };
export const journalList = (book: Book, query: string, who: string | null) => invoke<JournalListing>("journal_list", { book, query, who });
export const journalUnheard = () => invoke<Unheard>("journal_unheard");
export const journalUnheardEntries = (book: Book) => invoke<JournalEntry[]>("journal_unheard_entries", { book });
export const journalSetHeard = (ids: number[], heard: boolean) => invoke<void>("journal_set_heard", { ids, heard });
export const journalHeardAll = (book: Book) => invoke<void>("journal_heard_all", { book });
export const onJournalChanged = (f: (unheard: Unheard) => void): Promise<UnlistenFn> => listen<Unheard>("journal-changed", (e) => f(e.payload));

/** How a character sounds (src-tauri/src/cast.rs). lib/voice.ts turns it into speech. */
export interface Voice {
  gender: "feminine" | "masculine";
  /** Percent of the voice's own pitch, 50 to 200. */
  pitch: number;
  /** Percent of the player's speed, 60 to 150. */
  rate: number;
  /** `system` (the WebView's voices) or a bundled engine (`voiceEngines`); null lets `seed` choose from all. */
  engine: string | null;
  /** A voice of that engine: a system voice's name, a bundled one's ID; null lets `seed` choose within it. */
  voiceName: string | null;
  seed: number;
  /** Written in Heard, not spoken. */
  quiet: boolean;
}
export const PITCH = { least: 50, most: 200 };
export const RATE = { least: 60, most: 150 };

/** Someone met in the game. */
export interface CastMember {
  key: string;
  name: string;
  who: "npc" | "pc" | "unknown";
  /** Unix seconds. */
  firstMet: number;
  lastMet: number;
  /** Lines of talk heard from them. */
  lines: number;
  voice: Voice;
  /** The player changed the voice. */
  edited: boolean;
}
export interface CastListing {
  /** The world's name; null before the first connection. */
  world: string | null;
  characters: CastMember[];
}
export const castList = () => invoke<CastListing>("cast_list");
export const castSetVoice = (key: string, voice: Voice) => invoke<CastMember>("cast_set_voice", { key, voice });
export const castReset = (key: string) => invoke<CastMember>("cast_reset", { key });
export const castForget = (key: string) => invoke<boolean>("cast_forget", { key });
/** A voice of a speech engine bundled in Coupler (src-tauri/src/synth.rs). */
export interface BundledVoice {
  engine: string;
  id: string;
  name: string;
  /** The speaker's own. */
  gender: Voice["gender"];
  /** Suits either kind (pitched to suit): Automatic offers it to everyone. A real voice of one kind isn't. */
  anyGender: boolean;
}
export const voiceEngines = () => invoke<BundledVoice[]>("voice_engines");
/** A line in a bundled voice: the sample rate (u32), then 16-bit mono samples, little-endian. */
export const voiceSynth = (engine: string, voice: string, text: string, gender: Voice["gender"], pitch: number, rate: number, book?: Book) =>
  invoke<ArrayBuffer>("voice_synth", { engine, voice, text, gender, pitch: Math.round(pitch), rate: Math.round(rate), book: book ?? null });
/** Renders a line of talk into the voice cache ahead of being wanted (src-tauri/src/voicecache.rs). */
export const voicePrerender = (engine: string, voice: string, text: string, gender: Voice["gender"], pitch: number, rate: number, book: Book) =>
  invoke<void>("voice_prerender", { engine, voice, text, gender, pitch: Math.round(pitch), rate: Math.round(rate), book });

/** Someone new was met. */
export const onCastChanged = (f: () => void): Promise<UnlistenFn> => listen("cast-changed", () => f());

/** A share as a whole percent, or null when it can't be worked out. */
export function percentOf(now: number | null, most: number | null): number | null {
  return now !== null && most !== null && most > 0 ? Math.round((now * 100) / most) : null;
}

/** The vitals in one sentence: "Health 82 of 100, mana 40 of 50, movement 95 of 100." */
export function describeVitals(v: Vitals | null): string {
  if (!v) return "The game hasn't sent your health yet.";
  const part = (name: string, now: number | null, most: number | null) => (now === null ? null : most === null ? `${name} ${now}` : `${name} ${now} of ${most}`);
  const parts = [part("health", v.hp, v.maxHp), part("mana", v.mana, v.maxMana), part("movement", v.moves, v.maxMoves)].filter(Boolean) as string[];
  if (parts.length === 0) return "The game hasn't sent your health yet.";
  const text = parts.join(", ");
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
}

/** The fight in one sentence: who, how hurt, how far. */
export function describeOpponent(o: Opponent | null): string {
  if (!o) return "You're not fighting.";
  const who = o.name ?? "Someone";
  const health = o.health !== null && o.healthMax !== null ? `${o.health} of ${o.healthMax} hit points` : o.percent !== null ? `${o.percent} percent health` : "health unknown";
  const range = o.range === null ? "" : o.range === 0 ? ", toe to toe" : `, range ${o.range}`;
  return `Fighting ${who}: ${health}${range}.`;
}

// ---- Who's online (src-tauri/src/who.rs) ----

/** Someone online, as WHO showed them. */
export interface WhoPerson {
  /** The name alone ("Bob"). */
  name: string;
  /** Title and all ("Bob, the Brave"). */
  shown: string;
  /** Away: the game counts them idle. */
  idle: boolean;
}
export interface WhoChange {
  name: string;
  /** Logged on, or off. */
  on: boolean;
}
export interface WhoReport {
  /** A whole WHO has been read since the character came in. */
  known: boolean;
  /** Everyone else online. */
  online: WhoPerson[];
  last: (WhoChange & { secondsAgo: number }) | null;
}
/** Who's online now. */
export const whoNow = () => invoke<WhoReport>("who_now");
/** The one-line commands and what the narrator says for their answers (echo.rs). */
export const echoList = () => invoke<EchoListing>("echo_list");
/** Someone logged on or off (the game's announcement, or WHO's list changing). */
export const onWho = (f: (changes: WhoChange[]) => void): Promise<UnlistenFn> => listen<{ changes: WhoChange[] }>("who", (e) => f(e.payload.changes));

/** "Ann", "Ann and Bo", "Ann, Bo and Cy". */
function listInWords(items: string[]): string {
  return items.length < 2 ? (items[0] ?? "") : `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/** Who's online, short, for the narrator: "3 others online: Ann, Bo (idle) and Cy. Last, Dee logged off 2 minutes ago." */
export function describeWho(r: WhoReport): string {
  const names = r.online.map((p) => (p.idle ? `${p.name} (idle)` : p.name));
  const count = names.length === 0 ? "Nobody else is online." : `${names.length} ${names.length === 1 ? "other" : "others"} online: ${listInWords(names)}.`;
  const head = r.known ? count : names.length === 0 ? "Coupler hasn't read who's online yet." : `Not counted yet; known online: ${listInWords(names)}.`;
  const last = r.last && r.last.secondsAgo < 15 * 60 ? r.last : null;
  if (!last) return head;
  const minutes = Math.round(last.secondsAgo / 60);
  const when = minutes < 1 ? "just now" : minutes === 1 ? "a minute ago" : `${minutes} minutes ago`;
  return `${head} Last, ${last.name} logged ${last.on ? "on" : "off"} ${when}.`;
}

// ---- The map (src-tauri/src/mapper.rs) ----

export interface MapExit {
  /** The game's direction letter: N, NE, U, … */
  dir: string;
  /** "north", "up", …: the word, which is also the command. */
  dirName: string;
  to: string;
  /** Empty until the room behind the exit has been visited. */
  toName: string;
  visited: boolean;
  door: boolean;
  open: boolean;
  locked: boolean;
}

export interface MapRoom {
  id: string;
  name: string;
  zone: string;
  terrain: string;
  travel: string;
  landmark: string;
  exits: MapExit[];
}

/** A room on the picture; (0, 0) is where the player stands. */
export interface MapCell {
  id: string;
  name: string;
  x: number;
  y: number;
  visited: boolean;
  current: boolean;
  up: boolean;
  down: boolean;
  landmark: boolean;
  otherZone: boolean;
  exits: string[];
  /** The game's terrain, lower case (src/lib/terrain.ts colors it); empty when not visited. */
  terrain: string;
}

export interface MapSnapshot {
  /** Null when the map doesn't know where the player is. */
  room: MapRoom | null;
  cells: MapCell[];
  roomsKnown: number;
  zonesKnown: number;
}

export interface RoomRef {
  id: string;
  name: string;
  zone: string;
  landmark: string;
  /** Its terrain, lower case: the picture's color for it, in words. */
  terrain: string;
}

export interface WalkEvent {
  walking: boolean;
  message: string;
}

/** Full screen on or off. The window has no other size (lib/stage.ts). */
export const setFullscreen = (on: boolean) => invoke<void>("window_set_fullscreen", { on });
export const isFullscreen = () => invoke<boolean>("window_is_fullscreen");

export const mapSnapshot = () => invoke<MapSnapshot>("map_snapshot");
export const mapFind = (query: string) => invoke<RoomRef[]>("map_find", { query });
/** The way to a room in words, e.g. "3 north, east". */
export const mapDirections = (to: string) => invoke<string>("map_directions", { to });
/** Walks there one confirmed step at a time; resolves with the way in words. */
export const mapWalk = (to: string) => invoke<string>("map_walk", { to });
export const mapStop = () => invoke<void>("map_stop");
/** An empty name removes the landmark. */
export const mapSetLandmark = (id: string, name: string) => invoke<void>("map_set_landmark", { id, name });
export const mapClear = () => invoke<void>("map_clear");
export const onMapChanged = (f: (snapshot: MapSnapshot) => void): Promise<UnlistenFn> => listen<MapSnapshot>("map-changed", (e) => f(e.payload));
export const onMapWalk = (f: (e: WalkEvent) => void): Promise<UnlistenFn> => listen<WalkEvent>("map-walk", (e) => f(e.payload));

// ---- The hooks (src-tauri/src/hooks.rs) ----

/** One GMCP key and a value the game sent for it, as JSON text. */
export interface HookPair {
  key: string;
  value: string;
  /** What it sets off each time the game sends it, if anything. */
  trigger?: Trigger;
}

/**
 * What a pair sets off: up to one of each kind of asset, each named by
 * its path in the Assets folder (`wav/door.wav`). A trigger with none
 * set is removed.
 */
export interface Trigger {
  /** A sound effect, played once. */
  sfx: string | null;
  /** Its volume, 0 to 100 percent of the Mixer's SFX volume. */
  sfxVolume: number;
  /** Music, replacing whatever music is playing. */
  bgm: string | null;
  /** The music plays on in a loop, rather than once. */
  bgmLoop: boolean;
  /** Its volume, 0 to 100 percent of the Mixer's BGM volume. */
  bgmVolume: number;
  /** A picture, its top left corner at column `artX`, row `artY` of the screen (from 1). */
  art: string | null;
  artX: number;
  artY: number;
  /** Seconds before the picture fades away; 0 keeps it until it's clicked away or replaced. */
  artFade: number;
  /** Background noise, looped while in a room it's set for (on a room's id, its terrain or its room type). */
  bgn: string | null;
  bgnVolume: number;
  /** Background weather, looped while that weather is known and the player is under the sky. */
  bgw: string | null;
  bgwVolume: number;
}

export const NO_TRIGGER: Trigger = { sfx: null, sfxVolume: 100, bgm: null, bgmLoop: false, bgmVolume: 100, art: null, artX: 1, artY: 1, artFade: 0, bgn: null, bgnVolume: 100, bgw: null, bgwVolume: 100 };

/** The pairs a BGN can be set on: a room's id (that room), its terrain, its room type (src-tauri/src/ambient.rs). */
export const BGN_KEYS = ["room.info.id", "room.info.terrain", "coupler.room.type"];
/** The pair a BGW is set on: the weather Coupler read from the game's text. */
export const BGW_KEY = "coupler.weather";

/** A loop to play: an asset and its volume, a share of its bus's. */
export interface Loop {
  path: string;
  volume: number;
}

/** What should be looping now: background noise and weather. */
export interface Ambience {
  bgn: Loop | null;
  bgw: Loop | null;
}

/** What's looping now, for a frontend that has just started. */
export const ambienceNow = () => invoke<Ambience>("ambience_now");
/** Each time what should loop changes (the room, the weather, a trigger set). */
export const onAmbient = (f: (a: Ambience) => void): Promise<UnlistenFn> => listen<Ambience>("ambient", (e) => f(e.payload));

/** An asset's name as the player sees it: no folder, no file type. */
export const assetTitle = (path: string) => path.slice(path.indexOf("/") + 1).replace(/\.[^.]*$/, "");

/** A trigger that went off: the pair the game just sent, and what it sets off. */
export interface Fired {
  key: string;
  value: string;
  trigger: Trigger;
}

export interface HookListing {
  /** Every unique pair in the list (filtered keys aren't in it). */
  total: number;
  /** How many match the query; `pairs` holds the first of them. */
  matching: number;
  pairs: HookPair[];
}

/** A key moved out of the hooks list, and how many values it has. */
export interface FilteredKey {
  key: string;
  values: number;
}

export interface FilteredListing {
  total: number;
  matching: number;
  keys: FilteredKey[];
}

export const hooksCount = () => invoke<number>("hooks_count");
/** The hooks whose key or value contains `query`; all of them when it's empty. */
export const hooksList = (query: string) => invoke<HookListing>("hooks_list", { query });
/** The filtered keys containing `query`; all of them when it's empty. */
export const hooksFiltered = (query: string) => invoke<FilteredListing>("hooks_filtered", { query });
/** Moves a key to the Filtered list (true), or back to the hooks list. */
export const hooksSetFiltered = (key: string, filtered: boolean) => invoke<void>("hooks_set_filtered", { key, filtered });
/** Sets what a pair sets off; a trigger with nothing set removes it. */
export const hooksSetTrigger = (key: string, value: string, trigger: Trigger) => invoke<void>("hooks_set_trigger", { key, value, trigger });
/** The triggers the last read of the game set off, in the order they came. */
export const onHookFired = (f: (fired: Fired[]) => void): Promise<UnlistenFn> => listen<Fired[]>("hook-fired", (e) => f(e.payload));

// ---- The Assets folder (src-tauri/src/assets.rs) ----

/** SFX, BGM, ART, or a SoundFont (the instruments MIDI music plays with). */
export type AssetKind = "sfx" | "bgm" | "art" | "soundfont";

export interface Asset {
  /** Its type's folder and its name: `wav/door.wav`. */
  path: string;
  name: string;
  kind: AssetKind;
  /** How many hooks' triggers use it. */
  uses: number;
}

/** The Assets folder, and the assets just cleared from the hooks because their files are gone. */
export interface AssetsListing {
  assets: Asset[];
  /** Each missing asset's path, and how many hooks named it. */
  cleared: { path: string; hooks: number }[];
  /** Each MIDI file the player chose a SoundFont for, and the SoundFont (it may have left the folder); every other plays through Neumetik. */
  fonts: Record<string, string>;
}

/** An asset Coupler made from the player's words, and what it is in words. */
export interface Made extends Asset {
  about: string;
  /** How music was written, a sentence for each decision (compose.rs's `Piece::rules`). */
  rules?: string[];
}

/** A feel control's lean on the mood. */
export type Lean = "less" | "more";
/** Create Asset's music options (compose.rs's `Options`); each left out follows the words, then the mood. */
export interface MusicOptions {
  /** The key's note, C 0 to B 11. */
  key?: number;
  scale?: string;
  bars?: number;
  form?: "loop" | "piece";
  arc?: "steady" | "arch" | "rise" | "dissolve" | "waves";
  /** 50 straight, 67 a triplet. */
  swing?: number;
  /** How far off the grid, 0 to 100. */
  humanize?: number;
  brightness?: Lean;
  drive?: Lean;
  tension?: Lean;
  fills?: "every" | "end" | "none";
  parts?: "all" | "bed" | "rhythm" | "drums" | "noDrums" | "tune" | "duet";
}

/** A file an import took: copied in (`new`), under a new name because its own was taken (`renamed`), or already there (`already`). */
export interface Added extends Asset {
  /** Its name where it came from. */
  from: string;
  how: "new" | "renamed" | "already";
  /** A WAV just added that could be smaller, and how (its smaller copy waits for the player's answer). */
  smaller: Smaller | null;
}

/** A WAV's smaller copy: Opus (`opus`, lossy) or WavPack (`wv`, lossless), and both sizes in bytes. */
export interface Smaller {
  to: "opus" | "wv";
  before: number;
  after: number;
}

/** A WAV replaced by its smaller copy: the WAV's path (gone now), and the asset it became. */
export interface Shrunk extends Asset {
  from: string;
  before: number;
  after: number;
}

export interface Compressed {
  done: Shrunk[];
  /** Each WAV kept as it was, with why, in words. */
  skipped: string[];
}

/** What each smaller kind of sound is, in words. */
export const SMALLER_NAMES: Record<Smaller["to"], string> = { opus: "Opus", wv: "WavPack" };

export interface Imported {
  added: Added[];
  /** Each file not taken, with why, in words. */
  skipped: string[];
}

/** Where an import is: file `file` (from 1) of `files`, and what it's doing to it. */
export interface ImportProgress {
  file: number;
  files: number;
  name: string;
  step: "reading" | "checking" | "saving" | "compressing";
  doneBytes: number;
  totalBytes: number;
}

/** What clearing missing assets from the hooks did, in words. */
export function clearedInWords(cleared: AssetsListing["cleared"]): string {
  const each = cleared.map((c) => `${assetTitle(c.path)} (${c.path}) from ${c.hooks} ${c.hooks === 1 ? "hook" : "hooks"}`);
  return `Not in the Assets folder any more, so cleared: ${each.join(", ")}.`;
}

/** A size in words: 512 bytes, 34 KB, 1.2 MB. */
export function sizeInWords(bytes: number): string {
  if (bytes < 1024) return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** The file types the Assets folder takes, for the file chooser. */
export const ASSET_TYPES = ["wav", "ogg", "opus", "wv", "mid", "midi", "mp3", "mod", "xm", "s3m", "it", "png", "jpg", "jpeg", "ans", "asc", "sf2"];

/** ANSI art drawn into rows of styled characters (src-tauri/src/ansi_art.rs). */
export interface AnsiArt {
  columns: number;
  rows: number;
  lines: Line[];
}

/** The room as a picture sees it (src-tauri/src/paint.rs): the game's own words about it, and Coupler's weather and time. */
export interface Scene {
  world: string;
  room: string;
  name: string;
  zone: string;
  terrain: string;
  weather: string | null;
  time: string | null;
  /** Where the sun (the moon at night) is, left to right in thousandths; null when the hour isn't known. */
  arc: number | null;
  /** Which of the room's looks: 0 until the player asks for another (Cmd+Shift+P). */
  look: number;
  /** The player's own ART trigger for this room, an Assets path: it's shown instead of a painting. */
  art: string | null;
}

/** The two looks a room's picture can have: fantasy, or high fantasy. */
export type PictureStyle = "fantasy" | "high";

/** The room the picture shows now, or null when the player is nowhere. */
export const pictureNow = () => invoke<Scene | null>("picture_now");
/** A new look for the room the player is in, kept as its look. */
export const pictureAgain = () => invoke<Scene | null>("picture_again");
/** Every room back to its first look. */
export const pictureForgetLooks = () => invoke<void>("picture_forget_looks");
/** Coupler's painter's picture of a room, at a size in cells (src-tauri/src/painter.rs). */
/** A race's or a class's portrait by Coupler's painter (src-tauri/src/portrait.rs). */
export const portraitPaint = (kind: "race" | "class", name: string, columns: number, rows: number) => invoke<AnsiArt>("portrait_paint", { kind, name, columns, rows });
export const picturePaint = (scene: Scene, style: PictureStyle, columns: number, rows: number, again = 0) =>
  invoke<AnsiArt>("picture_paint", { scene, style, columns, rows, again });
/** A new room, weather or time of day for the picture; null once the player's nowhere. */
export const onPicture = (f: (scene: Scene | null) => void): Promise<UnlistenFn> => listen<Scene | null>("picture", (e) => f(e.payload));

/** Whether an ART asset is ANSI art (drawn in characters) rather than an image. */
export const isAnsiArt = (path: string) => path.startsWith("ans/") || path.startsWith("asc/");

/** The Assets folder, once any asset whose file is gone is cleared from every hook naming it. */
export const assetsList = () => invoke<AssetsListing>("assets_list");
/** Copies files into the Assets folder, each into its type's folder, telling `onProgress` where it is. */
export function assetsImport(paths: string[], onProgress: (p: ImportProgress) => void) {
  const progress = new Channel<ImportProgress>();
  progress.onmessage = onProgress;
  return invoke<Imported>("assets_import", { paths, progress });
}
/** Puts the smaller copies of these WAVs in their place, deleting the WAVs. */
export const assetsCompress = (paths: string[]) => invoke<Compressed>("assets_compress", { paths });
/** Forgets the smaller copies of these WAVs: they stay as they are. */
export const assetsKeep = (paths: string[]) => invoke<void>("assets_keep", { paths });
/** Chooses the SoundFont a MIDI asset plays through, or Neumetik (null). */
export const assetSetFont = (path: string, font: string | null) => invoke<void>("asset_set_font", { path, font });
/** Paints a picture from the words with Coupler's painter, at a size in characters, and saves it as ANSI art. */
export const assetCreateArt = (prompt: string, columns: number, rows: number) => invoke<Made>("asset_create_art", { prompt, columns, rows });
/** Composes a short piece from the words and options for Neumetik, and saves it as MIDI. */
export const assetCreateMusic = (prompt: string, options: MusicOptions = {}) => invoke<Made>("asset_create_music", { prompt, options });
/** Whether an asset is MIDI music, which a SoundFont can play instead of Neumetik. */
export const isMidi = (path: string) => path.startsWith("mid/") || path.startsWith("midi/");
/** A sound or piece of music as WAV or MP3 bytes, rendered first if the WebView can't play it (a SoundFont: its sample tune). */
export const assetAudio = (path: string, looping: boolean) => invoke<ArrayBuffer>("asset_audio", { path, looping });
/** An image's bytes (PNG or JPEG). */
export const assetPicture = (path: string) => invoke<ArrayBuffer>("asset_picture", { path });
/** ANSI art, drawn. */
export const assetAnsi = (path: string) => invoke<AnsiArt>("asset_ansi", { path });

/** The list's new total: the game sent a pair not seen before, or a key was filtered. */
export const onHooksChanged = (f: (count: number) => void): Promise<UnlistenFn> => listen<number>("hooks-changed", (e) => f(e.payload));

/** A room's exits as one sentence, each with what's behind it. */
export function describeExits(room: MapRoom): string {
  const exits = room.exits.map((e) => {
    const where = e.visited && e.toName ? ` to ${e.toName}` : ", not explored";
    const door = e.locked ? " (locked door)" : e.door && !e.open ? " (closed door)" : "";
    return `${e.dirName}${where}${door}`;
  });
  return exits.length > 0 ? `Exits: ${exits.join("; ")}.` : "No exits you can see.";
}

/** The current room as one sentence, for the status line and screen readers. */
export function describeRoom(room: MapRoom | null): string {
  if (!room) return "The map doesn't know where you are yet. Move or look once in the game.";
  const name = room.landmark ? `${room.name} (${room.landmark})` : room.name;
  return `${name}, in ${room.zone}. ${describeExits(room)}`;
}

/** The standard VGA palette for indexes 0–15, then the xterm 256-color cube and grays. */
const BASE16 = [
  "#000000", "#aa0000", "#00aa00", "#aa5500", "#0000aa", "#aa00aa", "#00aaaa", "#aaaaaa",
  "#555555", "#ff5555", "#55ff55", "#ffff55", "#5555ff", "#ff55ff", "#55ffff", "#ffffff",
];
const DEFAULT_FG = "#aaaaaa";
const DEFAULT_BG = "transparent";

function indexColor(i: number): string {
  if (i < 16) return BASE16[i];
  if (i < 232) {
    const n = i - 16;
    const level = (v: number) => (v === 0 ? 0 : 55 + v * 40);
    const [r, g, b] = [Math.floor(n / 36), Math.floor(n / 6) % 6, n % 6].map(level);
    return `rgb(${r}, ${g}, ${b})`;
  }
  const gray = 8 + (i - 232) * 10;
  return `rgb(${gray}, ${gray}, ${gray})`;
}

/** Colors already made readable, by text and background. */
const lifted = new Map<string, string>();

/**
 * CSS colors for a span: bold brightens the 8 base colors, as DOS did.
 * `colors` is the Display setting (lib/display.ts): the game's own, made
 * readable on their background, or none.
 */
export function spanColors(span: Span, colors: GameColors = "game"): { color: string; background: string } {
  if (colors === "none") {
    const bright = span.bold || (span.fg?.kind === "index" && span.fg.index >= 8 && span.fg.index !== 8);
    return span.inverse ? { color: "#000000", background: "#aaaaaa" } : { color: bright ? "#ffffff" : DEFAULT_FG, background: DEFAULT_BG };
  }
  let fg = span.fg;
  if (span.bold && fg?.kind === "index" && fg.index < 8) fg = { kind: "index", index: fg.index + 8 };
  const css = (c: Color | undefined, fallback: string) =>
    c === undefined ? fallback : c.kind === "index" ? indexColor(c.index) : `rgb(${c.r}, ${c.g}, ${c.b})`;
  const color = css(fg, span.bold ? "#ffffff" : DEFAULT_FG);
  const background = css(span.bg, DEFAULT_BG);
  const pair = span.inverse ? { color: background === DEFAULT_BG ? "#000000" : background, background: color } : { color, background };
  if (colors !== "lifted") return pair;
  const key = `${pair.color}|${pair.background}`;
  let made = lifted.get(key);
  if (made === undefined) {
    // The game output is black under a transparent background.
    const text = parseColor(pair.color);
    const under = parseColor(pair.background === DEFAULT_BG ? "#000000" : pair.background);
    made = text && under ? rgbCss(readable(text, under)) : pair.color;
    lifted.set(key, made);
  }
  return { color: made, background: pair.background };
}

// ---- Coupler Backup (src-tauri/src/backup.rs) ----

/** What's in a Coupler Backup. */
export interface BackupManifest {
  format: number;
  /** The Coupler version that made it. */
  app: string;
  /** When, in seconds since 1970. */
  made: number;
  files: number;
  /** The files' size, uncompressed. */
  bytes: number;
  settings: number;
}
/** How far a backup or a restore has got, in bytes of the files. */
export interface BackupProgress {
  done: number;
  total: number;
}
/** Writes a Coupler Backup of everything kept to `path`, with the settings (JSON). */
export function backupExport(path: string, settings: string, onProgress: (p: BackupProgress) => void) {
  const progress = new Channel<BackupProgress>();
  progress.onmessage = onProgress;
  return invoke<{ manifest: BackupManifest; size: number }>("backup_export", { path, settings, progress });
}
/** What's in a backup, read from its start. */
export const backupInspect = (path: string) => invoke<BackupManifest>("backup_inspect", { path });
/** Unpacks a backup, checked, to be put in place when Coupler next starts. */
export function backupRestore(path: string, onProgress: (p: BackupProgress) => void) {
  const progress = new Channel<BackupProgress>();
  progress.onmessage = onProgress;
  return invoke<BackupManifest>("backup_restore", { path, progress });
}
/** Starts Coupler again. */
export const appRestart = () => invoke<void>("app_restart");

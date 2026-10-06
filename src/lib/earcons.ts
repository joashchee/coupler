/**
 * Immersive mode's own sounds: short cues made on the spot with Web Audio
 * oscillators and noise, so they need no files, nothing fetched and
 * nothing in the Assets folder. They play on the CUE bus (lib/assets.ts),
 * whose volume is the Mixer's.
 *
 * Each cue is built to be told apart without words, and few enough to
 * learn in a session (never hundreds to memorize):
 *
 * - **Where the exits are**: one note per exit, in turn clockwise from
 *   north. East is in the right ear and west the left; north is higher
 *   and south lower; up glides up and down glides down. A pure tone is a
 *   way already explored, a brighter one with a sparkle a way not yet
 *   taken, a knock a closed door, a low double knock a locked one.
 * - **Moving**: a footstep each room; a rising sparkle when the room is
 *   new to the map; a soft open fifth when the area changes; a bell at a
 *   landmark.
 * - **The body**: a heartbeat while health is under half, faster the
 *   lower it goes; a thud for a blow worth noticing, a shimmer for
 *   healing; a glassy tone when mana runs low, a breath when movement
 *   does.
 * - **The fight**: a low rising call when it starts, a resolving chord
 *   when it ends, and a plucked note each time the opponent's health
 *   crosses a quarter, lower as they weaken.
 * - **Talk**: a two-note ping before a tell or the group is spoken.
 * - **Making a character** (components/CreationDialog.tsx): a soft page
 *   turn for each new question, and a rising fanfare when the new
 *   character comes into the game.
 * - **Who's online** (src-tauri/src/who.rs): a soft doorbell when
 *   someone logs on, rising, and the same falling when they log off. It
 *   stands in for the game's announcement, which isn't written, so it
 *   plays in every way to play, not only with the cues on.
 * - **The journal and the log** (lib/journal.ts): a low wooden
 *   knock-knock when the journal has a line not yet heard and nothing
 *   is about to say it; a soft blip for a line in the log (not
 *   spoken; at most one every 15 seconds, lib/journal.ts), its pitch the
 *   channel's own, so OOC and INFO are told apart; three falling blips
 *   when lines came in unblipped and the log's gone 10 minutes unread.
 *
 * - **Priority Audio** (lib/priority.ts): a cue on the player's list
 *   cuts into the speech queue (`voice.interrupt`): the queue pauses,
 *   the priority bell rings (its sound one of four, chosen in Priority
 *   Audio), the cue plays, the queue picks up.
 *
 * Every cue is the player's to change (`CUES`, `cueSetting`; Workshop's
 * Cues dialog, components/CuesDialog.tsx): its sound on or off, its
 * volume (a share of the CUE bus) and pitch, and its caption (the
 * visual cue, lib/captions.ts) on or off and in their own words. Kept
 * as `coupler.cues`, only what differs from the default.
 */
import { cueOutput } from "./assets";
import { caption } from "./captions";
import * as priority from "./priority";
import * as voice from "./voice";

export type CueId =
  | "exits" | "footstep" | "discovery" | "threshold" | "landmark" | "arrived"
  | "heartbeat" | "hurt" | "healed" | "lowMana" | "lowMoves"
  | "fightStarts" | "fightEnds" | "opponentAt"
  | "tell" | "loggedOn" | "loggedOff" | "logLine" | "logReminder" | "journalWaiting"
  | "problem" | "modeChanged" | "priorityBell" | "thinking"
  | "creationStep" | "creationDone";

export interface CueInfo {
  id: CueId;
  /** Moving, Body, Fight, Talk, Create or Coupler. */
  group: string;
  name: string;
  /** When it plays and how it sounds. */
  summary: string;
  /** The caption's words, before any detail ("Exits: east, west"). */
  words: string;
  /** Captioned until the player says otherwise. */
  captioned?: boolean;
}

export const CUES: CueInfo[] = [
  { id: "exits", group: "Moving", name: "The exits", words: "Exits", summary: "A note per exit, clockwise from north: east in the right ear, west in the left, north higher, south lower. Pure for a way explored, bright for a new one, a knock for a door, a double knock for a locked one." },
  { id: "footstep", group: "Moving", name: "A footstep", words: "A footstep", summary: "Each time you move to another room." },
  { id: "discovery", group: "Moving", name: "A new room", words: "A new room", summary: "A rising sparkle when the room is new to the map." },
  { id: "threshold", group: "Moving", name: "A new area", words: "A new area", summary: "A soft open fifth when you cross into another area." },
  { id: "landmark", group: "Moving", name: "A landmark", words: "A landmark", summary: "A bell in a room you named." },
  { id: "arrived", group: "Moving", name: "Arrived", words: "Arrived", summary: "Two rising notes when a walk reaches its room." },
  { id: "heartbeat", group: "Body", name: "The heartbeat", words: "A heartbeat", summary: "While your health is under half, faster the lower it goes." },
  { id: "hurt", group: "Body", name: "A blow", words: "A blow", summary: "A thud when you lose a twentieth of your health or more at once." },
  { id: "healed", group: "Body", name: "Healing", words: "Healing", summary: "A shimmer when you gain a tenth of your health or more at once." },
  { id: "lowMana", group: "Body", name: "Mana low", words: "Mana low", summary: "A glassy tone when mana falls to a fifth." },
  { id: "lowMoves", group: "Body", name: "Movement low", words: "Movement low", summary: "A breath when movement falls to a fifth." },
  { id: "fightStarts", group: "Fight", name: "A fight starts", words: "A fight starts", summary: "A low rising call." },
  { id: "fightEnds", group: "Fight", name: "The fight ends", words: "The fight ends", summary: "A resolving chord." },
  { id: "opponentAt", group: "Fight", name: "The opponent weakens", words: "Your opponent's health", summary: "A plucked note each time their health crosses a quarter, lower as they weaken." },
  { id: "tell", group: "Talk", name: "A tell", words: "A tell", summary: "A two-note ping before a tell or the group is spoken." },
  { id: "loggedOn", group: "Talk", name: "Someone logged on", words: "Logged on", summary: "A soft doorbell, rising. Plays in every way to play, in place of the game's announcement." },
  { id: "loggedOff", group: "Talk", name: "Someone logged off", words: "Logged off", summary: "The same doorbell, falling." },
  { id: "logLine", group: "Talk", name: "A line in the log", words: "A line in the log", summary: "A soft blip for a line on a channel, its note the channel's own; at most one every 15 seconds." },
  { id: "logReminder", group: "Talk", name: "The log reminds", words: "Unread in the log", summary: "Three soft blips, falling, when lines came in without a blip and the log has gone 10 minutes unread; with the voice on, the narrator says how many." },
  { id: "journalWaiting", group: "Talk", name: "The journal knocks", words: "Knock knock: the journal has lines not yet heard", summary: "A wooden knock-knock when the journal has a line not yet heard and nothing is about to say it." },
  { id: "problem", group: "Coupler", name: "A problem", words: "A problem", summary: "A low buzz when something went wrong." },
  { id: "priorityBell", group: "Coupler", name: "The priority bell", words: "Priority", summary: "Rung before priority audio when it cuts into speech; its sound is chosen in Priority Audio." },
  { id: "thinking", group: "Coupler", name: "Getting the answer ready", words: "Getting the answer ready", summary: "A soft, high ping each second while Coupler's voice gets your answer ready, until it speaks: your command was heard." },
  { id: "modeChanged", group: "Coupler", name: "A new way to play", words: "A new way to play", summary: "Three rising notes when you choose a way to play." },
  { id: "creationStep", group: "Create", name: "The next question", words: "The next question", summary: "A soft page turn when the game asks the next question about your new character." },
  { id: "creationDone", group: "Create", name: "Into the game", words: "Your character is in the game", summary: "A rising fanfare when your new character first comes into the game." },
];

export interface CueSetting {
  /** The sound plays. */
  sound: boolean;
  /** A share of the CUE bus, 0 to 100. */
  volume: number;
  /** Percent of the cue's own pitch. */
  pitch: number;
  /** The caption is written (in Heard, with sound captions on). */
  caption: boolean;
  /** The caption's words. */
  words: string;
}

/** How far pitch goes, percent of the cue's own. */
export const CUE_PITCH = { least: 50, most: 200 };

const CUES_KEY = "coupler.cues";

export const defaultCue = (id: CueId): CueSetting => {
  const info = CUES.find((c) => c.id === id)!;
  return { sound: true, volume: 100, pitch: 100, caption: info.captioned ?? true, words: info.words };
};

function loadCues(): Partial<Record<CueId, Partial<CueSetting>>> {
  try {
    const kept: unknown = JSON.parse(localStorage.getItem(CUES_KEY) ?? "{}");
    return kept && typeof kept === "object" ? (kept as Partial<Record<CueId, Partial<CueSetting>>>) : {};
  } catch {
    return {};
  }
}

let cueChanges = loadCues();
const cueListeners = new Set<() => void>();

const clamp = (n: unknown, least: number, most: number, otherwise: number) => (typeof n === "number" && Number.isFinite(n) ? Math.max(least, Math.min(most, Math.round(n))) : otherwise);

/** A cue as the player has it. */
export function cueSetting(id: CueId): CueSetting {
  const d = defaultCue(id);
  const kept = cueChanges[id] ?? {};
  return {
    sound: typeof kept.sound === "boolean" ? kept.sound : d.sound,
    volume: clamp(kept.volume, 0, 100, d.volume),
    pitch: clamp(kept.pitch, CUE_PITCH.least, CUE_PITCH.most, d.pitch),
    caption: typeof kept.caption === "boolean" ? kept.caption : d.caption,
    words: typeof kept.words === "string" && kept.words.trim() !== "" ? kept.words.trim() : d.words,
  };
}

/** Whether a cue is as Coupler made it. */
export const cueIsDefault = (id: CueId) => JSON.stringify(cueSetting(id)) === JSON.stringify(defaultCue(id));

/** Changes a cue (null puts it back as it was made), kept at once. */
export function setCue(id: CueId, change: Partial<CueSetting> | null) {
  const d = defaultCue(id);
  const next = change === null ? d : { ...cueSetting(id), ...change };
  next.words = next.words.trim() || d.words;
  const differs = Object.fromEntries(Object.entries(next).filter(([k, v]) => d[k as keyof CueSetting] !== v));
  if (Object.keys(differs).length === 0) delete cueChanges[id];
  else cueChanges = { ...cueChanges, [id]: differs };
  try {
    localStorage.setItem(CUES_KEY, JSON.stringify(cueChanges));
  } catch {
    // Not kept past this run, then.
  }
  if (id === "heartbeat" && heart) {
    // Heard (or captioned) again as it now is.
    const every = heart.every;
    window.clearInterval(heart.timer);
    heart = null;
    heartbeat(every === 500 ? 5 : every === 750 ? 20 : 40);
  }
  cueListeners.forEach((f) => f());
}

/** Every cue back as it was made. */
export function resetCues() {
  cueChanges = {};
  try {
    localStorage.removeItem(CUES_KEY);
  } catch {
    // Nothing kept, then.
  }
  cueListeners.forEach((f) => f());
}

export function onCuesChanged(f: () => void): () => void {
  cueListeners.add(f);
  return () => cueListeners.delete(f);
}

/** A cue's caption, its detail after its words. */
function captionOf(id: CueId, detail?: string) {
  const s = cueSetting(id);
  if (s.caption) caption(`♪ ${detail === undefined ? s.words : `${s.words}: ${detail}`}`);
}

interface Note {
  /** Hz, and where it glides to. */
  freq: number;
  to?: number;
  /** Seconds after the cue starts, and how long. */
  at: number;
  dur: number;
  wave?: OscillatorType | "noise";
  /** 0 to 1, before the bus. */
  gain?: number;
  /** -1 left to 1 right. */
  pan?: number;
  /** A low-pass filter's cutoff, Hz. */
  lowpass?: number;
}

let noiseBuffer: AudioBuffer | null = null;
function noise(context: AudioContext): AudioBuffer {
  if (!noiseBuffer || noiseBuffer.sampleRate !== context.sampleRate) {
    noiseBuffer = context.createBuffer(1, context.sampleRate, context.sampleRate);
    const data = noiseBuffer.getChannelData(0);
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
  }
  return noiseBuffer;
}

/**
 * Plays a cue: at once, or, when it's on the Priority Audio list and a
 * line is being said, after the bell with the speech paused around it
 * (`voice.interrupt`).
 */
function play(id: CueId, notes: Note[], delay = 0, detail?: string, quiet = false) {
  if (!quiet && priority.isCue(id)) voice.interrupt(() => sound(id, notes, delay, detail));
  else sound(id, notes, delay, detail, quiet);
}

/**
 * Plays a cue's notes `delay` seconds from now, at its volume and pitch,
 * and captions it (lib/captions.ts) as it starts, unless `quiet` (each
 * heartbeat: its caption is when it starts or changes). Returns how long
 * it lasts, seconds: 0 when it makes no sound.
 */
function sound(id: CueId, notes: Note[], delay = 0, detail?: string, quiet = false): number {
  if (!quiet) {
    if (delay > 0) window.setTimeout(() => captionOf(id, detail), delay * 1000);
    else captionOf(id, detail);
  }
  const s = cueSetting(id);
  if (!s.sound || s.volume === 0) return 0;
  let out: ReturnType<typeof cueOutput>;
  try {
    out = cueOutput();
  } catch {
    return 0;
  }
  const share = s.volume / 100;
  const pitch = s.pitch / 100;
  notes = notes.map((n) => ({ ...n, freq: n.freq * pitch, to: n.to && n.to * pitch, lowpass: n.lowpass && n.lowpass * pitch, gain: (n.gain ?? 0.3) * share }));
  const { context, bus } = out;
  const t0 = context.currentTime + 0.01 + delay;
  for (const n of notes) {
    const start = t0 + n.at;
    const end = start + n.dur;
    const env = context.createGain();
    const peak = n.gain ?? 0.3;
    env.gain.setValueAtTime(0.0001, start);
    env.gain.exponentialRampToValueAtTime(peak, start + Math.min(0.012, n.dur / 4));
    env.gain.exponentialRampToValueAtTime(0.0001, end);
    let source: AudioScheduledSourceNode;
    if (n.wave === "noise") {
      const s = context.createBufferSource();
      s.buffer = noise(context);
      source = s;
    } else {
      const o = context.createOscillator();
      o.type = n.wave ?? "sine";
      o.frequency.setValueAtTime(n.freq, start);
      if (n.to) o.frequency.exponentialRampToValueAtTime(n.to, end);
      source = o;
    }
    let node: AudioNode = source;
    if (n.lowpass || n.wave === "noise") {
      const filter = context.createBiquadFilter();
      filter.type = "lowpass";
      filter.frequency.value = n.lowpass ?? n.freq;
      node = node.connect(filter);
    }
    node = node.connect(env);
    if (n.pan) {
      const panner = context.createStereoPanner();
      panner.pan.value = Math.max(-1, Math.min(1, n.pan));
      node = node.connect(panner);
    }
    node.connect(bus);
    source.start(start);
    source.stop(end + 0.02);
  }
  return delay + Math.max(0, ...notes.map((n) => n.at + n.dur));
}

// ---- Exits ----

/** Across (east +) and up the compass (north +) for each of the game's direction letters. */
const COMPASS: Record<string, [number, number]> = {
  N: [0, 1], NE: [1, 1], E: [1, 0], SE: [1, -1], S: [0, -1], SW: [-1, -1], W: [-1, 0], NW: [-1, 1],
};
/** Clockwise from north, then up and down. */
const ORDER = ["N", "NE", "E", "SE", "S", "SW", "W", "NW", "U", "D"];
/** Each exit's note, north to south: G5, C5, G4. */
const PITCH = { north: 784, level: 523, south: 392 };
const STEP = 0.12;

export interface ExitCue {
  dir: string;
  visited: boolean;
  door: boolean;
  open: boolean;
  locked: boolean;
}

function exitNotes(exit: ExitCue, at: number): Note[] {
  const [dx, dy] = COMPASS[exit.dir] ?? [0, 0];
  const pan = dx * 0.85;
  const freq = dy > 0 ? PITCH.north : dy < 0 ? PITCH.south : PITCH.level;
  const glide = exit.dir === "U" ? freq * 2 : exit.dir === "D" ? freq / 2 : undefined;
  if (exit.locked) {
    return [
      { freq: freq / 3, at, dur: 0.05, wave: "square", gain: 0.22, pan, lowpass: 500 },
      { freq: freq / 3, at: at + 0.07, dur: 0.05, wave: "square", gain: 0.22, pan, lowpass: 500 },
    ];
  }
  if (exit.door && !exit.open) {
    return [{ freq: freq / 2, to: glide && glide / 2, at, dur: 0.07, wave: "square", gain: 0.2, pan, lowpass: 900 }];
  }
  if (!exit.visited) {
    return [
      { freq, to: glide, at, dur: 0.1, wave: "triangle", gain: 0.24, pan },
      { freq: freq * 3, to: glide && glide * 3, at: at + 0.02, dur: 0.06, gain: 0.07, pan },
    ];
  }
  return [{ freq, to: glide, at, dur: 0.1, gain: 0.22, pan }];
}

const DIR_WORDS: Record<string, string> = {
  N: "north", NE: "northeast", E: "east", SE: "southeast", S: "south", SW: "southwest", W: "west", NW: "northwest", U: "up", D: "down",
};

/** An exit as its caption says it: "east (new)", "north (locked)". */
function exitWords(e: ExitCue): string {
  const tag = e.locked ? " (locked)" : e.door && !e.open ? " (door)" : !e.visited ? " (new)" : "";
  return `${DIR_WORDS[e.dir] ?? e.dir.toLowerCase()}${tag}`;
}

/** The room's exits, one note each, clockwise from north. */
export function exits(list: ExitCue[], delay = 0) {
  const sorted = [...list].sort((a, b) => ORDER.indexOf(a.dir) - ORDER.indexOf(b.dir));
  play("exits", sorted.flatMap((e, i) => exitNotes(e, i * STEP)), delay, sorted.length > 0 ? sorted.map(exitWords).join(", ") : "none");
}

/** How long `exits` takes for this many, in seconds. */
export const exitsLength = (count: number) => count * STEP;

// ---- Moving ----

export function footstep() {
  play("footstep", [
    { freq: 300, at: 0, dur: 0.06, wave: "noise", gain: 0.25, pan: -0.2, lowpass: 380 },
    { freq: 300, at: 0.13, dur: 0.06, wave: "noise", gain: 0.2, pan: 0.2, lowpass: 340 },
  ], 0);
}

/** A room new to the map. */
export function discovery(delay = 0) {
  play(
    "discovery",
    [
      { freq: 1047, at: 0, dur: 0.12, gain: 0.12 },
      { freq: 1319, at: 0.06, dur: 0.12, gain: 0.12 },
      { freq: 1568, at: 0.12, dur: 0.22, gain: 0.12 },
    ],
    delay,
  );
}

/** Another area. */
export function threshold(delay = 0) {
  play(
    "threshold",
    [
      { freq: 262, at: 0, dur: 0.7, wave: "triangle", gain: 0.16 },
      { freq: 392, at: 0.08, dur: 0.7, wave: "triangle", gain: 0.12 },
    ],
    delay,
  );
}

/** A room the player named. */
export function landmark(delay = 0) {
  play(
    "landmark",
    [
      { freq: 880, at: 0, dur: 1.1, gain: 0.16 },
      { freq: 880 * 2.76, at: 0, dur: 0.5, gain: 0.05 },
    ],
    delay,
  );
}

/** A walk reached its room. */
export function arrived() {
  play("arrived", [
    { freq: 523, at: 0, dur: 0.15, gain: 0.18 },
    { freq: 784, at: 0.1, dur: 0.3, gain: 0.18 },
  ], 0);
}

// ---- The body ----

let heart: { timer: number; every: number } | null = null;

function beat() {
  play("heartbeat", [
    { freq: 70, to: 48, at: 0, dur: 0.12, gain: 0.5 },
    { freq: 62, to: 44, at: 0.17, dur: 0.12, gain: 0.38 },
  ], 0, undefined, true);
}

/**
 * The heartbeat for a health percent: none at half or over, then a beat
 * every 1.1 s, 0.75 s under a quarter, 0.5 s under a tenth.
 */
export function heartbeat(percent: number | null) {
  const every = percent === null || percent >= 50 ? 0 : percent < 10 ? 500 : percent < 25 ? 750 : 1100;
  if (heart && heart.every === every) return;
  if (heart) window.clearInterval(heart.timer);
  const was = heart;
  heart = null;
  if (every === 0) {
    if (was) captionOf("heartbeat", "stops");
    return;
  }
  captionOf("heartbeat", every === 500 ? "racing, health under a tenth" : every === 750 ? "fast, health under a quarter" : "slow, health under half");
  beat();
  heart = { timer: window.setInterval(beat, every), every };
}

export function hurt() {
  play("hurt", [
    { freq: 120, to: 50, at: 0, dur: 0.18, gain: 0.4 },
    { freq: 200, at: 0, dur: 0.08, wave: "noise", gain: 0.2, lowpass: 600 },
  ], 0);
}

export function healed() {
  play("healed", [
    { freq: 660, to: 1320, at: 0, dur: 0.35, gain: 0.08 },
    { freq: 990, to: 1980, at: 0.05, dur: 0.3, gain: 0.05 },
  ], 0);
}

export function lowMana() {
  play("lowMana", [
    { freq: 1319, at: 0, dur: 0.5, gain: 0.07 },
    { freq: 1976, at: 0.04, dur: 0.45, gain: 0.05 },
  ], 0);
}

export function lowMoves() {
  play("lowMoves", [{ freq: 900, at: 0, dur: 0.45, wave: "noise", gain: 0.12, lowpass: 900 }], 0);
}

// ---- The fight ----

export function fightStarts() {
  play("fightStarts", [
    { freq: 110, at: 0, dur: 0.18, wave: "sawtooth", gain: 0.18, lowpass: 700 },
    { freq: 165, at: 0.16, dur: 0.3, wave: "sawtooth", gain: 0.2, lowpass: 900 },
  ], 0);
}

export function fightEnds() {
  play("fightEnds", [
    { freq: 262, at: 0, dur: 0.6, wave: "triangle", gain: 0.14 },
    { freq: 330, at: 0.06, dur: 0.55, wave: "triangle", gain: 0.12 },
    { freq: 392, at: 0.12, dur: 0.5, wave: "triangle", gain: 0.12 },
  ], 0);
}

/** The opponent's health reached a quarter: 75, 50, 25 or 0, the note lower as they weaken. */
export function opponentAt(quarter: number) {
  const freq = 220 * 2 ** (Math.max(0, Math.min(100, quarter)) / 50);
  play("opponentAt", [{ freq, at: 0, dur: 0.16, wave: "sawtooth", gain: 0.14, lowpass: 2000 }], 0, `${quarter}%`);
}

// ---- Talk and the rest ----

export function tell() {
  play("tell", [
    { freq: 1175, at: 0, dur: 0.09, gain: 0.14 },
    { freq: 1568, at: 0.09, dur: 0.14, gain: 0.14 },
  ], 0);
}

/** Someone logged on (rising) or off (falling): a muted square doorbell, its own sound. */
export function loggedOnOff(name: string, on: boolean, delay = 0) {
  const [first, second] = on ? [392, 523] : [523, 392];
  play(
    on ? "loggedOn" : "loggedOff",
    [
      { freq: first, at: 0, dur: 0.22, wave: "square", gain: 0.09, lowpass: 1400 },
      { freq: second, at: 0.2, dur: 0.45, wave: "square", gain: 0.09, lowpass: 1400 },
      { freq: second * 2, at: 0.2, dur: 0.3, gain: 0.03 },
    ],
    delay,
    name,
  );
}

/** FNV-1a, so a channel keeps its note from run to run. */
function hashOf(text: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 0x01000193) >>> 0;
  return h;
}

/** A pentatonic scale from A5: any two channels' notes sit well together. */
const LOG_NOTES = [880, 988, 1109, 1319, 1480, 1760, 1976, 2217];

/** A line in the log: one soft blip, at the channel's own note. */
export function logLine(channel: string, delay = 0) {
  const freq = LOG_NOTES[hashOf(channel.toLowerCase()) % LOG_NOTES.length];
  play("logLine", [{ freq, at: 0, dur: 0.06, wave: "triangle", gain: 0.06 }], delay, channel);
}

/** Lines came in without a blip and the log's gone unread a while: three of its blips, falling. */
export function logReminder(count: number, delay = 0) {
  play(
    "logReminder",
    [
      { freq: LOG_NOTES[5], at: 0, dur: 0.06, wave: "triangle", gain: 0.07 },
      { freq: LOG_NOTES[3], at: 0.14, dur: 0.06, wave: "triangle", gain: 0.07 },
      { freq: LOG_NOTES[1], at: 0.28, dur: 0.1, wave: "triangle", gain: 0.07 },
    ],
    delay,
    `${count} ${count === 1 ? "line" : "lines"}`,
  );
}

/** The journal has a line not yet heard: a low wooden knock-knock. */
export function journalWaiting(delay = 0) {
  play(
    "journalWaiting",
    [
      { freq: 330, at: 0, dur: 0.1, wave: "triangle", gain: 0.16, lowpass: 1200 },
      { freq: 247, at: 0.13, dur: 0.14, wave: "triangle", gain: 0.16, lowpass: 1200 },
    ],
    delay,
  );
}

export function problem() {
  play("problem", [{ freq: 150, at: 0, dur: 0.22, wave: "square", gain: 0.12, lowpass: 800 }], 0);
}

/** Each bell's notes (lib/priority.ts's `BELLS`). */
const BELL_NOTES: Record<priority.BellSound, Note[]> = {
  bell: [
    { freq: 1047, at: 0, dur: 0.7, gain: 0.16 },
    { freq: 1047 * 2.76, at: 0, dur: 0.3, gain: 0.05 },
  ],
  chime: [
    { freq: 1319, at: 0, dur: 0.25, wave: "triangle", gain: 0.13 },
    { freq: 1047, at: 0.12, dur: 0.25, wave: "triangle", gain: 0.13 },
    { freq: 1568, at: 0.24, dur: 0.4, wave: "triangle", gain: 0.13 },
  ],
  dingDong: [
    { freq: 659, at: 0, dur: 0.45, gain: 0.18 },
    { freq: 523, at: 0.3, dur: 0.6, gain: 0.18 },
  ],
  ping: [{ freq: 1760, at: 0, dur: 0.18, gain: 0.14 }],
};

/** The priority bell, before priority audio cuts into speech. Returns how long to wait after it, seconds (the start of its fade, not the end of its ring). */
export function priorityBell(bell = priority.bellSound()): number {
  const length = sound("priorityBell", BELL_NOTES[bell]);
  return length === 0 ? 0 : Math.min(length, 0.5);
}
voice.setPriorityBell(() => priorityBell());

/**
 * The answer's being rendered: a soft, high ping, a sine with a quiet
 * octave above, fading fast. Captioned the first time a wait only.
 */
export function thinking(first = true) {
  sound("thinking", [
    { freq: 1568, at: 0, dur: 0.28, gain: 0.05 },
    { freq: 3136, at: 0, dur: 0.12, gain: 0.012 },
  ], 0, undefined, !first);
}
voice.setThinkingSound((first) => thinking(first));

/** A way to play chosen. */
export function modeChanged() {
  play("modeChanged", [
    { freq: 523, at: 0, dur: 0.08, gain: 0.12 },
    { freq: 659, at: 0.07, dur: 0.08, gain: 0.12 },
    { freq: 784, at: 0.14, dur: 0.16, gain: 0.12 },
  ], 0);
}

/** The game asks the next question about a new character: a page turning, a breath of noise and a soft note. */
export function creationStep() {
  play("creationStep", [
    { freq: 2400, to: 900, at: 0, dur: 0.12, wave: "noise", gain: 0.06, lowpass: 2400 },
    { freq: 659, at: 0.08, dur: 0.18, wave: "triangle", gain: 0.1 },
  ], 0);
}

/** A new character comes into the game: a rising major arpeggio and its octave. */
export function creationDone() {
  play("creationDone", [
    { freq: 523, at: 0, dur: 0.14, wave: "triangle", gain: 0.14 },
    { freq: 659, at: 0.12, dur: 0.14, wave: "triangle", gain: 0.14 },
    { freq: 784, at: 0.24, dur: 0.14, wave: "triangle", gain: 0.14 },
    { freq: 1047, at: 0.36, dur: 0.6, wave: "triangle", gain: 0.14 },
    { freq: 523, at: 0.36, dur: 0.6, gain: 0.08 },
  ], 0);
}

/** Stops the heartbeat (what's already playing is too short to need stopping). */
export function stopCues() {
  heartbeat(null);
}

/** Plays a cue as it now is, with made-up details, for the Cues dialog: even with its sound off, so it can be heard before it's turned back on. */
export function preview(id: CueId) {
  const s = cueSetting(id);
  if (!s.sound || s.volume === 0) {
    // Heard at the volume it had, or the default's, while it's off.
    const was = cueChanges[id];
    cueChanges = { ...cueChanges, [id]: { ...was, sound: true, volume: s.volume === 0 ? 100 : s.volume } };
    try {
      preview(id);
    } finally {
      cueChanges = { ...cueChanges, [id]: was ?? {} };
    }
    return;
  }
  switch (id) {
    case "exits":
      return exits([
        { dir: "N", visited: true, door: false, open: true, locked: false },
        { dir: "E", visited: false, door: false, open: true, locked: false },
        { dir: "S", visited: true, door: true, open: false, locked: false },
        { dir: "W", visited: true, door: true, open: false, locked: true },
        { dir: "U", visited: true, door: false, open: true, locked: false },
      ]);
    case "heartbeat":
      play("heartbeat", [
        { freq: 70, to: 48, at: 0, dur: 0.12, gain: 0.5 },
        { freq: 62, to: 44, at: 0.17, dur: 0.12, gain: 0.38 },
        { freq: 70, to: 48, at: 0.75, dur: 0.12, gain: 0.5 },
        { freq: 62, to: 44, at: 0.92, dur: 0.12, gain: 0.38 },
      ], 0, "fast, health under a quarter");
      return;
    case "opponentAt":
      return opponentAt(50);
    case "loggedOn":
      return loggedOnOff("Hassan", true);
    case "loggedOff":
      return loggedOnOff("Hassan", false);
    case "logLine":
      return logLine("OOC");
    case "logReminder":
      return logReminder(12);
    case "footstep": return footstep();
    case "discovery": return discovery();
    case "threshold": return threshold();
    case "landmark": return landmark();
    case "arrived": return arrived();
    case "hurt": return hurt();
    case "healed": return healed();
    case "lowMana": return lowMana();
    case "lowMoves": return lowMoves();
    case "fightStarts": return fightStarts();
    case "fightEnds": return fightEnds();
    case "tell": return tell();
    case "journalWaiting": return journalWaiting();
    case "problem": return problem();
    case "modeChanged": return modeChanged();
    case "priorityBell": return void priorityBell();
    case "thinking": return thinking();
    case "creationStep": return creationStep();
    case "creationDone": return creationDone();
  }
}

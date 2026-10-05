/**
 * Coupler's own voice, for Immersive mode: the system's speech through
 * the WebView's speech synthesis, so a player needs no screen reader.
 * It says only what sound can't: talk, the login, the answers to the
 * say keys, and problems (lib/immersive.ts decides).
 *
 * `urgent` speech cuts off what's being said (an answer to a key, a
 * problem); the rest waits its turn. Cmd+Period stops it all. Everything
 * said is also handed to `onSaid`, for the Heard panel, so nothing is
 * only ever spoken.
 *
 * Kept behind this one module so what speaks can change without
 * touching what calls it: the plan is the system voice
 * from Rust (AVSpeechSynthesizer, through the mixer) for this, Flite
 * where there's none, and Pocket TTS, a voice per speaker, for talk. Rate and volume are the
 * Mixer's, kept as `coupler.voice.rate` and `coupler.volume.voice`.
 *
 * Talk comes with its speaker's voice (src-tauri/src/cast.rs, edited in
 * gear → Characters' Voices), from any engine: the system's voices
 * (here, through the WebView) or one bundled in Coupler (Flite, and
 * Pocket TTS's 19 neural voices, rendered by src-tauri/src/synth.rs and
 * played through Web Audio at the VOX volume). Automatic (`characterVoice`) picks from all of them, for the
 * most different voices: the system's of the character's kind, never
 * the narrator's own nor a novelty like Zarvox, and every bundled one,
 * pitched to suit; the same one for the same seed while the voices stay
 * the same. Lines from every engine share one queue, so urgent speech
 * and Cmd+Period cut off whichever is talking.
 *
 * A line can ask to be told how it went (`onDone`): `true` once it was
 * said to the end, `false` when it was cut off, flushed or never said
 * (no volume, a quiet character). The journal (lib/journal.ts) marks a
 * line heard only on `true`.
 *
 * **The narrator** is the voice of everything Coupler says itself (a
 * line given no voice): the login, the answers to the say keys, the time
 * of day, and who's talking before they talk ("Hassan says", then
 * Hassan's own voice says the words: `speakTalk`). The player picks it
 * in Characters' Voices (any voice of any engine, its pitch and speed;
 * `coupler.narrator`); until then it's the system's own voice. Automatic
 * never gives a character the narrator's voice.
 *
 * A line of talk also says which `book` it's from: in a bundled voice it
 * comes from the voice cache (src-tauri/src/voicecache.rs), where
 * `prerender` put it the moment it arrived, so it starts without waiting
 * to be rendered. The system's voices speak through the WebView and
 * can't be rendered ahead. `onSpeaking` hears each line of talk as it
 * starts (Immersive's pop-up, components/SpeakingPopup.tsx) and null
 * when it ends or is cut off.
 */
import { sharedContext } from "./assets";
import * as mud from "./mud";
import type { Voice } from "./mud";

const RATE_KEY = "coupler.voice.rate";
const NARRATOR_KEY = "coupler.narrator";
const VOLUME_KEY = "coupler.volume.voice";

function kept(key: string, fallback: number, least: number, most: number): number {
  try {
    const raw = localStorage.getItem(key);
    const n = Number(raw);
    return raw !== null && Number.isFinite(n) ? Math.min(Math.max(n, least), most) : fallback;
  } catch {
    return fallback;
  }
}

/** Percent of normal speed, 50 to 300. */
let rate = kept(RATE_KEY, 130, 50, 300);
/** 0 to 100. */
let volume = kept(VOLUME_KEY, 100, 0, 100);
/** Utterances in flight: WebKit drops an utterance's events if nothing holds it. */
const speaking = new Set<SpeechSynthesisUtterance>();
const listeners = new Set<(text: string) => void>();

/** Whether this system can speak at all. */
export const canSpeak = () => typeof window !== "undefined" && "speechSynthesis" in window;

export function voiceRate() {
  return rate;
}
export function voiceVolume() {
  return volume;
}

function store(key: string, value: number) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    // Not kept past this run, then.
  }
}

export function setVoiceRate(percent: number) {
  rate = Math.min(Math.max(Math.round(percent), 50), 300);
  store(RATE_KEY, rate);
}

export function setVoiceVolume(percent: number) {
  volume = Math.min(Math.max(Math.round(percent), 0), 100);
  store(VOLUME_KEY, volume);
  if (vox) vox.gain.value = volume / 100;
}

/** Calls `f` with each thing said. */
export function onSaid(f: (text: string) => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

// ---- The system's voices, for the characters ----

/** macOS's voices by first name: feminine (true) or masculine (false). */
const GENDERS: Record<string, boolean> = Object.fromEntries([
  ...["samantha", "karen", "moira", "tessa", "fiona", "victoria", "allison", "ava", "susan", "vicki", "kathy", "veena", "serena", "kate", "nicky", "zoe", "shelley", "sandy", "flo", "grandma", "agnes", "princess", "isha", "catherine", "martha", "joelle", "noelle", "kyoko", "amelie", "anna", "alice", "ellen", "laura", "paulina", "monica", "sara", "melina", "yuna", "ting-ting", "mei-jia", "sin-ji", "zuzana", "carmit", "milena", "lekha", "damayanti", "kanya", "ioana", "mariska", "nora", "satu", "alva", "yelda", "lesya", "linh", "amira", "zosia", "luciana", "joana", "kiyara", "soumya", "emma"].map((n) => [n, true]),
  ...["alex", "daniel", "fred", "ralph", "bruce", "tom", "lee", "oliver", "rishi", "aaron", "arthur", "evan", "nathan", "gordon", "junior", "eddy", "reed", "rocko", "grandpa", "thomas", "jacques", "xander", "diego", "jorge", "juan", "luca", "yuri", "maged", "tarik", "otoya", "aru", "nicolas", "jamie", "malcolm", "stephen", "fergus", "lars"].map((n) => [n, false]),
]);
/** Voices for fun, not for people: chosen only by name. */
const NOVELTY = new Set(["albert", "bad news", "bahh", "bells", "boing", "bubbles", "cellos", "good news", "jester", "organ", "pipe organ", "superstar", "trinoids", "whisper", "wobble", "zarvox", "hysterical", "deranged"]);

const firstName = (v: SpeechSynthesisVoice) => v.name.toLowerCase().replace(/\s*\(.*$/, "").trim();

/** The system's English voices, by name: the game is in English. */
export function systemVoices(): SpeechSynthesisVoice[] {
  if (!canSpeak()) return [];
  const all = window.speechSynthesis.getVoices().filter((v) => v.lang.toLowerCase().startsWith("en"));
  return all.sort((a, b) => a.name.localeCompare(b.name));
}

/** Called when any engine's voices change. */
const voicesListeners = new Set<() => void>();

/** Calls `f` when the voices change: the system's can arrive after the first look, and the bundled ones come from Rust. */
export function onVoicesChanged(f: () => void): () => void {
  voicesListeners.add(f);
  if (canSpeak()) window.speechSynthesis.addEventListener("voiceschanged", f);
  return () => {
    voicesListeners.delete(f);
    if (canSpeak()) window.speechSynthesis.removeEventListener("voiceschanged", f);
  };
}

/** Whether a voice is a novelty (chosen only by name). */
export const isNovelty = (v: SpeechSynthesisVoice) => NOVELTY.has(firstName(v));

// ---- Every engine's voices ----

/** The bundled engines' voices (src-tauri/src/synth.rs), once Rust has said. */
let bundled: mud.BundledVoice[] = [];
void mud
  .voiceEngines()
  .then((voices) => {
    bundled = voices;
    voicesListeners.forEach((f) => f());
  })
  .catch(() => {
    // Not in Tauri (a test page): the system's voices only.
  });

/** A voice of any engine. */
export interface EngineVoice {
  /** `system`, or a bundled engine's ID. */
  engine: string;
  /** A system voice's name, a bundled one's ID. */
  id: string;
  name: string;
  /** Null when Coupler doesn't know (a system voice not in its list). */
  gender: Voice["gender"] | null;
  /** Suits either kind: Automatic offers it to every character. */
  anyGender: boolean;
  novelty: boolean;
  /** The system voice itself. */
  system?: SpeechSynthesisVoice;
}

/** The engines' names, as the player sees them. */
export const ENGINE_NAMES: Record<string, string> = { system: "System", flite: "Flite (built in)", pocket: "Pocket TTS (built in)" };

/** Every voice of every engine: the bundled ones, then the system's English ones. */
export function allVoices(): EngineVoice[] {
  const system = systemVoices().map((v): EngineVoice => {
    const g = GENDERS[firstName(v)];
    return { engine: "system", id: v.name, name: v.name, gender: g === undefined ? null : g ? "feminine" : "masculine", anyGender: false, novelty: isNovelty(v), system: v };
  });
  return [...bundled.map((b): EngineVoice => ({ engine: b.engine, id: b.id, name: b.name, gender: b.gender, anyGender: b.anyGender, novelty: false })), ...system];
}

/**
 * What Automatic picks from: no novelties, never the narrator's own
 * voice, every voice of the character's kind (the system's, Pocket
 * TTS's), and those that suit either (Flite's, pitched to suit by Rust).
 */
function automaticPool(voices: EngineVoice[], how: Voice): EngineVoice[] {
  const people = voices.filter((v) => !v.novelty);
  const chosen = narratorChoice(voices);
  const narrator = chosen ? people.find((v) => v.engine === chosen.engine && v.id === chosen.id) : people.find((v) => v.system?.default);
  const others = people.filter((v) => v !== narrator);
  const fits = others.filter((v) => v.anyGender || v.gender === how.gender);
  return fits.length > 0 ? fits : others.length > 0 ? others : people;
}

/** The voice a character speaks in: the one chosen, or the seed's pick, within the chosen engine or across all; null for the system's own. */
export function characterVoice(how: Voice): EngineVoice | null {
  const all = allVoices();
  const ofEngine = how.engine ? all.filter((v) => v.engine === how.engine) : [];
  if (how.voiceName) {
    const chosen = ofEngine.find((v) => v.id === how.voiceName);
    if (chosen) return chosen;
  }
  // An engine that's gone (or has nothing) falls back to every engine.
  const pool = automaticPool(ofEngine.length > 0 ? ofEngine : all, how);
  return pool.length > 0 ? pool[how.seed % pool.length] : null;
}

// ---- The narrator ----

/** The narrator as first heard: the system's own voice, as it is. */
export const DEFAULT_NARRATOR: Voice = { gender: "feminine", pitch: 100, rate: 100, engine: null, voiceName: null, seed: 0, quiet: false };

function loadNarrator(): Voice {
  try {
    const raw = JSON.parse(localStorage.getItem(NARRATOR_KEY) ?? "null") as Partial<Voice> | null;
    if (!raw || typeof raw !== "object") return DEFAULT_NARRATOR;
    const n = (v: unknown, least: number, most: number, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? Math.min(Math.max(v, least), most) : fallback);
    return {
      ...DEFAULT_NARRATOR,
      engine: typeof raw.engine === "string" ? raw.engine : null,
      voiceName: typeof raw.voiceName === "string" ? raw.voiceName : null,
      pitch: n(raw.pitch, mud.PITCH.least, mud.PITCH.most, 100),
      rate: n(raw.rate, mud.RATE.least, mud.RATE.most, 100),
    };
  } catch {
    return DEFAULT_NARRATOR;
  }
}

let narrator = loadNarrator();
const narratorListeners = new Set<(v: Voice) => void>();

/** The narrator's voice as set: an engine and a voice of it, or neither for the system's own. */
export function narratorVoice(): Voice {
  return narrator;
}

/** Sets the narrator (null: back to the system's own voice), kept. */
export function setNarratorVoice(v: Voice | null) {
  narrator = v ? { ...v, gender: DEFAULT_NARRATOR.gender, seed: 0, quiet: false } : DEFAULT_NARRATOR;
  try {
    if (v) localStorage.setItem(NARRATOR_KEY, JSON.stringify(narrator));
    else localStorage.removeItem(NARRATOR_KEY);
  } catch {
    // Not kept past this run, then.
  }
  narratorListeners.forEach((f) => f(narrator));
}

/** Calls `f` when the narrator changes. */
export function onNarratorChanged(f: (v: Voice) => void): () => void {
  narratorListeners.add(f);
  return () => narratorListeners.delete(f);
}

/** The voice the narrator speaks in, or null for the system's own (also when the chosen one's gone). */
function narratorChoice(voices: EngineVoice[] = allVoices()): EngineVoice | null {
  if (!narrator.engine || !narrator.voiceName) return null;
  return voices.find((v) => v.engine === narrator.engine && v.id === narrator.voiceName) ?? null;
}

/** The narrator's voice, as the player sees it: "Samantha", "Kal (built in)", or the system's own. */
export function narratorName(): string {
  const chosen = narratorChoice();
  if (chosen) return chosen.engine === "system" ? chosen.name : `${chosen.name} (built in)`;
  const own = systemVoices().find((v) => v.default);
  return own ? `the system's voice (${own.name})` : "the system's voice";
}

/**
 * A line of talk as the game prints it, cut into who says it and what:
 * "Hassan says 'Well met.'" is "Hassan says", "Well met." and nothing
 * after; "You hear someone say 'Help!' to the north." has ", to the
 * north" after. Null when there are no quoted words (an emote).
 */
export function splitTalk(text: string): { lead: string; words: string; after: string } | null {
  const open = text.indexOf(" '");
  const close = text.lastIndexOf("'");
  if (open < 0 || close <= open + 1) return null;
  const words = text.slice(open + 2, close).trim();
  if (words === "") return null;
  // "[GOSSIP] Bob" is said as "GOSSIP, Bob".
  const lead = text
    .slice(0, open)
    .replace(/^\[([^\]]+)\]\s*/, "$1, ")
    .trim();
  const after = text.slice(close + 1).replace(/^[\s.]+$/, "").trim();
  return { lead, words, after };
}

// ---- Speaking: one line at a time, whichever engine says it ----

/** What a line can say about itself: all optional. */
export interface LineInfo {
  /** Hears whether it was said to the end. */
  onDone?: (finished: boolean) => void;
  /** Talk from the journal or the log: cached, and shown while it's said. */
  book?: mud.Book;
  /** Who's saying it, for the pop-up. */
  speaker?: string;
}

/** A line of talk being said, for the pop-up. */
export interface Speaking {
  id: number;
  text: string;
  speaker: string;
  book: mud.Book;
}

interface Line extends LineInfo {
  words: string;
  how: Voice | null;
  /** What the pop-up shows, when it's more than the words said (the whole line of talk). */
  shown?: string;
}
const waiting: Line[] = [];
let busy = false;
/** The line being said. */
let current: Line | null = null;
let lineId = 0;
const speakingListeners = new Set<(line: Speaking | null) => void>();

/** Calls `f` with each line of talk as it starts being said, and null when it stops. */
export function onSpeaking(f: (line: Speaking | null) => void): () => void {
  speakingListeners.add(f);
  return () => speakingListeners.delete(f);
}

function tellSpeaking(line: Line | null) {
  const talk: Speaking | null = line?.book ? { id: ++lineId, text: line.shown ?? line.words, speaker: line.speaker ?? "", book: line.book } : null;
  speakingListeners.forEach((f) => f(talk));
}
/** Bumped by `hush`: a line that ends after it doesn't start the next. */
let generation = 0;
/** Stops the line being said. */
let stopLine: (() => void) | null = null;
/** The bundled engines' volume, under the Mixer's VOX. */
let vox: GainNode | null = null;

function voxOutput(): { context: AudioContext; gain: GainNode } {
  const context = sharedContext();
  if (!vox) {
    vox = context.createGain();
    vox.connect(context.destination);
  }
  vox.gain.value = volume / 100;
  return { context, gain: vox };
}

/**
 * Words as they should sound, for every engine: keys as words ("Esc" is
 * "escape", "Cmd+Shift+L" is "command shift L", "Ctrl", "Opt"), and
 * initialisms a voice would say as one word spelt out ("OOC" is "O O C",
 * not "ock"). What's shown is never changed, only what's said.
 */
const SPELT = ["OOC", "IC", "AFK", "PK", "PVP", "NPC", "XP", "HP"];
export function pronounce(words: string): string {
  return words
    .replace(/\b(Cmd|Ctrl|Opt|Option|Shift|Esc|Alt)\+(?=\S)/g, "$1 ")
    .replace(/\bEsc\b/gi, "escape")
    .replace(/\bCmd\b/gi, "command")
    .replace(/\bCtrl\b/gi, "control")
    .replace(/\bOpt\b/g, "option")
    .replace(new RegExp(`\\b(${SPELT.join("|")})\\b`, "g"), (word) => word.split("").join(" "));
}

/** The speed a line is said at, percent: the player's times the character's. */
const lineRate = (how: Voice | null) => (rate * (how?.rate ?? 100)) / 100;

function next() {
  if (busy) return;
  const line = waiting.shift();
  if (!line) return;
  busy = true;
  current = line;
  tellSpeaking(line);
  const mine = generation;
  const done = (finished = true) => {
    if (mine !== generation) return;
    busy = false;
    stopLine = null;
    current = null;
    if (line.book) tellSpeaking(null);
    line.onDone?.(finished);
    next();
  };
  // A line with no voice is the narrator's.
  const how = line.how ?? narrator;
  const chosen = line.how ? characterVoice(line.how) : narratorChoice();
  const words = pronounce(line.words);
  if (chosen && chosen.engine !== "system") sayBundled(words, how, chosen, done, mine, line.book);
  else saySystem(words, how, chosen?.system ?? null, done);
}

function saySystem(words: string, how: Voice | null, system: SpeechSynthesisVoice | null, done: (finished?: boolean) => void) {
  if (!canSpeak()) return done(false);
  const u = new SpeechSynthesisUtterance(words);
  u.rate = lineRate(how) / 100;
  u.volume = volume / 100;
  if (how) u.pitch = Math.min(how.pitch / 100, 2);
  if (system) u.voice = system;
  // WebKit sometimes never ends an utterance: don't let the queue wait forever.
  const timer = window.setTimeout(() => finish(true), 4000 + (words.length * 90 * 100) / lineRate(how));
  function finish(finished: boolean) {
    window.clearTimeout(timer);
    speaking.delete(u);
    done(finished);
  }
  u.onend = () => finish(true);
  u.onerror = () => finish(false);
  speaking.add(u);
  stopLine = () => {
    window.clearTimeout(timer);
    window.speechSynthesis.cancel();
    speaking.clear();
  };
  window.speechSynthesis.speak(u);
}

function sayBundled(words: string, how: Voice, chosen: EngineVoice, done: (finished?: boolean) => void, mine: number, book?: mud.Book) {
  mud
    .voiceSynth(chosen.engine, chosen.id, words, how.gender, how.pitch, lineRate(how), book)
    .then((bytes) => {
      if (mine !== generation) return;
      const view = new DataView(bytes);
      const samples = (bytes.byteLength - 4) >> 1;
      if (samples <= 0) return done(false);
      const { context, gain } = voxOutput();
      const buffer = context.createBuffer(1, samples, view.getUint32(0, true));
      const channel = buffer.getChannelData(0);
      for (let i = 0; i < samples; i++) channel[i] = view.getInt16(4 + 2 * i, true) / 32768;
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.connect(gain);
      source.onended = () => done(true);
      stopLine = () => {
        source.onended = null;
        try {
          source.stop();
        } catch {
          // Already over.
        }
      };
      source.start();
    })
    .catch((e) => {
      // Still heard, in the system's voice, rather than lost.
      console.warn(`Coupler: ${chosen.name} couldn't speak: ${String(e)}`);
      if (mine === generation) saySystem(words, how, null, done);
    });
}

/**
 * Says `text`: at once, cutting off anything being said, when urgent;
 * after what's queued otherwise. A character's `how` gives it their
 * voice, from whichever engine; `info` says whose talk it is and
 * hears whether it was said to the end.
 */
export function speak(text: string, urgent = false, how: Voice | null = null, info: LineInfo = {}) {
  const words = text.trim();
  if (words === "") return info.onDone?.(false);
  listeners.forEach((f) => f(words));
  if (volume === 0 || how?.quiet) return info.onDone?.(false);
  if (urgent) hush();
  waiting.push({ words, how, ...info });
  next();
}

/**
 * Says a line of talk as the game prints it: the narrator says who
 * ("Hassan says"), then the speaker's voice (`how`) says the words, then
 * the narrator anything after them. Heard as one line; `info` (the
 * pop-up, whether it was heard) goes with the words. A line with no
 * quoted words (an emote) is said whole in the speaker's voice.
 */
export function speakTalk(text: string, how: Voice | null, info: LineInfo = {}) {
  const parts = splitTalk(text.trim());
  if (!parts) return speak(text, false, how, info);
  listeners.forEach((f) => f(text.trim()));
  if (volume === 0 || how?.quiet) return info.onDone?.(false);
  if (parts.lead) waiting.push({ words: parts.lead, how: null });
  waiting.push({ words: parts.words, how, shown: text.trim(), ...info });
  if (parts.after) waiting.push({ words: parts.after, how: null });
  next();
}

/** What a line of talk's speaker says themselves: the quoted words, or all of it (an emote). */
const ownWords = (text: string) => splitTalk(text.trim())?.words ?? text;

/**
 * Renders a line of talk ahead, into the voice cache, when its speaker's
 * voice is a bundled one: so it plays at once whenever it's wanted. The
 * same voice, pitch and speed `speak` would use, so it's the same key.
 */
export function prerender(text: string, how: Voice | null, book: mud.Book) {
  const words = pronounce(ownWords(text).trim());
  if (words === "" || !how || how.quiet) return;
  const chosen = characterVoice(how);
  if (!chosen || chosen.engine === "system") return;
  void mud.voicePrerender(chosen.engine, chosen.id, words, how.gender, how.pitch, lineRate(how), book).catch(() => {
    // Not in Tauri, or the engine's gone: it's rendered when it's played.
  });
}

/** Whether a line is being said or waiting its turn. */
export function talking(): boolean {
  return busy || waiting.length > 0;
}

/** Stops speaking, and forgets what was waiting: none of it was heard. */
export function hush() {
  generation++;
  const cut = [...(current ? [current] : []), ...waiting.splice(0)];
  if (current?.book) tellSpeaking(null);
  current = null;
  busy = false;
  const stop = stopLine;
  stopLine = null;
  stop?.();
  if (canSpeak()) {
    window.speechSynthesis.cancel();
    speaking.clear();
  }
  cut.forEach((line) => line.onDone?.(false));
}

/**
 * Whether a line is worth saying: words, not art. A line of mostly
 * symbols (a banner, a border, a map) is skipped.
 */
export function speakable(text: string): boolean {
  const visible = text.replace(/\s/g, "");
  if (visible.length === 0) return false;
  const letters = (visible.match(/[\p{L}\p{N}]/gu) ?? []).length;
  return letters / visible.length >= 0.6;
}

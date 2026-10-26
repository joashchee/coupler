/**
 * Playing what the hooks' triggers set off (src-tauri/src/hooks.rs):
 * sound effects, music and pictures from the Assets folder
 * (src-tauri/src/assets.rs).
 *
 * Sound goes through one Web Audio context and two buses, SFX and BGM,
 * whose volumes are the Mixer's (kept per player: `coupler.volume.sfx`,
 * `coupler.volume.bgm`). Each trigger's own volume is a share of its
 * bus. A sound effect plays once, over anything else. Music is one piece
 * at a time: a trigger for other music stops what's playing and starts
 * it; a trigger for the music already playing (every room of a zone,
 * say) leaves it playing, at the new trigger's volume. The bytes come
 * from Rust as WAV or MP3, Rust having rendered what the WebView can't
 * play (Ogg, MIDI, tracker modules); each is decoded once and kept.
 * Two more buses are ambience, BGN (background noise) and BGW
 * (background weather): Rust says what should loop now
 * (src-tauri/src/ambient.rs, the `ambient` event) and `setLoop` makes it
 * so, fading the old loop out and the new one in, and leaving one that's
 * already playing as it is (only its volume follows).
 * The place's music (a BGM hooked to the room or its area) comes the
 * same way, in the `ambient` event: `setPlaceMusic` fades it in and out
 * as the player comes and goes, and out for a fight.
 * Leaving the game stops everything at once (`stopAll`); leaving a
 * character for another, or for the account menu, fades it out (`fadeAll`).
 * One more bus, CUE, carries Immersive mode's own sounds, made here
 * rather than played from the Assets folder (lib/earcons.ts).
 * While a voice speaks (lib/voice.ts), SFX, BGM, BGN and BGW are
 * ducked together (`duck`): lowered, never stopped, and brought back
 * up once the voice is done; CUE isn't.
 * A sound effect with a place (someone coming or going,
 * src-tauri/src/foley.rs) is heard from there, as Immersive's exits are
 * (`COMPASS`): east in the right ear, west in the left, north a little
 * higher, south a little lower, up and down further; its caption says
 * where in words.
 * Pictures are images, or ANSI art that Rust has drawn into styled rows
 * of characters.
 * The Hooks dialog's Assets tab tries one asset at a time (`playPreview`,
 * `stopPreview`): a sound on the SFX bus, music and a SoundFont's sample
 * tune on the BGM bus, over whatever the game has set off.
 */
import { caption } from "./captions";
import * as mud from "./mud";
import { palette } from "./palette";

export type Bus = "sfx" | "bgm" | "bgn" | "bgw" | "cue";
export const BUSES: Bus[] = ["sfx", "bgm", "bgn", "bgw", "cue"];
const VOLUME_KEY: Record<Bus, string> = { sfx: "coupler.volume.sfx", bgm: "coupler.volume.bgm", bgn: "coupler.volume.bgn", bgw: "coupler.volume.bgw", cue: "coupler.volume.cue" };

function storedVolume(bus: Bus): number {
  try {
    const n = Number(localStorage.getItem(VOLUME_KEY[bus]));
    return localStorage.getItem(VOLUME_KEY[bus]) !== null && Number.isFinite(n) ? Math.min(Math.max(n, 0), 100) : 100;
  } catch {
    return 100;
  }
}

const volumes: Record<Bus, number> = { sfx: storedVolume("sfx"), bgm: storedVolume("bgm"), bgn: storedVolume("bgn"), bgw: storedVolume("bgw"), cue: storedVolume("cue") };
let context: AudioContext | null = null;
let buses: Record<Bus, GainNode> | null = null;
/**
 * The game's sound (SFX, BGM, BGN, BGW) on its way out: lowered while
 * the narrator or a character speaks, for the words to be clear
 * (`duck`). Coupler's cues aren't: they're short, and say things too.
 */
let ducker: GainNode | null = null;
/** How loud the game's sound is under a voice (of the Mixer's), and how fast it goes down and comes back up, seconds. */
const DUCKED = 0.3;
const DUCK_DOWN = 0.15;
const DUCK_UP = 0.6;
let ducked = false;

function audio(): { context: AudioContext; buses: Record<Bus, GainNode> } {
  if (!context || !buses) {
    context = new AudioContext();
    ducker = context.createGain();
    ducker.gain.value = ducked ? DUCKED : 1;
    ducker.connect(context.destination);
    const bus = (b: Bus) => {
      const gain = context!.createGain();
      gain.gain.value = volumes[b] / 100;
      gain.connect(b === "cue" ? context!.destination : ducker!);
      return gain;
    };
    buses = { sfx: bus("sfx"), bgm: bus("bgm"), bgn: bus("bgn"), bgw: bus("bgw"), cue: bus("cue") };
    // iOS's WebKit stops the context ("interrupted") when the system's
    // speech starts, and doesn't start it again: the loops went quiet the
    // first time the narrator spoke. Started again as soon as it stops.
    const ours = context;
    // Not while the system's still speaking: that would fight it for the
    // sound; the narrator's line ending starts it (`keepAudio`).
    ours.addEventListener("statechange", () => {
      const speaking = typeof window !== "undefined" && "speechSynthesis" in window && window.speechSynthesis.speaking;
      if (ours === context && !speaking) resume(ours);
    });
  }
  // WebKit starts a context suspended until the page has been used.
  resume(context);
  return { context, buses };
}

/** Starts the context again if it isn't running: suspended, or interrupted (iOS's WebKit, by speech or a call). */
function resume(c: AudioContext) {
  // "interrupted" is WebKit's own state, not in the DOM's types.
  const state = c.state as string;
  if (state === "running" || state === "closed") return;
  c.resume().catch(() => {
    // Not allowed till the page is used again: the next key or touch tries (`wakeAudio`).
  });
}

/** The context and the CUE bus, for sounds made rather than played (lib/earcons.ts). */
export function cueOutput(): { context: AudioContext; bus: GainNode } {
  const { context, buses } = audio();
  return { context, bus: buses.cue };
}

/** The context, for Coupler's voice's bundled engines (lib/voice.ts), which add their own volume. */
export function sharedContext(): AudioContext {
  return audio().context;
}

/** Lets sound start, or start again after the system stopped it: call from a click or a key. */
export function wakeAudio() {
  audio();
}

/** Tries again after a line ends, ms: iOS ends its interruption a moment after the speech, not with it. */
const KEEP_RETRIES = [250, 1000, 2500];
let keepTimers: number[] = [];

/** Starts the sound again if the system stopped it (the narrator's line just ended, say); makes no context. */
export function keepAudio() {
  if (!context) return;
  resume(context);
  keepTimers.forEach((t) => window.clearTimeout(t));
  const ours = context;
  keepTimers = KEEP_RETRIES.map((ms) =>
    window.setTimeout(() => {
      const speaking = "speechSynthesis" in window && window.speechSynthesis.speaking;
      if (ours === context && !speaking) resume(ours);
    }, ms),
  );
}

/**
 * Lowers the game's sound under a voice (`on`), or brings it back:
 * nothing stops, it only goes quieter (lib/voice.ts calls it as lines
 * start and the queue empties).
 */
export function duck(on: boolean) {
  if (on === ducked) return;
  ducked = on;
  if (!context || !ducker) return;
  const gain = ducker.gain;
  const t = context.currentTime;
  gain.cancelScheduledValues(t);
  gain.setValueAtTime(gain.value, t);
  gain.linearRampToValueAtTime(on ? DUCKED : 1, t + (on ? DUCK_DOWN : DUCK_UP));
}

/** The Mixer's volume for a bus, 0 to 100. */
export function masterVolume(bus: Bus): number {
  return volumes[bus];
}

export function setMasterVolume(bus: Bus, percent: number) {
  volumes[bus] = Math.min(Math.max(Math.round(percent), 0), 100);
  try {
    localStorage.setItem(VOLUME_KEY[bus], String(volumes[bus]));
  } catch {
    // Not kept past this run, then.
  }
  if (buses) buses[bus].gain.value = volumes[bus] / 100;
}

const decoded = new Map<string, Promise<AudioBuffer>>();

function load(path: string, looping: boolean): Promise<AudioBuffer> {
  const key = `${path}\n${looping}`;
  let buffer = decoded.get(key);
  if (!buffer) {
    buffer = mud.assetAudio(path, looping).then((bytes) => audio().context.decodeAudioData(bytes));
    // A failure isn't kept: the file may be fixed, or a SoundFont added.
    buffer.catch(() => decoded.delete(key));
    decoded.set(key, buffer);
  }
  return buffer;
}

/** Across (east +) and up the compass (north +) for each of the game's direction letters. */
export const COMPASS: Record<string, [number, number]> = {
  N: [0, 1], NE: [1, 1], E: [1, 0], SE: [1, -1], S: [0, -1], SW: [-1, -1], W: [-1, 0], NW: [-1, 1],
};

/** Where a sound is heard from: left to right, and how much higher or lower it plays. */
export function placing(dir: string): { pan: number; rate: number } {
  const [dx, dy] = COMPASS[dir] ?? [0, 0];
  // About a semitone and a half for north and south, three for up and down.
  const rate = dir === "U" ? 1.19 : dir === "D" ? 0.84 : dy > 0 ? 1.09 : dy < 0 ? 0.92 : 1;
  return { pan: dx * 0.85, rate };
}

const FROM_WORDS: Record<string, string> = {
  N: "from the north", NE: "from the northeast", E: "from the east", SE: "from the southeast", S: "from the south",
  SW: "from the southwest", W: "from the west", NW: "from the northwest", U: "from above", D: "from below",
};
const TOWARD_WORDS: Record<string, string> = {
  N: "going north", NE: "going northeast", E: "going east", SE: "going southeast", S: "going south",
  SW: "going southwest", W: "going west", NW: "going northwest", U: "going up", D: "going down",
};

/** A buffer played through its own volume, into a bus; panned and pitched if it has a place. */
function start(buffer: AudioBuffer, bus: Bus, percent: number, looping: boolean, place?: mud.Place) {
  const { context, buses } = audio();
  const source = context.createBufferSource();
  source.buffer = buffer;
  source.loop = looping;
  const gain = context.createGain();
  gain.gain.value = percent / 100;
  if (place) {
    const { pan, rate } = placing(place.dir);
    source.playbackRate.value = rate;
    const panner = context.createStereoPanner();
    panner.pan.value = pan;
    source.connect(gain).connect(panner).connect(buses[bus]);
  } else {
    source.connect(gain).connect(buses[bus]);
  }
  source.start();
  return { source, gain };
}

const effects = new Set<{ source: AudioBufferSourceNode; gain: GainNode }>();

/** Plays a sound effect once, at `percent` of the SFX volume, heard from `place` if it has one. */
export async function playEffect(path: string, percent = 100, place?: mud.Place) {
  const buffer = await load(path, false);
  const playing = start(buffer, "sfx", percent, false, place);
  const where = place && (place.toward ? TOWARD_WORDS : FROM_WORDS)[place.dir];
  caption(`♪ ${mud.assetTitle(path)}${where ? `, ${where}` : ""}`);
  effects.add(playing);
  playing.source.onended = () => effects.delete(playing);
}

// ---- Music: one piece at a time ----

/**
 * The music playing: set off by a trigger, the place's (`place`,
 * `setPlaceMusic`), or the Music Editor's song (`editor`, `playEditorSong`:
 * `#play`), which holds the BGM channel till it stops; `taken` tells it
 * when something else stops it.
 */
let music: {
  path: string;
  /** What the layers and the menu bar call it. */
  title: string;
  looping: boolean;
  percent: number;
  playing: { source: AudioBufferSourceNode; gain: GainNode } | null;
  /** The place's music: what Rust asked for. */
  place?: mud.PlaceMusic;
  editor?: boolean;
  taken?: () => void;
} | null = null;
/** Bumped by every music request, so one that finishes loading late doesn't start. */
let musicTurn = 0;
/** The place's music Rust last asked for while the Music Editor's song played: it comes in once that stops. */
let placeWaiting: mud.PlaceMusic | null = null;
const musicListeners = new Set<(title: string | null) => void>();

function musicChanged() {
  const title = music?.playing ? music.title : null;
  musicListeners.forEach((f) => f(title));
}

/** Calls `f` with the music now playing (its name), or null, each time that changes. */
export function onMusic(f: (title: string | null) => void): () => void {
  musicListeners.add(f);
  return () => musicListeners.delete(f);
}

/** The Music Editor's song is playing: the game's music waits. */
const editorPlaying = () => !!music?.editor && !!music.playing;

/** Tells the Music Editor's player its song was stopped by something else. */
function taken(was: typeof music) {
  if (was?.editor) was.taken?.();
}

/** Plays music, once or in a loop, unless it's what's playing already (then only its volume changes). */
export async function playMusic(path: string, looping: boolean, percent = 100) {
  // The player's own song goes on: a trigger's music doesn't cut it.
  if (editorPlaying()) return;
  if (music && music.path === path && music.looping === looping) {
    music.percent = percent;
    if (music.playing) music.playing.gain.gain.value = percent / 100;
    return;
  }
  stopMusic();
  const turn = ++musicTurn;
  music = { path, title: mud.assetTitle(path), looping, percent, playing: null };
  let buffer: AudioBuffer;
  try {
    buffer = await load(path, looping);
  } catch (e) {
    // Forgotten, so the next trigger tries again (after a SoundFont is added, say).
    if (turn === musicTurn) music = null;
    throw e;
  }
  if (turn !== musicTurn || !music) return;
  const playing = start(buffer, "bgm", music.percent, looping);
  caption(`♪ Music: ${mud.assetTitle(path)}`);
  music.playing = playing;
  playing.source.onended = () => {
    if (music?.playing === playing) {
      music = null;
      musicChanged();
    }
  };
  musicChanged();
}

export function stopMusic() {
  musicTurn++;
  const before = music;
  const was = music?.playing;
  music = null;
  if (was) {
    was.source.onended = null;
    was.source.stop();
    musicChanged();
  }
  taken(before);
}

/** Fades the music out and forgets it, the way a loop goes (`FADE`). */
function fadeMusic() {
  musicTurn++;
  const before = music;
  const was = music?.playing;
  music = null;
  if (was) {
    was.source.onended = null;
    fadeOut(was);
    musicChanged();
  }
  taken(before);
}

/**
 * The place's music (src-tauri/src/ambient.rs: a BGM hooked to the room
 * or its area), as Rust says it should be: the same piece plays on (its
 * volume follows); another fades in as whatever played fades out; none
 * fades the place's music out (a room or area left, a fight begun), but
 * not music a trigger set off. Starting again after a fight, it starts
 * from the top.
 */
export async function setPlaceMusic(want: mud.PlaceMusic | null) {
  if (editorPlaying()) {
    placeWaiting = want;
    return;
  }
  placeWaiting = null;
  if (want && music && music.path === want.path) {
    music.percent = want.volume;
    music.place = want;
    if (music.playing) {
      const gain = music.playing.gain.gain;
      const t = audio().context.currentTime;
      gain.cancelScheduledValues(t);
      gain.setValueAtTime(gain.value, t);
      gain.linearRampToValueAtTime(want.volume / 100, t + FADE);
    }
    return;
  }
  if (!want) {
    if (music?.place) fadeMusic();
    return;
  }
  fadeMusic();
  const turn = ++musicTurn;
  music = { path: want.path, title: mud.assetTitle(want.path), looping: want.looping, percent: want.volume, playing: null, place: want };
  let buffer: AudioBuffer;
  try {
    buffer = await load(want.path, want.looping);
  } catch (e) {
    if (turn === musicTurn) music = null;
    throw e;
  }
  const now = music;
  if (turn !== musicTurn || !now) return;
  const playing = start(buffer, "bgm", 0, want.looping);
  caption(`♪ Music for this ${want.place}: ${mud.assetTitle(want.path)}`);
  const t = audio().context.currentTime;
  playing.gain.gain.setValueAtTime(0, t);
  playing.gain.gain.linearRampToValueAtTime(now.percent / 100, t + FADE);
  now.playing = playing;
  playing.source.onended = () => {
    if (music?.playing === playing) {
      music = null;
      musicChanged();
    }
  };
  musicChanged();
}

// ---- Played from a buffer: the Music Editor's song (lib/musicEditor.ts) ----

/**
 * Plays the Music Editor's song as the music, on the BGM bus from
 * `offset` seconds, once or over and over: what was playing fades out,
 * and the game's music waits till the song stops (`releaseEditorSong`).
 * `onTaken`: something else stopped it (the Mixer's Stop All, a logout).
 */
export function playEditorSong(buffer: AudioBuffer, looping: boolean, offset: number, title: string, onTaken: () => void): { source: AudioBufferSourceNode; gain: GainNode } {
  const { context, buses } = audio();
  if (music?.place) placeWaiting = music.place;
  fadeMusic();
  const source = context.createBufferSource();
  source.buffer = buffer;
  source.loop = looping;
  const gain = context.createGain();
  source.connect(gain).connect(buses.bgm);
  source.start(0, Math.max(0, Math.min(offset, buffer.duration)));
  const playing = { source, gain };
  music = { path: "", title, looping, percent: 100, playing, editor: true, taken: onTaken };
  caption(`♪ Music: ${title}`);
  musicChanged();
  return playing;
}

/** The Music Editor's song stopped (paused, ended, stopped): the channel's free, and the place's music comes back. */
export function releaseEditorSong(source: AudioBufferSourceNode) {
  if (!music?.editor || music.playing?.source !== source) return;
  music = null;
  musicChanged();
  const want = placeWaiting;
  placeWaiting = null;
  if (want) setPlaceMusic(want).catch(() => {});
}

/** Decodes WAV bytes (rendered music) into a buffer to play. */
export const decode = (bytes: ArrayBuffer) => audio().context.decodeAudioData(bytes);

/** The audio clock, seconds. */
export const audioTime = () => audio().context.currentTime;

// ---- Ambience: one loop each on BGN and BGW ----

export type AmbientBus = "bgn" | "bgw";
/** How long a loop takes to fade in or out, in seconds. */
const FADE = 1.5;

const loops: Record<AmbientBus, { path: string; percent: number; playing: { source: AudioBufferSourceNode; gain: GainNode } | null } | null> = { bgn: null, bgw: null };
/** Bumped by every request on a bus, so one that finishes loading late doesn't start. */
const loopTurn: Record<AmbientBus, number> = { bgn: 0, bgw: 0 };
const loopListeners = new Set<(playing: Record<AmbientBus, string | null>) => void>();

function loopsChanged() {
  const now = { bgn: loops.bgn?.playing ? loops.bgn.path : null, bgw: loops.bgw?.playing ? loops.bgw.path : null };
  loopListeners.forEach((f) => f(now));
}

/** Calls `f` with the loops now playing (asset paths, or null) each time that changes. */
export function onLoops(f: (playing: Record<AmbientBus, string | null>) => void): () => void {
  loopListeners.add(f);
  return () => loopListeners.delete(f);
}

/** Fades a playing loop out and stops it. */
function fadeOut(playing: { source: AudioBufferSourceNode; gain: GainNode }) {
  const { context } = audio();
  const now = context.currentTime;
  playing.gain.gain.cancelScheduledValues(now);
  playing.gain.gain.setValueAtTime(playing.gain.gain.value, now);
  playing.gain.gain.linearRampToValueAtTime(0, now + FADE);
  playing.source.stop(now + FADE);
}

/**
 * Makes a bus loop `loop` (or nothing): the one playing goes on if it's
 * the same asset, at the new volume; otherwise it fades out and the new
 * one fades in.
 */
export async function setLoop(bus: AmbientBus, loop: mud.Loop | null) {
  const was = loops[bus];
  if (loop && was && was.path === loop.path) {
    was.percent = loop.volume;
    if (was.playing) {
      const gain = was.playing.gain.gain;
      const t = audio().context.currentTime;
      gain.cancelScheduledValues(t);
      gain.setValueAtTime(gain.value, t);
      gain.linearRampToValueAtTime(loop.volume / 100, t + FADE);
    }
    return;
  }
  const turn = ++loopTurn[bus];
  if (was?.playing) fadeOut(was.playing);
  loops[bus] = loop ? { path: loop.path, percent: loop.volume, playing: null } : null;
  if (was?.playing) loopsChanged();
  if (!loop) return;
  let buffer: AudioBuffer;
  try {
    buffer = await load(loop.path, true);
  } catch (e) {
    if (turn === loopTurn[bus]) loops[bus] = null;
    throw e;
  }
  const now = loops[bus];
  if (turn !== loopTurn[bus] || !now) return;
  const playing = start(buffer, bus, 0, true);
  caption(`♪ ${bus === "bgn" ? "The place" : "The sky"}: ${mud.assetTitle(loop.path)}`);
  const t = audio().context.currentTime;
  playing.gain.gain.setValueAtTime(0, t);
  playing.gain.gain.linearRampToValueAtTime(now.percent / 100, t + FADE);
  now.playing = playing;
  loopsChanged();
}

/**
 * Stops both loops, fading them out: a loop never cuts off. They start
 * again when what should loop next changes (the `ambient` event only
 * comes on a change).
 */
function stopLoops() {
  for (const bus of ["bgn", "bgw"] as const) {
    loopTurn[bus]++;
    const was = loops[bus]?.playing;
    loops[bus] = null;
    if (was) fadeOut(was);
  }
  loopsChanged();
}

/** Stops the music, the background noise and weather, and every sound effect still playing. */
export function stopAll() {
  placeWaiting = null;
  stopMusic();
  stopPreview();
  stopLoops();
  effects.forEach((e) => e.source.stop());
  effects.clear();
}

/**
 * Fades out everything playing, over the ambience's fade, then stops it:
 * for leaving a character while still connected (a logout or a switch).
 * What's set off after it starts afresh; the ambience comes again from
 * Rust once there's a room to have it.
 */
export function fadeAll() {
  placeWaiting = null;
  fadeMusic();
  for (const bus of ["bgn", "bgw"] as const) {
    loopTurn[bus]++;
    const loop = loops[bus]?.playing;
    loops[bus] = null;
    if (loop) fadeOut(loop);
  }
  loopsChanged();
  effects.forEach(fadeOut);
  effects.clear();
}

// ---- Trying an asset: one at a time ----

let preview: { path: string; playing: { source: AudioBufferSourceNode; gain: GainNode } | null } | null = null;
let previewTurn = 0;
const previewListeners = new Set<(path: string | null) => void>();

function previewChanged() {
  const path = preview?.path ?? null;
  previewListeners.forEach((f) => f(path));
}

/** Calls `f` with the asset being tried (its path, from the moment it's asked for), or null, each time that changes. */
export function onPreview(f: (path: string | null) => void): () => void {
  previewListeners.add(f);
  return () => previewListeners.delete(f);
}

/** The asset being tried now, or null. */
export const previewing = () => preview?.path ?? null;

/**
 * Plays an asset once to try it, stopping the one tried before: a sound on
 * the SFX bus, music (or a SoundFont's sample) on the BGM bus. `loop`
 * plays it over and over (made as an exact loop) until `stopPreview`.
 */
export async function playPreview(path: string, kind: mud.AssetKind, loop = false) {
  stopPreview();
  const turn = ++previewTurn;
  preview = { path, playing: null };
  previewChanged();
  let buffer: AudioBuffer;
  try {
    buffer = await load(path, loop);
  } catch (e) {
    if (turn === previewTurn) {
      preview = null;
      previewChanged();
    }
    throw e;
  }
  if (turn !== previewTurn || !preview) return;
  const playing = start(buffer, kind === "sfx" ? "sfx" : "bgm", 100, loop);
  caption(`♪ Trying ${kind === "soundfont" ? `the SoundFont ${mud.assetTitle(path)}` : mud.assetTitle(path)}`);
  preview.playing = playing;
  playing.source.onended = () => {
    if (preview?.playing === playing) {
      preview = null;
      previewChanged();
    }
  };
}

/**
 * Plays music made elsewhere (the Music Editor's bars, as WAV bytes) the
 * way an asset is tried: one thing at a time, `id` standing for it in
 * `previewing`, over and over when `loop`, until `stopPreview`.
 */
export async function playPreviewBytes(id: string, bytes: ArrayBuffer, title: string, loop: boolean) {
  stopPreview();
  const turn = ++previewTurn;
  preview = { path: id, playing: null };
  previewChanged();
  let buffer: AudioBuffer;
  try {
    buffer = await audio().context.decodeAudioData(bytes);
  } catch (e) {
    if (turn === previewTurn) {
      preview = null;
      previewChanged();
    }
    throw e;
  }
  if (turn !== previewTurn || !preview) return;
  const playing = start(buffer, "bgm", 100, loop);
  caption(`♪ ${title}`);
  preview.playing = playing;
  playing.source.onended = () => {
    if (preview?.playing === playing) {
      preview = null;
      previewChanged();
    }
  };
}

/** Stops the asset being tried, if any. */
export function stopPreview() {
  previewTurn++;
  const was = preview;
  preview = null;
  if (was?.playing) {
    was.playing.source.onended = null;
    was.playing.source.stop();
  }
  if (was) previewChanged();
}

/** Forgets what was decoded, after the Assets folder changed (a SoundFont added, say). */
export function forgetDecoded() {
  decoded.clear();
}

// ---- Pictures ----

/** A trigger's picture, ready to show: an image's URL, or ANSI art drawn in characters. */
export type Shown = { url: string } | { art: mud.AnsiArt };

const shown = new Map<string, Promise<Shown>>();

/** A picture, read once and kept; ANSI art once for each of the game's colors (lib/palette.ts), shown in the colors now. */
export function picture(path: string): Promise<Shown> {
  const key = mud.isAnsiArt(path) ? `${path}\n${palette()}` : path;
  let ready = shown.get(key);
  if (!ready) {
    ready = mud.isAnsiArt(path)
      ? mud.assetAnsi(path).then((art) => ({ art }))
      : mud.assetPicture(path).then((bytes) => ({ url: URL.createObjectURL(new Blob([bytes], { type: path.startsWith("png/") ? "image/png" : "image/jpeg" })) }));
    ready.catch(() => shown.delete(key));
    shown.set(key, ready);
  }
  return ready;
}

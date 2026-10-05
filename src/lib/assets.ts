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
 * Leaving the game stops everything at once (`stopAll`); leaving a
 * character for another, or for the account menu, fades it out (`fadeAll`).
 * One more bus, CUE, carries Immersive mode's own sounds, made here
 * rather than played from the Assets folder (lib/earcons.ts).
 * Pictures are images, or ANSI art that Rust has drawn into styled rows
 * of characters.
 * The Hooks dialog's Assets tab tries one asset at a time (`playPreview`,
 * `stopPreview`): a sound on the SFX bus, music and a SoundFont's sample
 * tune on the BGM bus, over whatever the game has set off.
 */
import { caption } from "./captions";
import * as mud from "./mud";

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

function audio(): { context: AudioContext; buses: Record<Bus, GainNode> } {
  if (!context || !buses) {
    context = new AudioContext();
    const bus = (b: Bus) => {
      const gain = context!.createGain();
      gain.gain.value = volumes[b] / 100;
      gain.connect(context!.destination);
      return gain;
    };
    buses = { sfx: bus("sfx"), bgm: bus("bgm"), bgn: bus("bgn"), bgw: bus("bgw"), cue: bus("cue") };
  }
  // WebKit starts a context suspended until the page has been used.
  if (context.state === "suspended") void context.resume();
  return { context, buses };
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

/** Lets sound start: call once from a click or a key, before any trigger goes off. */
export function wakeAudio() {
  audio();
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

/** A buffer played through its own volume, into a bus. */
function start(buffer: AudioBuffer, bus: Bus, percent: number, looping: boolean) {
  const { context, buses } = audio();
  const source = context.createBufferSource();
  source.buffer = buffer;
  source.loop = looping;
  const gain = context.createGain();
  gain.gain.value = percent / 100;
  source.connect(gain).connect(buses[bus]);
  source.start();
  return { source, gain };
}

const effects = new Set<{ source: AudioBufferSourceNode; gain: GainNode }>();

/** Plays a sound effect once, at `percent` of the SFX volume. */
export async function playEffect(path: string, percent = 100) {
  const buffer = await load(path, false);
  const playing = start(buffer, "sfx", percent, false);
  caption(`♪ ${mud.assetTitle(path)}`);
  effects.add(playing);
  playing.source.onended = () => effects.delete(playing);
}

// ---- Music: one piece at a time ----

let music: { path: string; looping: boolean; percent: number; playing: { source: AudioBufferSourceNode; gain: GainNode } | null } | null = null;
/** Bumped by every music request, so one that finishes loading late doesn't start. */
let musicTurn = 0;
const musicListeners = new Set<(path: string | null) => void>();

function musicChanged() {
  const path = music?.playing ? music.path : null;
  musicListeners.forEach((f) => f(path));
}

/** Calls `f` with the music now playing (its asset path), or null, each time that changes. */
export function onMusic(f: (path: string | null) => void): () => void {
  musicListeners.add(f);
  return () => musicListeners.delete(f);
}

/** Plays music, once or in a loop, unless it's what's playing already (then only its volume changes). */
export async function playMusic(path: string, looping: boolean, percent = 100) {
  if (music && music.path === path && music.looping === looping) {
    music.percent = percent;
    if (music.playing) music.playing.gain.gain.value = percent / 100;
    return;
  }
  stopMusic();
  const turn = ++musicTurn;
  music = { path, looping, percent, playing: null };
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
  const was = music?.playing;
  music = null;
  if (was) {
    was.source.onended = null;
    was.source.stop();
    musicChanged();
  }
}

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
    if (was.playing) was.playing.gain.gain.setTargetAtTime(loop.volume / 100, audio().context.currentTime, 0.2);
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

/** Stops both loops at once. They start again when what should loop next changes (the `ambient` event only comes on a change). */
function stopLoops() {
  for (const bus of ["bgn", "bgw"] as const) {
    loopTurn[bus]++;
    const was = loops[bus]?.playing;
    loops[bus] = null;
    was?.source.stop();
  }
  loopsChanged();
}

/** Stops the music, the background noise and weather, and every sound effect still playing. */
export function stopAll() {
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
  musicTurn++;
  const was = music?.playing;
  music = null;
  if (was) {
    was.source.onended = null;
    fadeOut(was);
    musicChanged();
  }
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

/** A picture, read once and kept. */
export function picture(path: string): Promise<Shown> {
  let ready = shown.get(path);
  if (!ready) {
    ready = mud.isAnsiArt(path)
      ? mud.assetAnsi(path).then((art) => ({ art }))
      : mud.assetPicture(path).then((bytes) => ({ url: URL.createObjectURL(new Blob([bytes], { type: path.startsWith("png/") ? "image/png" : "image/jpeg" })) }));
    ready.catch(() => shown.delete(path));
    shown.set(path, ready);
  }
  return ready;
}

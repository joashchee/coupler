/**
 * **What the Music Editor has loaded**, kept here rather than in its
 * dialog, so Coupler's own commands (`#play`, `#save`, `#write`,
 * `#hook`: lib/commands.ts) work on it with the dialog closed:
 * the song, the file it came from, whether it has changes unsaved, its
 * title when it has no file (music read from a journal), and the
 * SoundFont it plays through.
 *
 * And **its player**: the whole song rendered once (again after an
 * edit), played from where it was paused, rewound, looped or stopped,
 * as the music on the BGM channel (lib/assets.ts `playEditorSong`): what
 * played fades out, the game's music waits till it stops, and the sound
 * layers call it "Music Editor - <its name>". One thing at a time with
 * the editor's own Play Bar and Play Song (lib/assets.ts's preview):
 * either stops the other.
 */
import { useSyncExternalStore } from "react";
import * as sound from "./assets";
import * as mud from "./mud";
import * as music from "./music";

export interface Loaded {
  song: music.Song | null;
  /** The MIDI asset it came from, or null (new, or read from a journal). */
  path: string | null;
  dirty: boolean;
  /** Its name with no file: what Save as… offers. */
  title: string;
  /** The SoundFont or DLS asset it plays through; "" is Neumetik. */
  font: string;
}

let loaded: Loaded = { song: null, path: null, dirty: false, title: "", font: "" };
const listeners = new Set<() => void>();

export const get = () => loaded;

export function set(change: Partial<Loaded>) {
  loaded = { ...loaded, ...change };
  if ("song" in change || "font" in change) forgetRender();
  listeners.forEach((f) => f());
}

export function subscribe(f: () => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

/** The loaded song, for a component, kept up to date. */
export const useLoaded = () => useSyncExternalStore(subscribe, get);

/** The loaded song's name: its file's, or its title. */
export const titleOf = (l: Loaded = loaded) => (l.path ? mud.assetTitle(l.path) : l.title || "New music");

// ---- The player ----

export type PlayerState = "stopped" | "playing" | "paused";

interface Player {
  state: PlayerState;
  looping: boolean;
  /** Seconds into the song where a pause left it. */
  at: number;
}

let player: Player = { state: "stopped", looping: true, at: 0 };
/** The whole song rendered, for the song it was made from. */
let rendered: { song: music.Song; font: string; buffer: AudioBuffer } | null = null;
let playing: { source: AudioBufferSourceNode; startedAt: number; from: number } | null = null;
const playerListeners = new Set<() => void>();

function changed(next: Partial<Player>) {
  player = { ...player, ...next };
  playerListeners.forEach((f) => f());
}

export const playerNow = () => player;
export const onPlayer = (f: () => void): (() => void) => {
  playerListeners.add(f);
  return () => playerListeners.delete(f);
};
export const usePlayer = () => useSyncExternalStore(onPlayer, playerNow);

function forgetRender() {
  rendered = null;
}

/** Where the song is, seconds. */
function position(): number {
  if (!playing || !rendered) return player.at;
  const length = rendered.buffer.duration;
  const t = playing.from + sound.audioTime() - playing.startedAt;
  return player.looping && length > 0 ? t % length : Math.min(t, length);
}

function halt() {
  if (!playing) return;
  playing.source.onended = null;
  playing.source.stop();
  sound.releaseEditorSong(playing.source);
  playing = null;
}

/** What the sound layers call the song playing: "Music Editor", and its name if it has one. */
function layerTitle(): string {
  const name = loaded.path ? mud.assetTitle(loaded.path) : loaded.title;
  return name ? `Music Editor - ${name}` : "Music Editor";
}

/** The song rendered whole, once a version. */
async function buffer(): Promise<AudioBuffer> {
  const { song, font } = loaded;
  if (!song) throw new Error("The Music Editor has no music loaded: open it (gear menu, Music Editor…) or read music from a journal.");
  if (rendered && rendered.song === song && rendered.font === font) return rendered.buffer;
  const bytes = await music.render(font || null, song, 0, song.bars, true);
  const decoded = await sound.decode(bytes);
  // The song may have changed while it rendered: kept only if it's still the one.
  if (loaded.song === song && loaded.font === font) rendered = { song, font, buffer: decoded };
  return decoded;
}

/** Plays the song from where it was (the start, or a pause). Resolves once it's sounding. */
export async function play(): Promise<void> {
  const b = await buffer();
  sound.stopPreview();
  halt();
  const from = player.state === "stopped" || player.at >= b.duration ? 0 : player.at;
  let now: { source: AudioBufferSourceNode; startedAt: number; from: number } | null = null;
  // Stopped by something else (the Mixer's Stop All, a logout): paused where it was.
  const onTaken = () => {
    if (!now || playing !== now) return;
    const at = position();
    playing = null;
    changed({ state: "paused", at });
  };
  const handle = sound.playEditorSong(b, player.looping, from, layerTitle(), onTaken);
  now = { source: handle.source, startedAt: sound.audioTime(), from };
  playing = now;
  handle.source.onended = () => {
    if (playing === now) {
      sound.releaseEditorSong(handle.source);
      playing = null;
      changed({ state: "stopped", at: 0 });
    }
  };
  changed({ state: "playing" });
}

/** Pauses where it is; resumes with `play`. */
export function pause() {
  if (player.state !== "playing") return;
  const at = position();
  halt();
  changed({ state: "paused", at });
}

/** Back to the start: playing on from there if it was playing. */
export async function rewind(): Promise<void> {
  if (player.state === "playing") {
    changed({ state: "stopped", at: 0 });
    await play();
  } else {
    changed({ at: 0, state: player.state === "paused" ? "paused" : "stopped" });
  }
}

/** Stops, back at the start. */
export function stop() {
  halt();
  changed({ state: "stopped", at: 0 });
}

/** Loops the song, or plays it once to its end; taking effect at once. */
export function setLooping(on: boolean) {
  if (playing) playing.source.loop = on;
  changed({ looping: on });
}

/** The song stops when it's taken away, or changed while not playing. */
subscribe(() => {
  if (!loaded.song && player.state !== "stopped") stop();
});

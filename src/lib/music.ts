/**
 * The Music Editor's song (src-tauri/src/music.rs reads and writes the
 * MIDI file) and its edits, pure: the grid (a step is a sixteenth note),
 * notes added, removed, lengthened and made louder, bars copied, pasted
 * and cleared, parts added and removed, and the words for a key, a drum
 * and a place in the bar. components/MusicEditorDialog.tsx is the
 * screen.
 */
import { invoke } from "@tauri-apps/api/core";
import type { Asset } from "./mud";

export interface Note {
  at: number;
  length: number;
  key: number;
  velocity: number;
}
export interface Kept {
  at: number;
  bytes: number[];
}
export interface Part {
  name: string;
  /** 0 to 15; 9 is the drums. */
  channel: number;
  bank: number;
  program: number;
  notes: Note[];
  kept: Kept[];
}
export interface Song {
  ppq: number;
  bpm: number;
  beats: number;
  unit: number;
  bars: number;
  parts: Part[];
  kept: Kept[];
}
export interface Instruments {
  banks: { msb: number; names: string[] }[];
  kits: [number, string][];
}

export const DRUMS = 9;

export const open = (path: string) => invoke<Song>("music_open", { path });
export const fresh = () => invoke<Song>("music_new");
export const save = (path: string, song: Song) => invoke<void>("music_save", { path, song });
export const saveAs = (name: string, song: Song) => invoke<Asset>("music_save_as", { name, song });
/** The bars `from` to `to` (from 0, `to` not included) as WAV bytes, through Neumetik or `font` (a SoundFont asset). */
export const render = (font: string | null, song: Song, from: number, to: number, looping: boolean) =>
  invoke<ArrayBuffer>("music_render", { font, song, from, to, looping });
export const instruments = () => invoke<Instruments>("music_instruments");

/** The banks' names, by Bank Select MSB. */
export const BANK_NAMES: Record<number, string> = { 0: "General MIDI", 80: "Classic synths: the 1980s", 81: "Classic synths: the 1990s", 82: "Classic synths: the 2000s to now" };

// ---- The grid ----

export const barTicks = (s: Song) => Math.max(1, Math.floor((s.ppq * 4 * Math.max(1, s.beats)) / Math.max(1, s.unit)));
/** A step: a sixteenth note. */
export const stepTicks = (s: Song) => Math.max(1, Math.floor(s.ppq / 4));
/** The steps a bar shows. */
export const stepsPerBar = (s: Song) => Math.max(1, Math.min(64, Math.floor(barTicks(s) / stepTicks(s))));
export const endTicks = (s: Song) => s.bars * barTicks(s);
/** The steps a beat (the meter's unit). */
export const stepsPerBeat = (s: Song) => Math.max(1, Math.round(16 / Math.max(1, s.unit)));

const NAMES = ["C", "C sharp", "D", "D sharp", "E", "F", "F sharp", "G", "G sharp", "A", "A sharp", "B"];
const SHORT = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/** A key as it's written: C4 is middle C (60). */
export const keyShort = (key: number) => `${SHORT[key % 12]}${Math.floor(key / 12) - 1}`;
/** A key as it's said. */
export const keyWords = (key: number) => `${NAMES[key % 12]} ${Math.floor(key / 12) - 1}`;
export const isBlack = (key: number) => [1, 3, 6, 8, 10].includes(key % 12);

/** GM's and GS's drum map, short. */
const DRUM_NAMES: Record<number, string> = {
  27: "High Q", 28: "Slap", 29: "Scratch Push", 30: "Scratch Pull", 31: "Sticks", 32: "Square Click", 33: "Metronome", 34: "Metronome Bell",
  35: "Kick 2", 36: "Kick", 37: "Side Stick", 38: "Snare", 39: "Clap", 40: "Snare 2", 41: "Low Floor Tom", 42: "Closed Hat",
  43: "High Floor Tom", 44: "Pedal Hat", 45: "Low Tom", 46: "Open Hat", 47: "Low Mid Tom", 48: "High Mid Tom", 49: "Crash", 50: "High Tom",
  51: "Ride", 52: "China", 53: "Ride Bell", 54: "Tambourine", 55: "Splash", 56: "Cowbell", 57: "Crash 2", 58: "Vibraslap",
  59: "Ride 2", 60: "High Bongo", 61: "Low Bongo", 62: "Mute Conga", 63: "Open Conga", 64: "Low Conga", 65: "High Timbale", 66: "Low Timbale",
  67: "High Agogo", 68: "Low Agogo", 69: "Cabasa", 70: "Maracas", 71: "Short Whistle", 72: "Long Whistle", 73: "Short Guiro", 74: "Long Guiro",
  75: "Claves", 76: "High Wood Block", 77: "Low Wood Block", 78: "Mute Cuica", 79: "Open Cuica", 80: "Mute Triangle", 81: "Open Triangle",
  82: "Shaker", 83: "Jingle Bell", 84: "Bell Tree", 85: "Castanets", 86: "Mute Surdo", 87: "Open Surdo",
};

/** A row's name: the drum on the drums, else the key. */
export const rowName = (part: Part, key: number) => (part.channel === DRUMS ? (DRUM_NAMES[key] ?? keyShort(key)) : keyShort(key));
export const rowWords = (part: Part, key: number) => (part.channel === DRUMS ? (DRUM_NAMES[key] ?? keyWords(key)) : keyWords(key));

/** Where a step is, in words: "beat 2", or "beat 2, step 3". */
export function stepWords(s: Song, step: number): string {
  const per = stepsPerBeat(s);
  const beat = Math.floor(step / per) + 1;
  const within = (step % per) + 1;
  return within === 1 ? `beat ${beat}` : `beat ${beat}, step ${within}`;
}

/** The instrument's name for a part. */
export function instrumentName(part: Part, inst: Instruments | null): string {
  if (!inst) return "";
  if (part.channel === DRUMS) return [...inst.kits].reverse().find(([p]) => p <= part.program)?.[1] ?? "Standard";
  const bank = inst.banks.find((b) => b.msb === part.bank) ?? inst.banks[0];
  return bank?.names[part.program] ?? inst.banks[0]?.names[part.program] ?? `Program ${part.program + 1}`;
}

// ---- Looking ----

/** The notes of a part starting in the ticks from `from` to `to`. */
export const notesIn = (part: Part, from: number, to: number) => part.notes.filter((n) => n.at >= from && n.at < to);

/** How a cell looks: a note starting in it, one going on through it, or nothing. */
export function cellOf(part: Part, key: number, from: number, to: number): { kind: "start" | "held" | "empty"; note?: Note } {
  const starts = part.notes.find((n) => n.key === key && n.at >= from && n.at < to);
  if (starts) return { kind: "start", note: starts };
  const held = part.notes.find((n) => n.key === key && n.at < from && n.at + n.length > from);
  return held ? { kind: "held", note: held } : { kind: "empty" };
}

/** The key a part's notes centre on, for where its piano roll opens. */
export function middleKey(part: Part): number {
  if (part.notes.length === 0) return part.channel === DRUMS ? 42 : 60;
  const keys = part.notes.map((n) => n.key).sort((a, b) => a - b);
  return keys[Math.floor(keys.length / 2)];
}

// ---- Editing (each returns a new song) ----

function withPart(s: Song, index: number, edit: (p: Part) => Part): Song {
  return { ...s, parts: s.parts.map((p, i) => (i === index ? edit(p) : p)) };
}

const sorted = (notes: Note[]) => [...notes].sort((a, b) => a.at - b.at || a.key - b.key);

/**
 * The cell at `key`, from `from` a step long: a note starting there or
 * going on through it is taken away, else a note a step long is put
 * there at `velocity`.
 */
export function toggle(s: Song, index: number, key: number, from: number, velocity: number): Song {
  const to = from + stepTicks(s);
  return withPart(s, index, (p) => {
    const hit = p.notes.find((n) => n.key === key && ((n.at >= from && n.at < to) || (n.at < from && n.at + n.length > from)));
    if (hit) return { ...p, notes: p.notes.filter((n) => n !== hit) };
    return { ...p, notes: sorted([...p.notes, { at: from, length: stepTicks(s), key, velocity }]) };
  });
}

/** Changes the note at a cell (starting or going on there) with `change`; the song as it was if there's none. */
export function changeNote(s: Song, index: number, key: number, from: number, change: (n: Note) => Note): Song {
  const to = from + stepTicks(s);
  const part = s.parts[index];
  const hit = part?.notes.find((n) => n.key === key && ((n.at >= from && n.at < to) || (n.at < from && n.at + n.length > from)));
  if (!hit) return s;
  const changed = change(hit);
  return withPart(s, index, (p) => ({ ...p, notes: sorted(p.notes.map((n) => (n === hit ? changed : n))) }));
}

/** A note `steps` longer (or shorter), a step at least, to the song's end at most. */
export const lengthen = (s: Song, index: number, key: number, from: number, steps: number) =>
  changeNote(s, index, key, from, (n) => ({ ...n, length: Math.max(stepTicks(s), Math.min(endTicks(s) - n.at, n.length + steps * stepTicks(s))) }));

/** A note made to end at the end of the step from `until`. */
export const lengthenTo = (s: Song, index: number, key: number, from: number, until: number) =>
  changeNote(s, index, key, from, (n) => ({ ...n, length: Math.max(stepTicks(s), Math.min(endTicks(s), until + stepTicks(s)) - n.at) }));

/** A note louder (or softer) by `by`, 1 to 127. */
export const louder = (s: Song, index: number, key: number, from: number, by: number) =>
  changeNote(s, index, key, from, (n) => ({ ...n, velocity: Math.max(1, Math.min(127, n.velocity + by)) }));

/** A bar's notes, as from its start, to paste. */
export function copyBar(s: Song, index: number, bar: number): Note[] {
  const from = bar * barTicks(s);
  return notesIn(s.parts[index], from, from + barTicks(s)).map((n) => ({ ...n, at: n.at - from }));
}

/** The bar's notes taken away. */
export function clearBar(s: Song, index: number, bar: number): Song {
  const from = bar * barTicks(s);
  const to = from + barTicks(s);
  return withPart(s, index, (p) => ({ ...p, notes: p.notes.filter((n) => n.at < from || n.at >= to) }));
}

/** The bar's notes replaced by those copied. */
export function pasteBar(s: Song, index: number, bar: number, notes: Note[]): Song {
  const cleared = clearBar(s, index, bar);
  const from = bar * barTicks(s);
  const end = endTicks(s);
  const pasted = notes
    .filter((n) => n.at < barTicks(s))
    .map((n) => ({ ...n, at: from + n.at }))
    .filter((n) => n.at < end)
    .map((n) => ({ ...n, length: Math.min(n.length, end - n.at) }));
  return withPart(cleared, index, (p) => ({ ...p, notes: sorted([...p.notes, ...pasted]) }));
}

/** The song `bars` long: what's past the end goes, notes running over it are cut. */
export function setBars(s: Song, bars: number): Song {
  const n = Math.max(1, Math.min(999, Math.round(bars)));
  const song = { ...s, bars: n };
  const end = endTicks(song);
  return {
    ...song,
    parts: song.parts.map((p) => ({
      ...p,
      notes: p.notes.filter((x) => x.at < end).map((x) => ({ ...x, length: Math.min(x.length, end - x.at) })),
      kept: p.kept.filter((k) => k.at < end),
    })),
  };
}

/** A new part on a free channel (the drums' own, for drums), or null when there's none free. */
export function addPart(s: Song, drums: boolean): Song | null {
  const used = new Set(s.parts.map((p) => p.channel));
  const channel = drums ? (used.has(DRUMS) ? null : DRUMS) : ([...Array(16).keys()].find((c) => c !== DRUMS && !used.has(c)) ?? null);
  if (channel === null) return null;
  const part: Part = { name: drums ? "Drums" : `Part ${s.parts.length + 1}`, channel, bank: 0, program: 0, notes: [], kept: [] };
  return { ...s, parts: [...s.parts, part] };
}

export const removePart = (s: Song, index: number): Song => ({ ...s, parts: s.parts.filter((_, i) => i !== index) });

export const setPart = (s: Song, index: number, change: Partial<Part>): Song => withPart(s, index, (p) => ({ ...p, ...change }));

/** How full a bar is, as two characters: none, a few, some, many. */
export function density(count: number): string {
  if (count === 0) return "··";
  if (count < 4) return "░░";
  if (count < 8) return "▒▒";
  return "▓▓";
}

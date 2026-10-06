/**
 * Priority Audio: the cues and speech the player can't wait for. Anything
 * on this list plays the moment it comes, never behind the speech queue
 * (lib/voice.ts): if a line is being said, the queue pauses, the
 * priority bell rings (a cue of its own, `priorityBell` in
 * lib/earcons.ts, its sound chosen here and the rest in Cues), the
 * priority audio plays, and the queue picks up where it left off. With
 * nothing being said, it just plays, no bell.
 *
 * What can be on it:
 *
 * - **A cue** (lib/earcons.ts's `CUES`, but the heartbeat, which repeats
 *   too often to interrupt anything).
 * - **Talk** by kind: tells, the group, says in the room (lib/journal.ts).
 * - **Coupler's own speech** tied to the game: the time of day, a long
 *   look's hidden details (lib/immersive.ts).
 * - **A command's answer** (src-tauri/src/echo.rs, by its ID): what the
 *   narrator says for it (lib/echoes.ts).
 * - **A line of the game's** containing the player's words (any case):
 *   said as written, even inside a block of lines.
 *
 * Kept as `coupler.priority`: the list, and the bell's sound. Pure but
 * for localStorage, so it imports nothing at run time.
 */
import type { CueId } from "./earcons";

export type SpeechId = "tell" | "group" | "say" | "time" | "look";

export type PriorityEntry =
  | { kind: "cue"; id: CueId }
  | { kind: "speech"; id: SpeechId }
  | { kind: "answer"; id: string }
  | { kind: "line"; text: string };

export const SPEECHES: { id: SpeechId; name: string; summary: string }[] = [
  { id: "tell", name: "A tell", summary: "A tell to you, said as it comes." },
  { id: "group", name: "The group", summary: "Your group's talk, said as it comes." },
  { id: "say", name: "A say", summary: "Someone speaking in the room, said as it comes." },
  { id: "time", name: "The time of day", summary: "Dawn, dusk and night, and what TIME says." },
  { id: "look", name: "A long look", summary: "The room and its hidden details, after LL." },
];

export type BellSound = "bell" | "chime" | "dingDong" | "ping";

export const BELLS: { id: BellSound; name: string }[] = [
  { id: "bell", name: "A bell" },
  { id: "chime", name: "A chime, three notes" },
  { id: "dingDong", name: "Ding-dong" },
  { id: "ping", name: "A short ping" },
];

/** Cues that can't be on the list: they repeat. */
export const NOT_PRIORITY: CueId[] = ["heartbeat", "priorityBell"];

interface Kept {
  entries: PriorityEntry[];
  bell: BellSound;
}

const KEY = "coupler.priority";

/** What a player starts with: tells, and a fight starting. */
export const DEFAULT_ENTRIES: PriorityEntry[] = [
  { kind: "speech", id: "tell" },
  { kind: "cue", id: "fightStarts" },
];

function valid(e: unknown): e is PriorityEntry {
  if (!e || typeof e !== "object") return false;
  const { kind, id, text } = e as Record<string, unknown>;
  if (kind === "line") return typeof text === "string" && text.trim() !== "";
  if (kind === "speech") return SPEECHES.some((s) => s.id === id);
  return (kind === "cue" || kind === "answer") && typeof id === "string" && id !== "";
}

function load(): Kept {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return { entries: DEFAULT_ENTRIES, bell: "bell" };
    const kept = JSON.parse(raw) as Partial<Kept>;
    return {
      entries: Array.isArray(kept.entries) ? kept.entries.filter(valid) : DEFAULT_ENTRIES,
      bell: BELLS.some((b) => b.id === kept.bell) ? (kept.bell as BellSound) : "bell",
    };
  } catch {
    return { entries: DEFAULT_ENTRIES, bell: "bell" };
  }
}

let kept = load();
const listeners = new Set<() => void>();

function store() {
  try {
    localStorage.setItem(KEY, JSON.stringify(kept));
  } catch {
    // Not kept past this run, then.
  }
  listeners.forEach((f) => f());
}

export function onPriorityChanged(f: () => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

export const entries = (): PriorityEntry[] => kept.entries;
export const bellSound = (): BellSound => kept.bell;

const same = (a: PriorityEntry, b: PriorityEntry) =>
  a.kind === b.kind && (a.kind === "line" ? a.text.trim().toLowerCase() === (b as { text: string }).text.trim().toLowerCase() : a.id === (b as { id: string }).id);

export const has = (e: PriorityEntry) => kept.entries.some((k) => same(k, e));
export const isCue = (id: CueId) => !NOT_PRIORITY.includes(id) && has({ kind: "cue", id });
export const isSpeech = (id: SpeechId) => has({ kind: "speech", id });
export const isAnswer = (id: string | null) => id !== null && has({ kind: "answer", id });

/** Whether a line of the game's has words on the list. */
export function matchesLine(text: string): boolean {
  const lower = text.toLowerCase();
  return kept.entries.some((e) => e.kind === "line" && lower.includes(e.text.trim().toLowerCase()));
}

/** Puts an entry on the list (once) or takes it off. */
export function setEntry(e: PriorityEntry, on: boolean) {
  const rest = kept.entries.filter((k) => !same(k, e));
  kept = { ...kept, entries: on ? [...rest, e.kind === "line" ? { kind: "line", text: e.text.trim() } : e] : rest };
  store();
}

export function setBellSound(bell: BellSound) {
  kept = { ...kept, bell };
  store();
}

/** The list back as Coupler starts it. */
export function resetPriority() {
  kept = { entries: DEFAULT_ENTRIES, bell: "bell" };
  try {
    localStorage.removeItem(KEY);
  } catch {
    // Nothing kept, then.
  }
  listeners.forEach((f) => f());
}

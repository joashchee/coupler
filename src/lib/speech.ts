/**
 * What the screen reader is given to read, by kind of line: the player
 * picks which kinds (the Speech dialog, `coupler.speech`). The game's
 * lines come with their kind from Rust (src-tauri/src/speech.rs); the
 * commands the player sent and Coupler's own messages are known here.
 *
 * The prompt is read only when it changes: the same prompt after every
 * command says nothing new, and the vitals are on a key (Cmd+Shift+V).
 * A finished prompt (the game ends it when it answers) is never read
 * again either.
 */
import type { LineKind } from "./mud";

export type SpokenKind = LineKind | "sent" | "note";

export const SPOKEN_KINDS: { id: SpokenKind; name: string }[] = [
  { id: "game", name: "The game's lines: rooms, things happening, the rest" },
  { id: "talk", name: "Talk: tells, the group, says and channels" },
  { id: "time", name: "The time of day: dawn, dusk, night and what TIME says" },
  { id: "combat", name: "Lines during a fight" },
  { id: "echo", name: "Commands' one-line answers: You sit down, You get a sword" },
  { id: "prompt", name: "The prompt, when it changes" },
  { id: "sent", name: "The commands you send" },
  { id: "note", name: "Coupler's own messages in the output" },
];

export type Spoken = Record<SpokenKind, boolean>;

const KEY = "coupler.speech";

/** Everything but your own commands, which the screen reader already read as you typed them. */
export const DEFAULT_SPOKEN: Spoken = { game: true, talk: true, time: true, combat: true, echo: true, prompt: true, sent: false, note: true };

export function loadSpoken(): Spoken {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Record<string, unknown>>;
    const spoken = { ...DEFAULT_SPOKEN };
    for (const { id } of SPOKEN_KINDS) if (typeof raw[id] === "boolean") spoken[id] = raw[id] as boolean;
    return spoken;
  } catch {
    return { ...DEFAULT_SPOKEN };
  }
}

export function saveSpoken(spoken: Spoken) {
  try {
    localStorage.setItem(KEY, JSON.stringify(spoken));
  } catch {
    // Not kept past this run, then.
  }
}

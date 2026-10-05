/**
 * The commands' one-line answers (src-tauri/src/echo.rs): in Immersive
 * they aren't written in the game output, and the narrator says each in
 * short ("You sit down and take a rest." is "Sitting."). The answers and
 * Coupler's own words for them come from Rust (`echo_list`); the player's
 * own words, and whether it happens at all, are kept here as
 * `coupler.echoes` (Workshop's gear, Narrator's Answers…,
 * components/EchoesDialog.tsx).
 *
 * An answer Coupler has no words for (a lone line after a one-line
 * command) is said as the game wrote it.
 */
import * as mud from "./mud";

const KEY = "coupler.echoes";

export interface EchoSettings {
  /** Immersive leaves the answers out and the narrator says them. */
  on: boolean;
  /** The player's own words, by answer: "" says nothing. */
  says: Record<string, string>;
}

const listeners = new Set<() => void>();
let settings: EchoSettings = load();

function load(): EchoSettings {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<EchoSettings>;
    const says: Record<string, string> = {};
    if (raw.says && typeof raw.says === "object") {
      for (const [id, words] of Object.entries(raw.says)) if (typeof words === "string") says[id] = words;
    }
    return { on: raw.on !== false, says };
  } catch {
    return { on: true, says: {} };
  }
}

function store() {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // Not kept past this run, then.
  }
  listeners.forEach((f) => f());
}

export function onEchoesChanged(f: () => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

export const echoesOn = () => settings.on;

export function setEchoesOn(on: boolean) {
  settings = { ...settings, on };
  store();
}

/** The player's own words for an answer, or undefined for Coupler's. */
export const ownWords = (id: string): string | undefined => settings.says[id];

/** Sets an answer's words; null goes back to Coupler's. */
export function setWords(id: string, words: string | null) {
  const says = { ...settings.says };
  if (words === null) delete says[id];
  else says[id] = words;
  settings = { ...settings, says };
  store();
}

export function resetAllWords() {
  settings = { ...settings, says: {} };
  store();
}

export const changedCount = () => Object.keys(settings.says).length;

/** `words` with `{1}`, `{2}`... filled in, as echo.rs's `fill`. */
export const fill = (words: string, values: string[]) => values.reduce((out, v, i) => out.split(`{${i + 1}}`).join(v), words);

/** What the narrator says for an answer found in the output: "" for nothing. */
export function spoken(e: mud.Echoed): string {
  const own = e.id === null ? undefined : settings.says[e.id];
  return own === undefined ? e.says : fill(own, e.values).trim();
}

/** Made-up details for an answer's `*`s, to show and hear it. */
export const EXAMPLES = ["a sword", "a chest", "Hassan"];

/** An answer's line with its `*`s shown as "…". */
export const shownLine = (line: string) => line.split("*").join("…");

let listing: Promise<mud.EchoListing> | null = null;

/** Both tables, asked once. */
export function echoListing(): Promise<mud.EchoListing> {
  listing ??= mud.echoList().catch((e: unknown) => {
    listing = null;
    throw e;
  });
  return listing;
}

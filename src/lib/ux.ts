/**
 * The three ways Coupler can look and sound:
 *
 * - **Terminal**: a plain text terminal, 80 characters wide and as many
 *   rows as the screen holds, the command line under it, one status row.
 *   Nothing else. For screen readers and anyone who wants the old way.
 * - **Immersive**: the game told in layers of sound, so as little as
 *   possible has to be read or said. Coupler speaks for itself (no
 *   screen reader needed) and only what can't be a sound: talk, the
 *   login, and the answers to its keys (lib/immersive.ts).
 * - **Workshop**: everything Coupler has, and every part of it the
 *   player's to arrange, show or hide, including Immersive's sound
 *   layers (components/WorkshopDialog.tsx).
 *
 * Kept as `coupler.ux`; Workshop until chosen otherwise, since it's the
 * screen Coupler always had.
 */
import { keys } from "./keys";
export type Ux = "terminal" | "immersive" | "workshop";

export const UXES: { id: Ux; name: string; key: string; summary: string }[] = [
  { id: "terminal", name: "Terminal", key: keys("Ctrl+Cmd+1"), summary: "A plain text terminal, 80 characters wide and as tall as the screen. Best with a screen reader." },
  { id: "immersive", name: "Immersive", key: keys("Ctrl+Cmd+2"), summary: "The game in layers of sound, with as little read aloud as possible. Coupler speaks for itself: no screen reader needed." },
  { id: "workshop", name: "Workshop", key: keys("Ctrl+Cmd+3"), summary: "Everything Coupler has, and every part of the screen yours to move, resize, show or hide." },
];

const UX_KEY = "coupler.ux";
/** Workshop's choices: which things show (only those changed from the default), and whether Immersive's cues and voice play there too. */
const SHOW_KEY = "coupler.workshop.show";
const LAYERS_KEY = "coupler.workshop.layers";

export function loadUx(): Ux {
  try {
    const kept = localStorage.getItem(UX_KEY);
    return kept === "terminal" || kept === "immersive" ? kept : "workshop";
  } catch {
    return "workshop";
  }
}

export function storeUx(ux: Ux) {
  try {
    localStorage.setItem(UX_KEY, ux);
  } catch {
    // Not kept past this run, then.
  }
}

/**
 * What Workshop can show or hide, in screen order, with whether it shows
 * until the player says otherwise. The game output, the command line,
 * the message bar, the gear and the ways to play always show: without
 * them there'd be no game, no way to type, no messages, no way back to
 * this list, and no way to connect.
 */
export const SHOWABLE: { id: string; name: string; shown: boolean }[] = [
  { id: "menu", name: "The menu bar's strip", shown: true },
  { id: "mode", name: "Combat or Explore mode", shown: true },
  { id: "nowPlaying", name: "The music playing", shown: true },
  { id: "state", name: "Online or Offline", shown: true },
  { id: "hangup", name: "Hang Up", shown: true },
  { id: "where", name: "Where am I?", shown: true },
  { id: "mapToggle", name: "The Map button", shown: true },
  { id: "mixer", name: "The Mixer button", shown: true },
  { id: "bgm", name: "The BGM button: the Music Editor", shown: true },
  { id: "vitals", name: "You: your health, mana and movement, and the fight, as bars", shown: true },
  { id: "scene", name: "Here: the room, its exits, your health and the sound layers", shown: false },
  { id: "heard", name: "Heard: what Coupler's voice said", shown: false },
  { id: "picture", name: "Picture: a painting of the room you're in", shown: false },
  { id: "hint", name: "The command line's hint", shown: true },
  { id: "journal", name: "The Journal button: what's said in the game", shown: true },
  { id: "log", name: "The Log button: the channels", shown: true },
  { id: "hooks", name: "The Hooks button", shown: true },
  { id: "readout", name: "The screen readout", shown: true },
];

export function loadShown(): Record<string, boolean> {
  const shown = Object.fromEntries(SHOWABLE.map((s) => [s.id, s.shown]));
  try {
    const kept: unknown = JSON.parse(localStorage.getItem(SHOW_KEY) ?? "{}");
    if (kept && typeof kept === "object") {
      for (const [id, on] of Object.entries(kept)) if (id in shown && typeof on === "boolean") shown[id] = on;
    }
  } catch {
    // The defaults, then.
  }
  return shown;
}

export function storeShown(shown: Record<string, boolean>) {
  try {
    localStorage.setItem(SHOW_KEY, JSON.stringify(shown));
  } catch {
    // Not kept past this run, then.
  }
}

/** Immersive's two layers of its own, which Workshop can have too. */
export interface Layers {
  cues: boolean;
  voice: boolean;
}

export function loadLayers(): Layers {
  try {
    const kept = JSON.parse(localStorage.getItem(LAYERS_KEY) ?? "{}") as Partial<Layers>;
    return { cues: kept.cues === true, voice: kept.voice === true };
  } catch {
    return { cues: false, voice: false };
  }
}

export function storeLayers(layers: Layers) {
  try {
    localStorage.setItem(LAYERS_KEY, JSON.stringify(layers));
  } catch {
    // Not kept past this run, then.
  }
}

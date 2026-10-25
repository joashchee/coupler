/**
 * How many colors Coupler's pictures are painted in: as many as the
 * player has the game send them (decided 2026-10-25). CoffeeMUD answers
 * `ANSI 16`, `ANSI 256` and `ANSI TRUE` (and its account menu's ANSI16,
 * ANSI256) with the colors now on; Rust reads that line
 * (`paint::Colors::heard`) and sends `game-colors`, kept here in
 * `coupler.palette` for the next time, since the game says it only when
 * it changes. 16 until the game says more. With more, the painter draws
 * its dithers and gradients in the colors between (`Canvas::art_in` in
 * src-tauri/src/painter.rs); nothing else about a picture changes, and
 * its words are the same.
 */
import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";

/** The game's colors: the 16, the 256, or true color (millions). */
export type Palette = "16" | "256" | "true";

const KEY = "coupler.palette";
const PALETTES: Palette[] = ["16", "256", "true"];

/** Each in words, for gear → Pictures…. */
export const PALETTE_WORDS: Record<Palette, string> = {
  "16": "the 16 colors",
  "256": "256 colors",
  true: "millions of colors",
};

function kept(): Palette {
  try {
    const value = localStorage.getItem(KEY);
    return PALETTES.find((p) => p === value) ?? "16";
  } catch {
    return "16";
  }
}

let now: Palette = kept();
const listeners = new Set<() => void>();

/** The colors pictures are painted in now. */
export const palette = (): Palette => now;

export function setPalette(next: Palette) {
  if (next === now) return;
  now = next;
  try {
    localStorage.setItem(KEY, next);
  } catch {
    // Not kept past this run, then.
  }
  for (const f of listeners) f();
}

function subscribe(f: () => void) {
  listeners.add(f);
  return () => listeners.delete(f);
}

/** The colors pictures are painted in, for a component that paints again when they change. */
export const usePalette = () => useSyncExternalStore(subscribe, palette);

/** Follows the game's answers; once, at launch. */
export const followGamePalette = () => listen<Palette>("game-colors", (e) => setPalette(e.payload));

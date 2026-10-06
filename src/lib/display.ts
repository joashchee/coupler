/**
 * How Coupler shows things, for low vision and for anyone who can't hear
 * the sounds (gear → Display…, `coupler.display`):
 *
 * - **Game colors**: as the game sends them; lifted, where a color is too
 *   dark (or too light) to read on its background, to the nearest that
 *   reads at 4.5:1 (WCAG AA) with its hue kept; or none, every line in
 *   light gray (bright text stays white, so the game's emphasis is kept).
 * - **Text size** in Terminal: 1, 2 or 3 times, the game output, the
 *   command line and the messages filling the screen at that size.
 * - **Sound captions**: every sound Coupler plays written in Heard, as a
 *   film's captions are, so the cues can be seen and learned.
 * - **The map's @ blinks**, so where you are is found at a glance; off
 *   for anyone a blink bothers (and never with reduced motion).
 *
 * Every change is at once and kept.
 */

export type GameColors = "game" | "lifted" | "none";
export type TextSize = 1 | 2 | 3;

export interface Display {
  colors: GameColors;
  textSize: TextSize;
  captions: boolean;
  blink: boolean;
}

export const GAME_COLORS: { id: GameColors; name: string; summary: string }[] = [
  { id: "game", name: "As the game sends them", summary: "CoffeeMUD's own colors, unchanged." },
  { id: "lifted", name: "Readable", summary: "A color too dark or too light to read on its background is made brighter or darker until it reads, keeping its hue." },
  { id: "none", name: "No colors", summary: "Every line light gray on black; bright text stays white." },
];

export const TEXT_SIZES: { id: TextSize; name: string; summary: string }[] = [
  { id: 1, name: "Usual", summary: "80 characters across, as the game expects." },
  { id: 2, name: "Twice as big", summary: "78 characters across, 19 lines." },
  { id: 3, name: "Three times as big", summary: "52 characters across, 11 lines. The game wraps its lines to fit." },
];

const KEY = "coupler.display";
export const DEFAULT_DISPLAY: Display = { colors: "game", textSize: 1, captions: true, blink: true };

export function loadDisplay(): Display {
  try {
    const kept = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Record<string, unknown>>;
    return {
      colors: GAME_COLORS.some((c) => c.id === kept.colors) ? (kept.colors as GameColors) : DEFAULT_DISPLAY.colors,
      textSize: TEXT_SIZES.some((t) => t.id === kept.textSize) ? (kept.textSize as TextSize) : DEFAULT_DISPLAY.textSize,
      captions: typeof kept.captions === "boolean" ? kept.captions : DEFAULT_DISPLAY.captions,
      blink: typeof kept.blink === "boolean" ? kept.blink : DEFAULT_DISPLAY.blink,
    };
  } catch {
    return { ...DEFAULT_DISPLAY };
  }
}

export function saveDisplay(display: Display) {
  try {
    localStorage.setItem(KEY, JSON.stringify(display));
  } catch {
    // Not kept past this run, then.
  }
}

// ---- Readable colors ----

type Rgb = [number, number, number];

/** "#aa00ff" or "rgb(1, 2, 3)"; null for anything else (transparent). */
export function parseColor(css: string): Rgb | null {
  if (css.startsWith("#") && css.length === 7) {
    return [1, 3, 5].map((i) => parseInt(css.slice(i, i + 2), 16)) as Rgb;
  }
  const m = /^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/.exec(css);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** WCAG relative luminance. */
function luminance([r, g, b]: Rgb): number {
  const lin = (v: number) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** WCAG contrast ratio, 1 to 21. */
export function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** Text needs this much (docs/ansiapps-color-contrast.md: the VGA font is never "large text"). */
export const READABLE = 4.5;

const mix = (from: Rgb, to: Rgb, t: number): Rgb => from.map((v, i) => Math.round(v + (to[i] - v) * t)) as Rgb;

/**
 * The text color moved toward white (on a dark background) or black (on
 * a light one) just far enough to read at 4.5:1; unchanged if it already
 * does. Toward whichever reaches it, white first on the usual black.
 */
export function readable(text: Rgb, background: Rgb): Rgb {
  if (contrast(text, background) >= READABLE) return text;
  const white: Rgb = [255, 255, 255];
  const black: Rgb = [0, 0, 0];
  const toward = contrast(white, background) >= contrast(black, background) ? white : black;
  // The least mix that reads, to a hundredth.
  let lo = 0;
  let hi = 1;
  for (let i = 0; i < 12; i++) {
    const mid = (lo + hi) / 2;
    if (contrast(mix(text, toward, mid), background) >= READABLE) hi = mid;
    else lo = mid;
  }
  return mix(text, toward, hi);
}

export const rgbCss = ([r, g, b]: Rgb) => `rgb(${r}, ${g}, ${b})`;

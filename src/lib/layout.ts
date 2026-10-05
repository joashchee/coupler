/**
 * Where the user has put things: a panel that's been moved, resized or
 * brought to the front (components/Movable.tsx) keeps that here, under
 * its ID, and has it again next launch. What was never changed has no
 * entry, and the panel has its usual place and size.
 *
 * Places and sizes are in stage pixels (lib/stage.ts); a place is the
 * panel's top left corner, counted from the top left of the area it
 * moves in.
 */
import { CELL_HEIGHT, CELL_WIDTH } from "./stage";

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export type Rect = Point & Size;

/** What's kept for one panel: `z` is its turn in the pile, the highest in front. */
export interface PanelLayout extends Partial<Rect> {
  z?: number;
}

/** Every changed panel, as JSON: `{ "map": { "x": 864, "y": 0, "z": 2 } }`. */
const LAYOUT_KEY = "coupler.layout";
const FIELDS = ["x", "y", "width", "height", "z"] as const;

function loadLayout(): Record<string, PanelLayout> {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(LAYOUT_KEY) ?? "{}");
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? (parsed as Record<string, PanelLayout>) : {};
  } catch {
    return {};
  }
}

/** What the user has changed about this panel: only those fields are set. */
export function storedPanel(id: string): PanelLayout {
  const entry: unknown = loadLayout()[id];
  const kept: PanelLayout = {};
  if (!entry || typeof entry !== "object") return kept;
  for (const field of FIELDS) {
    const value = (entry as Record<string, unknown>)[field];
    if (typeof value === "number" && Number.isFinite(value)) kept[field] = value;
  }
  return kept;
}

/** Keeps the fields given; a field given as undefined is forgotten, so it's back to the usual. */
export function storePanel(id: string, change: PanelLayout) {
  try {
    const layout = loadLayout();
    const entry: PanelLayout = { ...storedPanel(id), ...change };
    for (const field of FIELDS) if (entry[field] === undefined) delete entry[field];
    if (Object.keys(entry).length > 0) layout[id] = entry;
    else delete layout[id];
    localStorage.setItem(LAYOUT_KEY, JSON.stringify(layout));
  } catch {
    // localStorage unavailable: the panel just won't keep its place across launches.
  }
}

/** The highest turn in the pile any panel has: 0 when none was ever brought to the front. */
export function frontZ(): number {
  return Math.max(0, ...Object.keys(loadLayout()).map((id) => storedPanel(id).z ?? 0));
}

/**
 * The nearest rectangle to `wanted` that is whole inside its area and,
 * when `snap` is set (the ANSIapps theme), on the character grid. With
 * `least` (a panel that can be resized) the size is fitted too: no
 * smaller than that, no bigger than the area. Without it the size is
 * the panel's own and only the place is fitted.
 */
export function fitRect(wanted: Rect, least: Size | null, area: Size, snap: boolean): Rect {
  const whole = (value: number, most: number, cell: number) => {
    const clamped = Math.min(Math.max(0, value), Math.max(0, most));
    if (!snap) return Math.round(clamped);
    return Math.min(Math.round(clamped / cell) * cell, Math.floor(Math.max(0, most) / cell) * cell);
  };
  const width = least ? Math.max(whole(wanted.width, area.width, CELL_WIDTH), Math.min(least.width, area.width)) : wanted.width;
  const height = least ? Math.max(whole(wanted.height, area.height, CELL_HEIGHT), Math.min(least.height, area.height)) : wanted.height;
  return {
    x: whole(wanted.x, area.width - width, CELL_WIDTH),
    y: whole(wanted.y, area.height - height, CELL_HEIGHT),
    width,
    height,
  };
}

/** Tells every panel on screen that the layout was forgotten (components/Movable.tsx listens). */
export const LAYOUT_FORGOTTEN = "coupler-layout-forgotten";

/** Forgets everything the user arranged: every panel is back to its usual place, size and order. */
export function forgetLayout() {
  try {
    localStorage.removeItem(LAYOUT_KEY);
  } catch {
    // Nothing was kept, then.
  }
  window.dispatchEvent(new Event(LAYOUT_FORGOTTEN));
}

/** What's kept, as text: for carrying an arrangement worked out on screen back into the usual layout. */
export function layoutText(): string {
  return JSON.stringify(loadLayout(), null, 2);
}

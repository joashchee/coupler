/**
 * The facts the guide to making a character shows beside a race (each
 * player race's facts in few words and what sets it apart), and
 * CoffeeMUD's own portraits as ANSI art in the 16 VGA colors, shown only
 * when the player chooses them over Coupler's painter (gear → Pictures…;
 * CLAUDE.md rule 11: the painter's portraits, src-tauri/src/portrait.rs,
 * come first). `scripts/creation-art.py`
 * makes them from the CoffeeMUD source into `public/creation-art.json`,
 * read once, when the guide first needs it (a file of Coupler's own,
 * never the game's web host: CLAUDE.md rule 2).
 */
import type { Line } from "./mud";

interface Art {
  columns: number;
  rows: number;
  /** A byte a cell, base64: the top pixel's color << 4 | the bottom's. */
  cells: string;
}

export interface Portrait {
  name?: string;
  art?: Art;
  /** A race's facts in few words: stats, senses, height, lifespan, what it knows. */
  facts?: string;
  /** What sets a race apart from the other player races. */
  unlike?: string;
  /** CoffeeMUD's help on the race, for one its list gives no words for. */
  help?: string;
}

interface Book {
  races: Record<string, Portrait>;
  classes: Record<string, Portrait>;
}

let book: Promise<Book> | null = null;

/** The pictures and facts, read once. */
export function load(): Promise<Book> {
  book ??= fetch("/creation-art.json")
    .then((r) => (r.ok ? (r.json() as Promise<Book>) : { races: {}, classes: {} }))
    .catch(() => ({ races: {}, classes: {} }));
  return book;
}

/** How a race or class is looked up: letters only, lower case ("Half Elf" is "halfelf"). */
export const key = (name: string) => name.toLowerCase().replace(/[^a-z]/g, "");

/** A picture as lines of upper half blocks, top pixel the text color, bottom the background. */
export function lines(art: Art): Line[] {
  const bytes = Uint8Array.from(atob(art.cells), (c) => c.charCodeAt(0));
  const out: Line[] = [];
  for (let row = 0; row < art.rows; row++) {
    const line: Line = [];
    for (let column = 0; column < art.columns; column++) {
      const cell = bytes[row * art.columns + column] ?? 0;
      line.push({ text: "▀", fg: { kind: "index", index: cell >> 4 }, bg: { kind: "index", index: cell & 15 } });
    }
    out.push(line);
  }
  return out;
}

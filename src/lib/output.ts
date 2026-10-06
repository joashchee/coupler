/**
 * What Immersive leaves out of the game output, told from the text: the
 * player's prompt (also where the game wrote on after it without a new
 * line) and the room's exits (the cues play them, the map shows them).
 * Pure; components/Terminal.tsx and lib/immersive.ts use it.
 */
import type { Line } from "./mud";

/**
 * Whether an unfinished line is the player's prompt (`<100hp 50m 80mv>`),
 * not a question waiting for an answer ("Quit (y/N)?", "<pause - enter>",
 * "Choose one:"), which always shows.
 */
export function playerPrompt(text: string): boolean {
  const t = text.trim();
  return t !== "" && !/[?:]$/.test(t) && !/\b(y\/n|pause|enter|press|return)\b/i.test(t);
}

/**
 * How many characters at the start of a finished line are the player's
 * prompt left unfinished before it: the game wrote on after the prompt
 * (`<100hp 50m 80mv> A rat arrives from the north.`). 0 when it didn't.
 */
export function promptCut(text: string, prompt: string | null): number {
  if (!prompt || !playerPrompt(prompt)) return 0;
  const lead = text.startsWith(prompt) ? prompt.length : text.startsWith(prompt.trimEnd()) ? prompt.trimEnd().length : 0;
  if (lead === 0) return 0;
  // The spaces after it go with it.
  return lead + (text.slice(lead).length - text.slice(lead).trimStart().length);
}

/** A line with its first `n` characters left out, colors kept. */
export function cutLine(line: Line, n: number): Line {
  let left = n;
  const out: Line = [];
  for (const span of line) {
    if (left >= span.text.length) {
      left -= span.text.length;
      continue;
    }
    out.push(left > 0 ? { ...span, text: span.text.slice(left) } : span);
    left = 0;
  }
  return out;
}

/** The start of the room's exits: `[Exits: north, east]` (AUTOEXITS) or `Obvious exits:` (EXITS). */
const EXITS = /^\s*(\[\s*exits?\s*:|obvious\s+exits?\s*:|exits?\s*:)/i;
/** A row of the long list under `Obvious exits:`: `North : The town square`, or `None.` */
const EXIT_ROW = /^\s*([A-Za-z][A-Za-z -]{0,20}\s*:\s|none\.?\s*$)/i;

export const exitsLine = (text: string) => EXITS.test(text);

/**
 * Tells the exits' lines, across lines: a bracketed list runs to its `]`
 * (wrapped at 80), the long list's rows follow its header.
 */
export class ExitsReader {
  private open: "bracket" | "rows" | null = null;

  /** Whether this finished line is part of the exits. */
  line(text: string): boolean {
    if (this.open === "bracket") {
      if (text.includes("]")) this.open = null;
      return true;
    }
    if (this.open === "rows") {
      if (text.trim() !== "" && EXIT_ROW.test(text)) return true;
      this.open = null;
    }
    const m = EXITS.exec(text);
    if (!m) return false;
    if (m[1].startsWith("[")) this.open = text.includes("]") ? null : "bracket";
    else if (text.slice(m[0].length).trim() === "") this.open = "rows";
    return true;
  }

  reset() {
    this.open = null;
  }
}

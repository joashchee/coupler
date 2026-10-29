// Depth for cells: gradients that follow the kit (four steps or more, smeared vertically by preference), ordered (Bayer) dithers, and
// `tones`, the in-between colour of every cell, which 256-colour and truecolor output reads in place of a shade glyph. Browser-safe.
import { tone, rampCells, legalize } from './cells.js';
import { nearest } from './palette-ansi.js';

const DIRS = { down: [0, 1], up: [0, -1], right: [1, 0], left: [-1, 0] };

/**
 * Fill a region with a gradient along `ramp` (a name from `ANSI_RAMPS` or a list of VGA indexes). The kit's rules are kept: the
 * region needs room along the gradient, at least four cells, or it throws; the steps are the ramp's `░ ▒ ▓` ladders, legal for
 * `bgLimit`. `dir` is `down` (the default, a vertical smear), `up`, `right` or `left`; horizontal runs of shade glyphs look worse,
 * so prefer vertical. Returns the cells.
 */
export function gradient(cells, x, y, w, h, ramp, { dir = 'down', bgLimit = 8 } = {}) {
  const d = DIRS[dir]; if (!d) throw new Error(`unknown direction "${dir}"; known: ${Object.keys(DIRS).join(', ')}`);
  const along = d[0] ? w : h;
  if (along < 4) throw new RangeError(`a gradient needs at least four cells along it, not ${along}`);
  const steps = rampCells(ramp, { bgLimit });
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) {
    const t = d[0] ? (d[0] > 0 ? i : w - 1 - i) / (w - 1) : (d[1] > 0 ? j : h - 1 - j) / (h - 1);
    cells.set(x + i, y + j, steps[Math.min(steps.length - 1, Math.floor(t * steps.length))]);
  }
  return cells;
}

// The 4 by 4 Bayer matrix, thresholds 0 to 15.
const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

/**
 * An ordered dither between two colours: each cell is solid `a` or solid `b`, the share of `b` following `level` (0 to 1, or
 * `(x, y) => level`) by the 4 by 4 Bayer pattern. No randomness, so the same call is the same cells. Solid cells are legal at any
 * background limit when both colours are under 8; use `bayer` for night falls, hazes and fades that a `░▒▓` ladder would band.
 */
export function bayer(cells, x, y, w, h, a, b, level) {
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) {
    const t = typeof level === 'function' ? level(i, j) : level, on = BAYER[(j & 3) * 4 + (i & 3)] < t * 16;
    const c = on ? b : a;
    cells.set(x + i, y + j, { ch: ' ', fg: c, bg: c, flags: 0 });
  }
  return cells;
}

/** The effective colour of every cell as `#rrggbb`, row by row (`tone` of each cell): what a truecolor viewer paints for it. */
export function tones(cells) {
  const out = [];
  for (let y = 0; y < cells.h; y++) for (let x = 0; x < cells.w; x++) out.push(tone(cells.get(x, y)));
  return out;
}

const SHADE = new Set([0x2591, 0x2592, 0x2593]);

/**
 * Resolve the blends of a picture to solid colours at a depth: every `░ ▒ ▓` cell becomes a space whose background is the nearest of
 * the 256 (or the 16) to its in-between colour, so a dither or ladder reads as the colour it stood for. Half blocks keep their two
 * halves. The result is for a 256-colour file (`writeAnsi(..., { colors: 256 })`); at `depth: 16` it is legalized for `bgLimit`.
 * Colours are palette indexes, so truecolor output reads `tones` instead.
 */
export function resolveTones(cells, { depth = 256, bgLimit = 16 } = {}) {
  if (depth !== 16 && depth !== 256) throw new Error(`resolveTones depth is 16 or 256, not ${depth}`);
  const out = cells.clone(), memo = new Map();
  for (let i = 0; i < out.ch.length; i++) {
    if (!SHADE.has(out.ch[i])) continue;
    const key = `${out.ch[i]},${out.fg[i]},${out.bg[i]}`;
    if (!memo.has(key)) memo.set(key, nearest(tone({ ch: out.ch[i], fg: out.fg[i], bg: out.bg[i] }), depth).index);
    const c = memo.get(key); out.ch[i] = 0x20; out.fg[i] = c; out.bg[i] = c;
  }
  return depth === 16 ? legalize(out, { bgLimit }) : out;
}

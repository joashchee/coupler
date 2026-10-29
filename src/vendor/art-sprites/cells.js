// The cell canvas: the grid ANSI art lives on. A cell is a character and two colours (palette INDEXES, see palette-ansi.js), so
// where a Pixels canvas has a colour per pixel this has a glyph per cell. Two ways to fill it: draw on it directly with the primitives
// below (native ANSI recipes), or draw a pixel sprite and fold it (`foldPixels`: each cell is two square pixels, one above the other).
import { DB32 } from './palette.js';
import { DB32_VGA, ANSI_RAMPS, nearest, indexHex, rgbOf, distance } from './palette-ansi.js';
import { WEIGHTS, armsOf, join } from './boxes.js';
import { Pixels } from './pixels.js';

/** No colour: the cell shows the viewer's own background (or leaves what is below it, when blitted). */
export const NONE = 0xffff;
/** Cell flags. BLINK means a bright background when the file declares iCE colours (SAUCE), as the BBS scene drew it. */
export const BOLD = 1, BLINK = 2;

const weightsOf = (style) => WEIGHTS[style] ?? (() => { throw new Error(`unknown box style "${style}"; known: ${Object.keys(WEIGHTS).join(', ')}`); })();
const code = (ch) => (typeof ch === 'string' ? ch.codePointAt(0) ?? 0 : ch ?? 0);

export class Cells {
  constructor(w, h) {
    this.w = w; this.h = h;
    this.ch = new Uint32Array(w * h);                 // code point; 0 is empty (shows nothing, blits through)
    this.fg = new Uint16Array(w * h).fill(NONE);
    this.bg = new Uint16Array(w * h).fill(NONE);
    this.flags = new Uint8Array(w * h);
  }

  inb(x, y) { return x >= 0 && y >= 0 && x < this.w && y < this.h; }
  get(x, y) {
    if (!this.inb(x, y)) return { ch: 0, fg: NONE, bg: NONE, flags: 0 };
    const i = y * this.w + x; return { ch: this.ch[i], fg: this.fg[i], bg: this.bg[i], flags: this.flags[i] };
  }
  /** Set the parts of a cell that are given (`ch` a character or a code point); the rest stay as they were. */
  set(x, y, { ch, fg, bg, flags } = {}) {
    x = Math.floor(x); y = Math.floor(y); if (!this.inb(x, y)) return this;
    const i = y * this.w + x;
    if (ch !== undefined) this.ch[i] = code(ch);
    if (fg !== undefined) this.fg[i] = fg;
    if (bg !== undefined) this.bg[i] = bg;
    if (flags !== undefined) this.flags[i] = flags;
    return this;
  }
  /** Whether a cell has nothing in it: no character, no colours. A blit skips these. */
  isEmpty(x, y) { const i = y * this.w + x; return this.ch[i] === 0 && this.fg[i] === NONE && this.bg[i] === NONE; }

  rect(x, y, w, h, cell) { for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) this.set(x + i, y + j, cell); return this; }
  fill(cell) { return this.rect(0, 0, this.w, this.h, cell); }
  /** A string along a row, one character a cell. */
  text(x, y, str, fg, bg) { [...str].forEach((c, i) => this.set(x + i, y, { ch: c, fg, bg })); return this; }
  /** A glyph over a cell: its character and colour, keeping the background (a star on a sky, rain on a hill). */
  glyph(x, y, ch, fg) { return this.set(x, y, { ch, fg }); }
  blit(src, dx = 0, dy = 0) {
    for (let y = 0; y < src.h; y++) for (let x = 0; x < src.w; x++) if (!src.isEmpty(x, y)) this.set(x + dx, y + dy, src.get(x, y));
    return this;
  }
  /** A horizontal line of box-drawing characters; crossing or ending on another line makes the right join. */
  hline(x, y, len, { style = 'single', fg, bg } = {}) { return this.#line(x, y, len, 1, 0, weightsOf(style).h, fg, bg); }
  vline(x, y, len, { style = 'single', fg, bg } = {}) { return this.#line(x, y, len, 0, 1, weightsOf(style).v, fg, bg); }
  /** A box outline of `style` (`single`, `double`, or `mixed`: double horizontals, single verticals) with its corners, joined to any lines it meets. */
  box(x, y, w, h, { style = 'single', fg, bg } = {}) {
    if (w < 2 || h < 2) throw new RangeError(`a box is at least 2 by 2, not ${w} by ${h}`);
    const { h: hw, v: vw } = weightsOf(style), r = x + w - 1, b = y + h - 1;
    for (let i = 1; i < w - 1; i++) { this.#piece(x + i, y, [0, hw, 0, hw], fg, bg); this.#piece(x + i, b, [0, hw, 0, hw], fg, bg); }
    for (let j = 1; j < h - 1; j++) { this.#piece(x, y + j, [vw, 0, vw, 0], fg, bg); this.#piece(r, y + j, [vw, 0, vw, 0], fg, bg); }
    this.#piece(x, y, [0, hw, vw, 0], fg, bg); this.#piece(r, y, [0, 0, vw, hw], fg, bg);
    this.#piece(x, b, [vw, hw, 0, 0], fg, bg); this.#piece(r, b, [vw, 0, 0, hw], fg, bg);
    return this;
  }
  /** A rectangle of one shade glyph (` `, `░`, `▒`, `▓` or `█`): fg is the glyph, bg what shows through it. */
  shadeFill(x, y, w, h, glyph, fg, bg) {
    if (![...' ░▒▓█'].includes(glyph)) throw new Error(`"${glyph}" is not a shade glyph; known: space ░ ▒ ▓ █`);
    return this.rect(x, y, w, h, { ch: glyph, fg, bg });
  }
  // One piece of a line at a cell, merged with the box character already there. The end of a line that lands on another line is a
  // stub toward the line (a T), where on an empty cell it is a full piece.
  #piece(x, y, arms, fg, bg) {
    if (!this.inb(x, y)) return;
    this.set(x, y, { ch: join(this.ch[y * this.w + x], arms), fg, bg });
  }
  #line(x, y, len, dx, dy, wt, fg, bg) {
    for (let i = 0; i < len; i++) {
      const cx = x + dx * i, cy = y + dy * i, full = [dy * wt, dx * wt, dy * wt, dx * wt];
      const end = len > 1 && (i === 0 || i === len - 1) && this.inb(cx, cy) && armsOf(this.ch[cy * this.w + cx]);
      this.#piece(cx, cy, end ? (i === 0 ? [0, dx * wt, dy * wt, 0] : [dy * wt, 0, 0, dx * wt]) : full, fg, bg);
    }
    return this;
  }
  clone() { const c = new Cells(this.w, this.h); c.ch.set(this.ch); c.fg.set(this.fg); c.bg.set(this.bg); c.flags.set(this.flags); return c; }
  equals(o) { return o.w === this.w && o.h === this.h && ['ch', 'fg', 'bg', 'flags'].every((k) => this[k].every((v, i) => v === o[k][i])); }
  /** The characters alone, a string a row (empty cells as spaces): for looking at and for tests. */
  toText() { return Array.from({ length: this.h }, (_, y) => Array.from({ length: this.w }, (_, x) => String.fromCodePoint(this.ch[y * this.w + x] || 32)).join('')).join('\n'); }
}

// Legal cells. A classic .ans has 8 background colours (0 to 7): a bright background was the blink bit, honoured only when the file
// declares iCE colours. `bgLimit` is 8 for the classic target and 16 for iCE, UTF-8 and modern terminals. Backgrounds from 16 up
// (256-colour mode) and NONE are never touched here: reducing depth is the writer's job.
const SHADES = { 0x20: 0, 0x2591: 0.25, 0x2592: 0.5, 0x2593: 0.75, 0x2588: 1 };
const rgbAt = (i) => rgbOf(indexHex(i === NONE ? 0 : i));        // no colour shows the viewer's background, taken as black
const blend = (a, b, t) => a.map((v, k) => v + (b[k] - v) * t);
const hex2 = (n) => Math.round(n).toString(16).padStart(2, '0');
const legalBg = (bg, limit) => bg === NONE || bg < limit || bg >= 16;
const checkLimit = (limit) => { if (limit !== 8 && limit !== 16) throw new Error(`bgLimit is 8 (classic) or 16 (iCE, UTF-8), not ${limit}`); return limit; };

/** The colours of a cell's upper and lower halves as [r, g, b] each: `▀` and `▄` are two colours, a shade glyph mixes fg over bg
 *  at 25, 50 or 75 percent, and any other glyph is taken as a half mix (what it covers is not known). */
export function halves({ ch, fg, bg }) {
  const f = rgbAt(fg), b = rgbAt(bg);
  if (ch === 0x2580) return [f, b];
  if (ch === 0x2584) return [b, f];
  const m = blend(b, f, SHADES[ch ?? 0x20] ?? (ch === 0 ? 0 : 0.5));
  return [m, m];
}
/** The effective colour of a cell as `#rrggbb`: its two halves averaged. This is the colour 256 and truecolor output reads. */
export function tone(cell) {
  const [t, b] = halves(cell), m = blend(t, b, 0.5);
  return '#' + m.map(hex2).join('');
}

// Which glyph shows the same picture with the colours reversed.
const REVERSE = { 0x2580: 0x2584, 0x2584: 0x2580, 0x2591: 0x2593, 0x2593: 0x2591, 0x2592: 0x2592 };
const CANDIDATES = [0x20, 0x2591, 0x2592, 0x2593, 0x2588, 0x2580, 0x2584];
const error = (a, b) => { const [at, ab] = halves(a), [bt, bb] = halves(b); return distance(at, bt) + distance(ab, bb); };

/** One cell made legal for a limit: reversed where that does it, else the glyph and colours (foreground any of 16, background under
 *  the limit) that look least unlike it; a text glyph keeps its character and takes the nearest legal background. */
export function legalCell(cell, bgLimit = 8) {
  const limit = checkLimit(bgLimit), { ch, fg, bg, flags } = cell;
  if (legalBg(bg, limit)) return cell;
  if (ch === 0 || ch === 0x20) return { ch: 0x2588, fg: bg, bg: NONE, flags };        // a space on a bright background is a full block of it
  if (ch === 0x2588) return { ch, fg, bg: NONE, flags };                                 // the background never shows
  if (REVERSE[ch] && legalBg(fg, limit)) return { ch: REVERSE[ch], fg: bg, bg: fg, flags };
  if (!REVERSE[ch]) {
    let best = 0, bd = Infinity;
    for (let c = 0; c < limit; c++) { const d = distance(rgbAt(c), rgbAt(bg)); if (d < bd) { bd = d; best = c; } }
    return { ch, fg, bg: best, flags };
  }
  let out, od = Infinity;
  for (const g of CANDIDATES) for (let f = 0; f < 16; f++) for (let b = 0; b < limit; b++) {
    if ((g === 0x20 && f !== b) || (g === 0x2588 && b !== 0)) continue;                 // a space ignores fg, a full block ignores bg
    const c = { ch: g, fg: f, bg: b, flags }, d = error(c, cell);
    if (d < od) { od = d; out = c; }
  }
  return out.ch === 0x2588 ? { ...out, bg: NONE } : out;
}

/** A copy of the cells with every background legal for `bgLimit` (8 classic, 16 iCE and modern: a plain copy). It shows the same
 *  colours where a reversal allows it and the nearest legal ones where not. */
export function legalize(cells, { bgLimit = 8 } = {}) {
  const out = cells.clone();
  if (checkLimit(bgLimit) >= 16) return out;
  const memo = new Map();
  for (let i = 0; i < out.ch.length; i++) {
    if (legalBg(out.bg[i], bgLimit)) continue;
    const key = `${out.ch[i]},${out.fg[i]},${out.bg[i]},${out.flags[i]}`;
    if (!memo.has(key)) memo.set(key, legalCell({ ch: out.ch[i], fg: out.fg[i], bg: out.bg[i], flags: out.flags[i] }, bgLimit));
    const c = memo.get(key); out.ch[i] = c.ch; out.fg[i] = c.fg; out.bg[i] = c.bg; out.flags[i] = c.flags;
  }
  return out;
}

/**
 * The five-step run from colour `a` to colour `b`: `a`, then `░ ▒ ▓` of `b` over `a` (25, 50, 75 percent), then `b`, as cell
 * descriptors `{ ch, fg, bg, flags }`. Each is oriented to be legal for `bgLimit`: with `a` bright the steps use `b` as the background
 * and the reversed glyphs. When both are bright (and different) no glyph is exact and the steps are the nearest legal ones.
 */
export function ladder(a, b, { bgLimit = 8 } = {}) {
  return [
    { ch: 0x20, fg: a, bg: a }, { ch: 0x2591, fg: b, bg: a }, { ch: 0x2592, fg: b, bg: a }, { ch: 0x2593, fg: b, bg: a }, { ch: 0x20, fg: b, bg: b },
  ].map((c) => legalCell({ ...c, flags: 0 }, bgLimit));
}

/** A whole ramp as one run of steps (four to a pair of colours, and the last colour): a name from `ANSI_RAMPS` or a list of indexes. */
export function rampCells(ramp, opts) {
  const list = typeof ramp === 'string' ? ANSI_RAMPS[ramp] : ramp;
  if (!list) throw new Error(`unknown ramp "${ramp}"; known: ${Object.keys(ANSI_RAMPS).join(', ')}`);
  const out = [];
  for (let i = 0; i + 1 < list.length; i++) out.push(...ladder(list[i], list[i + 1], opts).slice(0, 4));
  out.push(ladder(list[list.length - 2], list[list.length - 1], opts)[4]);
  return out;
}

/** A palette (an array of hex strings) as a map from a pixel value (0 transparent, else 1 + index) to a colour index at 16. */
export function colorMap(palette, map = DB32_VGA) {
  if (palette === DB32 || palette.length === DB32.length && palette.every((h, i) => h === DB32[i])) return (v) => (v ? map[v - 1] : NONE);
  const cache = new Map();
  return (v) => { if (!v) return NONE; if (!cache.has(v)) cache.set(v, nearest(palette[v - 1], 16).index); return cache.get(v); };
}

/**
 * Fold a pixel canvas to cells: each cell is two pixels, the upper in the foreground of `▀` and the lower in the background, so a
 * w by h canvas becomes w by ceil(h / 2) cells and pixels stay square. Where only one of the two is drawn the cell is `▀` or `▄`
 * with no background, so a sprite leaves what is behind it; where both are the same colour it is a space of that colour. Transparent
 * pairs are empty cells. `map` sends a DB32 index to a VGA colour (see DB32_VGA); other palettes go to their nearest of the 16.
 * `bgLimit` (8 classic, the default; 16 for iCE and UTF-8 targets) makes the result legal: see `legalize`.
 */
export function foldPixels(px, { map, bgLimit = 8 } = {}) {
  const to = colorMap(px.palette, map), cells = new Cells(px.w, Math.ceil(px.h / 2));
  for (let y = 0; y < cells.h; y++) for (let x = 0; x < px.w; x++) {
    const t = to(px.get(x, y * 2)), b = to(px.get(x, y * 2 + 1));
    if (t === NONE && b === NONE) continue;
    if (t === b) cells.set(x, y, { ch: ' ', fg: t, bg: t });
    else if (b === NONE) cells.set(x, y, { ch: '▀', fg: t });
    else if (t === NONE) cells.set(x, y, { ch: '▄', fg: b });
    else cells.set(x, y, { ch: '▀', fg: t, bg: b });
  }
  return legalize(cells, { bgLimit });
}

const plain = (v) => v !== null && typeof v === 'object' && Object.getPrototypeOf(v) === Object.prototype;
// Folds every canvas found in a value (a frame, a clip, a layer, a mask); data it does not understand is kept as it was.
const foldValue = (v, opts) => (v instanceof Pixels ? foldPixels(v, opts) : Array.isArray(v) ? v.map((x) => foldValue(x, opts)) : plain(v) ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, k === 'anchors' ? foldAnchors(x) : foldValue(x, opts)])) : v);
// An anchor is a [x, y] pixel: x is a column still, y is half a row.
const foldAnchors = (a) => (plain(a) ? Object.fromEntries(Object.entries(a).map(([k, p]) => [k, Array.isArray(p) && p.length === 2 && p.every(Number.isFinite) ? [p[0], p[1] / 2] : p])) : a);

/** A whole drawing folded to cells: frames, clips, layers and masks become Cells, anchors move to cell units, and it says `style: 'ansi'`. */
export function foldArt(art, opts) { return { ...foldValue(art, opts), style: 'ansi', folded: true }; }

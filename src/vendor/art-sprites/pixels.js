// An indexed-colour canvas and the primitives every recipe draws with. Value 0 is transparent, any other value picks
// `palette[value - 1]`. Indexed, not RGBA, so a palette swap or a tier recolour is an exact mapping and never a guess.
import { DB32, luma } from './palette.js';

export class Pixels {
  constructor(w, h, palette = DB32) { this.w = w; this.h = h; this.palette = palette; this.data = new Uint8Array(w * h); }

  /** A canvas from text rows: `legend` maps each character to a colour value; '.' and ' ' are transparent. */
  static fromRows(rows, legend, palette = DB32) {
    const p = new Pixels(Math.max(...rows.map((r) => r.length)), rows.length, palette);
    rows.forEach((row, y) => [...row].forEach((ch, x) => { if (ch !== '.' && ch !== ' ') { if (!(ch in legend)) throw new Error(`no legend for "${ch}"`); p.set(x, y, legend[ch]); } }));
    return p;
  }
  /** A canvas from RGBA bytes; the palette is built from the colours found (at most 255). */
  static fromRgba(w, h, rgba) {
    const palette = [], seen = new Map(), p = new Pixels(w, h, palette);
    for (let i = 0; i < w * h; i++) {
      if (rgba[i * 4 + 3] < 128) continue;
      const hex = '#' + [0, 1, 2].map((k) => rgba[i * 4 + k].toString(16).padStart(2, '0')).join('');
      if (!seen.has(hex)) { if (palette.length >= 255) throw new Error('more than 255 colours'); palette.push(hex); seen.set(hex, palette.length); }
      p.data[i] = seen.get(hex);
    }
    return p;
  }

  inb(x, y) { return x >= 0 && y >= 0 && x < this.w && y < this.h; }
  get(x, y) { return this.inb(x, y) ? this.data[y * this.w + x] : 0; }
  set(x, y, c) { x = Math.floor(x); y = Math.floor(y); if (this.inb(x, y)) this.data[y * this.w + x] = c; return this; }
  clone() { const p = new Pixels(this.w, this.h, this.palette); p.data.set(this.data); return p; }
  equals(o) { return o.w === this.w && o.h === this.h && this.data.every((v, i) => v === o.data[i]); }

  line(x0, y0, x1, y1, c) {
    x0 = Math.round(x0); y0 = Math.round(y0); x1 = Math.round(x1); y1 = Math.round(y1);
    const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0), sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
    for (let e = dx + dy; ; ) {
      this.set(x0, y0, c);
      if (x0 === x1 && y0 === y1) break;
      const e2 = 2 * e;
      if (e2 >= dy) { e += dy; x0 += sx; }
      if (e2 <= dx) { e += dx; y0 += sy; }
    }
    return this;
  }
  /** A line `t` pixels thick, grown to the right and down so it stays on the pixel grid. */
  thick(x0, y0, x1, y1, c, t = 2) { for (let i = 0; i < t; i++) this.line(x0 + (i & 1), y0 + (i >> 1), x1 + (i & 1), y1 + (i >> 1), c); return this; }
  rect(x, y, w, h, c) { for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) this.set(x + i, y + j, c); return this; }
  frame(x, y, w, h, c) { return this.rect(x, y, w, 1, c).rect(x, y + h - 1, w, 1, c).rect(x, y, 1, h, c).rect(x + w - 1, y, 1, h, c); }
  /** Filled ellipse centred at (cx, cy) in pixel-edge coordinates, so cx = 8 is the line between pixels 7 and 8. */
  ellipse(cx, cy, rx, ry, c) {
    for (let y = Math.floor(cy - ry); y <= Math.ceil(cy + ry); y++) for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x++)
      if (((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1) this.set(x, y, c);
    return this;
  }
  /** Filled polygon by pixel centres: points are [x, y] pairs. */
  poly(pts, c) {
    const xs = pts.map((p) => p[0]), ys = pts.map((p) => p[1]);
    for (let y = Math.floor(Math.min(...ys)); y <= Math.ceil(Math.max(...ys)); y++) for (let x = Math.floor(Math.min(...xs)); x <= Math.ceil(Math.max(...xs)); x++) {
      let inside = false;
      for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
        const [xi, yi] = pts[i], [xj, yj] = pts[j], px = x + 0.5, py = y + 0.5;
        if ((yi > py) !== (yj > py) && px < ((xj - xi) * (py - yi)) / (yj - yi) + xi) inside = !inside;
      }
      if (inside) this.set(x, y, c);
    }
    return this;
  }
  fill(x, y, c) {
    const from = this.get(x, y); if (from === c || !this.inb(x, y)) return this;
    const todo = [[x, y]];
    while (todo.length) { const [i, j] = todo.pop(); if (this.get(i, j) !== from || !this.inb(i, j)) continue; this.set(i, j, c); todo.push([i + 1, j], [i - 1, j], [i, j + 1], [i, j - 1]); }
    return this;
  }
  /** Replace colour `a` with `b` (a single colour, or parallel arrays: a palette-group swap). */
  swap(a, b) {
    const m = new Map([].concat(a).map((v, i) => [v, [].concat(b)[i] ?? [].concat(b).at(-1)]));
    this.data.forEach((v, i) => { if (m.has(v)) this.data[i] = m.get(v); });
    return this;
  }
  /** Draw `src` over this at (dx, dy); transparent pixels of `src` leave this untouched. */
  blit(src, dx = 0, dy = 0) { for (let y = 0; y < src.h; y++) for (let x = 0; x < src.w; x++) { const v = src.data[y * src.w + x]; if (v) this.set(x + dx, y + dy, v); } return this; }
  /** Add colour `c` on transparent pixels touching the art (4-neighbour; `diag` adds corners). */
  outline(c, diag = false) {
    const src = this.clone();
    for (let y = 0; y < this.h; y++) for (let x = 0; x < this.w; x++) {
      if (src.get(x, y)) continue;
      const hit = src.get(x - 1, y) || src.get(x + 1, y) || src.get(x, y - 1) || src.get(x, y + 1) || (diag && (src.get(x - 1, y - 1) || src.get(x + 1, y - 1) || src.get(x - 1, y + 1) || src.get(x + 1, y + 1)));
      if (hit) this.set(x, y, c);
    }
    return this;
  }
  map(fn) { const p = new Pixels(this.w, this.h, this.palette); for (let y = 0; y < this.h; y++) for (let x = 0; x < this.w; x++) p.data[y * this.w + x] = fn(x, y); return p; }
  flipX() { return this.map((x, y) => this.get(this.w - 1 - x, y)); }
  flipY() { return this.map((x, y) => this.get(x, this.h - 1 - y)); }
  /** Quarter turn clockwise (square canvases only), exact: no resampling. */
  rot90() { if (this.w !== this.h) throw new Error('rot90 needs a square canvas'); return this.map((x, y) => this.get(y, this.w - 1 - x)); }
  /** The `w` x `h` window whose top left is (x, y), as a canvas of its own (cells outside this one are empty). */
  crop(x, y, w, h) { const p = new Pixels(w, h, this.palette); for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) p.data[j * w + i] = this.get(x + i, y + j); return p; }
  shift(dx, dy) { return this.map((x, y) => this.get(x - dx, y - dy)); }
  /** Whole sprite recoloured one colour (the hit flash). */
  silhouette(c) { return this.map((x, y) => (this.get(x, y) ? c : 0)); }
  bbox() {
    let x0 = this.w, y0 = this.h, x1 = -1, y1 = -1;
    for (let y = 0; y < this.h; y++) for (let x = 0; x < this.w; x++) if (this.get(x, y)) { x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y); }
    return x1 < 0 ? null : { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
  }
  count() { let n = 0; for (const v of this.data) if (v) n++; return n; }
  colors() { return [...new Set(this.data)].filter(Boolean).sort((a, b) => a - b); }
  hex(v) { return this.palette[v - 1]; }
  luma(v) { return luma(this.hex(v)); }

  /** RGBA bytes, optionally scaled by an integer factor (nearest neighbour) and laid over a background colour. */
  toRgba(scale = 1, bg = null) {
    const w = this.w * scale, out = new Uint8Array(w * this.h * scale * 4);
    const rgb = (hex) => { const n = parseInt(hex.slice(1), 16); return [n >> 16, (n >> 8) & 255, n & 255, 255]; };
    const back = bg ? rgb(bg) : [0, 0, 0, 0];
    for (let y = 0; y < this.h * scale; y++) for (let x = 0; x < w; x++) {
      const v = this.data[Math.floor(y / scale) * this.w + Math.floor(x / scale)];
      out.set(v ? rgb(this.palette[v - 1]) : back, (y * w + x) * 4);
    }
    return out;
  }
}

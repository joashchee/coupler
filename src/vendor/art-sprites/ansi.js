// ANSI files: cells to bytes (`writeAnsi`) and bytes to cells (`readAnsi`), the SAUCE record, CP437, a spans form for a page that
// colours the text itself (`toSpans`) and a plain-text fallback (`toPlainText`). No Node built-ins: it runs in a page, a worker, Node
// or Deno (TextEncoder and TextDecoder are globals). The rules it keeps are the file format's, from docs/ansi-expansion-plan.md:
// reset first and an escape only when attributes change, CRLF and never a bare LF, trailing spaces elided only over no background,
// plain forward printing, Ctrl-Z then SAUCE, 8 backgrounds unless iCE, and a warning, never silence, for what had to change.
import { Cells, NONE, BOLD, BLINK, legalize } from './cells.js';
import { nearest, indexHex, rgbOf } from './palette-ansi.js';

// IBM code page 437, byte to Unicode. Bytes 0 to 31 are the picture characters a DOS screen showed (0 is taken as a space); the
// control bytes among them (tab, line feed, carriage return, Ctrl-Z, escape) keep their meaning in a file and are never written as glyphs.
const LOW = '\0☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼';
const HIGH = 'ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ';
/** The 256 code points of CP437, indexed by byte. */
export const CP437 = Object.freeze([...LOW, ...Array.from({ length: 95 }, (_, i) => String.fromCharCode(0x20 + i)), '⌂', ...HIGH].map((c) => c.codePointAt(0)));
const CONTROL = new Set([0x00, 0x07, 0x08, 0x09, 0x0a, 0x0d, 0x1a, 0x1b]);   // bytes a file reader acts on: never written as glyphs
const TO437 = new Map(CP437.flatMap((cp, b) => (b && !CONTROL.has(b) ? [[cp, b]] : [])));
const IN_CP437 = new Set(CP437);

const ESC = 0x1b, SUB = 0x1a;
/** Reading caps, as Coupler's reader has them: a file wider than 400 columns or taller than 1000 rows is cut, with a warning. */
const MAX_COLUMNS = 400, MAX_ROWS = 1000;
const TARGETS = ['classic', 'ice', 'modern'], DEPTHS = [16, 256, 'true'];

const bright = (c) => (c === NONE ? 15 : c < 8 ? c + 8 : c);

// A copy with BOLD folded into the foreground (a bold colour 0 to 7 is its bright twin) and, when asked, colours above 15 reduced.
function prepare(cells, reduce16) {
  const c = cells.clone(); let reduced = 0;
  for (let i = 0; i < c.ch.length; i++) {
    if (c.flags[i] & BOLD) { c.fg[i] = bright(c.fg[i]); c.flags[i] &= ~BOLD; }
    if (!reduce16) continue;
    for (const a of [c.fg, c.bg]) if (a[i] !== NONE && a[i] > 15) { a[i] = nearest(indexHex(a[i]), 16).index; reduced++; }
  }
  return { cells: c, reduced };
}

// The cells of a row as runs of one style. A space shows no foreground and a full block no background, so those take the style of
// the cell before and do not split a run. Trailing empty cells (no background) are dropped.
function rowRuns(c, y, x0 = 0, x1 = c.w, keepTrail = false) {
  let end = x1;
  while (!keepTrail && end > x0) { const i = y * c.w + end - 1; if ((c.ch[i] === 0 || c.ch[i] === 0x20) && c.bg[i] === NONE) end--; else break; }
  const runs = []; let fg = NONE, bg = NONE;
  for (let x = x0; x < end; x++) {
    const i = y * c.w + x, ch = c.ch[i] || 0x20;
    if (ch !== 0x20) fg = c.fg[i]; if (ch !== 0x2588) bg = c.bg[i];
    const flags = c.flags[i], last = runs[runs.length - 1];
    if (last && last.fg === fg && last.bg === bg && last.flags === flags) last.cps.push(ch); else runs.push({ fg, bg, flags, cps: [ch] });
  }
  return runs;
}

// The SGR parameters that set a style from the reset state: reset, blink, background, bold for a bright foreground, foreground.
function sgr({ fg, bg, flags }, target, colors) {
  const p = [0]; let blink = !!(flags & BLINK);
  const rgb = (n, base) => { const [r, g, b] = rgbOf(indexHex(n)); return [base, 2, r, g, b]; };
  const back = [];
  if (bg !== NONE) {
    if (bg < 8) back.push(40 + bg);
    else if (bg < 16) { if (target === 'ice') { blink = true; back.push(40 + bg - 8); } else back.push(100 + bg - 8); }
    else back.push(...(colors === 'true' ? rgb(bg, 48) : [48, 5, bg]));
  }
  if (blink) p.push(5);
  p.push(...back);
  if (fg !== NONE) {
    if (fg < 8) p.push(30 + fg);
    else if (fg < 16) p.push(...(target === 'modern' ? [90 + fg - 8] : [1, 30 + fg - 8]));
    else p.push(...(colors === 'true' ? rgb(fg, 38) : [38, 5, fg]));
  }
  return p.join(';');
}

const text = (cps) => cps.map((cp) => String.fromCodePoint(cp)).join('');
const names = (set) => { const l = [...set].slice(0, 8).map((cp) => `${String.fromCodePoint(cp)} (U+${cp.toString(16).toUpperCase().padStart(4, '0')})`); return l.join(', ') + (set.size > 8 ? ', ...' : ''); };

/**
 * The byte writer under `writeAnsi` and `writeAnsimation`: `row(work, y, x0, x1, keepTrail)` appends the cells x0 to x1 of a row as
 * runs of SGR and glyphs (ending in a reset), `raw(string)` appends escape text, `bytes` is what has been written and `notes()` lists
 * the characters that could not be held. `keepTrail` writes empty cells at the end as spaces, which a delta needs to erase.
 */
export function makeEncoder({ target, charset, colors }) {
  const bytes = [], lost = new Set(), outside = new Set();
  const raw = (s) => { for (let i = 0; i < s.length; i++) bytes.push(s.charCodeAt(i)); };
  const glyphs = (cps) => {
    for (const cp of cps) if (!IN_CP437.has(cp)) outside.add(cp);
    if (charset === 'utf8') { bytes.push(...new TextEncoder().encode(text(cps))); return; }
    for (const cp of cps) { const b = TO437.get(cp); if (b === undefined) { lost.add(cp); bytes.push(0x3f); } else bytes.push(b); }
  };
  const row = (work, y, x0 = 0, x1 = work.w, keepTrail = false) => {
    let state = '0';
    for (const run of rowRuns(work, y, x0, x1, keepTrail)) {
      const code = sgr(run, target, colors);
      if (code !== state) { raw(`\x1b[${code}m`); state = code; }
      glyphs(run.cps);
    }
    if (state !== '0') raw('\x1b[0m');
  };
  const notes = () => {
    const out = [], control = new Set([...lost].filter((cp) => IN_CP437.has(cp)));
    if (control.size) out.push(`written as "?": these are control bytes in a file, not glyphs: ${names(control)}`);
    if (outside.size) out.push(charset === 'cp437' ? `written as "?": not in CP437: ${names(outside)}` : `not in CP437, so not in the IBM VGA font: ${names(outside)}`);
    return out;
  };
  return { bytes, raw, row, notes };
}

/**
 * Cells to the bytes of an ANSI file.
 *
 * `target` is `classic` (default: CP437 bytes and 8 backgrounds, the way a BBS saw a file; brighter backgrounds are made legal first),
 * `ice` (bright backgrounds as the blink bit, and the SAUCE iCE flag set only if the picture uses one) or `modern` (UTF-8, bright
 * backgrounds as `100` to `107`, bright foregrounds as `90` to `97`, no SAUCE unless asked). `charset` (`cp437` or `utf8`) and
 * `colors` (`16`, `256`, `true`) override the target's defaults; colours above 15 are reduced to the 16 with a warning at depth 16.
 * `sauce` is false, true, or `{ title, author, group, date: 'YYYYMMDD', font, comments, letterSpacing, aspect }`. The date is never
 * read from a clock, so the same cells give the same bytes. Returns `{ bytes, text?, charset, target, colors, ice, sauce, width,
 * height, warnings }`; `text` (the stream alone) is there for UTF-8.
 */
export function writeAnsi(cells, { target = 'classic', charset, colors = 16, sauce } = {}) {
  if (!TARGETS.includes(target)) throw new Error(`unknown target "${target}"; known: ${TARGETS.join(', ')}`);
  if (!DEPTHS.includes(colors)) throw new Error(`unknown colour depth "${colors}"; known: 16, 256, true`);
  charset ??= target === 'modern' ? 'utf8' : 'cp437';
  if (charset !== 'cp437' && charset !== 'utf8') throw new Error(`unknown charset "${charset}"; known: cp437, utf8`);
  const warnings = [], useSauce = sauce === undefined ? target !== 'modern' : !!sauce;

  const prep = prepare(cells, colors === 16);
  if (prep.reduced) warnings.push(`${prep.reduced} colour${prep.reduced === 1 ? '' : 's'} above 15 reduced to the nearest of the 16`);
  let work = legalize(prep.cells, { bgLimit: target === 'classic' ? 8 : 16 });
  if (target === 'ice' && work.flags.some((f) => f & BLINK)) { for (let i = 0; i < work.flags.length; i++) work.flags[i] &= ~BLINK; warnings.push('blinking cells dropped: in an iCE file the blink bit means a bright background'); }
  const ice = target === 'ice' && work.bg.some((b) => b >= 8 && b < 16);

  const enc = makeEncoder({ target, charset, colors });
  for (let y = 0; y < work.h; y++) { enc.row(work, y); enc.raw('\r\n'); }
  warnings.push(...enc.notes());

  const body = Uint8Array.from(enc.bytes), result = { charset, target, colors, ice, sauce: useSauce, width: work.w, height: work.h, warnings };
  if (charset === 'utf8') result.text = new TextDecoder().decode(body);
  if (!useSauce) return { bytes: body, ...result };
  const sealed = withSauce(body, typeof sauce === 'object' ? sauce : {}, work.w, work.h, ice);
  warnings.push(...sealed.notes);
  return { bytes: sealed.bytes, ...result };
}

/** The picture bytes, then Ctrl-Z, the comment block and the SAUCE record. `fileType` is 1 for a still (ANSi) and 2 for an ANSImation. */
export function withSauce(body, opts, w, h, ice, fileType = 1) {
  const { record, comments, notes } = buildSauce(opts, w, h, body.length, ice, fileType);
  const out = new Uint8Array(body.length + 1 + comments.length + record.length);
  out.set(body); out[body.length] = SUB; out.set(comments, body.length + 1); out.set(record, body.length + 1 + comments.length);
  return { bytes: out, notes };
}

const SPACING = { none: 0, '8': 1, '9': 2 }, ASPECT = { none: 0, legacy: 1, square: 2 };
const SAUCE_FONT = 'IBM VGA';

// A string as CP437 bytes padded to `len` with `pad` (cut if longer).
function field(s, len, pad = 0x20) {
  const out = new Uint8Array(len).fill(pad);
  [...String(s ?? '')].slice(0, len).forEach((ch, i) => { out[i] = TO437.get(ch.codePointAt(0)) ?? 0x3f; });
  return out;
}

function buildSauce({ title, author, group, date = '00000000', font = SAUCE_FONT, comments = [], letterSpacing = '8', aspect = 'none' }, w, h, size, ice, fileType = 1) {
  if (!/^\d{8}$/.test(date)) throw new Error(`SAUCE date is YYYYMMDD, not "${date}"`);
  if (!(letterSpacing in SPACING)) throw new Error(`unknown letterSpacing "${letterSpacing}"; known: none, 8, 9`);
  if (!(aspect in ASPECT)) throw new Error(`unknown aspect "${aspect}"; known: none, legacy, square`);
  if (comments.length > 255) throw new RangeError(`SAUCE holds at most 255 comment lines, not ${comments.length}`);
  const notes = [], cut = (s, n, what) => { if ([...String(s ?? '')].length > n) notes.push(`SAUCE ${what} cut to ${n} characters`); };
  cut(title, 35, 'title'); cut(author, 20, 'author'); cut(group, 20, 'group'); cut(font, 22, 'font name');
  comments.forEach((c) => cut(c, 64, 'comment line'));
  const rec = new Uint8Array(128), dv = new DataView(rec.buffer);
  rec.set(field('SAUCE00', 7), 0); rec.set(field(title, 35), 7); rec.set(field(author, 20), 42); rec.set(field(group, 20), 62); rec.set(field(date, 8), 82);
  dv.setUint32(90, size, true); rec[94] = 1; rec[95] = fileType;                // character data; ANSi (1) or ANSiMation (2)
  dv.setUint16(96, Math.min(w, 0xffff), true); dv.setUint16(98, Math.min(h, 0xffff), true);
  rec[104] = comments.length; rec[105] = (ice ? 1 : 0) | (SPACING[letterSpacing] << 1) | (ASPECT[aspect] << 3);
  rec.set(field(font, 22, 0), 106);
  const block = new Uint8Array(comments.length ? 5 + 64 * comments.length : 0);
  if (comments.length) { block.set(field('COMNT', 5), 0); comments.forEach((c, i) => block.set(field(c, 64), 5 + 64 * i)); }
  return { record: rec, comments: block, notes };
}

const str437 = (b) => String.fromCodePoint(...Array.from(b, (v) => CP437[v] || 0x20)).replace(/[\0 ]+$/, '');
const ascii = (b, at, n) => String.fromCharCode(...b.subarray(at, at + n));

/** The SAUCE record at the end of a file: `{ title, author, group, date, width, height, ice, letterSpacing, aspect, font, comments,
 *  dataType, fileType, fileSize }`, plus where the picture data ends. `null` if the bytes carry no SAUCE. */
function parseSauce(b) {
  if (b.length < 128 || ascii(b, b.length - 128, 7) !== 'SAUCE00') return null;
  const r = b.subarray(b.length - 128), dv = new DataView(r.buffer, r.byteOffset, 128), n = r[104];
  let end = b.length - 128; const comments = [];
  if (n && end >= 5 + 64 * n && ascii(b, end - 5 - 64 * n, 5) === 'COMNT') {
    end -= 5 + 64 * n;
    for (let i = 0; i < n; i++) comments.push(str437(b.subarray(end + 5 + 64 * i, end + 5 + 64 * (i + 1))));
  }
  const flags = r[105], spacing = (flags >> 1) & 3, aspect = (flags >> 3) & 3;
  return {
    sauce: {
      title: str437(r.subarray(7, 42)), author: str437(r.subarray(42, 62)), group: str437(r.subarray(62, 82)), date: ascii(r, 82, 8),
      dataType: r[94], fileType: r[95], fileSize: dv.getUint32(90, true), width: dv.getUint16(96, true), height: dv.getUint16(98, true),
      ice: !!(flags & 1), letterSpacing: ['none', '8', '9', 'none'][spacing], aspect: ['none', 'legacy', 'square', 'none'][aspect],
      font: str437(r.subarray(106, 128)).replace(/\0+$/, ''), comments,
    }, end,
  };
}

const decode437 = (b) => Array.from(b, (v) => (v === 0 ? 0x20 : v === 0x09 || v === 0x0a || v === 0x0d || v === ESC ? v : CP437[v]));

/**
 * The bytes (or string) of an ANSI file to cells. Understands SGR (16, 256 and truecolor, bold, blink, reverse), cursor moves
 * (`A B C D G H f s u`), `J` (clear) and `K`, wraps at the width (after the last column, as a terminal does), and ignores what it
 * does not know. Stops at Ctrl-Z; SAUCE and its comments are read, not drawn. Width is SAUCE's, else `width`, else 80; `charset` is
 * `auto` (UTF-8 if the bytes are valid UTF-8 with non-ASCII in them, else CP437), `cp437` or `utf8`; `ice` overrides SAUCE's flag
 * (in iCE the blink bit is a bright background). Colours the engine cannot hold (truecolor) are reduced to the 256, with a warning.
 * Returns `{ cells, width, height, sauce, charset, ice, warnings }`. Bold folds into the bright colour.
 */
export function readAnsi(input, { width, charset = 'auto', ice } = {}) {
  const warnings = [];
  let sauce = null, cps, used;
  if (typeof input === 'string') { cps = Array.from(input, (c) => c.codePointAt(0)); used = 'utf8'; }
  else {
    const found = parseSauce(input); sauce = found?.sauce ?? null;
    let data = input.subarray(0, found ? found.end : input.length);
    const stop = data.indexOf(SUB); if (stop >= 0) data = data.subarray(0, stop);
    used = charset;
    if (charset === 'auto') {
      used = 'cp437';
      if (data.some((v) => v >= 0x80)) { try { new TextDecoder('utf-8', { fatal: true }).decode(data); used = 'utf8'; } catch { /* not UTF-8 */ } }
    }
    if (used !== 'cp437' && used !== 'utf8') throw new Error(`unknown charset "${charset}"; known: auto, cp437, utf8`);
    cps = used === 'utf8' ? Array.from(new TextDecoder().decode(data), (c) => c.codePointAt(0)) : decode437(data);
  }
  const wide = sauce && sauce.dataType === 1 && sauce.fileType <= 2 && sauce.width > 0 ? sauce.width : width ?? 80;
  const cols = Math.max(1, Math.min(MAX_COLUMNS, wide));
  if (wide > MAX_COLUMNS) warnings.push(`width ${wide} cut to ${MAX_COLUMNS} columns`);
  const isIce = ice ?? sauce?.ice ?? false;

  let grid = new Cells(cols, Math.max(25, Math.min(MAX_ROWS, sauce?.height ?? 0)));
  const grow = (y) => { let h = grid.h; while (h <= y) h = Math.min(MAX_ROWS, h * 2); const g = new Cells(cols, h); g.ch.set(grid.ch); g.fg.set(grid.fg); g.bg.set(grid.bg); g.flags.set(grid.flags); grid = g; };
  let x = 0, y = 0, fg = NONE, bg = NONE, bold = false, blink = false, reverse = false, saved = [0, 0], maxY = -1, lines = 0, cut = false, deep = false;
  const place = (cp) => {
    if (x >= cols) { x = 0; y++; }
    if (y >= MAX_ROWS) { cut = true; return; }
    if (y >= grid.h) grow(y);
    let f = bold ? bright(fg) : fg, b = bg, fl = 0;
    if (reverse) [f, b] = [b === NONE ? 0 : b, f === NONE ? 7 : f];
    if (blink) { if (isIce) b = b === NONE ? 8 : b < 8 ? b + 8 : b; else fl = BLINK; }
    grid.set(x, y, { ch: cp, fg: f, bg: b, flags: fl }); maxY = Math.max(maxY, y); x++;
  };
  const sgrApply = (p) => {
    for (let k = 0; k < p.length; k++) {
      const c = p[k];
      if (c === 0) { fg = bg = NONE; bold = blink = reverse = false; }
      else if (c === 1) bold = true; else if (c === 22) bold = false;
      else if (c === 5 || c === 6) blink = true; else if (c === 25) blink = false;
      else if (c === 7) reverse = true; else if (c === 27) reverse = false;
      else if (c >= 30 && c <= 37) fg = c - 30; else if (c === 39) fg = NONE;
      else if (c >= 40 && c <= 47) bg = c - 40; else if (c === 49) bg = NONE;
      else if (c >= 90 && c <= 97) fg = c - 82; else if (c >= 100 && c <= 107) bg = c - 92;
      else if (c === 38 || c === 48) {
        let col = null;
        if (p[k + 1] === 5) { if (p[k + 2] >= 0 && p[k + 2] <= 255) col = p[k + 2]; k += 2; }
        else if (p[k + 1] === 2) {
          const rgb = p.slice(k + 2, k + 5); k += 4;
          if (rgb.length === 3 && rgb.every((v) => v >= 0 && v <= 255)) { col = nearest('#' + rgb.map((v) => v.toString(16).padStart(2, '0')).join(''), 256).index; deep = true; }
        }
        if (col !== null) { if (c === 38) fg = col; else bg = col; }
      }
    }
  };
  for (let i = 0; i < cps.length; i++) {
    const cp = cps[i];
    if (cp === ESC) {
      if (cps[i + 1] !== 0x5b) continue;                                          // a lone escape is dropped
      let j = i + 2, params = '';
      while (j < cps.length && cps[j] >= 0x30 && cps[j] <= 0x3f && j - i < 64) params += String.fromCharCode(cps[j++]);
      while (j < cps.length && cps[j] >= 0x20 && cps[j] <= 0x2f && j - i < 64) j++;
      const fin = cps[j]; i = j;
      if (!(fin >= 0x40 && fin <= 0x7e) || /[^0-9;]/.test(params)) continue;      // private (?) or malformed sequences are skipped
      const p = params === '' ? [] : params.split(';').map((s) => (s === '' ? 0 : parseInt(s, 10))), n = p[0] || 1;
      const at = Math.min(x, cols - 1);
      switch (String.fromCharCode(fin)) {
        case 'm': sgrApply(p.length ? p : [0]); break;
        case 'A': y = Math.max(0, y - n); x = at; break;
        case 'B': y = Math.min(MAX_ROWS - 1, y + n); x = at; break;
        case 'C': x = Math.min(cols - 1, at + n); break;
        case 'D': x = Math.max(0, at - n); break;
        case 'G': x = Math.min(cols - 1, n - 1); break;
        case 'H': case 'f': y = Math.min(MAX_ROWS - 1, (p[0] || 1) - 1); x = Math.min(cols - 1, (p[1] || 1) - 1); break;
        case 's': saved = [at, y]; break;
        case 'u': [x, y] = saved; break;
        case 'J': if (p[0] === 2) { grid = new Cells(cols, grid.h); x = y = 0; maxY = -1; } break;
        case 'K': if (y < MAX_ROWS) { if (y >= grid.h) grow(y); for (let k = p[0] === 2 ? 0 : at; k < cols; k++) grid.set(k, y, { ch: 0x20, fg: NONE, bg, flags: 0 }); maxY = Math.max(maxY, y); } break;
        default: break;
      }
    } else if (cp === 0x0d) x = 0;
    else if (cp === 0x0a) { x = 0; y++; lines = Math.max(lines, y); }
    else if (cp === 0x09) x = Math.min(cols - 1, (Math.floor(Math.min(x, cols - 1) / 8) + 1) * 8);
    else place(cp);
  }
  if (cut) warnings.push(`rows past ${MAX_ROWS} dropped`);
  if (deep) warnings.push('truecolor reduced to the 256-colour set');
  const rows = Math.min(MAX_ROWS, Math.max(maxY + 1, lines, sauce?.height ?? 0));
  const cells = new Cells(cols, rows);
  for (const k of ['ch', 'fg', 'bg', 'flags']) cells[k].set(grid[k].subarray(0, cols * rows));
  return { cells, width: cols, height: rows, sauce, charset: used, ice: isIce, warnings };
}

/**
 * The cells as spans, the shape a page or app that colours the text itself wants: `{ columns, rows, lines }`, a line being a list of
 * `{ text, fg, bg, blink }` runs of one style (`fg` and `bg` palette indexes or null; bold is folded into the bright colour).
 * Trailing empty cells are left out, so an empty row is `[]`.
 */
export function toSpans(cells) {
  const c = prepare(cells, false).cells;
  return {
    columns: c.w, rows: c.h,
    lines: Array.from({ length: c.h }, (_, y) => rowRuns(c, y).map((r) => ({ text: text(r.cps), fg: r.fg === NONE ? null : r.fg, bg: r.bg === NONE ? null : r.bg, blink: !!(r.flags & BLINK) }))),
  };
}

/** The characters alone, a string a row (`\n` between rows, trailing spaces cut): the fallback beside a `.ans` for hosts that strip
 *  SAUCE and for plain terminals. UTF-8; characters outside CP437 are kept as they are. */
export function toPlainText(cells) {
  return cells.toText().split('\n').map((l) => l.replace(/ +$/, '')).join('\n');
}

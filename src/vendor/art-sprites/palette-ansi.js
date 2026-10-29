// The ANSI palettes: the 16 VGA colours, the 256-colour xterm set built on them, and how to reduce a colour to either. Cells keep
// palette INDEXES (0 to 255), never RGB: which RGB an index looks like is the viewer's call, so a theme can change it (Coupler's rule).
// Index order is the ANSI one (SGR 30 to 37, then the bright 90 to 97), not the old DOS one: 1 is red and 4 is blue.
export const VGA16 = [
  '#000000', '#aa0000', '#00aa00', '#aa5500', '#0000aa', '#aa00aa', '#00aaaa', '#aaaaaa',
  '#555555', '#ff5555', '#55ff55', '#ffff55', '#5555ff', '#ff55ff', '#55ffff', '#ffffff',
];
const VGA_NAMES = ['black', 'red', 'green', 'brown', 'blue', 'magenta', 'cyan', 'gray', 'darkgray', 'brightred', 'brightgreen', 'yellow', 'brightblue', 'brightmagenta', 'brightcyan', 'white'];
/** Named VGA indexes: V.red, V.yellow, ... */
export const V = Object.freeze(Object.fromEntries(VGA_NAMES.map((n, i) => [n, i])));

const CUBE = [0, 95, 135, 175, 215, 255];
const hex2 = (n) => n.toString(16).padStart(2, '0');
/** The 256-colour set: 0 to 15 the VGA colours, 16 to 231 a 6x6x6 cube, 232 to 255 a gray ramp. */
export const XTERM256 = Object.freeze([
  ...VGA16,
  ...Array.from({ length: 216 }, (_, i) => '#' + [Math.floor(i / 36), Math.floor(i / 6) % 6, i % 6].map((k) => hex2(CUBE[k])).join('')),
  ...Array.from({ length: 24 }, (_, i) => '#' + hex2(8 + 10 * i).repeat(3)),
]);

/**
 * Where each DB32 colour goes in the 16 (index = DB32 index, value = VGA16 index). Written by hand, not by nearest colour, which
 * sends a dozen DB32 colours to dark gray: the ramps keep their order and their hue where 16 colours allow (greens stay green, the
 * fire ramp runs red, yellow, white, steel runs dark gray to white), and some neighbours share a colour because 16 is fewer than 32.
 */
export const DB32_VGA = Object.freeze([0, 0, 0, 3, 3, 11, 7, 15, 11, 10, 2, 6, 2, 3, 8, 4, 6, 12, 12, 14, 15, 15, 7, 7, 8, 8, 5, 1, 9, 13, 10, 3]);

export const rgbOf = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
// "Redmean" distance: close to how the eye weighs the channels, and cheap.
export const distance = (a, b) => { const m = (a[0] + b[0]) / 2, r = a[0] - b[0], g = a[1] - b[1], bl = a[2] - b[2]; return (2 + m / 256) * r * r + 4 * g * g + (2 + (255 - m) / 256) * bl * bl; };
// (`rgbOf` gives [r, g, b]; `distance` compares two such colours, fractional ones too.)
const RGB16 = VGA16.map(rgbOf), RGB256 = XTERM256.map(rgbOf);

/**
 * Reduce a colour (`#rrggbb`) to a palette index at a depth: 16 or 256 give the nearest index (the lowest one on a tie, so it is
 * deterministic); 'true' keeps the colour. Returns `{ index }` or `{ hex }`.
 */
export function nearest(hex, depth = 16) {
  if (depth === 'true') return { hex };
  const set = depth === 16 ? RGB16 : depth === 256 ? RGB256 : null;
  if (!set) throw new Error(`unknown colour depth "${depth}"; known: 16, 256, true`);
  const c = rgbOf(hex); let best = 0, bd = Infinity;
  for (let i = 0; i < set.length; i++) { const d = distance(c, set[i]); if (d < bd) { bd = d; best = i; } }
  return { index: best };
}

/** The hex of a palette index (0 to 255), for rendering and for tests. */
export function indexHex(index) { const h = XTERM256[index]; if (!h) throw new Error(`no colour ${index}`); return h; }

/**
 * Shade ramps by hand, dark to light, as VGA indexes. Each step to the next can be made with `ladder` at 8 backgrounds: one colour
 * of every pair is a legal background (0 to 7), which is why none runs dark gray straight to white (both are bright) and why the
 * blue ramp stops at light cyan: a white highlight is placed by hand. From the kit's two starter palettes (Classic BBS: white and
 * cyan highlights, magenta accent, yellow warm, red emphasis, green nature, blue cool shadow; Low-contrast study) and its rule that
 * dark gray is the main shadow colour and shades up through gray.
 */
export const ANSI_RAMPS = Object.freeze(Object.fromEntries(Object.entries({
  gray: [V.black, V.darkgray, V.gray, V.white],
  blue: [V.black, V.blue, V.brightblue, V.cyan, V.brightcyan],
  fire: [V.black, V.red, V.brightred, V.brown, V.yellow],
  green: [V.black, V.darkgray, V.green, V.brightgreen],
  magenta: [V.black, V.magenta, V.brightmagenta, V.gray, V.white],
}).map(([k, r]) => [k, Object.freeze(r)])));

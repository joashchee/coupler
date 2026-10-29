// The palette and the colour ramps every recipe draws from. DB32 (DawnBringer 32) is the palette the shipped 16x16 item pack
// already uses, so drawn art sits beside it. A colour value in a Pixels canvas is 0 (transparent) or 1 + the DB32 index.
export const DB32 = [
  '#000000', '#222034', '#45283c', '#663931', '#8f563b', '#df7126', '#d9a066', '#eec39a', '#fbf236', '#99e550', '#6abe30',
  '#37946e', '#4b692f', '#524b24', '#323c39', '#3f3f74', '#306082', '#5b6ee1', '#639bff', '#5fcde4', '#cbdbfc', '#ffffff',
  '#9badb7', '#847e87', '#696a6a', '#595652', '#76428a', '#ac3232', '#d95763', '#d77bba', '#8f974a', '#8a6f30',
];
const NAMES = ['black', 'night', 'plum', 'umber', 'brown', 'orange', 'tan', 'peach', 'yellow', 'lime', 'green', 'teal', 'forest', 'bark', 'slate', 'indigo',
  'steel', 'blue', 'sky', 'cyan', 'ice', 'white', 'mist', 'grey', 'dust', 'ash', 'violet', 'red', 'rose', 'pink', 'moss', 'bronze'];
/** Named colour values: C.night, C.red, ... */
export const C = Object.freeze(Object.fromEntries(NAMES.map((n, i) => [n, i + 1])));

/**
 * Four-step ramps, dark to light: shadow, shade, base, light. Hue shifts along each ramp (shadows lean cool, lights warm or pale)
 * rather than the same colour darker and lighter, as the learning kit teaches.
 */
export const RAMPS = Object.freeze({
  steel: [C.slate, C.dust, C.mist, C.ice], iron: [C.night, C.ash, C.dust, C.grey], wood: [C.plum, C.umber, C.brown, C.tan],
  leather: [C.plum, C.umber, C.brown, C.orange], gold: [C.umber, C.bronze, C.orange, C.yellow], red: [C.plum, C.red, C.rose, C.pink],
  blue: [C.night, C.indigo, C.blue, C.sky], green: [C.night, C.forest, C.green, C.lime], purple: [C.night, C.indigo, C.violet, C.pink],
  white: [C.grey, C.mist, C.ice, C.white], bone: [C.dust, C.tan, C.peach, C.white], glass: [C.steel, C.sky, C.cyan, C.ice],
  fire: [C.red, C.orange, C.yellow, C.white], stone: [C.slate, C.ash, C.dust, C.grey], grass: [C.forest, C.green, C.lime, C.moss],
  water: [C.indigo, C.steel, C.sky, C.cyan], sand: [C.bronze, C.tan, C.peach, C.yellow], dirt: [C.plum, C.umber, C.brown, C.orange],
  moss: [C.bark, C.forest, C.moss, C.lime], ice: [C.indigo, C.sky, C.ice, C.white],
  'skin-fair': [C.brown, C.tan, C.peach, C.ice], 'skin-light': [C.umber, C.brown, C.tan, C.peach], 'skin-medium': [C.plum, C.umber, C.brown, C.tan],
  'skin-deep': [C.night, C.plum, C.umber, C.brown], 'skin-green': [C.forest, C.green, C.lime, C.moss], 'skin-pale': [C.dust, C.grey, C.mist, C.ice],
  'skin-red': [C.plum, C.red, C.rose, C.pink], 'skin-blue': [C.indigo, C.blue, C.sky, C.ice],
});
export const ramp = (name) => { const r = RAMPS[name]; if (!r) throw new Error(`unknown ramp "${name}"`); return r; };

/** Value (0 to 255) of a colour, to judge contrast without caring about hue. */
export function luma(hex) {
  const n = parseInt(hex.slice(1), 16);
  return 0.299 * (n >> 16) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255);
}

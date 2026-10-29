// A picture of a place from facts alone, the engine's side of replacing Coupler's room painter (docs/ansi-expansion-plan.md, Phase 7).
// The input is data, not Coupler's types: `{ terrain, time, weather, style, arc, colors, seed, columns, rows }`. Terrain is one of
// CoffeeMUD's 24; a picture shows sky only where there is a sky (outdoors, not underwater); indoors is a room in one-point perspective
// in its material, a cave or a chasm. The pictures are not byte-identical to Coupler's Rust painter and are not meant to be: ideas
// were ported, not code. Drawn on a half-block pixel canvas (`vista-canvas.js`) and written as cells, legal for a classic file unless `bgLimit: 16` asks for the full picture (bright backgrounds, for iCE and modern files). Browser-safe.
import { legalize } from '../cells.js';
import { resolveTones } from '../ansi-depth.js';
import { drawOutdoors } from './vista-outdoors.js';
import { drawIndoors } from './vista-indoors.js';

export const OUTDOOR = ['city', 'woods', 'rocky', 'plains', 'underwater', 'air', 'watersurface', 'jungle', 'swamp', 'desert', 'hills', 'mountains', 'spaceport', 'seaport'];
export const INDOOR = ['stone', 'wooden', 'cave', 'magic', 'in_underwater', 'gap', 'cavelakesurface', 'metal', 'innerseaport', 'caveseaport'];
export const TERRAINS = [...OUTDOOR, ...INDOOR];
export const TIMES = ['dawn', 'day', 'dusk', 'night'];
export const WEATHER_KINDS = ['clear', 'cloudy', 'windy', 'rain', 'thunderstorm', 'snow', 'blizzard', 'hail', 'sleet', 'fog', 'dust', 'heat', 'drought', 'cold'];
export const VISTA_STYLES = ['fantasy', 'high'];
/** Whether a terrain has a sky: outdoors and not underwater (CoffeeMUD's `hasASky`). */
export const hasSky = (terrain) => OUTDOOR.includes(terrain) && terrain !== 'underwater';

/**
 * A room's seed as Coupler keeps it: FNV-1a (64 bit) of the world, a zero byte, the room ID and the look count as four little-endian
 * bytes, returned as a decimal string for the `seed` param. Same inputs, same look, from visit to visit; another `again` is another look.
 */
export function roomSeed(world, room, again = 0) {
  let h = 0xcbf29ce484222325n;
  const bytes = [...new TextEncoder().encode(String(world)), 0, ...new TextEncoder().encode(String(room)), again & 255, (again >>> 8) & 255, (again >>> 16) & 255, (again >>> 24) & 255];
  for (const b of bytes) { h ^= BigInt(b); h = (h * 0x100000001b3n) & 0xffffffffffffffffn; }
  return h.toString();
}

const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
const VGA_ORDER = Array.from({ length: 256 }, (_, i) => i);

/** A picture of a place: sky by time and weather where there is one, land by terrain, or a room in its material. */
export function drawVista({ terrain = 'plains', time = 'day', weather = 'clear', style = 'fantasy', arc, colors = 16, seed = 'vista', columns = 80, rows = 24, bgLimit = 8 } = {}) {
  if (!TERRAINS.includes(terrain)) throw new Error(`unknown terrain "${terrain}"; known: ${TERRAINS.join(', ')}`);
  if (!TIMES.includes(time)) throw new Error(`unknown time "${time}"; known: ${TIMES.join(', ')}`);
  if (!WEATHER_KINDS.includes(weather)) throw new Error(`unknown weather "${weather}"; known: ${WEATHER_KINDS.join(', ')}`);
  if (!VISTA_STYLES.includes(style)) throw new Error(`unknown style "${style}"; known: ${VISTA_STYLES.join(', ')}`);
  if (colors !== 16 && colors !== 256) throw new Error(`colors is 16 or 256, not ${colors}`);
  const w = clamp(Math.round(columns), 8, 160), h = clamp(Math.round(rows), 4, 45), high = style === 'high';
  const shown = hasSky(terrain) ? { time, weather } : { time: 'day', weather: 'clear' };     // no sky indoors or underwater: nothing of it shows
  const picture = OUTDOOR.includes(terrain) ? drawOutdoors({ terrain, time: shown.time, weather: shown.weather, high, arc, seed, columns: w, rows: h, map: VGA_ORDER, depth: colors }) : drawIndoors({ terrain, high, seed, columns: w, rows: h, map: VGA_ORDER, depth: colors });
  let out = legalize(picture, { bgLimit });
  if (colors === 256) out = resolveTones(out, { depth: 256 });
  return { id: `vista.${terrain}`, frames: [out], fps: 0, loop: false, says: describe(terrain, shown.time, shown.weather, high) };
}

/** What the picture shows, in words (a picture's facts are always in words beside it). */
function describe(terrain, time, weather, high) {
  const where = INDOOR.includes(terrain) ? `Indoors (${terrain.replace(/_/g, ' ')}).` : terrain === 'underwater' ? 'Underwater.' : `Outdoors: ${terrain}, ${time}${weather === 'clear' ? '' : `, ${weather}`}.`;
  return where + (high ? ' High fantasy: castle towers, banners and braziers where they fit.' : '');
}

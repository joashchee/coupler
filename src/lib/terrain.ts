/**
 * The map's terrain colors: each of CoffeeMUD's 24 terrains (GMCP
 * `room.info.terrain`, listed in docs/coffeemud-gmcp.md) drawn as a tile
 * of one of the 16 VGA colors, so the picture shows woods, water, stone
 * and the rest at a glance.
 *
 * 24 terrains, 15 usable colors (blue is the ANSIapps panel itself), so
 * alike terrains share: built stone, forest, rock, water at the surface,
 * underwater, ports. The text on a tile (the room's brackets and mark)
 * is black or white, whichever passes on that color in
 * docs/ansiapps-color-contrast.md (rule 7); the same pairs serve both
 * themes. Color is never the only cue: each room's terrain is also in
 * words (Where you are, Find a room, the tile's tooltip).
 */

/** The VGA colors a tile can be, and the text that passes on each (ratio from the contrast list). */
const VGA = {
  black: { bg: "#000000", fg: "#ffffff" }, // white on black 21.0
  green: { bg: "#00aa00", fg: "#000000" }, // 6.7
  cyan: { bg: "#00aaaa", fg: "#000000" }, // 7.3
  red: { bg: "#aa0000", fg: "#ffffff" }, // 7.8
  magenta: { bg: "#aa00aa", fg: "#ffffff" }, // 6.4
  brown: { bg: "#aa5500", fg: "#ffffff" }, // 5.2
  lightgray: { bg: "#aaaaaa", fg: "#000000" }, // 9.0
  darkgray: { bg: "#555555", fg: "#ffffff" }, // 7.5
  lightblue: { bg: "#5555ff", fg: "#ffffff" }, // 5.1
  lightgreen: { bg: "#55ff55", fg: "#000000" }, // 15.8
  lightcyan: { bg: "#55ffff", fg: "#000000" }, // 17.1
  lightred: { bg: "#ff5555", fg: "#000000" }, // 6.7
  lightmagenta: { bg: "#ff55ff", fg: "#000000" }, // 8.0
  yellow: { bg: "#ffff55", fg: "#000000" }, // 19.7
  white: { bg: "#ffffff", fg: "#000000" }, // 21.0
} as const;

type Vga = keyof typeof VGA;

/** Every terrain CoffeeMUD sends, outdoors then indoors, and its color. */
export const TERRAIN_COLORS: Record<string, Vga> = {
  // Outdoors
  city: "lightgray",
  woods: "green",
  rocky: "darkgray",
  plains: "lightgreen",
  underwater: "cyan",
  air: "lightcyan",
  watersurface: "lightblue",
  jungle: "green",
  swamp: "magenta",
  desert: "yellow",
  hills: "brown",
  mountains: "darkgray",
  spaceport: "white",
  seaport: "lightred",
  // Indoors
  stone: "lightgray",
  wooden: "red",
  cave: "darkgray",
  magic: "lightmagenta",
  in_underwater: "cyan",
  gap: "black",
  cavelakesurface: "lightblue",
  metal: "white",
  innerseaport: "lightred",
  caveseaport: "lightred",
};

/** How a terrain's word reads: `in_underwater` as "in underwater". */
export const terrainWords = (terrain: string) => terrain.replace(/_/g, " ");

/** A terrain's tile colors, or null for one CoffeeMUD doesn't list (drawn plain). */
export function terrainStyle(terrain: string): { backgroundColor: string; color: string } | null {
  const vga = TERRAIN_COLORS[terrain.toLowerCase()];
  return vga ? { backgroundColor: VGA[vga].bg, color: VGA[vga].fg } : null;
}

/**
 * The Painter engine (`art-sprites`, ansiapps' own, Apache-2.0 like
 * Coupler: src/vendor/art-sprites, copied by scripts/painter-sync.sh from
 * the `painter` repo) drawing a room's picture, offered in gear →
 * Pictures… beside Coupler's own painter (src-tauri/src/painter.rs, the
 * default) and compared with it in gear → Eyetest (dev builds). It ships
 * in the app and the web build, loaded on the first picture it draws.
 *
 * It runs here in the webview (no Rust, no network). Only the game's own
 * words about the room reach it: the terrain, the time, the weather, the
 * sun's place and a seed from the world, the room's ID and its look.
 * Never the room's name, its zone or a player's ART (rule 6,
 * `engineScene`). It never holds back a sound: it loads on the first
 * picture, draws in a few milliseconds, and when it fails the picture is
 * left out and says so in words (rule 12).
 */
import type { AnsiArt, PictureStyle, Scene, Span } from "./mud";
import { palette, type Palette } from "./palette";

/** The part of the engine Coupler uses (src/vendor/art-sprites/engine.js). */
interface Engine {
  drawVista(params: { terrain: string; time: string; weather: string; arc?: number; seed: string; style: PictureStyle; colors: 16 | 256; columns: number; rows: number }): {
    frames: unknown[];
    says: string;
  };
  roomSeed(world: string, room: string, again?: number): string;
  toSpans(cells: unknown): { columns: number; rows: number; lines: { text: string; fg: number | null; bg: number | null; blink: boolean }[][] };
  VERSION: string;
  TERRAINS: string[];
  TIMES: string[];
  WEATHER_KINDS: string[];
}

// A glob, so a checkout without the copy still builds and just has no engine.
const found = import.meta.glob("../vendor/art-sprites/engine.js") as Record<string, () => Promise<Engine>>;

/** Whether this build has the engine's copy. */
export const engineAvailable = Object.keys(found).length > 0;

/** The engine's release, or null without the copy. */
export const engineVersion = async (): Promise<string | null> => {
  const load = Object.values(found)[0];
  return load ? (await load()).VERSION : null;
};

let loading: Promise<Engine> | null = null;
const engine = () => {
  const load = Object.values(found)[0];
  if (!load) return Promise.reject(new Error("the Painter engine isn't in this checkout"));
  return (loading ??= load().catch((e) => {
    loading = null; // try again on the next picture
    throw e;
  }));
};

/** What the engine is told about a scene, and nothing more. */
export interface EngineScene {
  terrain: string;
  time: string;
  weather: string;
  arc?: number;
  seed: string;
}

/** The engine's words for a scene: unknown terrain is plains, unknown weather clear, unknown time day, as Coupler's painter reads them. */
export function engineScene(scene: Scene, e: Pick<Engine, "roomSeed" | "TERRAINS" | "TIMES" | "WEATHER_KINDS">): EngineScene {
  return {
    terrain: e.TERRAINS.includes(scene.terrain) ? scene.terrain : "plains",
    time: scene.time && e.TIMES.includes(scene.time) ? scene.time : "day",
    weather: scene.weather && e.WEATHER_KINDS.includes(scene.weather) ? scene.weather : "clear",
    ...(scene.arc === null ? {} : { arc: scene.arc }),
    seed: e.roomSeed(scene.world, scene.room, scene.look),
  };
}

/**
 * The engine's picture of a room, `columns` by `rows` cells (it clamps
 * them to 8 to 160 by 4 to 45), in the game's colors (or `colors`): the
 * 16, or the 256, which true color shares for now. And its words.
 */
export async function paintRoom(scene: Scene, style: PictureStyle, columns: number, rows: number, colors: Palette = palette()): Promise<{ art: AnsiArt; says: string }> {
  const e = await engine();
  const made = e.drawVista({ ...engineScene(scene, e), style, colors: colors === "16" ? 16 : 256, columns, rows });
  const spans = e.toSpans(made.frames[0]);
  const lines = spans.lines.map((runs) =>
    runs.map<Span>((r) => ({
      text: r.text,
      ...(r.fg === null ? {} : { fg: { kind: "index", index: r.fg } }),
      ...(r.bg === null ? {} : { bg: { kind: "index", index: r.bg } }),
    })),
  );
  return { art: { columns: spans.columns, rows: spans.rows, lines }, says: made.says };
}

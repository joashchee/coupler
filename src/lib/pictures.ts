/**
 * Room pictures' settings (gear → Pictures…, `coupler.pictures`): who
 * makes the picture and in which style, and who makes the race and class
 * portraits the guide to making a character shows. All ART is Coupler's
 * painter's first (CLAUDE.md): anything else is the player's choice.
 * Only Coupler's painter ships
 * (src-tauri/src/painter.rs, instant and the same every visit); the
 * models a player brings will join `PICTURE_ENGINES` once they can run. The picture is never announced and is
 * hidden from screen readers: the room's words are beside it.
 */
import type { PictureStyle, Scene } from "./mud";
import { terrainWords } from "./terrain";

export type PictureEngine = "painter" | "none";

/** When a room's picture shows: in every room, at landmarks only, or only when asked for by key (Cmd+Shift+P). */
export type PictureWhen = "every" | "landmarks" | "key";

/** Who makes the race and class portraits: Coupler's painter (src-tauri/src/portrait.rs) or CoffeeMUD's own pictures. */
export type PortraitSource = "painter" | "coffeemud";

export interface PictureSettings {
  engine: PictureEngine;
  style: PictureStyle;
  when: PictureWhen;
  portraits: PortraitSource;
}

export const PORTRAIT_SOURCES: { id: PortraitSource; name: string; summary: string }[] = [
  { id: "painter", name: "Coupler's painter", summary: "Draws each race at its size beside a six-foot stick, and each class in its clothes with its tools." },
  { id: "coffeemud", name: "CoffeeMUD's pictures", summary: "The game's own portraits from its web pages, made into the same blocky colors." },
];

export const PICTURE_ENGINES: { id: PictureEngine; name: string; summary: string }[] = [
  { id: "painter", name: "Coupler's painter", summary: "Draws each room in characters from its terrain, the time of day and the weather. Instant, and a room looks the same each visit." },
  { id: "none", name: "None", summary: "No pictures." },
];

export const PICTURE_STYLES: { id: PictureStyle; name: string; summary: string }[] = [
  { id: "fantasy", name: "Fantasy", summary: "Plain medieval landscapes and halls." },
  { id: "high", name: "High fantasy", summary: "Adds castles on the horizon, a second moon and auroras at night, banners and braziers indoors." },
];

export const PICTURE_WHENS: { id: PictureWhen; name: string; summary: string }[] = [
  { id: "every", name: "Every room", summary: "A picture of each room as you arrive." },
  { id: "landmarks", name: "Landmarks only", summary: "Only in rooms you've named as landmarks on the map." },
  { id: "key", name: "When I ask", summary: "Only after Cmd+Shift+P, until you leave the room." },
];

const KEY = "coupler.pictures";
const DEFAULT: PictureSettings = { engine: "painter", style: "fantasy", when: "every", portraits: "painter" };

export function loadPictures(): PictureSettings {
  try {
    const kept = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Record<string, unknown>>;
    return {
      engine: PICTURE_ENGINES.some((e) => e.id === kept.engine) ? (kept.engine as PictureEngine) : DEFAULT.engine,
      style: PICTURE_STYLES.some((s) => s.id === kept.style) ? (kept.style as PictureStyle) : DEFAULT.style,
      when: PICTURE_WHENS.some((w) => w.id === kept.when) ? (kept.when as PictureWhen) : DEFAULT.when,
      portraits: PORTRAIT_SOURCES.some((p) => p.id === kept.portraits) ? (kept.portraits as PortraitSource) : DEFAULT.portraits,
    };
  } catch {
    return { ...DEFAULT };
  }
}

export function savePictures(settings: PictureSettings) {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // Not kept past this run, then.
  }
}

/**
 * Whether the room's picture shows now: pictures on, and the room one
 * the "when" setting allows. The player's own ART for a room always
 * shows (pictures on).
 */
export function pictureShows(settings: PictureSettings, scene: Scene | null, landmark: boolean, askedFor: string | null): boolean {
  if (settings.engine === "none" || !scene) return false;
  if (scene.art || settings.when === "every") return true;
  return settings.when === "landmarks" ? landmark : askedFor === scene.room;
}

/** Why there's no picture here, for the place it would be. */
export function noPictureWords(settings: PictureSettings): string {
  if (settings.when === "landmarks") return "Pictures show at landmarks only. Cmd+Shift+P paints this room.";
  return "Cmd+Shift+P paints this room.";
}

/** What the picture shows, in words: "woods at dusk, rain". */
export function sceneWords(scene: Scene): string {
  const time = scene.time ? ` at ${scene.time === "day" ? "midday" : scene.time}` : "";
  const weather = scene.weather && scene.weather !== "clear" ? `, ${scene.weather}` : "";
  return `${terrainWords(scene.terrain) || "somewhere"}${time}${weather}`;
}

/**
 * Terminal's Control Panel (components/ControlPanel.tsx): what shows
 * beside the game output, kept as `coupler.terminalPanels`, and whether
 * it's being arranged, `coupler.terminalArrange`. Where each thing was
 * put is kept with Workshop's (lib/layout.ts) under "terminal:".
 */
import { WEB } from "./web";

/** What the Control Panel can show, in its order. */
export const CONTROLS = [
  { id: "map", name: "Map", about: "the rooms around you and finding your way" },
  { id: "vitals", name: "You", about: "your health, mana and movement as gauges" },
  { id: "picture", name: "Picture", about: "the room painted" },
  { id: "heard", name: "Heard", about: "what Coupler's voice said, written" },
  { id: "journal", name: "Journal button", about: "what's said in the game" },
  { id: "log", name: "Log button", about: "OOC, INFO and the other channels" },
  { id: "hooks", name: "Hooks button", about: "what the game sends behind its text" },
] as const;
export type ControlId = (typeof CONTROLS)[number]["id"];
export type Controls = Record<ControlId, boolean>;

/** The desktop's alone: the web build has no pictures, journal, log or hooks list. */
const DESKTOP: ControlId[] = ["picture", "journal", "log", "hooks"];
export const offered = () => CONTROLS.filter((c) => !(WEB && DESKTOP.includes(c.id)));

const KEY = "coupler.terminalPanels";
const ARRANGE_KEY = "coupler.terminalArrange";
const DEFAULTS: Controls = { map: true, vitals: false, picture: false, heard: false, journal: true, log: true, hooks: false };

export function loadControls(): Controls {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, unknown>;
    const out = { ...DEFAULTS };
    for (const c of CONTROLS) if (typeof raw?.[c.id] === "boolean") out[c.id] = raw[c.id] as boolean;
    return out;
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveControls(controls: Controls) {
  try {
    localStorage.setItem(KEY, JSON.stringify(controls));
  } catch {
    // Not kept past this run, then.
  }
}

export function loadArranging(): boolean {
  try {
    return localStorage.getItem(ARRANGE_KEY) === "on";
  } catch {
    return false;
  }
}

export function saveArranging(on: boolean) {
  try {
    localStorage.setItem(ARRANGE_KEY, on ? "on" : "off");
  } catch {
    // Not kept past this run, then.
  }
}

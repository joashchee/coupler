/**
 * Checking for a newer Coupler (src-tauri/src/update.rs): gear → Check
 * for Updates Now, and Check for Updates Automatically, off until the
 * player turns it on (CLAUDE.md rules 1 and 2: it's the one thing
 * Coupler asks of anything but CoffeeMUD). When on, Coupler asks GitHub
 * once a launch, at most once a day. Desktop only.
 */
import { invoke } from "@tauri-apps/api/core";

const AUTO_KEY = "coupler.updates";
/** When the last automatic check was, in ms; left out of backups (lib/backup.ts). */
export const LAST_KEY = "coupler.updates.last";
const DAY = 24 * 60 * 60 * 1000;

export interface UpdateCheck {
  current: string;
  latest: string;
  newer: boolean;
}

export const check = () => invoke<UpdateCheck>("update_check");
/** Opens the latest release's page in the player's browser. */
export const openPage = () => invoke<void>("update_open");

/** Automatic checks are on only once the player turned them on. */
export function loadAuto(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) === "on";
  } catch {
    return false;
  }
}

export function saveAuto(on: boolean) {
  try {
    localStorage.setItem(AUTO_KEY, on ? "on" : "off");
  } catch {
    // Not kept: asked again next launch.
  }
}

/** An automatic check is due: on, and none in the last day. Marks it done. */
export function autoDue(now = Date.now()): boolean {
  if (!loadAuto()) return false;
  try {
    const last = Number(localStorage.getItem(LAST_KEY) ?? 0);
    if (now - last < DAY && now >= last) return false;
    localStorage.setItem(LAST_KEY, String(now));
  } catch {
    // Unkept, it checks each launch.
  }
  return true;
}

/** A check's answer in words, for the status bar and the narrator. */
export function describe({ current, latest, newer }: UpdateCheck): string {
  return newer
    ? `Coupler ${latest} is out; this is ${current}. Get Coupler ${latest} in the gear menu opens its page.`
    : `Coupler ${current} is the latest.`;
}

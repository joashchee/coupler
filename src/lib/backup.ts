/**
 * **Coupler Backup**, the frontend's half: the settings it keeps (every
 * `coupler.` key in its storage) go into the backup beside app data's
 * files (src-tauri/src/backup.rs, which writes and reads the file). A
 * restored backup's settings are put back by the window's start script
 * before anything here reads them, once Coupler starts again.
 */

/** A backup's file name ends in this. */
export const EXTENSION = "coupler";

/** Kept out of a backup: the dev-only App Testing results, the restore's own mark, the web build's welcome. */
const LEFT_OUT = new Set(["coupler.appTestingResults", "coupler.restored", "coupler.web.accepted", "coupler.updates.last"]);

/** Every setting Coupler keeps, as JSON: an object of strings. */
export function settings(): string {
  const kept: Record<string, string> = {};
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (!key || !key.startsWith("coupler.") || LEFT_OUT.has(key)) continue;
      const value = localStorage.getItem(key);
      if (value !== null) kept[key] = value;
    }
  } catch {
    // Storage unavailable: the files alone.
  }
  return JSON.stringify(kept);
}

/** Whether a path is a Coupler Backup, by its name. */
export const isBackup = (path: string) => path.toLowerCase().endsWith(`.${EXTENSION}`);

/** The name a new backup is offered: the date it's made. */
export function defaultName(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `Coupler Backup ${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.${EXTENSION}`;
}

/** A backup's date in words. */
export const madeInWords = (seconds: number) => new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: "long", timeStyle: "short" });

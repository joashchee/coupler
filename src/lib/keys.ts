/**
 * The keyboard on every platform. Coupler's shortcuts are written the
 * Mac's way (Cmd+Shift+L, Ctrl+Cmd+1, Option+Up, Cmd+Period); on Windows
 * and Linux the Windows key belongs to the system, so Cmd is Ctrl there,
 * Ctrl+Cmd is Ctrl+Alt and Option is Alt. Matching a key and writing it
 * both go through here, so what's written is what works.
 */

/** What a key's matched by: a DOM keydown or React's. */
type Key = Pick<KeyboardEvent, "key" | "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

/** Whether this is a Mac (or an iPad, whose keyboards have Cmd too). */
export const MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent);

/** The platform's command key is held: Cmd on a Mac, Ctrl elsewhere (and not the other). */
export function primary(e: Key): boolean {
  return MAC ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
}

/** Cmd+Shift+letter (Ctrl+Shift+letter off the Mac), no Option or Alt. */
export function chord(e: Key): boolean {
  return primary(e) && e.shiftKey && !e.altKey;
}

/** Cmd+key alone (Ctrl+key off the Mac). */
export function command(e: Key): boolean {
  return primary(e) && !e.shiftKey && !e.altKey;
}

/** Ctrl+Cmd+key on a Mac, Ctrl+Alt+key elsewhere, no Shift. */
export function ctrlCmd(e: Key): boolean {
  if (e.shiftKey) return false;
  return MAC ? e.ctrlKey && e.metaKey && !e.altKey : e.ctrlKey && e.altKey && !e.metaKey;
}

/** Option+key on a Mac, Alt+key elsewhere, nothing else held. */
export function option(e: Key): boolean {
  return e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey;
}

/**
 * The letter or digit pressed, lowercase. `key` is what the layout makes
 * of it, but Alt (and Option on a Mac) turn a letter into another
 * character, so then the key's place (`code`) says which.
 */
export function letter(e: Key): string {
  const key = e.key.toLowerCase();
  if (/^[a-z0-9]$/.test(key)) return key;
  const code = /^(?:Key|Digit|Numpad)([A-Z0-9])$/.exec(e.code);
  return code ? code[1].toLowerCase() : key;
}

/** Whether the quiet key was pressed: Cmd+Period (Ctrl+Period off the Mac). */
export function quiet(e: Key): boolean {
  return command(e) && (e.key === "." || e.code === "Period");
}

/**
 * Words naming keys the Mac's way, written for this platform: Cmd+ is
 * Ctrl+, Ctrl+Cmd+ Ctrl+Alt+, Option Alt and Return Enter off the Mac.
 * Everything a player reads or hears about a key goes through it.
 */
export function keys(words: string): string {
  if (MAC) return words;
  return forOthers(words);
}

/** `keys` for Windows and Linux, whatever this is (exported for the tests). */
export function forOthers(words: string): string {
  return words
    .replace(/\bCtrl\+Cmd\+/g, "Ctrl+Alt+")
    .replace(/\bCmd\+/g, "Ctrl+")
    .replace(/\bOption\+/g, "Alt+")
    .replace(/\bReturn\b/g, "Enter");
}

/**
 * The web build (coupler.ansiapps.com), decided at
 * build time (`vite build --mode web`), so each build carries only its
 * own code. The web build is the desktop app made slim: the parts of the
 * screen it leaves out are listed here; `src/web/` stands in for Tauri.
 */
export const WEB = import.meta.env.MODE === "web";

/** The screen's parts (`defaultLayout`'s IDs) only the desktop has: the room's picture, the journal, the log, the hooks list and the BGM button (the Music Editor). */
export const DESKTOP_ONLY: ReadonlySet<string> = new Set(["picture", "journal", "log", "hooks", "bgm"]);

/** The shortcut keys (Cmd+Shift+letter) only the desktop has: the journal's U, N, J and K, the hooks list's H and the picture's P. */
export const DESKTOP_ONLY_KEYS = "unjkhp";

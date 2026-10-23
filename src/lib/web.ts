/**
 * The web build (coupler.ansiapps.com), decided at
 * build time (`vite build --mode web`), so each build carries only its
 * own code. The web build is the desktop app in a browser: the few parts
 * of the screen it leaves out are listed here; `src/web/` stands in for Tauri.
 */
export const WEB = import.meta.env.MODE === "web";

/** The screen's parts (`defaultLayout`'s IDs) only the desktop has: the hooks list and the BGM button (the Music Editor), both the player's own assets, which the web's mirror stands in for. */
export const DESKTOP_ONLY: ReadonlySet<string> = new Set(["hooks", "bgm"]);

/** The shortcut keys (Cmd+Shift+letter) only the desktop has: the hooks list's H. */
export const DESKTOP_ONLY_KEYS = "h";

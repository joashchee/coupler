/**
 * UI theme: "ansiapps" (Coupler's default: a MUD client belongs in text
 * mode), the old-school DOS look every ansiapps app offers
 * (docs/ansiapps-theme.md), or "modern". The choice is a
 * `data-theme` attribute on <html>, which src/ansiapps-theme.css keys
 * off. index.html's inline script applies the stored choice before first
 * paint, so it must use the same key and values.
 *
 * The theme covers Coupler's own screens only: the game output always
 * shows CoffeeMUD's own colors, like Stylus's art canvas.
 */

export type Theme = "modern" | "ansiapps";

/** Also read by the inline script in index.html. */
const THEME_KEY = "coupler.theme";

export function loadTheme(): Theme {
  try {
    return localStorage.getItem(THEME_KEY) === "modern" ? "modern" : "ansiapps";
  } catch {
    return "ansiapps";
  }
}

export function applyTheme(theme: Theme) {
  if (theme === "ansiapps") document.documentElement.dataset.theme = "ansiapps";
  else delete document.documentElement.dataset.theme;
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // localStorage unavailable — the theme just won't persist across launches.
  }
}

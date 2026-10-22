/**
 * The phone: Coupler on an iPhone, upright only, played by touch. Its
 * own fixed screen (lib/stage.ts, `PHONE_WIDTH`): 48 of the theme's
 * cells across, as many rows as the phone shows, and its own layout
 * (`phoneLayout` in App.tsx), not the desktop's 1280 by 720 shrunk.
 *
 * Decided when the page loads, never by the window's size: an iPhone is
 * a phone (an iPad runs the iPhone app as it is, the project being
 * iPhone only), and a Mac, Windows or Linux window is the desktop
 * however it's sized. In a dev build, `coupler.phone` = "on" in
 * localStorage shows the phone screen on the Mac to try it.
 *
 * From the Gemini survey of fixed-canvas UIs (the maintainer's notes):
 * the explicit profiles, the fixed canvas scaled as a whole, 48 px touch
 * targets and portrait locked are kept; one component tree, not a
 * second app, so a feature is never on one profile only by accident.
 */
const PHONE_KEY = "coupler.phone";

function devPhone(): boolean {
  if (!import.meta.env.DEV) return false;
  try {
    return localStorage.getItem(PHONE_KEY) === "on";
  } catch {
    return false;
  }
}

/** An iPhone (or iPod), or the dev build trying the phone screen. */
export const PHONE = (typeof navigator !== "undefined" && /iPhone|iPod/.test(navigator.userAgent)) || devPhone();

/** Marks <html> for phone.css, and lets the game's sounds play with the ring switch on silent. */
export function applyFormFactor() {
  if (!PHONE) return;
  document.documentElement.dataset.form = "phone";
  // Safari 16.4 and later: "playback" is what a game heard more than read
  // needs; by default the Web Audio a page plays is muted by the switch.
  const session = (navigator as Navigator & { audioSession?: { type: string } }).audioSession;
  if (session) session.type = "playback";
}

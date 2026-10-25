/**
 * The phone: Coupler on an iPhone, upright only, played by touch. Its
 * own fixed screen (lib/stage.ts, `PHONE_WIDTH`): 48 of the theme's
 * cells across, as many rows as the phone shows, and its own layout
 * (`phoneLayout` in App.tsx), not the desktop's 1280 by 720 shrunk.
 *
 * Decided when the page loads, never by the window's size: an iPhone is
 * a phone in the iPhone app (an iPad runs the iPhone app as it is, the
 * project being iPhone only) and gets the tablet screen in the web
 * build, and a Mac, Windows or Linux window is the desktop however it's
 * sized. On a tablet (an iPad), and in a dev build on the
 * Mac, gear → Phone Screen, for Testing (`coupler.phone` = "on") shows
 * the phone screen, phone-shaped, to try it.
 *
 * From the Gemini survey of fixed-canvas UIs (the maintainer's notes):
 * the explicit profiles, the fixed canvas scaled as a whole, 48 px touch
 * targets and portrait locked are kept; one component tree, not a
 * second app, so a feature is never on one profile only by accident.
 */
import { startTouchScrollbars } from "./scrollbars";
import { WEB } from "./web";

const PHONE_KEY = "coupler.phone";

function phoneAsked(): boolean {
  try {
    return localStorage.getItem(PHONE_KEY) === "on";
  } catch {
    return false;
  }
}

const touchMac = () => typeof navigator !== "undefined" && /Macintosh/.test(navigator.userAgent) && navigator.maxTouchPoints >= 2;
/** The screen's shorter side, in points, whatever the page's viewport. */
const screenSide = () => (typeof screen !== "undefined" ? Math.min(screen.width, screen.height) : 0);

/**
 * An iPhone asking for a site's desktop version: Safari's Request Desktop
 * Website (per site, or for every site in Settings) sends a Mac's user
 * agent, which Chrome and Brave on the iPhone don't. A Mac has no touch
 * screen, and the screen's own size (in points, whatever the page's
 * viewport) tells an iPhone, under 600 across even upright, from an
 * iPad, which stays the desktop.
 */
function iPhoneAsMac(): boolean {
  return touchMac() && screenSide() < 600;
}

const IPHONE = typeof navigator !== "undefined" && (/iPhone|iPod/.test(navigator.userAgent) || iPhoneAsMac());

/**
 * An iPhone in the web build plays on the tablet screen, not the phone's
 * (decided 2026-10-24): no zooming, the game above and Coupler's own
 * keyboard below, the same as a tablet's browser. The phone screen is
 * the iPhone app's.
 */
const WEB_IPHONE = IPHONE && WEB;

/** The iPhone app: the phone screen. */
const REAL_PHONE = IPHONE && !WEB;

/**
 * A tablet: an iPad (its Safari passes for a Mac with a touch screen) or
 * another touch screen 600 points or more across, or an iPhone in the
 * web build (`WEB_IPHONE`). The tablet screen (`TABLET_SCREEN`), unless
 * the player turns on the phone screen to try it (gear menu; not on an
 * iPhone, which is phone-sized already).
 */
export const TABLET =
  WEB_IPHONE ||
  (!IPHONE &&
    typeof navigator !== "undefined" &&
    (/iPad/.test(navigator.userAgent) || touchMac() || (/Android/.test(navigator.userAgent) && !/Mobile/.test(navigator.userAgent))) &&
    screenSide() >= 600);

/** Where the phone screen can be turned on and off to try it: a tablet (not an iPhone), or the dev build on the Mac. */
export const PHONE_TRYABLE = !IPHONE && (TABLET || import.meta.env.DEV);

/** An iPhone (or iPod), or a tablet or the dev build trying the phone screen. */
export const PHONE = REAL_PHONE || (PHONE_TRYABLE && phoneAsked());

/**
 * The phone screen shown on a bigger one, to try it: drawn the shape of a
 * phone held upright (lib/stage.ts, `phoneFit`'s `emulated`), whichever
 * way the tablet is turned.
 */
export const PHONE_EMULATED = PHONE && !REAL_PHONE;

/**
 * The tablet screen: Coupler on an iPad (or another tablet), upright
 * only, played by touch with Coupler's own keyboard. Its upper part is
 * Workshop's desk, as many cells as the player's text size fits
 * (lib/stage.ts, `tabletFit`); its lower part is one fixed size, the
 * command line, the keyboard or the touch controls, and the messages
 * (`tabletLower` in App.tsx). The system's keyboard never shows
 * (`inputmode="none"` on every field) and the page never zooms.
 *
 * Every tablet has it, unless the player tries the phone screen there;
 * the dev build on the Mac tries it from the gear menu
 * (`coupler.tablet` = "on"), drawn the shape of an iPad held upright.
 */
const TABLET_KEY = "coupler.tablet";

/** "on" tries the tablet screen, "iphone" the small one (`TABLET_SMALL`). */
function tabletAsked(): string | null {
  try {
    return localStorage.getItem(TABLET_KEY);
  } catch {
    return null;
  }
}

/** Where the tablet screen can be tried on a computer: the dev build on the Mac. */
export const TABLET_TRYABLE = !IPHONE && !TABLET && import.meta.env.DEV;

/** A tablet (not trying the phone screen), or the dev build trying the tablet screen. */
export const TABLET_SCREEN = !PHONE && (TABLET || (TABLET_TRYABLE && ["on", "iphone"].includes(tabletAsked() ?? "")));

/** The tablet screen tried on a computer: drawn as an iPad upright, 768 by 1024 points, fitted to the window (or an iPhone, `TABLET_SMALL`). */
export const TABLET_EMULATED = TABLET_SCREEN && !TABLET;

/**
 * The small tablet screen: an iPhone in the web build (decided
 * 2026-10-25), or the dev build on the Mac trying it, drawn as an
 * iPhone upright. The same two stages, but the lower part is in the
 * game output's own cells (lib/stage.ts, `tabletFit`'s `small`), with
 * Coupler's phone keyboard (components/SmallKeyboard.tsx) and fewer
 * touch controls, and the menu bar is one row (`tabletLayout` in
 * App.tsx). Marked on <html> as data-size="small" for tablet.css.
 */
export const TABLET_SMALL = TABLET_SCREEN && (WEB_IPHONE || (TABLET_EMULATED && tabletAsked() === "iphone"));

/** Turns the tablet screen on or off where it can be tried (`"iphone"`, the small one), and starts again with it. */
export function setTabletScreen(on: boolean | "iphone") {
  try {
    localStorage.setItem(TABLET_KEY, on === "iphone" ? "iphone" : on ? "on" : "off");
  } catch {
    // Nothing kept: the same screen comes back.
  }
  window.location.reload();
}

/** Turns the phone screen on or off where it can be tried, and starts again with it. */
export function setPhoneScreen(on: boolean) {
  try {
    localStorage.setItem(PHONE_KEY, on ? "on" : "off");
  } catch {
    // Nothing kept: the same screen comes back.
  }
  window.location.reload();
}

/** The text fields the system's keyboard would open for. */
const FIELDS = 'input:not([type]), input[type="text"], input[type="search"], input[type="password"], input[type="email"], input[type="url"], input[type="tel"], input[type="number"], textarea';

/**
 * The tablet: no zooming, no system keyboard, upright if the browser can
 * hold it there. iOS ignores the viewport's user-scalable=no for a
 * pinch, so Safari's own gesture events are stopped too; a double tap
 * doesn't zoom for touch-action (tablet.css). Every text field, now and
 * to come, gets inputmode="none": tapping it shows Coupler's keyboard
 * (components/TabletKeyboard.tsx), never the system's.
 */
function applyTablet() {
  document.documentElement.dataset.form = "tablet";
  if (TABLET_SMALL) document.documentElement.dataset.size = "small";
  document.querySelector('meta[name="viewport"]')?.setAttribute("content", "width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover");
  const stop = (e: Event) => e.preventDefault();
  for (const name of ["gesturestart", "gesturechange", "gestureend"]) document.addEventListener(name, stop, { passive: false });
  document.addEventListener("touchmove", (e) => e.touches.length > 1 && e.preventDefault(), { passive: false });
  const quiet = (root: ParentNode) => {
    for (const field of root.querySelectorAll<HTMLElement>(FIELDS)) if (field.getAttribute("inputmode") !== "none") field.setAttribute("inputmode", "none");
  };
  quiet(document);
  new MutationObserver((changes) => {
    for (const change of changes) for (const node of change.addedNodes) if (node instanceof Element) {
      if (node.matches(FIELDS)) node.setAttribute("inputmode", "none");
      quiet(node);
    }
  }).observe(document.documentElement, { childList: true, subtree: true });
  // Only some browsers (an installed web app on Android) can hold it upright; elsewhere a notice asks (App.tsx).
  const orientation = screen.orientation as ScreenOrientation & { lock?: (o: string) => Promise<void> };
  orientation?.lock?.("portrait").catch(() => {});
}

/** Marks <html> for phone.css and tablet.css, draws the scroll bars, and lets the game's sounds play with the ring switch on silent. */
export function applyFormFactor() {
  if (TABLET_SCREEN) applyTablet();
  if (!PHONE && !TABLET_SCREEN) return;
  // The system's scroll bars show only while scrolling there (lib/scrollbars.ts).
  startTouchScrollbars();
  if (PHONE) document.documentElement.dataset.form = "phone";
  // Safari 16.4 and later: "playback" is what a game heard more than read
  // needs; by default the Web Audio a page plays is muted by the switch.
  const session = (navigator as Navigator & { audioSession?: { type: string } }).audioSession;
  if (session) session.type = "playback";
}

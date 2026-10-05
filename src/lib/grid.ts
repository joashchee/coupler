/**
 * The ANSIapps theme's grid (docs/ansiapps-theme.md, "The grid"): every
 * character sits in a cell of the 8 by 16 grid, counted from the
 * screen's top left. The stylesheet does the placing; this file does the
 * two things CSS can't.
 */
import { CELL_HEIGHT, CELL_WIDTH, STAGE_WIDTH } from "./stage";

/**
 * Scrolling comes to rest on a whole row, so scrolled text is on the
 * grid too. One listener for every scrolling box on the screen; it does
 * nothing in the modern theme. Returns the function that removes it.
 */
export function installRowSnap(): () => void {
  const timers = new Map<Element, number>();
  const onScroll = (e: Event) => {
    const el = e.target;
    if (!(el instanceof HTMLElement) || document.documentElement.dataset.theme !== "ansiapps") return;
    window.clearTimeout(timers.get(el));
    // Once the scrolling has stopped, not during it: that would fight the trackpad.
    timers.set(
      el,
      window.setTimeout(() => {
        timers.delete(el);
        const snapped = Math.round(el.scrollTop / CELL_HEIGHT) * CELL_HEIGHT;
        if (snapped !== el.scrollTop) el.scrollTop = snapped;
      }, 120),
    );
  };
  document.addEventListener("scroll", onScroll, true);
  return () => {
    document.removeEventListener("scroll", onScroll, true);
    timers.forEach((t) => window.clearTimeout(t));
  };
}

export interface GridReport {
  checked: number;
  /** What's off the grid, in words: the text and where its first character is. */
  off: string[];
}

const MARK = "grid-off";

/**
 * Dev-only (gear → Check the Grid): measures every piece of text showing
 * on the stage, and every field, button and picture, and lists the ones
 * that aren't on a cell boundary, outlining them. Run it on each screen
 * and dialog after changing the theme's CSS.
 */
export function checkGrid(stage: HTMLElement): GridReport {
  stage.querySelectorAll(`.${MARK}`).forEach((el) => el.classList.remove(MARK));
  const frame = stage.getBoundingClientRect();
  const scale = frame.width / STAGE_WIDTH;
  const px = (v: number, origin: number) => (v - origin) / scale;
  const offBy = (v: number, cell: number) => {
    const r = ((v % cell) + cell) % cell;
    return Math.min(r, cell - r) > 0.05;
  };
  const showing = (el: Element | null) => {
    for (let e = el; e && e !== stage; e = e.parentElement) {
      const style = getComputedStyle(e);
      if (style.display === "none" || style.visibility === "hidden" || (e as HTMLElement).inert || e.classList.contains("sr-only")) return false;
      if (e.classList.contains("dialog-overlay") && !e.classList.contains("open")) return false;
    }
    return true;
  };
  const report: GridReport = { checked: 0, off: [] };
  const flag = (el: Element, what: string, x: number, y: number) => {
    el.classList.add(MARK);
    report.off.push(`"${what}" at ${(x / CELL_WIDTH + 1).toFixed(2)}, ${(y / CELL_HEIGHT + 1).toFixed(2)}`);
  };

  const walker = document.createTreeWalker(stage, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node as Text;
    const first = text.data.search(/\S/);
    if (first < 0 || !text.parentElement || !showing(text.parentElement)) continue;
    const range = document.createRange();
    range.setStart(text, first);
    range.setEnd(text, text.data.length);
    for (const rect of range.getClientRects()) {
      if (rect.width === 0) continue;
      report.checked++;
      const [x, y, width] = [px(rect.left, frame.left), px(rect.top, frame.top), rect.width / scale];
      // A width that isn't whole cells means a character came from another font.
      if (offBy(x, CELL_WIDTH) || offBy(y, CELL_HEIGHT) || offBy(width, CELL_WIDTH)) {
        flag(text.parentElement, text.data.trim().slice(0, 24), x, y);
        break;
      }
    }
  }
  // A trigger's picture is the player's own, whatever its size: only its corner is placed.
  for (const el of stage.querySelectorAll("input, textarea, select, button, svg, img:not(.hook-picture), .progress-bar")) {
    if (!showing(el)) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0) continue;
    report.checked++;
    const [x, y] = [px(rect.left, frame.left), px(rect.top, frame.top)];
    if (offBy(x, CELL_WIDTH) || offBy(y, CELL_HEIGHT) || offBy(rect.width / scale, CELL_WIDTH) || offBy(rect.height / scale, CELL_HEIGHT)) {
      flag(el, el.getAttribute("aria-label") ?? el.getAttribute("placeholder") ?? el.tagName.toLowerCase(), x, y);
    }
  }
  return report;
}

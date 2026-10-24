/**
 * Scroll bars that always show on a touch screen. Safari on an iPad or an
 * iPhone draws its own over the content only while it's being scrolled,
 * and ignores the themes' `::-webkit-scrollbar`, so a list with more in
 * it than shows (the ways to play, after hanging up) looked like all
 * there is. Here every box that scrolls up and down, and has more than
 * it shows, gets a bar at its right edge: the track the box's height,
 * the thumb what's in view and where. Drawn on a layer of its own over
 * the page (never inside React's markup), hidden from screen readers
 * (they tell a list's length themselves) and from touches, which still
 * scroll the box under it. A bar whose box is covered (a dialog over a
 * panel) isn't drawn.
 *
 * The desktop's browsers draw the themes' own bars all the time
 * (index.css, ansiapps-theme.css).
 */

/** The bar's width in the page's points: a thin strip inside the box's right edge, over no character's cell but the last. */
const WIDTH = 6;
/** The smallest thumb, so a long list's is still seen. */
const LEAST_THUMB = 16;
/** How often, at most, the page is looked over for boxes that scroll after it changes. */
const LOOK_EVERY = 300;

/** What never holds a box that scrolls, so isn't looked in: the game output's lines, pictures' rows, drawings. */
const SKIP = ".terminal-line, .hook-ansi, svg, canvas, .touch-scrollbars";

interface Bar {
  track: HTMLDivElement;
  thumb: HTMLDivElement;
}

export function startTouchScrollbars() {
  const layer = document.createElement("div");
  layer.className = "touch-scrollbars";
  layer.setAttribute("aria-hidden", "true");
  document.body.appendChild(layer);
  const bars = new Map<Element, Bar>();
  let boxes: Element[] = [];

  /** Every box that scrolls up and down and has more than it shows. */
  const find = () => {
    const found: Element[] = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
      acceptNode: (node) => ((node as Element).matches(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
    });
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const el = node as HTMLElement;
      if (el.scrollHeight <= el.clientHeight + 1 || el.clientHeight === 0) continue;
      const overflow = getComputedStyle(el).overflowY;
      if (overflow === "auto" || overflow === "scroll") found.push(el);
    }
    boxes = found;
    draw();
  };

  /** Whether the box is what's under its own right edge: not covered, not hidden. */
  const uncovered = (el: Element, rect: DOMRect) => {
    const x = rect.right - WIDTH / 2 - 1;
    return [0.1, 0.5, 0.9].some((at) => {
      const under = document.elementFromPoint(x, rect.top + rect.height * at);
      return under !== null && (under === el || el.contains(under));
    });
  };

  const draw = () => {
    const shown = new Set<Element>();
    for (const el of boxes) {
      if (!el.isConnected || el.closest("[inert]")) continue;
      const rect = el.getBoundingClientRect();
      if (rect.height < LEAST_THUMB || !uncovered(el, rect)) continue;
      const box = el as HTMLElement;
      // The page's scale (a fitted stage) between the box's own points and the screen's.
      const scale = box.offsetHeight > 0 ? rect.height / box.offsetHeight : 1;
      const top = rect.top + box.clientTop * scale;
      const height = box.clientHeight * scale;
      const right = rect.left + (box.clientLeft + box.clientWidth) * scale;
      let bar = bars.get(el);
      if (!bar) {
        bar = { track: document.createElement("div"), thumb: document.createElement("div") };
        bar.track.className = "touch-scrollbar";
        bar.thumb.className = "touch-scrollbar-thumb";
        bar.track.appendChild(bar.thumb);
        layer.appendChild(bar.track);
        bars.set(el, bar);
      }
      const thumb = Math.max(LEAST_THUMB, (height * box.clientHeight) / box.scrollHeight);
      const room = box.scrollHeight - box.clientHeight;
      const at = room > 0 ? ((height - thumb) * box.scrollTop) / room : 0;
      Object.assign(bar.track.style, { left: `${right - WIDTH}px`, top: `${top}px`, width: `${WIDTH}px`, height: `${height}px` });
      Object.assign(bar.thumb.style, { top: `${at}px`, height: `${thumb}px` });
      shown.add(el);
    }
    for (const [el, bar] of bars) {
      if (!shown.has(el)) {
        bar.track.remove();
        bars.delete(el);
      }
    }
  };

  let looking: number | null = null;
  const lookSoon = () => {
    if (looking === null) looking = window.setTimeout(() => ((looking = null), find()), LOOK_EVERY);
  };
  let drawing = false;
  const drawSoon = () => {
    if (drawing) return;
    drawing = true;
    requestAnimationFrame(() => ((drawing = false), draw()));
  };
  new MutationObserver((changes) => {
    if (changes.some((c) => !layer.contains(c.target))) lookSoon();
  }).observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "style", "hidden", "inert", "open"] });
  document.addEventListener("scroll", drawSoon, { capture: true, passive: true });
  window.addEventListener("resize", lookSoon);
  window.addEventListener("orientationchange", lookSoon);
  lookSoon();
}

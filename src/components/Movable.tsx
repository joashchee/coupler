/**
 * Something the user can arrange. Everything with a place on the screen
 * goes in one (the panels, the command line, the message bar, each
 * button of the menu bar), so the whole screen can be laid out to suit.
 * Its place, size and turn in the pile are kept (lib/layout.ts). It
 * stays inside the screen (its positioned parent) and, in the ANSIapps
 * theme, on the character grid.
 *
 * Arranging happens in Workshop mode (ArrangeContext, the gear menu).
 * Then its top left corner is for moving it and its bottom right corner
 * for resizing it: one character of each, laid over what's there and
 * not drawn, so the screen looks exactly as it does with the mode off.
 * The pointer's shape says which corner it's on. Touching a thing
 * brings it in front of the others of its tier (small controls are
 * always in front of bars, and bars of panels, so nothing can be buried
 * under something bigger). With the mode off the corners are ordinary
 * parts of the thing again.
 *
 * The corners are buttons, so the keyboard does it too: the arrow keys
 * by one character, Home back to the default place or size.
 *
 * Only Workshop's screen is the player's (lib/ux.ts), and Terminal's
 * Control Panel side (components/ControlPanel.tsx), kept apart from
 * Workshop's under its own scope (LayoutScopeContext, "terminal:"); in
 * Immersive (KeptContext false) everything is in its default place and
 * nothing is kept. A `fixed` thing (Terminal's game output, command
 * line) is never arranged.
 */
import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { fitRect, frontZ, LAYOUT_FORGOTTEN, storedPanel, storePanel, type Point, type Rect, type Size } from "../lib/layout";
import { CELL_HEIGHT, CELL_WIDTH } from "../lib/stage";

/** Whether Workshop mode is on: things can be moved and resized by their corners. */
export const ArrangeContext = createContext(true);
/** Whether the player's arrangement applies (Workshop) or only the default layout does. */
export const KeptContext = createContext(true);
/** Whose arrangement it is: "" for Workshop's, "terminal:" for Terminal's, a prefix to every kept ID. */
export const LayoutScopeContext = createContext("");

/** Room for each tier's pile of `z` turns, and the place above them all for an open menu. */
const TIER = 1_000_000;
const FRONT = 10 * TIER;
/** A trigger's picture: in front of the panels, behind the bars and the controls. */
export const PICTURE_Z = 2 * TIER - 1;

/** The smallest anything can be made, unless it says otherwise: four characters by one row. */
const LEAST: Size = { width: 4 * CELL_WIDTH, height: CELL_HEIGHT };

interface MovableProps {
  /** The key its place is kept under. */
  id: string;
  /** What it is, for the marks' names and the announcements: "map". */
  label: string;
  /**
   * Its default place, and its default size. A width or height left out
   * is the size of what's in it, until the user resizes it.
   */
  usual: { at: Point; size?: Partial<Size> };
  /** False for the one thing that can't be resized: the game output, always 80 by 25. */
  resizable?: boolean;
  /** The smallest it can be made. */
  least?: Size;
  /** Extra classes for the box. */
  className?: string;
  /** Stays behind everything and never comes to the front: a backdrop, like the menu bar's strip. */
  backdrop?: boolean;
  /**
   * Which pile it's in: 1 for framed panels, 2 for bars (the command
   * line, the message bar), 3 for small controls. A higher tier is
   * always in front; touching something brings it to the front of its
   * own tier only. Defaults to 3.
   */
  tier?: 1 | 2 | 3;
  /** In front of everything for now: it has a menu open. */
  front?: boolean;
  /** Brought to the front of its tier each time it appears: the ways to play, after a disconnect. */
  frontOnShow?: boolean;
  /** Always in its default place and size, never arranged. */
  fixed?: boolean;
  /** Says what just happened ("Map moved: column 109, row 2."), for the status bar. */
  announce?: (message: string) => void;
  children: ReactNode;
}

type Handle = "move" | "size";

const STEPS: Record<string, [number, number]> = {
  ArrowLeft: [-CELL_WIDTH, 0],
  ArrowRight: [CELL_WIDTH, 0],
  ArrowUp: [0, -CELL_HEIGHT],
  ArrowDown: [0, CELL_HEIGHT],
};

export function Movable({ id: own, label, usual, resizable = true, least = LEAST, className, backdrop, tier = 3, front, frontOnShow, fixed, announce, children }: MovableProps) {
  const id = useContext(LayoutScopeContext) + own;
  const keeps = useContext(KeptContext) && !fixed;
  const arranging = useContext(ArrangeContext) && keeps;
  const box = useRef<HTMLDivElement>(null);
  // What the user chose; null is "the usual", which can differ by theme.
  const [place, setPlace] = useState<Point | null>(() => {
    const kept = keeps ? storedPanel(id) : {};
    return kept.x !== undefined && kept.y !== undefined ? { x: kept.x, y: kept.y } : null;
  });
  const [size, setSize] = useState<Size | null>(() => {
    const kept = keeps ? storedPanel(id) : {};
    return kept.width !== undefined && kept.height !== undefined ? { width: kept.width, height: kept.height } : null;
  });
  const [z, setZ] = useState(() => (keeps ? (storedPanel(id).z ?? 0) : 0));
  const [rect, setRect] = useState<Rect | null>(null);
  const drag = useRef<{ handle: Handle; pointer: Point; from: Rect; to: Rect | null } | null>(null);

  // The size it's held to: null leaves it the size of what's in it.
  const wantedPlace = place ?? usual.at;
  const wantedSize = resizable ? (size ?? usual.size ?? null) : null;

  useEffect(() => {
    const forget = () => {
      setPlace(null);
      setSize(null);
      setZ(0);
    };
    window.addEventListener(LAYOUT_FORGOTTEN, forget);
    return () => window.removeEventListener(LAYOUT_FORGOTTEN, forget);
  }, []);

  const fit = (p: Point, s: Partial<Size> | null): Rect | null => {
    const el = box.current;
    const area = el?.offsetParent;
    if (!el || !(area instanceof HTMLElement)) return null;
    return fitRect(
      { x: p.x, y: p.y, width: s?.width ?? el.offsetWidth, height: s?.height ?? el.offsetHeight },
      s ? least : null,
      { width: area.clientWidth, height: area.clientHeight },
      document.documentElement.dataset.theme === "ansiapps",
    );
  };

  // What it wants may not fit just now (another panel's area, the other
  // theme's sizes): it's shown at the nearest that does, and what it
  // wants is kept for when there's room again.
  useLayoutEffect(() => {
    const el = box.current;
    const area = el?.offsetParent;
    if (!el || !area) return;
    const measure = () => {
      const next = fit(wantedPlace, wantedSize);
      if (!next) return;
      setRect((was) => (was && was.x === next.x && was.y === next.y && was.width === next.width && was.height === next.height ? was : next));
    };
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    observer.observe(area);
    measure();
    return () => observer.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wantedPlace.x, wantedPlace.y, wantedSize?.width, wantedSize?.height, least.width, least.height]);

  /** How much the stage is scaled (full screen): pointer distances are in the monitor's pixels. */
  const scale = () => {
    const el = box.current;
    return el && el.offsetWidth > 0 ? el.getBoundingClientRect().width / el.offsetWidth : 1;
  };

  /** In front of every other panel, and kept there. */
  const raise = () => {
    if (backdrop || !keeps) return;
    const top = frontZ();
    if (z === top && z > 0) return;
    setZ(top + 1);
    storePanel(id, { z: top + 1 });
  };

  useEffect(() => {
    if (frontOnShow) raise();
    // Only on appearing: it's mounted each time it's shown.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Keeps what a handle just did and says it; `usualAgain` is Home, which forgets it instead. */
  const done = (handle: Handle, to: Rect, usualAgain: boolean) => {
    const name = label.charAt(0).toUpperCase() + label.slice(1);
    if (handle === "size") {
      storePanel(id, usualAgain ? { width: undefined, height: undefined } : { width: to.width, height: to.height });
      const cells = `${Math.round(to.width / CELL_WIDTH)} columns by ${Math.round(to.height / CELL_HEIGHT)} rows`;
      announce?.(usualAgain ? `${name} back to its default size.` : `${name} resized: ${cells}.`);
      return;
    }
    storePanel(id, usualAgain ? { x: undefined, y: undefined } : { x: to.x, y: to.y });
    const area = box.current?.offsetParent;
    const stage = box.current?.closest(".app");
    if (!area || !stage) return;
    // The screen cell its corner is on, counted from 1 like the status bar's readout.
    const [a, s, k] = [area.getBoundingClientRect(), stage.getBoundingClientRect(), scale()];
    const column = Math.round(((a.left - s.left) / k + to.x) / CELL_WIDTH) + 1;
    const row = Math.round(((a.top - s.top) / k + to.y) / CELL_HEIGHT) + 1;
    announce?.(`${name} ${usualAgain ? "back in its default place" : "moved"}: column ${column}, row ${row}.`);
  };

  /** Where `handle` pulled by (dx, dy) stage pixels from `from` leaves the panel. */
  const pulled = (handle: Handle, from: Rect, dx: number, dy: number): Rect | null => {
    if (handle === "move") return fit({ x: from.x + dx, y: from.y + dy }, wantedSize && from);
    const area = box.current?.offsetParent;
    if (!(area instanceof HTMLElement)) return null;
    // The corner stops at the area's edge; it doesn't push the panel back the other way.
    return fit(from, {
      width: Math.min(from.width + dx, area.clientWidth - from.x),
      height: Math.min(from.height + dy, area.clientHeight - from.y),
    });
  };

  const apply = (handle: Handle, to: Rect) => {
    if (handle === "move") setPlace({ x: to.x, y: to.y });
    else setSize({ width: to.width, height: to.height });
  };

  const handleProps = (handle: Handle) => ({
    onPointerDown: (e: PointerEvent<HTMLButtonElement>) => {
      if (e.button !== 0 || !rect) return;
      e.currentTarget.setPointerCapture(e.pointerId);
      drag.current = { handle, pointer: { x: e.clientX, y: e.clientY }, from: rect, to: null };
    },
    onPointerMove: (e: PointerEvent<HTMLButtonElement>) => {
      const d = drag.current;
      if (!d) return;
      const k = scale();
      const to = pulled(d.handle, d.from, (e.clientX - d.pointer.x) / k, (e.clientY - d.pointer.y) / k);
      if (!to) return;
      const same = (a: Rect, b: Rect) => a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;
      if (!d.to && same(to, d.from)) return;
      d.to = to;
      apply(d.handle, to);
    },
    onPointerUp: () => {
      const d = drag.current;
      drag.current = null;
      // A click that went nowhere isn't a move.
      if (d?.to) done(d.handle, d.to, false);
    },
    onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => {
      if (e.metaKey || e.ctrlKey || e.altKey || !rect) return;
      const step = STEPS[e.key];
      if (!step && e.key !== "Home") return;
      e.preventDefault();
      raise();
      if (step) {
        const to = pulled(handle, rect, step[0], step[1]);
        if (!to) return;
        apply(handle, to);
        done(handle, to, false);
        return;
      }
      const to = handle === "move" ? fit(usual.at, wantedSize && rect) : fit(rect, usual.size ?? null);
      if (handle === "move") setPlace(null);
      else setSize(null);
      if (to) done(handle, to, true);
    },
  });
  const move = handleProps("move");
  const resizing = handleProps("size");

  const at = { ...wantedPlace, ...wantedSize, ...rect };
  return (
    <div
      ref={box}
      className={`movable${className ? ` ${className}` : ""}`}
      data-panel={own}
      style={{
        left: at.x,
        top: at.y,
        width: wantedSize?.width !== undefined ? at.width : undefined,
        height: wantedSize?.height !== undefined ? at.height : undefined,
        zIndex: front ? FRONT : backdrop ? 0 : tier * TIER + z,
      }}
      onPointerDownCapture={raise}
    >
      {arranging && (
        <button
          type="button"
          className="movable-handle movable-move"
          data-testid={`move-${own}`}
          aria-label={`Move the ${label}`}
          title={`Drag to move the ${label}. With the keyboard: the arrow keys move it, Home puts it back`}
          {...move}
          onPointerCancel={move.onPointerUp}
        />
      )}
      {children}
      {arranging && resizable && (
        <button
          type="button"
          className="movable-handle movable-size"
          data-testid={`resize-${own}`}
          aria-label={`Resize the ${label}`}
          title={`Drag to resize the ${label}. With the keyboard: the arrow keys resize it, Home gives it its default size`}
          {...resizing}
          onPointerCancel={resizing.onPointerUp}
        />
      )}
    </div>
  );
}

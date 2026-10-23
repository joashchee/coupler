/**
 * The phone screen's edge swipes (lib/phoneSections.ts): a finger that
 * starts within `EDGE` of the stage's left or right edge and moves across
 * turns the carousel; one that starts at its top edge and moves down
 * pulls the sheet of options down; while the sheet is down, a swipe up
 * anywhere puts it back. A mouse drag does the same in the dev preview.
 *
 * Measured against the stage's rectangle, not the window's, so on a
 * tablet trying the phone screen (drawn in the middle) its edges count.
 * Safari's own back swipe starts at the window's very edge; on the
 * iPhone the stage's edges are there too, and Coupler, one page with no
 * history, leaves nothing for it to go back to.
 */
import { useEffect, useRef } from "react";

/** How near an edge a swipe starts, in screen points. */
const EDGE = 32;
/** How far it must go to count. */
const FAR = 48;

export interface EdgeSwipes {
  /** In from the left edge, moving right. */
  fromLeft: () => void;
  /** In from the right edge, moving left. */
  fromRight: () => void;
  /** Down from the top edge. */
  fromTop: () => void;
  /** Up, anywhere, while `sheetDown`. */
  up: () => void;
  sheetDown: boolean;
}

type Start = { x: number; y: number; edge: "left" | "right" | "top" | null };

export function useEdgeSwipes(stage: { current: HTMLElement | null }, enabled: boolean, swipes: EdgeSwipes) {
  const latest = useRef(swipes);
  latest.current = swipes;

  useEffect(() => {
    if (!enabled) return;
    let start: Start | null = null;

    const begin = (x: number, y: number) => {
      const box = stage.current?.getBoundingClientRect();
      if (!box) return;
      const edge = x - box.left < EDGE ? "left" : box.right - x < EDGE ? "right" : y - box.top < EDGE ? "top" : null;
      start = { x, y, edge };
    };
    const end = (x: number, y: number) => {
      const s = start;
      start = null;
      if (!s) return;
      const dx = x - s.x;
      const dy = y - s.y;
      const across = Math.abs(dx) > Math.abs(dy);
      const now = latest.current;
      if (now.sheetDown) {
        if (!across && dy < -FAR) now.up();
        return;
      }
      if (s.edge === "left" && across && dx > FAR) now.fromLeft();
      else if (s.edge === "right" && across && dx < -FAR) now.fromRight();
      else if (s.edge === "top" && !across && dy > FAR) now.fromTop();
    };

    const touchStart = (e: TouchEvent) => {
      if (e.touches.length !== 1) {
        start = null;
        return;
      }
      begin(e.touches[0].clientX, e.touches[0].clientY);
    };
    const touchEnd = (e: TouchEvent) => {
      const t = e.changedTouches[0];
      if (t) end(t.clientX, t.clientY);
    };
    const touchCancel = () => {
      start = null;
    };
    const mouseDown = (e: PointerEvent) => {
      if (e.pointerType === "mouse" && e.button === 0) begin(e.clientX, e.clientY);
    };
    const mouseUp = (e: PointerEvent) => {
      if (e.pointerType === "mouse") end(e.clientX, e.clientY);
    };

    window.addEventListener("touchstart", touchStart, { passive: true });
    window.addEventListener("touchend", touchEnd, { passive: true });
    window.addEventListener("touchcancel", touchCancel, { passive: true });
    window.addEventListener("pointerdown", mouseDown);
    window.addEventListener("pointerup", mouseUp);
    return () => {
      window.removeEventListener("touchstart", touchStart);
      window.removeEventListener("touchend", touchEnd);
      window.removeEventListener("touchcancel", touchCancel);
      window.removeEventListener("pointerdown", mouseDown);
      window.removeEventListener("pointerup", mouseUp);
    };
  }, [enabled, stage]);
}

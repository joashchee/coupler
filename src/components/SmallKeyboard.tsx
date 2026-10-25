/**
 * The small tablet screen's keyboard (lib/softKeys.ts, `smallKeyRows`;
 * an iPhone in the web build): in the lower part whenever a text field
 * has the focus, in place of the touch controls. Five rows of keys three
 * rows tall (48 px, the least a finger hits), ten places across the
 * desk's width in whole cells, the labels in the game output's own text
 * size.
 *
 * As on the iPhone's own keyboard, a key goes on lifting the finger, not
 * on touching: while it's down the key under it shows above it, twice
 * the size, and sliding to another key moves to that one; lifting off
 * the keyboard types nothing. Backspace, the arrows and Space held still
 * go again and again after a moment, and then lifting adds no more.
 *
 * Shift, Ctrl, Option and Cmd hold for the next key, then let go; a
 * second press lets go at once. A held key is in yellow and pressed for
 * VoiceOver (rule 10), which presses a key by a click alone. The keys
 * never take the focus (it stays in the field), so they're skipped by
 * Tab; VoiceOver reaches each by touch.
 */
import { useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { prefix } from "../lib/commands";
import { NONE_HELD, press, smallKeyRows, spread, type Field, type Held, type SoftKey } from "../lib/softKeys";

/** How long a key is held before it repeats, and how often it does then, in ms. */
const REPEAT_AFTER = 400;
const REPEAT_EVERY = 60;
/** A key's height, and the shown key's above it. */
const KEY_HEIGHT = 48;
/** A click this soon after a finger lifted is that press's own, not another (VoiceOver's come alone). */
const CLICK_AFTER = 600;

interface SmallKeyboardProps {
  /** The field typed in last, or null: the keys go to whatever has the focus. */
  field: () => Field | null;
  /** The desk's width in cells. */
  columns: number;
}

interface Placed {
  k: SoftKey;
  x: number;
  y: number;
  width: number;
}

export function SmallKeyboard({ field, columns }: SmallKeyboardProps) {
  const [held, setHeld] = useState<Held>(NONE_HELD);
  const heldRef = useRef(held);
  heldRef.current = held;
  /** The key under the finger, by its place in `placed`. */
  const [touched, setTouched] = useState<number | null>(null);
  const touchedRef = useRef(touched);
  touchedRef.current = touched;
  const repeat = useRef<{ wait: number; every: number; went: boolean } | null>(null);
  const lifted = useRef(0);
  const pressing = useRef(false);
  const box = useRef<HTMLDivElement>(null);

  const places = spread(columns, 10);
  const placed: Placed[] = smallKeyRows(prefix()).flatMap((row, r) => {
    let at = 0;
    let x = 0;
    return row.map((k) => {
      const width = places.slice(at, at + k.cells).reduce((a, b) => a + b, 0) * 8;
      const key = { k, x, y: r * KEY_HEIGHT, width };
      at += k.cells;
      x += width;
      return key;
    });
  });
  const labelOf = (k: SoftKey) => (held.shift && k.shifted ? k.shifted : k.label);

  /** Stops a key repeating; whether it went again while held. */
  const stopRepeat = () => {
    const was = repeat.current;
    if (!was) return false;
    window.clearTimeout(was.wait);
    window.clearInterval(was.every);
    repeat.current = null;
    return was.went;
  };
  useEffect(() => () => void stopRepeat(), []);

  const act = (k: SoftKey) => {
    if (k.modifier) {
      const m = k.modifier;
      setHeld((was) => ({ ...was, [m]: !was[m] }));
      return;
    }
    press(k, heldRef.current, field());
    if (Object.values(heldRef.current).some(Boolean)) setHeld(NONE_HELD);
  };

  const startRepeat = (at: number | null) => {
    stopRepeat();
    if (at === null || !placed[at].k.repeats) return;
    const k = placed[at].k;
    const wait = window.setTimeout(() => {
      const now = repeat.current;
      if (!now) return;
      now.went = true;
      press(k, NONE_HELD, field());
      now.every = window.setInterval(() => press(k, NONE_HELD, field()), REPEAT_EVERY);
    }, REPEAT_AFTER);
    repeat.current = { wait, every: 0, went: false };
  };

  /** The key under a point, or null off the keys. */
  const under = (e: PointerEvent<HTMLDivElement>): number | null => {
    const el = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>("[data-place]");
    return el && box.current?.contains(el) ? Number(el.dataset.place) : null;
  };

  const down = (e: PointerEvent<HTMLDivElement>) => {
    // The field keeps the focus; the key goes when the finger lifts.
    e.preventDefault();
    pressing.current = true;
    box.current?.setPointerCapture(e.pointerId);
    const at = under(e);
    setTouched(at);
    startRepeat(at);
  };
  const move = (e: PointerEvent<HTMLDivElement>) => {
    if (!pressing.current) return;
    const at = under(e);
    if (at === touchedRef.current) return;
    setTouched(at);
    startRepeat(at);
  };
  const up = () => {
    if (!pressing.current) return;
    pressing.current = false;
    const at = touchedRef.current;
    const went = stopRepeat();
    setTouched(null);
    lifted.current = Date.now();
    if (at !== null && !went) act(placed[at].k);
  };
  const cancel = () => {
    pressing.current = false;
    stopRepeat();
    setTouched(null);
  };

  const shown = touched === null ? null : placed[touched];
  const shownLabel = shown ? labelOf(shown.k) : "";
  const shownWidth = shown ? Math.max(shown.width, (shownLabel.length + 1) * 16) : 0;
  const shownLeft = shown ? Math.max(0, Math.min(columns * 8 - shownWidth, Math.round((shown.x + shown.width / 2 - shownWidth / 2) / 8) * 8)) : 0;

  return (
    <div
      className="small-keyboard"
      ref={box}
      role="group"
      aria-label="Keyboard"
      data-testid="tablet-keyboard"
      style={{ width: columns * 8, height: 5 * KEY_HEIGHT }}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={cancel}
      onMouseDown={(e) => e.preventDefault()}
    >
      {placed.map(({ k, x, y, width }, i) => {
        const label = labelOf(k);
        const cells = width / 8;
        const left = Math.max(0, Math.floor((cells - label.length) / 2)) * 8;
        const on = !!k.modifier && held[k.modifier];
        return (
          <button
            key={`${k.code || k.key}-${i}`}
            type="button"
            tabIndex={-1}
            data-place={i}
            className={`tablet-key${on ? " held" : ""}${k.modifier || k.label.length > 1 ? " named" : ""}${touched === i ? " touched" : ""}`}
            data-testid={`key-${k.code || k.key}`}
            aria-label={k.name ?? label}
            aria-pressed={k.modifier ? on : undefined}
            style={{ left: x, top: y, width, "--left": `${left}px` } as CSSProperties}
            onClick={() => {
              // VoiceOver and Switch Control press by a click alone.
              if (Date.now() - lifted.current > CLICK_AFTER) act(k);
            }}
          >
            {label}
          </button>
        );
      })}
      {shown && (
        <div className="small-key-shown" aria-hidden="true" style={{ left: shownLeft, top: shown.y - KEY_HEIGHT, width: shownWidth, paddingLeft: Math.floor((shownWidth - shownLabel.length * 16) / 16) * 8 }}>
          {shownLabel}
        </div>
      )}
    </div>
  );
}

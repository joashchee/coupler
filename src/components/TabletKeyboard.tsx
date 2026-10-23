/**
 * The tablet's keyboard (lib/softKeys.ts): in the lower part of the
 * screen whenever a text field has the focus, in place of the touch
 * controls. 96 cells by 18 rows, six rows of keys three rows tall (48
 * px, the least a finger hits), each key's label on its middle row,
 * left by whole cells.
 *
 * Shift, Ctrl, Option and Cmd hold for the next key, then let go; a
 * second tap lets go at once. A held key is in yellow and pressed for
 * VoiceOver (rule 10). Backspace, Delete, the arrows and Space go again
 * while held. The keys never take the focus (it stays in the field), so
 * they're skipped by Tab; VoiceOver reaches each by touch.
 */
import { useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { KEY_ROWS, NONE_HELD, press, type Field, type Held, type SoftKey } from "../lib/softKeys";

/** How long a key is held before it repeats, and how often it does then, in ms. */
const REPEAT_AFTER = 400;
const REPEAT_EVERY = 60;

interface TabletKeyboardProps {
  /** The field typed in last, or null: the keys go to whatever has the focus. */
  field: () => Field | null;
  /** Hide was pressed: the field lets go of the focus and the controls come back. */
  onHide: () => void;
}

export function TabletKeyboard({ field, onHide }: TabletKeyboardProps) {
  const [held, setHeld] = useState<Held>(NONE_HELD);
  const heldRef = useRef(held);
  heldRef.current = held;
  const repeat = useRef<{ wait: number; every: number } | null>(null);
  /** A press by touch or pointer already acted: its click is not another press. */
  const pointed = useRef(false);

  const stopRepeat = () => {
    if (!repeat.current) return;
    window.clearTimeout(repeat.current.wait);
    window.clearInterval(repeat.current.every);
    repeat.current = null;
  };
  useEffect(() => stopRepeat, []);

  const act = (k: SoftKey) => {
    if (k.hide) {
      onHide();
      return;
    }
    if (k.modifier) {
      const m = k.modifier;
      setHeld((was) => ({ ...was, [m]: !was[m] }));
      return;
    }
    press(k, heldRef.current, field());
    if (Object.values(heldRef.current).some(Boolean)) setHeld(NONE_HELD);
  };

  const down = (k: SoftKey) => (e: PointerEvent<HTMLButtonElement>) => {
    // The field keeps the focus, and the press is now, not on lifting the finger.
    e.preventDefault();
    pointed.current = true;
    act(k);
    if (!k.repeats) return;
    stopRepeat();
    const wait = window.setTimeout(() => {
      if (repeat.current) repeat.current.every = window.setInterval(() => press(k, NONE_HELD, field()), REPEAT_EVERY);
    }, REPEAT_AFTER);
    repeat.current = { wait, every: 0 };
  };

  return (
    <div className="tablet-keyboard" role="group" aria-label="Keyboard" data-testid="tablet-keyboard">
      {KEY_ROWS.map((row, r) => (
        <div className="tablet-key-row" key={r}>
          {row.map((k, i) => {
            const label = held.shift && k.shifted ? k.shifted : k.label;
            const left = Math.max(0, Math.floor((k.cells - label.length) / 2)) * 8;
            const on = !!k.modifier && held[k.modifier];
            return (
              <button
                key={`${k.code}-${i}`}
                type="button"
                tabIndex={-1}
                className={`tablet-key${on ? " held" : ""}${k.modifier || k.label.length > 1 ? " named" : ""}`}
                data-testid={`key-${k.code || k.key}`}
                aria-label={k.name ?? label}
                aria-pressed={k.modifier ? on : undefined}
                style={{ width: k.cells * 8, "--left": `${left}px` } as CSSProperties}
                onPointerDown={down(k)}
                onPointerUp={stopRepeat}
                onPointerCancel={stopRepeat}
                onPointerLeave={stopRepeat}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  // VoiceOver and Switch Control press by a click alone.
                  if (pointed.current) pointed.current = false;
                  else act(k);
                }}
              >
                {label}
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}

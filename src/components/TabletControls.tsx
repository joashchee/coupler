/**
 * The tablet's touch controls: the lower part of the screen when no text
 * field has the focus, in place of the keyboard (components/
 * TabletKeyboard.tsx), 96 cells by 18 rows. Each control sits at its
 * column and row of keys (three rows each, 48 px), `cells` wide, its
 * label on its middle row, left by whole cells. The order given is the
 * reading order, whatever the places.
 *
 * `lit` marks a control in yellow (an exit the map knows), and it's in
 * its name too (rule 10), never only in its look. `pressed` (Open or
 * Close waiting for a way) is yellow as a held modifier is on the
 * keyboard, and `aria-pressed`.
 */
import type { CSSProperties } from "react";

export interface TabletControl {
  id: string;
  label: string;
  /** What VoiceOver says: "North, an exit". */
  name: string;
  /** Its column (0 to 95) and row of keys (0 to 5; the small screen's, its columns and 0 to 4). */
  column: number;
  row: number;
  /** Cells wide; 6 when left out. */
  cells?: number;
  lit?: boolean;
  /** A key that stays down till it's used (Open, Close): pressed, for VoiceOver too. */
  pressed?: boolean;
  disabled?: boolean;
  onPress: () => void;
}

interface TabletControlsProps {
  controls: TabletControl[];
  /** The small tablet screen's: its width in pixels and its rows of keys (96 cells by 6 when left out). */
  width?: number;
  rows?: number;
}

export function TabletControls({ controls, width, rows }: TabletControlsProps) {
  return (
    <div className="tablet-controls" role="group" aria-label="Controls" data-testid="tablet-controls" style={width ? { width, height: (rows ?? 6) * 48 } : undefined}>
      {controls.map((c) => {
        const cells = c.cells ?? 6;
        const left = Math.max(0, Math.floor((cells - c.label.length) / 2)) * 8;
        return (
          <button
            key={c.id}
            type="button"
            className={`tablet-key${c.lit ? " exit" : ""}${c.pressed ? " held" : ""}`}
            data-testid={`control-${c.id}`}
            aria-label={c.name}
            aria-pressed={c.pressed}
            disabled={c.disabled}
            style={{ left: c.column * 8, top: c.row * 48, width: cells * 8, "--left": `${left}px` } as CSSProperties}
            onClick={c.onPress}
          >
            {c.label}
          </button>
        );
      })}
    </div>
  );
}

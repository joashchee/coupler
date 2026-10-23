/**
 * The tablet's touch controls: the lower part of the screen when no text
 * field has the focus, in place of the keyboard (components/
 * TabletKeyboard.tsx), 96 cells by 18 rows. Each control sits at its
 * column and row of keys (three rows each, 48 px), `cells` wide, its
 * label on its middle row, left by whole cells. The order given is the
 * reading order, whatever the places.
 *
 * `lit` marks a control in yellow (an exit the map knows), and it's in
 * its name too (rule 10), never only in its look.
 */
import type { CSSProperties } from "react";

export interface TabletControl {
  id: string;
  label: string;
  /** What VoiceOver says: "North, an exit". */
  name: string;
  /** Its column (0 to 95) and row of keys (0 to 5). */
  column: number;
  row: number;
  /** Cells wide; 6 when left out. */
  cells?: number;
  lit?: boolean;
  disabled?: boolean;
  onPress: () => void;
}

interface TabletControlsProps {
  controls: TabletControl[];
}

export function TabletControls({ controls }: TabletControlsProps) {
  return (
    <div className="tablet-controls" role="group" aria-label="Controls" data-testid="tablet-controls">
      {controls.map((c) => {
        const cells = c.cells ?? 6;
        const left = Math.max(0, Math.floor((cells - c.label.length) / 2)) * 8;
        return (
          <button
            key={c.id}
            type="button"
            className={`tablet-key${c.lit ? " exit" : ""}`}
            data-testid={`control-${c.id}`}
            aria-label={c.name}
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

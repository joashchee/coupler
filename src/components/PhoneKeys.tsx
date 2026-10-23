/**
 * The phone's keys (lib/phoneSections.ts): the row above the command
 * line, its buttons the section's own. Each is 48 px tall, the least a
 * finger hits, and `cells` wide (6 by default, 48 px); its label sits
 * in the middle of its cells on the theme's grid, left by whole cells.
 *
 * `lit` marks a key in yellow (an exit the map knows); it's also in its
 * name (rule 10), never only in its look.
 */
import type { CSSProperties } from "react";

export interface PhoneKey {
  id: string;
  label: string;
  /** What VoiceOver says: "North, an exit". */
  name: string;
  /** Cells wide; 6 when left out. */
  cells?: number;
  lit?: boolean;
  disabled?: boolean;
  onPress: () => void;
}

interface PhoneKeysProps {
  keys: PhoneKey[];
  /** What the row is, for VoiceOver: "Touch pad". */
  label: string;
  /** Nothing in the row can be pressed: offline, or a password asked. */
  disabled?: boolean;
}

export function PhoneKeys({ keys, label, disabled = false }: PhoneKeysProps) {
  return (
    <div className="touch-pad" role="toolbar" aria-label={label} data-testid="touch-pad">
      {keys.map((k) => {
        const cells = k.cells ?? 6;
        const left = Math.max(0, Math.floor((cells - k.label.length) / 2)) * 8;
        return (
          <button
            key={k.id}
            type="button"
            className={k.lit ? "exit" : undefined}
            data-testid={`pad-${k.id}`}
            aria-label={k.name}
            disabled={disabled || k.disabled}
            style={{ width: cells * 8, "--left": `${left}px` } as CSSProperties}
            onClick={k.onPress}
          >
            {k.label}
          </button>
        );
      })}
    </div>
  );
}

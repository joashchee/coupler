/**
 * The phone's bar (lib/phoneSections.ts): ◄ the section before, the
 * section's name and the connection, ► the next, ▼ the sheet of options
 * (what a swipe down from the top does). The gear sits after it, its
 * own Movable. Each button 48 by 48.
 */
import { PHONE_SECTIONS, type PhoneSection } from "../lib/phoneSections";

interface PhoneBarProps {
  section: PhoneSection;
  /** "Online: Standard", "Offline". */
  state: string;
  online: boolean;
  sheetDown: boolean;
  onTurn: (by: 1 | -1) => void;
  onSheet: () => void;
}

export function PhoneBar({ section, state, online, sheetDown, onTurn, onSheet }: PhoneBarProps) {
  const at = PHONE_SECTIONS.findIndex((s) => s.id === section);
  const here = PHONE_SECTIONS[at];
  const n = PHONE_SECTIONS.length;
  const before = PHONE_SECTIONS[(at - 1 + n) % n];
  const after = PHONE_SECTIONS[(at + 1) % n];
  return (
    <nav className="phone-bar" aria-label="Sections" data-testid="phone-bar">
      <button type="button" className="phone-turn" data-testid="phone-prev" aria-label={`Before: ${before.name}`} onClick={() => onTurn(-1)}>
        {"◄"}
      </button>
      <h2 className="phone-title" data-testid="phone-section" aria-label={`${here.name}, ${at + 1} of ${n}. ${state}`}>
        <span>{`${here.name} ${at + 1}/${n}`}</span>
        <span className={`phone-online${online ? " on" : ""}`}>{online ? "Online" : "Offline"}</span>
      </h2>
      <button type="button" className="phone-turn" data-testid="phone-next" aria-label={`Next: ${after.name}`} onClick={() => onTurn(1)}>
        {"►"}
      </button>
      <button type="button" className="phone-turn" data-testid="phone-sheet-button" aria-label="Options" aria-expanded={sheetDown} onClick={onSheet}>
        {"▼"}
      </button>
    </nav>
  );
}

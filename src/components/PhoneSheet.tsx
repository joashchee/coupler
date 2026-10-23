/**
 * The phone's sheet of options (lib/phoneSections.ts), pulled down from
 * the top by a swipe or the bar's ▼: the general options first, then the
 * section's own. A dialog (focus in, Esc, focus back: rule 10), drawn
 * from under the bar, the phone's width; a swipe up, Close or a tap
 * below it puts it back. Choosing one closes it first.
 */
import { Dialog } from "./Dialog";

export interface SheetItem {
  id: string;
  label: string;
  onPress: () => void;
  disabled?: boolean;
}

interface PhoneSheetProps {
  open: boolean;
  onClose: () => void;
  /** "Online: Standard", "Offline". */
  state: string;
  general: SheetItem[];
  /** The section's name, and its options. */
  sectionName: string;
  own: SheetItem[];
}

export function PhoneSheet({ open, onClose, state, general, sectionName, own }: PhoneSheetProps) {
  const list = (items: SheetItem[]) => (
    <ul className="phone-sheet-list">
      {items.map((item) => (
        <li key={item.id}>
          <button
            type="button"
            data-testid={`sheet-${item.id}`}
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onPress();
            }}
          >
            {item.label}
          </button>
        </li>
      ))}
    </ul>
  );
  return (
    <Dialog open={open} onClose={onClose} title="Options" className="phone-sheet">
      <p className="phone-sheet-state">{state}</p>
      <h3>General</h3>
      {list(general)}
      <h3>{sectionName}</h3>
      {list(own)}
    </Dialog>
  );
}

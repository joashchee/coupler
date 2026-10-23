/**
 * The tablet screen's text size (lib/formFactor.ts, `TABLET_SCREEN`):
 * how big the upper desk's cells are, so how many columns and rows it
 * has (lib/stage.ts, `tabletFit`). The lower part, the command line and
 * the keyboard, is one fixed size whatever this is. Cmd+Plus and
 * Cmd+Minus, or the gear menu, change it; the page itself never zooms.
 */
import { CELL_HEIGHT, CELL_WIDTH, TABLET_TEXT_SIZES, type TabletTextSize } from "./stage";

const TEXT_KEY = "coupler.tabletText";

export function loadTabletText(): TabletTextSize {
  try {
    const kept = Number(localStorage.getItem(TEXT_KEY));
    return TABLET_TEXT_SIZES.find((size) => size === kept) ?? 1;
  } catch {
    return 1;
  }
}

export function storeTabletText(size: TabletTextSize) {
  try {
    localStorage.setItem(TEXT_KEY, String(size));
  } catch {
    // The usual size again next time.
  }
}

/** The next size up (`by` 1) or down (-1), or the same at either end. */
export function stepTabletText(size: TabletTextSize, by: 1 | -1): TabletTextSize {
  const at = TABLET_TEXT_SIZES.indexOf(size);
  return TABLET_TEXT_SIZES[Math.min(TABLET_TEXT_SIZES.length - 1, Math.max(0, at + by))];
}

/** "Text 12 by 24 points: 64 columns by 27 rows." */
export function tabletTextInWords(size: TabletTextSize, columns: number, rows: number): string {
  return `Text ${CELL_WIDTH * size} by ${CELL_HEIGHT * size} points: ${columns} columns by ${rows} rows.`;
}

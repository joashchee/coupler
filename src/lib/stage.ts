/**
 * The stage: Coupler's window is a game screen of one fixed size, so
 * every control and picture has a fixed place. The window can't be
 * resized; full screen is the only other mode, and it scales the same
 * stage up to fit.
 *
 * 1280 by 720 because it's the largest standard size that fits every
 * desktop still in common use, and it divides exactly into the theme's
 * 8 by 16 character cell: 160 columns by 45 rows.
 */
export const STAGE_WIDTH = 1280;
export const STAGE_HEIGHT = 720;
export const CELL_WIDTH = 8;
export const CELL_HEIGHT = 16;
export const STAGE_COLUMNS = STAGE_WIDTH / CELL_WIDTH;
export const STAGE_ROWS = STAGE_HEIGHT / CELL_HEIGHT;

/** How much the stage is scaled to fit the viewport: 1 in the window, more in full screen. */
export function stageScale(): number {
  const scale = Math.min(window.innerWidth / STAGE_WIDTH, window.innerHeight / STAGE_HEIGHT);
  // A viewport a fraction of a pixel off (a display scale factor) stays crisp at exactly 1.
  return Math.abs(scale - 1) < 0.005 ? 1 : scale;
}

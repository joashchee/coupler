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

/**
 * The phone's screen (lib/formFactor.ts): 48 cells across, 384 stage
 * pixels, which an iPhone shows a little over 1 to 1 (375 to 440 points
 * wide), so the font stays near its own size. Its height is as many
 * whole rows as the phone shows above the on-screen keyboard: the
 * layout (`phoneLayout` in App.tsx) gives the game output what's left
 * after the bars, so the command line is never under the keyboard.
 */
export const PHONE_WIDTH = 384;
export const PHONE_COLUMNS = PHONE_WIDTH / CELL_WIDTH;
/** Fewest rows the phone's layout is made for: the bars and a few lines of the game. */
export const PHONE_LEAST_ROWS = 20;
/** The scale is never less than this many rows fitting the full height, so the dev preview on a Mac isn't huge. */
const PHONE_FIT_ROWS = 36;

export interface PhoneFit {
  scale: number;
  rows: number;
}

/**
 * The phone stage's scale and rows. The scale comes from the full
 * height (the keyboard showing never shrinks everything); the rows from
 * the height still showing, `visible`.
 */
export function phoneFit(width: number, height: number, visible: number): PhoneFit {
  const scale = Math.min(width / PHONE_WIDTH, height / (PHONE_FIT_ROWS * CELL_HEIGHT));
  const rows = Math.max(PHONE_LEAST_ROWS, Math.floor(visible / scale / CELL_HEIGHT));
  return { scale: Math.abs(scale - 1) < 0.005 ? 1 : scale, rows };
}

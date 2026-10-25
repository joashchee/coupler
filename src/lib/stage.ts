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

/** The phone screen tried on a tablet (lib/formFactor.ts, `PHONE_EMULATED`): this many rows tall, a phone's shape upright (19.5 by 9). */
const PHONE_EMULATED_ROWS = 52;

/**
 * The phone stage's scale and rows. The scale comes from the full
 * height (the keyboard showing never shrinks everything); the rows from
 * the height still showing, `visible`. `emulated`, on a tablet, it's
 * scaled to a phone's height, so it keeps a phone's shape.
 */
export function phoneFit(width: number, height: number, visible: number, emulated = false): PhoneFit {
  const scale = emulated
    ? Math.min(width / PHONE_WIDTH, height / (PHONE_EMULATED_ROWS * CELL_HEIGHT))
    : Math.min(width / PHONE_WIDTH, height / (PHONE_FIT_ROWS * CELL_HEIGHT));
  const rows = Math.max(PHONE_LEAST_ROWS, Math.floor(visible / scale / CELL_HEIGHT));
  return { scale: Math.abs(scale - 1) < 0.005 ? 1 : scale, rows };
}

/**
 * The tablet's screen (lib/formFactor.ts, `TABLET_SCREEN`), upright: two
 * stages, one over the other. The lower is one fixed size whatever the
 * tablet, 96 cells by 23 rows (the command line's 3, the keyboard's 18,
 * the messages' 2), scaled to the screen's width; on a 768-point iPad
 * that's a little over a third of the height. The upper is Workshop's
 * desk in what's left, its cells the player's text size
 * (`TABLET_TEXT_SIZES`): bigger text, fewer columns and rows.
 */
export const TABLET_LOWER_WIDTH = 768;
export const TABLET_LOWER_ROWS = 23;
export const TABLET_LOWER_HEIGHT = TABLET_LOWER_ROWS * CELL_HEIGHT;
/** The fewest columns and rows the upper desk is made for, whatever the text size. */
const TABLET_LEAST_COLUMNS = 40;
const TABLET_LEAST_ROWS = 16;
/** An iPad upright, in points: the tablet screen tried on a computer is this, fitted to the window. */
export const TABLET_EMULATED_SIZE = { width: 768, height: 1024 };
/** An iPhone upright in Safari, in points, what's left under its bars: the small tablet screen tried on a computer. */
export const TABLET_SMALL_EMULATED_SIZE = { width: 390, height: 664 };
/**
 * The small tablet screen's lower part (lib/formFactor.ts,
 * `TABLET_SMALL`: an iPhone in the web build): the desk's width in its
 * cells, so the keys' text is the game output's size, and 20 rows (the
 * command line's 3, the keyboard's 15, the messages' 2).
 */
export const TABLET_SMALL_LOWER_ROWS = 20;

/** The text sizes the tablet's upper desk offers: how many times the theme's 8 by 16 cell each is. */
export const TABLET_TEXT_SIZES = [1, 1.25, 1.5, 1.75, 2] as const;
export type TabletTextSize = (typeof TABLET_TEXT_SIZES)[number];

export interface TabletFit {
  /** The upper desk's text size, its whole cells across and down, and where it starts across (centered on what's left over). */
  text: TabletTextSize;
  /** The largest text size that still leaves the desk its fewest columns: on an iPhone (the web build's), the smallest only. */
  largest: TabletTextSize;
  columns: number;
  rows: number;
  left: number;
  /** The lower stage's size in its own pixels, its scale, and its top and left, in the frame's points. */
  lowerWidth: number;
  lowerHeight: number;
  lowerScale: number;
  lowerTop: number;
  lowerLeft: number;
  /** Held sideways: the tablet screen is upright only. */
  sideways: boolean;
}

/**
 * The tablet's two stages in a frame `width` by `height` points, the
 * text at `asked` or the largest under it that fits. `small` (an iPhone
 * in the web build): the lower part is the desk's columns of the same
 * text, not 96 cells shrunk to the width.
 */
export function tabletFit(width: number, height: number, asked: TabletTextSize, small = false): TabletFit {
  const fits = TABLET_TEXT_SIZES.filter((size) => size === 1 || width / (CELL_WIDTH * size) >= TABLET_LEAST_COLUMNS);
  const largest = fits[fits.length - 1];
  const text = Math.min(asked, largest) as TabletTextSize;
  const columns = Math.max(TABLET_LEAST_COLUMNS, Math.floor(width / (CELL_WIDTH * text)));
  const left = Math.max(0, (width - columns * CELL_WIDTH * text) / 2);
  const lowerWidth = small ? columns * CELL_WIDTH : TABLET_LOWER_WIDTH;
  const lowerHeight = small ? TABLET_SMALL_LOWER_ROWS * CELL_HEIGHT : TABLET_LOWER_HEIGHT;
  const lowerScale = small ? text : width / TABLET_LOWER_WIDTH;
  const lowerTop = Math.max(0, height - lowerHeight * lowerScale);
  const rows = Math.max(TABLET_LEAST_ROWS, Math.floor(lowerTop / (CELL_HEIGHT * text)));
  return { text, largest, columns, rows, left, lowerWidth, lowerHeight, lowerScale, lowerTop, lowerLeft: small ? left : 0, sideways: width > height };
}

// CP437 box-drawing characters as arms: [up, right, down, left], each 0 (none), 1 (single line) or 2 (double). Drawing a line over
// an existing one merges the arms, so crossings and joins come out as the right character (a vertical meeting a horizontal is a T).
const ARMS = {
  '─': [0, 1, 0, 1], '│': [1, 0, 1, 0], '┌': [0, 1, 1, 0], '┐': [0, 0, 1, 1], '└': [1, 1, 0, 0], '┘': [1, 0, 0, 1],
  '├': [1, 1, 1, 0], '┤': [1, 0, 1, 1], '┬': [0, 1, 1, 1], '┴': [1, 1, 0, 1], '┼': [1, 1, 1, 1],
  '═': [0, 2, 0, 2], '║': [2, 0, 2, 0], '╔': [0, 2, 2, 0], '╗': [0, 0, 2, 2], '╚': [2, 2, 0, 0], '╝': [2, 0, 0, 2],
  '╠': [2, 2, 2, 0], '╣': [2, 0, 2, 2], '╦': [0, 2, 2, 2], '╩': [2, 2, 0, 2], '╬': [2, 2, 2, 2],
  // Double horizontal with single vertical, and the reverse.
  '╒': [0, 2, 1, 0], '╕': [0, 0, 1, 2], '╘': [1, 2, 0, 0], '╛': [1, 0, 0, 2], '╞': [1, 2, 1, 0], '╡': [1, 0, 1, 2], '╤': [0, 2, 1, 2], '╧': [1, 2, 0, 2], '╪': [1, 2, 1, 2],
  '╓': [0, 1, 2, 0], '╖': [0, 0, 2, 1], '╙': [2, 1, 0, 0], '╜': [2, 0, 0, 1], '╟': [2, 1, 2, 0], '╢': [2, 0, 2, 1], '╥': [0, 1, 2, 1], '╨': [2, 1, 0, 1], '╫': [2, 1, 2, 1],
};
const BY_CODE = new Map(Object.entries(ARMS).map(([ch, a]) => [ch.codePointAt(0), a]));
const BY_ARMS = new Map(Object.entries(ARMS).map(([ch, a]) => [a.join(''), ch.codePointAt(0)]));

/** The arms of a box-drawing code point, or undefined for any other character. */
export const armsOf = (cp) => BY_CODE.get(cp);
/** The code point with exactly these arms, or undefined (CP437 has no single-up, double-down piece, for instance). */
export const charOf = (arms) => BY_ARMS.get(arms.join(''));

/** Line weights by box style: `h` for horizontal lines, `v` for vertical. `mixed` is double horizontals with single verticals. */
export const WEIGHTS = Object.freeze({ single: { h: 1, v: 1 }, double: { h: 2, v: 2 }, mixed: { h: 2, v: 1 } });

/** The code point to put where a piece with arms `n` meets whatever character `existing` is: the arms merged (the new piece wins
 *  an arm both have), else the new piece alone, else what was there. */
export function join(existing, n) {
  const e = armsOf(existing);
  const m = e ? n.map((v, k) => v || e[k]) : n;
  return charOf(m) ?? charOf(n) ?? existing;
}

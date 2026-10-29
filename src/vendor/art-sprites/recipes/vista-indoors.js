// The indoor half of `vista`: a room in one-point perspective laid in its material (bricks, planks, panels, runes), a cave, a lake or
// a chasm, drawn on the half-block pixel canvas. The wall, joint, side, floor and ceiling colours are Coupler's `materials` table
// (docs/coupler-handover): ideas ported, no code. Browser-safe.
import { rng } from '../rng.js';
import { Px, BAYER, between, texture, clamp, ridgeLine, K, RED, GRN, BRN, BLU, MAG, CYN, GRY, DGR, LRED, YEL, LBLU, LCYN, WHT, LIT } from './vista-canvas.js';

// terrain: [wall, joint, side, floor, floorJoint, how the back wall is laid]. For bricks the wall is the mortar and the joint the brick.
const MATERIALS = {
  stone: [DGR, GRY, DGR, GRY, DGR, 'bricks'], wooden: [BRN, RED, RED, BRN, K, 'planks'], metal: [GRY, DGR, DGR, DGR, K, 'panels'],
  magic: [MAG, 13, BLU, BLU, MAG, 'runes'], in_underwater: [BLU, CYN, BLU, CYN, BLU, 'bricks'], innerseaport: [DGR, GRY, DGR, GRY, DGR, 'bricks'],
};
const BANNER = { stone: BLU, wooden: RED, metal: BLU, magic: BLU, in_underwater: RED, innerseaport: BLU };

/** Draw an indoor picture: `{ terrain, high, seed, columns, rows, map }`; returns Cells. */
export function drawIndoors({ terrain, high, seed, columns, rows, map, depth = 16, raw = false }) {
  const W = columns, H = rows * 2, px = new Px(W, H, depth === 256), r = rng(`${seed}:${terrain}`), s = Math.max(0.25, H / 48), later = [];
  if (terrain === 'cave' || terrain === 'cavelakesurface' || terrain === 'caveseaport') cave(px, r, terrain, W, H, s, later);
  else if (terrain === 'gap') gap(px, r, W, H, s);
  else room(px, r, terrain, W, H, s, high, later);
  for (const fn of later) fn();
  texture(px, r.fork('texture').int(1 << 30), 0, { land: false });
  return raw ? px : px.toCells(Array.from({ length: 256 }, (_, i) => i));
}

function room(px, r, terrain, W, H, s, high, later) {
  const [wall, joint, side, floor, floorJoint, laid] = MATERIALS[terrain];
  const x0 = Math.floor(W * 0.2), x1 = W - x0, y0 = Math.floor(H * 0.17), y1 = Math.floor(H * 0.68);
  const door = x0 + Math.round((x1 - x0) * 0.3) + r.int(Math.max(1, Math.round((x1 - x0) * 0.1))), dw = Math.max(3, Math.round(4 * s)), dh = Math.round((y1 - y0) * 0.66);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const t = x < x0 ? (x0 - x) / x0 : x >= x1 ? (x - x1 + 1) / x0 : 0, top = y0 * (1 - t), bottom = y1 + t * (H - y1);   // the side walls narrow toward the back
    const inBack = x >= x0 && x < x1 && y >= y0 && y <= y1;
    let c;
    if (inBack) c = backWall(laid, x - x0, y - y0, wall, joint, s);
    else if (y < top) c = y > top - 3 && t > 0 ? px.mix(x, y, K, side, 4) : K;                                              // the ceiling, dithered where it meets the wall
    else if (y > bottom || (x >= x0 && x < x1 && y > y1)) c = floorAt(x, y, y1, H, floor, floorJoint, laid, s);
    else c = px.mix(x, y, side, K, 8);
    px.set(x, y, c);
  }
  // the door, with a rounded top
  for (let y = y1 - dh; y <= y1; y++) for (let x = door; x < door + dw; x++) if (!(y === y1 - dh && (x === door || x === door + dw - 1))) px.set(x, y, K);
  if (laid === 'runes') for (let i = 0; i < Math.max(6, W / 6); i++) { const x = x0 + 1 + r.int(x1 - x0 - 2), y = y0 + 1 + r.int(Math.max(1, Math.round((y1 - y0) * 0.5))), c = r.pick([LCYN, YEL, WHT, 13]); later.push(() => px.set(x, y, c, LIT)); }
  if (terrain === 'innerseaport') {
    const wy = y1 + Math.max(2, Math.round((H - y1) * 0.25)), wh = Math.max(2, Math.round((H - y1) * 0.18));
    for (let y = wy; y < wy + wh; y++) for (let x = 0; x < W; x++) px.set(x, y, y === wy + wh - 1 ? LBLU : px.mix(x, y, BLU, LBLU, 6));
    px.rect(Math.round(W * 0.8), y1 + 1, Math.round(W * 0.8) + Math.max(3, Math.round(4 * s)), H - 1, BRN);       // the jetty across the channel
  }
  if (terrain === 'magic') {
    const cx = W / 2, cy = y1 + (H - y1) * 0.5, rx = W * 0.2, ry = Math.max(1.5, (H - y1) * 0.22);
    for (let a = 0; a < 96; a++) if (a % 12 < 9) { const t = (a / 96) * Math.PI * 2; later.push(() => px.set(cx + Math.cos(t) * rx, cy + Math.sin(t) * ry, LCYN, LIT)); }
  }
  if (terrain === 'in_underwater') for (let i = 0; i < W * H / 45; i++) { const x = r.int(W), y = r.int(H); later.push(() => px.set(x, y, LCYN, LIT)); }
  // lights: a torch on each side wall; in high fantasy banners on the back wall and braziers on the floor
  const flame = (x, y) => { later.push(() => { glow(px, x, y, Math.max(4, Math.round(7 * s))); px.set(x, y, YEL, LIT); px.set(x, y - 1, LRED, LIT); }); };
  if (high) {
    const bc = BANNER[terrain], bw = Math.max(3, Math.round(4 * s)), bh = Math.round((y1 - y0) * 0.55);
    for (const bx of [x0 + 2, x1 - 2 - bw]) {
      px.rect(bx - 1, y0, bx + bw, y0, YEL); px.rect(bx, y0 + 1, bx + bw - 1, y0 + bh, bc); px.set(bx + bw / 2 - 0.5, y0 + bh + 1, bc); px.set(bx + Math.floor(bw / 2), y0 + Math.round(bh / 2), YEL);
    }
    for (const bx of [Math.round(W * 0.11), Math.round(W * 0.89)]) { const by = H - Math.max(3, Math.round(4 * s)); px.rect(bx, by + 1, bx, by + 2, BRN); flame(bx, by); px.set(bx - 1, by, LRED); px.set(bx + 1, by, LRED); }
  } else {
    for (const tx of [Math.round(W * 0.11), Math.round(W * 0.89)]) {
      const ty = Math.round(H * 0.33), a = Math.max(1, Math.round(s));
      px.rect(tx - a, ty + 1, tx + a, ty + 1 + a, BRN); px.set(tx - a - 1, ty, BRN); px.set(tx + a + 1, ty, BRN); px.set(tx, ty + 2 + a, BRN);
      flame(tx, ty);
    }
  }
}
const backWall = (laid, x, y, wall, joint, s) => {
  if (laid === 'bricks') { const row = Math.floor(y / 4), off = row % 2 ? 4 : 0; return y % 4 === 3 || (x + off) % 8 === 7 ? wall : joint; }
  if (laid === 'planks') return x % 6 < 2 ? joint : wall;
  if (laid === 'panels') return y % 6 < 2 || x % 6 < 2 ? joint : wall;
  return wall;
};
const floorAt = (x, y, y1, H, floor, floorJoint, laid, s) => {
  const d = y - y1, row = Math.floor(d / 3), off = (row % 2) * 4, sp = 8 + Math.floor(d / 5) * 2;
  if (d % 3 === 0) return floorJoint;
  return (x + off) % sp === 0 ? floorJoint : floor;
};
// A warm glow on what is near a flame: a dither toward the bright version of each colour.
function glow(px, x, y, rad) { for (let yy = y - rad; yy <= y + rad; yy++) for (let xx = x - rad; xx <= x + rad; xx++) { const d = Math.hypot(xx - x, yy - y); const t = Math.round(5 * (1 - d / rad)); if (d < rad && !px.is(xx, yy, LIT) && px.smooth && t > 0) { const c = px.get(xx, yy); px.set(xx, yy, between(c, c === K ? DGR : WHT, Math.min(15, t * 2))); } else if (d < rad && !px.is(xx, yy, LIT) && BAYER[yy & 3][xx & 3] < t) { const c = px.get(xx, yy); if (c < 8 && c > 0) px.set(xx, yy, c + 8); else if (c === K) px.set(xx, yy, DGR); } } }

function cave(px, r, terrain, W, H, s, later) {
  const top = ridgeLine(r.fork('roof'), W, Math.max(2, Math.round(H * 0.08)), 0.3);
  for (let x = 0; x < W; x++) for (let y = 0; y < Math.round(H * 0.08) + top[x]; y++) px.set(x, y, y < 3 ? px.mix(x, y, DGR, BRN, 8) : px.mix(x, y, DGR, BRN, 5));
  for (let i = 0; i < Math.max(4, W / 6); i++) { const x = r.int(W), y = Math.round(H * 0.08) + top[x], len = Math.round((2 + r.int(Math.round(H * 0.3))) * (r.chance(0.15) ? 1.5 : 1)); for (let k = 0; k < len; k++) px.set(x, y + k, DGR); if (len > 8) for (let k = 0; k < len / 2; k++) px.set(x + 1, y + k, DGR); }
  for (let y = 0; y < H; y++) { px.set(0, y, DGR); if (r.chance(0.5)) px.set(1, y, DGR); px.set(W - 1, y, DGR); }
  if (terrain === 'cave') {
    const fl = ridgeLine(r.fork('floor'), W, Math.max(1, Math.round(H * 0.06)), 0.25), base = Math.round(H * 0.9);
    for (let x = 0; x < W; x++) for (let y = base - fl[x]; y < H; y++) px.set(x, y, px.mix(x, y, DGR, BRN, y > base ? 8 : 4));
    for (let i = 0; i < W / 8; i++) { const x = r.int(W); for (let y = base - fl[Math.min(W - 1, x)] - 1 - r.int(4); y < base - fl[Math.min(W - 1, x)]; y++) px.set(x, y, DGR); }
    const fx = Math.round(W * 0.47), fy = base - 1; later.push(() => { px.set(fx, fy, YEL, LIT); px.set(fx, fy - 1, LRED, LIT); px.set(fx - 1, fy, LRED, LIT); px.set(fx + 1, fy, LRED, LIT); glow(px, fx, fy, Math.max(4, Math.round(6 * s))); });
    return;
  }
  const lake = Math.round(H * 0.65);
  for (let y = lake; y < H; y++) for (let x = 0; x < W; x++) px.set(x, y, y > lake + (H - lake) * 0.4 ? px.mix(x, y, BLU, K, 5) : BLU);
  for (let i = 0; i < W * (H - lake) / 50; i++) px.set(r.int(W), lake + 1 + r.int(Math.max(1, H - lake - 1)), LBLU);
  for (let i = 0; i < W / 10; i++) { const x = r.int(W); px.set(x, lake - 1 - r.int(3), K); }
  if (terrain === 'caveseaport') {
    px.rect(0, lake - 1, Math.round(W * 0.32), lake, BRN);
    const bx = Math.round(W * 0.82), m = Math.max(3, Math.round(6 * s));
    px.rect(bx - m, lake + 1, bx + m, lake + 1, BRN); px.rect(bx - m + 1, lake + 2, bx + m - 1, lake + 2, BRN);
    for (let k = 0; k < m; k++) px.rect(bx - Math.round((m - k) * 0.6), lake - k, bx + Math.round((m - k) * 0.6), lake - k, WHT);
    later.push(() => px.set(bx, lake - m - 1, LRED, LIT));
  }
}
function gap(px, r, W, H, s) {
  const cl = Math.round(W * 0.19), cr = Math.round(W * 0.84), top = Math.round(H * 0.7);
  for (let y = top; y < H; y++) { for (let x = 0; x < cl; x++) px.set(x, y, px.mix(x, y, BRN, K, 5)); for (let x = cr; x < W; x++) px.set(x, y, px.mix(x, y, BRN, K, 5)); px.set(cl, y, DGR); px.set(cr - 1, y, DGR); }
  for (let x = 0; x < cl; x++) px.set(x, top, DGR); for (let x = cr; x < W; x++) px.set(x, top, DGR);
  const sag = Math.max(2, Math.round(4 * s));
  for (let x = cl; x < cr; x++) { const t = (x - cl) / (cr - cl - 1), y = top - 1 + Math.round(Math.sin(Math.PI * t) * sag); px.set(x, y, BRN); if ((x - cl) % 6 === 0) { px.set(x, y - 1, BRN); px.set(x, y - 2, BRN); } }
  for (let i = 0; i < W * H / 90; i++) { const x = cl + r.int(cr - cl), y = top + 3 + r.int(Math.max(1, H - top - 3)); px.set(x, y, DGR); }
}

// The outdoor half of `vista`: a picture of open air, drawn the way Coupler's painter does, on a pixel canvas two pixels tall to a cell
// (the half block: top pixel the foreground, bottom the background), so a sky can be solid bands that dither only where one meets the
// next and a ground can carry texture. Colours here are the 16 ANSI indexes in the order of VGA16 (1 red, 4 blue, 8 dark gray ...), the
// same indexes as `V`. Ideas from Coupler's painter.rs tables (`docs/coupler-handover`), no code; browser-safe, no Node built-ins.
import { rng } from '../rng.js';
import { Px, BAYER, between, texture, shade256, clamp, ridgeLine, K, RED, GRN, BRN, BLU, MAG, CYN, GRY, DGR, LRED, LGRN, YEL, LBLU, LCYN, WHT, LIT, WET } from './vista-canvas.js';

/** Sky colour stops from the top, by sky kind and time (Coupler's `skyStops`). */
const SKY = {
  clear: { dawn: [4, 5, 9, 11], day: [4, 12, 14], dusk: [0, 4, 5, 1, 9], night: [0, 0, 4] },
  rain: { dawn: [0, 8, 7], day: [8, 7], dusk: [0, 8, 7], night: [0, 8] },
  storm: { dawn: [0, 8, 8, 7], day: [0, 8, 8, 7], dusk: [0, 8, 8, 7], night: [0, 0, 8] },
  snow: { dawn: [0, 8, 7], day: [8, 7], dusk: [0, 8, 7], night: [0, 8, 7] },
  blizzard: { dawn: [0, 8, 8, 7], day: [0, 8, 8, 7], dusk: [0, 8, 8, 7], night: [0, 0, 8] },
  fog: { dawn: [0, 8, 7], day: [7, 7, 15], dusk: [0, 8, 7], night: [0, 8] },
  dust: { dawn: [3, 11], day: [3, 11], dusk: [3, 11], night: [0, 3] },
  hot: { dawn: [4, 5, 9, 11], day: [12, 14, 11], dusk: [0, 4, 5, 1, 9], night: [0, 0, 4] },
  cold: { dawn: [4, 5, 9, 11], day: [12, 14, 15], dusk: [0, 4, 5, 1, 9], night: [0, 0, 4] },
};
const SKY_OF = { clear: 'clear', cloudy: 'clear', windy: 'clear', rain: 'rain', thunderstorm: 'storm', snow: 'snow', blizzard: 'blizzard', hail: 'rain', sleet: 'rain', fog: 'fog', dust: 'dust', heat: 'hot', drought: 'hot', cold: 'cold' };
/** Weather marks (Coupler's `weather`): one in `every` pixels, a mark chosen from the colours; `glyph` is the shape drawn. */
const MARKS = {
  rain: { every: 10, colors: [LBLU, LCYN], shape: 'slash' }, thunderstorm: { every: 5, colors: [LBLU, GRY], shape: 'slash' }, snow: { every: 9, colors: [WHT, WHT, GRY], shape: 'dot' },
  blizzard: { every: 3, colors: [WHT, WHT, WHT], shape: 'dot' }, hail: { every: 10, colors: [WHT, GRY], shape: 'dot' }, sleet: { every: 8, colors: [GRY, WHT], shape: 'slash' }, dust: { every: 5, colors: [YEL, BRN, BRN], shape: 'dot' },
};
const OVERCAST = new Set(['cloudy', 'rain', 'thunderstorm', 'snow', 'blizzard', 'hail', 'sleet', 'fog', 'dust']);
/** Horizon share of the height, by terrain (Coupler's `horizon`). */
const HORIZON = { mountains: 0.6, watersurface: 0.5, seaport: 0.5, air: 1 };

/** Night: the day in its night colours (Coupler's `dim`): bright colours go to their dark self, the rest dither with black. Lit pixels stay. */
function dim(px) {
  for (let y = 0; y < px.h; y++) for (let x = 0; x < px.w; x++) {
    const i = y * px.w + x, c = px.a[i];
    if (px.f[i] & LIT || c === K) continue;
    px.a[i] = c > 15 || px.smooth ? shade256(c, 0.45) : c >= 9 ? c - 8 : BAYER[y & 3][x & 3] < 8 ? K : c;
  }
}

/**
 * Draw an outdoor picture onto cells: `{ terrain, time, weather, high, arc (thousandths), seed, columns, rows, map }` where `map` takes
 * the 16 indexes to the engine's VGA values (the caller's `V` order).
 */
export function drawOutdoors({ terrain, time, weather, high, arc, seed, columns, rows, map, depth = 16, raw = false }) {
  const W = columns, H = rows * 2, under = terrain === 'underwater', air = terrain === 'air';
  const hz = under ? 0 : Math.round(H * (HORIZON[terrain] ?? 0.45)), px = new Px(W, H, depth === 256), r = rng(`${seed}:${terrain}`), night = time === 'night';
  const s = Math.max(0.25, H / 48), later = [], kind = SKY_OF[weather];
  const R = (label) => r.fork(label);
  // sky
  if (under) { /* the sea fills the picture */ } else px.gradient(0, air ? H : hz, SKY[kind][time]);
  const sky = (fn) => { if (!under) fn(); };
  sky(() => {
    if (weather === 'cloudy' || weather === 'windy') clouds(px, R('clouds'), W, hz, s, weather === 'cloudy' ? 7 : 15, weather === 'cloudy' ? 5 : 3);
    if (kind === 'storm' || kind === 'blizzard' || kind === 'rain') clouds(px, R('storm'), W, hz, s, DGR, 5);
  });
  // land
  (LAND[terrain] ?? LAND.plains)({ px, r: R('land'), W, H, hz, s, time, night, weather, high, later, terrain });
  if (high && !['air', 'city', 'spaceport', 'underwater'].includes(terrain)) castle(px, R('castle'), W, hz, s);
  // weather that lies on the land, then the night
  if (weather === 'snow' || weather === 'blizzard') {
    const rs = R('lies'), p = weather === 'blizzard' ? 0.97 : 0.9;
    for (let y = hz; y < H; y++) for (let x = 0; x < W; x++) if (!px.is(x, y, LIT | WET) && rs.chance(p)) px.set(x, y, y > hz + (H - hz) * 0.6 && rs.chance(0.5) ? WHT : px.mix(x, y, GRY, WHT, 8 + (y - hz) * 8 / Math.max(1, H - hz)));
    // with 256 colours the snow is textured: drifts in long soft ripples, a bluish shade in the hollows, a few glints
    if (px.smooth) for (let y = hz; y < H; y++) for (let x = 0; x < W; x++) if (!px.is(x, y, LIT | WET) && px.get(x, y) !== K) {
      const drift = Math.sin(x * 0.23 + y * 0.9) + Math.sin(x * 0.07 - y * 0.4), c = px.get(x, y);
      px.set(x, y, drift < -1.1 ? between(c, LBLU, 5) : drift > 1.3 ? WHT : rs.chance(0.05) ? between(c, WHT, 8) : between(c, LBLU, 1 + rs.int(2)));
    }
  }
  if (weather === 'cold') px.speckle(R('frost'), 0, hz, W - 1, H - 1, WHT, 0.1);
  if (weather === 'drought') cracks(px, R('cracks'), W, H, hz);
  if (weather === 'heat') heat(px, W, hz, s);
  if (night && !under) dim(px);
  texture(px, r.fork('texture').int(1 << 30), hz, { land: !under });
  // lit things, drawn after the night so they stay bright: sun or moon, stars, windows, weather
  if (!under) {
    const t = clamp((arc ?? { dawn: 60, dusk: 940 }[time] ?? 500) / 1000, 0, 1);
    if (!OVERCAST.has(weather) || weather === 'cloudy') body(px, W, hz, s, t, night, weather === 'heat', high);
    if (night && !air) stars(px, R('stars'), W, hz, weather);
    if (night && high && !OVERCAST.has(weather)) aurora(px, R('aurora'), W, hz, s);
    if (night && air) stars(px, R('stars'), W, H * 0.6, weather);
  }
  for (const fn of later) fn(night);
  if (weather === 'fog' || weather === 'dust') fogBand(px, W, hz, s, weather === 'dust' ? BRN : GRY, R('fog'));
  if (weather === 'windy') windy(px, R('wind'), W, H, s);
  const mark = MARKS[weather];
  if (mark) {
    const rm = R('marks'), n = Math.max(1, Math.floor(W * rows / 2 / mark.every));
    for (let i = 0; i < n; i++) {
      const x = rm.int(W), y = rm.int(H); let c = rm.pick(mark.colors); if (night && c === WHT) c = GRY;
      px.set(x, y, c, LIT); if (mark.shape === 'slash') px.set(x + 1, y - 1, c, LIT);
    }
  }
  if (weather === 'thunderstorm') bolt(px, R('bolt'), W, hz || H);
  return raw ? px : px.toCells(map);
}

// --- the sky -----------------------------------------------------------------------------------------------------------------------
function clouds(px, r, W, hz, s, c, n) {
  const top = Math.max(1, Math.floor(hz * 0.6));
  for (let i = 0; i < n; i++) {
    const cx = r.int(W), cy = 2 + r.int(top), rx = Math.round((5 + r.int(7)) * s * 1.5), ry = Math.max(1, Math.round((1 + r.int(2)) * s));
    for (let y = cy - ry; y <= cy + ry; y++) for (let x = cx - rx; x <= cx + rx; x++) if (y < hz && ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1) px.set(x, y, c);
  }
}
/** The sun by day, the moon by night, on one arc across the sky (Coupler's `arc`, thousandths); a second small red moon in high fantasy. */
function body(px, W, hz, s, t, night, hot, high) {
  const r = Math.max(2, Math.round(4 * s)) + (hot ? 1 : 0), cx = Math.round(r + t * (W - 2 * r - 1)), cy = Math.max(r + 1, Math.round(hz - 1 - Math.sin(Math.PI * t) * (hz - r - 3)));
  px.disc(cx, cy, r, night ? WHT : YEL, LIT);
  if (high && night) px.disc(clamp(Math.round(W - cx * 0.8), 3, W - 4), Math.max(2, Math.round(cy * 0.6)), Math.max(1, Math.round(r / 2.5)), LRED, LIT);
}
function stars(px, r, W, to, weather) {
  if (OVERCAST.has(weather) && weather !== 'cloudy') return;
  for (let i = 0; i < Math.max(2, Math.floor(W * to / 40)); i++) px.set(r.int(W), r.int(Math.max(1, Math.floor(to))), r.chance(0.2) ? YEL : WHT, LIT);
}
function aurora(px, r, W, hz, s) { const ph = r.next() * 6; for (let x = 0; x < W; x++) { const y = Math.round(2 * s + 2 * Math.sin(x * 0.12 + ph) * s); for (let k = 0; k < Math.max(2, Math.round(3 * s)); k++) px.set(x, y + k, px.mix(x, y, LGRN, GRN, k * 6), LIT); } }
function fogBand(px, W, hz, s, c, r) { const reach = Math.max(2, Math.round(7 * s)); for (let y = Math.max(0, hz - reach); y < Math.min(px.h, hz + reach); y++) { const t = Math.round(16 * (1 - Math.abs(y - hz) / reach)); for (let x = 0; x < W; x++) if (px.smooth) { if (t > 0 && px.get(x, y) !== c) px.set(x, y, between(px.get(x, y), c, Math.min(15, t)), px.f[y * W + x]); } else if (BAYER[y & 3][x & 3] < t) px.set(x, y, c, px.f[y * W + x]); } }
function windy(px, r, W, H, s) { for (let i = 0; i < Math.max(2, Math.floor(W / 8)); i++) { const x = r.int(W), y = r.int(Math.max(1, H)), len = Math.round((4 + r.int(6)) * Math.max(1, s)); for (let k = 0; k < len; k++) px.set(x + k, y + (k > len / 2 ? 1 : 0), r.chance(0.7) ? WHT : GRY, LIT); } }
function heat(px, W, hz, s) { const reach = Math.max(2, Math.round(8 * s)); for (let y = Math.max(0, hz - reach); y < hz; y++) for (let x = 0; x < W; x++) { const t = Math.round(10 * (y - hz + reach) / reach); if (px.smooth) px.set(x, y, between(px.get(x, y), YEL, Math.min(15, Math.round(t * 0.7)))); else if (BAYER[y & 3][x & 3] < t) px.set(x, y, YEL); } }
function cracks(px, r, W, H, hz) {
  for (let y = hz; y < H; y++) for (let x = 0; x < W; x++) if (!px.is(x, y, WET) && (px.get(x, y) === GRN || px.get(x, y) === LGRN)) px.set(x, y, px.mix(x, y, BRN, YEL, 6));
  for (let i = 0; i < Math.max(2, Math.floor(W / 14)); i++) { let x = r.int(W), y = hz + r.int(Math.max(1, H - hz)); for (let k = 0; k < 10 + r.int(10); k++) { if (!px.is(x, y, WET)) px.set(x, y, K); x += r.int(3) - 1; y += r.chance(0.6) ? 1 : 0; } }
}
function bolt(px, r, W, to) { let x = 4 + r.int(Math.max(1, W - 8)); for (let y = 0; y < to; y++) { px.set(x, y, YEL, LIT); px.set(x + 1, y, WHT, LIT); x = clamp(x + r.int(3) - 1, 0, W - 2); } }
function castle(px, r, W, hz, s) {
  const u = Math.max(2, Math.round(2.5 * s)), x0 = Math.round((W - 8 * u) * (0.1 + 0.8 * r.next()));
  px.rect(x0, hz - 4 * u, x0 + 7 * u, hz, DGR);
  for (const [dx, hh] of [[0, 6], [3, 8], [6, 6]]) {
    px.rect(x0 + dx * u, hz - hh * u, x0 + dx * u + u + 1, hz, DGR);
    for (let k = 0; k <= u; k++) px.rect(x0 + dx * u - 1 + k, hz - hh * u - 1 - k, x0 + dx * u + u + 2 - k, hz - hh * u - 1 - k, BLU);
  }
  px.rect(x0 + 3 * u + 1, hz - 2 * u, x0 + 4 * u, hz, K);
}

// --- the land: one function per terrain, in perspective ------------------------------------------------------------------------------
const ground = (px, r, hz, stops, speck, p) => { px.gradient(hz, px.h, stops); if (speck !== undefined) px.speckle(r, 0, hz, px.w - 1, px.h - 1, speck, p); };
const tuft = (px, x, y, c) => { px.set(x, y, c); px.set(x - 1, y + 1, c); px.set(x + 1, y + 1, c); };
function tree(px, x, base, size, trunk, c1, c2) {
  const tw = Math.max(1, Math.round(size / 4)), th = Math.max(2, Math.round(size));
  px.rect(x, base - th, x + tw - 1, base, trunk);
  const cr = Math.max(2, Math.round(size * 0.6)), cy = base - th - cr * 0.6; px.disc(x + tw / 2, cy, cr, c1); for (let k = 0; k < cr; k++) px.set(x + tw / 2 + cr * 0.4 + (k % 2), cy + cr * 0.3 + k * 0.5, c2);
}
function forest(px, r, hz, W, H, s, count, canopyTop) {
  const band = ridgeLine(r.fork('line'), W, Math.max(2, Math.round(7 * s)), 0.35);
  for (let x = 0; x < W; x++) for (let y = hz - band[x]; y < hz; y++) px.set(x, y, px.mix(x, y, GRN, DGR, 4));
  const list = Array.from({ length: count }, () => ({ d: r.next(), x: r.int(W) })).sort((a, b) => a.d - b.d);
  for (const { d, x } of list) tree(px, x, hz + Math.round((H - hz) * (0.12 + 0.82 * d)), (2 + 6 * d) * s, BRN, LGRN, GRN);
  if (canopyTop) for (let x = 0; x < W; x++) { const hh = Math.round((2 + 4 * Math.abs(Math.sin(x * 0.21) + Math.sin(x * 0.07 + 1))) * s); for (let y = 0; y < hh; y++) px.set(x, y, y > hh - 2 && x % 3 === 0 ? GRN : px.mix(x, y, LGRN, GRN, 5 + y * 2)); }
}
function building(px, x, w, top, hz, c, roof, win, later) {
  px.rect(x, top, x + w - 1, hz, c);
  if (roof !== undefined) for (let k = 0; k < Math.ceil(w / 2); k++) px.rect(x + k, top - 1 - k, x + w - 1 - k, top - 1 - k, roof);
  for (let y = top + 2; y < hz - 1; y += 4) for (let xx = x + 2; xx < x + w - 2; xx += 3) { px.set(xx, y, win); px.set(xx + 1, y, win); later.push((night) => { if (night) { px.set(xx, y, YEL, LIT); px.set(xx + 1, y, YEL, LIT); } }); }
}
function town(px, r, x0, x1, hz, s, later) {
  const cols = [BRN, DGR, GRY, WHT, BRN, DGR];
  for (let x = x0; x < x1;) {
    const bw = Math.max(4, Math.round((5 + r.int(5)) * s)), top = hz - Math.max(5, Math.round((6 + r.int(14)) * s)), c = r.pick(cols);
    building(px, x, Math.min(bw, x1 - x), top, hz, c, r.chance(0.5) ? r.pick([BRN, RED, DGR]) : undefined, BLU, later);
    x += bw + (r.chance(0.35) ? 1 : 0);
  }
}
const paving = (px, hz, W, H) => { for (let y = hz; y < H; y++) for (let x = 0; x < W; x++) px.set(x, y, (x % 4 === 1 && y % 4 === 1) ? DGR : GRY); };
const road = (px, W, H, hz, c, dash) => { const cx = W / 2, hh = Math.max(1, H - hz); for (let y = hz; y < H; y++) { const half = 1 + ((y - hz) / hh) * W * 0.12; for (let x = Math.floor(cx - half); x <= cx + half; x++) px.set(x, y, c); if (dash && (y - hz) % 4 < 2) px.set(Math.round(cx), y, dash); } };

const LAND = {
  plains({ px, r, W, H, hz, s }) { ground(px, r, hz, [GRN, GRN, LGRN], LGRN, 0.07); const hills = ridgeLine(r.fork('far'), W, Math.max(1, Math.round(4 * s)), 0.12); for (let x = 0; x < W; x++) for (let y = hz - hills[x]; y < hz; y++) px.set(x, y, px.mix(x, y, GRN, DGR, 6)); road(px, W, H, hz, BRN, null); for (let i = 0; i < W / 6; i++) tuft(px, r.int(W), hz + 2 + r.int(Math.max(1, H - hz - 3)), LGRN); },
  woods({ px, r, W, H, hz, s }) { ground(px, r, hz, [GRN, GRN, LGRN], LGRN, 0.1); forest(px, r, hz, W, H, s, Math.max(4, Math.round(W / 9)), false); },
  jungle({ px, r, W, H, hz, s }) { ground(px, r, hz, [GRN, GRN, LGRN], LGRN, 0.12); forest(px, r, hz, W, H, s, Math.max(6, Math.round(W / 6)), true); },
  hills({ px, r, W, H, hz, s }) { ground(px, r, hz, [GRN, GRN, LGRN], LGRN, 0.05); for (const [c, f, a] of [[DGR, 0.07, 9], [GRN, 0.1, 7], [LGRN, 0.13, 5]]) { const line = ridgeLine(r.fork(`h${a}`), W, Math.max(2, Math.round(a * s)), f); for (let x = 0; x < W; x++) for (let y = hz - line[x]; y < hz + 3; y++) px.set(x, y, px.mix(x, y, c, GRN, 3)); } },
  rocky({ px, r, W, H, hz, s }) { ground(px, r, hz, [DGR, DGR, GRY], GRY, 0.08); const line = ridgeLine(r.fork('crag'), W, Math.max(3, Math.round(12 * s)), 0.2); for (let x = 0; x < W; x++) for (let y = hz - line[x]; y < hz; y++) px.set(x, y, y > hz - line[x] + 1 ? DGR : GRY); for (let i = 0; i < Math.max(3, W / 12); i++) { const x = r.int(W), y = hz + 3 + r.int(Math.max(1, H - hz - 4)), rr = Math.max(2, Math.round((1 + 3 * (y - hz) / Math.max(1, H - hz)) * s * 1.5)); px.disc(x, y, rr, DGR); px.disc(x - 1, y - 1, Math.max(1, rr - 1), GRY); px.set(x - 1, y - rr + 1, WHT); } },
  mountains({ px, r, W, H, hz, s }) {
    px.gradient(hz, H, [DGR, BRN, BRN, BRN, DGR]);
    // layered ranges, far to near: each paler and bluer the farther it is (haze), a lit face and a shade face to every peak, gullies down the
    // lit face, snow on the two far ranges. With 256 colours the haze, the faces and the rock are real in-between tones, and the rock has grain.
    const tone = (x, y, a, b, t) => px.smooth ? between(a, b, t) : px.mix(x, y, a, b, t);
    const ranges = [{ share: 0.8, lit: [GRY, LBLU, 0], shade: [DGR, GRY, 0], snow: true }, { share: 0.92, lit: [BRN, DGR, 0], shade: [DGR, K, 0], snow: true }, { share: 0.55, lit: [DGR, GRN, 0], shade: [K, DGR, 0], snow: false }];
    ranges.forEach(({ share, lit, shade, snow }, k) => {
      const rr = r.fork(`range${k}`), n = Math.max(3, Math.round(W / 20)) + k, lift = k === 2 ? 0 : Math.round((2 - k) * s), peaks = [];
      for (let i = 0; i < n; i++) peaks.push({ x: Math.round((W / n) * (i + 0.5) + (rr.next() - 0.5) * 12 * s), h: Math.round((hz - 2) * share * (0.5 + 0.5 * rr.next())), slope: 0.7 + rr.next() * 0.6, gully: 3 + rr.int(3) });
      for (let x = 0; x < W; x++) {
        let best = null, top = hz;
        for (const p of peaks) { const t = hz - lift - Math.round(p.h - Math.abs(x - p.x) / p.slope); if (t < top) { top = t; best = p; } }
        if (!best) continue;
        const right = x > best.x, depth = hz - top;
        for (let y = top; y < hz; y++) {
          const down = y - top, haze = Math.round((down / Math.max(1, depth)) * (k === 0 ? 7 : k === 1 ? 3 : 0));
          let c;
          if (snow && down < best.h * 0.28 - ((x * 7 + y * 3) % 3)) c = right ? GRY : WHT;
          else {
            const [a, b, t] = right ? shade : lit, gully = !right && (Math.abs(x - best.x) + down * 0.6) % best.gully < 0.7 && down > 2 && down < depth;
            c = gully ? shade[0] : tone(x, y, a, b, t);
            if (px.smooth) { c = haze ? between(c, LBLU, haze) : c; if (rr.chance(0.12)) c = between(c, right ? K : WHT, 2); }
          }
          px.set(x, y, c);
        }
      }
    });
    for (let i = 0; i < W / 5; i++) { const x = r.int(W), y = hz + 3 + r.int(Math.max(1, H - hz - 6)); px.set(x, y - 1, GRY); px.set(x, y, GRY); }
  },
  desert({ px, r, W, H, hz, s }) {
    px.gradient(hz, H, [YEL, YEL, YEL]);
    for (let k = 0; k < Math.max(2, Math.round(H / 16)); k++) { let y = hz + 1 + Math.round(((k + 0.5) / Math.max(2, H / 16)) * (H - hz - 2)); for (let x = 0; x < W;) { const len = 4 + r.int(Math.round(20 * s) + 1); px.rect(x, y, x + len, y + Math.max(2, Math.round(s * 3)) - 1, BRN); x += len; y = clamp(y + r.int(3) - 1, hz + 1, H - 2); } }
    for (let i = 0; i < Math.max(1, W / 90); i++) { const x = r.int(W), y = hz + 3 + r.int(Math.max(1, H - hz - 4)), t = Math.max(2, Math.round(5 * s)); px.rect(x, y - t, x, y, GRN); px.rect(x - 2, y - t + 2, x - 2, y - 2, GRN); px.rect(x - 2, y - 2, x, y - 2, GRN); }
  },
  swamp({ px, r, W, H, hz, s }) {
    ground(px, r, hz, [GRN, GRN, GRN, BRN], undefined);
    const mud = Math.max(2, Math.round((H - hz) * 0.28));
    for (let y = hz; y < hz + mud; y++) for (let x = 0; x < W; x++) px.set(x, y, y < hz + 2 ? px.mix(x, y, GRY, DGR, 8) : DGR);
    for (let i = 0; i < Math.max(3, W / 14); i++) { const y = hz + mud + 2 + r.int(Math.max(1, H - hz - mud - 4)), x = r.int(W), len = Math.round((6 + r.int(14)) * s * 1.5); px.rect(x, y, x + len, y, CYN, WET); px.rect(x, y + 1, x + len, y + 1, BLU, WET); }
    for (let i = 0; i < W / 2.5; i++) { const x = r.int(W), y = hz + 2 + r.int(Math.max(1, H - hz - 3)); px.set(x, y, BRN); if (r.chance(0.5)) px.set(x, y - 1, BRN); }
    // the sky is not left a flat band: a far line of dead trees and a mist lying on the water, thickest at the horizon
    const far = ridgeLine(r.fork('far'), W, Math.max(2, Math.round(4 * s)), 0.5);
    for (let x = 0; x < W; x++) for (let y = hz - far[x]; y < hz; y++) px.set(x, y, px.smooth ? between(px.get(x, y), DGR, 9) : px.mix(x, y, GRY, DGR, 8));
    const mist = Math.max(3, Math.round(6 * s)), rm = r.fork('mist');
    for (let y = hz - mist; y < hz + 1; y++) for (let x = 0; x < W; x++) {
      const t = Math.round(((y - (hz - mist)) / mist) * 12) + (px.smooth ? rm.int(3) : 0), c = px.get(x, y);
      px.set(x, y, px.smooth ? between(c, GRY, Math.min(14, t)) : px.mix(x, y, c, GRY, t));
    }
    const tx = Math.round(W * 0.55); px.rect(tx, hz - Math.round(9 * s), tx, hz + 1, DGR); px.rect(tx - 2, hz - Math.round(6 * s), tx + 1, hz - Math.round(6 * s), DGR);
  },
  city({ px, r, W, H, hz, s, later }) { paving(px, hz, W, H); town(px, r.fork('town'), 0, W, hz, s, later); road(px, W, H, hz + 1, DGR, WHT); },
  spaceport({ px, r, W, H, hz, s, later }) {
    px.gradient(hz, H, [DGR, DGR, GRY]);
    for (let k = -8; k <= 8; k++) for (let y = hz; y < H; y++) px.set(W / 2 + (k * W / 16) * (0.8 + 0.2 * (y - hz) / Math.max(1, H - hz)), y, y > hz + (H - hz) / 3 ? WHT : GRY);
    const tx = Math.round(W * 0.1); px.rect(tx, hz - Math.round(14 * s), tx + 2, hz, DGR); px.rect(tx - 1, hz - Math.round(16 * s), tx + 3, hz - Math.round(14 * s), GRY); later.push(() => px.set(tx + 1, hz - Math.round(17 * s), LRED, LIT));
    const sx = Math.round(W * 0.45), sy = hz + Math.round((H - hz) * 0.55), L = Math.round(22 * s); for (let k = 0; k < L; k++) { px.set(sx + k, sy, WHT); px.set(sx + k, sy - 1, WHT); } px.set(sx + L * 0.7, sy - 1, LCYN); px.set(sx + L, sy, WHT);
    for (let i = 0; i < 6; i++) later.push(() => px.set(Math.round(W * (0.05 + i * 0.18)), hz + 1, YEL, LIT));
  },
  watersurface({ px, r, W, H, hz, s, later }) { sea(px, r, 0, W, hz, H, s); },
  seaport({ px, r, W, H, hz, s, later }) {
    const q = Math.floor(W / 2); paving(px, hz, q, H); for (let y = hz; y < H; y++) for (let x = 0; x < q; x++) px.set(x, y, px.get(x, y)); town(px, r.fork('town'), 0, q, hz, s, later);
    sea(px, r, q, W, hz, H, s); const sx = Math.round(q + (W - q) * 0.6), sy = hz + Math.round((H - hz) * 0.45), m = Math.max(3, Math.round(8 * s));
    px.rect(sx - m, sy, sx + m, sy, BRN); px.rect(sx - m + 1, sy + 1, sx + m - 1, sy + 1, BRN); px.rect(sx, sy - m, sx, sy - 1, DGR); for (let k = 0; k < m - 1; k++) px.rect(sx + 1, sy - m + 1 + k, sx + 1 + Math.round(k * 0.6), sy - m + 1 + k, WHT);
    px.rect(q - Math.round(2 * s), hz + Math.round((H - hz) * 0.35), q + Math.round(10 * s), hz + Math.round((H - hz) * 0.35) + 1, BRN, 0);
  },
  air({ px, r, W, H, s }) {
    const base = Math.round(H * 0.8), line = ridgeLine(r.fork('cloud'), W, Math.max(2, Math.round(7 * s)), 0.15);
    for (let x = 0; x < W; x++) for (let y = base - line[x]; y < H; y++) px.set(x, y, y >= H - 2 ? GRY : WHT);
    for (let i = 0; i < Math.max(2, Math.round(W / 40)); i++) { const x = r.int(W), y = r.int(Math.round(H * 0.7)); px.set(x, y, DGR); px.set(x + 2, y, DGR); px.set(x + 1, y + 1, DGR); }
  },
  underwater({ px, r, W, H, s }) {
    const floor = Math.round(H * 0.8); px.gradient(0, floor, [CYN, BLU, BLU, BLU]);
    for (let y = floor; y < H; y++) for (let x = 0; x < W; x++) px.set(x, y, y < floor + 3 ? px.mix(x, y, K, BRN, (y - floor) * 6) : (y === H - 1 && x % 3 === 0 ? YEL : BRN));
    for (let i = 0; i < W / 4; i++) { const x = r.int(W), h = Math.round((3 + r.int(12)) * s * 1.3); for (let k = 0; k < h; k++) px.set(x + (k % 4 < 2 ? 0 : 1), floor - k, k % 3 === 2 ? LGRN : GRN); }
    const bubbles = Math.round(W * H / 45), rb = r.fork('bubbles');
    for (let i = 0; i < bubbles; i++) px.glyph(rb.int(W), rb.int(Math.max(1, (floor >> 1) - 1)), rb.pick(['o', 'o', '°', '·']), LCYN);
  },
};
function sea(px, r, x0, x1, hz, H, s) {
  for (let y = hz; y < H; y++) for (let x = x0; x < x1; x++) px.set(x, y, px.mix(x, y, LBLU, BLU, clamp(4 + Math.round((y - hz) * 10 / Math.max(1, H - hz)), 0, 16)), WET);
  for (let i = 0; i < (x1 - x0) * (H - hz) / 55; i++) { const x = x0 + r.int(x1 - x0), y = hz + 1 + r.int(Math.max(1, H - hz - 1)); px.set(x, y, r.chance(0.5) ? WHT : LCYN, WET); px.set(x + 1, y, LCYN, WET); }
}

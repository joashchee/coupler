#!/usr/bin/env node
// Checks Coupler's copy of the Painter engine (src/vendor/art-sprites) the
// way lib/sprites.ts uses it (CLAUDE.md rule 12: a room picture never breaks):
// every terrain, time and weather draws at both color depths and a few
// sizes, the same scene draws the same twice, the colors are indexes the
// page can show, a picture has its words, and nothing that came from
// the player or the room's name reaches the seed.
//
//   node scripts/sprites-check.mjs
import assert from 'node:assert/strict';
import { drawVista, roomSeed, toSpans, TERRAINS, TIMES, WEATHER_KINDS } from '../src/vendor/art-sprites/engine.js';

const draw = (p) => {
  const made = drawVista(p);
  return { says: made.says, spans: toSpans(made.frames[0]) };
};
let drawn = 0;
for (const terrain of TERRAINS)
  for (const time of TIMES)
    for (const weather of WEATHER_KINDS)
      for (const style of ['fantasy', 'high'])
        for (const [colors, columns, rows] of [[16, 48, 12], [256, 80, 24]]) {
          const p = { terrain, time, weather, style, colors, columns, rows, seed: roomSeed('standard', 'Midgaard#3001', 0) };
          const a = draw(p);
          assert.deepEqual(a, draw(p), `${terrain} ${time} ${weather} draws the same twice`);
          assert.equal(a.spans.columns, columns);
          assert.equal(a.spans.rows, rows);
          assert.ok(a.says.length > 0, `${terrain} has its words`);
          for (const line of a.spans.lines)
            for (const r of line) {
              for (const c of [r.fg, r.bg]) assert.ok(c === null || (Number.isInteger(c) && c >= 0 && c < colors), `${terrain}: color ${c} is in the ${colors}`);
              assert.ok(!r.blink, `${terrain} never blinks`);
            }
          drawn++;
        }
// The size is clamped, and the picture says so by its own columns and rows.
assert.deepEqual([draw({ columns: 400, rows: 99 }).spans.columns, draw({ columns: 400, rows: 99 }).spans.rows], [160, 45]);
assert.deepEqual([draw({ columns: 2, rows: 1 }).spans.columns, draw({ columns: 2, rows: 1 }).spans.rows], [8, 4]);
// The seed is Coupler's (src-tauri/src/paint.rs `seed`): another room or another look is another seed.
assert.notEqual(roomSeed('standard', 'Midgaard#3001', 0), roomSeed('standard', 'Midgaard#3002', 0));
assert.notEqual(roomSeed('standard', 'Midgaard#3001', 0), roomSeed('standard', 'Midgaard#3001', 1));
// What the engine is not given is not a param it knows: a name or a zone changes nothing.
const base = { terrain: 'city', time: 'dusk', weather: 'rain', seed: '1' };
assert.deepEqual(draw(base), draw({ ...base, name: 'Alice the Bold', zone: 'Secret Place', art: 'ans/x' }));
console.log(`${drawn} pictures drawn; engine copy ok`);

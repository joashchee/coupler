#!/usr/bin/env node
// The reference pictures drawn by the Painter engine (`art-sprites`, the
// `painter` repo beside this one): the handoff's B1 matrix through the
// `vista` recipe, as .ans (UTF-8 and CP437 for 16 colors, UTF-8 for 256
// and true) with a PNG render of each, a manifest (params, seed, the
// engine's words, its ANSI lint) and the kits report. Nothing here reads
// what Coupler's Rust painter drew (`painter-data/reference/`).
//
//   node scripts/painter-engine-reference.mjs [--painter ../painter] [--to painter-data/engine]
import { mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: { painter: { type: 'string', default: '../painter' }, to: { type: 'string', default: 'painter-data/engine' } } });
const home = resolve(values.painter);
const at = (f) => import(pathToFileURL(join(home, f)).href);
const { draw, roomSeed, writeAnsi, lintAnsi, toSpans } = await at('index.js');
const { renderCells } = await at('ansi-render.js');
const { loadAtlas } = await at('atlas.js');
const { rgbaSheet, } = await at('sheet.js');
const { TERRAINS } = await at('recipes/vista.js');
const version = JSON.parse((await import('node:fs')).readFileSync(join(home, 'package.json'), 'utf8')).version;
const commit = execFileSync('git', ['-C', home, 'rev-parse', '--short', 'HEAD']).toString().trim();

const out = resolve(values.to);
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

const WORLD = 'standard', ROOM = 'Midgaard#3001', OTHER = 'Midgaard#3002';
const SCENES = [['day', 'clear'], ['night', 'thunderstorm'], ['dusk', 'snow']];
const job = (terrain, [time, weather], style = 'fantasy', more = {}) => ({ terrain, time, weather, style, columns: 80, rows: 24, colors: 16, room: ROOM, ...more });
const jobs = [];
for (const t of TERRAINS) for (const s of SCENES) for (const style of ['fantasy', 'high']) jobs.push(job(t, s, style));
for (const t of ['plains', 'woods', 'city', 'mountains', 'watersurface', 'underwater', 'stone', 'cave'])
  for (const [columns, rows] of [[25, 12], [8, 4], [160, 45]]) for (const s of [SCENES[0], SCENES[1]]) jobs.push(job(t, s, 'fantasy', { columns, rows }));
for (const arc of [0, 500, 1000]) for (const s of [['day', 'clear'], ['night', 'clear']]) for (const style of ['fantasy', 'high']) jobs.push(job('plains', s, style, { arc }));
for (const t of ['plains', 'woods', 'mountains', 'city', 'watersurface', 'stone', 'cave', 'magic']) for (const s of SCENES) for (const colors of [256, 'true']) jobs.push(job(t, s, 'fantasy', { colors }));
for (const t of ['plains', 'woods', 'hills', 'mountains', 'city', 'stone', 'wooden', 'cave']) jobs.push(job(t, SCENES[0], 'fantasy', { room: OTHER }));

const atlas = loadAtlas();
const files = [];
for (const j of jobs) {
  const seed = roomSeed(WORLD, j.room, 0);
  // 256 draws with the engine's 256 colors; true colour draws as 256 and is read with `tones` (writeAnsi colors: 'true').
  const art = draw('vista', { terrain: j.terrain, time: j.time, weather: j.weather, style: j.style, arc: j.arc, colors: j.colors === 'true' ? 256 : j.colors, seed, columns: j.columns, rows: j.rows }, { style: 'ansi' });
  const cells = art.frames[0];
  const name = `${j.terrain}_${j.time}-${j.weather}_${j.style}${j.arc === undefined ? '' : `_arc${j.arc}`}_${cells.w}x${cells.h}_c${j.colors}_${j.room === ROOM ? 'room1' : 'room2'}`;
  const found = lintAnsi(cells, { name, depth: j.colors === 'true' ? 'true' : j.colors, skip: ['outline', 'contrast', 'ramp'] });
  const written = [];
  const utf8 = writeAnsi(cells, { target: 'modern', charset: 'utf8', colors: j.colors, sauce: art.sauce ?? true });
  writeFileSync(join(out, `${name}.utf8.ans`), utf8.bytes); written.push(`${name}.utf8.ans`);
  if (j.colors === 16) {
    const classic = writeAnsi(cells, { target: 'classic', charset: 'cp437', colors: 16, sauce: art.sauce ?? true });
    writeFileSync(join(out, `${name}.cp437.ans`), classic.bytes); written.push(`${name}.cp437.ans`);
  }
  writeFileSync(join(out, `${name}.png`), rgbaSheet([renderCells(cells, { atlas, letterSpacing: 8 })], { scale: 1, cols: 1, gap: 0 }));
  files.push({ file: name, written: [...written, `${name}.png`], ...j, columns: cells.w, rows: cells.h, world: WORLD, look: 0, seed, says: art.says, warnings: utf8.warnings, lint: found.map((f) => `${f.level} [${f.rule}] ${f.message}`) });
}
writeFileSync(join(out, 'manifest.json'), JSON.stringify({ source: `art-sprites ${version} (painter ${commit}), recipe vista, style ansi`, made: '2026-10-29', note: 'Drawn by the Painter engine, never from the Rust painter\'s output. 16 colors are written as UTF-8 (modern target) and CP437 (classic target); 256 and true are UTF-8. true colour is the 256 drawing written with colors "true".', files }, null, 2) + '\n');

// The kits report and the contact sheet (every variant of vista), as the handoff's parity step asks.
writeFileSync(join(out, 'kits.txt'), execFileSync('node', [join(home, 'cli.js'), 'kits'], { cwd: home }).toString());
execFileSync('node', [join(home, 'cli.js'), 'sheet', 'vista', '--style', 'ansi', '--out', join(out, 'vista-sheet.png')], { cwd: home });
console.log(`${jobs.length} pictures from art-sprites ${version} (${commit}) in ${out}`);

/**
 * The web build's stand-in for `@tauri-apps/api/core` (vite.config.ts
 * points the import here in `--mode web`): the desktop's commands, by the
 * same names and arguments, answered in the page. The game logic is the
 * desktop's own Rust, compiled to WebAssembly (`src-web/`) and loaded
 * here with no generated glue (its interface is `src-web/src/ffi.rs`).
 *
 * The only network this page uses: the game's WebSocket, whose address
 * comes from the Rust (`ports.rs`, `web_socket_url`), for playing and for
 * the greeting's MSSP probe, and its own files (the module, the mirror).
 * The page's Content-Security-Policy allows nothing else (`web/_headers`).
 *
 * What the desktop keeps in app data, the browser keeps for this site in
 * IndexedDB (`coupler` database, store `kept`), each under the path the
 * desktop gives its file (`maps/<world>.json`, `cast/`, `pictures/`, `fights/`,
 * `played.json`), but the journal and the Bestiary, the web's own JSON
 * (`web/journal.json`, `web/bestiary.json`); settings in localStorage as
 * on the desktop. A Coupler Backup is those files and the settings, in
 * the desktop's format (`backup_format.rs`). The hooks and assets are the
 * mirror's (`mirror.json` and `mirror/`, made by
 * `src-tauri/examples/web_mirror.rs`), read-only.
 */
import wasmUrl from "./coupler.wasm?url";
import { chosen } from "./dialog";
import { emit } from "./event";

interface Exports {
  memory: WebAssembly.Memory;
  coupler_alloc(len: number): number;
  coupler_free(ptr: number, len: number): void;
  coupler_call(ptr: number, len: number): number;
  coupler_feed(ptr: number, len: number): number;
  coupler_take_outgoing(): void;
  coupler_backup_write(ptr: number, len: number): number;
  coupler_backup_read(ptr: number, len: number): number;
  coupler_out_ptr(): number;
  coupler_out_len(): number;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** The mirror's asset files, by the desktop's asset path. */
let mirrorFiles: Record<string, string> = {};
let core: Promise<Exports> | null = null;
let socket: WebSocket | null = null;
/** WHO's clock (src-tauri/src/who.rs): each second while connected, the module sends WHO when it's due. */
let whoClock: number | null = null;

function start(): Promise<Exports> {
  core ??= (async () => {
    const imports = { coupler: { coupler_now_ms: () => performance.now(), coupler_unix_ms: () => Date.now() } };
    const { instance } = await WebAssembly.instantiateStreaming(fetch(wasmUrl), imports);
    const x = instance.exports as unknown as Exports;
    const [mirror, journal, bestiary, played] = await Promise.all([readMirror(), load("web/journal.json"), load("web/bestiary.json"), load("played.json")]);
    run(x, "call", encoder.encode(JSON.stringify({ cmd: "init", version: __APP_VERSION__, mirror, journal, bestiary, played: timesPlayed(played) })));
    // What's waiting to be kept is kept as the page is hidden or closed.
    const keepNow = () => {
      call(x, "save_now");
      flush(x);
    };
    document.addEventListener("visibilitychange", () => document.visibilityState === "hidden" && keepNow());
    window.addEventListener("pagehide", keepNow);
    return x;
  })();
  // A start that failed is tried again by the next command, not kept.
  core.catch(() => {
    core = null;
  });
  return core;
}

/**
 * The mirror's `hooks.json`, or "" when there's none. Cloudflare Pages
 * answers a file it doesn't have with the page itself (200, text/html),
 * so only JSON that parses counts: anything else means no hooks, never a
 * Coupler that can't start.
 */
async function readMirror(): Promise<string> {
  try {
    const r = await fetch("mirror.json", { cache: "no-cache" });
    if (!r.ok || !(r.headers.get("content-type") ?? "").includes("json")) return "";
    const text = await r.text();
    const parsed = JSON.parse(text) as { files?: Record<string, string>; triggers?: unknown };
    if (!Array.isArray(parsed.triggers)) return "";
    mirrorFiles = parsed.files ?? {};
    return text;
  } catch {
    mirrorFiles = {};
    return "";
  }
}

/** The times played, from the desktop's `played.json`. */
function timesPlayed(json: string): number {
  try {
    return Number((JSON.parse(json) as { played?: number }).played) || 0;
  } catch {
    return 0;
  }
}

/** Writes `input` into the module, runs `what`, and reads the output buffer. */
function run(x: Exports, what: "call" | "feed" | "backup_write" | "backup_read", input: Uint8Array): Uint8Array {
  const ptr = x.coupler_alloc(input.length);
  new Uint8Array(x.memory.buffer, ptr, input.length).set(input);
  const failed = {
    call: x.coupler_call,
    feed: x.coupler_feed,
    backup_write: x.coupler_backup_write,
    backup_read: x.coupler_backup_read,
  }[what](ptr, input.length);
  x.coupler_free(ptr, input.length);
  const out = new Uint8Array(x.memory.buffer, x.coupler_out_ptr(), x.coupler_out_len()).slice();
  if (failed) throw decoder.decode(out);
  return out;
}

function call<T>(x: Exports, cmd: string, args: Record<string, unknown> = {}): T {
  const out = run(x, "call", encoder.encode(JSON.stringify({ ...args, cmd })));
  return JSON.parse(decoder.decode(out)) as T;
}

/** Sends what the game should get, and hands the events to the listeners. */
function flush(x: Exports) {
  x.coupler_take_outgoing();
  const bytes = new Uint8Array(x.memory.buffer, x.coupler_out_ptr(), x.coupler_out_len()).slice();
  if (bytes.length > 0 && socket?.readyState === WebSocket.OPEN) socket.send(bytes);
  for (const [name, payload] of call<[string, unknown][]>(x, "take_events")) {
    if (name === "coupler-save") void save(payload as { key: string; json: string });
    else emit(name, payload);
  }
}

// ---- The browser's storage for this site: IndexedDB, one store ----

let db: Promise<IDBDatabase> | null = null;
function store(): Promise<IDBDatabase> {
  db ??= new Promise((resolve, reject) => {
    const open = indexedDB.open("coupler", 1);
    open.onupgradeneeded = () => open.result.createObjectStore("kept");
    open.onsuccess = () => resolve(open.result);
    open.onerror = () => reject(open.error);
  });
  return db;
}

async function load(key: string): Promise<string> {
  try {
    const d = await store();
    return await new Promise((resolve) => {
      const get = d.transaction("kept").objectStore("kept").get(key);
      get.onsuccess = () => resolve(typeof get.result === "string" ? get.result : "");
      get.onerror = () => resolve("");
    });
  } catch {
    return "";
  }
}

/** Saves under way, so a backup waits for them. */
const saving = new Set<Promise<void>>();

function save({ key, json }: { key: string; json: string }): Promise<void> {
  const done = (async () => {
    try {
      const d = await store();
      await new Promise<void>((resolve, reject) => {
        const t = d.transaction("kept", "readwrite");
        t.objectStore("kept").put(json, key);
        t.oncomplete = () => resolve();
        t.onerror = () => reject(t.error);
      });
    } catch (e) {
      console.warn(`couldn't keep ${key}: ${e}`);
    }
  })();
  saving.add(done);
  void done.finally(() => saving.delete(done));
  return done;
}

/** Everything kept, by path. */
async function everything(): Promise<[string, string][]> {
  const d = await store();
  return new Promise((resolve, reject) => {
    const kept = d.transaction("kept").objectStore("kept");
    const keys = kept.getAllKeys();
    const values = kept.getAll();
    values.onsuccess = () => {
      const all = keys.result.map((k, i): [string, unknown] => [String(k), values.result[i]]);
      resolve(all.filter((e): e is [string, string] => typeof e[1] === "string"));
    };
    values.onerror = () => reject(values.error);
  });
}

/** Everything kept replaced by `files`. */
async function replaceAll(files: [string, string][]): Promise<void> {
  const d = await store();
  await new Promise<void>((resolve, reject) => {
    const t = d.transaction("kept", "readwrite");
    const kept = t.objectStore("kept");
    kept.clear();
    for (const [key, json] of files) kept.put(json, key);
    t.oncomplete = () => resolve();
    t.onerror = () => reject(t.error);
  });
}

// ---- The connection ----

async function connect(x: Exports, portId: string): Promise<void> {
  if (socket) return;
  const world = call<string | null>(x, "world_of", { portId });
  if (!world) throw "Coupler doesn't know that way to play.";
  const [map, cast, looks, fights] = await Promise.all([load(`maps/${world}.json`), load(`cast/${world}.json`), load(`pictures/${world}.json`), load(`fights/${world}.json`)]);
  const url = call<string>(x, "mud_connect", { portId, map, cast, looks, fights });
  const ws = new WebSocket(url);
  ws.binaryType = "arraybuffer";
  socket = ws;
  await new Promise<void>((resolve, reject) => {
    ws.onopen = () => {
      flush(x);
      whoClock ??= window.setInterval(() => {
        if (socket !== ws) return;
        call(x, "who_tick");
        flush(x);
      }, 1000);
      resolve();
    };
    ws.onerror = () => reject("Couldn't reach CoffeeMUD. Check the internet connection.");
    ws.onclose = (e) => {
      if (socket !== ws) return;
      socket = null;
      stopWhoClock();
      reject("Couldn't reach CoffeeMUD.");
      call(x, "mud_closed", { reason: e.wasClean ? "CoffeeMUD closed the connection." : "The connection to CoffeeMUD dropped." });
      flush(x);
    };
  });
  ws.onmessage = (e) => {
    if (socket !== ws) return;
    try {
      run(x, "feed", new Uint8Array(e.data as ArrayBuffer));
      flush(x);
    } catch (why) {
      socket = null;
      stopWhoClock();
      ws.close();
      flush(x);
      call(x, "mud_closed", { reason: String(why) });
      flush(x);
    }
  };
}

function stopWhoClock() {
  if (whoClock !== null) window.clearInterval(whoClock);
  whoClock = null;
}

function disconnect(x: Exports) {
  const ws = socket;
  socket = null;
  stopWhoClock();
  ws?.close();
  call(x, "mud_closed", { reason: null });
  flush(x);
}

// ---- The greeting at launch (src-tauri/src/mssp.rs) ----

/** As the desktop's `PROBE_TIMEOUT`: the most the greeting waits for the game's figures. */
const PROBE_TIMEOUT = 6000;

/**
 * How many are online and the game's CoffeeMUD version, by MSSP, as the
 * desktop asks: a socket of its own to the game's WebSocket, which never
 * logs in and closes once the table comes (or in `PROBE_TIMEOUT`).
 */
async function launchCounts(x: Exports, portId: string): Promise<unknown> {
  const url = call<string>(x, "probe_start", { portId });
  await new Promise<void>((resolve) => {
    let ws: WebSocket;
    try {
      ws = new WebSocket(url);
    } catch {
      resolve();
      return;
    }
    ws.binaryType = "arraybuffer";
    const end = () => {
      window.clearTimeout(timer);
      ws.close();
      resolve();
    };
    const timer = window.setTimeout(end, PROBE_TIMEOUT);
    ws.onmessage = (e) => {
      const fed = call<{ reply: number[]; done: boolean }>(x, "probe_feed", { bytes: Array.from(new Uint8Array(e.data as ArrayBuffer)) });
      if (fed.reply.length > 0 && ws.readyState === WebSocket.OPEN) ws.send(Uint8Array.from(fed.reply));
      if (fed.done) end();
    };
    ws.onerror = end;
    ws.onclose = end;
  });
  return call(x, "probe_result");
}

// ---- Coupler Backup (src-tauri/src/backup_format.rs) ----

interface Manifest {
  format: number;
  app: string;
  made: number;
  files: number;
  bytes: number;
  settings: number;
}

/** A backup the player chose, read, by its path, waiting for a yes. */
const backups = new Map<string, { manifest: Manifest; settings: string; files: [string, string][] }>();

/** Kept across a restore: the dev-only App Testing results and the in-development warning's answer. */
const KEPT_ACROSS = new Set(["coupler.appTestingResults", "coupler.web.accepted"]);

/** Everything kept in this browser and the settings, as a backup downloaded under `name`. */
async function exportBackup(x: Exports, name: string, settings: string, progress?: { onmessage: (p: unknown) => void }) {
  call(x, "save_now");
  flush(x);
  await Promise.all([...saving]);
  const files = await everything();
  const made = Math.floor(Date.now() / 1000);
  const bytes = run(x, "backup_write", encoder.encode(JSON.stringify({ app: __APP_VERSION__, made, settings, files })));
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: "application/octet-stream" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name.split(/[\/]/).pop() ?? name;
  a.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  const size = files.reduce((n, [, json]) => n + encoder.encode(json).length, 0);
  progress?.onmessage({ done: size, total: size });
  const manifest: Manifest = { format: 1, app: __APP_VERSION__, made, files: files.length, bytes: size, settings: Object.keys(JSON.parse(settings) as object).length };
  return { manifest, size: bytes.length };
}

/** A backup chosen or dropped, read and checked whole. */
async function readBackup(x: Exports, path: string) {
  const bytes = new Uint8Array(await chosen(path).arrayBuffer());
  const read = JSON.parse(decoder.decode(run(x, "backup_read", bytes))) as { manifest: Manifest; settings: string; files: [string, string][] };
  backups.set(path, read);
  return read;
}

/** Puts a backup in place of everything kept here; the page then starts again (`app_restart`). */
async function restoreBackup(x: Exports, path: string, progress?: { onmessage: (p: unknown) => void }) {
  if (socket) throw "Disconnect from CoffeeMUD first, then restore the backup.";
  const read = backups.get(path) ?? (await readBackup(x, path));
  await replaceAll(read.files);
  const settings = JSON.parse(read.settings) as Record<string, string>;
  try {
    for (const key of Object.keys(localStorage)) if (key.startsWith("coupler.") && !KEPT_ACROSS.has(key)) localStorage.removeItem(key);
    for (const [key, value] of Object.entries(settings)) if (key.startsWith("coupler.") && !KEPT_ACROSS.has(key)) localStorage.setItem(key, value);
  } catch {
    // Storage unavailable: the files alone.
  }
  progress?.onmessage({ done: read.manifest.bytes, total: read.manifest.bytes });
  return read.manifest;
}

// ---- The mirror's assets ----

async function mirrored(path: string, looping = false): Promise<Response> {
  const file = (looping && mirrorFiles[`${path}#loop`]) || mirrorFiles[path];
  if (!file) throw `${path} isn't on the web.`;
  // Pages answers a missing file with the page itself. A failure is
  // tried once more past the browser's cache, which Safari on the iPhone
  // has been seen to answer from badly (2026-10-25).
  let why = "";
  for (const cache of ["default", "reload"] as const) {
    try {
      const r = await fetch(`mirror/${file}`, { cache });
      const type = r.headers.get("content-type") ?? "";
      if (r.ok && !type.includes("text/html")) return r;
      why = r.ok ? "the site sent a page instead" : `the site answered ${r.status}`;
    } catch (e) {
      why = `${e instanceof Error ? e.message : String(e)}`;
    }
  }
  throw `${path} couldn't be fetched (${why}).`;
}

// ---- The commands ----

type Args = Record<string, unknown>;

/** Coupler's community pages, as src-tauri/src/update.rs's COMMUNITY. */
const COMMUNITY: Record<string, string> = { discord: "https://discord.gg/ZH8ZhXChY3", reddit: "https://www.reddit.com/r/coupler_app/s/wnwhNjQV9d" };

export async function invoke<T>(cmd: string, args: Args = {}): Promise<T> {
  const x = await start();
  const result = await answer(x, cmd, args);
  // A command may have something to send or say (a walk's first step).
  flush(x);
  return result as T;
}

async function answer(x: Exports, cmd: string, args: Args): Promise<unknown> {
  switch (cmd) {
    case "mud_connect":
      return connect(x, String(args.portId));
    case "mud_disconnect":
      return disconnect(x);
    case "launch_counts":
      return launchCounts(x, String(args.portId));
    case "backup_export":
      return exportBackup(x, String(args.path), String(args.settings), args.progress as { onmessage: (p: unknown) => void } | undefined);
    case "backup_inspect":
      return (await readBackup(x, String(args.path))).manifest;
    case "backup_restore":
      return restoreBackup(x, String(args.path), args.progress as { onmessage: (p: unknown) => void } | undefined);
    case "app_restart":
      window.location.reload();
      return null;
    case "community_open": {
      // The desktop's update::COMMUNITY: a fixed page, in a new tab.
      const page = COMMUNITY[String(args.id)];
      if (!page) throw "Coupler has no such page.";
      window.open(page, "_blank", "noopener");
      return null;
    }
    case "server_info":
    case "mud_send":
    case "mud_resize":
    case "map_snapshot":
    case "map_find":
    case "map_directions":
    case "map_walk":
    case "map_stop":
    case "map_set_landmark":
    case "map_clear":
    case "cast_list":
    case "fights_list":
    case "fight_page":
    case "hooks_count":
    case "ambience_now":
    case "who_now":
    case "who_ask":
    case "echo_list":
    case "portrait_paint":
    case "cast_set_voice":
    case "cast_reset":
    case "cast_forget":
    case "journal_list":
    case "journal_last":
    case "journal_unheard":
    case "journal_unheard_entries":
    case "journal_set_heard":
    case "journal_heard_all":
    case "bestiary_mobs":
    case "bestiary_mob":
    case "bestiary_items":
    case "bestiary_item":
    case "bestiary_face":
    case "fight_paint":
    case "bestiary_item_art":
    case "bestiary_auto_look":
    case "automation_set":
    case "channels_ask":
    case "channel_set":
    case "channels_all":
    case "automation_recent":
    case "automation_suggest":
    case "automation_match":
    case "picture_now":
    case "picture_again":
    case "picture_forget_looks":
    case "picture_paint":
      return call(x, cmd, args);
    case "asset_audio":
      return (await mirrored(String(args.path), Boolean(args.looping))).arrayBuffer();
    case "asset_picture":
      return (await mirrored(String(args.path))).arrayBuffer();
    case "asset_ansi": {
      // In the game's colors, as the mirror drew them (examples/web_mirror.rs): the 16's when there's no other.
      const path = String(args.path);
      const colors = String(args.colors ?? "16");
      return (await mirrored(colors !== "16" && mirrorFiles[`${path}#${colors}`] ? `${path}#${colors}` : path)).json();
    }
    case "assets_list":
      return { assets: [], cleared: [] };
    case "soundbank_list":
      return [];
    case "voice_engines":
      return [];
    case "screen_reader_running":
      // A web page can't tell: Coupler's voice doesn't take turns there.
      return false;
    case "voice_prerender":
      return null;
    case "window_is_fullscreen":
      return document.fullscreenElement !== null;
    case "window_set_fullscreen":
      if (args.on) await document.documentElement.requestFullscreen();
      else if (document.fullscreenElement) await document.exitFullscreen();
      return null;
    default:
      throw "That's in the desktop Coupler, not on the web.";
  }
}

/** Only the desktop imports files over a channel (the asset import). */
export class Channel<T> {
  onmessage: (message: T) => void = () => {};
}

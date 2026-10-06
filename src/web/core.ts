/**
 * The web build's stand-in for `@tauri-apps/api/core` (vite.config.ts
 * points the import here in `--mode web`): the desktop's commands, by the
 * same names and arguments, answered in the page. The game logic is the
 * desktop's own Rust, compiled to WebAssembly (`src-web/`) and loaded
 * here with no generated glue (its interface is `src-web/src/ffi.rs`).
 *
 * The only network this page uses: the game's WebSocket, whose address
 * comes from the Rust (`ports.rs`, `web_socket_url`), and its own files
 * (the module, the mirror). The page's Content-Security-Policy allows
 * nothing else (`web/_headers`).
 *
 * What the desktop keeps in app data, the browser keeps for this site:
 * each world's map and cast in IndexedDB (`coupler` database), settings in
 * localStorage as on the desktop. The hooks and assets are the mirror's
 * (`mirror.json` and `mirror/`, made by
 * `src-tauri/examples/web_mirror.rs`), read-only.
 */
import wasmUrl from "./coupler.wasm?url";
import { emit } from "./event";

interface Exports {
  memory: WebAssembly.Memory;
  coupler_alloc(len: number): number;
  coupler_free(ptr: number, len: number): void;
  coupler_call(ptr: number, len: number): number;
  coupler_feed(ptr: number, len: number): number;
  coupler_take_outgoing(): void;
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
    const mirror = await readMirror();
    run(x, "call", encoder.encode(JSON.stringify({ cmd: "init", version: __APP_VERSION__, mirror })));
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

/** Writes `input` into the module, runs `what`, and reads the output buffer. */
function run(x: Exports, what: "call" | "feed", input: Uint8Array): Uint8Array {
  const ptr = x.coupler_alloc(input.length);
  new Uint8Array(x.memory.buffer, ptr, input.length).set(input);
  const failed = what === "call" ? x.coupler_call(ptr, input.length) : x.coupler_feed(ptr, input.length);
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
    if (name === "coupler-save") void save(payload as { kind: string; world: string; json: string });
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

async function save({ kind, world, json }: { kind: string; world: string; json: string }) {
  try {
    const d = await store();
    d.transaction("kept", "readwrite").objectStore("kept").put(json, `${kind}/${world}`);
  } catch (e) {
    console.warn(`couldn't keep the ${kind}: ${e}`);
  }
}

// ---- The connection ----

async function connect(x: Exports, portId: string): Promise<void> {
  if (socket) return;
  const world = call<string | null>(x, "world_of", { portId });
  if (!world) throw "Coupler doesn't know that way to play.";
  const [map, cast] = await Promise.all([load(`map/${world}`), load(`cast/${world}`)]);
  const url = call<string>(x, "mud_connect", { portId, map, cast });
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

// ---- The mirror's assets ----

async function mirrored(path: string, looping = false): Promise<Response> {
  const file = (looping && mirrorFiles[`${path}#loop`]) || mirrorFiles[path];
  if (!file) throw `${path} isn't on the web.`;
  const r = await fetch(`mirror/${file}`);
  // Pages answers a missing file with the page itself.
  if (!r.ok || (r.headers.get("content-type") ?? "").includes("text/html")) throw `${path} couldn't be fetched.`;
  return r;
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
    case "hooks_count":
    case "ambience_now":
    case "who_now":
    case "who_ask":
    case "echo_list":
    case "portrait_paint":
      return call(x, cmd, args);
    case "asset_audio":
      return (await mirrored(String(args.path), Boolean(args.looping))).arrayBuffer();
    case "asset_picture":
      return (await mirrored(String(args.path))).arrayBuffer();
    case "asset_ansi":
      return (await mirrored(String(args.path))).json();
    case "assets_list":
      return { assets: [], cleared: [] };
    case "journal_unheard":
      return { journal: 0, log: 0, speakers: [] };
    case "voice_engines":
      return [];
    case "screen_reader_running":
      // A web page can't tell: Coupler's voice doesn't take turns there.
      return false;
    case "voice_prerender":
    case "picture_now":
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

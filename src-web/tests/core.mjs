// The web core's interface (src-web/src/ffi.rs), driven as the page
// drives it (src/web/core.ts), in Node: a game's telnet bytes in, the
// desktop's events out. Run by scripts/web-build.sh after building:
//   node src-web/tests/core.mjs src/web/coupler.wasm
import fs from "node:fs";
import assert from "node:assert/strict";

const bytes = fs.readFileSync(process.argv[2] ?? "src/web/coupler.wasm");
const { instance } = await WebAssembly.instantiate(bytes, { coupler: { coupler_now_ms: () => performance.now(), coupler_unix_ms: () => Date.now() } });
const x = instance.exports;
const enc = new TextEncoder();
const dec = new TextDecoder();

function run(what, input) {
  const ptr = x.coupler_alloc(input.length);
  new Uint8Array(x.memory.buffer, ptr, input.length).set(input);
  const failed = what === "call" ? x.coupler_call(ptr, input.length) : x.coupler_feed(ptr, input.length);
  x.coupler_free(ptr, input.length);
  const out = new Uint8Array(x.memory.buffer, x.coupler_out_ptr(), x.coupler_out_len()).slice();
  if (failed) throw new Error(dec.decode(out));
  return out;
}
const call = (cmd, args = {}) => JSON.parse(dec.decode(run("call", enc.encode(JSON.stringify({ ...args, cmd })))));
const outgoing = () => {
  x.coupler_take_outgoing();
  return [...new Uint8Array(x.memory.buffer, x.coupler_out_ptr(), x.coupler_out_len())];
};
const events = () => call("take_events");

const mirror = JSON.stringify({
  version: 1,
  files: { "wav/bell.wav": "abc.opus" },
  triggers: [{ key: "room.info.zone", value: '"Midgaard"', trigger: { sfx: "wav/bell.wav", sfxVolume: 40 } }],
});
// A mirror that isn't hooks (Cloudflare Pages answers a missing
// mirror.json with the page itself) means no hooks, not no Coupler.
call("init", { version: "0.0.0", mirror: "<!doctype html><html></html>" });
assert.equal(call("hooks_count"), 0);
assert.equal(call("server_info").host, "coffeemud.net");

call("init", { version: "0.0.0", mirror });
assert.equal(call("hooks_count"), 1);

// The only address: the game's WebSocket, by the port's ID.
const info = call("server_info");
assert.equal(info.host, "coffeemud.net");
assert.equal(call("world_of", { portId: "standard" }), "standard");
assert.equal(call("mud_connect", { portId: "standard", map: "", cast: "" }), "wss://coffeemud.net/WebSock?port=23");
assert.throws(() => call("mud_connect", { portId: "elsewhere", map: "", cast: "" }), /doesn't know/);
assert.equal(call("connected"), true);

const IAC = 255, WILL = 251, DO = 253, SB = 250, SE = 240, GMCP = 201;
// The game offers GMCP: Coupler agrees and says hello.
run("feed", Uint8Array.from([IAC, WILL, GMCP]));
const reply = outgoing();
assert.deepEqual(reply.slice(0, 3), [IAC, DO, GMCP]);
assert.ok(dec.decode(Uint8Array.from(reply)).includes("Core.Hello"));
events();

// Text, then a room with a mirrored trigger.
const room = enc.encode('Room.Info {"num":1,"id":"Midgaard#3001","name":"The Temple","zone":"Midgaard","terrain":"city","exits":{"N":"Midgaard#3002"}}');
run("feed", Uint8Array.from([...enc.encode("Welcome to \x1b[1;33mCoffeeMUD\x1b[0m!\r\n"), IAC, SB, GMCP, ...room, IAC, SE]));
const got = Object.fromEntries(events().map(([name, payload]) => [name, payload]));
const text = got["mud-output"].lines.map((l) => l.map((s) => s.text).join("")).join("\n");
assert.equal(text, "Welcome to CoffeeMUD!");
assert.equal(got["hook-fired"][0].trigger.sfx, "wav/bell.wav");
assert.equal(got["hook-fired"][0].trigger.sfxVolume, 40);
assert.equal(got["map-changed"].room.id, "Midgaard#3001");
assert.equal(got["mud-gmcp"].package, "Room.Info");

// A typed line goes out as telnet; hanging up says so, and keeps the map.
call("mud_send", { line: "look" });
assert.equal(dec.decode(Uint8Array.from(outgoing())), "look\r\n");
call("mud_closed", { reason: null });
const closed = events();
assert.ok(closed.some(([n]) => n === "mud-closed"));
assert.ok(closed.some(([n, p]) => n === "coupler-save" && p.kind === "map" && p.json.includes("Midgaard#3001")));
assert.throws(() => call("picture_paint"), /web build has no/);
console.log("web core: ok");

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

call("init", { version: "0.0.0", mirror, journal: "", bestiary: "", played: 2 });
assert.equal(call("hooks_count"), 1);

// The only address: the game's WebSocket, by the port's ID.
const info = call("server_info");
assert.equal(info.host, "coffeemud.net");
assert.equal(call("world_of", { portId: "standard" }), "standard");
assert.equal(call("mud_connect", { portId: "standard", map: "", cast: "", looks: "" }), "wss://coffeemud.net/WebSock?port=23");
assert.throws(() => call("mud_connect", { portId: "elsewhere", map: "", cast: "", looks: "" }), /doesn't know/);
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
// The room's picture, by Coupler's painter.
assert.equal(got["picture"].room, "Midgaard#3001");
assert.equal(got["picture"].name, "The Temple");
const painted = call("picture_paint", { scene: got["picture"], style: "fantasy", columns: 20, rows: 6, again: 0 });
assert.equal(painted.lines.length, 6);
assert.equal(call("picture_again").look, 1);
assert.ok(events().some(([n, p]) => n === "coupler-save" && p.key === "pictures/standard.json"));

const gmcp = (pkg, data) => [IAC, SB, GMCP, ...enc.encode(`${pkg} ${JSON.stringify(data)}`), IAC, SE];
// A character in the game: counted once, as the desktop's played.json.
run("feed", Uint8Array.from(gmcp("Char.Base", { name: "Tester" })));
let saved = events().filter(([n]) => n === "coupler-save").map(([, p]) => p);
assert.ok(saved.some((p) => p.key === "played.json" && JSON.parse(p.json).played === 3));
outgoing();

// Talk goes in the journal, or a channel in the log, as on the desktop.
run("feed", Uint8Array.from([...gmcp("Comm.Channel", { chan: "OOC", msg: "Bob OOCs 'hi all'", player: "Bob" }), ...enc.encode("Bob OOCs 'hi all'\r\n")]));
const said = events();
const talk = said.find(([n]) => n === "talk")[1];
assert.ok(said.some(([n, p]) => n === "coupler-save" && p.key === "web/journal.json"));
assert.equal(talk.entry.book, "log");
assert.equal(call("journal_unheard").log, 1);
assert.equal(call("journal_list", { book: "log", query: "" }).entries.length, 1);
assert.equal(call("journal_last", { line: "ooc last 5" }).entries[0].text, "Bob OOCs 'hi all'");
call("journal_heard_all", { book: "log" });
assert.equal(call("journal_unheard").log, 0);

// A creature met goes in the Bestiary, with its face.
run("feed", Uint8Array.from(gmcp("Room.Mobiles", { npcs: [{ "a fat rat": "rat" }] })));
const met = events();
assert.ok(met.some(([n]) => n === "bestiary-changed"));
assert.ok(met.some(([n, p]) => n === "coupler-save" && p.key === "web/bestiary.json"));
const mobs = call("bestiary_mobs", { query: "rat" });
assert.equal(mobs[0].name, "a fat rat");
assert.equal(call("bestiary_mob", { key: mobs[0].key }).name, "a fat rat");
assert.ok(call("bestiary_face", { key: mobs[0].key }).lines.length > 0);
// Kept again once changed, when the page asks (it's hidden or closing).
call("journal_set_heard", { ids: [talk.entry.id], heard: false });
call("save_now");
assert.ok(events().some(([n, p]) => n === "coupler-save" && p.key === "web/journal.json"));

// A Coupler Backup, the desktop's format: written and read back.
const backupBytes = (what, input) => {
  const ptr = x.coupler_alloc(input.length);
  new Uint8Array(x.memory.buffer, ptr, input.length).set(input);
  const failed = x[what](ptr, input.length);
  x.coupler_free(ptr, input.length);
  const out = new Uint8Array(x.memory.buffer, x.coupler_out_ptr(), x.coupler_out_len()).slice();
  if (failed) throw new Error(dec.decode(out));
  return out;
};
const files = [["maps/standard.json", '{"rooms":[]}'], ["web/journal.json", '{"rows":[]}'], ["voice-cache/x", "left out"]];
const made = backupBytes("coupler_backup_write", enc.encode(JSON.stringify({ app: "0.0.0", made: 1, settings: '{"coupler.theme":"modern"}', files })));
assert.equal(dec.decode(made.slice(0, 14)), "COUPLER BACKUP");
const back = JSON.parse(dec.decode(backupBytes("coupler_backup_read", made)));
assert.deepEqual(back.files, files.slice(0, 2));
assert.equal(back.settings, '{"coupler.theme":"modern"}');
assert.throws(() => backupBytes("coupler_backup_read", enc.encode("not a backup")), /isn't a Coupler Backup/);

// The greeting's probe: MSSP's table on a socket of its own.
assert.equal(call("probe_start", { portId: "standard" }), "wss://coffeemud.net/WebSock?port=23");
const MSSP = 70;
const offer = call("probe_feed", { bytes: [IAC, WILL, MSSP] });
assert.deepEqual(offer.reply, [IAC, DO, MSSP]);
const table = [IAC, SB, MSSP, 1, ...enc.encode("PLAYERS"), 2, ...enc.encode("7"), 1, ...enc.encode("CODEBASE"), 2, ...enc.encode("CoffeeMUD v5.11.0.4"), IAC, SE];
assert.equal(call("probe_feed", { bytes: table }).done, true);
const counts = call("probe_result");
assert.deepEqual([counts.played, counts.online, counts.version, counts.fit], [3, 7, "5.11.0.4", "same"]);

// A typed line goes out as telnet; hanging up says so, and keeps the map.
call("mud_send", { line: "look" });
assert.equal(dec.decode(Uint8Array.from(outgoing())), "look\r\n");
call("mud_closed", { reason: null });
const closed = events();
assert.ok(closed.some(([n]) => n === "mud-closed"));
assert.ok(closed.some(([n, p]) => n === "coupler-save" && p.key === "maps/standard.json" && p.json.includes("Midgaard#3001")));
assert.throws(() => call("hooks_list"), /web build has no/);
console.log("web core: ok");

/**
 * **Coupler's own commands**, typed on the command line after a prefix
 * (`#` unless the player chose another: `coupler.commandPrefix`), pure:
 * what's typed read as a command and its words. They're never sent to
 * the game nor written in the output (App.tsx runs them, with a cue and
 * a word in the status bar); the prefix twice sends the line to the
 * game with one of it (`##` is `#`).
 *
 * They're the Music Editor's (lib/musicEditor.ts): its player, saving,
 * writing the song into a journal, and hooking it to the room or area as
 * its music; and the tables' (src-tauri/src/tables.rs, gear → Tables…):
 * `#check` says one (`#check loot`: what the last fight brought), `#add`
 * adds a row to one of the player's.
 */

export interface CommandInfo {
  name: string;
  /** Its words after the name, as help shows them. */
  usage: string;
  /** What it does, in a few words. */
  does: string;
  /** Only in the desktop app (the Music Editor is). */
  desktop?: boolean;
}

export const COMMANDS: CommandInfo[] = [
  { name: "play", usage: "", does: "plays the Music Editor's song, from where it paused", desktop: true },
  { name: "pause", usage: "", does: "pauses it; play goes on from there", desktop: true },
  { name: "rewind", usage: "", does: "goes back to its start", desktop: true },
  { name: "loop", usage: "[on|off]", does: "plays it over and over, or once", desktop: true },
  { name: "stop", usage: "", does: "stops it, back at the start", desktop: true },
  { name: "save", usage: "[name]", does: "saves it over its file, or as a new one named so", desktop: true },
  { name: "write", usage: "<journal>", does: "writes it into a journal (a clan's is its forum) for others to load", desktop: true },
  { name: "load", usage: "", does: "opens the music last read in a journal", desktop: true },
  { name: "hook", usage: "room|area", does: "makes it the music of this room or area", desktop: true },
  { name: "unhook", usage: "room|area", does: "takes the music off this room or area", desktop: true },
  { name: "check", usage: "[table]", does: "says a table: loot is what your last fight brought, stats your character's (read again from the game); with no table, lists them" },
  { name: "add", usage: "<table> <cell> | <cell>…", does: "adds a row to a table of yours, its cells split by |" },
  { name: "prefix", usage: "<character>", does: "types Coupler's commands after another character" },
  { name: "help", usage: "[command]", does: "lists these; with a command's name, says what it does" },
];

const PREFIX_KEY = "coupler.commandPrefix";
export const DEFAULT_PREFIX = "#";

/** A prefix can be one character that isn't a letter, a digit or a space. */
export const isPrefix = (c: string) => [...c].length === 1 && !/[\p{L}\p{N}\s]/u.test(c);

export function prefix(): string {
  try {
    const p = localStorage.getItem(PREFIX_KEY);
    return p && isPrefix(p) ? p : DEFAULT_PREFIX;
  } catch {
    return DEFAULT_PREFIX;
  }
}

export function setPrefix(p: string) {
  try {
    if (p === DEFAULT_PREFIX) localStorage.removeItem(PREFIX_KEY);
    else localStorage.setItem(PREFIX_KEY, p);
  } catch {
    // Not kept past this run, then.
  }
}

export type Parsed =
  /** Not a command of Coupler's: for the game. */
  | { kind: "game" }
  /** The prefix twice: this line goes to the game. */
  | { kind: "literal"; line: string }
  /** A command: its name (lower case, a known one or not) and the rest. */
  | { kind: "command"; name: string; rest: string; info: CommandInfo | null };

/** What a typed line is. A name can be cut short to what starts only one command (`#pa` is pause). */
export function parse(line: string, p = prefix()): Parsed {
  const text = line.trimStart();
  if (!text.startsWith(p)) return { kind: "game" };
  const after = text.slice(p.length);
  if (after.startsWith(p)) return { kind: "literal", line: after };
  const match = /^(\S*)\s*([\s\S]*)$/.exec(after);
  const name = (match?.[1] ?? "").toLowerCase();
  const rest = (match?.[2] ?? "").trim();
  if (name === "") return { kind: "command", name: "help", rest, info: COMMANDS.find((c) => c.name === "help")! };
  const exact = COMMANDS.find((c) => c.name === name);
  const starting = COMMANDS.filter((c) => c.name.startsWith(name));
  const info = exact ?? (starting.length === 1 ? starting[0] : null);
  return { kind: "command", name: info?.name ?? name, rest, info };
}

const usable = (desktop: boolean) => COMMANDS.filter((c) => desktop || !c.desktop);
const typed = (c: CommandInfo, p: string) => `${p}${c.name}${c.usage ? ` ${c.usage}` : ""}`;

/** The commands' names in a line, said and put in the status bar: the list itself is `helpLines`. */
export function helpText(p = prefix(), desktop = true): string {
  return `Coupler's commands: ${usable(desktop).map((c) => c.name).join(", ")}. ${p}help and a command's name says what it does.`;
}

/** Every command with its words and what it does, a line each, for the output. */
export function helpLines(p = prefix(), desktop = true): string[] {
  const list = usable(desktop);
  const width = Math.max(...list.map((c) => typed(c, p).length));
  return [
    `Coupler's commands, never sent to the game (a name can be cut short):`,
    ...list.map((c) => `  ${typed(c, p).padEnd(width)}  ${c.does}`),
    `  ${p}${p} sends a line starting with ${p} to the game.`,
  ];
}

/** One command's words and what it does, or null for none of that name. */
export function helpFor(name: string, p = prefix(), desktop = true): string | null {
  const bare = name.trim().toLowerCase();
  const want = bare.startsWith(p) ? bare.slice(p.length) : bare;
  const starting = usable(desktop).filter((c) => c.name.startsWith(want));
  const c = usable(desktop).find((c) => c.name === want) ?? (starting.length === 1 ? starting[0] : null);
  if (!want || !c) return null;
  const does = c.does.charAt(0).toUpperCase() + c.does.slice(1);
  return `${typed(c, p)}: ${does}.`;
}

/**
 * `#add`'s words: the table (its name or the start of it, one word, or
 * any words in quotes) and the row's cells, split by `|`.
 */
export function addWords(rest: string): { table: string; cells: string[] } | null {
  const quoted = /^"([^"]+)"\s*([\s\S]*)$/.exec(rest.trim()) ?? /^(\S+)\s*([\s\S]*)$/.exec(rest.trim());
  if (!quoted) return null;
  const cells = quoted[2].split("|").map((c) => c.trim());
  return cells.some((c) => c !== "") ? { table: quoted[1], cells } : null;
}


/** Commands that start with `name`, when it's ambiguous: for the error. */
export const candidates = (name: string) => COMMANDS.filter((c) => c.name.startsWith(name)).map((c) => c.name);

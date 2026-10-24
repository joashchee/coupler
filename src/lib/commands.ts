/**
 * **Coupler's own commands**, typed on the command line after a prefix
 * (`#` unless the player chose another: `coupler.commandPrefix`), pure:
 * what's typed read as a command and its words. They're never sent to
 * the game nor written in the output (App.tsx runs them, with a cue and
 * a word in the status bar); the prefix twice sends the line to the
 * game with one of it (`##` is `#`).
 *
 * Today they're the Music Editor's (lib/musicEditor.ts): its player,
 * saving, writing the song into a journal, and hooking it to the room
 * or area as its music.
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
  { name: "prefix", usage: "<character>", does: "types Coupler's commands after another character" },
  { name: "help", usage: "", does: "lists these" },
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

/** The commands in a line, for the status bar and the screen reader. */
export function helpText(p = prefix(), desktop = true): string {
  const list = COMMANDS.filter((c) => desktop || !c.desktop).map((c) => `${p}${c.name}${c.usage ? ` ${c.usage}` : ""}`);
  return `Coupler's commands: ${list.join(", ")}. ${p}${p} sends a line starting with ${p} to the game.`;
}

/** Commands that start with `name`, when it's ambiguous: for the error. */
export const candidates = (name: string) => COMMANDS.filter((c) => c.name.startsWith(name)).map((c) => c.name);

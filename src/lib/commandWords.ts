/**
 * The smart tips for an alias's or a ticker's commands: every command,
 * skill and spell CoffeeMUD has, with its usage, examples and a line
 * about it, from `public/command-words.json` (made from the snapshot's
 * source and help by scripts/command-words.py), fetched the first time
 * a tip's wanted. `tipsFor` reads the line being typed as the game does
 * (`EnglishParser.findCommand`: the first word, an exact access word)
 * and says what it is, what could finish it, and what to know. Pure but
 * for the fetch.
 */
import type { Alias } from "./mud";

export interface CommandWord {
  /** Its access words, the first the usual. */
  words: string[];
  kind: "command" | "skill";
  /** A skill's name: "Butchering". */
  name?: string;
  usage: string;
  examples: string[];
  about: string;
}

export interface AbilityWord {
  name: string;
  usage: string;
  examples: string[];
  about: string;
}

export interface Words {
  commands: CommandWord[];
  /** Spells and the like, by the command that uses them: CAST, PRAY, CHANT, SING, PLAY, DANCE. */
  abilities: Record<string, AbilityWord[]>;
}

let book: Promise<Words> | null = null;

/** CoffeeMUD's words, fetched once. */
export function loadWords(): Promise<Words> {
  book ??= fetch("/command-words.json")
    .then((r) => {
      if (!r.ok) throw new Error(`${r.status}`);
      return r.json() as Promise<Words>;
    })
    .catch((e: unknown) => {
      book = null;
      throw e;
    });
  return book;
}

/** The words that use spells and the like, as each kind's base class has them. */
const INVOKERS: Record<string, string> = { CAST: "CAST", CA: "CAST", C: "CAST", PRAY: "PRAY", PR: "PRAY", CHANT: "CHANT", CH: "CHANT", SING: "SING", SI: "SING", PLAY: "PLAY", PL: "PLAY", PLA: "PLAY", DANCE: "DANCE", DA: "DANCE" };

export interface Tip {
  /** What it's called: "GET", "Magic Missile", "kk (your alias)". */
  label: string;
  /** The line with it put in. */
  line: string;
  usage: string;
  about: string;
}

export interface Tips {
  /** What the line's command is, if it's one: its usage and the rest. */
  known: { label: string; usage: string; examples: string[]; about: string } | null;
  /** What could finish the word being typed. */
  suggestions: Tip[];
  /** Things to know, in words. */
  notes: string[];
}

const MOST = 8;

const quote = (name: string) => (name.includes(" ") ? `"${name.toLowerCase()}"` : name.toLowerCase());

/**
 * Tips for one line of commands. `aliases` are the player's (they're
 * expanded before the game sees them); `self` is the alias being
 * written, which can't use itself.
 */
export function tipsFor(line: string, words: Words | null, aliases: Alias[], self?: string): Tips {
  const tips: Tips = { known: null, suggestions: [], notes: [] };
  const typed = line.trimStart();
  if (typed === "") return tips;
  if (typed.startsWith("\\")) {
    tips.notes.push("A line starting with \\ is sent as it is, without your aliases.");
    return tips;
  }
  const space = typed.search(/\s/);
  const first = (space < 0 ? typed : typed.slice(0, space)).toUpperCase();
  const rest = space < 0 ? "" : typed.slice(space).trimStart();
  const mine = aliases.filter((a) => a.on && a.name.trim() !== "" && a.name.toLowerCase() !== self?.toLowerCase());
  if (self && first === self.toUpperCase()) tips.notes.push("An alias can't use itself: the game is sent the word as it is.");

  if (space < 0) {
    // The first word, still being typed.
    for (const a of mine.filter((a) => a.name.toUpperCase().startsWith(first))) {
      tips.suggestions.push({ label: `${a.name} (your alias)`, line: `${a.name} `, usage: a.send.split("\n").join("; "), about: "Your own alias." });
    }
    if (words) {
      const exact = words.commands.filter((c) => c.words.includes(first));
      const starts = words.commands.filter((c) => !exact.includes(c) && c.words.some((w) => w.startsWith(first)));
      for (const c of [...exact, ...starts]) {
        const word = c.words.find((w) => w === first) ?? c.words.find((w) => w.startsWith(first)) ?? c.words[0];
        tips.suggestions.push({ label: c.name ? `${word} (${c.name})` : word, line: `${word.toLowerCase()} `, usage: c.usage, about: c.about });
        if (tips.suggestions.length >= MOST) break;
      }
    }
  }

  const alias = mine.find((a) => a.name.toUpperCase() === first);
  if (alias) {
    tips.known = { label: `${alias.name}, your alias`, usage: alias.send.split("\n").join("; "), examples: [], about: "Your own alias: its commands go in its place." };
    return tips;
  }
  if (!words) return tips;
  const command = words.commands.find((c) => c.words.includes(first));
  const invoker = INVOKERS[first];
  if (command) {
    const others = command.words.filter((w) => w !== first);
    tips.known = {
      label: command.name ? `${first}: ${command.name}` : first,
      usage: command.usage,
      examples: command.examples,
      about: command.about + (others.length > 0 ? ` Also ${others.join(", ")}.` : ""),
    };
  }
  if (invoker && space >= 0) {
    // CAST "magic missile" orc: the spell's name, in quotes when it's more than a word.
    const list = words.abilities[invoker] ?? [];
    const open = rest.startsWith('"') || rest.startsWith("'");
    const name = open ? rest.slice(1).split(/["']/)[0] : rest;
    const lower = name.toLowerCase().trim();
    const exact = list.find((a) => (open ? lower === a.name.toLowerCase() : lower.startsWith(a.name.toLowerCase()) && (lower.length === a.name.length || lower[a.name.length] === " ")));
    if (exact) {
      tips.known = { label: `${first} ${exact.name}`, usage: exact.usage, examples: exact.examples, about: exact.about };
    } else if (lower !== "") {
      const starts = list.filter((a) => a.name.toLowerCase().startsWith(lower));
      const inside = list.filter((a) => !starts.includes(a) && a.name.toLowerCase().includes(lower));
      for (const a of [...starts, ...inside].slice(0, MOST)) {
        tips.suggestions.push({ label: a.name, line: `${first.toLowerCase()} ${quote(a.name)} `, usage: a.usage, about: a.about });
      }
      if (starts.length + inside.length === 0) tips.notes.push(`No ${invoker.toLowerCase()} of CoffeeMUD's is called that. A name of more than one word goes in quotes.`);
    }
  } else if (!command && space >= 0) {
    const could = words.commands.filter((c) => c.words.some((w) => w.startsWith(first))).map((c) => c.words[0]);
    tips.notes.push(
      could.length > 0
        ? `${first} isn't one of CoffeeMUD's words in full. The game also takes a short form, so it could be ${could.slice(0, 4).join(", ")}${could.length > 4 ? " or another" : ""}: write it out to be sure.`
        : `${first} isn't a command Coupler knows of CoffeeMUD's. It may still be a social, a channel or a skill the game knows.`,
    );
  }
  return tips;
}

/** Notes on the commands as a whole: placeholders, how many, and the like. */
export function sendNotes(send: string, of: "alias" | "ticker" | "trigger"): string[] {
  const lines = send
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
  const notes: string[] = [];
  if (lines.length > 1) notes.push(`${lines.length} commands, sent one after another.`);
  const places = send.match(/%[0-9*]/g) ?? [];
  if (of === "alias") {
    if (places.some((p) => p === "%0")) notes.push("%0 is only for triggers; in an alias, %* is all the words typed after it.");
    if (places.length === 0) notes.push("Words typed after the alias go on the end of its last command. Use %1, %2… or %* to put them elsewhere.");
    else notes.push("%1 is the first word typed after the alias, %2 the second, %* all of them.");
  } else if (of === "ticker" && places.length > 0) {
    notes.push("A ticker has no words to put in place of %1 or %*: they're sent empty.");
  }
  return notes;
}

/**
 * The Bestiary's words (components/BestiaryDialog.tsx): what's known of
 * a creature or an item (src-tauri/src/bestiary.rs), a fact a line, the
 * same lines for the eye and for a screen reader. Pure.
 *
 * A fact not known yet says how the game would tell it, by its own
 * commands (CONSIDER, a long look with LL, EXAMINE), never more.
 *
 * And the player's choice of Coupler looking at each creature never
 * looked at, once, unobtrusively (`coupler.bestiaryLook`, off at first),
 * handed to Rust at launch and on each change.
 */
import { bestiaryAutoLook, type Counted, type ItemEntry, type ItemRow, type MobArea, type MobEntry, type MobRow } from "./mud";

const LOOK_KEY = "coupler.bestiaryLook";

/** Whether Coupler looks at each new creature by itself. */
export function autoLook(): boolean {
  try {
    return localStorage.getItem(LOOK_KEY) === "on";
  } catch {
    return false;
  }
}

/** Keeps the choice and tells Rust. */
export function setAutoLook(on: boolean): Promise<void> {
  try {
    localStorage.setItem(LOOK_KEY, on ? "on" : "off");
  } catch {
    // Not kept, but still on for this run.
  }
  return bestiaryAutoLook(on);
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
const times = (n: number) => (n === 1 ? "once" : n === 2 ? "twice" : `${n} times`);

/** "October 3, 14:05": when, in words. */
export function whenWords(at: number): string {
  return new Date(at).toLocaleString(undefined, { month: "long", day: "numeric", hour: "numeric", minute: "2-digit" });
}

/** A level range: "level 10", "level 8 to 12", "level 14 or more". */
export function levels(low: number | null, high: number | null): string | null {
  if (low === null && high === null) return null;
  if (high === null) return `level ${low} or more`;
  if (low === null || low === high) return `level ${high}`;
  return `level ${low} to ${high}`;
}

/** "2 leg(s), 1 arm(s) and a tail" as it reads: "2 legs, 1 arm and a tail". */
export function body(text: string): string {
  return text.replace(/(\d+) (\w+?)\(s\)/g, (_, n: string, word: string) => `${n} ${word}${n === "1" ? "" : "s"}`);
}

/** A list in words: "a, b and c". */
function list(items: string[]): string {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/** Names counted, each with how often when more than once. */
function counted(items: Counted[], what: (c: Counted) => string | null = (c) => c.what): string {
  return list(
    items.map((c) => {
      const notes = [what(c), c.count > 1 ? times(c.count) : null].filter(Boolean);
      return notes.length > 0 ? `${c.name} (${notes.join(", ")})` : c.name;
    }),
  );
}

/** One area's line. */
function areaLine(a: MobArea): string {
  const facts = [`seen ${times(a.seen || 1)}`, levels(a.levelLow, a.levelHigh), a.maxHp !== null ? plural(a.maxHp, "hit point", "hit points") : null, a.alignment];
  return `In ${a.area}: ${facts.filter(Boolean).join(", ")}.`;
}

/** A creature's row in the list, in short. */
export function mobSummary(m: MobRow): string {
  return m.race ?? "Race not known";
}

/** Everything known of a creature, a fact a line. */
export function mobFacts(m: MobEntry): string[] {
  const lines: string[] = [];
  if (m.race) {
    const kind = m.category && m.category !== m.race ? ` (${m.category})` : "";
    lines.push(`Race: ${m.race}${kind}, ${m.raceSeen ? "as the game said" : "read from its name"}.`);
  } else {
    lines.push("Race: not known yet. A long look (LL) tells it when it's stronger or weaker than most of its kind.");
  }
  if (m.creature) lines.push(`Its cry: CoffeeMUD's ${m.creature} sound, as it comes, fights you and dies.`);
  if (m.seems) lines.push(`Seems to be: a ${m.seems}, by the abilities it uses.`);
  if (m.height !== null && m.weight !== null) lines.push(`Size: ${m.sex && m.sex !== "neuter" ? `${m.sex}, ` : ""}${m.height} inches tall, ${plural(m.weight, "pound", "pounds")}.`);
  if (m.body) lines.push(`Body: ${body(m.body)}.`);
  if (m.stats.length > 0) lines.push(`Beside the rest of its kind: ${list(m.stats)}.`);
  for (const a of m.areas) lines.push(areaLine(a));
  if (m.areas.some((a) => a.levelLow === null && a.levelHigh === null)) lines.push("Its level: CONSIDER it, or EXAMINE its body, to learn it.");
  if (m.abilities.length > 0) lines.push(`Uses: ${counted(m.abilities)}.`);
  if (m.drops.length > 0) lines.push(`Dropped: ${counted(m.drops)}.`);
  if (m.wears.length > 0) lines.push(`Wears: ${counted(m.wears, (c) => c.what)}.`);
  if (m.carried.length > 0) lines.push(`Carried, seen in its body: ${counted(m.carried)}.`);
  if (m.deaths > 0) lines.push(`Seen to die ${times(m.deaths)}.`);
  lines.push(`First seen ${whenWords(m.firstSeen)}, last ${whenWords(m.lastSeen)}.`);
  if (m.description) lines.push(m.description);
  else lines.push("Look at it (LOOK) for its description, its size and what it wears.");
  return lines;
}

/** An item's row in the list, in short. */
export function itemSummary(i: ItemRow): string {
  return levels(i.levelLow, i.levelHigh) ?? "Level not known";
}

/** Everything known of an item, a fact a line; `slot` where it goes, as its picture writes it. */
export function itemFacts(i: ItemEntry, slot: string | null = null): string[] {
  const lines: string[] = [];
  if (slot) lines.push(`Goes: ${slot.toLowerCase()}.`);
  if (i.auras.length > 0) lines.push(`Seen ${list(i.auras)}.`);
  const level = levels(i.levelLow, i.levelHigh);
  const weight =
    i.weightLow === null ? null : i.weightHigh !== null && i.weightHigh !== i.weightLow ? `${i.weightLow} to ${i.weightHigh} pounds` : plural(i.weightLow, "pound", "pounds");
  if (level || weight || i.material) {
    const said = [level ? level.replace(/^level/, "Level") : null, weight ? `weighs ${weight}` : null, i.material ? `made of ${i.material}` : null].filter(Boolean);
    lines.push(`${said.join(", ")}.`);
  } else {
    lines.push("EXAMINE it to learn its level, its weight and what it's made of.");
  }
  if (i.container) lines.push("A container.");
  if (i.areas.length > 0) lines.push(`Found in: ${counted(i.areas, () => null)}.`);
  if (i.droppedBy.length > 0) lines.push(`Dropped by: ${counted(i.droppedBy)}.`);
  if (i.wornBy.length > 0) lines.push(`Worn by: ${counted(i.wornBy)}.`);
  if (i.carriedBy.length > 0) lines.push(`Carried by: ${counted(i.carriedBy)}.`);
  lines.push(`First seen ${whenWords(i.firstSeen)}, last ${whenWords(i.lastSeen)}.`);
  if (i.description) lines.push(i.description);
  return lines;
}

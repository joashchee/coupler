/**
 * The Artisan's skills (CoffeeMUD's `Artisan.java`, as
 * `scripts/artisan-tree.py` writes them into `artisanSkills.ts`): the
 * tree, laid out for the eye, and the mentor, who says in a few words
 * what a skill needs before it can be gained.
 *
 * How the game reads it (`CMAbleMap`): a skill is gained from a
 * guildmaster (GAIN, once QUALIFY lists it) at its level, once every
 * skill it names is known at the proficiency in brackets (none means
 * known at all) and the base stat it names is high enough. The tree is
 * a web, not a tree: Smelting needs both Fire Building and Mining, and
 * Mining opens up Sculpting, Smelting and Masonry too.
 *
 * Pure: no React, no Tauri, so the web build has it as it is.
 */
import { ARTISAN_SKILLS } from "./artisanSkills";

export interface ArtisanSkill {
  /** The game's ID: "Smelting", "Skill_Climb". */
  id: string;
  /** As the game names it: "Wood Chopping". */
  name: string;
  /** The level it's had at. */
  level: number;
  /** A crafting (common) skill, a weapon's proficiency or familiarity, or another skill. */
  kind: "craft" | "weapon" | "skill";
  /** The first word that uses it in the game, if it's used by a word. */
  word?: string;
  /** Had without gaining it (Recall). */
  auto?: boolean;
  /** A base stat it needs: ["Strength", 9]. */
  stat?: [string, number];
  /** The skills it needs and how well, in percent (0: known at all). */
  needs: [string, number][];
}

export const SKILLS: readonly ArtisanSkill[] = ARTISAN_SKILLS;
const BY_ID = new Map(SKILLS.map((s) => [s.id, s]));

export function skill(id: string): ArtisanSkill | undefined {
  return BY_ID.get(id);
}

/** What each skill opens up: the skills that name it. */
const OPENS = new Map<string, string[]>(SKILLS.map((s) => [s.id, []]));
for (const s of SKILLS) for (const [need] of s.needs) OPENS.get(need)?.push(s.id);

export function opensUp(id: string): ArtisanSkill[] {
  return (OPENS.get(id) ?? []).map((o) => BY_ID.get(o)!);
}

/** How far a skill is from the start: 0 needs nothing, else one more than the deepest it needs. */
const TIER = new Map<string, number>();
function tierOf(id: string): number {
  const known = TIER.get(id);
  if (known !== undefined) return known;
  const s = BY_ID.get(id)!;
  const t = s.needs.length === 0 ? 0 : 1 + Math.max(...s.needs.map(([n]) => tierOf(n)));
  TIER.set(id, t);
  return t;
}
for (const s of SKILLS) tierOf(s.id);

export function tier(id: string): number {
  return TIER.get(id) ?? 0;
}

/**
 * Everything a skill needs, however far back, each at the most any
 * skill on the way needs it: Weaponsmithing at 100%, since Master
 * Weaponsmithing wants it there, though Sword Proficiency wants 75.
 */
export function ancestors(id: string): Map<string, number> {
  const out = new Map<string, number>();
  const visit = (at: string) => {
    for (const [need, prof] of BY_ID.get(at)?.needs ?? []) {
      const was = out.get(need);
      if (was === undefined || prof > was) out.set(need, Math.max(prof, was ?? 0));
      if (was === undefined) visit(need);
    }
  };
  visit(id);
  return out;
}

/** Everything a skill opens up, however far on. */
export function descendants(id: string): Set<string> {
  const out = new Set<string>();
  const visit = (at: string) => {
    for (const next of OPENS.get(at) ?? []) {
      if (out.has(next)) continue;
      out.add(next);
      visit(next);
    }
  };
  visit(id);
  return out;
}

/**
 * The columns of the picture: a column a tier, each in an order that
 * keeps a skill near what it needs and what it opens up (a few sweeps
 * of each skill to the average row of its neighbours), so the lines
 * between them cross less.
 */
export function columns(): ArtisanSkill[][] {
  const deepest = Math.max(...SKILLS.map((s) => tier(s.id)));
  const cols: ArtisanSkill[][] = Array.from({ length: deepest + 1 }, () => []);
  for (const s of SKILLS) cols[tier(s.id)].push(s);
  const place = new Map<string, number>();
  const measure = () => cols.forEach((col) => col.forEach((s, i) => place.set(s.id, col.length > 1 ? i / (col.length - 1) : 0.5)));
  const mean = (ids: string[], fallback: number) => (ids.length === 0 ? fallback : ids.reduce((sum, i) => sum + (place.get(i) ?? 0), 0) / ids.length);
  measure();
  for (let sweep = 0; sweep < 6; sweep++) {
    const order = sweep % 2 === 0 ? cols.map((_, i) => i) : cols.map((_, i) => cols.length - 1 - i);
    for (const c of order) {
      const by = new Map(
        cols[c].map((s) => {
          const near = sweep % 2 === 0 ? s.needs.map(([n]) => n) : (OPENS.get(s.id) ?? []);
          return [s.id, mean(near, place.get(s.id) ?? 0)];
        }),
      );
      cols[c].sort((a, b) => by.get(a.id)! - by.get(b.id)! || a.name.localeCompare(b.name));
      measure();
    }
  }
  return cols;
}

/** Letters and digits only, lower case: "Wood Chopping" and "woodchopping" match. */
function squash(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]/g, "");
}

/** Words a question about a skill has that aren't the skill. */
const FILLER = new Set(
  "a an the i me my to do does how can could would get gain learn want need needs needed for what whats skill skills about tell is it of become".split(" "),
);

/** The edit distance between two squashed names, for a misspelling. */
function distance(a: string, b: string): number {
  let row = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const next = [i];
    for (let j = 1; j <= b.length; j++) next[j] = Math.min(row[j] + 1, next[j - 1] + 1, row[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    row = next;
  }
  return row[b.length];
}

export interface Found {
  /** The one skill meant, if it's clear. */
  skill?: ArtisanSkill;
  /** Otherwise the skills it might be, best first (none: nothing like it). */
  maybe: ArtisanSkill[];
}

/**
 * The skill a player asks for, by its name, its game ID or the word
 * that uses it, in any case, with or without spaces, inside a question
 * ("how do I get blacksmithing?") or misspelt a little.
 */
export function find(asked: string): Found {
  const words = asked
    .toLowerCase()
    .replace(/[^a-z0-9' ]/g, " ")
    .split(/\s+/)
    .map((w) => w.replace(/'/g, ""))
    .filter((w) => w !== "" && !FILLER.has(w));
  const q = squash(words.join(""));
  if (q === "") return { maybe: [] };
  const keys = (s: ArtisanSkill) => [squash(s.name), squash(s.id), squash(s.id.replace(/^[A-Za-z]+_/, "")), ...(s.word ? [squash(s.word)] : [])];
  const exact = SKILLS.filter((s) => keys(s).includes(q));
  if (exact.length === 1) return { skill: exact[0], maybe: [] };
  if (exact.length > 1) return { maybe: exact };
  const starts = SKILLS.filter((s) => keys(s).some((k) => k.startsWith(q)));
  if (starts.length === 1) return { skill: starts[0], maybe: [] };
  const inside = starts.length > 0 ? starts : SKILLS.filter((s) => squash(s.name).includes(q));
  if (inside.length === 1) return { skill: inside[0], maybe: [] };
  if (inside.length > 1) return { maybe: [...inside].sort((a, b) => a.name.length - b.name.length || a.name.localeCompare(b.name)) };
  const near = SKILLS.map((s) => ({ s, d: Math.min(...keys(s).map((k) => distance(q, k))) }))
    .filter(({ s, d }) => d <= Math.max(2, Math.floor(Math.min(q.length, squash(s.name).length) / 4)))
    .sort((a, b) => a.d - b.d || a.s.name.localeCompare(b.s.name));
  if (near.length > 0 && (near.length === 1 || near[0].d < near[1].d)) return { skill: near[0].s, maybe: [] };
  return { maybe: near.map(({ s }) => s) };
}

/** "A", "A and B", "A, B and C". */
export function andList(items: string[]): string {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/** One skill wanted at a proficiency: "Mining to 75%", "Composting at any skill". */
function wanted(id: string, prof: number): string {
  const name = BY_ID.get(id)!.name;
  return prof > 0 ? `${name} to ${prof}%` : `${name} at any skill`;
}

/** A step's skills, those wanted equally well said together: "Fire Building and Mining to 75%". */
function stepWords(step: [string, number][]): string {
  const byProf = new Map<number, string[]>();
  for (const [id, prof] of step) byProf.set(prof, [...(byProf.get(prof) ?? []), id]);
  return andList(
    [...byProf.entries()]
      .sort(([a], [b]) => b - a)
      .map(([prof, ids]) => {
        const names = ids.map((i) => BY_ID.get(i)!.name);
        return prof > 0 ? `${andList(names)} to ${prof}%` : `${andList(names)} at any skill`;
      }),
  );
}

/**
 * The order to learn what a skill needs: a step a tier, what's needed
 * earliest first, each skill at the proficiency it's wanted at.
 */
export function plan(id: string): [string, number][][] {
  const need = ancestors(id);
  const steps = new Map<number, [string, number][]>();
  for (const [n, prof] of need) steps.set(tier(n), [...(steps.get(tier(n)) ?? []), [n, prof]]);
  return [...steps.entries()]
    .sort(([a], [b]) => a - b)
    .map(([, step]) => step.sort((a, b) => BY_ID.get(a[0])!.name.localeCompare(BY_ID.get(b[0])!.name)));
}

/** The base stats the whole way needs: "Strength 9 for Mining". */
function statWords(ids: string[]): string[] {
  const byStat = new Map<string, string[]>();
  for (const i of ids) {
    const s = BY_ID.get(i)!;
    if (s.stat) byStat.set(`${s.stat[0]} ${s.stat[1]}`, [...(byStat.get(`${s.stat[0]} ${s.stat[1]}`) ?? []), s.name]);
  }
  return [...byStat.entries()].map(([stat, names]) => `${stat} for ${andList(names)}`);
}

/**
 * What the mentor says about a skill, in as few words as tell it all:
 * the steps, earliest first, the base stats and the level.
 */
export function advice(id: string): string {
  const s = BY_ID.get(id);
  if (!s) return "";
  if (s.auto) return `${s.name} comes by itself at level ${s.level}.`;
  const steps = plan(id);
  const ids = [...ancestors(id).keys(), id];
  const level = Math.max(...ids.map((i) => BY_ID.get(i)!.level));
  const stats = statWords(ids);
  const statLine = stats.length > 0 ? ` Base ${stats.length === 1 ? "stat" : "stats"}: ${stats.join("; ")}.` : "";
  if (steps.length === 0) return `${s.name} needs no other skill: gain it at level ${level}.${statLine}`;
  const count = ids.length - 1;
  const said = steps.map((step, i) => `${i === 0 ? "First" : "Then"} ${stepWords(step)}.`).join(" ");
  return `${s.name}: ${count} ${count === 1 ? "skill" : "skills"} first. ${said} Then gain ${s.name}${level > 1 ? ` at level ${level}` : ""}.${statLine}`;
}

/** What the skill opens up, in words, for beside the picture. */
export function opensWords(id: string): string {
  const opens = opensUp(id).map((o) => o.name);
  const all = descendants(id).size;
  if (opens.length === 0) return "It opens up nothing further.";
  const more = all - opens.length;
  return `It opens up ${andList(opens)}${more > 0 ? `, and ${more} more after ${opens.length === 1 ? "it" : "them"}` : ""}.`;
}

/** The mentor's answer to whatever was asked. */
export function answer(asked: string): { skill?: ArtisanSkill; words: string } {
  if (asked.trim() === "") return { words: "Tell me which skill you want, and I'll tell you what you need first." };
  const found = find(asked);
  if (found.skill) return { skill: found.skill, words: advice(found.skill.id) };
  if (found.maybe.length === 0) return { words: `No Artisan skill is called "${asked.trim()}".` };
  const some = found.maybe.slice(0, 6).map((m) => m.name);
  const more = found.maybe.length - some.length;
  return { words: `Which one? ${andList(some)}${more > 0 ? `, or ${more} more` : ""}.` };
}

/** The needs of one skill, in short, for its name in the picture: "needs Fire Building 75%, Mining 75%". */
export function needsShort(id: string): string {
  const s = BY_ID.get(id);
  if (!s || s.needs.length === 0) return "needs nothing first";
  return `needs ${s.needs.map(([n, prof]) => wanted(n, prof).replace(" to ", " ")).join(", ")}`;
}

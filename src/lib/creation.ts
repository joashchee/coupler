/**
 * Making a character, in Coupler's own few words. CoffeeMUD asks its
 * questions after screens of text (src-tauri/src/creation.rs finds the
 * question, its choices and the text); here each kind of question gets
 * a short question and a line of help in plain words, the stage it
 * belongs to, and what Immersive's narrator says. The game's own text
 * is never lost: the guide (components/CreationDialog.tsx) shows it on
 * request, and the terminal has it all.
 *
 * The goal is Immersive's: fewer words. A new player hears "Race. Which
 * race will you be? 8 to choose from." and then each race as they move
 * to it, not the five paragraphs before the list.
 */
import type { CreationChoice, CreationKind, CreationStats, CreationStep } from "./mud";

/**
 * How a character is made: with Coupler's guide (a question at a time, in
 * few words) or CoffeeMUD's own way (its screens of text in the terminal,
 * answers typed). Kept as `coupler.creationMode`; the guide until chosen.
 */
export type CreationMode = "guide" | "standard";

const MODE_KEY = "coupler.creationMode";

export function loadMode(): CreationMode {
  try {
    return localStorage.getItem(MODE_KEY) === "standard" ? "standard" : "guide";
  } catch {
    return "guide";
  }
}

export function storeMode(mode: CreationMode) {
  try {
    localStorage.setItem(MODE_KEY, mode);
  } catch {
    // Not kept past this run, then.
  }
}

/** The stages of making a character, in order. Account and World only where the game asks them. */
export const STAGES = ["Account", "Name", "World", "Race", "Gender", "Stats", "Class", "Beliefs", "Begin"] as const;
export type Stage = (typeof STAGES)[number];

/** The stage a question belongs to; null for one the guide doesn't know (it keeps the last). */
export function stageOf(kind: CreationKind): Stage | null {
  switch (kind) {
    case "accountName":
    case "newAccount":
    case "accountPassword":
    case "email":
    case "emailAgain":
    case "accountMenu":
    case "colors":
      return "Account";
    case "loginName":
    case "newCharacter":
    case "characterName":
    case "confirmName":
    case "password":
      return "Name";
    case "theme":
      return "World";
    case "race":
    case "confirmRace":
      return "Race";
    case "gender":
      return "Gender";
    case "stats":
    case "reroll":
    case "statAmount":
      return "Stats";
    case "class":
    case "confirmClass":
      return "Class";
    case "faction":
    case "deity":
    case "confirmDeity":
      return "Beliefs";
    case "rules":
      return "Begin";
    default:
      return null;
  }
}

/** The stages to show: Account and World only once seen (the game may never ask them). */
export const shownStages = (seen: Set<Stage>): Stage[] => STAGES.filter((s) => (s !== "Account" && s !== "World") || seen.has(s));

/** "a Fighter", "an Elf". */
const article = (name: string) => `${/^[aeiou]/i.test(name) ? "an" : "a"} ${name}`;

/** The question in Coupler's words, and a line of help. */
export function words(step: CreationStep): { question: string; help: string } {
  const s = step.subject ?? "";
  switch (step.kind) {
    case "accountName":
      return { question: "What will your account be called?", help: "An account holds all your characters. Type a new name to make one, or the name of an account you have." };
    case "loginName":
      return { question: "What's your character's name?", help: "Type a new name to make a character, or the name of one you have." };
    case "newAccount":
      return { question: `Make a new account called ${s}?`, help: "There's no account by that name yet. Yes makes it; No asks for the name again." };
    case "newCharacter":
      return { question: `Make a new character called ${s}?`, help: "There's no character by that name yet. Yes makes one; No asks for the name again." };
    case "accountPassword":
      return { question: "Choose a password for the account.", help: "Coupler hides it as you type and never keeps it. The game asks for it each time you log in." };
    case "email":
      return { question: "What's your e-mail address?", help: "The game keeps it secret and uses it if you forget your password. It asks twice to be sure." };
    case "emailAgain":
      return { question: "Type the e-mail address again.", help: "The same address, to be sure it's right." };
    case "accountMenu":
      return { question: "Your account: what next?", help: "Make a new character, or type the name of one of yours to play them." };
    case "characterName":
      return { question: "What will your character be called?", help: "One word, letters only: the name everyone in the game will see. Or let the game pick a random name." };
    case "confirmName":
      return { question: `Call your character ${s}?`, help: "Yes goes on to make them; No goes back to the menu." };
    case "password":
      return { question: "Choose a password for your character.", help: "Coupler hides it as you type and never keeps it. The game asks for it each time you log in." };
    case "colors":
      return { question: "Show the game in color?", help: "Coupler shows the game's colors as they're sent. Yes is best." };
    case "theme":
      return { question: "Which kind of world?", help: "This game has more than one. Your choice decides the races and classes you can pick." };
    case "race":
      return { question: "Which race will you be?", help: "Races differ in size, senses and natural strengths. Pick one to read more; you'll be asked before it's settled." };
    case "confirmRace":
      return { question: `Be ${article(s)}?`, help: "The game's words about it are below. Yes settles it; No goes back to the races." };
    case "gender":
      return { question: "Which gender is your character?", help: "It's how the game will speak of you." };
    case "stats":
      return step.stats?.points === 0
        ? { question: "All your points are spent.", help: "Choose Done, or lower a stat to move its points elsewhere. Classes need certain stats: the list below says which you qualify for." }
        : { question: "Spend your points on your stats.", help: "Raise the stats your class will need. When none are left, choose Done. Or let the game roll them at random." };
    case "reroll":
      return { question: "Keep these stats?", help: "The game rolled them at random. Keep them, or roll again." };
    case "statAmount":
      return { question: "How many points?", help: "A number such as +2 to add, or -1 to take away." };
    case "class":
      return { question: "Which class will you be?", help: "Your class is your career: how you fight and what you learn. Those that suit your best stat say so. Not sure? Apprentice lets you pick later." };
    case "confirmClass":
      return { question: `Be ${article(s)}?`, help: "The game's words about it are below. Yes settles it; No goes back to the classes." };
    case "faction":
      if (step.subject === "Alignment") return { question: "Good, neutral or evil?", help: "Your alignment. You earn no experience for killing a creature of your own alignment, and some classes must keep theirs." };
      if (step.subject === "Inclination") return { question: "Lawful, moderate or chaotic?", help: "Your inclination: how much your character holds to order." };
      return { question: `Choose your ${step.subject?.toLowerCase() ?? "side"}.`, help: "The game's words about it are below." };
    case "deity":
      return { question: "Which deity will you serve?", help: "The god your character worships." };
    case "confirmDeity":
      return { question: `Serve ${s}?`, help: "Yes settles it; No goes back to the list." };
    case "rules":
      return { question: "Your character is made.", help: "Read the game's rules, then begin. In the game, LOOK shows where you are and HELP explains the rest." };
    case "yesNo":
      return { question: step.asked, help: "" };
    default:
      return { question: step.asked, help: "" };
  }
}

/**
 * What each alignment and inclination does to a character, in few words,
 * from CoffeeMUD's own factions (`resources/factions/alignment.ini`,
 * `inclination.ini`, and their intros in `resources/text/`): said as the
 * choice is reached, so the player knows what they're choosing.
 */
const FACTION_IMPACT: Record<string, Record<string, string>> = {
  alignment: {
    good: "No experience for killing good creatures. Holy prayers, not unholy ones. Paladins must stay good.",
    neutral: "No experience for killing neutral creatures. Both holy and unholy prayers, and druids' chants. Druids must stay neutral.",
    evil: "No experience for killing evil creatures. Unholy prayers, not holy ones. Evil paladins' dark powers.",
  },
  inclination: {
    lawful: "Order: obeying the law, building, keeping clean. It changes no experience.",
    moderate: "The balance between order and free will. It changes no experience.",
    chaotic: "Free will: crime, stealing and dirty fighting push you here. It changes no experience.",
  },
};

/** What a faction's choice does, or null for one Coupler has no words for. */
export function factionImpact(subject: string | null, choice: string): string | null {
  return FACTION_IMPACT[(subject ?? "").toLowerCase()]?.[choice.toLowerCase()] ?? null;
}

/** A choice as it's said and read: "Fighter, suits your best stat. Fighters are…". `extra`: Coupler's own words first (a race's facts, what an alignment does). */
export function choiceWords(choice: CreationChoice, extra?: string | null): string {
  const name = choice.suggested ? `${choice.name}, suits your best stat` : choice.name;
  const words = [extra, choice.about].filter(Boolean).join(" ");
  return words ? `${name}. ${words}` : `${name}.`;
}

/** "Fighter, Thief and Mage". */
function list(items: string[]): string {
  return items.length < 2 ? (items[0] ?? "") : `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/** The classes some stats qualify for, in words. */
export function qualifiesWords(stats: CreationStats): string {
  if (stats.qualifies.length === 0) return "These stats qualify for no class yet.";
  const suit = stats.qualifies.filter((c) => c.suggested).map((c) => c.name);
  return `Qualifies for ${list(stats.qualifies.map((c) => c.name))}.${suit.length > 0 ? ` Best suited: ${list(suit)}.` : ""}`;
}

/** Points left, in words. */
export const pointsWords = (points: number | null) => (points === null ? "" : points === 0 ? "No points left." : points === 1 ? "1 point left." : `${points} points left.`);

/**
 * What the narrator says for a question: the stage the first time it
 * comes, the question, a problem with the last answer, how many choices.
 * For the stats again (after a point spent), only what changed.
 */
export function spoken(step: CreationStep, before: CreationStep | null): string {
  const problem = step.problem ? `${step.problem} ` : "";
  if (before && step.stats && before.stats && step.kind === before.kind) {
    const changed = step.stats.stats
      .filter((s) => before.stats!.stats.find((b) => b.name === s.name)?.value !== s.value)
      .map((s) => `${s.name} ${s.value}.`);
    const points = step.stats.points !== before.stats.points ? pointsWords(step.stats.points) : "";
    const said = `${problem}${changed.join(" ")} ${points}`.trim();
    return said || (step.kind === "reroll" ? "Rolled again." : "");
  }
  const stage = stageOf(step.kind);
  const sameStage = before !== null && stageOf(before.kind) === stage;
  const { question } = words(step);
  const count = step.choices.length > 1 && step.kind !== "accountMenu" ? ` ${step.choices.length} to choose from.` : "";
  const stats = step.stats ? ` ${pointsWords(step.stats.points)}` : "";
  return `${problem}${stage && !sameStage ? `${stage}. ` : ""}${question}${count}${stats}`.trim();
}

/** Whether a step is a new question for the guide (focus moves, the cue plays), or the same one again (the stats after a point). */
export const isNewQuestion = (step: CreationStep, before: CreationStep | null) =>
  !before || before.kind !== step.kind || before.subject !== step.subject || (before.asked !== step.asked && !step.stats);

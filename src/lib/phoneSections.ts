/**
 * The phone screen's sections (lib/formFactor.ts, `PHONE`): one at a
 * time fills the middle of the screen, the bar above and the keys below
 * it, as a carousel. A swipe in from the right edge brings the next, one
 * in from the left the one before; past the last comes the first again.
 * The bar's ◄ and ► do the same, for VoiceOver (its own gestures take
 * the screen's swipes) and for anyone who'd rather tap.
 *
 * Each section brings its own keys (the row above the command line, the
 * phone's context-sensitive input) and its own options in the sheet
 * pulled down from the top (`PhoneSheet.tsx`), under the general ones.
 */
export type PhoneSection = "game" | "map" | "here" | "heard";

export const PHONE_SECTIONS: { id: PhoneSection; name: string; what: string }[] = [
  { id: "game", name: "Game", what: "the game's output" },
  { id: "map", name: "Map", what: "the map and finding your way" },
  { id: "here", name: "Here", what: "the room, its picture and your health" },
  { id: "heard", name: "Heard", what: "the sounds heard, the journal and the log" },
];

const SECTION_KEY = "coupler.phone.section";

/** The section shown last time, or the game. */
export function storedSection(): PhoneSection {
  try {
    const kept = localStorage.getItem(SECTION_KEY);
    if (PHONE_SECTIONS.some((s) => s.id === kept)) return kept as PhoneSection;
  } catch {
    // Not kept: the game, then.
  }
  return "game";
}

export function storeSection(section: PhoneSection) {
  try {
    localStorage.setItem(SECTION_KEY, section);
  } catch {
    // Not kept; the game comes first next time.
  }
}

/** The section `by` steps round from `from` (1 the next, -1 the one before), round the carousel. */
export function turnSection(from: PhoneSection, by: 1 | -1): PhoneSection {
  const at = PHONE_SECTIONS.findIndex((s) => s.id === from);
  const n = PHONE_SECTIONS.length;
  return PHONE_SECTIONS[(at + by + n) % n].id;
}

export const sectionName = (id: PhoneSection) => PHONE_SECTIONS.find((s) => s.id === id)?.name ?? "Game";

/** Where it is in the carousel, in words: "Map, 2 of 4: the map and finding your way." */
export function sectionInWords(id: PhoneSection): string {
  const at = PHONE_SECTIONS.findIndex((s) => s.id === id);
  const s = PHONE_SECTIONS[at];
  return `${s.name}, ${at + 1} of ${PHONE_SECTIONS.length}: ${s.what}.`;
}

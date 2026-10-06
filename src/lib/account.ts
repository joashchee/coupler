/**
 * The account menu's words (components/AccountMenuDialog.tsx): what the
 * narrator says when it opens, and each character as a button names it.
 * Pure. The menu itself is read by src-tauri/src/account.rs.
 */
import type { AccountCharacter, AccountMenu } from "./mud";

/** "level 12 Human Fighter", as much as the list shows. */
export function describeCharacter(c: AccountCharacter): string {
  const what = [c.race, c.class].filter((w) => w.trim() !== "").join(" ");
  const level = c.level.trim() !== "" ? `level ${c.level}` : "";
  return [level, what].filter(Boolean).join(" ");
}

/** A character's button, in full for a screen reader. */
export function characterLabel(c: AccountCharacter): string {
  const about = describeCharacter(c);
  const last = c.last && c.last !== "N/A" ? `, last played ${c.last}` : "";
  return `Play ${c.name}${about ? `, ${about}` : ""}${c.online ? ", playing now" : ""}${last}`;
}

/** The game's questions a dialog button answers yes to for the player, who already said yes in the dialog. */
export const CONFIRMS = {
  new: "create a new character called",
  delete: "retire and delete",
  quit: "quit -- are you sure",
} as const;
export type Confirm = keyof typeof CONFIRMS;

/** Whether the game's unfinished line is the y/N question `confirm` waits for. */
export const asksToConfirm = (confirm: Confirm, partial: string) => {
  const low = partial.trim().toLowerCase();
  return low.includes(CONFIRMS[confirm]) && /\(y\/n\)\??$/.test(low);
};

/** What the narrator says of the menu: in few words, the characters by name. */
export function spokenMenu(menu: AccountMenu): string {
  const list = menu.characters;
  if (list === null) return "Account menu. Reading your characters…";
  if (list.length === 0) return "Account menu. No characters yet: make a new one.";
  const names = list.map((c) => {
    const about = describeCharacter(c);
    return `${c.name}${about ? `, ${about}` : ""}${c.online ? ", playing now" : ""}`;
  });
  return `Account menu. ${list.length === 1 ? "Your character" : `Your ${list.length} characters`}: ${names.join("; ")}. Pick one to play, or make a new one.`;
}

/**
 * Coupler's own keyboard on the tablet (components/TabletKeyboard.tsx):
 * its keys, and what pressing one does. The system's keyboard never
 * shows there (lib/formFactor.ts), so this one types into whatever text
 * field has the focus, the command line or a dialog's.
 *
 * A key press is a keydown and a keyup sent to the field (or the page,
 * with no field), so every shortcut and the command line's own keys
 * (Return sends, Up and Down go through what was typed) work as they do
 * from a real keyboard. A browser never acts on a keydown it didn't make
 * itself, so what the key would have done is done here after, unless a
 * handler stopped it: the character typed, Backspace and Delete, the
 * caret moved, Tab to the next control. With Cmd, Ctrl or Option held a
 * key is only ever a shortcut.
 */
import { MAC } from "./keys";

export type Modifier = "shift" | "ctrl" | "alt" | "meta";

export interface SoftKey {
  /** KeyboardEvent.key unshifted ("a", "Enter", "F1"), or the modifier for a modifier key. */
  key: string;
  /** KeyboardEvent.code: the key's place ("KeyA", "Enter"). */
  code: string;
  /** What's written on it; a letter or symbol key shows `shifted` with Shift on. */
  label: string;
  /** Its key with Shift: "A", "!". */
  shifted?: string;
  /** What VoiceOver says, when it isn't the label: "Backspace". */
  name?: string;
  /** Cells wide (8 px each); every row is 96. */
  cells: number;
  modifier?: Modifier;
  /** Held down, it goes again and again: Backspace, Delete, the arrows, Space. */
  repeats?: boolean;
  /** Not a key at all: lowers the keyboard (the field loses its focus). */
  hide?: boolean;
}

const char = (key: string, shifted: string, code: string): SoftKey => ({ key, code, label: key, shifted, cells: 6 });
const letters = (row: string, codes = row.toUpperCase()) => [...row].map((c, i) => char(c, c.toUpperCase(), `Key${codes[i]}`));
const OPTION = MAC ? "Opt" : "Alt";
const COMMAND = MAC ? "Cmd" : "Win";

/**
 * Six rows of 96 cells, each key three rows tall (48 px): the function
 * keys, the numbers, three of letters, then the modifiers, Space and the
 * arrows. Ctrl sits where Caps Lock would, as on the old terminals.
 */
export const KEY_ROWS: SoftKey[][] = [
  [
    { key: "Escape", code: "Escape", label: "Esc", name: "Escape", cells: 6 },
    ...Array.from({ length: 12 }, (_, i): SoftKey => ({ key: `F${i + 1}`, code: `F${i + 1}`, label: `F${i + 1}`, cells: 6 })),
    { key: "Home", code: "Home", label: "Home", cells: 6 },
    { key: "End", code: "End", label: "End", cells: 6 },
    { key: "Delete", code: "Delete", label: "Del", name: "Delete", cells: 6, repeats: true },
  ],
  [
    char("`", "~", "Backquote"),
    ...[..."1234567890"].map((d, i) => char(d, "!@#$%^&*()"[i], `Digit${d}`)),
    char("-", "_", "Minus"),
    char("=", "+", "Equal"),
    { key: "Backspace", code: "Backspace", label: "Bksp", name: "Backspace", cells: 18, repeats: true },
  ],
  [
    { key: "Tab", code: "Tab", label: "Tab", cells: 12 },
    ...letters("qwertyuiop"),
    char("[", "{", "BracketLeft"),
    char("]", "}", "BracketRight"),
    { ...char("\\", "|", "Backslash"), cells: 12 },
  ],
  [
    { key: "ctrl", code: "ControlLeft", label: "Ctrl", name: "Control", cells: 12, modifier: "ctrl" },
    ...letters("asdfghjkl"),
    char(";", ":", "Semicolon"),
    char("'", '"', "Quote"),
    { key: "Enter", code: "Enter", label: "Return", cells: 18 },
  ],
  [
    { key: "shift", code: "ShiftLeft", label: "Shift", cells: 18, modifier: "shift" },
    ...letters("zxcvbnm"),
    char(",", "<", "Comma"),
    char(".", ">", "Period"),
    char("/", "?", "Slash"),
    { key: "hide", code: "", label: "Hide", name: "Hide the keyboard", cells: 6, hide: true },
    { key: "ArrowUp", code: "ArrowUp", label: "↑", name: "Up arrow", cells: 6, repeats: true },
    { key: "shift", code: "ShiftRight", label: "Shift", cells: 6, modifier: "shift" },
  ],
  [
    { key: "alt", code: "AltLeft", label: OPTION, name: MAC ? "Option" : "Alt", cells: 9, modifier: "alt" },
    { key: "meta", code: "MetaLeft", label: COMMAND, name: MAC ? "Command" : "Windows", cells: 9, modifier: "meta" },
    { key: " ", code: "Space", label: "Space", cells: 42, repeats: true },
    { key: "meta", code: "MetaRight", label: COMMAND, name: MAC ? "Command" : "Windows", cells: 9, modifier: "meta" },
    { key: "alt", code: "AltRight", label: OPTION, name: MAC ? "Option" : "Alt", cells: 9, modifier: "alt" },
    { key: "ArrowLeft", code: "ArrowLeft", label: "←", name: "Left arrow", cells: 6, repeats: true },
    { key: "ArrowDown", code: "ArrowDown", label: "↓", name: "Down arrow", cells: 6, repeats: true },
    { key: "ArrowRight", code: "ArrowRight", label: "→", name: "Right arrow", cells: 6, repeats: true },
  ],
];

export type Field = HTMLInputElement | HTMLTextAreaElement;

const TEXT_TYPES = new Set(["text", "search", "password", "email", "url", "tel", "number"]);

/** A text field that can be typed in now. */
export function isField(el: Element | null): el is Field {
  if (el instanceof HTMLTextAreaElement) return !el.disabled && !el.readOnly;
  return el instanceof HTMLInputElement && TEXT_TYPES.has(el.type) && !el.disabled && !el.readOnly;
}

export type Held = Record<Modifier, boolean>;
export const NONE_HELD: Held = { shift: false, ctrl: false, alt: false, meta: false };

/** What a field holds and where its caret is: start to end selected, or both the caret. */
export interface Text {
  value: string;
  start: number;
  end: number;
}

/**
 * What a key does to a field's text, or null when it does nothing to it
 * (a function key, Escape). Pure, for the edits; `multiline` for a
 * textarea, where Return is a new line.
 */
export function edit(text: Text, key: string, multiline: boolean): Text | null {
  const { value, start, end } = text;
  const put = (s: string): Text => ({ value: value.slice(0, start) + s + value.slice(end), start: start + s.length, end: start + s.length });
  if (key.length === 1) return put(key);
  if (key === "Enter") return multiline ? put("\n") : null;
  const at = (n: number): Text => ({ value, start: n, end: n });
  switch (key) {
    case "Backspace":
      if (start !== end) return put("");
      return start === 0 ? null : { value: value.slice(0, start - 1) + value.slice(end), start: start - 1, end: start - 1 };
    case "Delete":
      if (start !== end) return put("");
      return end >= value.length ? null : { value: value.slice(0, start) + value.slice(end + 1), start, end: start };
    case "ArrowLeft":
      return at(start !== end ? start : Math.max(0, start - 1));
    case "ArrowRight":
      return at(start !== end ? end : Math.min(value.length, end + 1));
    case "Home":
      return at(multiline ? value.lastIndexOf("\n", start - 1) + 1 : 0);
    case "End": {
      const next = multiline ? value.indexOf("\n", end) : -1;
      return at(next === -1 ? value.length : next);
    }
    default:
      return null;
  }
}

/** A field's text and caret; a field with no caret (a number field) is typed at its end. */
function textOf(field: Field): Text {
  try {
    const start = field.selectionStart ?? field.value.length;
    return { value: field.value, start, end: field.selectionEnd ?? start };
  } catch {
    return { value: field.value, start: field.value.length, end: field.value.length };
  }
}

/**
 * Puts the text in the field the way typing does: React's controlled
 * fields only see a value set through the element's own setter, then
 * an input event.
 */
function setText(field: Field, next: Text, key: string) {
  if (next.value !== field.value) {
    if (field.maxLength >= 0 && next.value.length > field.maxLength && next.value.length > field.value.length) return;
    Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value")?.set?.call(field, next.value);
    const inputType = key === "Backspace" ? "deleteContentBackward" : key === "Delete" ? "deleteContentForward" : key === "Enter" ? "insertLineBreak" : "insertText";
    field.dispatchEvent(new InputEvent("input", { bubbles: true, inputType, data: key.length === 1 ? key : null }));
  }
  try {
    field.setSelectionRange(next.start, next.end);
  } catch {
    // A number field has no caret to place.
  }
}

/** Everything Tab can reach, in order, inside the dialog on top if one is open. */
function tabStops(): HTMLElement[] {
  const dialogs = [...document.querySelectorAll<HTMLElement>(".dialog-overlay.open:not([inert]) [role='dialog']")];
  const root: ParentNode = dialogs[dialogs.length - 1] ?? document;
  const all = root.querySelectorAll<HTMLElement>("a[href], button, input, select, textarea, [tabindex]");
  return [...all].filter((el) => el.tabIndex >= 0 && !(el as HTMLButtonElement).disabled && !el.closest("[inert]") && el.getClientRects().length > 0 && !el.closest(".tablet-keyboard"));
}

/**
 * Presses `key` with what's `held`, into `field` (the field typed in
 * last, still on the page) or, with none, at whatever has the focus.
 */
export function press(key: SoftKey, held: Held, field: Field | null) {
  const typed = held.shift && key.shifted ? key.shifted : key.key;
  const target: HTMLElement = field ?? (document.activeElement instanceof HTMLElement ? document.activeElement : document.body);
  if (field && document.activeElement !== field) field.focus({ preventScroll: true });
  const init: KeyboardEventInit = { key: typed, code: key.code, bubbles: true, cancelable: true, shiftKey: held.shift, ctrlKey: held.ctrl, altKey: held.alt, metaKey: held.meta };
  const go = target.dispatchEvent(new KeyboardEvent("keydown", init));
  target.dispatchEvent(new KeyboardEvent("keyup", init));
  if (!go || held.ctrl || held.alt || held.meta) return;
  if (key.key === "Tab") {
    const stops = tabStops();
    const at = stops.indexOf(target);
    const next = stops[(at + (held.shift ? -1 : 1) + stops.length) % stops.length];
    next?.focus();
    return;
  }
  if (!field) return;
  const next = edit(textOf(field), typed, field instanceof HTMLTextAreaElement);
  if (next) setText(field, next, typed);
  else if (typed === "Enter") field.form?.requestSubmit();
}

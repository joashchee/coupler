/**
 * Taking turns with a screen reader. Coupler asks every few seconds
 * whether one is running (`screen_reader.rs`: VoiceOver, NVDA, JAWS,
 * Narrator, Orca). While one is, Coupler's voice gives way whenever the
 * player presses a key the screen reader reads (`voice.yieldTurn`): the
 * line being said pauses and the queue waits until the keys stop, so
 * the two never talk over each other. A say key's answer takes the turn
 * back at once. What Coupler says, the status bar doesn't also give the
 * screen reader (`App.tsx`).
 */
import * as mud from "./mud";
import * as voice from "./voice";

/** How often Coupler asks, ms. */
const EVERY = 5000;
/** How long Coupler's voice waits after the last key the screen reader reads, ms. */
export const TURN = 1500;

let on = false;
const listeners = new Set<(on: boolean) => void>();

/** Whether a screen reader was running when last asked. */
export const running = () => on;

export function onChange(f: (on: boolean) => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

async function ask() {
  const now = await mud.screenReaderRunning().catch(() => false);
  if (now === on) return;
  on = now;
  listeners.forEach((f) => f(on));
}

/**
 * Whether the screen reader reads this key: moving (Tab, arrows), a
 * character typed (echoed), Escape, Space on a control. Not the
 * modifiers alone, nor Return in the command line (the game answers,
 * and Coupler should say it).
 */
function readsKey(e: KeyboardEvent): boolean {
  if (["Shift", "Control", "Alt", "Meta", "CapsLock"].includes(e.key)) return false;
  if (e.key === "Enter" && e.target instanceof HTMLInputElement) return false;
  return true;
}

/** Starts asking, and taking turns. Returns the function that stops it. */
export function start(): () => void {
  void ask();
  const timer = window.setInterval(() => void ask(), EVERY);
  const onFocus = () => void ask();
  // Capturing, so the turn's given before a shortcut's own answer takes it back.
  const onKey = (e: KeyboardEvent) => {
    if (on && readsKey(e)) voice.yieldTurn(TURN);
  };
  window.addEventListener("focus", onFocus);
  document.addEventListener("keydown", onKey, true);
  return () => {
    window.clearInterval(timer);
    window.removeEventListener("focus", onFocus);
    document.removeEventListener("keydown", onKey, true);
  };
}

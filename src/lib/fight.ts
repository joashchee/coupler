/**
 * **The Fight panel's facts** (components/FightPanel.tsx): the bout from
 * Combat mode's start to a moment after its end, how it ended (the game's
 * own lines, `endingOf` in lib/fightTalk.ts), and each fighter's mood and
 * hurt as the painter draws them (src-tauri/src/face.rs `Moment`). The
 * words are pure; `useBout` keeps the bout.
 */
import { useEffect, useRef, useState } from "react";
import { endingOf, type Ending } from "./fightTalk";
import * as mud from "./mud";

/** How long the result stays up after the fight ends. */
export const RESULT_MS = 2000;

/** A fight from its first second to a moment after its end. */
export interface Bout {
  /** The latest of the fight; the last seen once it's over. */
  opponent: mud.Opponent;
  /** When it started (Date.now()). */
  started: number;
  /** How it ended and when, once over: the result shows for `RESULT_MS`. */
  ended: { at: number; ending: Ending } | null;
}

/** How a face looks out (face.rs `Mood`). */
export type Mood = "calm" | "angry" | "pain" | "afraid" | "asleep" | "dead";

/** A fighter as the painter draws them now. */
export interface FaceMoment {
  mood: Mood;
  /** 0 unhurt to 5 near death. */
  hurt: number;
  /** What's on them, by name: a halo for Sanctuary, poison's green (face.rs `effect`). */
  effects?: string[];
}

/** Health left, out of 100, in the painter's five steps of hurt. */
export function hurtStep(percent: number | null): number {
  if (percent === null || percent >= 90) return 0;
  if (percent >= 70) return 1;
  if (percent >= 50) return 2;
  if (percent >= 30) return 3;
  return percent >= 10 ? 4 : 5;
}

/** Each step of hurt in words, as the face shows it. */
const HURT_WORDS = ["unhurt", "scratched", "cut", "bruised and cut", "bleeding badly", "near death"];
export const hurtWords = (step: number) => HURT_WORDS[Math.max(0, Math.min(5, step))];

/** Low health turns a face to pain. */
const PAIN_BELOW = 25;

/** The foe now: angry, in pain when low; at the end, dead, afraid or still angry. */
export function foeMoment(bout: Bout): FaceMoment {
  const percent = bout.opponent.percent;
  const hurt = hurtStep(percent);
  switch (bout.ended?.ending) {
    case "killed":
      return { mood: "dead", hurt: 5 };
    case "fled":
      return { mood: "afraid", hurt };
    case undefined:
      return { mood: percent !== null && percent < PAIN_BELOW ? "pain" : "angry", hurt };
    default:
      return { mood: "angry", hurt };
  }
}

/** The player now, from their health out of 100 and what's on them. */
export function yourMoment(bout: Bout, percent: number | null, effects: string[] = []): FaceMoment {
  const hurt = hurtStep(percent);
  switch (bout.ended?.ending) {
    case "youDied":
      return { mood: "dead", hurt: 5 };
    case "youFled":
      return { mood: "afraid", hurt, effects };
    case undefined:
      return { mood: percent !== null && percent < PAIN_BELOW ? "pain" : "angry", hurt, effects };
    default:
      return { mood: "calm", hurt, effects };
  }
}

/**
 * The effects row's entries, soonest to end first: "Shield spell 3:20",
 * a cooldown "wait majesty ~1h 2m". Forever ones last.
 */
export function effectEntries(status: mud.Status | null, now: number): string[] {
  if (!status) return [];
  const effects = [...status.effects].sort((a, b) => (a.endsAt ?? Infinity) - (b.endsAt ?? Infinity));
  return [
    ...effects.map((e) => (e.endsAt === null ? e.name : `${e.name} ${mud.timeLeft(e.endsAt, now)}`)),
    ...status.cooldowns.map((c) => `wait ${c.name} ${c.about ? "~" : ""}${mud.timeLeft(c.endsAt, now)}`),
  ];
}

/** The result in a few words, once it's over: "KO" at a death. */
export function resultWords(bout: Bout): string | null {
  const name = bout.opponent.name ?? "Your foe";
  switch (bout.ended?.ending) {
    case undefined:
      return null;
    case "killed":
      return `KO! ${name} is dead.`;
    case "youDied":
      return "KO! You died.";
    case "fled":
      return `${name} fled.`;
    case "youFled":
      return "You fled.";
    default:
      return "The fight is over.";
  }
}

/** The fight's clock: "0:42". */
export function clock(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

/** How far apart, in words. */
export function rangeWords(range: number | null): string {
  if (range === null) return "";
  return range === 0 ? "toe to toe" : `${range} away`;
}

/**
 * The bout: it starts with Combat mode, follows the foe (a new one
 * keeps the clock), reads the game's lines for how it ended, and stays
 * `RESULT_MS` after the end. Gone at once on leaving the game.
 */
export function useBout(opponent: mud.Opponent | null, connected: boolean): Bout | null {
  const [bout, setBout] = useState<Bout | null>(null);
  const ending = useRef<Ending>(null);

  useEffect(() => {
    const listening = mud.onOutput((e) => {
      e.lines.forEach((line, i) => {
        const said = endingOf(line.map((s) => s.text).join(""));
        if (!said) return;
        if (e.kinds[i] === "combat") ending.current = said;
        // A death's line can come just after MSDP ended the fight: still the result's.
        if (said === "killed" || said === "youDied") setBout((was) => (was?.ended && was.ended.ending === null ? { ...was, ended: { ...was.ended, ending: said } } : was));
      });
    });
    return () => void listening.then((unlisten) => unlisten());
  }, []);

  useEffect(() => {
    if (!connected) {
      setBout(null);
      return;
    }
    if (opponent) {
      setBout((was) => {
        if (was && !was.ended) return { ...was, opponent };
        ending.current = null;
        return { opponent, started: Date.now(), ended: null };
      });
      return;
    }
    setBout((was) => (was && !was.ended ? { ...was, ended: { at: Date.now(), ending: ending.current } } : was));
  }, [opponent, connected]);

  // The result shows for a moment, then the panel closes.
  useEffect(() => {
    if (!bout?.ended) return;
    const timer = window.setTimeout(() => setBout((was) => (was === bout ? null : was)), Math.max(0, bout.ended.at + RESULT_MS - Date.now()));
    return () => window.clearTimeout(timer);
  }, [bout]);

  return bout;
}

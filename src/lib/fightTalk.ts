/**
 * **Fight talk**: in a fight, Immersive's narrator never leaves a
 * silence. Whenever nothing's being said for a moment, it fills it with
 * the freshest fact about the fight, in a few words ("Rat at 37 of 90,
 * falling fast.", "You at 62 percent.", "You hit hard."). The opponent's
 * exact hit points when the game sends them (MSDP, src-tauri/src/
 * combat.rs), else their percentage. Something that
 * matters (the fight starting or ending, a death, a big blow, health
 * running low, someone fleeing) cuts whatever's queued and is said at
 * once. The moment it's over, everything waiting to be said is dropped
 * and the narrator gives the fight in one line (`summary`): how it ended,
 * how long it took, and the player's health.
 *
 * Outside a fight it says nothing: the grand goal, fewer spoken words,
 * stands everywhere else. Used by lib/immersive.ts for the game and by
 * the tutorial (lib/tutorial.ts), so a practice fight sounds the same.
 * `insight` and `urgent` are pure, for the words.
 */
import * as mud from "./mud";
import * as voice from "./voice";

/** How long a silence is let be before the narrator fills it. */
const SILENCE_MS = 250;
/** How often the silence is checked: often enough that a silence is never much longer. */
const TICK_MS = 50;
/** A blow this share of the maximum or more, at once, is said at once. */
const BIG_HIT = 15;
/** Health under these is said at once, each the first time it's crossed. */
const LOW_MARKS = [30, 15];
/** The same words aren't said again within this long. */
const REPEAT_MS = 4000;

/** What the narrator knows of the fight. */
export interface FightState {
  opponent: mud.Opponent | null;
  /** The player's health, percent, or null. */
  mine: number | null;
  /** The newest line of the fight not yet said, if any. */
  line: string | null;
}

/** What was last said about it, to tell what's changed. */
export interface Said {
  theirs: number | null;
  mine: number | null;
  /** When `theirs` was last said, and what it was before (for the trend). */
  at: number;
}

const percent = (n: number) => `${Math.round(n)} percent`;
const nameOf = (o: mud.Opponent | null) => o?.name ?? "Your opponent";

/** The opponent's health as said: their exact hit points ("37 of 90"), else the percentage, or null. */
export function theirFigures(o: mud.Opponent): string | null {
  if (o.health !== null && o.healthMax !== null && o.healthMax > 0) return `${o.health} of ${o.healthMax}`;
  return o.percent !== null ? percent(o.percent) : null;
}

/** The opponent's health in words, with how fast it's going. */
function theirHealth(o: mud.Opponent, before: number | null): string | null {
  const figures = theirFigures(o);
  if (o.percent === null || figures === null) return null;
  const drop = before === null ? 0 : before - o.percent;
  const trend = drop >= 20 ? ", falling fast" : drop >= 5 ? ", falling" : drop <= -5 ? ", recovering" : "";
  return `${nameOf(o)} at ${figures}${trend}.`;
}

/**
 * The freshest fact to fill a silence with, and what's then been said,
 * or null when there's nothing new. In order: the opponent's health if
 * it moved, the player's if it moved, the newest line of the fight if
 * it's short, then (nothing new) by turns: both healths side by side,
 * the range, who it is.
 */
export function insight(state: FightState, said: Said, turn = 0): { text: string; said: Said } | null {
  const o = state.opponent;
  if (!o) return null;
  const now = Date.now();
  if (o.percent !== null && (said.theirs === null || Math.abs(said.theirs - o.percent) >= 5)) {
    const text = theirHealth(o, said.theirs);
    if (text) return { text, said: { ...said, theirs: o.percent, at: now } };
  }
  if (state.mine !== null && (said.mine === null || Math.abs(said.mine - state.mine) >= 5)) {
    return { text: `You at ${percent(state.mine)}.`, said: { ...said, mine: state.mine } };
  }
  if (state.line && state.line.length <= 90) return { text: state.line, said };
  const theirs = theirFigures(o);
  const parts = [theirs !== null ? `${nameOf(o)} ${theirs.replace(/ percent$/, "")}` : null, state.mine !== null ? `you ${Math.round(state.mine)}` : null].filter(Boolean);
  const both = parts.length > 0 ? `${parts.join(", ")}.` : null;
  const range = o.range === null ? null : o.range === 0 ? `Toe to toe with ${nameOf(o)}.` : `${nameOf(o)} at range ${o.range}.`;
  const choices = [both, range, `Still fighting ${nameOf(o)}.`].filter((t): t is string => t !== null);
  const text = choices[turn % choices.length];
  return { text: text.charAt(0).toUpperCase() + text.slice(1), said };
}

/** A line of the fight that matters enough to be said at once: a death, someone fleeing, a fight won or lost. */
export function urgent(line: string): boolean {
  return /\bis DEAD\b|\bYou are DEAD\b|\bflees?\b|\byou flee\b|\bYou have been KILLED\b|\bcorpse\b/i.test(line) && line.length <= 120;
}

/** How a fight ended, as its lines told it. */
export type Ending = "killed" | "fled" | "youFled" | "youDied" | null;

/** What a fight's line says of its end, if anything. */
export function endingOf(line: string): Ending {
  if (/You are DEAD|You have been KILLED/i.test(line)) return "youDied";
  if (/you flee/i.test(line)) return "youFled";
  if (/is DEAD|is slain/i.test(line)) return "killed";
  if (/flees/i.test(line)) return "fled";
  return null;
}

/**
 * A fight in one line, once it's over: "Rat killed. 12 seconds. You at
 * 64 percent, down 20." Its name, how it ended, how long, the player's
 * health and what the fight cost.
 */
export function summary(opponent: mud.Opponent, ending: Ending, seconds: number, mine: number | null, mineBefore: number | null): string {
  const name = nameOf(opponent);
  const how =
    ending === "killed" ? `${name} killed.`
    : ending === "fled" ? `${name} fled.`
    : ending === "youFled" ? `You fled from ${name}.`
    : ending === "youDied" ? `${name} killed you.`
    : opponent.percent !== null && opponent.percent <= 15 ? `${name} is down.`
    : `Fight with ${name} over.`;
  const took = seconds < 1 ? "" : ` ${Math.round(seconds)} ${Math.round(seconds) === 1 ? "second" : "seconds"}.`;
  let health = "";
  if (mine !== null && ending !== "youDied") {
    const lost = mineBefore === null ? 0 : Math.round(mineBefore - mine);
    health = ` You at ${percent(mine)}${lost >= 5 ? `, down ${lost}` : ""}.`;
  }
  return `${how}${took}${health}`;
}

/** The talk for one fight after another. Feed it the game; `stop` when leaving. */
export class FightTalk {
  private opponent: mud.Opponent | null = null;
  private mine: number | null = null;
  private line: string | null = null;
  private said: Said = { theirs: null, mine: null, at: 0 };
  private lowSaid = new Set<number>();
  private quietSince = Date.now();
  private last = { text: "", at: 0 };
  private timer: number | null = null;
  /** Turns through the fillers when nothing's new. */
  private turn = 0;
  /** When this fight started, the player's health then, and how its lines say it ended. */
  private startedAt = 0;
  private mineBefore: number | null = null;
  private ending: Ending = null;
  /** Whether it may speak (Immersive's voice is on). */
  on = true;

  /** `cuts`: whether what matters cuts off what's queued (the game), or waits its turn (the tutorial, whose lesson is being said). */
  constructor(private cuts = true) {}

  /** The opponent changed: a fight starts or ends, or its health moved. */
  setOpponent(o: mud.Opponent | null) {
    const before = this.opponent;
    this.opponent = o;
    if (o && !before) {
      this.said = { theirs: o.percent, mine: this.mine, at: Date.now() };
      this.lowSaid.clear();
      this.startedAt = Date.now();
      this.mineBefore = this.mine;
      this.ending = null;
      const figures = theirFigures(o);
      this.now(`Fighting ${nameOf(o)}${figures !== null && o.percent !== null && o.percent < 100 ? `, at ${figures}` : ""}.`);
      this.start();
    } else if (!o && before) {
      this.stopTicking();
      const text = summary(before, this.ending, (Date.now() - this.startedAt) / 1000, this.mine, this.mineBefore);
      // Over: nothing still waiting about it is worth hearing now.
      if (this.on && this.cuts) voice.hush();
      this.now(text);
      this.line = null;
    }
  }

  /** The player's vitals changed. */
  setVitals(v: mud.Vitals | null) {
    const mine = v ? mud.percentOf(v.hp, v.maxHp) : null;
    const before = this.mine;
    this.mine = mine;
    if (!this.opponent || mine === null) return;
    if (before !== null && before - mine >= BIG_HIT) {
      this.now(`Big hit! You at ${percent(mine)}.`);
      this.said = { ...this.said, mine };
      return;
    }
    const mark = LOW_MARKS.find((m) => mine < m && !this.lowSaid.has(m));
    if (mark !== undefined) {
      LOW_MARKS.filter((m) => m >= mark).forEach((m) => this.lowSaid.add(m));
      this.now(`Health low: ${percent(mine)}!`);
      this.said = { ...this.said, mine };
    }
  }

  /** A line of the game during the fight. */
  heard(line: string) {
    if (!this.opponent) return;
    const text = line.trim();
    if (text === "") return;
    this.ending = endingOf(text) ?? this.ending;
    if (urgent(text)) {
      this.now(text);
      this.line = null;
    } else this.line = text;
  }

  /** Stops talking about the fight: the game's over, or the player left. */
  stop() {
    this.stopTicking();
    this.opponent = null;
    this.line = null;
  }

  /** Said at once, cutting off what's queued. */
  private now(text: string) {
    if (!this.on) return;
    voice.speak(text, this.cuts);
    this.last = { text, at: Date.now() };
  }

  private start() {
    this.stopTicking();
    this.quietSince = Date.now();
    this.timer = window.setInterval(() => this.tick(), TICK_MS);
  }

  private stopTicking() {
    if (this.timer !== null) window.clearInterval(this.timer);
    this.timer = null;
  }

  /** Fills a silence that's gone on long enough. */
  private tick() {
    if (!this.opponent || !this.on) return;
    const now = Date.now();
    if (voice.talking()) {
      this.quietSince = now;
      return;
    }
    if (now - this.quietSince < SILENCE_MS) return;
    const next = insight({ opponent: this.opponent, mine: this.mine, line: this.line }, this.said, this.turn++);
    this.quietSince = now;
    if (!next) return;
    if (next.text === this.line) this.line = null;
    if (next.text === this.last.text && now - this.last.at < REPEAT_MS) return;
    this.said = next.said;
    this.last = { text: next.text, at: now };
    voice.speak(next.text);
  }
}

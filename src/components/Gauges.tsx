/**
 * Gauges: health, mana and movement as bars of characters, and the fight
 * (the opponent's health). For a sighted player at a glance; the numbers
 * beside each bar are the facts, in words (rule 10), and the bars are
 * hidden from assistive technology. The bar's color repeats its level
 * (half or more, a quarter or more, less) and is never the only cue.
 *
 * Used by Here (ScenePanel.tsx) and Workshop's You panel (VitalsPanel.tsx),
 * which on the small tablet screen (an iPhone in the web build) is
 * compact: two letters for each name and bars five characters wide, each
 * character a quarter step (█ ▓ ▒ ░), so as many steps as the long bar.
 */
import * as mud from "../lib/mud";

/** How wide a bar is, in characters. */
export const BAR = 20;

export function bar(percent: number | null): string {
  if (percent === null) return "░".repeat(BAR);
  const full = Math.max(0, Math.min(BAR, Math.round((percent * BAR) / 100)));
  return "█".repeat(full) + "░".repeat(BAR - full);
}

/** How wide a compact bar is (the small tablet screen's You), in characters. */
export const SHORT_BAR = 5;
/** A compact bar's cell from empty to full: each of its characters four steps, so five make twenty. */
const STEPS = [" ", "░", "▒", "▓", "█"];

export function shortBar(percent: number | null): string {
  if (percent === null) return " ".repeat(SHORT_BAR);
  const most = SHORT_BAR * (STEPS.length - 1);
  const filled = Math.max(0, Math.min(most, Math.round((percent * most) / 100)));
  return Array.from({ length: SHORT_BAR }, (_, i) => STEPS[Math.max(0, Math.min(STEPS.length - 1, filled - i * (STEPS.length - 1)))]).join("");
}

/** The level a bar's color repeats. */
function level(percent: number | null): string {
  if (percent === null) return "";
  return percent >= 50 ? " gauge-good" : percent >= 25 ? " gauge-warn" : " gauge-low";
}

/** `short`: the compact gauge, its name in two letters (said in full) and its bar five characters. */
export function Gauge({ name, short, now, most }: { name: string; short?: string; now: number | null; most: number | null }) {
  const percent = mud.percentOf(now, most);
  const words = now === null ? "not known" : most === null ? `${now}` : `${now} of ${most}`;
  return (
    <p className="scene-gauge">
      {short ? (
        <>
          <span className="scene-label" aria-hidden="true">{short.padEnd(3)}</span>
          <span className="sr-only">{name}</span>
        </>
      ) : (
        <span className="scene-label">{name.padEnd(8)}</span>
      )}
      <span className={`gauge-bar${level(percent)}`} aria-hidden="true">
        {short ? shortBar(percent) : bar(percent)}
      </span>
      <span>{` ${words}`}</span>
    </p>
  );
}

/** `compact`: HP, MN and MV with five-character bars, for the small tablet screen. */
export function Vitals({ vitals, compact = false }: { vitals: mud.Vitals | null; compact?: boolean }) {
  return (
    <>
      <Gauge name="Health" short={compact ? "HP" : undefined} now={vitals?.hp ?? null} most={vitals?.maxHp ?? null} />
      <Gauge name="Mana" short={compact ? "MN" : undefined} now={vitals?.mana ?? null} most={vitals?.maxMana ?? null} />
      <Gauge name="Moves" short={compact ? "MV" : undefined} now={vitals?.moves ?? null} most={vitals?.maxMoves ?? null} />
    </>
  );
}

export function Fight({ opponent, compact = false }: { opponent: mud.Opponent | null; compact?: boolean }) {
  if (!opponent) return <p className="map-muted">Not fighting.</p>;
  return (
    <p className="scene-gauge scene-fight">
      <span className="scene-label">{(opponent.name ?? "Someone").slice(0, 7).padEnd(8)}</span>
      <span className="gauge-bar gauge-foe" aria-hidden="true">
        {compact ? shortBar(opponent.percent) : bar(opponent.percent)}
      </span>
      <span className="sr-only">{mud.describeOpponent(opponent)}</span>
      <span aria-hidden="true">
        {opponent.health !== null && opponent.healthMax !== null ? ` ${opponent.health} of ${opponent.healthMax}` : opponent.percent !== null ? ` ${opponent.percent}%` : ""}
      </span>
    </p>
  );
}

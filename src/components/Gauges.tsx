/**
 * Gauges: health, mana and movement as bars of characters, and the fight
 * (the opponent's health). For a sighted player at a glance; the numbers
 * beside each bar are the facts, in words (rule 10), and the bars are
 * hidden from assistive technology. The bar's color repeats its level
 * (half or more, a quarter or more, less) and is never the only cue.
 *
 * Used by Here (ScenePanel.tsx) and Workshop's You panel (VitalsPanel.tsx).
 */
import * as mud from "../lib/mud";

/** How wide a bar is, in characters. */
export const BAR = 20;

export function bar(percent: number | null): string {
  if (percent === null) return "░".repeat(BAR);
  const full = Math.max(0, Math.min(BAR, Math.round((percent * BAR) / 100)));
  return "█".repeat(full) + "░".repeat(BAR - full);
}

/** The level a bar's color repeats. */
function level(percent: number | null): string {
  if (percent === null) return "";
  return percent >= 50 ? " gauge-good" : percent >= 25 ? " gauge-warn" : " gauge-low";
}

export function Gauge({ name, now, most }: { name: string; now: number | null; most: number | null }) {
  const percent = mud.percentOf(now, most);
  const words = now === null ? "not known" : most === null ? `${now}` : `${now} of ${most}`;
  return (
    <p className="scene-gauge">
      <span className="scene-label">{name.padEnd(8)}</span>
      <span className={`gauge-bar${level(percent)}`} aria-hidden="true">
        {bar(percent)}
      </span>
      <span>{` ${words}`}</span>
    </p>
  );
}

export function Vitals({ vitals }: { vitals: mud.Vitals | null }) {
  return (
    <>
      <Gauge name="Health" now={vitals?.hp ?? null} most={vitals?.maxHp ?? null} />
      <Gauge name="Mana" now={vitals?.mana ?? null} most={vitals?.maxMana ?? null} />
      <Gauge name="Moves" now={vitals?.moves ?? null} most={vitals?.maxMoves ?? null} />
    </>
  );
}

export function Fight({ opponent }: { opponent: mud.Opponent | null }) {
  if (!opponent) return <p className="map-muted">Not fighting.</p>;
  return (
    <p className="scene-gauge scene-fight">
      <span className="scene-label">{(opponent.name ?? "Someone").slice(0, 7).padEnd(8)}</span>
      <span className="gauge-bar gauge-foe" aria-hidden="true">
        {bar(opponent.percent)}
      </span>
      <span className="sr-only">{mud.describeOpponent(opponent)}</span>
      <span aria-hidden="true">
        {opponent.health !== null && opponent.healthMax !== null ? ` ${opponent.health} of ${opponent.healthMax}` : opponent.percent !== null ? ` ${opponent.percent}%` : ""}
      </span>
    </p>
  );
}

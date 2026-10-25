/**
 * You: Workshop's gauges (Gauges.tsx), the character's health, mana and
 * movement and the fight, while connected. Immersive has the same in Here.
 */
import type * as mud from "../lib/mud";
import { Fight, Vitals } from "./Gauges";

interface VitalsPanelProps {
  vitals: mud.Vitals | null;
  opponent: mud.Opponent | null;
  /** The small tablet screen's: HP, MN and MV, bars five characters wide (Gauges.tsx). */
  compact?: boolean;
}

export function VitalsPanel({ vitals, opponent, compact = false }: VitalsPanelProps) {
  return (
    <section className="vitals-panel panel" aria-labelledby="vitals-title" data-testid="vitals-panel">
      <h2 id="vitals-title">You</h2>
      <div className="scene-body">
        <Vitals vitals={vitals} compact={compact} />
        <Fight opponent={opponent} compact={compact} />
      </div>
    </section>
  );
}

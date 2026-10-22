/**
 * The phone's touch pad (lib/formFactor.ts): what's typed most, one tap
 * each. The six ways CoffeeMUD's rooms join, LOOK, and the last command
 * back on the command line (Up on a keyboard). Each button is six cells
 * by three rows, 48 by 48, the least a finger hits.
 *
 * An exit the map knows for this room is marked in words as well as by
 * its look (rule 10): VoiceOver says "North, an exit". One not known is
 * still sent: the map can be behind the game, and the game answers.
 */
import type { MapExit } from "../lib/mud";

const WAYS: { command: string; dir: string; label: string; name: string }[] = [
  { command: "north", dir: "N", label: "N", name: "North" },
  { command: "south", dir: "S", label: "S", name: "South" },
  { command: "east", dir: "E", label: "E", name: "East" },
  { command: "west", dir: "W", label: "W", name: "West" },
  { command: "up", dir: "U", label: "U", name: "Up" },
  { command: "down", dir: "D", label: "D", name: "Down" },
];

interface TouchPadProps {
  /** The room's exits, from the map. */
  exits: MapExit[];
  /** Offline, or a password asked: nothing here is sent. */
  disabled: boolean;
  onCommand: (line: string) => void;
  /** The last command back on the command line, to change or send. */
  onRecall: () => void;
}

export function TouchPad({ exits, disabled, onCommand, onRecall }: TouchPadProps) {
  const open = new Set(exits.map((e) => e.dir.toUpperCase()));
  return (
    <div className="touch-pad" role="toolbar" aria-label="Touch pad" data-testid="touch-pad">
      {WAYS.map((w) => {
        const exit = open.has(w.dir);
        return (
          <button
            key={w.dir}
            type="button"
            className={exit ? "exit" : undefined}
            data-testid={`pad-${w.command}`}
            aria-label={exit ? `${w.name}, an exit` : w.name}
            disabled={disabled}
            onClick={() => onCommand(w.command)}
          >
            {w.label}
          </button>
        );
      })}
      <button type="button" data-testid="pad-look" aria-label="Look" disabled={disabled} onClick={() => onCommand("look")}>
        Look
      </button>
      <button type="button" data-testid="pad-recall" aria-label="The last command, to send again" disabled={disabled} onClick={onRecall}>
        {"↑"}
      </button>
    </div>
  );
}

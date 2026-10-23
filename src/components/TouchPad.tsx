/**
 * The phone's touch pad (lib/formFactor.ts): what's typed most, one tap
 * each. The six ways CoffeeMUD's rooms join, then two keys of the
 * section's own (`tail`; the Game's: LOOK, and the last command back on
 * the command line, Up on a keyboard). Each button is six cells by three
 * rows, 48 by 48, the least a finger hits.
 *
 * An exit the map knows for this room is marked in words as well as by
 * its look (rule 10): VoiceOver says "North, an exit". One not known is
 * still sent: the map can be behind the game, and the game answers.
 */
import type { MapExit } from "../lib/mud";
import { PhoneKeys, type PhoneKey } from "./PhoneKeys";

const WAYS: { command: string; dir: string; name: string }[] = [
  { command: "north", dir: "N", name: "North" },
  { command: "south", dir: "S", name: "South" },
  { command: "east", dir: "E", name: "East" },
  { command: "west", dir: "W", name: "West" },
  { command: "up", dir: "U", name: "Up" },
  { command: "down", dir: "D", name: "Down" },
];

interface TouchPadProps {
  /** The room's exits, from the map. */
  exits: MapExit[];
  /** Offline, or a password asked: nothing here is sent. */
  disabled: boolean;
  onCommand: (line: string) => void;
  /** The two keys after the ways. */
  tail: PhoneKey[];
}

export function TouchPad({ exits, disabled, onCommand, tail }: TouchPadProps) {
  const open = new Set(exits.map((e) => e.dir.toUpperCase()));
  const ways: PhoneKey[] = WAYS.map((w) => {
    const exit = open.has(w.dir);
    return { id: w.command, label: w.dir, name: exit ? `${w.name}, an exit` : w.name, lit: exit, onPress: () => onCommand(w.command) };
  });
  return <PhoneKeys label="Touch pad" keys={[...ways, ...tail]} disabled={disabled} />;
}

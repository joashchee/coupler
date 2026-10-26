/**
 * Speech: which kinds of line the screen reader is given as they come
 * (lib/speech.ts). Every change is at once and kept. Coupler's own
 * voice, in every way to play, chooses for itself and isn't changed
 * here; while it speaks, the screen reader isn't given the game.
 */
import * as keys from "../lib/keys";
import { SPOKEN_KINDS, type Spoken } from "../lib/speech";
import { Dialog } from "./Dialog";

interface SpeechDialogProps {
  open: boolean;
  onClose: () => void;
  spoken: Spoken;
  /** Whether a screen reader is running now (lib/screenReader.ts). */
  screenReader: boolean;
  onSpoken: (spoken: Spoken) => void;
  /** Whether Coupler's narrator speaks before a character's in the game (off unless turned on). */
  pregameNarrator: boolean;
  onPregameNarrator: (on: boolean) => void;
}

export function SpeechDialog({ open, onClose, spoken, screenReader, onSpoken, pregameNarrator, onPregameNarrator }: SpeechDialogProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Speech"
      className="dialog-wide"
      actions={
        <button type="button" className="primary" onClick={onClose}>
          Done
        </button>
      }
    >
      <p>{keys.keys("What your screen reader reads as it comes in. Everything still shows in the game output, and Option+Up reads it back.")}</p>
      <ul className="check-list" data-testid="speech-kinds">
        {SPOKEN_KINDS.map((k) => (
          <li key={k.id}>
            <label className="check-row">
              <input
                type="checkbox"
                data-testid={`speech-${k.id}`}
                checked={spoken[k.id]}
                onChange={(e) => onSpoken({ ...spoken, [k.id]: e.currentTarget.checked })}
              />
              <span>{k.name}</span>
            </label>
          </li>
        ))}
      </ul>
      <ul className="check-list">
        <li>
          <label className="check-row">
            <input type="checkbox" data-testid="speech-pregame-narrator" checked={pregameNarrator} onChange={(e) => onPregameNarrator(e.currentTarget.checked)} />
            <span>Coupler's narrator before the game too: the first screen, logging in, making a character</span>
          </label>
        </li>
      </ul>
      <p className="about-section-desc">
        {pregameNarrator
          ? "Coupler's narrator speaks from the first screen. Your screen reader isn't given what it says."
          : "Until a character is in the game, Coupler's narrator is quiet and your screen reader is given everything. Coupler's narrator starts once you're in."}
      </p>
      <p className="about-section-desc" data-testid="speech-screen-reader">
        {screenReader
          ? "A screen reader is running. Coupler's own voice takes turns with it: when you press a key it reads, Coupler pauses and picks up once the keys stop, and what Coupler says isn't given to it twice."
          : "No screen reader is running. When one is, Coupler's own voice takes turns with it, so the two never talk at once."}
      </p>
      <p className="about-section-desc">A prompt that hasn't changed is never read again. Coupler's own voice (VOX at 0 in the Mixer gives the game back to the screen reader) reads only talk, the time of day, the commands' answers in short and what you ask for.</p>
    </Dialog>
  );
}

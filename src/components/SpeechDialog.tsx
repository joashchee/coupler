/**
 * Speech: which kinds of line the screen reader is given as they come
 * (lib/speech.ts). Every change is at once and kept. Coupler's own
 * voice (Immersive) chooses for itself and isn't changed here.
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
}

export function SpeechDialog({ open, onClose, spoken, screenReader, onSpoken }: SpeechDialogProps) {
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
      <p className="about-section-desc" data-testid="speech-screen-reader">
        {screenReader
          ? "A screen reader is running. Coupler's own voice takes turns with it: when you press a key it reads, Coupler pauses and picks up once the keys stop, and what Coupler says isn't given to it twice."
          : "No screen reader is running. When one is, Coupler's own voice takes turns with it, so the two never talk at once."}
      </p>
      <p className="about-section-desc">A prompt that hasn't changed is never read again. Coupler's own voice, in Immersive, reads only talk, the time of day, the commands' answers in short and what you ask for.</p>
    </Dialog>
  );
}

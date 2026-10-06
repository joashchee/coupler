/**
 * Display: the game's colors, big text in Terminal and the sound
 * captions (lib/display.ts). Every change is at once and kept.
 */
import * as keys from "../lib/keys";
import { GAME_COLORS, TEXT_SIZES, type Display } from "../lib/display";
import { Dialog } from "./Dialog";

interface DisplayDialogProps {
  open: boolean;
  onClose: () => void;
  display: Display;
  onDisplay: (display: Display) => void;
}

export function DisplayDialog({ open, onClose, display, onDisplay }: DisplayDialogProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Display"
      className="dialog-wide"
      actions={
        <button type="button" className="primary" onClick={onClose}>
          Done
        </button>
      }
    >
      <fieldset className="radio-group" data-testid="display-colors">
        <legend>The game's colors</legend>
        {GAME_COLORS.map((c) => (
          <label key={c.id} className="check-row">
            <input type="radio" name="display-colors" data-testid={`display-colors-${c.id}`} checked={display.colors === c.id} onChange={() => onDisplay({ ...display, colors: c.id })} />
            <span>
              {c.name}: {c.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <fieldset className="radio-group" data-testid="display-sizes">
        <legend>{keys.keys("Text size in Terminal (Ctrl+Cmd+1)")}</legend>
        {TEXT_SIZES.map((t) => (
          <label key={t.id} className="check-row">
            <input type="radio" name="display-size" data-testid={`display-size-${t.id}`} checked={display.textSize === t.id} onChange={() => onDisplay({ ...display, textSize: t.id })} />
            <span>
              {t.name}: {t.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <label className="check-row">
        <input type="checkbox" data-testid="display-captions" checked={display.captions} onChange={(e) => onDisplay({ ...display, captions: e.currentTarget.checked })} />
        <span>Sound captions: every sound Coupler plays is written in the Heard panel, so it can be seen as well as heard.</span>
      </label>
      <label className="check-row">
        <input type="checkbox" data-testid="display-blink" checked={display.blink} onChange={(e) => onDisplay({ ...display, blink: e.currentTarget.checked })} />
        <span>The map's @ blinks, so you find where you are at a glance. Never with reduced motion on.</span>
      </label>
      <p className="about-section-desc">{keys.keys("Full screen (Ctrl+Cmd+F) makes everything bigger in every way to play.")}</p>
    </Dialog>
  );
}

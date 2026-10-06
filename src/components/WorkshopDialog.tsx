/**
 * Customize the Screen: Workshop mode's controls for every part of the
 * screen. Arranging (moving and resizing, components/Movable.tsx), what
 * shows (lib/ux.ts, `SHOWABLE`, and the map), Immersive's own sound
 * layers (and Cues, CuesDialog.tsx, for each cue), and going back to
 * the default layout. Every change is at once and kept.
 */
import * as keys from "../lib/keys";
import { SHOWABLE, type Layers } from "../lib/ux";
import { Dialog } from "./Dialog";

interface WorkshopDialogProps {
  open: boolean;
  onClose: () => void;
  arranging: boolean;
  onArranging: () => void;
  shown: Record<string, boolean>;
  onShown: (id: string, on: boolean) => void;
  mapOpen: boolean;
  onMap: () => void;
  layers: Layers;
  onLayers: (layers: Layers) => void;
  onReset: () => void;
  /** Opens Cues over this dialog. */
  onCues: () => void;
  /** Cues is open over it. */
  covered?: boolean;
}

export function WorkshopDialog({ open, onClose, arranging, onArranging, shown, onShown, mapOpen, onMap, layers, onLayers, onReset, onCues, covered = false }: WorkshopDialogProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Customize the Screen"
      className="dialog-wide"
      covered={covered}
      actions={
        <>
          <button type="button" data-testid="workshop-reset" onClick={onReset}>
            Reset to Default Layout…
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <p>Everything on Workshop's screen is yours to place. Changes happen at once and are kept.</p>
      <label className="check-row">
        <input type="checkbox" data-testid="workshop-arrange" checked={arranging} onChange={onArranging} />
        <span>Move and resize: drag a thing's top left corner to move it and its bottom right corner to resize it, or Tab to a corner and use the arrow keys.</span>
      </label>

      <h3 className="about-section-title">Show on the screen</h3>
      <ul className="check-list" data-testid="workshop-shown">
        <li>
          <label className="check-row">
            <input type="checkbox" checked={mapOpen} onChange={onMap} />
            <span>{keys.keys("The map (Cmd+Shift+M)")}</span>
          </label>
        </li>
        {SHOWABLE.map((s) => (
          <li key={s.id}>
            <label className="check-row">
              <input type="checkbox" data-testid={`workshop-show-${s.id}`} checked={shown[s.id]} onChange={(e) => onShown(s.id, e.currentTarget.checked)} />
              <span>{s.name}</span>
            </label>
          </li>
        ))}
      </ul>
      <p className="about-section-desc">The game output, the command line, the message bar, the gear and the ways to play always show.</p>

      <h3 className="about-section-title">Immersive's sound layers</h3>
      <label className="check-row">
        <input type="checkbox" data-testid="workshop-cues" checked={layers.cues} onChange={(e) => onLayers({ ...layers, cues: e.currentTarget.checked })} />
        <span>Sound cues: the exits as notes, footsteps, a heartbeat when hurt, the fight starting and ending.</span>
      </label>
      <label className="check-row">
        <input type="checkbox" data-testid="workshop-voice" checked={layers.voice} onChange={(e) => onLayers({ ...layers, voice: e.currentTarget.checked })} />
        <span>Coupler's voice: tells and says, the login, and the say keys. Leave it off with a screen reader, which already reads the game.</span>
      </label>
      <p>
        <button type="button" data-testid="workshop-cues-edit" onClick={onCues}>
          Customize the Cues…
        </button>
      </p>
    </Dialog>
  );
}

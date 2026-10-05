/**
 * Pictures: who paints the room's picture and in which style
 * (lib/pictures.ts). Every change is at once and kept. Only Coupler's
 * painter is here for now; the models a player brings will be listed
 * here too, each with where to get it and its license.
 */
import { PICTURE_ENGINES, PICTURE_STYLES, PICTURE_WHENS, PORTRAIT_SOURCES, type PictureSettings } from "../lib/pictures";
import { Dialog } from "./Dialog";

interface PicturesDialogProps {
  open: boolean;
  onClose: () => void;
  settings: PictureSettings;
  onSettings: (settings: PictureSettings) => void;
  /** Every room back to the look it had before Cmd+Shift+P. */
  onForgetLooks: () => void;
}

export function PicturesDialog({ open, onClose, settings, onSettings, onForgetLooks }: PicturesDialogProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Pictures"
      className="dialog-wide"
      actions={
        <>
          <button type="button" data-testid="picture-forget-looks" onClick={onForgetLooks}>
            First Looks Back
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <p>
        A picture of the room you're in, in Immersive's Here panel and Workshop's Picture panel. It's made on this computer, never fetched, and never
        holds back a sound or a voice. It shows nothing the room's own description doesn't say, so screen readers skip it.
      </p>
      <fieldset className="radio-group" data-testid="picture-engines">
        <legend>Painted by</legend>
        {PICTURE_ENGINES.map((e) => (
          <label key={e.id} className="check-row">
            <input
              type="radio"
              name="picture-engine"
              data-testid={`picture-engine-${e.id}`}
              checked={settings.engine === e.id}
              onChange={() => onSettings({ ...settings, engine: e.id })}
            />
            <span>
              {e.name}: {e.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <fieldset className="radio-group" data-testid="picture-styles" disabled={settings.engine === "none"}>
        <legend>Style</legend>
        {PICTURE_STYLES.map((s) => (
          <label key={s.id} className="check-row">
            <input
              type="radio"
              name="picture-style"
              data-testid={`picture-style-${s.id}`}
              checked={settings.style === s.id}
              onChange={() => onSettings({ ...settings, style: s.id })}
            />
            <span>
              {s.name}: {s.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <fieldset className="radio-group" data-testid="picture-whens" disabled={settings.engine === "none"}>
        <legend>When</legend>
        {PICTURE_WHENS.map((w) => (
          <label key={w.id} className="check-row">
            <input type="radio" name="picture-when" data-testid={`picture-when-${w.id}`} checked={settings.when === w.id} onChange={() => onSettings({ ...settings, when: w.id })} />
            <span>
              {w.name}: {w.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <fieldset className="radio-group" data-testid="portrait-sources">
        <legend>Race and class portraits, while making a character</legend>
        {PORTRAIT_SOURCES.map((p) => (
          <label key={p.id} className="check-row">
            <input
              type="radio"
              name="portrait-source"
              data-testid={`portrait-source-${p.id}`}
              checked={settings.portraits === p.id}
              onChange={() => onSettings({ ...settings, portraits: p.id })}
            />
            <span>
              {p.name}: {p.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <p className="about-section-desc">
        Cmd+Shift+P paints the room you're in again, in a new look it keeps. First Looks Back returns every room to its first look. A picture you set for a
        room in the hooks list shows in place of the painting.
      </p>
    </Dialog>
  );
}

/**
 * Pictures: who paints the room's picture and in which style
 * (lib/pictures.ts). Every change is at once and kept. Only Coupler's
 * painter is here for now; the models a player brings will be listed
 * here too, each with where to get it and its license.
 */
import * as keys from "../lib/keys";
import { indexColor } from "../lib/mud";
import { PALETTE_WORDS, usePalette, type Palette } from "../lib/palette";
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

/** Stripes of equal width, each a hard-edged band of its color. */
const stripes = (colors: string[]) =>
  `linear-gradient(to right, ${colors.map((c, i) => `${c} ${(i / colors.length) * 100}% ${((i + 1) / colors.length) * 100}%`).join(", ")})`;

/** The 256's cube color nearest a hue at full strength. */
function cubeHue(hue: number): string {
  const f = (n: number) => {
    const k = (n + hue / 60) % 6;
    return 1 - Math.max(0, Math.min(1, Math.min(k, 4 - k)));
  };
  const [r, g, b] = [f(5), f(3), f(1)].map((v) => Math.round(v * 5));
  return indexColor(16 + r * 36 + g * 6 + b);
}

const ACROSS = 32;
const steps = Array.from({ length: ACROSS }, (_, i) => i);

/**
 * Each palette's two rows, colors above and grays below: the 16 as eight
 * blocks over their bright eight; the 256 in its cube's steps and its
 * 24 grays; true color smooth.
 */
const SWATCHES: Record<Palette, [string, string]> = {
  "16": [stripes([0, 1, 2, 3, 4, 5, 6, 7].map(indexColor)), stripes([8, 9, 10, 11, 12, 13, 14, 15].map(indexColor))],
  "256": [stripes(steps.map((i) => cubeHue((i * 360) / ACROSS))), stripes(steps.map((i) => indexColor(232 + Math.floor((i * 24) / ACROSS))))],
  true: [
    `linear-gradient(to right, ${[0, 60, 120, 180, 240, 300, 360].map((h) => `hsl(${h}, 100%, 50%)`).join(", ")})`,
    "linear-gradient(to right, #000000, #ffffff)",
  ],
};

/** What the game is told to give each, and what the badge says. */
const PALETTE_NAMES: Record<Palette, string> = { "16": "ANSI 16", "256": "ANSI 256", true: "ANSI TRUE" };

/** The game's colors now: its name and a strip of them. Hidden from screen readers: the paragraph under it says the same in words. */
function PaletteBadge({ palette }: { palette: Palette }) {
  const [above, below] = SWATCHES[palette];
  return (
    <div className="palette-badge" data-testid="picture-palette-badge" data-palette={palette} aria-hidden="true">
      <span className="palette-badge-name">
        {PALETTE_NAMES[palette]}
      </span>
      <span className="palette-badge-swatch" style={{ backgroundImage: `${above}, ${below}` }} />
    </div>
  );
}

export function PicturesDialog({ open, onClose, settings, onSettings, onForgetLooks }: PicturesDialogProps) {
  const palette = usePalette();
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
      <PaletteBadge palette={palette} />
      <p data-testid="picture-palette">
        Colors: pictures are painted in {PALETTE_WORDS[palette]}, as many as you have the game send you.
        {palette === "16" ? " Type ANSI 256 or ANSI TRUE in the game for more, and smoother skies and shadows." : " Type ANSI 16 in the game for the 16 alone."}
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
      <label className="check-row">
        <input type="checkbox" data-testid="picture-fight" checked={settings.fight} onChange={(e) => onSettings({ ...settings, fight: e.currentTarget.checked })} />
        <span>{keys.keys("The Fight: in a fight, the two of you face off over the room, with health, mana and moves (Cmd+Shift+F)")}</span>
      </label>
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
        {keys.keys("Cmd+Shift+P")} paints the room you're in again, in a new look it keeps. First Looks Back returns every room to its first look. A picture you set for a
        room in the hooks list shows in place of the painting.
      </p>
    </Dialog>
  );
}

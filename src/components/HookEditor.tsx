/**
 * The hook editor: everything one pair of the hooks list can set off,
 * opened from its row's Edit button (HooksDialog.tsx), over the list.
 *
 * A row a kind: its name in words with its short name (SFX, BGM, ART,
 * BGN, BGW), the asset (None, or a file from the Assets folder by its
 * name), then what goes with it once one is chosen: a volume (percent
 * of the Mixer's), whether music loops, the picture's column and row
 * and how many seconds it stays (0: until it's clicked away).
 * Background noise is offered only on a room's pairs and weather sound
 * only on coupler.weather (src-tauri/src/ambient.rs), with a line
 * saying so elsewhere, so nothing offered does nothing. Every control
 * has a visible label. A change is saved at once and said in the status
 * bar; Set Off Nothing clears them all.
 */
import { useId, type ReactNode } from "react";
import * as mud from "../lib/mud";
import { Dialog } from "./Dialog";

/** An asset as the menus list it. */
export interface Choice {
  path: string;
  shown: string;
}

interface HookEditorProps {
  /** The pair being edited, with what it sets off now; null when closed. */
  pair: mud.HookPair | null;
  /** Each kind's assets; BGN and BGW take the SFX list. */
  choices: (kind: "sfx" | "bgm" | "art") => Choice[];
  /** Saves a change to the pair's trigger. */
  onChange: (change: Partial<mud.Trigger>) => Promise<void>;
  onClose: () => void;
}

/** The screen, in cells: where a picture's corner can go. */
const COLUMNS = 160;
const ROWS = 45;

/** A number typed in a field, kept in its range; the old one if it isn't a number. */
function within(text: string, least: number, most: number, was: number): number {
  const n = Number.parseInt(text, 10);
  return Number.isFinite(n) ? Math.min(Math.max(n, least), most) : was;
}

type Slot = "sfx" | "bgm" | "art" | "bgn" | "bgw";
type NumberField = "sfxVolume" | "bgmVolume" | "bgnVolume" | "bgwVolume" | "artX" | "artY" | "artFade";

export function HookEditor({ pair, choices, onChange, onClose }: HookEditorProps) {
  const id = useId();
  const t = pair?.trigger ?? mud.NO_TRIGGER;
  const roomPair = pair !== null && mud.BGN_KEYS.includes(pair.key);
  const weatherPair = pair?.key === mud.BGW_KEY;

  function assetMenu(slot: Slot, label: string) {
    const current = t[slot] ?? "";
    const list = choices(slot === "bgn" || slot === "bgw" ? "sfx" : slot);
    return (
      <select id={`${id}-${slot}`} className="hook-asset" data-testid={`hook-edit-${slot}`} value={current} onChange={(e) => void onChange({ [slot]: e.currentTarget.value || null })} aria-label={label}>
        <option value="">None</option>
        {/* One chosen before its file left the folder still shows. */}
        {current && !list.some((a) => a.path === current) && <option value={current}>{mud.assetTitle(current)} (missing)</option>}
        {list.map((a) => (
          <option key={a.path} value={a.path}>
            {a.shown}
          </option>
        ))}
      </select>
    );
  }

  /** A labelled number field: a volume, the picture's column or row, or its seconds; `unit` shown after it, `unitSaid` its word. */
  function numberField(field: NumberField, label: string, spoken: string, least: number, most: number, unit = "", unitSaid = "percent") {
    return (
      <span className="hook-edit-field">
        <label htmlFor={`${id}-${field}`}>{label}</label>
        <input
          id={`${id}-${field}`}
          // A new value from the database resets what's typed.
          key={`${field}-${t[field]}`}
          type="text"
          inputMode="numeric"
          className="hook-place"
          data-testid={`hook-edit-${field}`}
          aria-label={`${spoken}, ${least} to ${most}${unit ? ` ${unitSaid}` : ""}`}
          defaultValue={String(t[field])}
          maxLength={3}
          onBlur={(e) => {
            const n = within(e.currentTarget.value, least, most, t[field]);
            if (n !== t[field]) void onChange({ [field]: n });
            else e.currentTarget.value = String(n);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
        />
        {unit && <span aria-hidden="true">{unit}</span>}
      </span>
    );
  }

  /** One kind's row: its name, its asset, and what goes with it once one is chosen. */
  function row(slot: Slot, short: string, long: string, extras: ReactNode) {
    return (
      <div className="hook-edit-row" key={slot}>
        <label htmlFor={`${id}-${slot}`}>
          {short} {long}
        </label>
        {assetMenu(slot, `${long} (${short})`)}
        <span className="hook-edit-extras">{t[slot] ? extras : null}</span>
      </div>
    );
  }

  return (
    <Dialog
      open={pair !== null}
      onClose={onClose}
      title="Edit Hook"
      className="dialog-wide dialog-hook-edit"
      actions={
        <>
          <button type="button" data-testid="hook-edit-clear" disabled={!(t.sfx || t.bgm || t.art || t.bgn || t.bgw)} onClick={() => void onChange({ ...mud.NO_TRIGGER })}>
            Set Off Nothing
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <p className="hook-edit-pair">
        <span className="hook-key">{pair?.key}</span> <span className="hook-value">{pair?.value}</span>
      </p>
      <p>What this sets off each time the game sends it. Each change is saved at once; volumes are percent of the Mixer's. A picture's fade is the seconds it stays: 0 keeps it until it's clicked away or replaced.</p>
      <div className="hook-edit-rows">
        {row("sfx", "SFX", "Sound effect", numberField("sfxVolume", "Volume", "Sound effect volume", 0, 100, "%"))}
        {row(
          "bgm",
          "BGM",
          "Music",
          <>
            {numberField("bgmVolume", "Volume", "Music volume", 0, 100, "%")}
            <label className="hook-loop">
              <input type="checkbox" data-testid="hook-edit-loop" checked={t.bgmLoop} onChange={(e) => void onChange({ bgmLoop: e.currentTarget.checked })} />
              <span>Loop</span>
            </label>
          </>,
        )}
        {row(
          "art",
          "ART",
          "Picture",
          <>
            {numberField("artX", "Column", "Picture's column", 1, COLUMNS)}
            {numberField("artY", "Row", "Picture's row", 1, ROWS)}
            {numberField("artFade", "Fade", "Seconds the picture stays before it fades (0 keeps it until clicked)", 0, 999, "s", "seconds")}
          </>,
        )}
        {roomPair && row("bgn", "BGN", "Background noise", numberField("bgnVolume", "Volume", "Background noise volume", 0, 100, "%"))}
        {weatherPair && row("bgw", "BGW", "Weather sound", numberField("bgwVolume", "Volume", "Weather sound volume", 0, 100, "%"))}
      </div>
      {!roomPair && !weatherPair && (
        <p>Background noise (BGN) is set on a room's room.info.id, room.info.terrain or coupler.room.type, and weather sound (BGW) on coupler.weather.</p>
      )}
      {roomPair && <p>Background noise loops while you're here: a room's own first, then its terrain's, then indoors or outdoors.</p>}
      {weatherPair && <p>Weather sound loops while this is the weather where you are, outdoors.</p>}
    </Dialog>
  );
}

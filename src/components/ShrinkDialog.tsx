import { Dialog } from "./Dialog";
import * as mud from "../lib/mud";

interface ShrinkDialogProps {
  /** WAVs just added that could be smaller, or null when there's no question. */
  offers: mud.Added[] | null;
  onKeep: () => void;
  onShrink: () => void;
}

/**
 * The question after an import brings in WAVs: each could be stored as
 * Opus or WavPack, whichever is smallest (src-tauri/src/assets.rs,
 * `try_smaller`). Says what each would save and what the kinds mean;
 * Escape or Keep leaves them as WAVs.
 */
export function ShrinkDialog({ offers, onKeep, onShrink }: ShrinkDialogProps) {
  const list = offers ?? [];
  const before = list.reduce((sum, a) => sum + (a.smaller?.before ?? 0), 0);
  const after = list.reduce((sum, a) => sum + (a.smaller?.after ?? 0), 0);
  const kinds = new Set(list.map((a) => a.smaller?.to));
  const one = list.length === 1;
  return (
    <Dialog
      open={offers !== null}
      onClose={onKeep}
      title={one ? "Make this sound smaller?" : "Make these sounds smaller?"}
      actions={
        <>
          <button type="button" data-testid="shrink-keep" onClick={onKeep}>
            {one ? "Keep the WAV" : "Keep the WAVs"}
          </button>
          <button type="button" className="primary" data-testid="shrink-confirm" onClick={onShrink}>
            {one ? "Make It Smaller" : "Make Them Smaller"}
          </button>
        </>
      }
    >
      <p>
        {one ? "The WAV you added" : `The ${list.length} WAVs you added`} can be stored in {mud.sizeInWords(after)} instead of {mud.sizeInWords(before)}, saving{" "}
        {mud.sizeInWords(before - after)}:
      </p>
      <ul className="shrink-list" data-testid="shrink-list">
        {list.map((a) =>
          a.smaller ? (
            <li key={a.path}>
              {a.name}: {mud.sizeInWords(a.smaller.before)} down to {mud.sizeInWords(a.smaller.after)} as {mud.SMALLER_NAMES[a.smaller.to]}, saving{" "}
              {mud.sizeInWords(a.smaller.before - a.smaller.after)}.
            </li>
          ) : null,
        )}
      </ul>
      {kinds.has("opus") && <p>Opus is the smallest there is. It leaves out detail too fine to hear, so it sounds the same.</p>}
      {kinds.has("wv") && <p>WavPack keeps every detail: it's exactly the same sound, packed tighter.</p>}
      <p>Coupler's copy of the WAV is then deleted. The files you added from aren't touched.</p>
    </Dialog>
  );
}

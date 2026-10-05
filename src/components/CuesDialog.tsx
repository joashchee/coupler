/**
 * Cues (Workshop, gear menu, and Customize the Screen's sound layers):
 * every cue Coupler makes (lib/earcons.ts, `CUES`), in its groups, each
 * row its name, its group, how it is now in short, and Edit, which opens
 * the cue's editor over the list.
 *
 * The editor: the sound on or off, its volume (a share of the Mixer's
 * CUE) and pitch as sliders (letting go plays it), and the caption, the
 * cue's visual half (written in Heard with sound captions on, Display),
 * on or off and in the player's own words. Play It plays it with
 * made-up details; Back to Coupler's Own undoes every change. Every
 * change is at once and kept.
 */
import { useEffect, useId, useState } from "react";
import * as earcons from "../lib/earcons";
import { Dialog } from "./Dialog";

interface CuesDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
}

/** A cue in short: "Sound 80%, pitch 120%, caption 'Exits'", or "Off". */
export function cueSummary(id: earcons.CueId): string {
  const s = earcons.cueSetting(id);
  const sound = s.sound && s.volume > 0 ? [`sound ${s.volume}%`, s.pitch !== 100 && `pitch ${s.pitch}%`].filter(Boolean).join(", ") : "no sound";
  const caption = s.caption ? `caption "${s.words}"` : "no caption";
  const text = `${sound}, ${caption}`;
  return text.charAt(0).toUpperCase() + text.slice(1);
}

export function CuesDialog({ open, onClose, onStatus }: CuesDialogProps) {
  const [editing, setEditing] = useState<earcons.CueId | null>(null);
  const [, setVersion] = useState(0);
  useEffect(() => earcons.onCuesChanged(() => setVersion((n) => n + 1)), []);
  useEffect(() => {
    if (!open) setEditing(null);
  }, [open]);
  const changed = earcons.CUES.filter((c) => !earcons.cueIsDefault(c.id)).length;

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        title="Cues"
        className="dialog-wide"
        covered={open && editing !== null}
        actions={
          <>
            <button
              type="button"
              data-testid="cues-reset"
              disabled={changed === 0}
              onClick={() => {
                earcons.resetCues();
                onStatus("Every cue is back as Coupler made it.");
              }}
            >
              Reset All Cues
            </button>
            <button type="button" className="primary" onClick={onClose}>
              Done
            </button>
          </>
        }
      >
        <p>
          Every cue Coupler makes: a sound, and a caption that writes it in Heard with sound captions on (Display). They play with Immersive's sound cues on. Edit changes
          how one sounds and reads; your changes are kept.
        </p>
        <p className="hooks-summary" data-testid="cues-summary">
          {changed === 0 ? `${earcons.CUES.length} cues, all as Coupler made them.` : `${earcons.CUES.length} cues, ${changed} changed by you.`}
        </p>
        <div className="hooks-head cues-row" aria-hidden="true">
          <span>Cue</span>
          <span>Group</span>
          <span>Now</span>
          <span />
        </div>
        <ul className="hooks-list cues-list" data-testid="cues-list" aria-label="Cues: name, group, then how it is now">
          {earcons.CUES.map((c) => {
            const summary = cueSummary(c.id);
            const mine = !earcons.cueIsDefault(c.id);
            return (
              <li key={c.id} className="cues-row">
                <span className="hook-key">{c.name}</span>
                <span className="hook-value">{c.group}</span>
                <span className={mine ? "hook-sets-off" : "hook-value"}>
                  {summary}
                  {mine ? ", set by you" : ""}
                </span>
                <button type="button" className="hook-edit" data-testid={`cues-edit-${c.id}`} aria-label={`Edit ${c.name}: ${summary}`} onClick={() => setEditing(c.id)}>
                  Edit…
                </button>
              </li>
            );
          })}
        </ul>
      </Dialog>
      <CueEditor id={open ? editing : null} onClose={() => setEditing(null)} onStatus={onStatus} />
    </>
  );
}

interface CueEditorProps {
  /** The cue being edited; null when closed. */
  id: earcons.CueId | null;
  onClose: () => void;
  onStatus: (message: string) => void;
}

function CueEditor({ id, onClose, onStatus }: CueEditorProps) {
  const uid = useId();
  const info = earcons.CUES.find((c) => c.id === id) ?? null;
  const [, setVersion] = useState(0);
  useEffect(() => earcons.onCuesChanged(() => setVersion((n) => n + 1)), []);
  const s = id ? earcons.cueSetting(id) : null;
  // The words are typed freely (an empty field means the cue's own), so
  // the field is set from what's kept only on opening, leaving and Back.
  const [words, setWords] = useState("");
  useEffect(() => setWords(id ? earcons.cueSetting(id).words : ""), [id]);

  const change = (c: Partial<earcons.CueSetting>) => id && earcons.setCue(id, c);
  const released = (e: { key?: string }) => {
    if (id && (e.key === undefined || /^(Arrow|Page|Home|End)/.test(e.key))) earcons.preview(id);
  };

  return (
    <Dialog
      open={info !== null}
      onClose={onClose}
      title={info ? `Cue: ${info.name}` : "Cue"}
      className="dialog-wide"
      actions={
        <>
          <button type="button" data-testid="cue-play" onClick={() => id && earcons.preview(id)}>
            Play It
          </button>
          <button
            type="button"
            data-testid="cue-reset"
            disabled={!id || earcons.cueIsDefault(id)}
            onClick={() => {
              if (!id || !info) return;
              earcons.setCue(id, null);
              setWords(info.words);
              onStatus(`${info.name} is back as Coupler made it.`);
            }}
          >
            Back to Coupler's Own
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      {info && s && (
        <>
          <p>
            {info.summary}{info.id === "loggedOn" || info.id === "loggedOff" ? "" : " Plays with Immersive's sound cues on."}
          </p>
          <div className="hook-edit-rows">
            <div className="hook-edit-row">
              <span>Sound</span>
              <label className="hook-loop">
                <input type="checkbox" data-testid="cue-sound" checked={s.sound} onChange={(e) => change({ sound: e.currentTarget.checked })} />
                <span>Plays</span>
              </label>
              <span className="hook-value">Off, only its caption is left.</span>
            </div>
            <div className="hook-edit-row">
              <label htmlFor={`${uid}-volume`}>Volume</label>
              <input
                id={`${uid}-volume`}
                type="range"
                className="voice-slider"
                min={0}
                max={100}
                step={5}
                disabled={!s.sound}
                data-testid="cue-volume"
                aria-valuetext={`${s.volume} percent of the cues' volume`}
                value={s.volume}
                onChange={(e) => change({ volume: Number(e.currentTarget.value) })}
                onPointerUp={() => released({})}
                onKeyUp={released}
              />
              <span className="mixer-value" aria-hidden="true">
                {String(s.volume).padStart(3)}% of CUE
              </span>
            </div>
            <div className="hook-edit-row">
              <label htmlFor={`${uid}-pitch`}>Pitch</label>
              <input
                id={`${uid}-pitch`}
                type="range"
                className="voice-slider"
                min={earcons.CUE_PITCH.least}
                max={earcons.CUE_PITCH.most}
                step={5}
                disabled={!s.sound}
                data-testid="cue-pitch"
                aria-valuetext={`${s.pitch} percent of its own pitch`}
                value={s.pitch}
                onChange={(e) => change({ pitch: Number(e.currentTarget.value) })}
                onPointerUp={() => released({})}
                onKeyUp={released}
              />
              <span className="mixer-value" aria-hidden="true">
                {String(s.pitch).padStart(3)}% of its own
              </span>
            </div>
            <div className="hook-edit-row">
              <span>Caption</span>
              <label className="hook-loop">
                <input type="checkbox" data-testid="cue-caption" checked={s.caption} onChange={(e) => change({ caption: e.currentTarget.checked })} />
                <span>Written</span>
              </label>
              <span className="hook-value">In Heard, with sound captions on (Display).</span>
            </div>
            <div className="hook-edit-row">
              <label htmlFor={`${uid}-words`}>Caption's words</label>
              <input
                id={`${uid}-words`}
                type="text"
                className="hook-asset"
                data-testid="cue-words"
                disabled={!s.caption}
                value={words}
                placeholder={info.words}
                autoComplete="off"
                autoCorrect="off"
                spellCheck={false}
                onChange={(e) => {
                  setWords(e.currentTarget.value);
                  change({ words: e.currentTarget.value });
                }}
                onBlur={() => setWords(s.words)}
              />
              <span className="hook-value">{captionExample(info, s.words)}</span>
            </div>
          </div>
        </>
      )}
    </Dialog>
  );
}

/** How the caption reads, details and all. */
function captionExample(info: earcons.CueInfo, words: string): string {
  const detail: Partial<Record<earcons.CueId, string>> = {
    exits: "north, east (new)",
    heartbeat: "fast, health under a quarter",
    opponentAt: "50%",
    loggedOn: "Hassan",
    loggedOff: "Hassan",
    logLine: "OOC",
    logReminder: "12 lines",
  };
  const d = detail[info.id];
  return `Reads: ♪ ${d ? `${words}: ${d}` : words}`;
}

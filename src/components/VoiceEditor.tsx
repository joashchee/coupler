/**
 * The voice editor: how one character sounds (src-tauri/src/cast.rs),
 * opened from their row's Edit button in Characters' Voices
 * (CastDialog.tsx), over the list.
 *
 * A row each: feminine or masculine (which of the system's voices
 * Automatic picks from, and how a built-in voice is pitched), the voice
 * (Automatic across every engine, Automatic within one, or any voice by
 * name, grouped by engine, the novelty ones too, each Automatic naming
 * the one it picked), pitch and speed as
 * sliders (the arrow keys move them by 5, Page Up and Page Down by more)
 * with their values beside them, and Quiet. Each change is saved at once
 * and kept; letting go of a slider says a line in the new voice, and Try
 * It does too. Back to Automatic makes the voice from the name again;
 * Forget drops the character until they're met again.
 *
 * It edits the narrator too (`narrator`): then there's no kind of voice,
 * Quiet or Forget, the voice is the system's own or any one by name, and
 * Back to the System's Voice puts it back as it was first heard.
 */
import { useEffect, useId, useState } from "react";
import * as mud from "../lib/mud";
import * as voice from "../lib/voice";
import { WHO } from "./CastDialog";
import { Dialog } from "./Dialog";

interface VoiceEditorProps {
  /** The character being edited; null when closed. */
  member: mud.CastMember | null;
  onChange: (change: Partial<mud.Voice>) => Promise<void>;
  onReset: () => Promise<void>;
  onForget: () => Promise<void>;
  onClose: () => void;
  /** Editing the narrator (lib/voice.ts), not a character. */
  narrator?: boolean;
}

const when = (seconds: number) => new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });

export function VoiceEditor({ member, onChange, onReset, onForget, onClose, narrator = false }: VoiceEditorProps) {
  const id = useId();
  const [voices, setVoices] = useState(voice.allVoices);
  useEffect(() => voice.onVoicesChanged(() => setVoices(voice.allVoices())), []);
  const engines = [...new Set(voices.map((x) => x.engine))];
  // The sliders move at once; what's saved catches up.
  const [pitch, setPitch] = useState(100);
  const [rate, setRate] = useState(100);
  // Only for another character, or Back to Automatic: each save's reply
  // would pull a slider back while it's still moving.
  const pitchSaved = member?.voice.pitch ?? 100;
  const rateSaved = member?.voice.rate ?? 100;
  useEffect(() => {
    setPitch(pitchSaved);
    setRate(rateSaved);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [member?.key, member?.edited]);

  const v = member?.voice;
  // What each Automatic would pick, to name it.
  const picks = (engine: string | null) => (v ? voice.characterVoice({ ...v, engine, voiceName: null }) : null);
  const automatic = narrator ? null : picks(null);
  const systemOwn = voice.systemVoices().find((x) => x.default)?.name ?? null;
  /** The menu's value: the engine and the voice, either may be empty. */
  const chosen = v?.engine ? `${v.engine}\n${v.voiceName ?? ""}` : "";
  const known = voices.some((x) => v?.engine === x.engine && v?.voiceName === x.id);
  const tryIt = (change: Partial<mud.Voice> = {}) => {
    if (!member || !v) return;
    tryVoice(member, narrator, { pitch, rate, ...change });
  };
  const released = (e: { key?: string }, change: Partial<mud.Voice>) => {
    if (e.key === undefined || /^(Arrow|Page|Home|End)/.test(e.key)) tryIt(change);
  };

  return (
    <Dialog
      open={member !== null}
      onClose={onClose}
      title={narrator ? "The Narrator's Voice" : "Edit Voice"}
      className="dialog-wide"
      actions={
        <>
          <button type="button" data-testid="voice-try" onClick={() => tryIt()}>
            Try It
          </button>
          <button type="button" data-testid="voice-reset" disabled={!member?.edited} onClick={() => void onReset()}>
            {narrator ? "Back to the System's Voice" : "Back to Automatic"}
          </button>
          {!narrator && (
            <button type="button" data-testid="voice-forget" onClick={() => void onForget()}>
              Forget
            </button>
          )}
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      {member && v && (
        <>
          <p className="hook-edit-pair">
            <span className="hook-key">{member.name}</span> <span className="hook-value">{narrator ? "Coupler" : WHO[member.who]}</span>
          </p>
          {narrator ? (
            <p>
              The voice of everything Coupler says itself: who's talking before they talk ("Hassan says", then Hassan's own voice), the time of day, the login and the
              answers to your keys. {member.edited ? "You've chosen this voice: it's kept as it is." : "It's the system's own voice until you choose another."}
            </p>
          ) : (
          <p>
            Met first {when(member.firstMet)}, last {when(member.lastMet)}. {member.lines === 0 ? "Not heard talking yet." : `Heard ${member.lines} ${member.lines === 1 ? "line" : "lines"}.`}{" "}
            {member.edited ? "You've changed this voice: it's kept as it is." : "Made from the name; change anything to keep it as you like."}
          </p>
          )}
          <div className="hook-edit-rows">
            {!narrator && (
            <div className="hook-edit-row">
              <label htmlFor={`${id}-gender`}>Kind of voice</label>
              <select
                id={`${id}-gender`}
                className="hook-asset"
                data-testid="voice-gender"
                value={v.gender}
                onChange={(e) => void onChange({ gender: e.currentTarget.value as mud.Voice["gender"] })}
              >
                <option value="feminine">Feminine</option>
                <option value="masculine">Masculine</option>
              </select>
              <span className="hook-value">Which voices Automatic picks from; Flite, one voice for everyone, is pitched to suit.</span>
            </div>
            )}
            <div className="hook-edit-row">
              <label htmlFor={`${id}-voice`}>Voice</label>
              <select
                id={`${id}-voice`}
                className="hook-asset"
                data-testid="voice-name"
                value={chosen}
                onChange={(e) => {
                  const [engine, name] = e.currentTarget.value.split("\n");
                  void onChange({ engine: engine || null, voiceName: name || null });
                }}
              >
                {narrator ? (
                  <option value="">{systemOwn ? `The system's voice (${systemOwn})` : "The system's voice"}</option>
                ) : (
                  <option value="">{automatic ? `Automatic, any engine (${automatic.name})` : "Automatic, any engine"}</option>
                )}
                {engines.map((engine) => (
                  <optgroup key={engine} label={voice.ENGINE_NAMES[engine] ?? engine}>
                    {!narrator && <option value={`${engine}\n`}>{`Automatic in ${voice.ENGINE_NAMES[engine] ?? engine} (${picks(engine)?.name ?? "none"})`}</option>}
                    {voices
                      .filter((x) => x.engine === engine)
                      .map((x) => (
                        <option key={x.id} value={`${engine}\n${x.id}`}>
                          {x.novelty ? `${x.name} (novelty)` : x.name}
                        </option>
                      ))}
                  </optgroup>
                ))}
                {/* One chosen that's no longer here still shows. */}
                {v.engine && v.voiceName && !known && <option value={chosen}>{v.voiceName} (not installed)</option>}
              </select>
              <span className="hook-value">Built-in voices sound the same on every Mac. More system voices: System Settings, Accessibility, Spoken Content.</span>
            </div>
            <div className="hook-edit-row">
              <label htmlFor={`${id}-pitch`}>Pitch</label>
              <input
                id={`${id}-pitch`}
                type="range"
                className="voice-slider"
                min={mud.PITCH.least}
                max={mud.PITCH.most}
                step={5}
                data-testid="voice-pitch"
                aria-valuetext={`${pitch} percent of the voice's own`}
                value={pitch}
                onChange={(e) => {
                  const n = Number(e.currentTarget.value);
                  setPitch(n);
                  void onChange({ pitch: n });
                }}
                onPointerUp={(e) => released({}, { pitch: Number(e.currentTarget.value) })}
                onKeyUp={(e) => released(e, { pitch: Number(e.currentTarget.value) })}
              />
              <span className="mixer-value" aria-hidden="true">
                {String(pitch).padStart(3)}%
              </span>
            </div>
            <div className="hook-edit-row">
              <label htmlFor={`${id}-rate`}>Speed</label>
              <input
                id={`${id}-rate`}
                type="range"
                className="voice-slider"
                min={mud.RATE.least}
                max={mud.RATE.most}
                step={5}
                data-testid="voice-rate"
                aria-valuetext={`${rate} percent of your speed`}
                value={rate}
                onChange={(e) => {
                  const n = Number(e.currentTarget.value);
                  setRate(n);
                  void onChange({ rate: n });
                }}
                onPointerUp={(e) => released({}, { rate: Number(e.currentTarget.value) })}
                onKeyUp={(e) => released(e, { rate: Number(e.currentTarget.value) })}
              />
              <span className="mixer-value" aria-hidden="true">
                {String(rate).padStart(3)}% of yours
              </span>
            </div>
            {!narrator && (
            <div className="hook-edit-row">
              <span />
              <label className="hook-loop">
                <input type="checkbox" data-testid="voice-quiet" checked={v.quiet} onChange={(e) => void onChange({ quiet: e.currentTarget.checked })} />
                <span>Quiet</span>
              </label>
              <span className="hook-value">Their lines are written in Heard, not spoken.</span>
            </div>
            )}
          </div>
        </>
      )}
    </Dialog>
  );
}

/**
 * Says a sample line in a character's voice, or the narrator's, at once:
 * the editor's Try It, and the one beside each row's Edit in the list.
 * `change`: settings being tried that aren't kept yet.
 */
export function tryVoice(member: mud.CastMember, narrator: boolean, change: Partial<mud.Voice> = {}) {
  // Every change to the narrator is in at once: a line with no voice is theirs.
  if (narrator) voice.speak("I'm the narrator. I say who's talking, the time of day, and the answers to your keys.", true);
  else voice.speak(`Well met. I am ${member.name}, and this is how I sound.`, true, { ...member.voice, quiet: false, ...change });
}

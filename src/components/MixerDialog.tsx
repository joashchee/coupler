/**
 * The Mixer: the master volumes for sound effects (SFX), music (BGM),
 * background noise (BGN), background weather (BGW) and Immersive's
 * sound cues (CUE, lib/earcons.ts), which every
 * trigger's own volume is a share of (lib/assets.ts), what music, noise
 * and weather are playing, and a way to stop it all. Opened from the menu bar's
 * Mixer button, or the now-playing note beside it.
 *
 * Each volume is a slider the arrow keys move by 1 (Page Up and Page
 * Down by 10), with its value beside it in words and numbers; a change
 * is heard at once and kept for next time. Coupler's voice
 * (lib/voice.ts) has its volume and speed here too.
 */
import { useEffect, useId, useState } from "react";
import * as sound from "../lib/assets";
import { assetTitle } from "../lib/mud";
import * as voice from "../lib/voice";
import { Dialog } from "./Dialog";

interface MixerDialogProps {
  open: boolean;
  onClose: () => void;
  /** The music playing now (an asset path), or null. */
  playing: string | null;
  onStatus: (message: string) => void;
}

const BUSES: [sound.Bus, string, string][] = [
  ["sfx", "SFX", "Sound effects"],
  ["bgm", "BGM", "Music"],
  ["bgn", "BGN", "Background noise"],
  ["bgw", "BGW", "Background weather"],
  ["cue", "CUE", "Immersive's sound cues"],
];

const levelsNow = () => Object.fromEntries(sound.BUSES.map((b) => [b, sound.masterVolume(b)])) as Record<sound.Bus, number>;

export function MixerDialog({ open, onClose, playing, onStatus }: MixerDialogProps) {
  const id = useId();
  const [levels, setLevels] = useState(levelsNow);
  const [speech, setSpeech] = useState({ volume: voice.voiceVolume(), rate: voice.voiceRate() });
  const [loops, setLoops] = useState<Record<sound.AmbientBus, string | null>>({ bgn: null, bgw: null });
  useEffect(() => sound.onLoops(setLoops), []);

  useEffect(() => {
    if (open) {
      setLevels(levelsNow());
      setSpeech({ volume: voice.voiceVolume(), rate: voice.voiceRate() });
    }
  }, [open]);

  function change(bus: sound.Bus, percent: number) {
    sound.setMasterVolume(bus, percent);
    setLevels((l) => ({ ...l, [bus]: sound.masterVolume(bus) }));
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Mixer"
      actions={
        <>
          <button
            type="button"
            data-testid="mixer-stop"
            onClick={() => {
              sound.stopAll();
              onStatus("Sounds, music, background noise and weather stopped.");
            }}
          >
            Stop All Sound
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <p>How loud the hooks' sounds, music, background noise and weather play, all together, and Immersive's cues and voice. Each hook's own volume is a share of these.</p>
      <div className="mixer-rows">
        {BUSES.map(([bus, short, long]) => (
          <div key={bus} className="mixer-row">
            <label htmlFor={`${id}-${bus}`}>{short}</label>
            <input
              id={`${id}-${bus}`}
              type="range"
              min={0}
              max={100}
              step={1}
              data-testid={`mixer-${bus}`}
              aria-label={`${long} volume`}
              aria-valuetext={`${levels[bus]} percent`}
              value={levels[bus]}
              onChange={(e) => change(bus, Number(e.currentTarget.value))}
            />
            <span className="mixer-value" aria-hidden="true">
              {String(levels[bus]).padStart(3)}%
            </span>
          </div>
        ))}
        <div className="mixer-row">
          <label htmlFor={`${id}-voice`}>VOX</label>
          <input
            id={`${id}-voice`}
            type="range"
            min={0}
            max={100}
            step={1}
            data-testid="mixer-voice"
            aria-label="Coupler's voice volume"
            aria-valuetext={`${speech.volume} percent`}
            value={speech.volume}
            onChange={(e) => {
              voice.setVoiceVolume(Number(e.currentTarget.value));
              setSpeech((s) => ({ ...s, volume: voice.voiceVolume() }));
            }}
          />
          <span className="mixer-value" aria-hidden="true">
            {String(speech.volume).padStart(3)}%
          </span>
        </div>
        <div className="mixer-row">
          <label htmlFor={`${id}-rate`}>RATE</label>
          <input
            id={`${id}-rate`}
            type="range"
            min={50}
            max={300}
            step={10}
            data-testid="mixer-rate"
            aria-label="Coupler's voice speed, percent of normal"
            aria-valuetext={`${speech.rate} percent of normal speed`}
            value={speech.rate}
            onChange={(e) => {
              voice.setVoiceRate(Number(e.currentTarget.value));
              setSpeech((s) => ({ ...s, rate: voice.voiceRate() }));
            }}
            onPointerUp={() => voice.speak("This is how fast I speak.", true)}
            onKeyUp={(e) => {
              if (/^(Arrow|Page|Home|End)/.test(e.key)) voice.speak("This is how fast I speak.", true);
            }}
          />
          <span className="mixer-value" aria-hidden="true">
            {String(speech.rate).padStart(3)}%
          </span>
        </div>
      </div>
      <p className="mixer-playing" data-testid="mixer-playing">
        {playing ? `Music: ${assetTitle(playing)}` : "No music playing."}
      </p>
      <p className="mixer-playing" data-testid="mixer-loops">
        {loops.bgn ? `Noise: ${assetTitle(loops.bgn)}` : "No background noise."} {loops.bgw ? `Weather: ${assetTitle(loops.bgw)}` : "No weather sound."}
      </p>
    </Dialog>
  );
}

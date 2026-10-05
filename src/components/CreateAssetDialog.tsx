/**
 * Create Asset, from the Hooks dialog beside Add Assets: an asset made
 * on this computer from the player's words, never fetched.
 *
 * - ART: a picture by Coupler's painter (src-tauri/src/create.rs reads
 *   the words for a place, the weather, the time of day and "epic"),
 *   saved as ANSI art at one of three sizes.
 * - BGM: a short loop of music by Coupler's composer for Neumetik
 *   (src-tauri/src/compose.rs reads a mood, an instrument, slow or fast,
 *   a number of beats a minute), saved as MIDI.
 *
 * Create saves it in the Assets folder under a name from the words; the
 * same words again make another take (`name (2)`). What was made is
 * said in the status bar and shown in words beside the picture (rule
 * 10), and music plays here in a loop, over and over until Stop (or
 * closing). Enter in the words creates.
 */
import { useEffect, useId, useState, type FormEvent } from "react";
import type { ActivityUpdate } from "../lib/activity";
import * as sound from "../lib/assets";
import * as mud from "../lib/mud";
import { ArtView } from "./AssetPreview";
import { Dialog } from "./Dialog";

type Kind = "art" | "bgm";

const KINDS: { id: Kind; name: string; summary: string; placeholder: string; hint: string }[] = [
  {
    id: "art",
    name: "ART",
    summary: "a picture by Coupler's painter",
    placeholder: "a stormy night out on the sea",
    hint: "It understands places (a forest, the sea, a tavern, a dungeon, a town...), the weather (rain, snow, fog, a storm), the time of day (dawn, day, dusk, night) and epic for high fantasy.",
  },
  {
    id: "bgm",
    name: "BGM",
    summary: "a short loop of music for Neumetik",
    placeholder: "a merry tavern jig with a fiddle",
    hint: "It understands moods (peaceful, tavern, dark, battle, heroic, magic, sad, sea, forest, desert, winter, night, synth, holy), instruments (harp, flute, piano...), slow, fast, soft, loud, major, minor, waltz, jig, march and a tempo (90 bpm).",
  },
];

/** The picture's sizes, in characters. */
const SIZES = [
  { id: "small", name: "Small, 40 by 12", columns: 40, rows: 12 },
  { id: "medium", name: "Medium, 60 by 18", columns: 60, rows: 18 },
  { id: "large", name: "Large, 80 by 25", columns: 80, rows: 25 },
] as const;

interface CreateAssetDialogProps {
  open: boolean;
  onClose: () => void;
  runActivity: <T>(label: string, task: (update: ActivityUpdate) => Promise<T>, key?: string) => Promise<T>;
  /** The Assets folder has a new file. */
  onMade: (made: mud.Made) => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

export function CreateAssetDialog({ open, onClose, runActivity, onMade, onStatus, onError }: CreateAssetDialogProps) {
  const id = useId();
  const [kind, setKind] = useState<Kind>("art");
  const [words, setWords] = useState("");
  const [size, setSize] = useState<(typeof SIZES)[number]["id"]>("large");
  const [busy, setBusy] = useState(false);
  /** What was made last, and its picture once read. */
  const [made, setMade] = useState<mud.Made | null>(null);
  const [picture, setPicture] = useState<sound.Shown | null>(null);
  const [trying, setTrying] = useState<string | null>(sound.previewing());
  useEffect(() => sound.onPreview(setTrying), []);
  // Closing stops the music it was playing.
  useEffect(() => {
    if (!open && made && sound.previewing() === made.path) sound.stopPreview();
  }, [open, made]);

  const chosen = KINDS.find((k) => k.id === kind) ?? KINDS[0];
  const playing = made !== null && trying === made.path;

  async function create(e?: FormEvent) {
    e?.preventDefault();
    if (busy) return;
    const prompt = words.trim() || chosen.placeholder;
    const { columns, rows } = SIZES.find((s) => s.id === size) ?? SIZES[2];
    setBusy(true);
    try {
      const done = await runActivity(
        kind === "art" ? `Painting ${prompt}…` : `Composing ${prompt}…`,
        () => (kind === "art" ? mud.assetCreateArt(prompt, columns, rows) : mud.assetCreateMusic(prompt)),
        "create-asset",
      );
      setMade(done);
      setPicture(null);
      onMade(done);
      onStatus(`Made ${done.name} in the Assets folder. ${done.about}`);
      if (done.kind === "art") setPicture(await sound.picture(done.path));
      else await sound.playPreview(done.path, "bgm", true);
    } catch (err) {
      onError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function playOrStop() {
    if (!made) return;
    if (playing) sound.stopPreview();
    else sound.playPreview(made.path, "bgm", true).catch((err) => onError(String(err)));
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Create Asset"
      className="dialog-wide create-asset"
      actions={
        <>
          <button type="button" onClick={onClose}>
            Close
          </button>
          <button type="button" className="primary" data-testid="create-asset-go" disabled={busy} aria-busy={busy} onClick={() => void create()}>
            Create
          </button>
        </>
      }
    >
      <fieldset className="radio-group" data-testid="create-asset-kinds">
        <legend>Make</legend>
        {KINDS.map((k) => (
          <label key={k.id} className="check-row">
            <input type="radio" name={`${id}-kind`} data-testid={`create-asset-${k.id}`} checked={kind === k.id} onChange={() => setKind(k.id)} />
            <span>
              {k.name}: {k.summary}
            </span>
          </label>
        ))}
      </fieldset>
      <form className="hooks-search create-asset-row" onSubmit={(e) => void create(e)}>
        <label htmlFor={`${id}-words`}>Describe it</label>
        <input
          id={`${id}-words`}
          type="text"
          data-testid="create-asset-words"
          aria-describedby={`${id}-hint`}
          placeholder={chosen.placeholder}
          value={words}
          maxLength={200}
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          onChange={(e) => setWords(e.currentTarget.value)}
        />
        {kind === "art" && (
          <>
            <label htmlFor={`${id}-size`}>Size</label>
            <select id={`${id}-size`} className="hook-asset create-asset-size" data-testid="create-asset-size" value={size} onChange={(e) => setSize(e.currentTarget.value as typeof size)}>
              {SIZES.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>
          </>
        )}
      </form>
      <p id={`${id}-hint`}>{chosen.hint} Nothing is fetched: it's all made on this computer.</p>
      {made && (
        <p className="create-asset-made" data-testid="create-asset-made">
          {made.name}: {made.about}
        </p>
      )}
      {made?.kind === "art" && picture && <ArtView picture={picture} name={`${made.name}: ${made.about}`} className="create-asset-picture" />}
      {made?.kind === "bgm" && (
        <button type="button" data-testid="create-asset-play" aria-label={`${playing ? "Stop" : "Play"} ${mud.assetTitle(made.path)}`} onClick={playOrStop}>
          {playing ? "Stop" : "Play"}
        </button>
      )}
    </Dialog>
  );
}

/**
 * **The Music Editor**, Workshop's BGM button: basic editing of the MIDI
 * files in the Assets folder, in a pattern and a piano roll.
 *
 * - The file: a MIDI asset opened, or New (a piano and the drums, four
 *   bars). Tempo, beats a bar and the number of bars.
 * - **Pattern**: a row a part (an instrument, or the drums), a column a
 *   bar, each cell how full that bar is (·· none, ░░ a few, ▒▒ some,
 *   ▓▓ many). Arrows move, Enter opens the bar in the piano roll, Cmd+C
 *   and Cmd+V copy and paste a bar, Delete clears it.
 * - **Part**: its name and instrument (or drum kit), Add Part, Add
 *   Drums, Remove Part.
 * - **Piano roll**: the chosen part's chosen bar, a row a key (a drum on
 *   the drums), a column a step (a sixteenth). Arrows move; Space or
 *   Enter puts a note in or takes it out; Shift+Right and Shift+Left
 *   make it longer and shorter; = and - louder and softer; Page Up and
 *   Page Down an octave. A click does what
 *   Space does, a Shift+click makes the chosen note reach the cell.
 * - Play Bar and Play Song loop until Stop (P and Shift+P in either
 *   grid), through Neumetik unless the player picks one of their
 *   SoundFonts beside them (Plays through); the SoundFont the file was
 *   given in the Assets tab isn't used here.
 * - Save… (over the file, after a yes) and Save as… (a new name in the
 *   Assets folder; nothing is written over). Closing or opening another
 *   with changes unsaved asks first.
 *
 * Both grids are buttons on the 8 by 16 grid with one Tab stop each,
 * named for VoiceOver ("D4, beat 2: a note, 2 steps, velocity 100"), and
 * a polite live region says where the cursor is. Every look of a cell
 * is in its name as well (rule 10). The song itself is src-tauri's
 * music.rs and lib/music.ts.
 */
import * as keys from "../lib/keys";
import { useCallback, useEffect, useId, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import * as sound from "../lib/assets";
import * as mud from "../lib/mud";
import * as music from "../lib/music";
import { Dialog } from "./Dialog";

interface MusicEditorDialogProps {
  open: boolean;
  onClose: () => void;
  /** The Assets folder changed elsewhere (a file added), to list again. */
  assetsVersion: number;
  /** A file was saved: what was decoded of it is out of date. */
  onSaved: (path: string) => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

/** The piano roll's rows, and the pattern's visible parts and bars. */
const ROLL_ROWS = 13;
const PATTERN_ROWS = 6;
const PATTERN_BARS = 24;
const PLAYING_ID = "music-editor";

type Ask = null | "over" | "as" | { discard: () => void };

export function MusicEditorDialog({ open, onClose, assetsVersion, onSaved, onStatus, onError }: MusicEditorDialogProps) {
  const id = useId();
  const [files, setFiles] = useState<string[]>([]);
  /** The SoundFonts in the Assets folder, and the one chosen to play through ("" is Neumetik, the default). */
  const [fonts, setFonts] = useState<string[]>([]);
  const [font, setFont] = useState("");
  const [chosen, setChosen] = useState("");
  const [path, setPath] = useState<string | null>(null);
  const [song, setSong] = useState<music.Song | null>(null);
  const [dirty, setDirty] = useState(false);
  const [inst, setInst] = useState<music.Instruments | null>(null);
  const [part, setPartIndex] = useState(0);
  const [bar, setBar] = useState(0);
  const [cursor, setCursor] = useState({ key: 60, step: 0 });
  const [top, setTop] = useState(67);
  const [clip, setClip] = useState<music.Note[] | null>(null);
  const [velocity, setVelocity] = useState(100);
  const [said, setSaid] = useState("");
  const [ask, setAsk] = useState<Ask>(null);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);
  const [trying, setTrying] = useState<string | null>(sound.previewing());
  useEffect(() => sound.onPreview(setTrying), []);
  const playing = trying === PLAYING_ID;
  const patternRef = useRef<HTMLDivElement>(null);
  const rollRef = useRef<HTMLDivElement>(null);

  // The MIDI files in the Assets folder, and Neumetik's instruments.
  useEffect(() => {
    if (!open) return;
    mud
      .assetsList()
      .then((l) => {
        const midi = l.assets.filter((a) => mud.isMidi(a.path)).map((a) => a.path);
        setFiles(midi);
        setChosen((c) => (c && midi.includes(c) ? c : (midi[0] ?? "")));
        const sf2 = l.assets.filter((a) => a.path.startsWith("sf2/")).map((a) => a.path);
        setFonts(sf2);
        // A SoundFont that's left the folder: back to Neumetik.
        setFont((f) => (f && sf2.includes(f) ? f : ""));
      })
      .catch((e) => onError(String(e)));
    if (!inst) music.instruments().then(setInst).catch((e) => onError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, assetsVersion]);

  // Closing stops what it was playing.
  useEffect(() => {
    if (!open && sound.previewing() === PLAYING_ID) sound.stopPreview();
  }, [open]);

  const title = path ? mud.assetTitle(path) : "New music";
  const current = song?.parts[part] ?? null;
  const steps = song ? music.stepsPerBar(song) : 16;
  const stepTicks = song ? music.stepTicks(song) : 120;
  const barStart = song ? bar * music.barTicks(song) : 0;

  /** Puts a song in the editor, the cursor on its first part and bar. */
  const load = useCallback((s: music.Song, from: string | null) => {
    setSong(s);
    setPath(from);
    setDirty(false);
    setPartIndex(0);
    setBar(0);
    const key = s.parts[0] ? music.middleKey(s.parts[0]) : 60;
    setCursor({ key, step: 0 });
    setTop(Math.min(127, key + Math.floor(ROLL_ROWS / 2)));
    if (sound.previewing() === PLAYING_ID) sound.stopPreview();
  }, []);

  /** Asks first when there are changes unsaved. */
  const unlessDirty = (then: () => void) => (dirty ? setAsk({ discard: then }) : then());

  function openChosen() {
    if (!chosen) return;
    unlessDirty(() => {
      setBusy(true);
      music
        .open(chosen)
        .then((s) => {
          load(s, chosen);
          onStatus(`Opened ${mud.assetTitle(chosen)}: ${s.parts.length} ${s.parts.length === 1 ? "part" : "parts"}, ${s.bars} ${s.bars === 1 ? "bar" : "bars"}.`);
        })
        .catch((e) => onError(String(e)))
        .finally(() => setBusy(false));
    });
  }

  function newSong() {
    unlessDirty(() => {
      music
        .fresh()
        .then((s) => {
          load(s, null);
          onStatus("A new song: a piano and the drums, four bars. Save as… names it.");
        })
        .catch((e) => onError(String(e)));
    });
  }

  /** Closes; changes left unsaved are let go, so they aren't there next time. */
  function close() {
    if (!dirty) {
      onClose();
      return;
    }
    setAsk({
      discard: () => {
        setSong(null);
        setPath(null);
        if (sound.previewing() === PLAYING_ID) sound.stopPreview();
        onClose();
      },
    });
  }

  /** An edit: the song changes and is unsaved. */
  function edit(next: music.Song | null) {
    if (!next || next === song) return;
    setSong(next);
    setDirty(true);
  }

  // ---- Saving ----

  async function saveOver() {
    if (!song || !path) return;
    setAsk(null);
    try {
      await music.save(path, song);
      setDirty(false);
      onSaved(path);
      onStatus(`Saved ${mud.assetTitle(path)}.`);
    } catch (e) {
      onError(String(e));
    }
  }

  async function saveAsNew() {
    if (!song) return;
    const name = newName.trim();
    if (!name) return;
    setAsk(null);
    try {
      const made = await music.saveAs(name, song);
      setPath(made.path);
      setChosen(made.path);
      setFiles((f) => [...f, made.path].sort());
      setDirty(false);
      onSaved(made.path);
      onStatus(`Saved as ${made.name} in the Assets folder.`);
    } catch (e) {
      onError(String(e));
    }
  }

  function askSaveAs() {
    setNewName(path ? `${mud.assetTitle(path)} edit` : "my music");
    setAsk("as");
  }

  // ---- Playing ----

  async function play(whole: boolean) {
    if (!song) return;
    if (playing) {
      sound.stopPreview();
      return;
    }
    const [from, to] = whole ? [0, song.bars] : [bar, bar + 1];
    try {
      const bytes = await music.render(font || null, song, from, to, true);
      await sound.playPreviewBytes(PLAYING_ID, bytes, whole ? `${title}, the whole song, looping` : `${title}, bar ${bar + 1}, looping`, true);
    } catch (e) {
      onError(String(e));
    }
  }

  // ---- Telling where the cursor is ----

  const cellWords = useCallback(
    (s: music.Song, p: music.Part, key: number, step: number) => {
      const from = bar * music.barTicks(s) + step * music.stepTicks(s);
      const cell = music.cellOf(p, key, from, from + music.stepTicks(s));
      const where = `${music.rowWords(p, key)}, ${music.stepWords(s, step)}`;
      if (cell.kind === "empty" || !cell.note) return `${where}: empty`;
      const long = Math.max(1, Math.round(cell.note.length / music.stepTicks(s)));
      const what = cell.kind === "start" ? "a note" : "a note going on";
      return `${where}: ${what}, ${long} ${long === 1 ? "step" : "steps"}, velocity ${cell.note.velocity}`;
    },
    [bar],
  );

  const barWords = (s: music.Song, p: music.Part, b: number) => {
    const n = music.notesIn(p, b * music.barTicks(s), (b + 1) * music.barTicks(s)).length;
    return `${p.name}, bar ${b + 1}: ${n === 0 ? "empty" : `${n} ${n === 1 ? "note" : "notes"}`}`;
  };

  // ---- The pattern ----

  // The parts and bars in view: always the chosen one's.
  const partOffset = Math.max(0, part - PATTERN_ROWS + 1);
  const firstBar = Math.max(0, Math.min(bar - Math.floor(PATTERN_BARS / 2), (song?.bars ?? 0) - PATTERN_BARS));
  const shownParts = song ? song.parts.slice(partOffset, partOffset + PATTERN_ROWS) : [];
  const shownBars = song ? [...Array(Math.min(PATTERN_BARS, song.bars)).keys()].map((i) => firstBar + i) : [];

  function focusIn(ref: RefObject<HTMLDivElement | null>) {
    window.setTimeout(() => ref.current?.querySelector<HTMLButtonElement>("button[tabindex='0']")?.focus(), 0);
  }

  function choosePart(index: number) {
    if (!song) return;
    setPartIndex(index);
    const p = song.parts[index];
    if (p) {
      const key = music.middleKey(p);
      setCursor((c) => ({ ...c, key }));
      setTop(Math.min(127, key + Math.floor(ROLL_ROWS / 2)));
    }
  }

  function onPatternKey(e: KeyboardEvent<HTMLDivElement>) {
    if (!song || song.parts.length === 0) return;
    const cmd = keys.command(e);
    let p = part;
    let b = bar;
    if (e.key === "ArrowUp") p = Math.max(0, part - 1);
    else if (e.key === "ArrowDown") p = Math.min(song.parts.length - 1, part + 1);
    else if (e.key === "ArrowLeft") b = Math.max(0, bar - 1);
    else if (e.key === "ArrowRight") b = Math.min(song.bars - 1, bar + 1);
    else if (e.key === "Home") b = 0;
    else if (e.key === "End") b = song.bars - 1;
    else if (e.key === "Enter") {
      e.preventDefault();
      focusIn(rollRef);
      return;
    } else if (cmd && keys.letter(e) === "c") {
      e.preventDefault();
      setClip(music.copyBar(song, part, bar));
      setSaid(`Copied bar ${bar + 1} of ${song.parts[part].name}.`);
      return;
    } else if (cmd && keys.letter(e) === "v") {
      e.preventDefault();
      if (!clip) {
        setSaid("Nothing copied yet.");
        return;
      }
      const next = music.pasteBar(song, part, bar, clip);
      edit(next);
      setSaid(`Pasted into bar ${bar + 1}. ${barWords(next, next.parts[part], bar)}.`);
      return;
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      edit(music.clearBar(song, part, bar));
      setSaid(`Cleared bar ${bar + 1} of ${song.parts[part].name}.`);
      return;
    } else if (e.key.toLowerCase() === "p" && !cmd) {
      e.preventDefault();
      void play(e.shiftKey);
      return;
    } else return;
    e.preventDefault();
    if (p !== part) choosePart(p);
    setBar(b);
    setSaid(barWords(song, song.parts[p], b));
    focusIn(patternRef);
  }

  // ---- The piano roll ----

  const rows = [...Array(ROLL_ROWS).keys()].map((i) => top - i).filter((k) => k >= 0 && k <= 127);

  function moveCursor(key: number, step: number) {
    if (!song || !current) return;
    let k = Math.max(0, Math.min(127, key));
    let s = step;
    let b = bar;
    if (s < 0 && b > 0) {
      b -= 1;
      s = steps - 1;
    } else if (s >= steps && b < song.bars - 1) {
      b += 1;
      s = 0;
    }
    s = Math.max(0, Math.min(steps - 1, s));
    if (k > top) setTop(k);
    else if (k < top - ROLL_ROWS + 1) setTop(k + ROLL_ROWS - 1);
    k = Math.max(0, Math.min(127, k));
    setBar(b);
    setCursor({ key: k, step: s });
    const fromBar = b;
    // The words for the cell in the bar it's now in.
    const from = fromBar * music.barTicks(song) + s * stepTicks;
    const cell = music.cellOf(current, k, from, from + stepTicks);
    const where = `${b !== bar ? `Bar ${b + 1}. ` : ""}${music.rowWords(current, k)}, ${music.stepWords(song, s)}`;
    setSaid(cell.kind === "empty" ? `${where}: empty` : `${where}: ${cell.kind === "start" ? "a note" : "a note going on"}`);
    focusIn(rollRef);
  }

  function toggleAt(key: number, step: number) {
    if (!song || !current) return;
    const next = music.toggle(song, part, key, barStart + step * stepTicks, velocity);
    edit(next);
    setSaid(cellWords(next, next.parts[part], key, step));
  }

  function onRollKey(e: KeyboardEvent<HTMLDivElement>) {
    if (!song || !current) return;
    const { key, step } = cursor;
    const from = barStart + step * stepTicks;
    if (e.key === "ArrowUp") moveCursor(key + 1, step);
    else if (e.key === "ArrowDown") moveCursor(key - 1, step);
    else if (e.key === "ArrowRight" && e.shiftKey) {
      const next = music.lengthen(song, part, key, from, 1);
      edit(next);
      setSaid(cellWords(next, next.parts[part], key, step));
    } else if (e.key === "ArrowLeft" && e.shiftKey) {
      const next = music.lengthen(song, part, key, from, -1);
      edit(next);
      setSaid(cellWords(next, next.parts[part], key, step));
    } else if (e.key === "ArrowRight") moveCursor(key, step + 1);
    else if (e.key === "ArrowLeft") moveCursor(key, step - 1);
    else if (e.key === "PageUp") moveCursor(key + 12, step);
    else if (e.key === "PageDown") moveCursor(key - 12, step);
    else if (e.key === " " || e.key === "Enter") toggleAt(key, step);
    else if (e.key === "+" || e.key === "=" || e.key === "-" || e.key === "_") {
      const by = e.key === "+" || e.key === "=" ? 10 : -10;
      const next = music.louder(song, part, key, from, by);
      if (next === song) {
        setVelocity((v) => Math.max(1, Math.min(127, v + by)));
        setSaid(`New notes at velocity ${Math.max(1, Math.min(127, velocity + by))}.`);
      } else {
        edit(next);
        setSaid(cellWords(next, next.parts[part], key, step));
      }
    } else if (e.key.toLowerCase() === "p" && !e.metaKey && !e.ctrlKey) void play(e.shiftKey);
    else if (e.key === "Escape") {
      e.stopPropagation();
      focusIn(patternRef);
    } else return;
    e.preventDefault();
  }

  function clickCell(key: number, step: number, shift: boolean) {
    if (!song || !current) return;
    if (shift) {
      const from = barStart + cursor.step * stepTicks;
      const next = music.lengthenTo(song, part, cursor.key, from, barStart + step * stepTicks);
      edit(next);
      setSaid(cellWords(next, next.parts[part], cursor.key, cursor.step));
      return;
    }
    setCursor({ key, step });
    toggleAt(key, step);
  }

  // ---- The part ----

  const instrumentOptions = useMemo(() => {
    if (!inst) return [];
    return inst.banks.map((b) => ({ label: music.BANK_NAMES[b.msb] ?? `Bank ${b.msb}`, items: b.names.map((n, i) => ({ value: `${b.msb}:${i}`, name: `${i + 1} ${n}` })) }));
  }, [inst]);

  function addPart(drums: boolean) {
    if (!song) return;
    const next = music.addPart(song, drums);
    if (!next) {
      onError(drums ? "The song has drums already: they have a channel of their own." : "Every channel has a part already (16 at most, the drums' among them).");
      return;
    }
    edit(next);
    choosePart(next.parts.length - 1);
    setSaid(`Added ${next.parts[next.parts.length - 1].name}.`);
  }

  function removePart() {
    if (!song || !current) return;
    const next = music.removePart(song, part);
    edit(next);
    setPartIndex(Math.max(0, Math.min(part, next.parts.length - 1)));
    setSaid(`Removed ${current.name}.`);
  }

  const askTitle = ask === "over" ? `Save over ${title}?` : ask === "as" ? "Save as…" : "Leave your changes?";

  return (
    <>
      <Dialog
        open={open}
        onClose={close}
        covered={ask !== null}
        title={`Music Editor: ${title}${dirty ? " (changed)" : ""}`}
        className="dialog-wide dialog-music"
        actions={
          <>
            <button type="button" data-testid="music-close" onClick={close}>
              Close
            </button>
            <button type="button" data-testid="music-save-as" disabled={!song} onClick={askSaveAs}>
              Save as…
            </button>
            <button type="button" className="primary" data-testid="music-save" disabled={!song || (!dirty && path !== null)} onClick={() => (path ? setAsk("over") : askSaveAs())}>
              Save…
            </button>
          </>
        }
      >
        <div className="music-row">
          <label htmlFor={`${id}-file`}>MIDI file</label>
          <select id={`${id}-file`} className="music-file" data-testid="music-file" value={chosen} onChange={(e) => setChosen(e.currentTarget.value)}>
            {files.length === 0 && <option value="">No MIDI files in the Assets folder</option>}
            {files.map((f) => (
              <option key={f} value={f}>
                {mud.assetTitle(f)}
              </option>
            ))}
          </select>
          <button type="button" data-testid="music-open" disabled={!chosen || busy} aria-busy={busy} onClick={openChosen}>
            Open
          </button>
          <button type="button" data-testid="music-new" onClick={newSong}>
            New
          </button>
        </div>
        {!song && <p>Open a MIDI file from the Assets folder, or start a new one. Coupler's composer (Create Asset) makes some to begin with.</p>}
        {song && (
          <>
            <div className="music-row">
              <label htmlFor={`${id}-bpm`}>Tempo</label>
              <input
                id={`${id}-bpm`}
                type="number"
                className="music-number"
                data-testid="music-bpm"
                min={20}
                max={300}
                value={Math.round(song.bpm)}
                onChange={(e) => {
                  const v = Number(e.currentTarget.value);
                  if (v >= 20 && v <= 300) edit({ ...song, bpm: v });
                }}
              />
              <span>beats a minute</span>
              <label htmlFor={`${id}-beats`}>Bar</label>
              <select id={`${id}-beats`} className="music-beats" data-testid="music-beats" value={song.beats} onChange={(e) => edit({ ...song, beats: Number(e.currentTarget.value) })}>
                {[2, 3, 4, 5, 6, 7, 8, 9, 12].map((b) => (
                  <option key={b} value={b}>
                    {`${b}/${song.unit}`}
                  </option>
                ))}
              </select>
              <label htmlFor={`${id}-bars`}>Bars</label>
              <input
                id={`${id}-bars`}
                type="number"
                className="music-number"
                data-testid="music-bars"
                min={1}
                max={999}
                value={song.bars}
                onChange={(e) => {
                  const v = Number(e.currentTarget.value);
                  if (v >= 1 && v <= 999) {
                    edit(music.setBars(song, v));
                    setBar((b) => Math.min(b, v - 1));
                  }
                }}
              />
            </div>

            <p className="music-heading" id={`${id}-pattern-label`}>
              Pattern: a row a part, a column a bar (·· none, ░░ a few notes, ▒▒ some, ▓▓ many)
            </p>
            <div className="music-grid music-pattern" ref={patternRef} role="group" aria-labelledby={`${id}-pattern-label`} aria-describedby={`${id}-pattern-keys`} onKeyDown={onPatternKey} data-testid="music-pattern">
              <div className="music-line" aria-hidden="true">
                <span className="music-label">{"".padEnd(20)}</span>
                {shownBars.map((b) => (
                  <span key={b} className="music-bar-number">
                    {(b + 1) % 4 === 1 ? String(b + 1).padEnd(3) : "   "}
                  </span>
                ))}
              </div>
              {shownParts.map((p, i) => {
                const index = partOffset + i;
                return (
                  <div key={index} className="music-line">
                    <span className={`music-label${index === part ? " chosen" : ""}`} aria-hidden="true">
                      {p.name.slice(0, 19).padEnd(20)}
                    </span>
                    {shownBars.map((b) => {
                      const n = music.notesIn(p, b * music.barTicks(song), (b + 1) * music.barTicks(song)).length;
                      const here = index === part && b === bar;
                      return (
                        <button
                          key={b}
                          type="button"
                          className={`music-cell music-bar${here ? " chosen" : ""}`}
                          tabIndex={here ? 0 : -1}
                          aria-label={barWords(song, p, b)}
                          aria-current={here ? "true" : undefined}
                          onClick={() => {
                            if (index !== part) choosePart(index);
                            setBar(b);
                          }}
                          onDoubleClick={() => focusIn(rollRef)}
                        >
                          {`${music.density(n)} `}
                        </button>
                      );
                    })}
                  </div>
                );
              })}
            </div>
            <p className="music-keys" id={`${id}-pattern-keys`}>
              {keys.keys("Arrows choose a bar, Enter opens it below. Cmd+C, Cmd+V copy and paste a bar, Delete clears it. P plays the bar, Shift+P the song.")}
            </p>

            {current && (
              <div className="music-row">
                <label htmlFor={`${id}-name`}>Part</label>
                <input
                  id={`${id}-name`}
                  type="text"
                  className="music-name"
                  data-testid="music-part-name"
                  value={current.name}
                  maxLength={40}
                  onChange={(e) => edit(music.setPart(song, part, { name: e.currentTarget.value }))}
                />
                {current.channel === music.DRUMS ? (
                  <>
                    <label htmlFor={`${id}-inst`}>Kit</label>
                    <select id={`${id}-inst`} className="music-instrument" data-testid="music-instrument" value={[...(inst?.kits ?? [])].reverse().find(([p]) => p <= current.program)?.[0] ?? 0} onChange={(e) => edit(music.setPart(song, part, { program: Number(e.currentTarget.value), bank: 0 }))}>
                      {(inst?.kits ?? []).map(([p, n]) => (
                        <option key={p} value={p}>
                          {n}
                        </option>
                      ))}
                    </select>
                  </>
                ) : (
                  <>
                    <label htmlFor={`${id}-inst`}>Instrument</label>
                    <select
                      id={`${id}-inst`}
                      className="music-instrument"
                      data-testid="music-instrument"
                      value={`${inst?.banks.some((b) => b.msb === current.bank) ? current.bank : 0}:${current.program}`}
                      onChange={(e) => {
                        const [bank, program] = e.currentTarget.value.split(":").map(Number);
                        edit(music.setPart(song, part, { bank, program }));
                      }}
                    >
                      {instrumentOptions.map((g) => (
                        <optgroup key={g.label} label={g.label}>
                          {g.items.map((o) => (
                            <option key={o.value} value={o.value}>
                              {o.name}
                            </option>
                          ))}
                        </optgroup>
                      ))}
                    </select>
                  </>
                )}
                <button type="button" data-testid="music-add-part" onClick={() => addPart(false)}>
                  Add Part
                </button>
                <button type="button" data-testid="music-add-drums" onClick={() => addPart(true)}>
                  Add Drums
                </button>
                <button type="button" data-testid="music-remove-part" onClick={removePart}>
                  Remove Part
                </button>
              </div>
            )}

            {current && (
              <>
                <p className="music-heading" id={`${id}-roll-label`}>
                  {`Piano roll: ${current.name}, bar ${bar + 1} of ${song.bars}`}
                </p>
                <div className="music-grid music-roll" ref={rollRef} role="group" aria-labelledby={`${id}-roll-label`} aria-describedby={`${id}-roll-keys`} onKeyDown={onRollKey} data-testid="music-roll">
                  {rows.map((key) => (
                    <div key={key} className={`music-line${music.isBlack(key) && current.channel !== music.DRUMS ? " black" : ""}`}>
                      <span className="music-label" aria-hidden="true">
                        {music.rowName(current, key).slice(0, 13).padEnd(14)}
                      </span>
                      {[...Array(steps).keys()].map((step) => {
                        const from = barStart + step * stepTicks;
                        const cell = music.cellOf(current, key, from, from + stepTicks);
                        const here = cursor.key === key && cursor.step === step;
                        const beat = step % music.stepsPerBeat(song) === 0;
                        const text = cell.kind === "start" ? "██" : cell.kind === "held" ? "▒▒" : beat ? "│·" : " ·";
                        return (
                          <button
                            key={step}
                            type="button"
                            className={`music-cell music-step ${cell.kind}${here ? " here" : ""}`}
                            tabIndex={here ? 0 : -1}
                            aria-label={cellWords(song, current, key, step)}
                            onClick={(e) => clickCell(key, step, e.shiftKey)}
                          >
                            {text}
                          </button>
                        );
                      })}
                    </div>
                  ))}
                </div>
                <p className="music-keys" id={`${id}-roll-keys`}>
                  Arrows move, Space puts a note in or takes it out, Shift+Right and Shift+Left make it longer and shorter, = and - louder and softer, Page Up and
                  Page Down an octave. P plays the bar, Shift+P the song.
                </p>
              </>
            )}

            <div className="music-row">
              <button type="button" data-testid="music-play-bar" onClick={() => void play(false)}>
                {playing ? "Stop" : "Play Bar"}
              </button>
              <button type="button" data-testid="music-play-song" onClick={() => void play(true)}>
                {playing ? "Stop" : "Play Song"}
              </button>
              <label htmlFor={`${id}-font`}>Plays through</label>
              <select
                id={`${id}-font`}
                className="music-file"
                data-testid="music-font"
                value={font}
                onChange={(e) => {
                  setFont(e.currentTarget.value);
                  if (playing) sound.stopPreview();
                }}
              >
                <option value="">Neumetik</option>
                {fonts.map((f) => (
                  <option key={f} value={f}>
                    {`SoundFont: ${mud.assetTitle(f)}`}
                  </option>
                ))}
              </select>
              <span>{`New notes at velocity ${velocity}.`}</span>
            </div>
          </>
        )}
        <p className="sr-only" aria-live="polite" data-testid="music-said">
          {said}
        </p>
      </Dialog>

      <Dialog
        open={ask !== null}
        onClose={() => setAsk(null)}
        title={askTitle}
        actions={
          ask === "over" ? (
            <>
              <button type="button" onClick={() => setAsk(null)}>
                Cancel
              </button>
              <button type="button" className="primary" data-testid="music-save-confirm" onClick={() => void saveOver()}>
                Save
              </button>
            </>
          ) : ask === "as" ? (
            <>
              <button type="button" onClick={() => setAsk(null)}>
                Cancel
              </button>
              <button type="button" className="primary" data-testid="music-save-as-confirm" disabled={!newName.trim()} onClick={() => void saveAsNew()}>
                Save
              </button>
            </>
          ) : (
            <>
              <button type="button" onClick={() => setAsk(null)}>
                Keep Editing
              </button>
              <button
                type="button"
                className="primary"
                data-testid="music-discard"
                onClick={() => {
                  const then = ask && typeof ask === "object" ? ask.discard : null;
                  setAsk(null);
                  setDirty(false);
                  then?.();
                }}
              >
                Leave Them
              </button>
            </>
          )
        }
      >
        {ask === "over" && <p>{`Your changes take the place of ${title} in the Assets folder. Hooks that play it will play the new one.`}</p>}
        {ask === "as" && (
          <form
            className="music-row"
            onSubmit={(e) => {
              e.preventDefault();
              void saveAsNew();
            }}
          >
            <label htmlFor={`${id}-new-name`}>Name</label>
            <input
              id={`${id}-new-name`}
              type="text"
              className="music-name"
              data-testid="music-new-name"
              value={newName}
              maxLength={40}
              autoComplete="off"
              spellCheck={false}
              onChange={(e) => setNewName(e.currentTarget.value)}
            />
            <span>.mid, in the Assets folder</span>
          </form>
        )}
        {ask !== null && typeof ask === "object" && <p>{`The changes to ${title} aren't saved. Leave them?`}</p>}
      </Dialog>
    </>
  );
}

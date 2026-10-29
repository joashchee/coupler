import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { Dialog } from "./Dialog";
import * as mud from "../lib/mud";
import { palette as gamePalette, PALETTE_WORDS, type Palette } from "../lib/palette";
import type { PictureStyle } from "../lib/mud";
import { engineAvailable, engineVersion, paintRoom } from "../lib/sprites";

/**
 * Eyetest (gear → Eyetest, dev only like App Testing): Coupler's own
 * painter (Rust, src-tauri/src/painter.rs) beside the Painter engine
 * (lib/sprites.ts) drawing the same scene, so a person can judge them by
 * eye (the bar is "as informative as Coupler's", not "the same"). Any of
 * the game's three color depths without touching the game's own, a walk
 * through every terrain, time and weather, and buttons (and keys 1 to 4)
 * to report each: which is better, what is wrong, a note. The reports are
 * kept here and become a text report to copy or save.
 *
 * Everything shown is made up (a fixed world and room names); no game
 * text or player data is in a report (rule 1). Mounted by App.tsx only
 * behind `import.meta.env.DEV`: it never reaches a release.
 */

interface EyetestProps {
  open: boolean;
  onClose: () => void;
}

const OUTDOOR = ["city", "woods", "rocky", "plains", "underwater", "air", "watersurface", "jungle", "swamp", "desert", "hills", "mountains", "spaceport", "seaport"];
const INDOOR = ["stone", "wooden", "cave", "magic", "in_underwater", "gap", "cavelakesurface", "metal", "innerseaport", "caveseaport"];
const TERRAINS = [...OUTDOOR, ...INDOOR];
const TIMES = ["dawn", "day", "dusk", "night"];
const WEATHERS = ["clear", "cloudy", "windy", "rain", "thunderstorm", "snow", "blizzard", "hail", "sleet", "fog", "dust", "heat", "drought", "cold"];
const SKYLESS = (terrain: string) => INDOOR.includes(terrain) || terrain === "underwater";

/** The sweep: each outdoor terrain in a clear day, a thunderstorm at night and a snowy dusk, each indoor one once. */
const SWEEP: { terrain: string; time: string; weather: string }[] = TERRAINS.flatMap((terrain) =>
  SKYLESS(terrain)
    ? [{ terrain, time: "day", weather: "clear" }]
    : [
        { terrain, time: "day", weather: "clear" },
        { terrain, time: "night", weather: "thunderstorm" },
        { terrain, time: "dusk", weather: "snow" },
      ],
);

const SIZES = ["40x12", "64x18", "72x20", "80x24", "160x45"];
/** The dialog's inner width in stage pixels: two pictures fit side by side up to this. */
const ACROSS = 1200;

const PALETTES: { id: Palette; name: string }[] = [
  { id: "16", name: "ANSI 16" },
  { id: "256", name: "ANSI 256" },
  { id: "true", name: "ANSI TRUE" },
];

type Verdict = "engine" | "same" | "rust" | "broken";
const VERDICTS: { id: Verdict; key: string; label: string; words: string }[] = [
  { id: "engine", key: "1", label: "Engine better", words: "Painter engine better" },
  { id: "same", key: "2", label: "Same", words: "About the same" },
  { id: "rust", key: "3", label: "Coupler's better", words: "Coupler's painter better" },
  { id: "broken", key: "4", label: "Engine broken", words: "Painter engine broken or empty" },
];
const TAGS = ["Ground", "Sky", "Weather", "Light", "Detail", "Colors", "Depth", "Glitch"];

interface Report {
  terrain: string;
  time: string;
  weather: string;
  style: PictureStyle;
  palette: Palette;
  size: string;
  look: number;
  verdict: Verdict | null;
  tags: string[];
  note: string;
}

const KEY = "coupler.eyetest";
const keyOf = (r: Pick<Report, "terrain" | "time" | "weather" | "style" | "palette">) => `${r.terrain}|${r.time}|${r.weather}|${r.style}|${r.palette}`;

function loadReports(): Record<string, Report> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, Report>;
  } catch {
    return {};
  }
}

interface Drawn {
  art: mud.AnsiArt | null;
  says: string;
  error: string | null;
}

function Art({ drawn }: { drawn: Drawn | null }) {
  if (!drawn) return <p className="map-muted">Drawing…</p>;
  if (!drawn.art) return <p className="map-muted">Nothing drawn: {drawn.error}</p>;
  return (
    <div className="room-picture-art" style={{ width: drawn.art.columns * 8 }}>
      {drawn.art.lines.map((line, row) => (
        <div key={row} className="room-picture-row">
          {line.map((span, i) => {
            const { color, background } = mud.spanColors(span);
            return (
              <span key={i} style={{ color, background }}>
                {span.text}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
}

export function EyetestDialog({ open, onClose }: EyetestProps) {
  const [terrain, setTerrain] = useState("city");
  const [time, setTime] = useState("night");
  const [weather, setWeather] = useState("thunderstorm");
  const [style, setStyle] = useState<PictureStyle>("fantasy");
  const [depth, setDepth] = useState<Palette>(gamePalette());
  const [size, setSize] = useState("64x18");
  const [look, setLook] = useState(0);
  const [flip, setFlip] = useState<"rust" | "engine">("rust");
  const [advance, setAdvance] = useState(true);
  const [reports, setReports] = useState<Record<string, Report>>(loadReports);
  const [rust, setRust] = useState<Drawn | null>(null);
  const [engine, setEngine] = useState<Drawn | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const body = useRef<HTMLDivElement>(null);

  useEffect(() => {
    try {
      localStorage.setItem(KEY, JSON.stringify(reports));
    } catch {
      // Not kept past this run, then.
    }
  }, [reports]);
  useEffect(() => {
    void engineVersion().then(setVersion);
  }, []);

  const [columns, rows] = size.split("x").map(Number);
  const sideBySide = columns * 8 * 2 + 16 <= ACROSS;
  const skyless = SKYLESS(terrain);
  /** The scene as the game would give it: a made-up world and room, so nothing real is in it. */
  const scene = useMemo<mud.Scene>(
    () => ({ world: "eyetest", room: `Eyetest#${terrain}`, name: "", zone: "", terrain, weather, time, arc: null, look, art: null }),
    [terrain, weather, time, look],
  );

  useEffect(() => {
    if (!open) return;
    let stale = false;
    setRust(null);
    setEngine(null);
    mud
      .picturePaint(scene, style, columns, rows, look, depth)
      .then((art) => !stale && setRust({ art, says: "", error: null }))
      .catch((e) => !stale && setRust({ art: null, says: "", error: String(e) }));
    if (engineAvailable) {
      paintRoom(scene, style, columns, rows, depth)
        .then(({ art, says }) => !stale && setEngine({ art, says, error: null }))
        .catch((e) => !stale && setEngine({ art: null, says: "", error: String(e) }));
    } else {
      setEngine({ art: null, says: "", error: "this checkout has no copy of the engine (scripts/painter-sync.sh)" });
    }
    return () => {
      stale = true;
    };
  }, [open, scene, style, columns, rows, look, depth]);

  const current = reports[keyOf({ terrain, time, weather, style, palette: depth })];
  const at = SWEEP.findIndex((s) => s.terrain === terrain && s.time === time && s.weather === weather);
  const done = SWEEP.filter((s) => reports[keyOf({ ...s, style, palette: depth })]?.verdict).length;

  const go = useCallback(
    (step: number) => {
      const next = SWEEP[(((at < 0 ? 0 : at) + step) % SWEEP.length + SWEEP.length) % SWEEP.length];
      setTerrain(next.terrain);
      setTime(next.time);
      setWeather(next.weather);
      setLook(0);
    },
    [at],
  );

  const put = (change: Partial<Report>) =>
    setReports((all) => {
      const base: Report = all[keyOf({ terrain, time, weather, style, palette: depth })] ?? { terrain, time, weather, style, palette: depth, size, look, verdict: null, tags: [], note: "" };
      return { ...all, [keyOf(base)]: { ...base, size, look, ...change } };
    });

  const verdict = (v: Verdict) => {
    put({ verdict: v });
    if (advance) go(1);
  };
  const tag = (t: string) => put({ tags: current?.tags.includes(t) ? current.tags.filter((x) => x !== t) : [...(current?.tags ?? []), t] });

  const onKey = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest("input, textarea, select") || e.metaKey || e.ctrlKey || e.altKey) return;
    const v = VERDICTS.find((x) => x.key === e.key);
    if (v) verdict(v.id);
    else if (e.key === "n") go(1);
    else if (e.key === "p") go(-1);
    else if (e.key === "f") setFlip((f) => (f === "rust" ? "engine" : "rust"));
    else return;
    e.preventDefault();
  };

  /** The report as text: the tally, each verdict's scenes with tags and notes, and which tags came up where. */
  const reportText = () => {
    const made = Object.values(reports).filter((r) => r.verdict);
    const line = (r: Report) => `  - ${r.terrain}, ${r.time}, ${r.weather}, ${r.style}, ${PALETTE_NAMES[r.palette]}, ${r.size}${r.look ? `, look ${r.look + 1}` : ""}${r.tags.length ? ` [${r.tags.join(", ")}]` : ""}${r.note ? `: ${r.note}` : ""}`;
    const out = [`Eyetest report: Coupler ${__APP_VERSION__}, Painter engine ${version ?? "not present"}`, `Made ${new Date().toLocaleString()}; ${made.length} scenes judged`, ""];
    out.push(`Tally: ${VERDICTS.map((v) => `${v.words} ${made.filter((r) => r.verdict === v.id).length}`).join("; ")}`, "");
    for (const v of [...VERDICTS].reverse()) {
      const list = made.filter((r) => r.verdict === v.id);
      if (list.length === 0) continue;
      out.push(`${v.words} (${list.length}):`, ...list.map(line), "");
    }
    const byTag = TAGS.map((t) => [t, [...new Set(made.filter((r) => r.tags.includes(t)).map((r) => r.terrain))]] as const).filter(([, where]) => where.length > 0);
    if (byTag.length > 0) out.push("Where each problem came up:", ...byTag.map(([t, where]) => `  ${t}: ${where.join(", ")}`), "");
    return out.join("\n");
  };

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(reportText());
      setStatus("Report copied.");
    } catch {
      setStatus("Couldn't copy: use Save Report… instead.");
    }
  };
  const saveReport = async () => {
    const path = await save({ defaultPath: `coupler-eyetest-${new Date().toISOString().slice(0, 10)}.txt`, filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path) return;
    try {
      await invoke("export_app_testing_report", { path, report: reportText() });
      setStatus("Report saved.");
    } catch (e) {
      setStatus(String(e));
    }
  };
  const clear = () => {
    setReports({});
    setStatus("Reports cleared.");
  };

  const rustLabel = "A: Coupler's painter (Rust)";
  const engineLabel = `B: Painter engine ${version ?? ""}${depth === "true" ? " (the 256's drawing)" : ""}`;
  const shown = (which: "rust" | "engine") => (
    <div className="eyetest-pane" data-testid={`eyetest-${which}`}>
      <p>
        <strong>{which === "rust" ? rustLabel : engineLabel}</strong>
      </p>
      <Art drawn={which === "rust" ? rust : engine} />
      <p className="map-muted">{which === "engine" && engine?.says ? engine.says : `${terrain.replace(/_/g, " ")}${skyless ? "" : `, ${time}, ${weather}`}, ${style}`}</p>
    </div>
  );

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Eyetest"
      className="dialog-eyetest"
      actions={
        <>
          <button type="button" data-testid="eyetest-copy" onClick={() => void copy()}>
            Copy Report
          </button>
          <button type="button" data-testid="eyetest-save" onClick={() => void saveReport()}>
            Save Report…
          </button>
          <button type="button" data-testid="eyetest-clear" onClick={clear}>
            Clear Reports
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <div ref={body} onKeyDown={onKey} data-testid="eyetest">
        <div className="eyetest-controls">
          <label>
            Terrain{" "}
            <select data-testid="eyetest-terrain" value={terrain} onChange={(e) => setTerrain(e.currentTarget.value)}>
              {TERRAINS.map((t) => (
                <option key={t} value={t}>
                  {t.replace(/_/g, " ")}
                </option>
              ))}
            </select>
          </label>
          <label>
            Time{" "}
            <select data-testid="eyetest-time" value={time} disabled={skyless} onChange={(e) => setTime(e.currentTarget.value)}>
              {TIMES.map((t) => (
                <option key={t}>{t}</option>
              ))}
            </select>
          </label>
          <label>
            Weather{" "}
            <select data-testid="eyetest-weather" value={weather} disabled={skyless} onChange={(e) => setWeather(e.currentTarget.value)}>
              {WEATHERS.map((w) => (
                <option key={w}>{w}</option>
              ))}
            </select>
          </label>
          <label>
            Style{" "}
            <select data-testid="eyetest-style" value={style} onChange={(e) => setStyle(e.currentTarget.value as PictureStyle)}>
              <option value="fantasy">Fantasy</option>
              <option value="high">High fantasy</option>
            </select>
          </label>
          <label>
            Size{" "}
            <select data-testid="eyetest-size" value={size} onChange={(e) => setSize(e.currentTarget.value)}>
              {SIZES.map((s) => (
                <option key={s}>{s}</option>
              ))}
            </select>
          </label>
          <button type="button" data-testid="eyetest-look" onClick={() => setLook((l) => l + 1)}>
            Another look ({look + 1})
          </button>
        </div>
        <fieldset className="radio-group eyetest-depths" data-testid="eyetest-depths">
          <legend>Colors (the game's own: {PALETTE_WORDS[gamePalette()]}, left alone)</legend>
          {PALETTES.map((p) => (
            <label key={p.id} className="check-row">
              <input type="radio" name="eyetest-depth" data-testid={`eyetest-depth-${p.id}`} checked={depth === p.id} onChange={() => setDepth(p.id)} />
              <span>{p.name}</span>
            </label>
          ))}
        </fieldset>
        <div className="eyetest-pictures" data-testid="eyetest-pictures">
          {sideBySide ? (
            <>
              {shown("rust")}
              {shown("engine")}
            </>
          ) : (
            <div>
              <button type="button" data-testid="eyetest-flip" onClick={() => setFlip((f) => (f === "rust" ? "engine" : "rust"))}>
                Flip to {flip === "rust" ? "B" : "A"} (f)
              </button>
              {shown(flip)}
            </div>
          )}
        </div>
        <div className="eyetest-report" data-testid="eyetest-report">
          <p>
            {at < 0 ? "Not in the sweep" : `Scene ${at + 1} of ${SWEEP.length}`}; {done} judged at {PALETTE_NAMES[depth]}, {style}.{" "}
            {current?.verdict ? `This one: ${VERDICTS.find((v) => v.id === current.verdict)?.words}.` : "This one: not judged."}
          </p>
          <div className="eyetest-buttons">
            <button type="button" data-testid="eyetest-prev" onClick={() => go(-1)}>
              Previous (p)
            </button>
            {VERDICTS.map((v) => (
              <button key={v.id} type="button" className={current?.verdict === v.id ? "primary" : undefined} aria-pressed={current?.verdict === v.id} data-testid={`eyetest-verdict-${v.id}`} onClick={() => verdict(v.id)}>
                {v.label} ({v.key})
              </button>
            ))}
            <button type="button" data-testid="eyetest-next" onClick={() => go(1)}>
              Next (n)
            </button>
          </div>
          <div className="eyetest-buttons" role="group" aria-label="What is wrong">
            {TAGS.map((t) => (
              <button key={t} type="button" className={current?.tags.includes(t) ? "primary" : undefined} aria-pressed={!!current?.tags.includes(t)} data-testid={`eyetest-tag-${t.toLowerCase()}`} onClick={() => tag(t)}>
                {t}
              </button>
            ))}
          </div>
          <label className="eyetest-note">
            Note{" "}
            <input type="text" data-testid="eyetest-note" value={current?.note ?? ""} onChange={(e) => put({ note: e.currentTarget.value })} />
          </label>
          <label className="check-row">
            <input type="checkbox" data-testid="eyetest-advance" checked={advance} onChange={(e) => setAdvance(e.currentTarget.checked)} />
            <span>Go to the next scene after a verdict</span>
          </label>
          <p role="status" className="map-muted">
            {status}
          </p>
        </div>
      </div>
    </Dialog>
  );
}

const PALETTE_NAMES: Record<Palette, string> = { "16": "ANSI 16", "256": "ANSI 256", true: "ANSI TRUE" };

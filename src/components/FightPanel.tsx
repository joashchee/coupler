/**
 * **The Fight**: a face-off for the length of a fight, in every way to
 * play (over what a fight doesn't need: the room's picture, or Terminal's
 * Control Panel). It opens when Combat mode starts and shows the result
 * for a moment after (lib/fight.ts `useBout`).
 *
 * On a fixed grid of cells in the VGA font, as the picture is: the names
 * and health figures; the two health bars, the player's filling from the
 * left toward the middle, the foe's from the right (a loss drops a bar at
 * once and leaves a lighter strip that drains away, none with reduced
 * motion); the player's mana and moves, and across from them the fight's
 * clock, the range and the result; then the picture, painted by Rust
 * (src-tauri/src/fight_art.rs): the room with the two of them in it, each
 * with their mood and hurt; and under it each one's hurt in words.
 *
 * Words first (rule 10): one sentence says it all for a screen reader,
 * and the rows, the bars and the picture are hidden from it. No live
 * region: the fight talk (lib/fightTalk.ts) and the say key speak the
 * fight. It never takes the focus.
 */
import { useEffect, useRef, useState } from "react";
import * as mud from "../lib/mud";
import { clock, foeMoment, hurtWords, rangeWords, resultWords, yourMoment, type Bout } from "../lib/fight";
import type { PictureSettings } from "../lib/pictures";

interface FightPanelProps {
  bout: Bout;
  vitals: mud.Vitals | null;
  settings: PictureSettings;
  /** The room the fight is in, as its picture has it: painted again when it changes. */
  scene: mud.Scene | null;
}

/** How long a lost strip of a health bar stays before it drains away. */
const DRAIN_MS = 500;
/** The picture's rows (fight_art.rs `ROWS`). */
const PICTURE_ROWS = 12;

const reducedMotion = () => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

/** A health that lags behind a loss for `DRAIN_MS`, for the strip that drains away. */
function useLagging(percent: number | null): number | null {
  const [lag, setLag] = useState(percent);
  useEffect(() => {
    if (percent === null || lag === null || percent >= lag || reducedMotion()) {
      setLag(percent);
      return;
    }
    const timer = window.setTimeout(() => setLag(percent), DRAIN_MS);
    return () => window.clearTimeout(timer);
  }, [percent, lag]);
  return percent === null || lag === null ? percent : Math.max(lag, percent);
}

/** The cells of a bar `width` wide: full, the strip just lost, empty. */
function cells(percent: number | null, lagging: number | null, width: number): [number, number, number] {
  const of = (p: number | null) => (p === null ? 0 : Math.max(0, Math.min(width, Math.round((p * width) / 100))));
  const full = of(percent);
  const lost = Math.max(0, of(lagging) - full);
  return [full, lost, width - full - lost];
}

/** The level a health bar's color repeats (as Gauges.tsx's). */
const level = (percent: number | null) => (percent === null ? "" : percent >= 50 ? " gauge-good" : percent >= 25 ? " gauge-warn" : " gauge-low");

/** A health bar, filling from `from` toward the middle. */
function HealthBar({ percent, width, from, foe }: { percent: number | null; width: number; from: "left" | "right"; foe?: boolean }) {
  const lagging = useLagging(percent);
  const [full, lost, empty] = cells(percent, lagging, width);
  const parts = [
    <span key="full" className={foe ? "gauge-foe" : `gauge-bar${level(percent)}`}>
      {"█".repeat(full)}
    </span>,
    <span key="lost" className="gauge-ghost">
      {"▓".repeat(lost)}
    </span>,
    <span key="empty" className="fight-muted">
      {"░".repeat(empty)}
    </span>,
  ];
  return <>{from === "left" ? parts : parts.reverse()}</>;
}

/** A thin bar with its name and figures, `width` in all: "Mana  ▄▄▄▄▄▄▄▄   30/40". */
function ThinBar({ name, now, most, width }: { name: string; now: number | null; most: number | null; width: number }) {
  const figures = now === null ? "" : most === null ? `${now}` : `${now}/${most}`;
  const bar = Math.max(0, width - 6 - figures.length - 1);
  const percent = mud.percentOf(now, most);
  const full = percent === null ? 0 : Math.max(0, Math.min(bar, Math.round((percent * bar) / 100)));
  return (
    <>
      <span className="fight-muted">{name.padEnd(6).slice(0, 6)}</span>
      <span className="gauge-bar gauge-thin">{"▄".repeat(full)}</span>
      <span className="fight-muted">{"▄".repeat(bar - full)}</span>
      <span>{` ${figures}`.padEnd(width - 6 - bar)}</span>
    </>
  );
}

/** Text fitted to `width` cells: cut, then padded on the side away from `align`. */
const fit = (text: string, width: number, align: "left" | "right" = "left") => {
  const cut = text.length > width ? text.slice(0, width) : text;
  return align === "left" ? cut.padEnd(width) : cut.padStart(width);
};

/** The player's health in figures, or the foe's: "37 of 90", "41%". */
const yourFigures = (v: mud.Vitals | null) => (v?.hp == null ? "" : v.maxHp == null ? `${v.hp}` : `${v.hp} of ${v.maxHp}`);
const foeFigures = (o: mud.Opponent) => (o.health !== null && o.healthMax !== null ? `${o.health} of ${o.healthMax}` : o.percent !== null ? `${o.percent}%` : "");

export function FightPanel({ bout, vitals, settings, scene }: FightPanelProps) {
  const boxRef = useRef<HTMLDivElement>(null);
  const [columns, setColumns] = useState(0);
  const [art, setArt] = useState<mud.AnsiArt | null>(null);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    // offsetWidth is in stage pixels: the stage's scaling doesn't change it.
    const measure = () => setColumns(Math.floor(box.offsetWidth / 8));
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(box);
    return () => watch.disconnect();
  }, []);

  // The clock runs until the end.
  useEffect(() => {
    if (bout.ended) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [bout.ended]);

  const o = bout.opponent;
  const foe = o.name ?? "someone";
  const mine = mud.percentOf(vitals?.hp ?? null, vitals?.maxHp ?? null);
  const you = yourMoment(bout, mine);
  const theirs = foeMoment(bout);
  const painting = settings.engine !== "none";
  const sceneKey = scene ? `${scene.world}/${scene.room}/${scene.time}/${scene.weather}/${scene.look}` : "";

  // Painted again only when what it shows changes: a step of hurt, a mood, the room, the width.
  useEffect(() => {
    if (!painting || columns < 1) return;
    let stale = false;
    mud
      .fightPaint(foe, you, theirs, settings.style, columns)
      .then((painted) => !stale && setArt(painted))
      .catch(() => !stale && setArt(null));
    return () => {
      stale = true;
    };
    // The moments by their parts, so a new object with the same look doesn't paint again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [painting, foe, you.mood, you.hurt, theirs.mood, theirs.hurt, settings.style, columns, sceneKey]);

  const half = Math.floor(columns / 2);
  const right = columns - half;
  const foeName = foe.charAt(0).toUpperCase() + foe.slice(1);
  const elapsed = clock((bout.ended?.at ?? now) - bout.started);
  const range = rangeWords(o.range);
  const result = resultWords(bout);
  const yourHurt = hurtWords(you.hurt);
  const theirHurt = theirs.mood === "dead" ? "dead" : hurtWords(theirs.hurt);
  const words =
    `You, ${yourFigures(vitals) ? `${yourFigures(vitals)} health, ` : ""}${yourHurt}. ` +
    `${foeName}, ${foeFigures(o) ? `${foeFigures(o)}, ` : ""}${theirHurt}. ` +
    `${elapsed}${range ? `, ${range}` : ""}.${result ? ` ${result}` : ""}`;

  return (
    <section className="fight-panel panel" aria-labelledby="fight-title" data-testid="fight-panel">
      <h2 id="fight-title">The Fight</h2>
      <p className="sr-only" data-testid="fight-words">
        {words}
      </p>
      <div ref={boxRef} className="fight-body" aria-hidden="true">
        {columns > 0 && (
          <>
            <p className="fight-row">
              {fit(`You ${yourFigures(vitals)}`, half)}
              {fit(`${foeFigures(o)} ${foeName}`, right, "right")}
            </p>
            <p className="fight-row">
              <HealthBar percent={mine} width={half - 1} from="left" />
              {"  "}
              <HealthBar percent={o.percent} width={right - 1} from="right" foe />
            </p>
            <p className="fight-row">
              <ThinBar name="Mana" now={vitals?.mana ?? null} most={vitals?.maxMana ?? null} width={half} />
              {fit(`${range ? `${range}  ` : ""}${elapsed}`, right, "right")}
            </p>
            <p className="fight-row">
              <ThinBar name="Moves" now={vitals?.moves ?? null} most={vitals?.maxMoves ?? null} width={half} />
              <span className={result?.startsWith("KO") ? "fight-ko" : undefined}>{fit(result ?? "", right, "right")}</span>
            </p>
            <div className="fight-picture" style={{ height: PICTURE_ROWS * 16 }}>
              {painting &&
                art?.lines.map((line, row) => (
                  <div key={row} className="fight-picture-row">
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
            <p className="fight-row">
              {fit(`You, ${yourHurt}`, half)}
              {fit(`${foeName}, ${theirHurt}`, right, "right")}
            </p>
          </>
        )}
      </div>
    </section>
  );
}

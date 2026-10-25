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
 * with their mood and hurt, the player with what's on them; under it each
 * one's hurt in words; then what's on the player and their cooldowns,
 * each with its time left, and whom the foe is hitting; then a row for
 * each ally in the group, their health a bar and figures, mana and moves
 * (src-tauri/src/effects.rs).
 *
 * Its moments (src-tauri/src/fight_art.rs `frames`): a big blow jolts
 * the face hit a cell back and flashes it once (no flash within a third
 * of a second of the last); a new foe slides in from the right. With
 * reduced motion, the flash alone.
 *
 * On the phone, a strip above the touch pad in the Game section
 * (`strip`): eight rows, the picture painted small (fight_art.rs
 * `Size::Small`: the room, the two faces drawn at 16 by 8 cells at its
 * two ends, their moments as on the panel), and between the faces, in a
 * band of its own, the player's name, figures, health bar, mana and moves,
 * the clock or the result, what's on the player (scrolling when it's too
 * long), the foe's hurt, health bar, figures and name. With pictures off
 * (gear → Pictures…), four rows of words across the strip instead:
 * names and figures with the clock between, the two health bars, mana
 * and moves, and what's on the player.
 *
 * Words first (rule 10): one sentence says it all for a screen reader,
 * and the rows, the bars and the picture are hidden from it. No live
 * region: the fight talk (lib/fightTalk.ts) and the say key speak the
 * fight. It never takes the focus. Once it's over, Combat Summary… opens
 * the fight's page (components/CombatSummaryDialog.tsx).
 */
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import * as mud from "../lib/mud";
import { usePalette } from "../lib/palette";
import { bigBlow, BLOW_MS, blowFrames, clock, effectEntries, FLASH_GAP_MS, foeMoment, hurtWords, rangeWords, resultWords, SLIDE_MS, slideFrames, SMALL_FACE, yourMoment, type Bout } from "../lib/fight";
import type { PictureSettings } from "../lib/pictures";

interface FightPanelProps {
  bout: Bout;
  vitals: mud.Vitals | null;
  /** What's on the player, their cooldowns and group. */
  status: mud.Status | null;
  settings: PictureSettings;
  /** The room the fight is in, as its picture has it: painted again when it changes. */
  scene: mud.Scene | null;
  /** Opens Combat Summary on this fight, once it's over. */
  onSummary: () => void;
  /** The phone's strip: the picture small with the words between the faces (or four rows of words with pictures off), no button. */
  strip?: boolean;
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

/** An ally's row: "Bob ◄  ████░░ 37/90   M 30/40  V 80/90". */
function AllyRow({ ally, tank, half, right }: { ally: mud.Ally; tank: boolean; half: number; right: number }) {
  const name = fit(`${ally.name}${tank ? " ◄" : ""}`, 12);
  const figures = ally.hp === null ? "" : ally.maxHp === null ? `${ally.hp}` : `${ally.hp}/${ally.maxHp}`;
  const bar = Math.max(4, half - 12 - figures.length - 2);
  const pair = (letter: string, now: number | null, most: number | null) => (now === null ? "" : most === null ? `${letter} ${now}` : `${letter} ${now}/${most}`);
  return (
    <p className="fight-row">
      {name}
      <HealthBar percent={mud.percentOf(ally.hp, ally.maxHp)} width={bar} from="left" />
      {fit(` ${figures}`, half - 12 - bar)}
      {fit([pair("M", ally.mana, ally.maxMana), pair("V", ally.moves, ally.maxMoves)].filter(Boolean).join("  "), right, "right")}
    </p>
  );
}

/** The picture's rows of cells. */
function Picture({ art, rows = PICTURE_ROWS }: { art: mud.AnsiArt | null; rows?: number }) {
  return (
    <div className="fight-picture" style={{ height: rows * 16 }}>
      {art?.lines.map((line, row) => (
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
  );
}

/** A row of text `width` cells wide that scrolls by itself, round and round, when it's longer; with reduced motion it stays put. */
function Scrolling({ text, width }: { text: string; width: number }) {
  if (text.length <= width || reducedMotion()) return <>{fit(text, width)}</>;
  const lap = `${text}   `;
  return (
    <span className="fight-scroll" style={{ width: width * 8 }}>
      <span className="fight-scroll-text" style={{ "--lap": `${lap.length * 8}px`, animationDuration: `${lap.length * 0.25}s` } as CSSProperties}>
        {lap + lap}
      </span>
    </span>
  );
}

export function FightPanel({ bout, vitals, status, settings, scene, onSummary, strip = false }: FightPanelProps) {
  /** The game's colors: painted again when they change. */
  const colors = usePalette();
  const boxRef = useRef<HTMLDivElement>(null);
  const [columns, setColumns] = useState(0);
  const [art, setArt] = useState<mud.AnsiArt | null>(null);
  /** A moment's frame showing over the still picture, while one plays. */
  const [frame, setFrame] = useState<mud.AnsiArt | null>(null);
  const playing = useRef<number[]>([]);
  const [now, setNow] = useState(Date.now());

  /** Plays `frames`, `ms` each, then the still picture again; a new moment cuts the one playing. */
  const play = useCallback((frames: mud.AnsiArt[], ms: number) => {
    playing.current.forEach((t) => window.clearTimeout(t));
    playing.current = frames.map((f, i) => window.setTimeout(() => setFrame(f), i * ms));
    playing.current.push(window.setTimeout(() => setFrame(null), frames.length * ms));
  }, []);
  useEffect(() => () => playing.current.forEach((t) => window.clearTimeout(t)), []);

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
  const on = status?.effects.map((e) => e.name) ?? [];
  const you = yourMoment(bout, mine, on);
  const theirs = foeMoment(bout);
  const painting = settings.engine !== "none";
  const sceneKey = scene ? `${scene.world}/${scene.room}/${scene.time}/${scene.weather}/${scene.look}` : "";

  // Painted again only when what it shows changes: a step of hurt, a mood, the room, the width.
  // A foe not yet painted slides in first, its frames painted in the same go.
  const paintedFoe = useRef<string | null>(null);
  useEffect(() => {
    if (!painting || columns < 1) return;
    let stale = false;
    mud
      .fightPaint(foe, you, theirs, settings.style, columns, paintedFoe.current === foe ? [] : slideFrames(reducedMotion(), strip), strip)
      .then((painted) => {
        if (stale) return;
        paintedFoe.current = foe;
        setArt(painted[painted.length - 1] ?? null);
        if (painted.length > 1) play(painted.slice(0, -1), SLIDE_MS);
      })
      .catch(() => !stale && setArt(null));
    return () => {
      stale = true;
    };
    // The moments by their parts, so a new object with the same look doesn't paint again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [painting, strip, foe, you.mood, you.hurt, on.join("|"), theirs.mood, theirs.hurt, settings.style, columns, sceneKey, colors]);

  // A big blow, to either: the one hit jolts and flashes.
  const healths = useRef({ mine, theirs: o.percent });
  const lastFlash = useRef(0);
  useEffect(() => {
    const was = healths.current;
    healths.current = { mine, theirs: o.percent };
    if (!painting || columns < 1 || bout.ended || paintedFoe.current !== foe) return;
    const [hitYou, hitFoe] = [bigBlow(was.mine, mine), bigBlow(was.theirs, o.percent)];
    if (!hitYou && !hitFoe) return;
    const flash = Date.now() - lastFlash.current >= FLASH_GAP_MS;
    const frames = blowFrames(hitYou, hitFoe, flash, reducedMotion());
    if (frames.length === 0) return;
    if (flash) lastFlash.current = Date.now();
    mud
      .fightPaint(foe, you, theirs, settings.style, columns, frames, strip)
      .then((painted) => play(painted, BLOW_MS))
      .catch(() => undefined);
    // Only a change of health is a blow.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mine, o.percent]);

  const half = Math.floor(columns / 2);
  const right = columns - half;
  const foeName = foe.charAt(0).toUpperCase() + foe.slice(1);
  const elapsed = clock((bout.ended?.at ?? now) - bout.started);
  const range = rangeWords(o.range);
  const result = resultWords(bout);
  const yourHurt = hurtWords(you.hurt);
  const theirHurt = theirs.mood === "dead" ? "dead" : hurtWords(theirs.hurt);
  const entries = effectEntries(status, now);
  const group = status?.group ?? null;
  const tank = group?.tank ? (group.tank === "you" ? "hitting you" : `hitting ${group.tank}`) : "";
  const words =
    `You, ${yourFigures(vitals) ? `${yourFigures(vitals)} health, ` : ""}${yourHurt}. ` +
    `${foeName}, ${foeFigures(o) ? `${foeFigures(o)}, ` : ""}${theirHurt}${tank ? `, ${tank}` : ""}. ` +
    `${elapsed}${range ? `, ${range}` : ""}.${result ? ` ${result}` : ""}` +
    (entries.length > 0 ? ` On you: ${entries.join(", ").replace(/~/g, "about ")}.` : "") +
    (group ? ` Your group: ${group.allies.map((a) => `${a.name}${a.hp !== null && a.maxHp !== null ? ` ${a.hp} of ${a.maxHp}` : ""}`).join(", ")}.` : "");

  if (strip) {
    const middle = result ?? `${range ? `${range} ` : ""}${elapsed}`;
    const left = Math.floor((columns - middle.length) / 2);
    const short = (now: number | null | undefined, most: number | null | undefined) => (now == null ? "" : most == null ? `${now}` : `${now}/${most}`);
    const yours = short(vitals?.hp, vitals?.maxHp);
    const theirFigures = o.health !== null && o.healthMax !== null ? `${o.health}/${o.healthMax}` : o.percent !== null ? `${o.percent}%` : "";
    const onYou = [tank, ...entries].filter(Boolean).join("  ");
    if (painting) {
      // Between the two faces: a band of words, a row each.
      const mid = Math.max(0, columns - 2 * SMALL_FACE.columns);
      const ends = (a: string, b: string) => (a.length + 1 + b.length > mid ? fit(`${a} ${b}`, mid) : a + b.padStart(mid - a.length));
      const mana = short(vitals?.mana, vitals?.maxMana);
      const moves = short(vitals?.moves, vitals?.maxMoves);
      const centre = middle.length >= mid ? fit(middle, mid) : " ".repeat(Math.floor((mid - middle.length) / 2)) + middle;
      return (
        <section className="fight-panel fight-strip" style={{ height: SMALL_FACE.rows * 16 }} aria-labelledby="fight-title" data-testid="fight-panel">
          <h2 id="fight-title" className="sr-only">
            The Fight
          </h2>
          <p className="sr-only" data-testid="fight-words">
            {words}
          </p>
          <div ref={boxRef} className="fight-body fight-strip-painted" aria-hidden="true">
            {columns > 0 && (
              <>
                <Picture art={frame ?? art} rows={SMALL_FACE.rows} />
                <div className="fight-strip-middle" style={{ left: SMALL_FACE.columns * 8, width: mid * 8 }}>
                  <p className="fight-row">{ends("You", yours)}</p>
                  <p className="fight-row">
                    <HealthBar percent={mine} width={mid} from="left" />
                  </p>
                  <p className="fight-row">{fit([mana && `M ${mana}`, moves && `V ${moves}`].filter(Boolean).join(" "), mid)}</p>
                  <p className="fight-row">
                    <span className={result?.startsWith("KO") ? "fight-ko" : undefined}>{fit(centre, mid)}</span>
                  </p>
                  <p className="fight-row" data-testid="fight-effects">
                    {onYou ? <Scrolling text={onYou} width={mid} /> : <span className="fight-muted">{fit(yourHurt, mid)}</span>}
                  </p>
                  <p className="fight-row fight-muted">{fit(theirHurt, mid, "right")}</p>
                  <p className="fight-row">
                    <HealthBar percent={o.percent} width={mid} from="right" foe />
                  </p>
                  <p className="fight-row">{ends(theirFigures, foeName)}</p>
                </div>
              </>
            )}
          </div>
        </section>
      );
    }
    return (
      <section className="fight-panel fight-strip" style={{ height: 4 * 16 }} aria-labelledby="fight-title" data-testid="fight-panel">
        <h2 id="fight-title" className="sr-only">
          The Fight
        </h2>
        <p className="sr-only" data-testid="fight-words">
          {words}
        </p>
        <div ref={boxRef} className="fight-body" aria-hidden="true">
          {columns > 0 && (
            <>
              <p className="fight-row">
                {fit(`You ${yours}`, left)}
                <span className={result?.startsWith("KO") ? "fight-ko" : undefined}>{middle}</span>
                {fit(`${theirFigures} ${foeName}`, columns - left - middle.length, "right")}
              </p>
              <p className="fight-row">
                <HealthBar percent={mine} width={half - 1} from="left" />
                {"  "}
                <HealthBar percent={o.percent} width={right - 1} from="right" foe />
              </p>
              <p className="fight-row">
                <ThinBar name="Mana" now={vitals?.mana ?? null} most={vitals?.maxMana ?? null} width={half} />
                <ThinBar name="Moves" now={vitals?.moves ?? null} most={vitals?.maxMoves ?? null} width={right} />
              </p>
              <p className="fight-row" data-testid="fight-effects">
                {onYou ? <Scrolling text={onYou} width={columns} /> : <span className="fight-muted">{fit(`You, ${yourHurt}. ${foeName}, ${theirHurt}.`, columns)}</span>}
              </p>
            </>
          )}
        </div>
      </section>
    );
  }

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
            <Picture art={painting ? (frame ?? art) : null} />
            <p className="fight-row">
              {fit(`You, ${yourHurt}`, half)}
              {fit(`${foeName}, ${theirHurt}`, right, "right")}
            </p>
            <p className="fight-row" data-testid="fight-effects">
              <span className="fight-muted">{entries.length > 0 ? "On you " : ""}</span>
              {fit(entries.join("  "), half - (entries.length > 0 ? 7 : 0))}
              {fit(tank, right, "right")}
            </p>
            {group?.allies.map((ally) => (
              <AllyRow key={ally.name} ally={ally} tank={group.tank !== null && group.tank.toLowerCase() === ally.name.toLowerCase()} half={half} right={right} />
            ))}
          </>
        )}
      </div>
      {bout.ended && (
        <button type="button" className="fight-summary" data-testid="fight-summary" onClick={onSummary}>
          Combat Summary…
        </button>
      )}
    </section>
  );
}

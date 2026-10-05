/**
 * The game output: CoffeeMUD's lines in its own colors, a capped
 * scrollback, following the bottom unless the user has scrolled up to
 * read. It also measures itself in character cells, for NAWS.
 *
 * In review mode (Option+Up and Down, App.tsx) it holds still on the
 * line being read, marked in the gutter, and follows the bottom again
 * once the player is back to live. Meanwhile its bottom rows show the
 * live output under a rule (the split view), so a sighted player sees
 * what's coming in without losing their place; the game is told the
 * same size throughout, and screen readers aren't given it twice.
 */
import { memo, useEffect, useLayoutEffect, useRef, type CSSProperties, type ReactNode } from "react";
import type { GameColors } from "../lib/display";
import { spanColors, type Line } from "../lib/mud";

export interface TermLine {
  id: number;
  /** "sent": a command the user typed, echoed locally. "note": Coupler's own message. */
  kind?: "sent" | "note";
  /** A line of talk (src-tauri/src/speech.rs): it's in the journal or the log. */
  talk?: boolean;
  /** A line about the time of day (src-tauri/src/speech.rs): Immersive's narrator says it. */
  time?: boolean;
  /** A command's one-line answer (src-tauri/src/echo.rs): Immersive's narrator says it in short. */
  echo?: boolean;
  /** The prompt, finished by what came after it (src-tauri/src/speech.rs). */
  prompt?: boolean;
  line: Line;
}

/** A line Immersive speaks instead of writing: talk, and the time of day. */
export const spokenAside = (l: TermLine) => Boolean(l.talk || l.time);

/**
 * Whether an unfinished line is the player's prompt (`<100hp 50m 80mv>`),
 * not a question waiting for an answer ("Quit (y/N)?", "<pause - enter>",
 * "Choose one:"), which always shows.
 */
export function playerPrompt(text: string): boolean {
  const t = text.trim();
  return t !== "" && !/[?:]$/.test(t) && !/\b(y\/n|pause|enter|press|return)\b/i.test(t);
}

/** What Immersive leaves out of the game output (`talk`: talk and the time of day; `prompt`: the player's prompt; `echo`: the commands' one-line answers). */
export interface Hidden {
  talk: boolean;
  prompt: boolean;
  echo: boolean;
}

export const lineText = (line: Line) => line.map((span) => span.text).join("");

/** Whether a line is left out of the game output. */
export const hiddenLine = (l: TermLine, hide: Hidden) => (hide.talk && spokenAside(l)) || (hide.echo && !!l.echo) || (hide.prompt && !!l.prompt && playerPrompt(lineText(l.line)));

interface TerminalProps {
  lines: TermLine[];
  partial: Line | null;
  /** Shown while there's nothing to show yet. */
  empty: ReactNode;
  onSize: (columns: number, rows: number) => void;
  /** How many rows tall: 25, the classic terminal, unless Terminal mode makes it the screen's height. */
  rows?: number;
  /**
   * What to leave out (Immersive): talk and the time of day (talk is in
   * the journal and the log and shown while it's said, and the narrator
   * says the time), and the player's prompt (the cues and the say keys
   * stand in for it).
   */
  hide?: Hidden;
  /** The line review mode is on (its id), or null when live. */
  reviewing?: number | null;
  /** The game's colors, made readable, or none (lib/display.ts). */
  colors?: GameColors;
}

const LineView = memo(function LineView({ line, kind, current, colors }: { line: Line; kind?: TermLine["kind"]; current?: boolean; colors: GameColors }) {
  return (
    <div className={`terminal-line${kind ? ` ${kind}` : ""}${current ? " reviewed" : ""}`} aria-current={current ? "true" : undefined}>
      {line.map((span, i) => {
        if (kind) return <span key={i}>{span.text}</span>;
        const { color, background } = spanColors(span, colors);
        return (
          <span
            key={i}
            style={{
              color,
              background,
              fontStyle: span.italic ? "italic" : undefined,
              textDecoration: span.underline ? "underline" : undefined,
            }}
          >
            {span.text}
          </span>
        );
      })}
    </div>
  );
});

const NOTHING_HIDDEN: Hidden = { talk: false, prompt: false, echo: false };

/** The live lines shown under the held view while reviewing: at most this many, and a third of the rows. */
const LIVE = 5;

export function Terminal({ lines, partial, empty, onSize, rows = 25, hide = NOTHING_HIDDEN, reviewing = null, colors = "game" }: TerminalProps) {
  const ref = useRef<HTMLDivElement>(null);
  const measure = useRef<HTMLSpanElement>(null);
  const atBottom = useRef(true);

  // Follow new output only while the user is at the bottom and not reviewing.
  useLayoutEffect(() => {
    const el = ref.current;
    if (el && atBottom.current && reviewing === null) el.scrollTop = el.scrollHeight;
  }, [lines, partial, reviewing]);

  // Review mode keeps the line being read in view; back to live, the bottom.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (reviewing === null) {
      atBottom.current = true;
      el.scrollTop = el.scrollHeight;
    } else {
      el.querySelector(".terminal-line.reviewed")?.scrollIntoView({ block: "nearest" });
    }
  }, [reviewing]);

  useEffect(() => {
    const el = ref.current;
    const probe = measure.current;
    if (!el || !probe) return;
    let last = "";
    const report = () => {
      const probed = probe.getBoundingClientRect();
      // In full screen the stage is scaled, and these rectangles with it; the sizes below aren't.
      const scale = el.offsetWidth > 0 ? el.getBoundingClientRect().width / el.offsetWidth : 1;
      const cell = { width: probed.width / scale, height: probed.height / scale };
      const style = getComputedStyle(el);
      const width = el.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
      const height = el.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
      if (cell.width === 0 || cell.height === 0) return;
      const columns = Math.max(20, Math.floor(width / (cell.width / 10)));
      const rows = Math.max(5, Math.floor(height / cell.height));
      const key = `${columns}x${rows}`;
      if (key !== last) {
        last = key;
        onSize(columns, rows);
      }
    };
    const observer = new ResizeObserver(report);
    observer.observe(el);
    report();
    return () => observer.disconnect();
  }, [onSize]);

  const shown = hide.talk || hide.prompt || hide.echo ? lines.filter((l) => !hiddenLine(l, hide)) : lines;
  const unfinished = partial && hide.prompt && playerPrompt(lineText(partial)) ? null : partial;
  const live = Math.max(1, Math.min(LIVE, Math.floor(rows / 3)));

  return (
    <div className="terminal-wrap">
    <div
      ref={ref}
      className={`terminal${reviewing !== null ? " terminal-held" : ""}`}
      data-testid="terminal"
      role="log"
      // Not read as it comes: App gives the screen reader its lines by
      // kind (lib/speech.ts), through a hidden live region of its own.
      aria-live="off"
      style={{ "--rows": rows, "--live-rows": live + 1 } as CSSProperties}
      aria-label="Game output"
      // Focusable, so the keyboard can scroll back through it.
      tabIndex={0}
      onScroll={(e) => {
        const el = e.currentTarget;
        atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
    >
      <span ref={measure} className="terminal-measure" aria-hidden="true">
        MMMMMMMMMM
      </span>
      {lines.length === 0 && !unfinished && <div className="terminal-empty">{empty}</div>}
      {shown.map((l) => (
        <LineView key={l.id} line={l.line} kind={l.kind} current={l.id === reviewing} colors={colors} />
      ))}
      {unfinished && <LineView line={unfinished} colors={colors} />}
      {/* Room under the live lines, so the newest held line can still be read above them. */}
      {reviewing !== null && <div className="terminal-spacer" aria-hidden="true" />}
    </div>
      {reviewing !== null && (
        <div className="terminal terminal-live" style={{ "--rows": live + 1 } as CSSProperties} aria-hidden="true" data-testid="terminal-live">
          <div className="terminal-line terminal-live-rule">{"── Live (Option+End) ".padEnd(80, "─")}</div>
          {(unfinished ? shown.slice(-(live - 1)) : shown.slice(-live)).map((l) => (
            <LineView key={l.id} line={l.line} kind={l.kind} colors={colors} />
          ))}
          {unfinished && <LineView line={unfinished} colors={colors} />}
        </div>
      )}
    </div>
  );
}

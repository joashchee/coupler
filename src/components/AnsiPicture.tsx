/**
 * ANSI art on the screen, for a trigger's picture: the rows Rust drew
 * (src-tauri/src/ansi_art.rs) in the game's palette, one 8 by 16 cell a
 * character in the IBM VGA font whatever the theme, on black, as ANSI
 * art is meant to be seen. Clicking it hides it.
 *
 * An ANSImation (`art.play`) plays once from a blank picture, its steps
 * at the times Rust gave (a modem's pace, flashes already held to three
 * a second), then stays as the finished picture. With reduced motion
 * the finished picture shows at once. Its size is the finished
 * picture's all along, so nothing around it moves.
 */
import * as keys from "../lib/keys";
import { useEffect, useState, type CSSProperties } from "react";
import { spanColors, type AnsiArt, type Line } from "../lib/mud";

const reducedMotion = () => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

/** Whether `art` plays: an ANSImation, and motion is welcome. */
const plays = (art: AnsiArt) => (art.play?.length ?? 0) > 0 && !reducedMotion();

/** The rows an ANSImation shows now; null once it's done (or for a still picture): the finished picture. */
function usePlaying(art: AnsiArt): Line[] | null {
  const [lines, setLines] = useState<Line[] | null>(() => (plays(art) ? [] : null));
  useEffect(() => {
    const steps = art.play ?? [];
    if (!plays(art)) {
      setLines(null);
      return;
    }
    let now: Line[] = [];
    setLines(now);
    const timers = steps.map((step) =>
      window.setTimeout(() => {
        now = step.clear ? [] : [...now];
        for (const [row, line] of step.rows) now[row] = line;
        setLines(now);
      }, step.at),
    );
    timers.push(window.setTimeout(() => setLines(null), steps[steps.length - 1].at));
    return () => timers.forEach((timer) => window.clearTimeout(timer));
  }, [art]);
  return lines;
}

interface AnsiPictureProps {
  art: AnsiArt;
  /** The file's name: what a screen reader is told it is. */
  name: string;
  style: CSSProperties;
  /** Extra classes: `hook-picture-fading` as it fades away. */
  className?: string;
  onClick: () => void;
}

export function AnsiPicture({ art, name, style, className, onClick }: AnsiPictureProps) {
  const playing = usePlaying(art);
  const lines = playing ? Array.from({ length: art.rows }, (_, row) => playing[row] ?? []) : art.lines;
  return (
    <div
      className={`hook-picture hook-ansi${className ? ` ${className}` : ""}`}
      data-testid="hook-picture"
      role="img"
      aria-label={name}
      title={keys.keys("Click to hide the picture (Cmd+Shift+S)")}
      style={{ ...style, width: art.columns * 8 }}
      onClick={onClick}
    >
      {lines.map((line, row) => (
        <div key={row} className="hook-ansi-row" aria-hidden="true">
          {line.map((span, i) => {
            const { color, background } = spanColors(span);
            return (
              <span key={i} style={{ color, background, textDecoration: span.underline ? "underline" : undefined }}>
                {span.text}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
}

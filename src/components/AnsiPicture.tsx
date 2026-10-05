/**
 * ANSI art on the screen, for a trigger's picture: the rows Rust drew
 * (src-tauri/src/ansi_art.rs) in the game's palette, one 8 by 16 cell a
 * character in the IBM VGA font whatever the theme, on black, as ANSI
 * art is meant to be seen. Clicking it hides it.
 */
import type { CSSProperties } from "react";
import { spanColors, type AnsiArt } from "../lib/mud";

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
  return (
    <div
      className={`hook-picture hook-ansi${className ? ` ${className}` : ""}`}
      data-testid="hook-picture"
      role="img"
      aria-label={name}
      title="Click to hide the picture (Cmd+Shift+S)"
      style={{ ...style, width: art.columns * 8 }}
      onClick={onClick}
    >
      {art.lines.map((line, row) => (
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

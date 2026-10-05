/**
 * The picture of the room the player is in, as wide as its box and
 * `rows` tall (or as tall as its box), painted by Rust
 * (src-tauri/src/painter.rs) in the game's palette, one 8 by 16 cell a
 * character in the IBM VGA font whatever the theme, as ANSI art is.
 *
 * The player's own ART trigger for the room (`scene.art`) wins over the
 * painting: an image fitted to the box, or ANSI art cut to it.
 *
 * Hidden from screen readers (rule 10): it says nothing the room's own
 * words don't, and it's never announced. A sighted player sees under it
 * what it shows and who made it.
 */
import { useEffect, useRef, useState } from "react";
import * as sound from "../lib/assets";
import * as mud from "../lib/mud";
import { sceneWords, type PictureSettings } from "../lib/pictures";

interface RoomPictureProps {
  scene: mud.Scene;
  settings: PictureSettings;
  /** Rows of characters; left out, the picture fills its box's height (less its caption). */
  rows?: number;
}

export function RoomPicture({ scene, settings, rows }: RoomPictureProps) {
  const boxRef = useRef<HTMLDivElement>(null);
  const captionRef = useRef<HTMLParagraphElement>(null);
  /** The box's size in cells. */
  const [cells, setCells] = useState<{ columns: number; rows: number } | null>(null);
  const [art, setArt] = useState<mud.AnsiArt | null>(null);
  /** The player's own picture for the room, when there is one and it's an image. */
  const [own, setOwn] = useState<string | null>(null);

  useEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    // offsetWidth is in stage pixels: the stage's scaling doesn't change it.
    const measure = () => {
      const caption = captionRef.current?.offsetHeight ?? 16;
      setCells({ columns: Math.floor(box.offsetWidth / 8), rows: rows ?? Math.floor((box.offsetHeight - caption) / 16) });
    };
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(box);
    return () => watch.disconnect();
  }, [rows]);

  useEffect(() => {
    if (!cells || cells.columns < 1 || cells.rows < 1) return;
    let stale = false;
    if (scene.art) {
      // The player's own: ANSI art is cut to the box, an image is fitted to it.
      sound
        .picture(scene.art)
        .then((shown) => {
          if (stale) return;
          if ("art" in shown) {
            setOwn(null);
            setArt({ ...shown.art, columns: Math.min(shown.art.columns, cells.columns), lines: shown.art.lines.slice(0, cells.rows) });
          } else {
            setArt(null);
            setOwn(shown.url);
          }
        })
        .catch(() => {
          if (stale) return;
          setArt(null);
          setOwn(null);
        });
    } else {
      setOwn(null);
      mud
        .picturePaint(scene, settings.style, cells.columns, cells.rows, scene.look)
        .then((painted) => !stale && setArt(painted))
        .catch(() => !stale && setArt(null));
    }
    return () => {
      stale = true;
    };
  }, [scene, settings.style, cells]);

  const caption = scene.art
    ? `${mud.assetTitle(scene.art)}, your picture for this room`
    : `${sceneWords(scene)}, by Coupler's painter${scene.look > 0 ? `, look ${scene.look + 1}` : ""}`;

  return (
    <div ref={boxRef} className={`room-picture${rows === undefined ? " room-picture-fill" : ""}`} aria-hidden="true" data-testid="room-picture">
      {own && cells && <img className="room-picture-image" src={own} alt="" style={{ height: cells.rows * 16 }} />}
      {art && !own && (
        <div className="room-picture-art" style={{ width: art.columns * 8 }}>
          {art.lines.map((line, row) => (
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
      )}
      <p ref={captionRef} className="room-picture-caption map-muted">
        {caption}
      </p>
    </div>
  );
}

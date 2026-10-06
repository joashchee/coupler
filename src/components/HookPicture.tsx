/**
 * A trigger's picture on the screen (App.tsx): an image, or ANSI art
 * (AnsiPicture.tsx). Clicking it closes it. With a fade (the trigger's
 * ART Fade, in seconds) it fades away once that's passed, and is gone;
 * the same trigger going off again starts the wait over (`shownAt`).
 * With 0 it stays until it's clicked away or replaced.
 */
import * as keys from "../lib/keys";
import { useEffect, useState, type CSSProperties } from "react";
import type { Shown } from "../lib/assets";
import { AnsiPicture } from "./AnsiPicture";

/** How long the fading takes, in ms: `.hook-picture-fading` in App.css. */
const FADING = 1000;

interface HookPictureProps {
  picture: Shown;
  /** The file's name: what a screen reader is told it is. */
  name: string;
  style: CSSProperties;
  /** Seconds before it fades away; 0 keeps it. */
  fade: number;
  /** Changes each time its trigger goes off again, so the wait starts over. */
  shownAt: number;
  onClose: () => void;
}

export function HookPicture({ picture, name, style, fade, shownAt, onClose }: HookPictureProps) {
  const [fading, setFading] = useState(false);

  useEffect(() => {
    setFading(false);
    if (fade <= 0) return;
    let gone: number | undefined;
    const wait = window.setTimeout(() => {
      setFading(true);
      gone = window.setTimeout(onClose, FADING);
    }, fade * 1000);
    return () => {
      window.clearTimeout(wait);
      window.clearTimeout(gone);
    };
    // `onClose` is new each render; only a new fade or showing restarts the wait.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fade, shownAt]);

  const className = fading ? "hook-picture-fading" : undefined;
  return "art" in picture ? (
    <AnsiPicture art={picture.art} name={name} style={style} className={className} onClick={onClose} />
  ) : (
    <img
      className={`hook-picture${className ? ` ${className}` : ""}`}
      data-testid="hook-picture"
      src={picture.url}
      alt={name}
      title={keys.keys("Click to close the picture (Cmd+Shift+S closes them all)")}
      style={style}
      onClick={onClose}
    />
  );
}

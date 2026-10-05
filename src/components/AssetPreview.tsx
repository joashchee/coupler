/**
 * Looking at an ART asset before a hook shows it: the Assets tab's
 * Show… (HooksDialog.tsx) opens it over the list, and Create Asset
 * (CreateAssetDialog.tsx) shows what it just painted. The picture sits
 * in a box 80 characters by 25 rows (fewer in Create) on black, and
 * scrolls inside it when bigger; its facts in words come first, since a
 * screen reader can't see it (it's one image named for the file).
 */
import { useEffect, useState } from "react";
import * as sound from "../lib/assets";
import * as mud from "../lib/mud";
import { Dialog } from "./Dialog";

/** A picture in a fixed box: ANSI art in its characters, or an image. */
export function ArtView({ picture, name, className }: { picture: sound.Shown; name: string; className?: string }) {
  return (
    <div className={`asset-preview${className ? ` ${className}` : ""}`} data-testid="asset-preview">
      {"art" in picture ? (
        <div className="hook-ansi" role="img" aria-label={name} style={{ width: picture.art.columns * 8 }}>
          {picture.art.lines.map((line, row) => (
            <div key={row} className="hook-ansi-row" aria-hidden="true">
              {line.map((span, i) => {
                const { color, background } = mud.spanColors(span);
                return (
                  <span key={i} style={{ color, background, textDecoration: span.underline ? "underline" : undefined }}>
                    {span.text}
                  </span>
                );
              })}
            </div>
          ))}
        </div>
      ) : (
        <img className="asset-preview-image" src={picture.url} alt={name} />
      )}
    </div>
  );
}

/** What a picture is, in words: its kind and size. */
export function pictureInWords(path: string, picture: sound.Shown | null): string {
  if (!picture) return "Reading the picture…";
  if ("art" in picture) return `ANSI art, ${picture.art.columns} characters wide and ${picture.art.rows} rows tall.`;
  return `A ${path.startsWith("png/") ? "PNG" : "JPEG"} picture.`;
}

interface AssetPreviewDialogProps {
  /** The ART asset to show, or null when closed. */
  path: string | null;
  onClose: () => void;
  onError: (message: string) => void;
}

export function AssetPreviewDialog({ path, onClose, onError }: AssetPreviewDialogProps) {
  const [picture, setPicture] = useState<sound.Shown | null>(null);
  useEffect(() => {
    setPicture(null);
    if (!path) return;
    let stale = false;
    sound
      .picture(path)
      .then((p) => !stale && setPicture(p))
      .catch((e) => {
        onError(String(e));
        onClose();
      });
    return () => {
      stale = true;
    };
    // A new path is a new picture; the callbacks are new each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);
  const title = path ? mud.assetTitle(path) : "";
  return (
    <Dialog open={path !== null} onClose={onClose} title={title || "Picture"} className="dialog-wide">
      <p data-testid="asset-preview-words">{path ? `${path}: ${pictureInWords(path, picture)}` : ""}</p>
      {path && picture && <ArtView picture={picture} name={title} />}
    </Dialog>
  );
}

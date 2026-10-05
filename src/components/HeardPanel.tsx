/**
 * Heard: everything Coupler's voice said (lib/voice.ts), newest at the
 * bottom, so nothing in Immersive mode is only ever spoken; and, with
 * sound captions on (lib/display.ts), every sound Coupler played, in
 * words (lib/captions.ts), so nothing is only ever heard. Not a live
 * region: the voice has already said it, and a screen reader saying it
 * again would double it.
 */
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { onCaption } from "../lib/captions";
import * as voice from "../lib/voice";

/** Lines kept. */
const KEPT = 200;

interface HeardPanelProps {
  /** Write the sound captions too. */
  captions: boolean;
}

export function HeardPanel({ captions }: HeardPanelProps) {
  const [said, setSaid] = useState<{ id: number; text: string; sound?: boolean }[]>([]);
  const next = useRef(0);
  const body = useRef<HTMLOListElement>(null);
  const add = (text: string, sound?: boolean) =>
    setSaid((list) => {
      const more = [...list, { id: next.current++, text, sound }];
      return more.length > KEPT ? more.slice(more.length - KEPT) : more;
    });
  useEffect(() => voice.onSaid((text) => add(text)), []);
  useEffect(() => (captions ? onCaption((text) => add(text, true)) : undefined), [captions]);
  useLayoutEffect(() => {
    const el = body.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [said]);

  return (
    <section className="heard-panel panel" aria-labelledby="heard-title" data-testid="heard-panel">
      <h2 id="heard-title">Heard</h2>
      <ol className="heard-body" ref={body} tabIndex={0} aria-label={captions ? "What Coupler's voice said, and the sounds it played" : "What Coupler's voice said"}>
        {said.length === 0 && (
          <li className="map-muted">{captions ? "What Coupler's voice says, and every sound it plays, is written here too." : "What Coupler's voice says is written here too."}</li>
        )}
        {said.map((s) => (
          <li key={s.id} className={s.sound ? "heard-caption" : undefined}>
            {s.text}
          </li>
        ))}
      </ol>
    </section>
  );
}

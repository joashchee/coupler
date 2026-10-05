/**
 * Immersive's pop-up: the line of talk being said, at the top of the
 * screen, titled with who's saying it. In Immersive, talk isn't written
 * in the game output (it's in the journal and the log); this shows it
 * for exactly as long as it's heard. It goes when the line ends or is
 * cut off (lib/voice.ts `onSpeaking`), or when it's clicked; the voice
 * goes on (Cmd+Period stops that).
 *
 * Hidden from screen readers: the voice is already saying it, and a
 * second reading would double it (rule 10, not too much speech). It's
 * never focused, so it can't take the keyboard away from the command
 * line, and vanishing can't strand focus.
 */
import { useEffect, useState } from "react";
import * as voice from "../lib/voice";

export function SpeakingPopup() {
  const [line, setLine] = useState<voice.Speaking | null>(null);
  useEffect(() => voice.onSpeaking(setLine), []);
  if (!line) return null;
  return (
    <section className={`panel speaking-popup ${line.book}`} aria-hidden="true" data-testid="speaking-popup" onClick={() => setLine(null)} title="Click to hide; the voice goes on">
      <h2>{line.speaker || (line.book === "log" ? "Log" : "Someone")}</h2>
      <p className="speaking-text">{line.text}</p>
    </section>
  );
}

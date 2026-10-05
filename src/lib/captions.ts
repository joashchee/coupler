/**
 * Sound captions: every sound Coupler plays, in words, as a film's
 * captions are ("♪ footstep", "♪ a fight starts"), for anyone who can't
 * hear it and for learning what each cue means by seeing it beside it.
 * lib/earcons.ts captions its cues, lib/assets.ts the hooks' sounds and
 * music; the Heard panel (components/HeardPanel.tsx) writes them when
 * the Display setting has them on.
 *
 * Never spoken and never a live region: the sound already said it, and a
 * screen reader saying it again would be more speech, not less (rule 10).
 */
const listeners = new Set<(text: string) => void>();

/** Says a sound in words to whoever's listening. */
export function caption(text: string) {
  for (const f of listeners) f(text);
}

export function onCaption(f: (text: string) => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

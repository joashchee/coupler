/**
 * Characters' Voices (gear menu): everyone met in the game, NPC or
 * player, each with the voice Coupler gave them from their name
 * (src-tauri/src/cast.rs), so in Immersive mode who's talking is heard
 * before what they say. The list is the world last connected to's, the
 * most recently met first, narrowed by Search; a long list is cut short
 * with the full count above it, as in the hooks list. It reads again
 * when someone new is met while it's open.
 *
 * Each row: the name, NPC or Player (Unknown when only heard, never
 * seen), the voice in short, and an Edit button that opens the voice
 * editor over the list (VoiceEditor.tsx). The list isn't a live region:
 * people arrive all the time while playing.
 *
 * Above the list, the narrator (lib/voice.ts): the voice of everything
 * Coupler says itself, who's talking before they talk among it. Its
 * Edit opens the same editor; it's kept on this Mac, for every world.
 */
import { memo, useCallback, useEffect, useId, useState } from "react";
import * as mud from "../lib/mud";
import * as voice from "../lib/voice";
import { Dialog } from "./Dialog";
import { tryVoice, VoiceEditor } from "./VoiceEditor";

interface CastDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

/** What `editing` holds while the narrator is edited: no character's key is empty. */
const NARRATOR = "";

/** Rows drawn at most; Search narrows the rest down. */
const SHOWN = 500;

export const WHO: Record<mud.CastMember["who"], string> = { npc: "NPC", pc: "Player", unknown: "Unknown" };

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** A voice in short: "Samantha, pitch 112%, speed 95%, set by you". */
export function voiceSummary(c: mud.CastMember): string {
  const chosen = voice.characterVoice(c.voice);
  const parts = [
    chosen ? (chosen.engine === "system" ? chosen.name : `${chosen.name} (built in)`) : c.voice.gender === "feminine" ? "Feminine" : "Masculine",
    `pitch ${c.voice.pitch}%`,
    `speed ${c.voice.rate}%`,
    c.voice.quiet && "quiet",
    c.edited && "set by you",
  ].filter(Boolean);
  return parts.join(", ");
}

export const CastDialog = memo(function CastDialog({ open, onClose, onStatus, onError }: CastDialogProps) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [listing, setListing] = useState<mud.CastListing | null>(null);
  /** The key of the character being edited, NARRATOR for the narrator. */
  const [editing, setEditing] = useState<string | null>(null);
  /** The system's voices arrive late: the summaries are drawn again. */
  const [, setVoices] = useState(0);
  useEffect(() => voice.onVoicesChanged(() => setVoices((n) => n + 1)), []);

  const read = useCallback(() => {
    mud
      .castList()
      .then(setListing)
      .catch((e) => onError(String(e)));
  }, [onError]);

  useEffect(() => {
    if (!open) {
      setEditing(null);
      return;
    }
    read();
    const unlisten = mud.onCastChanged(read);
    return () => void unlisten.then((f) => f());
  }, [open, read]);

  /** Puts a changed character in the list, where they were. */
  const replace = (c: mud.CastMember) => setListing((l) => l && { ...l, characters: l.characters.map((x) => (x.key === c.key ? c : x)) });

  const all = listing?.characters ?? [];
  const q = query.trim().toLowerCase();
  const matching = q === "" ? all : all.filter((c) => c.name.toLowerCase().includes(q) || WHO[c.who].toLowerCase().includes(q));
  const shown = matching.slice(0, SHOWN);
  const summary = !listing
    ? "Reading who you've met…"
    : listing.world === null
      ? "Connect to a game to see who you've met there."
      : all.length === 0
        ? "No one met yet. Characters are added as you meet them: in a room, or talking."
        : matching.length === 0
          ? `None of the ${plural(all.length, "character matches", "characters match")}.`
          : shown.length < matching.length
            ? `Showing the first ${shown.length} of ${matching.length}. Type in Search to narrow them down.`
            : q !== ""
              ? `${matching.length} of ${plural(all.length, "character", "characters")} match.`
              : `${plural(all.length, "character", "characters")} met, the most recent first.`;
  const [narrator, setNarrator] = useState(voice.narratorVoice);
  useEffect(() => voice.onNarratorChanged(setNarrator), []);
  const narratorEdited = narrator.engine !== null || narrator.pitch !== 100 || narrator.rate !== 100;
  /** The narrator as the editor sees a character. */
  const narratorMember: mud.CastMember = { key: NARRATOR, name: "Narrator", who: "unknown", firstMet: 0, lastMet: 0, lines: 0, voice: narrator, edited: narratorEdited };
  const narratorSummary = [voice.narratorName(), `pitch ${narrator.pitch}%`, `speed ${narrator.rate}%`].join(", ");
  const editingNarrator = open && editing === NARRATOR;
  const editingMember = editingNarrator ? narratorMember : open && editing ? (all.find((c) => c.key === editing) ?? null) : null;

  return (
    <>
      <Dialog open={open} onClose={onClose} title="Characters' Voices" className="dialog-wide dialog-hooks" covered={editingMember !== null}>
        <p>
          Everyone you've met in this game, people and players, each with a voice made from their name: feminine or masculine, lower for a giant,
          higher for a pixie, slower when old. In Immersive, what they say is spoken in their voice. Edit changes a voice; your changes are kept.
        </p>
        <div className="cast-row cast-narrator">
          <span className="hook-key">Narrator</span>
          <span className="hook-value">Coupler</span>
          <span className={narratorEdited ? "hook-sets-off" : "hook-value"}>{narratorSummary}</span>
          <button type="button" className="hook-edit" data-testid="narrator-try" aria-label="Try the narrator's voice" onClick={() => tryVoice(narratorMember, true)}>
            Try It
          </button>
          <button type="button" className="hook-edit" data-testid="narrator-edit" aria-label={`Edit the narrator's voice: ${narratorSummary}`} onClick={() => setEditing(NARRATOR)}>
            Edit…
          </button>
        </div>
        <div className="hooks-search">
          <label htmlFor={`${id}-search`}>Search</label>
          <input
            id={`${id}-search`}
            type="text"
            data-testid="cast-search"
            aria-describedby={`${id}-summary`}
            placeholder="Part of a name, or NPC or Player"
            value={query}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
        <p id={`${id}-summary`} className="hooks-summary" data-testid="cast-summary">
          {summary}
        </p>
        {/* The columns' names, for the eye; each control says its own. */}
        <div className="hooks-head cast-row" aria-hidden="true">
          <span>Name</span>
          <span>Who</span>
          <span>Voice</span>
          <span />
          <span />
        </div>
        <ul className="hooks-list cast-list" data-testid="cast-list" tabIndex={0} aria-label="Characters: name, NPC or player, then their voice">
          {open &&
            shown.map((c) => (
              <li key={c.key} className="cast-row">
                <span className="hook-key">{c.name}</span>
                <span className="hook-value">{WHO[c.who]}</span>
                <span className={c.edited ? "hook-sets-off" : "hook-value"}>{voiceSummary(c)}</span>
                <button type="button" className="hook-edit" data-testid="cast-try" aria-label={`Try ${c.name}'s voice`} onClick={() => tryVoice(c, false)}>
                  Try It
                </button>
                <button type="button" className="hook-edit" data-testid="cast-edit" aria-label={`Edit ${c.name}'s voice: ${voiceSummary(c)}`} onClick={() => setEditing(c.key)}>
                  Edit…
                </button>
              </li>
            ))}
        </ul>
      </Dialog>
      {/* Beside the list's dialog, not in it, as with the hook editor. */}
      <VoiceEditor
        member={editingMember}
        narrator={editingNarrator}
        onChange={async (change) => {
          if (!editingMember) return;
          if (editingNarrator) return voice.setNarratorVoice({ ...narrator, ...change });
          try {
            replace(await mud.castSetVoice(editingMember.key, { ...editingMember.voice, ...change }));
          } catch (e) {
            onError(String(e));
          }
        }}
        onReset={async () => {
          if (!editingMember) return;
          if (editingNarrator) {
            voice.setNarratorVoice(null);
            onStatus("The narrator speaks in the system's voice again.");
            return;
          }
          try {
            replace(await mud.castReset(editingMember.key));
            onStatus(`${editingMember.name}'s voice is made from their name again.`);
          } catch (e) {
            onError(String(e));
          }
        }}
        onForget={async () => {
          if (!editingMember) return;
          try {
            await mud.castForget(editingMember.key);
            setEditing(null);
            setListing((l) => l && { ...l, characters: l.characters.filter((x) => x.key !== editingMember.key) });
            onStatus(`Forgot ${editingMember.name}. Met again, they get a new voice.`);
          } catch (e) {
            onError(String(e));
          }
        }}
        onClose={() => setEditing(null)}
      />
    </>
  );
});

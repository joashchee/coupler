/**
 * Narrator's Answers (Workshop, gear menu): every one-line answer a
 * command can give (src-tauri/src/echo.rs), each row the command, the
 * line as the game writes it and what Immersive's narrator says instead,
 * then Edit, which opens the answer's editor over the list. A search
 * narrows the list down; the checkbox above it turns the whole thing
 * off (Immersive writes the answers again, and says nothing).
 *
 * The editor: the narrator's words (empty means Coupler's own), whether
 * it's said at all, Hear It (with made-up details for the `*`s) and Back
 * to Coupler's Own. Every change is at once and kept (lib/echoes.ts).
 */
import { useEffect, useId, useMemo, useState } from "react";
import * as echoes from "../lib/echoes";
import type { EchoInfo, EchoListing } from "../lib/mud";
import * as voice from "../lib/voice";
import { Dialog } from "./Dialog";

interface EchoesDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
}

/** The words that call an answer's commands: "SIT", "GET, DRAW", or "Any command". */
function calledBy(info: EchoInfo, listing: EchoListing): string {
  if (info.commands.length === 0) return "Any command";
  return info.commands.map((name) => listing.commands.find((c) => c.name === name)?.words[0] ?? name).join(", ");
}

/** What the narrator says for an answer now, in short. */
function saysNow(info: EchoInfo): string {
  const own = echoes.ownWords(info.id);
  if (own === undefined) return info.says;
  return own.trim() === "" ? "(nothing)" : own;
}

export function EchoesDialog({ open, onClose, onStatus }: EchoesDialogProps) {
  const id = useId();
  const [listing, setListing] = useState<EchoListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<EchoInfo | null>(null);
  const [, setVersion] = useState(0);
  useEffect(() => echoes.onEchoesChanged(() => setVersion((n) => n + 1)), []);
  useEffect(() => {
    if (!open) {
      setEditing(null);
      return;
    }
    echoes
      .echoListing()
      .then((l) => {
        setListing(l);
        setError(null);
      })
      .catch((e: unknown) => setError(`Couldn't list the answers. (${String(e)})`));
  }, [open]);

  const shown = useMemo(() => {
    if (!listing) return [];
    const q = query.trim().toLowerCase();
    if (q === "") return listing.echoes;
    return listing.echoes.filter((e) => [e.line, e.says, echoes.ownWords(e.id) ?? "", calledBy(e, listing)].some((t) => t.toLowerCase().includes(q)));
  }, [listing, query]);
  const changed = echoes.changedCount();
  const on = echoes.echoesOn();
  const total = listing?.echoes.length ?? 0;
  const summary = !listing
    ? (error ?? "Listing the answers…")
    : `${total} answers to ${listing.commands.length} commands${changed > 0 ? `, ${changed} changed by you` : ", all as Coupler says them"}.${shown.length < total ? ` Showing ${shown.length}.` : ""}`;

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        title="Narrator's Answers"
        className="dialog-wide"
        covered={open && editing !== null}
        actions={
          <>
            <button
              type="button"
              data-testid="echoes-reset"
              disabled={changed === 0}
              onClick={() => {
                echoes.resetAllWords();
                onStatus("Every answer is back as Coupler says it.");
              }}
            >
              Reset All Answers
            </button>
            <button type="button" className="primary" onClick={onClose}>
              Done
            </button>
          </>
        }
      >
        <p>
          Many commands answer in one line: SIT says "You sit down and take a rest." In Immersive, that line isn't written; the narrator says it in short instead
          ("Sitting."). Edit changes what the narrator says; your changes are kept. Any other one-line answer is said as the game wrote it.
        </p>
        <label className="check-row echoes-on">
          <input
            type="checkbox"
            data-testid="echoes-on"
            checked={on}
            onChange={(e) => {
              const now = e.currentTarget.checked;
              echoes.setEchoesOn(now);
              onStatus(now ? "Immersive's narrator says the commands' answers in short." : "Immersive writes the commands' answers again.");
            }}
          />
          <span>In Immersive, say these answers instead of writing them</span>
        </label>
        <div className="hooks-search">
          <label htmlFor={`${id}-search`}>Search</label>
          <input
            id={`${id}-search`}
            type="text"
            data-testid="echoes-search"
            aria-describedby={`${id}-summary`}
            placeholder="Part of a command, a line or the narrator's words"
            value={query}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
        <p id={`${id}-summary`} className="hooks-summary" data-testid="echoes-summary">
          {summary}
        </p>
        {/* The columns' names, for the eye; each Edit button says its row. */}
        <div className="hooks-head echoes-row" aria-hidden="true">
          <span>Command</span>
          <span>The game writes</span>
          <span>The narrator says</span>
          <span />
        </div>
        <ul className="hooks-list echoes-list" data-testid="echoes-list" tabIndex={0} aria-label="Answers: the command, what the game writes, then what the narrator says">
          {open &&
            listing &&
            shown.map((e) => {
              const says = saysNow(e);
              const mine = echoes.ownWords(e.id) !== undefined;
              const line = echoes.shownLine(e.line);
              return (
                <li key={e.id} className="echoes-row">
                  <span className="hook-value">{calledBy(e, listing)}</span>
                  <span className="hook-key">{line}</span>
                  <span className={mine ? "hook-sets-off" : "hook-value"}>
                    {echoes.shownLine(says.replace(/\{\d\}/g, "*"))}
                    {mine ? ", set by you" : ""}
                  </span>
                  <button type="button" className="hook-edit" data-testid={`echoes-edit-${e.id}`} aria-label={`Edit: ${line} The narrator says: ${says}`} onClick={() => setEditing(e)}>
                    Edit…
                  </button>
                </li>
              );
            })}
        </ul>
      </Dialog>
      <EchoEditor info={open ? editing : null} listing={listing} onClose={() => setEditing(null)} onStatus={onStatus} />
    </>
  );
}

interface EchoEditorProps {
  /** The answer being edited; null when closed. */
  info: EchoInfo | null;
  listing: EchoListing | null;
  onClose: () => void;
  onStatus: (message: string) => void;
}

function EchoEditor({ info, listing, onClose, onStatus }: EchoEditorProps) {
  const uid = useId();
  const [, setVersion] = useState(0);
  useEffect(() => echoes.onEchoesChanged(() => setVersion((n) => n + 1)), []);
  const answer = info?.id ?? null;
  const own = answer === null ? undefined : echoes.ownWords(answer);
  const silent = own !== undefined && own.trim() === "";
  // The words are typed freely (empty means Coupler's own), so the field
  // is set from what's kept only on opening and Back.
  const [words, setWordsField] = useState("");
  useEffect(() => setWordsField((answer === null ? undefined : echoes.ownWords(answer)) ?? ""), [answer]);
  const stars = info ? info.line.split("*").length - 1 : 0;
  const example = info ? echoes.fill(silent ? "" : words.trim() || info.says, echoes.EXAMPLES.slice(0, stars)) : "";

  return (
    <Dialog
      open={info !== null}
      onClose={onClose}
      title="Narrator's Answer"
      className="dialog-wide"
      actions={
        <>
          <button type="button" data-testid="echo-hear" disabled={silent || example.trim() === ""} onClick={() => voice.speak(example, true)}>
            Hear It
          </button>
          <button
            type="button"
            data-testid="echo-reset"
            disabled={own === undefined}
            onClick={() => {
              if (!info) return;
              echoes.setWords(info.id, null);
              setWordsField("");
              onStatus(`The narrator says "${info.says}" again.`);
            }}
          >
            Back to Coupler's Own
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      {info && listing && (
        <>
          <p>
            {calledBy(info, listing)} can answer: "{echoes.shownLine(info.line)}"
          </p>
          <p>
            {stars === 0
              ? "Write what the narrator says for it instead, or leave the box empty for Coupler's own words."
              : `Write what the narrator says for it instead, or leave the box empty for Coupler's own words. In your words, ${Array.from({ length: stars }, (_, i) => `{${i + 1}}`).join(" and ")} ${stars === 1 ? "is" : "are"} what the game wrote where the line has ${stars === 1 ? "…" : "each …, in order"}.`}
          </p>
          <div className="hook-edit-rows">
            <div className="hook-edit-row">
              <span>Said</span>
              <label className="hook-loop">
                <input
                  type="checkbox"
                  data-testid="echo-said"
                  checked={!silent}
                  onChange={(e) => {
                    const said = e.currentTarget.checked;
                    echoes.setWords(info.id, said ? (words.trim() === "" ? null : words) : "");
                  }}
                />
                <span>Aloud</span>
              </label>
              <span className="hook-value">Off, the answer is neither written nor said.</span>
            </div>
            <div className="hook-edit-row">
              <label htmlFor={`${uid}-words`}>The narrator says</label>
              <input
                id={`${uid}-words`}
                type="text"
                className="hook-asset"
                data-testid="echo-words"
                disabled={silent}
                value={words}
                placeholder={info.says}
                autoComplete="off"
                autoCorrect="off"
                spellCheck={false}
                onChange={(e) => {
                  const next = e.currentTarget.value;
                  setWordsField(next);
                  echoes.setWords(info.id, next.trim() === "" ? null : next);
                }}
              />
              <span className="hook-value">{silent ? "Says nothing." : `Sounds like: ${example}`}</span>
            </div>
          </div>
        </>
      )}
    </Dialog>
  );
}

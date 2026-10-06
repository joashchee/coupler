/**
 * Priority Audio (Workshop, gear menu): the cues and speech that never
 * wait behind the speech queue (lib/priority.ts). Each row is what's on
 * the list, its kind and Remove; under it, Add puts a cue, talk,
 * Coupler's own speech or a command's answer on it from one menu, and
 * Add Line the game's lines with the player's words. The bell rung
 * before priority audio cuts into speech is chosen here (Ring It plays
 * it); its volume, pitch and caption are in Cues, like any cue's. Every
 * change is at once and kept.
 */
import { useEffect, useId, useMemo, useState } from "react";
import * as earcons from "../lib/earcons";
import * as echoes from "../lib/echoes";
import type { EchoListing } from "../lib/mud";
import * as priority from "../lib/priority";
import { Dialog } from "./Dialog";

interface PriorityDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
}

const KINDS: Record<priority.PriorityEntry["kind"], string> = { cue: "Cue", speech: "Speech", answer: "Answer", line: "Game line" };

/** An entry in words: its name. */
function nameOf(e: priority.PriorityEntry, listing: EchoListing | null): string {
  switch (e.kind) {
    case "cue":
      return earcons.CUES.find((c) => c.id === e.id)?.name ?? e.id;
    case "speech":
      return priority.SPEECHES.find((s) => s.id === e.id)?.name ?? e.id;
    case "answer": {
      const info = listing?.echoes.find((a) => a.id === e.id);
      return info ? `"${echoes.shownLine(info.line)}"` : "A command's answer";
    }
    case "line":
      return `A line with "${e.text}"`;
  }
}

/** The menu's value for an entry, and back. */
const valueOf = (e: priority.PriorityEntry) => (e.kind === "line" ? "" : `${e.kind}:${e.id}`);
function entryOf(value: string): priority.PriorityEntry | null {
  const [kind, ...rest] = value.split(":");
  const id = rest.join(":");
  if (id === "") return null;
  if (kind === "cue") return { kind, id: id as earcons.CueId };
  if (kind === "speech") return { kind, id: id as priority.SpeechId };
  if (kind === "answer") return { kind, id };
  return null;
}

export function PriorityDialog({ open, onClose, onStatus }: PriorityDialogProps) {
  const id = useId();
  const [, setVersion] = useState(0);
  useEffect(() => priority.onPriorityChanged(() => setVersion((n) => n + 1)), []);
  const [listing, setListing] = useState<EchoListing | null>(null);
  const [chosen, setChosen] = useState("");
  const [words, setWords] = useState("");
  useEffect(() => {
    if (!open) return;
    echoes.echoListing().then(setListing, () => {
      // Not in Tauri: no answers to offer, the rest still is.
    });
  }, [open]);

  const list = priority.entries();
  const cues = earcons.CUES.filter((c) => !priority.NOT_PRIORITY.includes(c.id) && !priority.has({ kind: "cue", id: c.id }));
  const speeches = priority.SPEECHES.filter((s) => !priority.has({ kind: "speech", id: s.id }));
  const answers = useMemo(
    () => (listing?.echoes ?? []).filter((a) => !list.some((e) => e.kind === "answer" && e.id === a.id)),
    [listing, list],
  );
  const add = (e: priority.PriorityEntry) => {
    priority.setEntry(e, true);
    onStatus(`${nameOf(e, listing)} is Priority Audio now.`);
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Priority Audio"
      className="dialog-wide"
      actions={
        <>
          <button
            type="button"
            data-testid="priority-reset"
            onClick={() => {
              priority.resetPriority();
              onStatus("Priority Audio is back as Coupler starts it: tells, and a fight starting.");
            }}
          >
            Reset the List
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <p>
        What's on this list never waits its turn. If Coupler is saying something, it pauses, the bell rings, the priority audio plays, and then it carries on where
        it stopped. Speech needs Immersive's voice on, and cues its sound cues.
      </p>
      <div className="hooks-search">
        <label htmlFor={`${id}-bell`}>Bell</label>
        <select
          id={`${id}-bell`}
          className="hook-asset priority-pick"
          data-testid="priority-bell"
          value={priority.bellSound()}
          onChange={(e) => {
            const bell = e.currentTarget.value as priority.BellSound;
            priority.setBellSound(bell);
            earcons.priorityBell(bell);
          }}
        >
          {priority.BELLS.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        <button type="button" data-testid="priority-ring" onClick={() => earcons.priorityBell()}>
          Ring It
        </button>
      </div>
      <p className="hooks-summary">Its volume, pitch and caption are in Cues, as The priority bell.</p>
      <p id={`${id}-summary`} className="hooks-summary" data-testid="priority-summary">
        {list.length === 0 ? "Nothing is Priority Audio: everything waits its turn." : `${list.length} ${list.length === 1 ? "thing" : "things"} on the list.`}
      </p>
      <div className="hooks-head priority-row" aria-hidden="true">
        <span>What</span>
        <span>Kind</span>
        <span />
      </div>
      <ul className="hooks-list priority-list" data-testid="priority-list" aria-label="Priority Audio: what, then its kind">
        {list.map((e) => {
          const name = nameOf(e, listing);
          return (
            <li key={`${e.kind}:${e.kind === "line" ? e.text : e.id}`} className="priority-row">
              <span className="hook-key">{name}</span>
              <span className="hook-value">{KINDS[e.kind]}</span>
              <button
                type="button"
                className="hook-edit"
                aria-label={`Remove ${name}`}
                data-testid={`priority-remove-${e.kind === "line" ? e.text : e.id}`}
                onClick={() => {
                  priority.setEntry(e, false);
                  onStatus(`${name} waits its turn again.`);
                }}
              >
                Remove
              </button>
            </li>
          );
        })}
      </ul>
      <div className="hooks-search priority-add">
        <label htmlFor={`${id}-add`}>Add</label>
        <select id={`${id}-add`} className="hook-asset priority-pick" data-testid="priority-add" value={chosen} onChange={(e) => setChosen(e.currentTarget.value)}>
          <option value="">Choose…</option>
          <optgroup label="Speech">
            {speeches.map((s) => (
              <option key={s.id} value={valueOf({ kind: "speech", id: s.id })}>
                {`${s.name}: ${s.summary}`}
              </option>
            ))}
          </optgroup>
          <optgroup label="Cues">
            {cues.map((c) => (
              <option key={c.id} value={valueOf({ kind: "cue", id: c.id })}>
                {`${c.name} (${c.group})`}
              </option>
            ))}
          </optgroup>
          {answers.length > 0 && (
            <optgroup label="Commands' answers">
              {answers.map((a) => (
                <option key={a.id} value={valueOf({ kind: "answer", id: a.id })}>
                  {echoes.shownLine(a.line)}
                </option>
              ))}
            </optgroup>
          )}
        </select>
        <button
          type="button"
          data-testid="priority-add-button"
          disabled={entryOf(chosen) === null}
          onClick={() => {
            const e = entryOf(chosen);
            if (!e) return;
            add(e);
            setChosen("");
          }}
        >
          Add
        </button>
      </div>
      <form
        className="hooks-search priority-add"
        onSubmit={(ev) => {
          ev.preventDefault();
          if (words.trim() === "") return;
          add({ kind: "line", text: words });
          setWords("");
        }}
      >
        <label htmlFor={`${id}-line`}>A game line with</label>
        <input
          id={`${id}-line`}
          type="text"
          data-testid="priority-line"
          placeholder="Words in the line, such as: is DEAD"
          value={words}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          onChange={(e) => setWords(e.currentTarget.value)}
        />
        <button type="submit" data-testid="priority-line-add" disabled={words.trim() === ""}>
          Add Line
        </button>
      </form>
    </Dialog>
  );
}

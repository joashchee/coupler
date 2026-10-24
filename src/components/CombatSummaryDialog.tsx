/**
 * **Combat Summary** (gear menu, Cmd+Shift+Y, The Fight's result): every
 * fight of the character last played, kept by src-tauri/src/fight_log.rs,
 * the latest first; Show… opens one fight's page over the list. The
 * last 200 fights are kept whole; an older one is its overview.
 *
 * The page is one flat list of headings, no tabs, all in words made in
 * Rust: the overview, damage dealt and taken, mana and moves, the group,
 * health over time and the timeline, then the fights with the same foe
 * before. Health over time is also two charts of `█ ▄` cells, hidden from
 * screen readers (the line above each says it in figures). Copy as Text
 * puts the page on the clipboard, only when the player asks: other
 * players' names are in it (CLAUDE.md rule 1).
 *
 * The list isn't a live region: a fight kept while it's open is read in
 * quietly.
 */
import { memo, useCallback, useEffect, useState } from "react";
import * as mud from "../lib/mud";
import { clock } from "../lib/fight";
import { Dialog } from "./Dialog";

interface CombatSummaryDialogProps {
  open: boolean;
  /**
   * A fight to show at once, or null for the list. "next" (The Fight's
   * result) is the one just over: it's kept a few seconds after its end,
   * for the experience and the loot, and shows the moment it is.
   */
  showFirst: number | "next" | null;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** How it ended, in a few words. */
export function endingWords(o: mud.FightOverview): string {
  switch (o.ending) {
    case "killed":
      return "Won";
    case "fled":
      return "It fled";
    case "youFled":
      return "You fled";
    case "youDied":
      return "You died";
    default:
      return "Ended";
  }
}

/** When, short: "Oct 24, 14:02". */
const when = (ms: number) => {
  const d = new Date(ms);
  return `${d.toLocaleDateString(undefined, { month: "short", day: "numeric" })}, ${d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}`;
};

const foesOf = (o: mud.FightOverview) => (o.foes.length === 0 ? "Someone" : o.foes.join(", "));
const capital = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

/** Whether the fight just over may still be on its way: none kept in the last few seconds. */
const fightsWaiting = (all: mud.FightOverview[]) => all.length === 0 || Date.now() - (all[0].started + all[0].length) > 15_000;

/** The page's title: "A rat, Oct 24, 14:02". */
const titleOf = (o: mud.FightOverview) => `${capital(foesOf(o))}, ${when(o.started)}`;

/** The whole page as text, for Copy as Text. */
export function pageText(page: mud.FightPage): string {
  const parts = [`Combat Summary: ${titleOf(page.overview)}`];
  for (const s of page.sections) parts.push("", s.heading, ...s.lines);
  return parts.join("\n");
}

export const CombatSummaryDialog = memo(function CombatSummaryDialog({ open, showFirst, onClose, onStatus, onError }: CombatSummaryDialogProps) {
  const [fights, setFights] = useState<mud.FightOverview[] | null>(null);
  const [showing, setShowing] = useState<number | null>(null);
  const [page, setPage] = useState<mud.FightPage | null>(null);

  const read = useCallback(() => {
    mud.fightsList().then(setFights, (e) => onError(String(e)));
  }, [onError]);

  useEffect(() => {
    if (!open) {
      setShowing(null);
      return;
    }
    let waiting = showFirst === "next";
    setShowing(showFirst === "next" ? null : showFirst);
    read();
    const unlisten = mud.onFightKept((o) => {
      read();
      if (waiting && o) {
        waiting = false;
        setShowing(o.id);
      }
    });
    return () => void unlisten.then((f) => f());
  }, [open, showFirst, read]);

  useEffect(() => {
    if (showing === null) return setPage(null);
    let stale = false;
    mud.fightPage(showing).then(
      (p) => !stale && setPage(p),
      (e) => onError(String(e)),
    );
    return () => {
      stale = true;
    };
  }, [showing, onError]);

  function copy() {
    if (!page) return;
    navigator.clipboard.writeText(pageText(page)).then(
      () => onStatus("Copied the Combat Summary as text."),
      () => onError("Couldn't copy the Combat Summary."),
    );
  }

  const all = fights ?? [];
  const summary = !fights
    ? "Reading the fights kept…"
    : showFirst === "next" && showing === null && open && fightsWaiting(all)
      ? "The fight just over shows in a moment, once its experience and loot are in."
    : all.length === 0
      ? "No fights yet. Each fight is kept here the moment it's over, with everything The Fight saw."
      : `${plural(all.length, "fight", "fights")}, the latest first.`;

  return (
    <>
      <Dialog open={open} onClose={onClose} title="Combat Summary" className="dialog-wide dialog-hooks" covered={showing !== null}>
        <p>Every fight you've had as this character: who, how it ended and what it cost, blow by blow. Damage marked "about" is measured from the health that fell.</p>
        <p className="hooks-summary" data-testid="fights-summary">
          {summary}
        </p>
        {/* The columns' names, for the eye; each row says its own. */}
        <div className="hooks-head fights-row" aria-hidden="true">
          <span />
          <span>Against</span>
          <span>Result</span>
          <span>When</span>
        </div>
        <ul className="hooks-list fights-list" data-testid="fights-list" tabIndex={0} aria-label="Fights: against whom, the result, and when">
          {open &&
            all.map((o) => (
              <li key={o.id} className="fights-row">
                <button type="button" className="hook-edit" data-testid="fight-show" aria-label={`Show the fight with ${foesOf(o)}, ${when(o.started)}`} onClick={() => setShowing(o.id)}>
                  Show…
                </button>
                <span className="hook-key">{capital(foesOf(o))}</span>
                <span className="hook-value">
                  {endingWords(o)}, {clock(o.length)}
                </span>
                <span className="hook-value">{when(o.started)}</span>
              </li>
            ))}
        </ul>
      </Dialog>
      {/* Beside the list's dialog, not in it, as with the Bestiary's entry. */}
      <Dialog
        open={open && showing !== null && page !== null}
        onClose={() => setShowing(null)}
        title={page ? titleOf(page.overview) : "Fight"}
        className="dialog-wide"
        actions={
          <>
            <button type="button" data-testid="fight-copy" onClick={copy}>
              Copy as Text
            </button>
            <button type="button" className="primary" onClick={() => setShowing(null)}>
              Close
            </button>
          </>
        }
      >
        <div className="summary-page" data-testid="fight-page" tabIndex={0} aria-label="The fight, section by section">
          {page?.sections.map((s) => (
            <section key={s.heading} aria-label={s.heading}>
              <h3 className="summary-heading">{s.heading}</h3>
              <ul className="summary-lines">
                {s.lines.map((line, i) => (
                  <li key={i}>{line}</li>
                ))}
              </ul>
              {s.charts.map((c) => (
                <div key={c.name} className="summary-chart" aria-hidden="true">
                  <div className="summary-chart-name">{c.name}</div>
                  {c.rows.map((row, i) => (
                    <div key={i} className={`summary-chart-row${c.name === "You" ? " summary-chart-you" : " summary-chart-foe"}`}>
                      {row}
                    </div>
                  ))}
                </div>
              ))}
            </section>
          ))}
        </div>
      </Dialog>
    </>
  );
});

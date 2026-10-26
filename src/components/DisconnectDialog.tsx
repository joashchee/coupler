/**
 * **Connection Lost**: what Coupler saw as a connection ended without
 * the player leaving (src-tauri/src/disconnect.rs), kept in the
 * Disconnections table. Offered once, as it happens (`asking`): the
 * likely cause in a sentence, and Show Findings opens the rest; a row's
 * Show… in gear → Tables… opens them at once. Copy as Text puts them
 * on the clipboard, only when the player asks, for a report to the
 * game's makers: the game's last lines can hold other players' names
 * (CLAUDE.md rule 1).
 */
import { useEffect, useState } from "react";
import * as mud from "../lib/mud";
import { Dialog } from "./Dialog";

/** The findings to show: from the moment it ended, or a row of the table. */
export interface Lost {
  finding: Pick<mud.Finding, "at" | "how" | "cause" | "details">;
  /** Offered as it happened: the question first, the findings on a yes. */
  asking: boolean;
}

interface DisconnectDialogProps {
  lost: Lost | null;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
  /** Another dialog is open over this one. */
  covered?: boolean;
}

/** When it ended, in full: "Mon, Oct 26, 14:02:31". */
const endedAt = (ms: number) =>
  new Date(ms).toLocaleString(undefined, { weekday: "short", month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit" });

/** The findings as text, for the clipboard. */
export const findingsText = (f: Lost["finding"]) => [`Coupler: a connection to CoffeeMUD ended ${endedAt(f.at)}.`, ...f.details].join("\n");

export function DisconnectDialog({ lost, onClose, onStatus, onError, covered }: DisconnectDialogProps) {
  const [shown, setShown] = useState(false);
  useEffect(() => setShown(!!lost && !lost.asking), [lost]);
  const f = lost?.finding;

  function copy() {
    if (!f) return;
    navigator.clipboard.writeText(findingsText(f)).then(
      () => onStatus("Copied the findings as text."),
      () => onError("Couldn't copy the findings."),
    );
  }

  return (
    <Dialog
      open={lost !== null}
      onClose={onClose}
      covered={covered}
      title={lost?.asking ? "Connection Lost" : "What Coupler Saw"}
      className="dialog-wide"
      actions={
        shown ? (
          <>
            <button type="button" data-testid="disconnect-copy" onClick={copy}>
              Copy as Text
            </button>
            <button type="button" className="primary" onClick={onClose}>
              Done
            </button>
          </>
        ) : (
          <>
            <button type="button" onClick={onClose}>
              Not Now
            </button>
            <button type="button" className="primary" data-testid="disconnect-show" onClick={() => setShown(true)}>
              Show Findings
            </button>
          </>
        )
      }
    >
      {f && (
        <>
          <p data-testid="disconnect-cause">
            {lost?.asking ? "The connection to CoffeeMUD ended, and you didn't leave. " : `Ended ${endedAt(f.at)}. `}
            {f.cause}
          </p>
          {shown ? (
            <pre className="licenses-text disconnect-findings" data-testid="disconnect-findings" tabIndex={0} aria-label="The findings, a line each">
              {f.details.join("\n")}
            </pre>
          ) : (
            <p>Coupler kept what it saw in its last moments (the game's last lines, what was sent, the times) in the Disconnections table, in Tables in the gear menu. Show them now?</p>
          )}
        </>
      )}
    </Dialog>
  );
}

import { Dialog } from "./Dialog";
import * as backup from "../lib/backup";
import * as mud from "../lib/mud";

/** A backup chosen or dropped, waiting for a yes. */
export interface RestoreOffer {
  path: string;
  manifest: mud.BackupManifest;
}

interface RestoreDialogProps {
  offer: RestoreOffer | null;
  busy: boolean;
  onCancel: () => void;
  onRestore: () => void;
}

/**
 * The question before a Coupler Backup is restored: what's in it, when
 * it was made and by which Coupler, and that it replaces everything
 * Coupler keeps now and starts Coupler again. Escape or Cancel leaves
 * everything as it is.
 */
export function RestoreDialog({ offer, busy, onCancel, onRestore }: RestoreDialogProps) {
  const m = offer?.manifest;
  const name = offer ? (offer.path.split(/[\\/]/).pop() ?? offer.path) : "";
  return (
    <Dialog
      open={offer !== null}
      onClose={onCancel}
      title="Restore this Coupler Backup?"
      actions={
        <>
          <button type="button" data-testid="restore-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button type="button" className="primary" data-testid="restore-confirm" disabled={busy} aria-busy={busy} onClick={onRestore}>
            Restore and Start Again
          </button>
        </>
      }
    >
      {m && (
        <>
          <p data-testid="restore-facts">
            {name}: made {backup.madeInWords(m.made)} by Coupler {m.app}. It holds {m.files} {m.files === 1 ? "file" : "files"} ({mud.sizeInWords(m.bytes)}) and {m.settings}{" "}
            {m.settings === 1 ? "setting" : "settings"}.
          </p>
          <p>
            Everything Coupler keeps now is replaced by the backup's: the settings, the Assets folder, the hooks, the maps, the characters' voices, the room
            pictures' looks, the journal and the log. Coupler then starts again.
          </p>
        </>
      )}
    </Dialog>
  );
}

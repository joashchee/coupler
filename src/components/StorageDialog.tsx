/**
 * Storage (gear menu): what Coupler keeps on this computer, kind by
 * kind (src-tauri/src/storage.rs), and making room.
 *
 * - Each kind's size and how many files, and the disk's free space.
 * - **Clear the Caches**: the voice cache (the narrator's phrases too),
 *   CoffeeMUD's sounds as rendered, the smaller copies waiting for an
 *   answer and a backup's leftovers. Made again as they're wanted, so
 *   nothing is lost but a moment the next time; no warning.
 * - What's the player's own (assets no trigger uses, the maps, the
 *   rooms' looks, the journal and the log, the Bestiary, the copy from
 *   before the last restore) is deleted only after a warning that says
 *   it's for good and offers Back Up First…; the maps, looks, journal
 *   and Bestiary only while hung up.
 * - Export Coupler Backup… and Export the Smallest Backup… (Brotli's
 *   strongest setting on every file, slower). Not on the phone, which
 *   has no backups.
 *
 * Words first (rule 10): every row is a sentence for VoiceOver, each
 * button names its kind.
 */
import { useCallback, useEffect, useState } from "react";
import * as mud from "../lib/mud";
import { PHONE } from "../lib/formFactor";
import { Dialog } from "./Dialog";

interface StorageDialogProps {
  open: boolean;
  onClose: () => void;
  connected: boolean;
  /** The Music Editor's file, never deleted as unused. */
  musicFile: string | null;
  /** Makes a backup the player names; `smallest` in Brotli's strongest setting throughout. */
  exportBackup: (smallest: boolean) => Promise<void>;
  /** The Assets folder changed, to list again. */
  onAssetsChanged: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

type Forget = "unusedAssets" | "beforeRestore" | "maps" | "looks" | "journal" | "bestiary";

/** Each kind's name, in the order shown, and what deletes it. */
const ROWS: { id: string; name: string; cache?: true; forget?: Forget }[] = [
  { id: "assetsSfx", name: "Your sounds (SFX)" },
  { id: "assetsBgm", name: "Your music (BGM)" },
  { id: "assetsArt", name: "Your pictures (ART)" },
  { id: "assetsFonts", name: "Your SoundFonts" },
  { id: "voiceCache", name: "Voices said before", cache: true },
  { id: "soundbankCache", name: "CoffeeMUD's sounds, ready", cache: true },
  { id: "assetsWaiting", name: "Smaller copies waiting", cache: true },
  { id: "backupLeftovers", name: "A backup's leftovers", cache: true },
  { id: "beforeRestore", name: "From before a restore", forget: "beforeRestore" },
  { id: "restoring", name: "A restore waiting" },
  { id: "maps", name: "The maps", forget: "maps" },
  { id: "looks", name: "The rooms' looks", forget: "looks" },
  { id: "cast", name: "Characters' voices" },
  { id: "hooks", name: "The hooks" },
  { id: "journal", name: "The journal and the log", forget: "journal" },
  { id: "bestiary", name: "The Bestiary and Items", forget: "bestiary" },
  { id: "other", name: "Everything else" },
];

/** The warning before each: what goes, for good. */
const WARNINGS: Record<Forget, { title: string; words: string; done: string; whilePlaying?: true }> = {
  unusedAssets: {
    title: "Delete Assets Not in Use?",
    words:
      "Every sound, piece of music, picture and SoundFont in the Assets folder that no hook plays or shows is deleted, for good. A SoundFont a MIDI file plays through and the file open in the Music Editor are kept.",
    done: "Deleted the assets no hook uses",
  },
  beforeRestore: {
    title: "Delete What Was There Before the Restore?",
    words: "When a backup was restored, what Coupler held before it was kept aside. Deleted, the restore can't be undone by hand any more.",
    done: "Deleted what was kept from before the restore",
  },
  maps: {
    title: "Forget Every Map?",
    words: "Every room you've explored in every world, your landmarks and the portals and ships noted in them are forgotten, for good. The map starts over as you move.",
    done: "Forgot every map",
    whilePlaying: true,
  },
  looks: {
    title: "Forget the Rooms' Looks?",
    words: "Every room you painted a new look for (Paint a New Look) goes back to its first look, for good.",
    done: "Forgot the rooms' looks",
    whilePlaying: true,
  },
  journal: {
    title: "Delete the Journal and the Log?",
    words: "Everything said to you and on the channels, in every world and by every character, is deleted, for good.",
    done: "Deleted the journal and the log",
    whilePlaying: true,
  },
  bestiary: {
    title: "Delete the Bestiary and the Items?",
    words: "Every creature and item you've come across, and all Coupler learned of them, is deleted, for good.",
    done: "Deleted the Bestiary and the Items",
    whilePlaying: true,
  },
};

const filesInWords = (n: number) => `${n.toLocaleString()} ${n === 1 ? "file" : "files"}`;

export function StorageDialog({ open, onClose, connected, musicFile, exportBackup, onAssetsChanged, onStatus, onError }: StorageDialogProps) {
  const [report, setReport] = useState<mud.StorageReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [asking, setAsking] = useState<Forget | null>(null);

  const measure = useCallback(() => {
    mud.storageReport().then(setReport, (e) => onError(String(e)));
  }, [onError]);
  useEffect(() => {
    if (open) measure();
  }, [open, measure]);

  const uses = (id: string) => report?.uses.find((u) => u.id === id) ?? { id, bytes: 0, files: 0 };
  const caches = ROWS.filter((r) => r.cache).reduce((sum, r) => sum + uses(r.id).bytes, 0);
  const assets = ["assetsSfx", "assetsBgm", "assetsArt", "assetsFonts"].reduce((sum, id) => sum + uses(id).bytes, 0);

  async function clearCaches() {
    setBusy(true);
    try {
      const freed = await mud.storageClearCaches();
      onStatus(`Cleared the caches: ${mud.sizeInWords(freed)} back. They're made again as they're wanted.`);
      measure();
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function forget(what: Forget) {
    setAsking(null);
    setBusy(true);
    try {
      const freed = await mud.storageForget(what, musicFile);
      onStatus(`${WARNINGS[what].done}: ${mud.sizeInWords(freed)} back.`);
      if (what === "unusedAssets") onAssetsChanged();
      measure();
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const warning = asking ? WARNINGS[asking] : null;
  const blocked = (what: Forget) => connected && WARNINGS[what].whilePlaying === true;

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        covered={asking !== null}
        title="Storage"
        className="dialog-wide"
        actions={
          <>
            {!PHONE && (
              <>
                <button type="button" data-testid="storage-backup" disabled={busy} onClick={() => void exportBackup(false)}>
                  Export Backup…
                </button>
                <button type="button" data-testid="storage-backup-smallest" disabled={busy} onClick={() => void exportBackup(true)}>
                  Export the Smallest Backup…
                </button>
              </>
            )}
            <button type="button" className="primary" onClick={onClose}>
              Done
            </button>
          </>
        }
      >
        <p data-testid="storage-summary" aria-live="polite">
          {report
            ? `Coupler keeps ${mud.sizeInWords(report.total)} on this computer: ${mud.sizeInWords(assets)} of it your assets, ${mud.sizeInWords(caches)} caches.${report.free !== null ? ` ${mud.sizeInWords(report.free)} free on the disk.` : ""}`
            : "Measuring…"}
        </p>
        <div className="hooks-head storage-row" aria-hidden="true">
          <span>What</span>
          <span>Size</span>
          <span />
        </div>
        <ul className="hooks-list storage-list" data-testid="storage-list" aria-label="What Coupler keeps: each kind, then its size">
          {ROWS.map((row) => {
            const u = uses(row.id);
            const size = u.files === 0 ? "nothing" : `${mud.sizeInWords(u.bytes)}, ${filesInWords(u.files)}`;
            return (
              <li key={row.id} className="storage-row" data-testid={`storage-${row.id}`}>
                <span className="hook-key">{row.cache ? `${row.name} (cache)` : row.name}</span>
                <span className="hook-value">{size}</span>
                {row.forget && u.files > 0 ? (
                  <button
                    type="button"
                    className="hook-edit"
                    disabled={busy || blocked(row.forget)}
                    title={blocked(row.forget) ? "Hang up first: it's in use while you play." : undefined}
                    aria-label={`Delete ${row.name}`}
                    data-testid={`storage-forget-${row.forget}`}
                    onClick={() => setAsking(row.forget!)}
                  >
                    Delete…
                  </button>
                ) : (
                  <span />
                )}
              </li>
            );
          })}
        </ul>
        <div className="storage-actions">
          <button type="button" data-testid="storage-clear-caches" disabled={busy || caches === 0} aria-busy={busy} onClick={() => void clearCaches()}>
            Clear the Caches
          </button>
          <button type="button" data-testid="storage-forget-unusedAssets" disabled={busy || assets === 0} onClick={() => setAsking("unusedAssets")}>
            Delete Assets Not in Use…
          </button>
        </div>
        <p className="hooks-summary">
          The caches are made again as they're wanted: nothing is lost but a moment the next time. What's yours is deleted only after you say so, and a backup first
          keeps it.{connected ? " Hang up to delete the maps, looks, journal or Bestiary." : ""}
        </p>
      </Dialog>

      <Dialog
        open={asking !== null}
        onClose={() => setAsking(null)}
        title={warning?.title ?? ""}
        actions={
          <>
            {!PHONE && (
              <button
                type="button"
                data-testid="storage-backup-first"
                onClick={() => {
                  void exportBackup(false);
                }}
              >
                Back Up First…
              </button>
            )}
            <button type="button" onClick={() => setAsking(null)}>
              Cancel
            </button>
            <button type="button" className="primary" data-testid="storage-forget-confirm" onClick={() => asking && void forget(asking)}>
              Delete
            </button>
          </>
        }
      >
        {warning && <p>{`${warning.words} This can't be undone, unless you make a backup first.`}</p>}
      </Dialog>
    </>
  );
}

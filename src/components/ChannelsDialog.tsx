/**
 * Channels (the Log's Every Channel tab → Channels…): every channel the
 * game lets this character use, each on or off, turned with CoffeeMUD's
 * own commands (lib/channels.ts, src-tauri/src/channels.rs): NO<NAME>
 * turns one off and <NAME> ON back on, so the game itself stops sending
 * it, in any client; Turn All On and Turn All Off are CHANNELS ON and
 * CHANNELS OFF. The list is the game's CHANNELS, read when it opens and
 * again after each change; its answers aren't shown in the game output.
 */
import { useEffect, useId, useState } from "react";
import * as channels from "../lib/channels";
import type * as mud from "../lib/mud";
import { Dialog } from "./Dialog";

interface ChannelsDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

export function ChannelsDialog({ open, onClose, onStatus, onError }: ChannelsDialogProps) {
  const id = useId();
  const [listing, setListing] = useState<mud.ChannelListing | null>(channels.current());
  // Not in the game, or waiting for the game's first list.
  const [problem, setProblem] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  // A change sent, its new list not yet come.
  const [busy, setBusy] = useState(false);

  useEffect(() => channels.onListing(setListing), []);

  useEffect(() => {
    if (!open) return;
    setProblem(null);
    setAsking(true);
    void channels.ask().then((why) => {
      setProblem(why);
      if (why) setAsking(false);
    });
  }, [open]);
  useEffect(() => {
    if (listing) setAsking(false);
  }, [listing]);

  async function change(what: string, task: () => Promise<string>) {
    setBusy(true);
    onStatus(what);
    try {
      onStatus(await task());
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const list = listing?.channels ?? [];
  const on = list.filter((c) => c.on).length;
  const summary = problem
    ? problem
    : asking || !listing
      ? "Asking the game for your channels…"
      : listing.none
        ? listing.none
        : `${list.length} channels, ${on} on and ${list.length - on} off.`;

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Channels"
      className="dialog-wide"
      actions={
        <>
          <button type="button" data-testid="channels-all-on" disabled={busy || problem !== null || !listing} onClick={() => void change("Turning every channel on…", () => channels.all(true))}>
            Turn All On
          </button>
          <button type="button" data-testid="channels-all-off" disabled={busy || problem !== null || !listing} onClick={() => void change("Turning every channel off…", () => channels.all(false))}>
            Turn All Off
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Close
          </button>
        </>
      }
    >
      <p>
        The channels the game lets your character use. Off, the game stops sending a channel to you, in Coupler or any other client, until it's
        turned on again. Each change is the game's own command (NOGOSSIP turns off GOSSIP), so it lasts with your character.
      </p>
      <p id={`${id}-summary`} className="hooks-summary" data-testid="channels-summary" aria-live="polite">
        {summary}
      </p>
      <ul className="hooks-list channels-list" data-testid="channels-list" aria-label="Your channels" aria-describedby={`${id}-summary`}>
        {!problem &&
          list.map((c) => (
            <li key={c.name}>
              <input
                type="checkbox"
                checked={c.on}
                disabled={busy}
                aria-label={`${c.name}, on`}
                data-testid={`channel-${c.name}`}
                onChange={(e) => {
                  const turnOn = e.currentTarget.checked;
                  void change(`Turning ${c.name} ${turnOn ? "on" : "off"}…`, () => channels.turn(c.name, turnOn));
                }}
              />
              <span className={c.on ? "hook-key" : "hook-value"}>{c.name}</span>
              <span className="hook-value">{c.on ? "on" : "off: the game doesn't send it"}</span>
            </li>
          ))}
      </ul>
    </Dialog>
  );
}

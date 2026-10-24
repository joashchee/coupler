/**
 * The Journal and the Log (Cmd+Shift+J, Cmd+Shift+K, and their buttons
 * by the message bar): every line of talk the character came across,
 * kept by src-tauri/src/journal.rs, oldest first with the newest at the
 * bottom. The journal is what was said in the game (says, tells, the
 * group), the log what was said outside it (OOC, INFO, every channel).
 *
 * Search finds words or a name; Who narrows the journal to one speaker,
 * and the log's tabs (Every Channel, then one a channel, most lines
 * first; Left and Right move between them) to one channel. A green bullet marks a line not yet heard to the end; a
 * screen reader hears "Not heard yet" instead. Play says a line in its
 * speaker's voice at once, cutting off anything being said; Play Unheard
 * says every unheard line in order; Cmd+Period stops it all. Lines
 * played to the end lose their bullet. On a channel's tab, Mute Channel
 * turns it off with the game's own NO<NAME> (lib/channels.ts), so the
 * game stops sending it; Unmute Channel turns it back on. On Every
 * Channel, Channels… lists them all, on and off (ChannelsDialog.tsx).
 * Opening the Log asks the game which are off, its answer not shown.
 *
 * The list isn't a live region: talk arrives all the time while playing.
 */
import * as keys from "../lib/keys";
import { memo, useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import * as mud from "../lib/mud";
import * as gameChannels from "../lib/channels";
import { ChannelsDialog } from "./ChannelsDialog";
import { Dialog } from "./Dialog";

interface JournalDialogProps {
  book: mud.Book;
  open: boolean;
  onClose: () => void;
  unheard: mud.Unheard;
  /** Plays lines now, in order; false when there's no voice to say them. */
  onPlay: (entries: mud.JournalEntry[]) => Promise<boolean>;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

const TWO = (n: number) => String(n).padStart(2, "0");
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** "14:05" today, "Oct  3 14:05" before: always 12 characters, so the columns stay put. */
export function when(at: number, today = new Date()): string {
  const d = new Date(at);
  const time = `${TWO(d.getHours())}:${TWO(d.getMinutes())}`;
  const sameDay = d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth() && d.getDate() === today.getDate();
  return sameDay ? time.padStart(12) : `${MONTHS[d.getMonth()]} ${String(d.getDate()).padStart(2)} ${time}`;
}

/** The same in words, for a screen reader. */
function whenWords(at: number): string {
  const d = new Date(at);
  return d.toLocaleString(undefined, { month: "long", day: "numeric", hour: "numeric", minute: "2-digit" });
}

export const JournalDialog = memo(function JournalDialog({ book, open, onClose, unheard, onPlay, onStatus, onError }: JournalDialogProps) {
  const id = useId();
  const log = book === "log";
  const title = log ? "Log" : "Journal";
  const [query, setQuery] = useState("");
  const [who, setWho] = useState("");
  const [listing, setListing] = useState<mud.JournalListing | null>(null);
  const list = useRef<HTMLUListElement>(null);
  // The game's channels, on and off, as it last said (lib/channels.ts).
  const [, setChannelList] = useState(gameChannels.current());
  useEffect(() => gameChannels.onListing(setChannelList), []);
  useEffect(() => {
    if (open && log) void gameChannels.ask();
  }, [open, log]);
  const [choosing, setChoosing] = useState(false);
  const [turning, setTurning] = useState(false);

  const read = useCallback(() => {
    mud
      .journalList(book, query, who || null)
      .then(setListing)
      .catch((e) => onError(String(e)));
  }, [book, query, who, onError]);

  // Read again as lines arrive or are heard; a burst is read once.
  useEffect(() => {
    if (!open) return;
    read();
    let timer: number | null = null;
    const unlisten = mud.onJournalChanged(() => {
      if (timer === null) {
        timer = window.setTimeout(() => {
          timer = null;
          read();
        }, 250);
      }
    });
    return () => {
      if (timer !== null) window.clearTimeout(timer);
      void unlisten.then((f) => f());
    };
  }, [open, read]);

  // The newest is at the bottom: start there, and stay there while
  // following it, but not when the player has scrolled up to read.
  const following = useRef(true);
  useEffect(() => {
    if (open) following.current = true;
  }, [open, query, who]);
  useLayoutEffect(() => {
    const el = list.current;
    if (el && following.current) el.scrollTop = el.scrollHeight;
  }, [listing]);

  const entries = listing?.entries ?? [];
  const speakers = listing?.speakers ?? [];
  const waiting = log ? unheard.log : unheard.journal;
  const narrowed = query.trim() !== "" || who !== "";
  const summary = !listing
    ? `Reading the ${title.toLowerCase()}…`
    : listing.matching === 0
      ? narrowed
        ? "Nothing matches."
        : log
          ? "Nothing yet. Every channel's lines are kept here as they come: OOC, INFO, gossip and the rest."
          : "Nothing yet. What's said to you and around you in the game is kept here as it's said."
      : `${plural(listing.matching, "line", "lines")}${narrowed ? " match" : ""}${entries.length < listing.matching ? `, the newest ${entries.length} shown` : ""}, oldest first. ${
          waiting > 0 ? `${waiting} not heard yet.` : "All heard."
        }`;

  /** The log's tabs: every channel, then each one. */
  const tabs = [
    { key: "", name: "Every Channel", channel: "", unheard: unheard.log, muted: false },
    ...speakers.map((s) => ({ key: s.key, name: s.name || "Someone", channel: s.name, unheard: s.unheard, muted: s.name !== "" && gameChannels.isOn(s.name) === false })),
  ];
  /** The channel whose tab is chosen, if one is. */
  const channel = (log && tabs.find((t) => t.key === who)?.channel) || null;
  const channelMuted = channel !== null && gameChannels.isOn(channel) === false;
  function onTabKey(e: KeyboardEvent<HTMLButtonElement>) {
    const at = tabs.findIndex((t) => t.key === who);
    const to = e.key === "ArrowRight" ? at + 1 : e.key === "ArrowLeft" ? at - 1 + tabs.length : e.key === "Home" ? 0 : e.key === "End" ? tabs.length - 1 : -1;
    if (to < 0) return;
    e.preventDefault();
    const next = tabs[to % tabs.length];
    setWho(next.key);
    document.getElementById(`${id}-tab-${tabs.indexOf(next)}`)?.focus();
  }

  async function play(chosen: mud.JournalEntry[], what: string) {
    if (await onPlay(chosen)) onStatus(what);
    else onError("Coupler's voice is silent: turn VOX up in the Mixer to hear it.");
  }

  async function playUnheard() {
    try {
      const chosen = await mud.journalUnheardEntries(book);
      if (chosen.length === 0) onStatus(`Everything in the ${title.toLowerCase()} has been heard.`);
      else await play(chosen, keys.keys(`Playing ${plural(chosen.length, "unheard line", "unheard lines")}. Cmd+Period stops.`));
    } catch (e) {
      onError(String(e));
    }
  }

  async function mute(muting: boolean) {
    if (!channel) return;
    setTurning(true);
    onStatus(`Asking the game to turn ${channel} ${muting ? "off" : "on"}…`);
    try {
      onStatus(await gameChannels.turn(channel, !muting));
    } catch (e) {
      onError(String(e));
    } finally {
      setTurning(false);
    }
  }

  async function markAll() {
    try {
      await mud.journalHeardAll(book);
      onStatus(`Everything in the ${title.toLowerCase()} is marked heard.`);
    } catch (e) {
      onError(String(e));
    }
  }

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        title={title}
        className="dialog-wide dialog-hooks"
        covered={open && choosing}
        actions={
          <>
            {log && !channel && (
              <button type="button" data-testid="log-channels" onClick={() => setChoosing(true)}>
                Channels…
              </button>
            )}
            {channel && (
              <button type="button" data-testid="log-mute" aria-pressed={channelMuted} disabled={turning} onClick={() => void mute(!channelMuted)}>
                {channelMuted ? "Unmute Channel" : "Mute Channel"}
              </button>
            )}
            <button type="button" data-testid={`${book}-mark-all`} disabled={waiting === 0} onClick={() => void markAll()}>
              Mark All Heard
            </button>
            <button type="button" data-testid={`${book}-play-unheard`} disabled={waiting === 0} onClick={() => void playUnheard()}>
              Play Unheard
            </button>
            <button type="button" className="primary" onClick={onClose}>
              Close
            </button>
          </>
        }
      >
        <p>
          {log
            ? "What's said outside the game: OOC, INFO and every other channel. These lines aren't spoken as they come; each is a soft blip, a note of its channel's own. On a channel's tab, Mute Channel has the game stop sending it; Channels… lists them all."
            : "What's said in the game, to you and around you, by who said it and when. Someone repeating a line they've said before is kept once."}{" "}
          {keys.keys("A green bullet is a line not yet heard to the end. Play says one now; Cmd+Period stops.")}
        </p>
        {log && (
          <div className="hooks-tabs journal-tabs" role="tablist" aria-label="Channels">
            {tabs.map((t, i) => (
              <button
                key={t.key || "every"}
                type="button"
                role="tab"
                id={`${id}-tab-${i}`}
                className={`hooks-tab${who === t.key ? " selected" : ""}`}
                data-testid={`log-tab-${t.key || "every"}`}
                aria-selected={who === t.key}
                aria-controls={`${id}-panel`}
                aria-label={`${t.name}${t.muted ? ", off" : ""}${t.unheard > 0 ? `, ${t.unheard} unheard` : ""}`}
                tabIndex={who === t.key ? 0 : -1}
                onClick={() => setWho(t.key)}
                onKeyDown={onTabKey}
              >
                <span className="hooks-tab-mark" aria-hidden="true">
                  •
                </span>
                {`${t.name}${t.muted ? " (off)" : ""}${t.unheard > 0 ? ` (${t.unheard})` : ""}`}
              </button>
            ))}
          </div>
        )}
        <div className="hooks-search" role={log ? "tabpanel" : undefined} id={log ? `${id}-panel` : undefined}>
          <label htmlFor={`${id}-search`}>Search</label>
          <input
            id={`${id}-search`}
            type="text"
            data-testid={`${book}-search`}
            aria-describedby={`${id}-summary`}
            placeholder="Words, or a name"
            value={query}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
          {!log && (
            <>
              <label htmlFor={`${id}-who`}>Who</label>
              <select id={`${id}-who`} className="hook-asset journal-who" data-testid={`${book}-who`} value={who} onChange={(e) => setWho(e.currentTarget.value)}>
                <option value="">Everyone</option>
                {speakers.map((s) => (
                  <option key={s.key} value={s.key}>
                    {`${s.name || "Someone"} (${s.lines}${s.unheard > 0 ? `, ${s.unheard} unheard` : ""})`}
                  </option>
                ))}
              </select>
            </>
          )}
        </div>
        <p id={`${id}-summary`} className="hooks-summary" data-testid={`${book}-summary`}>
          {summary}
        </p>
        {/* The columns' names, for the eye; each row says its own. */}
        <div className="hooks-head journal-row" aria-hidden="true">
          <span />
          <span />
          <span>When</span>
          <span>{log ? "Channel" : "Who"}</span>
          <span>What was said</span>
        </div>
        <ul
          ref={list}
          className="hooks-list journal-list"
          data-testid={`${book}-list`}
          tabIndex={0}
          aria-label={`${title}: when, ${log ? "the channel" : "who"}, and what was said, oldest first`}
          onScroll={(e) => {
            const el = e.currentTarget;
            following.current = el.scrollTop + el.clientHeight >= el.scrollHeight - 4;
          }}
        >
          {open &&
            entries.map((e) => {
              const name = log ? e.channel : e.mine ? "You" : e.speaker || "Someone";
              return (
                <li key={e.id} className={`journal-row${e.heard ? "" : " unheard"}`} data-testid="journal-entry">
                  <button type="button" className="hook-edit" data-testid="journal-play" aria-label={`Play: ${e.text}`} onClick={() => void play([e], `Playing: ${e.text}`)}>
                    Play
                  </button>
                  <span className="asset-dot" aria-hidden="true">
                    {e.heard ? "" : "•"}
                  </span>
                  <span className="hook-value journal-when" aria-hidden="true">
                    {when(e.at)}
                  </span>
                  <span className="hook-key">{name}</span>
                  <span className="hook-sets-off">
                    <span className="sr-only">{`${e.heard ? "" : "Not heard yet. "}${whenWords(e.at)}. `}</span>
                    {e.text}
                  </span>
                </li>
              );
            })}
        </ul>
      </Dialog>
      {log && <ChannelsDialog open={open && choosing} onClose={() => setChoosing(false)} onStatus={onStatus} onError={onError} />}
    </>
  );
});

/**
 * The journal and the log, as heard (src-tauri/src/journal.rs keeps
 * them): what's spoken as it arrives, what's only a tone, and when a line
 * counts as heard.
 *
 * - **The journal** (says, tells, the group: the game's own talk) is
 *   spoken the moment it arrives when Coupler's voice is on, the
 *   narrator saying who ("Hassan says") and the speaker's voice what
 *   (`voice.speakTalk`): at once if nothing's being said, else after
 *   what is (lib/voice.ts's one queue).
 *   A line is **heard** only once it's said to the end; Cmd+Period cuts
 *   off the line and flushes the queue, and what was cut stays unheard.
 *   A repeat (an NPC's line said before) isn't kept, and isn't spoken.
 *   Talk on the Priority Audio list (tells, say, the group; lib/priority.ts)
 *   doesn't wait: the queue pauses for it.
 * - **The log** (OOC, INFO, every channel) is never spoken on arrival:
 *   a line is a soft blip, at its channel's own note, and captioned, but
 *   at most one blip every `LOG_COOLDOWN`: a busy channel isn't a
 *   stream of them. A line that came in the cooldown wasn't blipped; if
 *   the log then goes `LOG_REMIND` without being noticed (a blip heard,
 *   the Log opened, its lines played, Cmd+Shift+U), a reminder plays
 *   with how many lines it has unread (the narrator says it with the
 *   voice on), and those count as noticed: it doesn't remind again
 *   until another blip is held back.
 * - **Tones**: when the journal has a line that nothing is about to say
 *   (the voice is off, or the queue was cut off), a knock-knock says so.
 * - Lines the player asks for (the Journal or Log dialog, Cmd+Shift+N)
 *   cut off whatever was being said and play at once, in order.
 *
 * Every line (but the player's own and repeats) is also rendered ahead
 * into the voice cache the moment it arrives (`voice.prerender`), heard
 * now or not, so it starts at once whenever it's played.
 *
 * Cues follow the CUE layer and speech the voice layer, like the rest of
 * Immersive (lib/immersive.ts).
 */
import { useCallback, useEffect, useRef, useState } from "react";
import * as earcons from "./earcons";
import * as mud from "./mud";
import * as priority from "./priority";
import * as voice from "./voice";

interface Options {
  cues: boolean;
  voiced: boolean;
  /** The Log dialog is open: the log is being read. */
  logOpen: boolean;
}

/** At most one log blip this often, ms. */
const LOG_COOLDOWN = 15_000;
/** A blip held back, and the log not noticed for this long: the reminder, ms. */
const LOG_REMIND = 10 * 60_000;

/** How long after the voice falls silent the journal's tone may say lines are waiting. */
const SETTLE = 400;

/** "Bob 2, a guard 1 and 3 others". */
function counts(list: { name: string; count: number }[]): string {
  const named = list.slice(0, 3).map((c) => `${c.name || "someone"} ${c.count}`);
  const rest = list.length - named.length;
  return rest > 0 ? `${named.join(", ")} and ${rest} ${rest === 1 ? "other" : "others"}` : named.join(", ");
}

/** What's still to be heard, said shortly: the say key's answer. */
export function describeUnheard(u: mud.Unheard): string {
  if (u.journal === 0 && u.log === 0) return "Everything's been heard.";
  const parts = [];
  if (u.journal > 0) parts.push(`Journal ${u.journal}: ${counts(u.speakers)}.`);
  if (u.log > 0) parts.push(`Log ${u.log}: ${counts(u.channels)}.`);
  return parts.join(" ");
}

/** The voices of everyone in the cast, by key, for lines played again. */
async function voices(): Promise<Map<string, mud.Voice>> {
  try {
    const cast = await mud.castList();
    return new Map(cast.characters.map((c) => [c.key, c.voice]));
  } catch {
    return new Map();
  }
}

export function useJournal({ cues, voiced, logOpen }: Options) {
  const [unheard, setUnheard] = useState<mud.Unheard>(mud.NOTHING_UNHEARD);
  const now = useRef({ cues, voiced, unheard, logOpen });
  now.current = { cues, voiced, unheard, logOpen };

  /** The log's blips: when the last played, when the player last noticed the log, and whether one was held back since. */
  const log = useRef({ blipped: 0, noticed: Date.now(), heldBack: false, timer: null as number | null });
  /** The log was read or asked about: no reminder for what's in it now. */
  const touchLog = useCallback(() => {
    const l = log.current;
    l.noticed = Date.now();
    l.heldBack = false;
    if (l.timer !== null) window.clearTimeout(l.timer);
    l.timer = null;
  }, []);
  /** Waits until the log's gone unnoticed `LOG_REMIND`, then reminds, if there's still something to remind of. */
  const awaitReminder = useCallback(() => {
    const l = log.current;
    if (l.timer !== null) return;
    l.timer = window.setTimeout(() => {
      l.timer = null;
      const { cues, voiced, unheard, logOpen } = now.current;
      if (!l.heldBack) return;
      if (logOpen || Date.now() - l.noticed < LOG_REMIND - 1000) {
        if (logOpen) l.noticed = Date.now();
        awaitReminder();
        return;
      }
      l.heldBack = false;
      l.noticed = Date.now();
      if (!cues || unheard.log === 0) return;
      earcons.logReminder(unheard.log);
      if (voiced) voice.speak(`The log has ${unheard.log} unread ${unheard.log === 1 ? "line" : "lines"}.`);
    }, Math.max(1000, log.current.noticed + LOG_REMIND - Date.now()));
  }, []);
  useEffect(() => {
    if (logOpen) touchLog();
  }, [logOpen, touchLog]);
  useEffect(() => () => {
    if (log.current.timer !== null) window.clearTimeout(log.current.timer);
  }, []);

  useEffect(() => {
    mud.journalUnheard().then(setUnheard, () => {
      // Not in Tauri (a test page): nothing's kept.
    });
  }, []);

  /** After a line is cut off, or arrives unspoken: the knock, once the voice is quiet. */
  const settle = useRef<number | null>(null);
  const remind = useCallback(() => {
    if (settle.current !== null) window.clearTimeout(settle.current);
    settle.current = window.setTimeout(() => {
      settle.current = null;
      if (now.current.cues && now.current.unheard.journal > 0 && !voice.talking()) earcons.journalWaiting();
    }, SETTLE);
  }, []);

  /** Marks a line heard once it's said to the end; a line cut off stays unheard. */
  const heardWhenDone = useCallback(
    (id: number) => (finished: boolean) => {
      if (finished) void mud.journalSetHeard([id], true).catch(() => {});
      else remind();
    },
    [remind],
  );

  useEffect(() => {
    const subscriptions = [
      mud.onJournalChanged(setUnheard),
      mud.onTalk((talk) => {
        // An entry is null for a repeat, or if the journal couldn't be
        // used: then the line is still heard, just not kept.
        const entry = talk.entry;
        if (talk.mine || talk.repeat) return;
        const { cues, voiced } = now.current;
        const book: mud.Book = talk.kind === "channel" ? "log" : "journal";
        const speaker = entry?.speaker || talk.from;
        // Rendered now, whether or not it's heard now, so it plays at once when it is.
        voice.prerender(talk.text, talk.voice, book);
        if (book === "log") {
          if (!cues) return;
          const l = log.current;
          const at = Date.now();
          // With the Log open, every line is blipped: the player's reading it.
          if (now.current.logOpen || at - l.blipped >= LOG_COOLDOWN) {
            l.blipped = at;
            touchLog();
            earcons.logLine(talk.channel);
          } else {
            l.heldBack = true;
            awaitReminder();
          }
          return;
        }
        if (cues && (talk.kind === "tell" || talk.kind === "group")) earcons.tell();
        const first = talk.kind !== "channel" && priority.isSpeech(talk.kind);
        if (voiced && !talk.voice?.quiet) voice.speakTalk(talk.text, talk.voice, { book, speaker, onDone: entry ? heardWhenDone(entry.id) : undefined, priority: first });
        else if (entry && !talk.voice?.quiet) remind();
      }),
    ];
    return () => {
      subscriptions.forEach((s) => void s.then((unlisten) => unlisten()));
      if (settle.current !== null) window.clearTimeout(settle.current);
    };
  }, [heardWhenDone, remind, touchLog, awaitReminder]);

  /**
   * Plays lines now, in order, cutting off what was being said and
   * flushing the queue: the player chose them. Each is marked heard once
   * it's said to the end. Returns false when the voice can't speak.
   */
  const play = useCallback(
    async (entries: mud.JournalEntry[]) => {
      voice.hush();
      if (entries.some((e) => e.book === "log")) touchLog();
      if (entries.length === 0) return true;
      if (!voice.canSpeak() || voice.voiceVolume() === 0) return false;
      const cast = await voices();
      for (const e of entries) {
        // Asked for, so heard even from a character kept quiet.
        const how = e.mine ? undefined : cast.get(e.speakerKey);
        voice.speakTalk(e.text, how ? { ...how, quiet: false } : null, { book: e.book, speaker: e.mine ? "You" : e.speaker, onDone: heardWhenDone(e.id) });
      }
      return true;
    },
    [heardWhenDone, touchLog],
  );

  /** Plays a book's unheard lines, oldest first. Returns how many. */
  const playUnheard = useCallback(
    async (book: mud.Book) => {
      if (book === "log") touchLog();
      const entries = await mud.journalUnheardEntries(book);
      await play(entries);
      return entries.length;
    },
    [play, touchLog],
  );

  return { unheard, play, playUnheard, touchLog };
}

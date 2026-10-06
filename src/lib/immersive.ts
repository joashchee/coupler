/**
 * Immersive mode's director: it watches what Rust reports (the map, the
 * fight, the vitals, the talk, the output) and decides what's heard. The
 * grand goal is fewer spoken words: whatever can be a sound is a cue
 * (lib/earcons.ts), and the voice (lib/voice.ts) says only what can't.
 *
 * The layers, from the bottom up:
 *
 * 1. **Place** and **Sky**: background noise and weather, looping (the
 *    hooks' BGN and BGW, src-tauri/src/ambient.rs). Already Rust's.
 * 2. **Music**: the hooks' BGM.
 * 3. **Events**: the hooks' SFX, from anything the game sends.
 * 4. **Cues**, made here: the exits of each room as notes in space, a
 *    footstep, a new room, a new area, a landmark; the heartbeat, blows
 *    and healing, low mana and movement; the fight starting, ending and
 *    the opponent weakening; a ping for a tell, the journal's and the
 *    log's tones (lib/journal.ts).
 * 5. **Voice**: tells and the group, says in the room (lib/journal.ts,
 *    which keeps them in the journal and knows what was heard, each
 *    introduced by the narrator: "Hassan says", then Hassan's voice),
 *    the time of day (the change at dawn, dusk and night, and what TIME
 *    says), in the narrator's voice and not written in Immersive's game
 *    output (src-tauri/src/speech.rs's `time` lines), the commands'
 *    one-line answers in short ("Sitting.", src-tauri/src/echo.rs and
 *    lib/echoes.ts, not written either), any other lone line of the
 *    game's that no hook gave a sound to, a long look's room and the
 *    hidden details only its colors show (src-tauri/src/hidden.rs,
 *    written too), the login (until
 *    the character is in the game, every line of words, since there's
 *    no sound for a menu, but the intro: "Connecting…", then nothing
 *    till the game's welcome or its first question; but making a character, the guide's question
 *    in few words and each choice as it's reached, components/
 *    CreationDialog.tsx), what the player asks for by key, and in a
 *    fight, no silence: the freshest fact whenever nothing's said, and
 *    what matters at once, cutting the queue (lib/fightTalk.ts). What's
 *    on the Priority Audio list (lib/priority.ts: the time, a long look,
 *    a command's answer, a game line with the player's words) is said at
 *    once, the queue paused around it.
 *
 * Layers 1 to 3 play in every way to play; 4 and 5 in Immersive, and in
 * Workshop when the player turns them on.
 */
import { useEffect, useRef } from "react";
import * as earcons from "./earcons";
import * as echoes from "./echoes";
import { FightTalk } from "./fightTalk";
import * as mud from "./mud";
import { exitsLine, promptCut } from "./output";
import * as priority from "./priority";
import * as voice from "./voice";

interface Immersive {
  /** Layer 4. */
  cues: boolean;
  /** Layer 5. */
  voice: boolean;
  connected: boolean;
  snapshot: mud.MapSnapshot | null;
  opponent: mud.Opponent | null;
  vitals: mud.Vitals | null;
  /** The player hid the guide to making a character (components/CreationDialog.tsx), which says each question in few words while it's open. */
  guideHidden: boolean;
  /** The player hid the account menu's dialog (components/AccountMenuDialog.tsx), which says the menu itself while it's open. */
  accountHidden: boolean;
}

/** A blow worth a thud: this share of the maximum or more, at once. */
const HURT = 5;
/** Healing worth a shimmer (less is the ordinary mending between ticks). */
const HEALED = 10;
/** Mana and movement this low or under are low. */
const LOW = 20;

/** TIME's "(Hour: 0/5)" as it's said: ", hour 1 of 6." */
const spokenTime = (text: string) => text.replace(/\s*\(Hour: (\d+)\/(\d+)\)/, (_, hour: string, last: string) => `, hour ${Number(hour) + 1} of ${Number(last) + 1}.`);

/** A long look said: "Hidden details found.", the description, then the details by name. */
const spokenLook = ({ text, words }: mud.Hidden) =>
  words.length > 0 ? `Hidden details found. ${text} Hidden: ${mud.hiddenList(words)}.` : `${text} No hidden details.`;

/** The game's first line on connecting: the narrator says "Connecting…" and nothing more till the welcome. */
const CONNECTING = /^\s*Connecting to CoffeeMUD/i;
/** The game's welcome, which ends the quiet. */
const WELCOME = /Welcome to CoffeeMUD/i;

/** Where the player was, for the cues on coming into a room. */
export interface Whereabouts {
  id: string | null;
  zone: string | null;
  /** How many rooms the map knew. */
  known: number;
}

/**
 * The cues for coming into `room`: a footstep (not for the first room),
 * a sparkle if the map now knows more rooms, the open fifth for another
 * area, the bell at a landmark, then the exits. Also the tutorial's
 * (components/TutorialDialog.tsx), so it sounds as the game will.
 */
export function arrivalCues(room: mud.MapRoom, before: Whereabouts, known: number) {
  let delay = 0;
  if (before.id !== null) {
    earcons.footstep();
    delay = 0.3;
  }
  if (before.id !== null && known > before.known) {
    earcons.discovery(delay);
    delay += 0.3;
  }
  if (before.zone !== null && room.zone !== before.zone) {
    earcons.threshold(delay);
    delay += 0.5;
  }
  if (room.landmark) {
    earcons.landmark(delay);
    delay += 0.4;
  }
  earcons.exits(room.exits, delay + 0.1);
}

/** The cues for a change in the body: a blow, healing, mana or movement running low. The heartbeat is the caller's. */
export function bodyCues(before: mud.Vitals | null, vitals: mud.Vitals | null) {
  if (!vitals || !before) return;
  const most = vitals.maxHp ?? 0;
  if (most > 0 && vitals.hp !== null && before.hp !== null) {
    const change = ((vitals.hp - before.hp) * 100) / most;
    if (change <= -HURT) earcons.hurt();
    else if (change >= HEALED) earcons.healed();
  }
  const crossed = (now: number | null, then: number | null) => now !== null && now <= LOW && (then === null || then > LOW);
  if (crossed(mud.percentOf(vitals.mana, vitals.maxMana), mud.percentOf(before.mana, before.maxMana))) earcons.lowMana();
  if (crossed(mud.percentOf(vitals.moves, vitals.maxMoves), mud.percentOf(before.moves, before.maxMoves))) earcons.lowMoves();
}

/** A fight as the cues remember it: whether there's one, and the quarter the opponent's health is in. */
export interface Fight {
  on: boolean;
  quarter: number | null;
}

export const NO_FIGHT: Fight = { on: false, quarter: null };

export const fightOf = (opponent: mud.Opponent | null): Fight => ({
  on: opponent !== null,
  quarter: opponent?.percent != null ? Math.floor(opponent.percent / 25) * 25 : null,
});

/** The cues for a change in the fight: it starts, it ends, or the opponent's health crosses a quarter. */
export function fightCues(before: Fight, opponent: mud.Opponent | null) {
  const { quarter } = fightOf(opponent);
  if (opponent && !before.on) earcons.fightStarts();
  else if (!opponent && before.on) earcons.fightEnds();
  else if (opponent && quarter !== null && before.quarter !== null && quarter < before.quarter) earcons.opponentAt(quarter);
}

export function useImmersive({ cues, voice: voiced, connected, snapshot, opponent, vitals, guideHidden, accountHidden }: Immersive) {
  const on = useRef({ cues, voiced, connected, guideHidden, accountHidden });
  on.current = { cues, voiced, connected, guideHidden, accountHidden };

  // ---- Moving: a step, what's new, then the exits ----
  const was = useRef<Whereabouts>({ id: null, zone: null, known: 0 });
  useEffect(() => {
    const room = snapshot?.room ?? null;
    const before = was.current;
    was.current = { id: room?.id ?? null, zone: room?.zone ?? before.zone, known: snapshot?.roomsKnown ?? before.known };
    if (!cues || !connected || !room || room.id === before.id) return;
    arrivalCues(room, before, snapshot?.roomsKnown ?? 0);
  }, [snapshot, cues, connected]);

  // ---- The body ----
  const body = useRef<mud.Vitals | null>(null);
  useEffect(() => {
    const before = body.current;
    body.current = vitals;
    const hp = vitals ? mud.percentOf(vitals.hp, vitals.maxHp) : null;
    earcons.heartbeat(cues && connected ? hp : null);
    if (cues && connected) bodyCues(before, vitals);
  }, [vitals, cues, connected]);
  useEffect(() => () => earcons.stopCues(), []);

  // ---- The fight ----
  const fight = useRef<Fight>(NO_FIGHT);
  useEffect(() => {
    const before = fight.current;
    fight.current = fightOf(opponent);
    if (cues) fightCues(before, opponent);
  }, [opponent, cues]);

  // ---- The fight told: no silence while it lasts (lib/fightTalk.ts) ----
  const talk = useRef<FightTalk | null>(null);
  talk.current ??= new FightTalk();
  talk.current.on = voiced && connected;
  useEffect(() => talk.current?.setOpponent(voiced && connected ? opponent : null), [opponent, voiced, connected]);
  useEffect(() => talk.current?.setVitals(vitals), [vitals]);
  useEffect(() => () => talk.current?.stop(), []);

  // ---- The login and the time of day spoken (talk is lib/journal.ts's) ----
  const inGame = useRef(false);
  /** A character's being made (src-tauri/src/creation.rs): known as its question comes, before that read's output. */
  const creating = useRef(false);
  const lastPrompt = useRef("");
  /** The unfinished line after the last read, to leave the prompt out of a line the game wrote on after it. */
  const unfinished = useRef<string | null>(null);
  /** Between "Connecting to CoffeeMUD" and the game's welcome: the intro's art and words aren't said. */
  const connecting = useRef(false);
  /** The game's showing the account menu: its dialog says it (App.tsx), not its lines (unless the player hid it). */
  const accountMenu = useRef(false);
  /** A hook went off for the read about to come (its `hook-fired` comes just before its `mud-output`). */
  const hooked = useRef(false);
  useEffect(() => {
    const say = (text: string, first = false) => {
      if (on.current.voiced && voice.speakable(text)) voice.speak(text, false, null, { priority: first });
    };
    const subscriptions = [
      mud.onGmcp((e) => {
        const p = e.package.toLowerCase();
        if (p === "room.info" || p === "char.vitals") inGame.current = true;
      }),
      mud.onHookFired(() => {
        hooked.current = true;
      }),
      mud.onOutput((e) => {
        const fired = hooked.current;
        hooked.current = false;
        // The fight's lines, for the fight talk's freshest fact.
        e.lines.forEach((line, i) => {
          if (e.kinds[i] === "combat") talk.current?.heard(line.map((s) => s.text).join(""));
        });
        // The narrator's (no voice given): it says the time, the game output doesn't.
        e.lines.forEach((line, i) => {
          if (e.kinds[i] === "time") say(spokenTime(line.map((s) => s.text).join("")), priority.isSpeech("time"));
        });
        // The commands' one-line answers, in short (lib/echoes.ts).
        if (echoes.echoesOn()) e.echoes.forEach((answer) => say(echoes.spoken(answer), priority.isAnswer(answer.id)));
        // A long look: the room as written, and what only color showed (src-tauri/src/hidden.rs).
        if (e.hidden) say(spokenLook(e.hidden), priority.isSpeech("look"));
        // The game's lines with the player's priority words, said at once, as written (not an answer, said above).
        const answered = new Set(e.echoes.flatMap((a) => Array.from({ length: a.lines }, (_, k) => a.line + k)));
        const urgent = new Set<number>();
        if (inGame.current && !creating.current) {
          e.lines.forEach((line, i) => {
            const text = line.map((s) => s.text).join("").trim();
            if (e.kinds[i] === "game" && !answered.has(i) && text !== "" && priority.matchesLine(text)) {
              urgent.add(i);
              say(text, true);
            }
          });
        }
        // A lone line of the game's ("A rat arrives from the north."), with no
        // hook's sound to tell it: said, rather than missed. A block (a
        // room, a list) is the screen reader's or the review keys'.
        // The prompt (if the game wrote on after it) and the exits (the cues play them) aren't said.
        const before = unfinished.current;
        unfinished.current = e.partial ? e.partial.map((s) => s.text).join("") : null;
        if (inGame.current && !creating.current && !fired) {
          const lone = e.lines
            .map((line, i) => {
              const text = line.map((s) => s.text).join("");
              return { text: (i === 0 ? text.slice(promptCut(text, before)) : text).trim(), kind: e.kinds[i], i };
            })
            .filter((l) => l.text !== "" && l.kind === "game" && !exitsLine(l.text));
          if (lone.length === 1 && e.echoes.length === 0 && !e.hidden && !urgent.has(lone[0].i)) say(lone[0].text);
        }
        // Making a character, the guide says the question, not the screens of text before it.
        if (inGame.current || (creating.current && !on.current.guideHidden)) return;
        // The account menu's dialog says the menu, in few words.
        if (accountMenu.current && !on.current.accountHidden && !connecting.current) {
          lastPrompt.current = e.partial ? e.partial.map((s) => s.text).join("").trim() : "";
          return;
        }
        e.lines.forEach((line, i) => {
          const text = line.map((s) => s.text).join("");
          if (CONNECTING.test(text)) {
            connecting.current = true;
            say("Connecting…");
          } else if (connecting.current) {
            if (WELCOME.test(text)) {
              connecting.current = false;
              say(text);
            }
          } else if (e.kinds[i] !== "time") say(text);
        });
        const prompt = e.partial ? e.partial.map((s) => s.text).join("").trim() : "";
        // A question before any welcome ends the quiet too: it needs an
        // answer. Not any unfinished line: a read can end mid-art.
        if (connecting.current && !/[:?]$/.test(prompt)) return;
        connecting.current = false;
        if (prompt !== lastPrompt.current) {
          lastPrompt.current = prompt;
          say(prompt);
        }
      }),
      mud.onCreation((step) => {
        creating.current = step !== null;
      }),
      mud.onAccountMenu((menu) => {
        accountMenu.current = menu !== null;
      }),
      mud.onClosed(() => {
        talk.current?.stop();
        inGame.current = false;
        creating.current = false;
        lastPrompt.current = "";
        unfinished.current = null;
        connecting.current = false;
      }),
      mud.onCharacterLeft(() => {
        talk.current?.stop();
        inGame.current = false;
      }),
    ];
    return () => subscriptions.forEach((s) => void s.then((unlisten) => unlisten()));
  }, []);
}

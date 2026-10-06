/**
 * Making a character: CoffeeMUD's character creation a question at a
 * time (src-tauri/src/creation.rs finds each one), in place of screens
 * of text. Every way to play gets it, opened by the game's first
 * question about a new account or character; Type Instead (or Esc)
 * hides it, and Cmd+Shift+G brings it back.
 *
 * Each question is shown in Coupler's few words (lib/creation.ts), with
 * where it is among the stages and what's settled so far; its choices
 * are buttons, each with the game's own line about it, and the classes
 * that suit the character's best stat say so in words (the game shows
 * that only by color). The stats are a table with Raise and Lower. The
 * game's full text for any question is one button away, and the
 * terminal behind keeps all of it.
 *
 * With Coupler's voice on (Immersive), the narrator says the question
 * and then each choice as the keyboard reaches it; a soft page turn
 * marks each new question (lib/earcons.ts). Without it, the question is
 * a live region the screen reader reads, and each choice is read with
 * its words as it's focused.
 *
 * Answers go to the game as typed lines. A password is sent from a
 * hidden field and written in the output as stars, never kept.
 */
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import * as creation from "../lib/creation";
import * as creationArt from "../lib/creationArt";
import * as earcons from "../lib/earcons";
import * as mud from "../lib/mud";
import { spanColors, type CreationChoice, type CreationStats, type CreationStep, type Line } from "../lib/mud";
import type { PortraitSource } from "../lib/pictures";
import * as voice from "../lib/voice";
import { Dialog } from "./Dialog";

interface CreationDialogProps {
  open: boolean;
  step: CreationStep | null;
  /** Coupler's voice speaks (Immersive, or Workshop with it on). */
  voiced: boolean;
  /** Immersive's cues play. */
  cues: boolean;
  /** Who makes the race and class portraits: Coupler's painter first (gear → Pictures…). */
  portraits: PortraitSource;
  /** Sends an answer to the game; a secret one is written as stars. */
  onSend: (line: string, secret: boolean) => void;
  /** Hide the guide and type instead, this time. */
  onClose: () => void;
  /** CoffeeMUD's own way from now on (the standard mode). */
  onStandard: () => void;
}

/** The questions answered by typing, and the field's name. */
const TYPED: Partial<Record<CreationStep["kind"], string>> = {
  accountName: "Account name",
  loginName: "Character name",
  accountPassword: "Password",
  email: "E-mail address",
  emailAgain: "E-mail address again",
  accountMenu: "A character's name to play",
  characterName: "Character name",
  password: "Password",
  statAmount: "Points",
  other: "Answer",
};

/** A stat's buttons, by how much each changes it: a game may give fifty points to spend. */
const STAT_STEPS = [-5, -1, 1, 5];

/** The questions whose game text is shown from the start: it's what was asked for. */
const TEXT_SHOWN = new Set<CreationStep["kind"]>(["confirmRace", "confirmClass", "confirmDeity", "rules", "other", "yesNo"]);

export function CreationDialog({ open, step, voiced, cues, portraits, onSend, onClose, onStandard }: CreationDialogProps) {
  const id = useId();
  const [seen, setSeen] = useState<Set<creation.Stage>>(new Set());
  /** The stage of the last question that had one: a question the guide doesn't know stays in it. */
  const [stage, setStage] = useState<creation.Stage | null>(null);
  const [textShown, setTextShown] = useState(false);
  const [typed, setTyped] = useState("");
  /** What the focused choice or stat is about, shown beside the list. */
  const [about, setAbout] = useState<string | null>(null);
  /** The race or class reached in the list, for its picture and facts. */
  const [reached, setReached] = useState<string | null>(null);
  /** CoffeeMUD's pictures and the races' facts (lib/creationArt.ts), once read. */
  const [art, setArt] = useState<Awaited<ReturnType<typeof creationArt.load>> | null>(null);
  useEffect(() => {
    if (open && !art) void creationArt.load().then(setArt);
  }, [open, art]);
  /** The question last shown, while the guide was open. */
  const shown = useRef<CreationStep | null>(null);
  const wasOpen = useRef(false);
  const list = useRef<HTMLUListElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const firstStat = useRef<HTMLButtonElement>(null);
  /** Focus moved by the guide itself: not said again (the question already said it). */
  const quietFocus = useRef(false);
  const voicedRef = useRef(voiced);
  voicedRef.current = voiced;
  const artRef = useRef(art);
  artRef.current = art;

  // A new question: the stage seen, the text folded or not, focus to the first answer, said and cued.
  useEffect(() => {
    const opened = open && !wasOpen.current;
    wasOpen.current = open;
    if (!step) {
      setSeen(new Set());
      setStage(null);
      shown.current = null;
      return;
    }
    const now = creation.stageOf(step.kind);
    if (now) {
      setStage(now);
      setSeen((s) => (s.has(now) ? s : new Set(s).add(now)));
    }
    if (!open) return;
    const last = shown.current;
    shown.current = step;
    const fresh = creation.isNewQuestion(step, last);
    // Opened again on the same question: focus goes back to its answers.
    if (opened && !fresh) {
      window.setTimeout(() => (list.current?.querySelector<HTMLButtonElement>("button") ?? firstStat.current ?? field.current)?.focus(), 0);
    }
    if (fresh) {
      setTextShown(TEXT_SHOWN.has(step.kind));
      setTyped("");
      setAbout(step.choices[0]?.about ?? step.stats?.stats[0]?.about ?? null);
      setReached(step.choices[0]?.name ?? null);
      if (cues) earcons.creationStep();
      // After the dialog's own focus on opening (a parent's effect runs after this one).
      window.setTimeout(() => {
        quietFocus.current = true;
        const target = list.current?.querySelector<HTMLButtonElement>("button") ?? firstStat.current ?? field.current;
        target?.focus();
        quietFocus.current = false;
      }, 0);
    }
    if (voicedRef.current) {
      const said = creation.spoken(step, last);
      if (said) voice.speak(said, true);
      const first = step.choices[0];
      if (fresh && first && step.choices.length > 1 && !TYPED[step.kind]) voice.speak(creation.choiceWords(first, extraWords(step, first, artRef.current)));
    }
    // A step is a new object each time the game asks; cues and voiced are read when it comes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step, open]);

  if (!step) {
    return (
      <Dialog open={false} onClose={onClose} title="Making Your Character">
        {null}
      </Dialog>
    );
  }

  const { question, help } = creation.words(step);
  const stages = creation.shownStages(seen);
  const current = creation.stageOf(step.kind) ?? stage;
  const typedLabel = TYPED[step.kind];
  const statsTable = step.stats && step.stats.stats.length > 0 ? step.stats : null;
  const canChange = step.kind === "stats";

  const send = (line: string, secret = false) => onSend(line, secret);
  const say = (text: string) => {
    if (voicedRef.current && !quietFocus.current) voice.speak(text, true);
  };
  const focusChoice = (choice: CreationChoice) => {
    setAbout(choice.about);
    setReached(choice.name);
    say(creation.choiceWords(choice, extraWords(step, choice, art)));
  };
  // The race or class shown: the one reached in the list, or the one being confirmed.
  const shownName = step.kind === "confirmRace" || step.kind === "confirmClass" ? step.subject : reached;
  const portrait = portraitOf(step, shownName, art);
  const portraitKind = step.kind === "race" || step.kind === "confirmRace" ? "race" : step.kind === "class" || step.kind === "confirmClass" ? "class" : null;
  const impact = step.kind === "faction" && reached ? creation.factionImpact(step.subject, reached) : null;
  /** Up and Down (and Home and End) move through the choices. */
  const onListKey = (e: KeyboardEvent<HTMLUListElement>) => {
    const buttons = Array.from(list.current?.querySelectorAll<HTMLButtonElement>("button") ?? []);
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (at < 0) return;
    const to = e.key === "ArrowDown" ? (at + 1) % buttons.length : e.key === "ArrowUp" ? (at - 1 + buttons.length) % buttons.length : e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : -1;
    if (to < 0) return;
    e.preventDefault();
    buttons[to].focus();
  };
  const chosen = step.chosen.length > 0 ? `So far: ${step.chosen.map((c) => `${c.what} ${c.value}`).join(", ")}.` : "Nothing settled yet.";

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Making Your Character"
      className="dialog-wide creation-dialog"
      actions={
        <>
          <button type="button" data-testid="creation-standard" title="CoffeeMUD's own screens from now on, answers typed. The gear menu brings the guide back." onClick={onStandard}>
            Standard Mode (CoffeeMUD)
          </button>
          <button type="button" data-testid="creation-type-instead" onClick={onClose}>
            Type Instead
          </button>
        </>
      }
    >
      <ol className="creation-stages" aria-label="The steps">
        {stages.map((s) => (
          <li key={s} aria-current={s === current ? "step" : undefined} className={s === current ? "current" : undefined}>
            {s === current ? `[${s}]` : s}
          </li>
        ))}
      </ol>
      <p className="creation-chosen" data-testid="creation-chosen">
        {chosen}
      </p>
      {/* The question: read by the screen reader as it changes, unless Coupler's own voice says it. */}
      <p className="creation-question" role="status" aria-live={voiced ? "off" : "polite"} data-testid="creation-question">
        {step.problem && <span className="sr-only">{`The game says: ${step.problem} `}</span>}
        {question}
      </p>
      {/* A problem with the last answer takes the help's place: it's what matters now. */}
      <p className="creation-help" aria-hidden={step.problem ? "true" : undefined}>
        {step.problem ? <span className="creation-problem">{`The game says: ${step.problem}`}</span> : help || "\u00a0"}
      </p>

      {statsTable && (
        <div className="creation-stats-box">
          <table className="creation-stats" data-testid="creation-stats">
            <thead>
              <tr>
                <th scope="col">Stat</th>
                <th scope="col">Now</th>
                <th scope="col">Most</th>
                <th scope="col">Race</th>
                {canChange && <th scope="col">Change</th>}
              </tr>
            </thead>
            <tbody>
              {statsTable.stats.map((s, i) => {
                const words = `${s.name} ${s.value} of ${s.most}${s.race ? `, ${s.race > 0 ? "+" : ""}${s.race} from your race` : ""}.`;
                const hear = () => {
                  setAbout(s.about);
                  say(`${words} ${s.about ?? ""}`);
                };
                return (
                  <tr key={s.name}>
                    <th scope="row">{s.name}</th>
                    <td>{s.value}</td>
                    <td>{s.most}</td>
                    <td>{s.race ? `${s.race > 0 ? "+" : ""}${s.race}` : ""}</td>
                    {canChange && (
                      <td className="creation-stat-change">
                        {STAT_STEPS.map((by) => (
                          <button
                            key={by}
                            type="button"
                            ref={i === 0 && by === 1 ? firstStat : undefined}
                            aria-label={`${by > 0 ? "Raise" : "Lower"} ${s.name}${Math.abs(by) > 1 ? ` by ${Math.abs(by)}` : ""}`}
                            // Focusable while it can't be used, so the row's still reached and heard.
                            aria-disabled={!statStepFits(s, by, statsTable.points) || undefined}
                            onFocus={hear}
                            onClick={() => {
                              const why = statStepWhy(s, by, statsTable.points);
                              if (why) {
                                setAbout(why);
                                say(why);
                              } else send(`${s.name} ${by > 0 ? "+" : ""}${by}`);
                            }}
                          >
                            {by === 1 ? "+" : by === -1 ? "-" : by > 0 ? `+${by}` : `${by}`}
                          </button>
                        ))}
                      </td>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
          <div className="creation-stats-side">
            <p>{`${creation.pointsWords(statsTable.points)} Total ${statsTable.total} of ${statsTable.most}.`.trim()}</p>
            <p>{creation.qualifiesWords(statsTable)}</p>
            <p className="creation-about">{about ?? ""}</p>
          </div>
        </div>
      )}
      {canChange && (
        <div className="creation-row">
          <button type="button" onClick={() => send("R")}>
            Roll at Random
          </button>
          <button type="button" className="primary" disabled={(step.stats?.points ?? 0) > 0} onClick={() => send("")}>
            Done
          </button>
          <span className="creation-note">{(step.stats?.points ?? 0) > 0 ? "Spend every point to finish." : ""}</span>
        </div>
      )}

      {step.choices.length > 0 && (
        <div className="creation-choices">
          <ul ref={list} onKeyDown={onListKey} aria-label={question} data-testid="creation-choices">
            {step.choices.map((c) => (
              <li key={`${c.name}-${c.send}`}>
                <button
                  type="button"
                  className={c.suggested ? "suggested" : undefined}
                  aria-label={creation.choiceWords(c, extraWords(step, c, art))}
                  onFocus={() => focusChoice(c)}
                  onMouseEnter={() => {
                    setAbout(c.about);
                    setReached(c.name);
                  }}
                  onClick={() => send(c.send)}
                >
                  {c.suggested ? `${c.name} (suits you)` : c.name}
                </button>
              </li>
            ))}
          </ul>
          {!statsTable && (
            <div className="creation-side" aria-hidden="true">
              {portraitKind && shownName && <Portrait kind={portraitKind} name={shownName} source={portraits} coffeemud={portrait?.art ?? null} />}
              <div className="creation-side-words">
                {portrait?.facts && <p className="creation-facts">{portrait.facts}</p>}
                {portrait?.unlike && <p className="creation-facts">{portrait.unlike}</p>}
                {impact && <p className="creation-facts">{impact}</p>}
                <p className="creation-about">{about ?? portrait?.help ?? ""}</p>
              </div>
            </div>
          )}
        </div>
      )}


      {typedLabel && (
        <form
          className="hooks-search creation-typed"
          onSubmit={(e) => {
            e.preventDefault();
            send(typed, step.secret);
            setTyped("");
          }}
        >
          <label htmlFor={`${id}-typed`}>{typedLabel}</label>
          <input
            id={`${id}-typed`}
            ref={field}
            type={step.secret ? "password" : "text"}
            value={typed}
            autoComplete="off"
            autoCapitalize="off"
            spellCheck={false}
            data-testid="creation-typed"
            onChange={(e) => setTyped(e.target.value)}
          />
          <button type="submit" className="primary">
            Send
          </button>
        </form>
      )}

      {step.text.length > 0 && (
        <>
          <button type="button" className="creation-text-toggle" aria-expanded={textShown} aria-controls={`${id}-text`} onClick={() => setTextShown((s) => !s)}>
            {textShown ? "Hide the Game's Own Words" : "Show the Game's Own Words"}
          </button>
          {textShown && (
            <pre id={`${id}-text`} className="creation-text" tabIndex={0} aria-label="The game's own words">
              {step.text.join("\n")}
            </pre>
          )}
        </>
      )}
    </Dialog>
  );
}

type ArtBook = Awaited<ReturnType<typeof creationArt.load>>;

/** The picture and facts for a race or a class, by its name, when the question's about one. */
function portraitOf(step: CreationStep, name: string | null, art: ArtBook | null): creationArt.Portrait | null {
  if (!art || !name) return null;
  if (step.kind === "race" || step.kind === "confirmRace") return art.races[creationArt.key(name)] ?? null;
  if (step.kind === "class" || step.kind === "confirmClass") return art.classes[creationArt.key(name)] ?? null;
  return null;
}

/**
 * Coupler's own words said before a choice's: a race's facts and what
 * sets it apart, what an alignment does. A race the game's list gave no
 * words for (one a game turns on, like Pixie) has CoffeeMUD's help on it.
 */
function extraWords(step: CreationStep, choice: CreationChoice, art: ArtBook | null): string | null {
  if (step.kind === "faction") return creation.factionImpact(step.subject, choice.name);
  const p = portraitOf(step, choice.name, art);
  return p ? [p.facts, p.unlike, choice.about ? null : p.help].filter(Boolean).join(" ") || null : null;
}

/**
 * Why the game would refuse raising or lowering a stat by `by`, or null
 * if it wouldn't: CharCreation's own checks. Each point costs at least
 * one, and a stat's most (the game's 18 plus the race's change) caps it.
 * Lowering below where it started isn't known here: the game says so.
 */
function statStepWhy(s: CreationStats["stats"][number], by: number, points: number | null): string | null {
  if (by < 0) return s.value + by < 1 ? `${s.name} can't go that low.` : null;
  if (points !== null && points < by) return points === 0 ? "No points are left: lower another stat first." : `Only ${points} ${points === 1 ? "point is" : "points are"} left.`;
  if (s.value + by > s.most) return s.value >= s.most ? `${s.name} is at its most, ${s.most}.` : `${s.name} can go up only ${s.most - s.value} more.`;
  return null;
}

const statStepFits = (s: CreationStats["stats"][number], by: number, points: number | null) => statStepWhy(s, by, points) === null;

/** The painter's portraits, by kind and name and size: each painted once. */
const painted = new Map<string, Promise<mud.AnsiArt | null>>();
const PORTRAIT_SIZE = { columns: 32, rows: 16 };

function paintedPortrait(kind: "race" | "class", name: string): Promise<mud.AnsiArt | null> {
  const k = `${kind}\n${creationArt.key(name)}`;
  let art = painted.get(k);
  if (!art) {
    art = mud.portraitPaint(kind, name, PORTRAIT_SIZE.columns, PORTRAIT_SIZE.rows).catch(() => null);
    painted.set(k, art);
  }
  return art;
}

/**
 * A race's or class's portrait: Coupler's painter's (src-tauri/src/
 * portrait.rs) unless the player chose CoffeeMUD's own pictures, and
 * then the painter's where CoffeeMUD has none. Hidden from screen
 * readers; its facts are beside it in words.
 */
function Portrait({ kind, name, source, coffeemud }: { kind: "race" | "class"; name: string; source: PortraitSource; coffeemud: creationArt.Portrait["art"] | null }) {
  const [art, setArt] = useState<{ columns: number; lines: Line[] } | null>(null);
  useEffect(() => {
    if (source === "coffeemud" && coffeemud) {
      setArt({ columns: coffeemud.columns, lines: creationArt.lines(coffeemud) });
      return;
    }
    let live = true;
    void paintedPortrait(kind, name).then((a) => live && setArt(a));
    return () => {
      live = false;
    };
  }, [kind, name, source, coffeemud]);
  if (!art) return <div className="creation-picture" style={{ width: PORTRAIT_SIZE.columns * 8 }} />;
  return (
    <div className="hook-ansi creation-picture" style={{ width: art.columns * 8 }}>
      {art.lines.map((line, row) => (
        <div key={row} className="hook-ansi-row">
          {line.map((span, i) => {
            const { color, background } = spanColors(span);
            return (
              <span key={i} style={{ color, background }}>
                {span.text}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
}

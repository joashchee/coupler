/**
 * Before You Play: the tutorial (lib/tutorial.ts), a practice game in a
 * dialog that teaches Immersive a lesson at a time. Offered, never
 * forced: from the ways to play before connecting, and the gear menu.
 *
 * Coupler's voice teaches (it's Immersive's), and everything it says is
 * written here too: the lesson, what to do, the practice game's output
 * (in the game's colors), and Heard, which writes each sound's caption
 * and each line said for the game. The say keys, Cmd+Period and review
 * mode work in it as they do in the game; App's own shortcuts are off
 * while a dialog is open, so they don't reach the real game.
 */
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { onCaption } from "../lib/captions";
import { spanColors, type Line } from "../lib/mud";
import { LESSONS, Stage, type Act } from "../lib/tutorial";
import * as voice from "../lib/voice";
import { Dialog } from "./Dialog";

interface TutorialDialogProps {
  open: boolean;
  onClose: () => void;
  /** Close and play in Immersive from now on. */
  onPlayImmersive: () => void;
}

/** Lines of the practice game kept, and of Heard. */
const KEPT_LINES = 300;
const KEPT_HEARD = 60;

const textOf = (line: Line) => line.map((s) => s.text).join("").trim();

export function TutorialDialog({ open, onClose, onPlayImmersive }: TutorialDialogProps) {
  const id = useId();
  const [index, setIndex] = useState(0);
  /** Bumped by Hear It Again: the lesson starts over. */
  const [again, setAgain] = useState(0);
  const [lines, setLines] = useState<{ id: number; line: Line }[]>([]);
  const [heard, setHeard] = useState<{ id: number; text: string; sound?: boolean }[]>([]);
  const [done, setDone] = useState(false);
  /** The line review is on, as an index into `lines`, or null when live. */
  const [reviewAt, setReviewAt] = useState<number | null>(null);
  const nextId = useRef(0);
  const doneRef = useRef(false);
  const linesRef = useRef(lines);
  linesRef.current = lines;
  /** The first line after the last command typed, for Cmd+Shift+O. */
  const sentAt = useRef(0);
  const input = useRef<HTMLInputElement>(null);
  const output = useRef<HTMLDivElement>(null);
  const heardBox = useRef<HTMLOListElement>(null);
  const lesson = LESSONS[index];
  const last = index === LESSONS.length - 1;

  const markDone = useRef<() => void>(() => {});
  const stage = useRef<Stage | null>(null);
  stage.current ??= new Stage({
    write: (more) =>
      setLines((list) => {
        const all = [...list, ...more.map((line) => ({ id: nextId.current++, line }))];
        return all.length > KEPT_LINES ? all.slice(all.length - KEPT_LINES) : all;
      }),
    heard: (text) => addHeard(text),
    done: () => markDone.current(),
  });
  const addHeard = (text: string, sound?: boolean) =>
    setHeard((list) => {
      const all = [...list, { id: nextId.current++, text, sound }];
      return all.length > KEPT_HEARD ? all.slice(all.length - KEPT_HEARD) : all;
    });

  markDone.current = () => {
    if (doneRef.current) return;
    doneRef.current = true;
    setDone(true);
    const s = stage.current!;
    // After the key that finished it has had its own answer (Cmd+Period's hush comes after this).
    if (lesson.well) s.later(150, () => s.say(`${lesson.well} ${last ? "" : "Press Return for the next lesson."}`));
  };

  // Each lesson: set the scene, teach, play it, then say what to do.
  useEffect(() => {
    if (!open) return;
    const s = stage.current!;
    s.stop();
    voice.hush();
    doneRef.current = false;
    setDone(false);
    setReviewAt(null);
    s.reviews = 0;
    const l = LESSONS[index];
    l.begin?.(s);
    const sayTask = () => {
      if (!doneRef.current) s.say(l.task);
    };
    s.say(`Lesson ${index + 1} of ${LESSONS.length}: ${l.title}. ${l.teach}`, () => (l.play ? l.play(s, sayTask) : sayTask()));
    // After the dialog's own focus, so the command box has it.
    const t = window.setTimeout(() => input.current?.focus(), 0);
    return () => window.clearTimeout(t);
  }, [open, index, again]);

  // Closing stops it all, and the next time it opens it starts from the first lesson.
  useEffect(() => {
    if (!open) return;
    return () => {
      stage.current?.stop();
      voice.hush();
      setIndex(0);
      setLines([]);
      setHeard([]);
      sentAt.current = 0;
    };
  }, [open]);

  useEffect(() => (open ? onCaption((text) => addHeard(text, true)) : undefined), [open]);

  useLayoutEffect(() => {
    const el = output.current;
    if (el && reviewAt === null) el.scrollTop = el.scrollHeight;
  }, [lines, reviewAt]);
  useLayoutEffect(() => {
    const el = heardBox.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [heard]);

  const act = useCallback(
    (a: Act) => {
      const s = stage.current!;
      if (!doneRef.current && LESSONS[index].finished(a, s)) markDone.current();
    },
    [index],
  );

  /** Review mode, as the game's: Option+Up the line before, Option+Down the line after, Option+End back to live. */
  const review = useCallback(
    (how: "back" | "forward" | "live") => {
      const s = stage.current!;
      const list = linesRef.current;
      const worded = list.map((l, i) => (textOf(l.line) === "" ? -1 : i)).filter((i) => i >= 0);
      if (how === "live" || worded.length === 0) {
        setReviewAt(null);
        s.answer(worded.length === 0 ? "Nothing to read back yet." : reviewAt === null ? "Already live." : "Live.");
        return;
      }
      const at = reviewAt === null ? (how === "back" ? worded[worded.length - 1] : null) : how === "back" ? [...worded].reverse().find((i) => i < reviewAt) ?? worded[0] : worded.find((i) => i > reviewAt) ?? null;
      if (at === null) {
        setReviewAt(null);
        s.answer("Live.");
        return;
      }
      setReviewAt(at);
      const text = textOf(list[at].line);
      s.answer(voice.speakable(text) ? text : "A line of symbols.");
      document.getElementById(`${id}-line-${list[at].id}`)?.scrollIntoView({ block: "nearest" });
      if (how === "back") act({ key: "review" });
    },
    [reviewAt, act, id],
  );

  // The game's keys, inside the tutorial.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      const s = stage.current!;
      // App's own listener does the hushing; this only notices it.
      if (e.metaKey && !e.ctrlKey && !e.shiftKey && !e.altKey && e.key === ".") return act({ key: "hush" });
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && !e.altKey) {
        const k = e.key.toLowerCase();
        if (k === "l" || k === "v" || k === "e") {
          e.preventDefault();
          const key = k === "l" ? "where" : k === "v" ? "vitals" : "enemy";
          s.key(key);
          act({ key });
        } else if (k === "w") {
          e.preventDefault();
          s.answer("In the game, Cmd+Shift+W says who's online. This practice game is just you.");
        } else if (k === "o") {
          e.preventDefault();
          const said = linesRef.current.filter((l) => l.id >= sentAt.current).map((l) => textOf(l.line)).filter(voice.speakable).slice(-15);
          s.answer(said.length > 0 ? said.join(" ") : "The game hasn't said anything since your last command.");
        }
        return;
      }
      if (e.altKey && !e.metaKey && !e.ctrlKey && (e.key === "ArrowUp" || e.key === "ArrowDown" || e.key === "End")) {
        e.preventDefault();
        review(e.key === "ArrowUp" ? "back" : e.key === "ArrowDown" ? "forward" : "live");
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, act, review]);

  const go = (to: number) => {
    if (to < 0 || to >= LESSONS.length) return;
    setIndex(to);
  };

  const submit = () => {
    const field = input.current;
    const text = field?.value ?? "";
    if (field) field.value = "";
    const s = stage.current!;
    if (text.trim() === "") {
      if (doneRef.current && !last) go(index + 1);
      else s.answer(lesson.task);
      return;
    }
    setReviewAt(null);
    sentAt.current = nextId.current;
    if (!lesson.command?.(text, s)) s.command(text);
    act({ command: text });
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Before You Play"
      className="dialog-wide tutorial-dialog"
      actions={
        <>
          <button type="button" data-testid="tutorial-back" disabled={index === 0} onClick={() => go(index - 1)}>
            Back
          </button>
          <button type="button" data-testid="tutorial-again" onClick={() => setAgain((n) => n + 1)}>
            Hear It Again
          </button>
          {last ? (
            <button type="button" className="primary" data-testid="tutorial-immersive" onClick={onPlayImmersive}>
              Play in Immersive
            </button>
          ) : (
            <button type="button" className="primary" data-testid="tutorial-next" onClick={() => go(index + 1)}>
              {done ? "Next Lesson" : "Skip to Next"}
            </button>
          )}
          <button type="button" data-testid="tutorial-close" onClick={onClose}>
            Close
          </button>
        </>
      }
    >
      <p className="tutorial-lesson" data-testid="tutorial-lesson">{`Lesson ${index + 1} of ${LESSONS.length}: ${lesson.title}`}</p>
      <p className="tutorial-teach">{lesson.teach}</p>
      <div
        ref={output}
        className="tutorial-output"
        data-testid="tutorial-output"
        tabIndex={0}
        role="region"
        aria-label="The practice game's output. Option+Up reads it back."
      >
        {lines.map((l, i) => (
          <div key={l.id} id={`${id}-line-${l.id}`} className={`terminal-line${i === reviewAt ? " reviewed" : ""}`} aria-current={i === reviewAt ? "true" : undefined}>
            {l.line.map((span, j) => {
              const { color, background } = spanColors(span);
              return (
                <span key={j} style={{ color, background }}>
                  {span.text}
                </span>
              );
            })}
          </div>
        ))}
      </div>
      <p className={`tutorial-task${done ? " done" : ""}`} id={`${id}-task`} data-testid="tutorial-task">
        {done ? (last ? lesson.task : `Done. ${lesson.well} Press Return for the next lesson.`) : `Try it: ${lesson.task}`}
      </p>
      <div className="hooks-search tutorial-command">
        <label htmlFor={`${id}-command`}>Command</label>
        <input
          ref={input}
          id={`${id}-command`}
          type="text"
          data-testid="tutorial-input"
          aria-describedby={`${id}-task`}
          placeholder="Type a command and press Return"
          autoComplete="off"
          spellCheck={false}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.metaKey && !e.ctrlKey && !e.altKey) {
              e.preventDefault();
              submit();
            }
          }}
        />
      </div>
      <p className="tutorial-heard-title" id={`${id}-heard`}>
        Heard: what the voice said for the game, and every sound in words
      </p>
      <ol className="tutorial-heard" ref={heardBox} tabIndex={0} aria-labelledby={`${id}-heard`} data-testid="tutorial-heard">
        {heard.map((h) => (
          <li key={h.id} className={h.sound ? "heard-caption" : undefined}>
            {h.text}
          </li>
        ))}
      </ol>
    </Dialog>
  );
}

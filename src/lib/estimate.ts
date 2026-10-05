/**
 * Estimated completion for a progress bar (CLAUDE.md's "Feedback for every
 * user activity" convention). ProgressBar calls this for every determinate
 * bar, so a new bar gets an estimate without extra work.
 *
 * The rate is measured from the first value seen, not from zero, so a bar
 * that mounts partway (an activity row appearing after its grace period, a
 * scan pass whose total arrives late) isn't skewed. The estimate is held
 * steady and only redone when the work crosses a milestone (20, 40, 60, 80,
 * 90%) or every 10 seconds, so the text doesn't jitter with each event.
 * A value that drops (a scan moving on to its next pass) starts over.
 */
import { useEffect, useRef, useState } from "react";

const MILESTONES = [0.2, 0.4, 0.6, 0.8, 0.9];
const REESTIMATE_MS = 10_000;
/** Measurements over less time than this are noise, and work this short needs no estimate. */
const MIN_ELAPSED_MS = 1_000;

export interface CompletionEstimate {
  /** When the work should finish (ms since epoch). */
  finishAt: number;
  /** Remaining time when the estimate was made. */
  remainingMs: number;
}

function milestonesPassed(value: number): number {
  return MILESTONES.filter((m) => value >= m).length;
}

/** `value` is 0–1, or undefined while the work can't be counted (no estimate then). */
export function useCompletionEstimate(value: number | undefined): CompletionEstimate | null {
  const [estimate, setEstimate] = useState<CompletionEstimate | null>(null);
  const baseline = useRef<{ at: number; value: number } | null>(null);
  const last = useRef<{ at: number; milestones: number }>({ at: 0, milestones: 0 });
  const latest = useRef(value);
  latest.current = value;

  function check(force: boolean) {
    const v = latest.current;
    const now = Date.now();
    if (v === undefined || v >= 1) {
      baseline.current = null;
      setEstimate(null);
      return;
    }
    const base = baseline.current;
    if (!base || v < base.value) {
      baseline.current = { at: now, value: v };
      last.current = { at: now, milestones: milestonesPassed(v) };
      setEstimate(null);
      return;
    }
    const elapsed = now - base.at;
    const milestones = milestonesPassed(v);
    const due = force || milestones > last.current.milestones || now - last.current.at >= REESTIMATE_MS;
    if (!due || elapsed < MIN_ELAPSED_MS) return;
    last.current = { at: now, milestones };
    const done = v - base.value;
    // No progress since the baseline: nothing to estimate from yet.
    if (done <= 0) return;
    const remainingMs = ((1 - v) * elapsed) / done;
    setEstimate({ finishAt: now + remainingMs, remainingMs });
  }

  useEffect(() => check(false), [value]);

  // The 10-second re-estimate also has to happen when no new value arrives
  // (one big file holding a pass up): the estimate then grows, as it should.
  useEffect(() => {
    const timer = window.setInterval(() => check(false), 1_000);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return estimate;
}

/** "About 4 min left, done around 14:32". */
export function describeEstimate(e: CompletionEstimate): string {
  const s = e.remainingMs / 1000;
  let left: string;
  if (s < 10) left = "A few seconds left";
  else if (s < 60) left = `About ${Math.round(s / 5) * 5} seconds left`;
  else if (s < 3600) left = `About ${Math.round(s / 60)} min left`;
  else {
    const h = Math.floor(s / 3600);
    const m = Math.round((s % 3600) / 60);
    left = `About ${h} h${m ? ` ${m} min` : ""} left`;
  }
  if (s < 60) return left;
  const finish = new Date(e.finishAt);
  const sameDay = finish.toDateString() === new Date().toDateString();
  const clock = finish.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  return `${left}, done around ${sameDay ? clock : `${finish.toLocaleDateString([], { weekday: "short" })} ${clock}`}`;
}

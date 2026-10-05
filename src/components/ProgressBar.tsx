/**
 * One reusable progress bar for every "this is taking a moment" moment in
 * the app: startup, exports, conversions, and every other user activity via
 * ActivityStatus (lib/activity.ts). Same component/CSS as Diskette's, so
 * a new indeterminate or timed use doesn't need a new design:
 *
 * - `value` (0–1) — a determinate bar for countable work: rows of a PNG
 *   export, cells of an image conversion, frames of an animation encode,
 *   parts of a theme-pack render.
 * - `durationMs` — a bar that fills itself from 0% to 100% over a known
 *   duration via a CSS animation. Pass a changing `key` at the call site to
 *   restart it (a fresh mount re-triggers the CSS animation; toggling a
 *   style property on the same element wouldn't).
 * - `indeterminate` — a sweeping bar for work with no knowable length or
 *   count, e.g. loading a file before its size is known.
 *
 * A determinate or timed bar shows its estimated completion below itself
 * ("About 4 min left, done around 14:32"; lib/estimate.ts). A bar with no
 * room for that line (the startup screen) passes `hideEstimate` and must
 * show `useCompletionEstimate`'s result somewhere else instead.
 */
import { useState, type CSSProperties } from "react";
import { describeEstimate, useCompletionEstimate, type CompletionEstimate } from "../lib/estimate";

interface ProgressBarProps {
  value?: number;
  durationMs?: number;
  indeterminate?: boolean;
  label: string;
  className?: string;
  hideEstimate?: boolean;
}

export function ProgressBar({ value, durationMs, indeterminate, label, className, hideEstimate }: ProgressBarProps) {
  const mode = indeterminate ? "indeterminate" : durationMs !== undefined ? "timed" : "determinate";
  const pct = value !== undefined ? Math.max(0, Math.min(100, Math.round(value * 100))) : undefined;
  const fillStyle: CSSProperties | undefined =
    mode === "timed" ? { animationDuration: `${durationMs}ms` } : mode === "determinate" ? { width: `${pct ?? 0}%` } : undefined;
  const measured = useCompletionEstimate(mode === "determinate" ? value : undefined);
  // A timed bar knows its finish from the start (remounted per run via `key`).
  const [timed] = useState<CompletionEstimate | null>(() =>
    durationMs !== undefined ? { finishAt: Date.now() + durationMs, remainingMs: durationMs } : null,
  );
  const estimate = mode === "timed" ? timed : mode === "determinate" ? measured : null;
  const estimateText = estimate ? describeEstimate(estimate) : undefined;

  return (
    <>
      <div
        className={`progress-bar progress-bar-${mode}${className ? ` ${className}` : ""}`}
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={mode === "determinate" ? pct : undefined}
        aria-valuetext={mode === "determinate" && estimateText ? `${pct}%. ${estimateText}` : undefined}
      >
        <div className="progress-bar-fill" style={fillStyle} />
      </div>
      {estimateText && !hideEstimate && <span className="progress-eta">{estimateText}</span>}
    </>
  );
}

/**
 * Feedback for anything the user sets off (CLAUDE.md's "Feedback for every
 * user activity" convention). Wrap the work in `runActivity` and it shows
 * in the activity area (components/ActivityStatus.tsx) with a ProgressBar
 * until it settles, so a click never leads to a silent wait:
 *
 * - Countable work (rows deleted, files read): call `update({ value })`
 *   with 0–1 as it goes and the bar turns determinate.
 * - Work with no knowable length (compressing a backup, mounting an
 *   image): leave `value` unset and the bar sweeps.
 *
 * `key` names the thing being worked on (e.g. `delete:<volumeId>`), so its
 * own control can show it's busy via `isBusy(key)`.
 *
 * The finished result is the caller's to report (showStatus, or setError on
 * failure); this only covers the wait.
 */
import { useCallback, useRef, useState } from "react";

export interface Activity {
  id: number;
  label: string;
  /** 0–1 once the work is countable; unset keeps the bar indeterminate. */
  value?: number;
  key?: string;
}

export type ActivityUpdate = (patch: { label?: string; value?: number }) => void;

export function useActivities() {
  const [activities, setActivities] = useState<Activity[]>([]);
  const nextId = useRef(0);

  const runActivity = useCallback(async <T>(label: string, task: (update: ActivityUpdate) => Promise<T>, key?: string): Promise<T> => {
    const id = ++nextId.current;
    setActivities((list) => [...list, { id, label, key }]);
    const update: ActivityUpdate = (patch) => setActivities((list) => list.map((a) => (a.id === id ? { ...a, ...patch } : a)));
    try {
      return await task(update);
    } finally {
      setActivities((list) => list.filter((a) => a.id !== id));
    }
  }, []);

  const isBusy = useCallback((key: string) => activities.some((a) => a.key === key), [activities]);

  return { activities, runActivity, isBusy };
}

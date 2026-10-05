/**
 * The activity area: one label and ProgressBar per running activity
 * (lib/activity.ts). A row appears only once its activity has run for
 * SHOW_AFTER_MS, so instant actions don't flash a bar; anything slower
 * than that is on screen before the wait is noticeable.
 */
import { useEffect, useState } from "react";
import type { Activity } from "../lib/activity";
import { ProgressBar } from "./ProgressBar";

const SHOW_AFTER_MS = 150;

function ActivityRow({ activity }: { activity: Activity }) {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const timer = window.setTimeout(() => setShown(true), SHOW_AFTER_MS);
    return () => window.clearTimeout(timer);
  }, []);
  if (!shown) return null;
  return (
    <div className="scan-status-row" data-testid="activity-row">
      <span className="scan-status-label">{activity.label}</span>
      <ProgressBar indeterminate={activity.value === undefined} value={activity.value} label={activity.label} />
    </div>
  );
}

export function ActivityStatus({ activities }: { activities: Activity[] }) {
  return (
    <div aria-live="polite">
      {activities.map((a) => (
        <ActivityRow key={a.id} activity={a} />
      ))}
    </div>
  );
}

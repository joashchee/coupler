import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { Dialog } from "./Dialog";
import { CHECKLIST_DATA } from "../lib/testChecklist";

interface AppTestingProps {
  open: boolean;
  onClose: () => void;
}

interface ResultEntry {
  status: "done" | "bug";
  note?: string;
}

const RESULTS_KEY = "coupler.appTestingResults";

function loadResults(): Record<string, ResultEntry> {
  try {
    const raw = localStorage.getItem(RESULTS_KEY);
    return raw ? (JSON.parse(raw) as Record<string, ResultEntry>) : {};
  } catch {
    return {};
  }
}

/**
 * In-app testing checklist, copied from Diskette (CLAUDE.md's Testing
 * section). Dev-only: App.tsx mounts this only behind
 * `import.meta.env.DEV`, so it never reaches a production `tauri build`.
 * Marked items drop out of view (Cue's own behavior) — "Show marked"
 * brings them back to review or clear.
 */
export function AppTesting({ open, onClose }: AppTestingProps) {
  const [results, setResults] = useState<Record<string, ResultEntry>>(loadResults);
  const [bugDraftId, setBugDraftId] = useState<string | null>(null);
  const [bugDraftText, setBugDraftText] = useState("");
  const [showMarked, setShowMarked] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  useEffect(() => {
    try {
      localStorage.setItem(RESULTS_KEY, JSON.stringify(results));
    } catch {
      // localStorage unavailable — results just won't persist across relaunches.
    }
  }, [results]);

  const bySection = useMemo(() => {
    const map = new Map<string, typeof CHECKLIST_DATA>();
    for (const item of CHECKLIST_DATA) {
      const list = map.get(item.section) ?? [];
      list.push(item);
      map.set(item.section, list);
    }
    return map;
  }, []);

  const doneCount = Object.values(results).filter((r) => r.status === "done").length;
  const bugCount = Object.values(results).filter((r) => r.status === "bug").length;
  const leftCount = CHECKLIST_DATA.length - doneCount - bugCount;

  function markDone(id: string) {
    setResults((r) => ({ ...r, [id]: { status: "done" } }));
    setBugDraftId(null);
  }

  function startBug(id: string) {
    setBugDraftId(id);
    setBugDraftText(results[id]?.note ?? "");
  }

  function confirmBug() {
    if (!bugDraftId) return;
    setResults((r) => ({ ...r, [bugDraftId]: { status: "bug", note: bugDraftText.trim() || undefined } }));
    setBugDraftId(null);
    setBugDraftText("");
  }

  function clearItem(id: string) {
    setResults((r) => {
      const next = { ...r };
      delete next[id];
      return next;
    });
  }

  /** Closes the overlay, then scrolls to and briefly highlights the jump target — mirrors Cue's "Go to app" button. */
  function goTo(selector?: string) {
    if (!selector) return;
    onClose();
    setTimeout(() => {
      const el = document.querySelector(selector);
      if (!el) return;
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      el.classList.add("testing-highlight");
      setTimeout(() => el.classList.remove("testing-highlight"), 1500);
    }, 150);
  }

  async function handleExport() {
    setExportError(null);
    const defaultPath = `coupler-app-testing-${new Date().toISOString().slice(0, 10)}.txt`;
    const path = await save({ defaultPath, filters: [{ name: "Text", extensions: ["txt"] }] });
    if (!path) return;
    const lines: string[] = [`Coupler App Testing results — v${__APP_VERSION__}`, `Exported ${new Date().toLocaleString()}`, ""];
    for (const [section, items] of bySection) {
      lines.push(section);
      for (const item of items) {
        const r = results[item.id];
        const status = r ? (r.status === "done" ? "Done" : "Bug") : "Not tested";
        lines.push(`  [${status}] ${item.label}`);
        if (r?.status === "bug" && r.note) lines.push(`    Note: ${r.note}`);
      }
      lines.push("");
    }
    try {
      // Checklist status/notes only — no game text, no device/account identifying
      // data (CLAUDE.md rule 1, extended to this dev-only tool).
      await invoke("export_app_testing_report", { path, report: lines.join("\n") });
    } catch (e) {
      setExportError(String(e));
    }
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="App Testing"
      actions={
        <>
          <button type="button" onClick={() => setShowMarked((v) => !v)}>
            {showMarked ? "Hide marked" : `Show marked (${doneCount + bugCount})`}
          </button>
          <button type="button" onClick={() => void handleExport()}>
            Export results…
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Close
          </button>
        </>
      }
    >
      <p className="about-section-desc">
        Dev-only checklist — never part of a shipped build. Mark each item Done or Bug as you exercise it; Export writes a
        plain-text report (checklist status/notes only, no game text).
      </p>
      <p className="testing-tally">
        <span className="status-pill">{leftCount} left</span>
        <span className="status-pill">{doneCount} done</span>
        <span className="status-pill">
          {bugCount} bug{bugCount === 1 ? "" : "s"}
        </span>
      </p>
      {exportError && <p className="error">{exportError}</p>}

      {[...bySection.entries()].map(([section, items]) => {
        const visible = items.filter((item) => showMarked || !results[item.id]);
        if (visible.length === 0) return null;
        return (
          <div key={section}>
            <h3 className="about-section-title">{section}</h3>
            <ul className="testing-item-list">
              {visible.map((item) => {
                const r = results[item.id];
                return (
                  <li key={item.id} className={r ? `testing-item testing-item-${r.status}` : "testing-item"}>
                    <span className="testing-item-label">{item.label}</span>
                    <span className="testing-item-actions">
                      {item.selector && (
                        <button type="button" className="small" onClick={() => goTo(item.selector)}>
                          Go to app
                        </button>
                      )}
                      {r ? (
                        <button type="button" className="small" onClick={() => clearItem(item.id)}>
                          Clear
                        </button>
                      ) : (
                        <>
                          <button type="button" className="small" onClick={() => markDone(item.id)}>
                            Done
                          </button>
                          <button type="button" className="small danger" onClick={() => startBug(item.id)}>
                            Bug
                          </button>
                        </>
                      )}
                    </span>
                    {r?.status === "bug" && r.note && <p className="testing-item-note">{r.note}</p>}
                    {bugDraftId === item.id && (
                      <div className="testing-bug-draft">
                        <textarea
                          value={bugDraftText}
                          onChange={(e) => setBugDraftText(e.currentTarget.value)}
                          placeholder="What went wrong?"
                          rows={2}
                        />
                        <button type="button" className="small primary" onClick={confirmBug}>
                          Save bug
                        </button>
                        <button type="button" className="small" onClick={() => setBugDraftId(null)}>
                          Cancel
                        </button>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
        );
      })}
    </Dialog>
  );
}

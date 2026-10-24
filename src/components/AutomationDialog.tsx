/**
 * Triggers, Gags, Aliases and Tickers (gear menu, every way to play,
 * both builds): the player's own rules (lib/automation.ts, run by
 * src-tauri/src/automation.rs), a tab each.
 *
 * - **Triggers**: the game's last ten lines (gagged ones too), each with
 *   Create…, which opens the trigger's editor over the list: patterns
 *   offered for the line (`automation_suggest`) or the player's own, and
 *   once one's chosen, what happens (commands, words said, a sound, the
 *   line gagged) and Add Trigger. Below, the triggers, each to turn off,
 *   edit or delete. New Trigger… starts one without a line.
 * - **Gags**: the last ten lines shown, each with Gag (that line), a
 *   pattern of the player's own, and the gags, each with Delete.
 * - **Aliases** and **Tickers**: the list, and an editor whose commands
 *   box gives tips as the player types (lib/commandWords.ts: CoffeeMUD's
 *   commands, skills and spells, their usage and a line about each).
 *
 * The tabs are a tablist (arrows move between them, rule 10); each list
 * is a list of rows whose buttons say their row; the tips are buttons,
 * and only what the line's command is reaches the screen reader, once
 * it changes. The phone and the tablet have it as it is, from the gear
 * menu (the phone's All Settings…).
 */
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import * as automation from "../lib/automation";
import * as words from "../lib/commandWords";
import * as mud from "../lib/mud";
import * as assets from "../lib/assets";
import * as voice from "../lib/voice";
import { WEB } from "../lib/web";
import { Dialog } from "./Dialog";

type Tab = "triggers" | "gags" | "aliases" | "tickers";
const TABS: [Tab, string][] = [
  ["triggers", "Triggers"],
  ["gags", "Gags"],
  ["aliases", "Aliases"],
  ["tickers", "Tickers"],
];
/** How many of the last lines each list shows. */
const SHOWN = 10;

interface AutomationDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

/** What a trigger does, in short. */
function doesInWords(t: mud.LineTrigger): string {
  const parts: string[] = [];
  if (t.send.trim()) parts.push(`sends ${automation.commandsInLine(t.send)}`);
  if (t.say.trim()) parts.push(`says "${t.say.trim()}"`);
  if (t.sound) parts.push(`plays ${mud.assetTitle(t.sound)}`);
  if (t.gag) parts.push("gags the line");
  return parts.length > 0 ? parts.join(", ") : "does nothing yet";
}

export function AutomationDialog({ open, onClose, onStatus, onError }: AutomationDialogProps) {
  const id = useId();
  const [tab, setTab] = useState<Tab>("triggers");
  const [recent, setRecent] = useState<mud.RecentLine[]>([]);
  const [, setVersion] = useState(0);
  useEffect(() => automation.onRulesChanged(() => setVersion((n) => n + 1)), []);
  // The trigger being made from a line ("" for none), or edited.
  const [trigger, setTrigger] = useState<{ line: string; rule: mud.LineTrigger | null } | null>(null);
  const [alias, setAlias] = useState<mud.Alias | null>(null);
  const [ticker, setTicker] = useState<mud.Ticker | null>(null);
  const [gagPattern, setGagPattern] = useState("");

  useEffect(() => {
    if (!open) {
      setTrigger(null);
      setAlias(null);
      setTicker(null);
      return;
    }
    let live = true;
    mud
      .automationRecent()
      .then((r) => live && setRecent(r))
      .catch(() => {});
    const off = mud.onAutomationRecent((r) => live && setRecent(r));
    return () => {
      live = false;
      void off.then((f) => f());
    };
  }, [open]);

  const rules = automation.currentRules();
  const lastLines = useMemo(() => recent.slice(-SHOWN).reverse(), [recent]);
  const lastShown = useMemo(
    () =>
      recent
        .filter((r) => !r.gagged)
        .slice(-SHOWN)
        .reverse(),
    [recent],
  );

  const onTabKey = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const at = TABS.findIndex(([which]) => which === tab);
    const other = TABS[(at + (e.key === "ArrowRight" ? 1 : TABS.length - 1)) % TABS.length][0];
    setTab(other);
    document.getElementById(`${id}-tab-${other}`)?.focus();
  };

  const addGag = (pattern: string) => {
    const problem = automation.patternProblem(pattern);
    if (problem) return onError(problem);
    if (rules.gags.some((g) => g.pattern.trim().toLowerCase() === pattern.trim().toLowerCase())) return onStatus(`Already gagged: ${pattern.trim()}`);
    automation.saveRule("gags", { id: automation.newId(), pattern: pattern.trim(), on: true });
    onStatus(`Gagged: lines like "${pattern.trim()}" no longer show.`);
  };

  const recentList = (lines: mud.RecentLine[], label: string, button: string, act: (r: mud.RecentLine) => void, testid: string) => (
    <ul className="hooks-list automation-recent" data-testid={testid} aria-label={label}>
      {lines.length === 0 && <li className="hook-value automation-empty">No lines from the game yet. Play a little, and the last {SHOWN} show here.</li>}
      {lines.map((r, i) => (
        <li key={`${i}-${r.text}`}>
          <button type="button" className="hook-edit" aria-label={`${button}: ${r.text}`} onClick={() => act(r)}>
            {button}
          </button>
          <span className={r.gagged ? "hook-value" : "hook-key"}>
            {r.text}
            {r.gagged ? " (gagged)" : ""}
          </span>
        </li>
      ))}
    </ul>
  );

  const covered = trigger !== null || alias !== null || ticker !== null;
  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        title="Triggers, Gags, Aliases and Tickers"
        className="dialog-wide"
        covered={open && covered}
        actions={
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        }
      >
        <div className="hooks-tabs" role="tablist" aria-label="Which rules">
          {TABS.map(([which, label]) => (
            <button
              key={which}
              type="button"
              role="tab"
              id={`${id}-tab-${which}`}
              className={`hooks-tab${tab === which ? " selected" : ""}`}
              data-testid={`automation-tab-${which}`}
              aria-selected={tab === which}
              aria-controls={`${id}-panel`}
              tabIndex={tab === which ? 0 : -1}
              onClick={() => setTab(which)}
              onKeyDown={onTabKey}
            >
              <span className="hooks-tab-mark" aria-hidden="true">
                •
              </span>
              {label} ({rules[which].length})
            </button>
          ))}
        </div>
        <div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-tab-${tab}`}>
          {tab === "triggers" && (
            <>
              <p>
                A trigger does something each time the game sends a line that matches. Create one from a line below, or start a new one. In a pattern, *
                is any text; what it stands for is %1, %2… in what the trigger does.
              </p>
              <h3 className="automation-heading">The game's last lines</h3>
              {recentList(lastLines, "The game's last lines, the newest first", "Create…", (r) => setTrigger({ line: r.text, rule: null }), "automation-recent-triggers")}
              <div className="automation-bar">
                <h3 className="automation-heading">Your triggers</h3>
                <button type="button" data-testid="automation-new-trigger" onClick={() => setTrigger({ line: "", rule: null })}>
                  New Trigger…
                </button>
              </div>
              <RuleList
                kind="triggers"
                empty="No triggers yet."
                rows={rules.triggers.map((t) => ({ id: t.id, on: t.on, name: t.pattern, does: doesInWords(t) }))}
                onToggle={(rid, on) => {
                  const t = rules.triggers.find((r) => r.id === rid);
                  if (t) automation.saveRule("triggers", { ...t, on });
                }}
                onEdit={(rid) => {
                  const t = rules.triggers.find((r) => r.id === rid);
                  if (t) setTrigger({ line: "", rule: t });
                }}
                onDelete={(rid) => {
                  automation.deleteRule("triggers", rid);
                  onStatus("Trigger deleted.");
                }}
              />
            </>
          )}
          {tab === "gags" && (
            <>
              <p>A gagged line isn't shown, said or read. Gag one of the game's last lines, or write a pattern of your own: * is any text.</p>
              <h3 className="automation-heading">The game's last lines shown</h3>
              {recentList(lastShown, "The game's last lines shown, the newest first", "Gag", (r) => addGag(r.text), "automation-recent-gags")}
              <div className="hooks-search">
                <label htmlFor={`${id}-gag`}>Your own</label>
                <input
                  id={`${id}-gag`}
                  type="text"
                  data-testid="automation-gag-pattern"
                  placeholder="The wind blows*"
                  value={gagPattern}
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck={false}
                  onChange={(e) => setGagPattern(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      addGag(gagPattern);
                      setGagPattern("");
                    }
                  }}
                />
                <button
                  type="button"
                  data-testid="automation-gag-add"
                  onClick={() => {
                    addGag(gagPattern);
                    setGagPattern("");
                  }}
                >
                  Gag
                </button>
              </div>
              <h3 className="automation-heading">Your gags</h3>
              <RuleList
                kind="gags"
                empty="Nothing gagged."
                rows={rules.gags.map((g) => ({ id: g.id, on: g.on, name: g.pattern, does: "not shown" }))}
                onToggle={(rid, on) => {
                  const g = rules.gags.find((r) => r.id === rid);
                  if (g) automation.saveRule("gags", { ...g, on });
                }}
                onDelete={(rid) => {
                  automation.deleteRule("gags", rid);
                  onStatus("Gag deleted: those lines show again.");
                }}
              />
            </>
          )}
          {tab === "aliases" && (
            <>
              <div className="automation-bar">
                <p>
                  An alias is a word you type that stands for commands. Type \ first to send a line as it is. Aliases work only in the game, never into
                  a password.
                </p>
                <button type="button" data-testid="automation-new-alias" onClick={() => setAlias({ id: automation.newId(), name: "", send: "", on: true })}>
                  New Alias…
                </button>
              </div>
              <RuleList
                kind="aliases"
                empty="No aliases yet."
                rows={rules.aliases.map((a) => ({ id: a.id, on: a.on, name: a.name, does: automation.commandsInLine(a.send) }))}
                onToggle={(rid, on) => {
                  const a = rules.aliases.find((r) => r.id === rid);
                  if (a) automation.saveRule("aliases", { ...a, on });
                }}
                onEdit={(rid) => setAlias(rules.aliases.find((r) => r.id === rid) ?? null)}
                onDelete={(rid) => {
                  automation.deleteRule("aliases", rid);
                  onStatus("Alias deleted.");
                }}
              />
            </>
          )}
          {tab === "tickers" && (
            <>
              <div className="automation-bar">
                <p>
                  A ticker sends commands every so often while a character is in the game: every {mud.MIN_EVERY} seconds at most, and it waits while
                  you've been away 9 minutes, so the game's idle timers still work.
                  {WEB && " In a browser tab you aren't looking at, the browser may hold one back up to a minute."}
                </p>
                <button type="button" data-testid="automation-new-ticker" onClick={() => setTicker({ id: automation.newId(), every: 300, send: "", on: true })}>
                  New Ticker…
                </button>
              </div>
              <RuleList
                kind="tickers"
                empty="No tickers yet."
                rows={rules.tickers.map((k) => ({ id: k.id, on: k.on, name: automation.everyInWords(k.every), does: automation.commandsInLine(k.send) }))}
                onToggle={(rid, on) => {
                  const k = rules.tickers.find((r) => r.id === rid);
                  if (k) automation.saveRule("tickers", { ...k, on });
                }}
                onEdit={(rid) => setTicker(rules.tickers.find((r) => r.id === rid) ?? null)}
                onDelete={(rid) => {
                  automation.deleteRule("tickers", rid);
                  onStatus("Ticker deleted.");
                }}
              />
            </>
          )}
        </div>
      </Dialog>
      <TriggerEditor making={open ? trigger : null} onClose={() => setTrigger(null)} onStatus={onStatus} onError={onError} />
      <AliasEditor alias={open ? alias : null} onClose={() => setAlias(null)} onStatus={onStatus} onError={onError} />
      <TickerEditor ticker={open ? ticker : null} onClose={() => setTicker(null)} onStatus={onStatus} onError={onError} />
    </>
  );
}

interface Row {
  id: string;
  on: boolean;
  /** The pattern, the word or the wait. */
  name: string;
  does: string;
}

function RuleList({ kind, rows, empty, onToggle, onEdit, onDelete }: { kind: string; rows: Row[]; empty: string; onToggle: (id: string, on: boolean) => void; onEdit?: (id: string) => void; onDelete: (id: string) => void }) {
  return (
    <ul className={`hooks-list automation-rules${onEdit ? "" : " automation-rules-fixed"}`} data-testid={`automation-${kind}`} aria-label={`Your ${kind}`}>
      {rows.length === 0 && <li className="hook-value automation-empty">{empty}</li>}
      {rows.map((r) => (
        <li key={r.id}>
          {onEdit && (
            <button type="button" className="hook-edit" aria-label={`Edit: ${r.name}`} onClick={() => onEdit(r.id)}>
              Edit…
            </button>
          )}
          <button type="button" className="hook-edit" aria-label={`Delete: ${r.name}`} onClick={() => onDelete(r.id)}>
            Delete
          </button>
          <input type="checkbox" checked={r.on} aria-label={`On: ${r.name}`} onChange={(e) => onToggle(r.id, e.currentTarget.checked)} />
          <span className={r.on ? "hook-key" : "hook-value"}>{r.name}</span>
          <span className="hook-value">{r.does}</span>
        </li>
      ))}
    </ul>
  );
}

/** A sound a trigger can play: the player's SFX and CoffeeMUD's own one-shots. */
interface SoundChoice {
  path: string;
  label: string;
}

function useSounds(open: boolean): SoundChoice[] {
  const [sounds, setSounds] = useState<SoundChoice[]>([]);
  useEffect(() => {
    if (!open) return;
    let live = true;
    Promise.all([mud.assetsList().catch(() => null), mud.soundbankList().catch(() => [])]).then(([own, bank]) => {
      if (!live) return;
      const mine = (own?.assets ?? []).filter((a) => a.kind === "sfx").map((a) => ({ path: a.path, label: `${mud.assetTitle(a.path)} (yours)` }));
      const theirs = bank.filter((b) => !b.looped).map((b) => ({ path: b.path, label: `${mud.assetTitle(b.path)} (CoffeeMUD's)` }));
      setSounds([...mine, ...theirs]);
    });
    return () => {
      live = false;
    };
  }, [open]);
  return sounds;
}

interface TriggerEditorProps {
  /** The line it's made from ("" for none) or the trigger edited; null when closed. */
  making: { line: string; rule: mud.LineTrigger | null } | null;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

function TriggerEditor({ making, onClose, onStatus, onError }: TriggerEditorProps) {
  const uid = useId();
  const open = making !== null;
  const line = making?.line ?? "";
  const editing = making?.rule ?? null;
  const [suggestions, setSuggestions] = useState<mud.Suggestion[]>([]);
  // Which pattern: a suggestion's index, or -1 for the player's own.
  const [chosen, setChosen] = useState<number | null>(null);
  const [own, setOwn] = useState("");
  const [send, setSend] = useState("");
  const [say, setSay] = useState("");
  const [sound, setSound] = useState("");
  const [gag, setGag] = useState(false);
  const [captures, setCaptures] = useState<string[] | null>(null);
  const sounds = useSounds(open);

  useEffect(() => {
    if (!making) return;
    const rule = making.rule;
    setSend(rule?.send ?? "");
    setSay(rule?.say ?? "");
    setSound(rule?.sound ?? "");
    setGag(rule?.gag ?? false);
    setOwn(rule?.pattern ?? making.line);
    setChosen(rule || making.line === "" ? -1 : null);
    setSuggestions([]);
    if (making.line !== "")
      mud
        .automationSuggest(making.line)
        .then(setSuggestions)
        .catch(() => {});
  }, [making]);

  const pattern = chosen === null ? "" : chosen < 0 ? own : (suggestions[chosen]?.pattern ?? "");
  useEffect(() => {
    if (line === "" || pattern.trim() === "") return setCaptures(null);
    let live = true;
    mud
      .automationMatch(pattern, line)
      .then((c) => live && setCaptures(c))
      .catch(() => live && setCaptures(null));
    return () => {
      live = false;
    };
  }, [pattern, line]);

  const confirm = () => {
    const problem = automation.patternProblem(pattern);
    if (problem) return onError(problem);
    if (!send.trim() && !say.trim() && !sound && !gag) return onError("Choose something for the trigger to do: commands, words, a sound or a gag.");
    automation.saveRule("triggers", { id: editing?.id ?? automation.newId(), pattern: pattern.trim(), send, say, sound: sound || null, gag, on: editing?.on ?? true });
    onStatus(editing ? `Trigger saved: ${pattern.trim()}` : `Trigger added: ${pattern.trim()}`);
    onClose();
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={editing ? "Edit the Trigger" : "Create a Trigger"}
      className="dialog-wide"
      actions={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="primary" data-testid="trigger-confirm" disabled={chosen === null} onClick={confirm}>
            {editing ? "Save Trigger" : "Add Trigger"}
          </button>
        </>
      }
    >
      {line !== "" && <p className="hook-key">The line: {line}</p>}
      <fieldset className="radio-group automation-patterns">
        <legend>Which lines set it off</legend>
        {suggestions.map((s, i) => (
          <label key={s.pattern} className="check-row">
            <input type="radio" name={`${uid}-pattern`} checked={chosen === i} onChange={() => setChosen(i)} />
            <span>
              <span className="hook-key">{s.pattern}</span> <span className="hook-value">{s.about}</span>
            </span>
          </label>
        ))}
        <label className="check-row">
          <input type="radio" name={`${uid}-pattern`} data-testid="trigger-own" checked={chosen === -1} onChange={() => setChosen(-1)} />
          <span>Your own pattern</span>
        </label>
        {chosen === -1 && (
          <input
            type="text"
            className="automation-own"
            aria-label="Your own pattern: the whole line, * any text"
            data-testid="trigger-pattern"
            value={own}
            placeholder="* says 'heal me'"
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            onChange={(e) => setOwn(e.currentTarget.value)}
          />
        )}
      </fieldset>
      {line !== "" && pattern.trim() !== "" && (
        <p className="hook-value" role="status">
          {captures === null
            ? "This pattern doesn't match the line."
            : captures.length === 0
              ? "It matches the line."
              : `It matches the line: ${captures.map((c, i) => `%${i + 1} is "${c}"`).join(", ")}.`}
        </p>
      )}
      {chosen !== null && (
        <div className="hook-edit-rows">
          <CommandsField label="Commands it sends" value={send} onChange={setSend} of="trigger" />
          <div className="hook-edit-row">
            <label htmlFor={`${uid}-say`}>The narrator says</label>
            <input
              id={`${uid}-say`}
              type="text"
              className="hook-asset"
              data-testid="trigger-say"
              value={say}
              placeholder="%1 wants healing"
              autoComplete="off"
              onChange={(e) => setSay(e.currentTarget.value)}
            />
            <button type="button" className="automation-row-button" disabled={say.trim() === ""} onClick={() => voice.speak(say.replace(/%(\d)/g, (_, n: string) => captures?.[Number(n) - 1] ?? ""), true)}>
              Hear It
            </button>
          </div>
          {sounds.length > 0 && (
            <div className="hook-edit-row">
              <label htmlFor={`${uid}-sound`}>A sound</label>
              <select id={`${uid}-sound`} className="hook-asset" data-testid="trigger-sound" value={sound} onChange={(e) => setSound(e.currentTarget.value)}>
                <option value="">None</option>
                {sounds.map((s) => (
                  <option key={s.path} value={s.path}>
                    {s.label}
                  </option>
                ))}
              </select>
              <button type="button" className="automation-row-button" disabled={sound === ""} onClick={() => void assets.playPreview(sound, "sfx")}>
                Play
              </button>
            </div>
          )}
          <label className="check-row">
            <input type="checkbox" data-testid="trigger-gag" checked={gag} onChange={(e) => setGag(e.currentTarget.checked)} />
            <span>Gag the line too: it isn't shown, said or read</span>
          </label>
          <p className="hook-value">Commands go only while a character is in the game, never into a password; a trigger answering its own line is held back.</p>
        </div>
      )}
    </Dialog>
  );
}

interface CommandsFieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  of: "alias" | "ticker" | "trigger";
  /** The alias being written, which can't use itself. */
  self?: string;
}

/** A box of commands, one a line, with tips for the line being typed. */
function CommandsField({ label, value, onChange, of, self }: CommandsFieldProps) {
  const uid = useId();
  const box = useRef<HTMLTextAreaElement>(null);
  const book = useWords();
  const [caret, setCaret] = useState(0);
  const start = value.lastIndexOf("\n", caret - 1) + 1;
  const endAt = value.indexOf("\n", caret);
  const end = endAt < 0 ? value.length : endAt;
  const current = value.slice(start, end);
  const tips = words.tipsFor(current, book, automation.currentRules().aliases, self);
  const notes = [...tips.notes, ...words.sendNotes(value, of)];
  const knownWords = tips.known ? `${tips.known.label}: ${tips.known.usage || "no usage given"}. ${tips.known.about}` : "";

  const put = (line: string) => {
    const next = value.slice(0, start) + line + value.slice(end);
    onChange(next);
    const at = start + line.length;
    requestAnimationFrame(() => {
      box.current?.focus();
      box.current?.setSelectionRange(at, at);
      setCaret(at);
    });
  };

  return (
    <div className="automation-commands">
      <label htmlFor={`${uid}-send`}>{label}, one a line</label>
      <textarea
        id={`${uid}-send`}
        ref={box}
        rows={4}
        data-testid={`${of}-send`}
        aria-describedby={`${uid}-known`}
        value={value}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
        onChange={(e) => {
          onChange(e.currentTarget.value);
          setCaret(e.currentTarget.selectionStart);
        }}
        onSelect={(e) => setCaret(e.currentTarget.selectionStart)}
      />
      {/* What the line's command is: said once it changes. */}
      <p id={`${uid}-known`} className="automation-known" aria-live="polite">
        {knownWords}
      </p>
      {tips.known && tips.known.examples.length > 0 && <p className="hook-value">For example: {tips.known.examples.join("; ")}</p>}
      {tips.suggestions.length > 0 && (
        <div className="automation-tips" role="group" aria-label="Suggestions for this line">
          {tips.suggestions.map((s) => (
            <button key={s.label} type="button" className="automation-tip" title={`${s.usage} ${s.about}`.trim()} aria-label={`${s.label}. ${s.usage}. ${s.about}`} onClick={() => put(s.line)}>
              {s.label}
            </button>
          ))}
        </div>
      )}
      {notes.map((n) => (
        <p key={n} className="hook-value">
          {n}
        </p>
      ))}
    </div>
  );
}

interface AliasEditorProps {
  alias: mud.Alias | null;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

function AliasEditor({ alias, onClose, onStatus, onError }: AliasEditorProps) {
  const uid = useId();
  const [name, setName] = useState("");
  const [send, setSend] = useState("");
  useEffect(() => {
    setName(alias?.name ?? "");
    setSend(alias?.send ?? "");
  }, [alias]);
  const book = useWords();
  const clash = book && name.trim() !== "" ? book.commands.find((c) => c.words.includes(name.trim().toUpperCase())) : undefined;
  const save = () => {
    if (!alias) return;
    const next = { ...alias, name: name.trim(), send };
    const problem = automation.aliasProblem(next, automation.currentRules().aliases);
    if (problem) return onError(problem);
    automation.saveRule("aliases", next);
    onStatus(`Alias saved: ${next.name} sends ${automation.commandsInLine(send)}.`);
    onClose();
  };
  return (
    <Dialog
      open={alias !== null}
      onClose={onClose}
      title="Alias"
      className="dialog-wide"
      actions={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="primary" data-testid="alias-save" onClick={save}>
            Save Alias
          </button>
        </>
      }
    >
      <div className="hook-edit-rows">
        <div className="hook-edit-row">
          <label htmlFor={`${uid}-name`}>The word you type</label>
          <input
            id={`${uid}-name`}
            type="text"
            className="hook-asset"
            data-testid="alias-name"
            value={name}
            placeholder="kk"
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            onChange={(e) => setName(e.currentTarget.value)}
          />
          <span className="hook-value" aria-live="polite">
            {clash ? `${name.trim().toUpperCase()} is CoffeeMUD's own ${clash.kind}: the alias takes its place (type \\${name.trim()} for the game's).` : ""}
          </span>
        </div>
        <CommandsField label="Commands it stands for" value={send} onChange={setSend} of="alias" self={name.trim()} />
      </div>
    </Dialog>
  );
}

function useWords(): words.Words | null {
  const [book, setBook] = useState<words.Words | null>(null);
  useEffect(() => {
    words
      .loadWords()
      .then(setBook)
      .catch(() => {});
  }, []);
  return book;
}

interface TickerEditorProps {
  ticker: mud.Ticker | null;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

function TickerEditor({ ticker, onClose, onStatus, onError }: TickerEditorProps) {
  const uid = useId();
  const [every, setEvery] = useState("300");
  const [send, setSend] = useState("");
  useEffect(() => {
    setEvery(String(ticker?.every ?? 300));
    setSend(ticker?.send ?? "");
  }, [ticker]);
  const seconds = Math.round(Number(every));
  const save = () => {
    if (!ticker) return;
    const next = { ...ticker, every: seconds, send };
    const problem = automation.tickerProblem(next);
    if (problem) return onError(problem);
    automation.saveRule("tickers", next);
    onStatus(`Ticker saved: ${automation.commandsInLine(send)}, ${automation.everyInWords(seconds)}.`);
    onClose();
  };
  return (
    <Dialog
      open={ticker !== null}
      onClose={onClose}
      title="Ticker"
      className="dialog-wide"
      actions={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="primary" data-testid="ticker-save" onClick={save}>
            Save Ticker
          </button>
        </>
      }
    >
      <div className="hook-edit-rows">
        <div className="hook-edit-row">
          <label htmlFor={`${uid}-every`}>Every, in seconds</label>
          <input
            id={`${uid}-every`}
            type="number"
            className="hook-asset"
            data-testid="ticker-every"
            min={mud.MIN_EVERY}
            step={1}
            value={every}
            onChange={(e) => setEvery(e.currentTarget.value)}
          />
          <span className="hook-value" aria-live="polite">
            {Number.isFinite(seconds) && seconds >= mud.MIN_EVERY ? `Sent ${automation.everyInWords(seconds)}.` : `At least ${mud.MIN_EVERY} seconds.`}
          </span>
        </div>
        <CommandsField label="Commands it sends" value={send} onChange={setSend} of="ticker" />
      </div>
    </Dialog>
  );
}

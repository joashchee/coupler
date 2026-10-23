/**
 * The player's triggers, gags, aliases and tickers, kept here as
 * `coupler.automation` (so a backup has them) and handed to Rust
 * (src-tauri/src/automation.rs, `automation_set`), which runs them: at
 * launch (`startAutomation`) and on every change. Edited in gear →
 * Triggers, Gags, Aliases and Tickers… (components/AutomationDialog.tsx).
 * Both builds.
 */
import * as mud from "./mud";

const KEY = "coupler.automation";

const EMPTY: mud.Rules = { triggers: [], gags: [], aliases: [], tickers: [] };
const listeners = new Set<() => void>();

const text = (v: unknown) => (typeof v === "string" ? v : "");
const list = (v: unknown): Record<string, unknown>[] => (Array.isArray(v) ? v.filter((x): x is Record<string, unknown> => typeof x === "object" && x !== null) : []);

function load(): mud.Rules {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, unknown>;
    return {
      triggers: list(raw.triggers).map((t) => ({
        id: text(t.id) || newId(),
        pattern: text(t.pattern),
        send: text(t.send),
        say: text(t.say),
        sound: text(t.sound) || null,
        gag: t.gag === true,
        on: t.on !== false,
      })),
      gags: list(raw.gags).map((g) => ({ id: text(g.id) || newId(), pattern: text(g.pattern), on: g.on !== false })),
      aliases: list(raw.aliases).map((a) => ({ id: text(a.id) || newId(), name: text(a.name), send: text(a.send), on: a.on !== false })),
      tickers: list(raw.tickers).map((k) => ({
        id: text(k.id) || newId(),
        every: Math.max(mud.MIN_EVERY, Math.round(Number(k.every) || 60)),
        send: text(k.send),
        on: k.on !== false,
      })),
    };
  } catch {
    return EMPTY;
  }
}

let made = 0;
/** An ID no other rule has. */
export function newId(): string {
  made += 1;
  return `${Date.now().toString(36)}-${made}`;
}

// Read once the helpers above exist.
let rules: mud.Rules = load();

function store() {
  try {
    localStorage.setItem(KEY, JSON.stringify(rules));
  } catch {
    // Not kept past this run, then.
  }
  mud.automationSet(rules).catch(() => {});
  listeners.forEach((f) => f());
}

/** Hands the kept rules to Rust, at launch. */
export function startAutomation() {
  mud.automationSet(rules).catch(() => {});
}

// Another tab of the web build saved its rules: this one takes them, so
// its next save can't put back what that tab changed.
if (typeof window !== "undefined") {
  window.addEventListener("storage", (e) => {
    if (e.key !== KEY) return;
    rules = load();
    mud.automationSet(rules).catch(() => {});
    listeners.forEach((f) => f());
  });
}

export const currentRules = () => rules;

export function onRulesChanged(f: () => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

type Kind = keyof mud.Rules;
type RuleOf<K extends Kind> = mud.Rules[K][number];

/** Adds a rule, or puts it in place of the one with its ID. */
export function saveRule<K extends Kind>(kind: K, rule: RuleOf<K>) {
  const now = rules[kind] as RuleOf<K>[];
  const at = now.findIndex((r) => r.id === rule.id);
  const next = at < 0 ? [...now, rule] : now.map((r, i) => (i === at ? rule : r));
  rules = { ...rules, [kind]: next };
  store();
}

export function deleteRule(kind: Kind, id: string) {
  rules = { ...rules, [kind]: (rules[kind] as { id: string }[]).filter((r) => r.id !== id) };
  store();
}

/** What's wrong with a pattern, in words, or null. */
export function patternProblem(pattern: string): string | null {
  const p = pattern.trim();
  if (p === "") return "Write the line to match.";
  if (/^\**$/.test(p)) return "A pattern of only * would match every line the game sends.";
  return null;
}

/** What's wrong with an alias, in words, or null. */
export function aliasProblem(alias: mud.Alias, others: mud.Alias[]): string | null {
  const name = alias.name.trim();
  if (name === "") return "Give the alias a word to type.";
  if (/\s/.test(name)) return "An alias is one word, without spaces.";
  if (name.startsWith("\\")) return "An alias can't start with \\: that sends a line as it is.";
  if (others.some((a) => a.id !== alias.id && a.name.trim().toLowerCase() === name.toLowerCase())) return `There's already an alias ${name}.`;
  if (alias.send.trim() === "") return "Write the commands it stands for.";
  return null;
}

/** What's wrong with a ticker, in words, or null. */
export function tickerProblem(ticker: mud.Ticker): string | null {
  if (!Number.isFinite(ticker.every) || ticker.every < mud.MIN_EVERY) return `A ticker waits at least ${mud.MIN_EVERY} seconds, to be gentle to the game.`;
  if (ticker.send.trim() === "") return "Write the commands it sends.";
  return null;
}

/** "every 90 seconds", "every 5 minutes", "every 1 hour 30 minutes". */
export function everyInWords(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const part = (n: number, one: string) => (n === 0 ? "" : `${n} ${one}${n === 1 ? "" : "s"}`);
  if (seconds < 120) return `every ${seconds} seconds`;
  return `every ${[part(h, "hour"), part(m, "minute"), part(s, "second")].filter(Boolean).join(" ")}`;
}

/** A rule's commands in one line: "kill %1; bash %1". */
export const commandsInLine = (send: string) =>
  send
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean)
    .join("; ");

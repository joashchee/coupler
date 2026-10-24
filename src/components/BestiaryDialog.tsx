/**
 * The Bestiary (gear menu, Cmd+Shift+B): every creature and every item
 * met in the game last played, and what's been learned of each
 * (src-tauri/src/bestiary.rs), the most lately seen first. Two tabs,
 * Creatures and Items (Left and Right move between them); Search finds a
 * name, a race or material, or an area. A long list is cut short with the
 * full count above it, as in the hooks list.
 *
 * Each row: the name in the game's colors on black, as the game output
 * shows it (CoffeeMUD's `^` codes, read by `ansi::caret_line`), its race or level in short, the areas it's been
 * seen in, then Show…, which opens everything known of it over the list,
 * a fact a line (lib/bestiary.ts), and its picture by Coupler's painter
 * beside them, 200 by 192 (25 by 12 cells): a creature's face (src-tauri/src/face.rs: its
 * race, description and what it wears) or the item (item_art.rs: its
 * name, what it's made of); hidden from screen readers, its facts being
 * the lines beside it. An item's picture shows what's on it (glowing,
 * invisible, floating…) and writes its slot on its bottom row; both are
 * among the facts too. Both read again as the game teaches
 * Coupler more while they're open. The list isn't a live region: it
 * changes all the time while playing.
 *
 * Above the tabs, the player's choice of Coupler looking at each new
 * creature by itself, unobtrusively (the room isn't told), its answer
 * kept but not shown.
 */
import { memo, useCallback, useEffect, useId, useState, type KeyboardEvent } from "react";
import * as bestiary from "../lib/bestiary";
import { loadDisplay } from "../lib/display";
import * as mud from "../lib/mud";
import { Dialog } from "./Dialog";

interface BestiaryDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

type Tab = "creatures" | "items";
const TABS: { key: Tab; name: string }[] = [
  { key: "creatures", name: "Creatures" },
  { key: "items", name: "Items" },
];

/** Rows drawn at most; Search narrows the rest down. */
const SHOWN = 500;

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** A row as the list shows it, either kind. */
interface Row {
  key: string;
  name: string;
  /** The name in the game's colors, drawn on black as the game output is. */
  styled: mud.Line;
  summary: string;
  areas: string;
}

export const BestiaryDialog = memo(function BestiaryDialog({ open, onClose, onStatus, onError }: BestiaryDialogProps) {
  const id = useId();
  const [looking, setLooking] = useState(bestiary.autoLook);
  const [tab, setTab] = useState<Tab>("creatures");
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<Row[] | null>(null);
  /** The key of the entry shown, and what's known of it. */
  const [showing, setShowing] = useState<string | null>(null);
  const [entry, setEntry] = useState<{ name: string; facts: string[]; picture: mud.AnsiArt | null } | null>(null);
  /** The names' colors as the player sees the game's (gear → Display…). */
  const colors = open ? loadDisplay().colors : "game";

  const read = useCallback(() => {
    const areas = (a: string[]) => a.join(", ");
    const listed =
      tab === "creatures"
        ? mud.bestiaryMobs(query).then((r) => r.map((m) => ({ key: m.key, name: m.name, styled: m.styled, summary: bestiary.mobSummary(m), areas: areas(m.areas) })))
        : mud.bestiaryItems(query).then((r) => r.map((i) => ({ key: i.key, name: i.name, styled: i.styled, summary: bestiary.itemSummary(i), areas: areas(i.areas) })));
    listed.then(setRows).catch((e) => onError(String(e)));
  }, [tab, query, onError]);

  const readEntry = useCallback(() => {
    if (showing === null) return setEntry(null);
    const found =
      tab === "creatures"
        ? Promise.all([mud.bestiaryMob(showing), mud.bestiaryFace(showing)]).then(([m, picture]) => m && { name: m.name, facts: bestiary.mobFacts(m), picture })
        : Promise.all([mud.bestiaryItem(showing), mud.bestiaryItemArt(showing)]).then(([i, painted]) => i && { name: i.name, facts: bestiary.itemFacts(i, painted?.slot ?? null), picture: painted?.art ?? null });
    found.then(setEntry).catch((e) => onError(String(e)));
  }, [tab, showing, onError]);

  // Read again as the game teaches more; a burst is read once.
  useEffect(() => {
    if (!open) {
      setShowing(null);
      return;
    }
    read();
    readEntry();
    let timer: number | null = null;
    const unlisten = mud.onBestiaryChanged(() => {
      if (timer === null) {
        timer = window.setTimeout(() => {
          timer = null;
          read();
          readEntry();
        }, 500);
      }
    });
    return () => {
      if (timer !== null) window.clearTimeout(timer);
      void unlisten.then((f) => f());
    };
  }, [open, read, readEntry]);

  function onTabKey(e: KeyboardEvent<HTMLButtonElement>) {
    const at = TABS.findIndex((t) => t.key === tab);
    const to = e.key === "ArrowRight" ? at + 1 : e.key === "ArrowLeft" ? at + TABS.length - 1 : e.key === "Home" ? 0 : e.key === "End" ? TABS.length - 1 : -1;
    if (to < 0) return;
    e.preventDefault();
    const next = TABS[to % TABS.length];
    setTab(next.key);
    setRows(null);
    document.getElementById(`${id}-tab-${next.key}`)?.focus();
  }

  const creatures = tab === "creatures";
  const all = rows ?? [];
  const shown = all.slice(0, SHOWN);
  const narrowed = query.trim() !== "";
  const summary = !rows
    ? "Reading the Bestiary…"
    : all.length === 0
      ? narrowed
        ? "Nothing matches."
        : creatures
          ? "No creatures yet. Each one you come across in the game is added, and what you learn of it as you play."
          : "No items yet. Each one you see lying about, take from a body, or see worn is added."
      : shown.length < all.length
        ? `Showing the first ${shown.length} of ${all.length}. Type in Search to narrow them down.`
        : `${plural(all.length, creatures ? "creature" : "item", creatures ? "creatures" : "items")}${narrowed ? " match" : ""}, the most lately seen first.`;
  const what = creatures ? "Race" : "Level";

  return (
    <>
      <Dialog open={open} onClose={onClose} title="Bestiary" className="dialog-wide dialog-hooks" covered={showing !== null}>
        <p>
          Every creature and item you come across in this game and what you learn of each: where it's found, its race and level, what it does, drops
          and wears. LOOK, LL, CONSIDER and EXAMINE (a body too) tell more; Coupler keeps it all, quietly.
        </p>
        <label className="check-row bestiary-look">
          <input
            type="checkbox"
            data-testid="bestiary-look"
            checked={looking}
            onChange={(e) => {
              const on = e.currentTarget.checked;
              setLooking(on);
              bestiary
                .setAutoLook(on)
                .then(() => onStatus(on ? "Coupler looks at each new creature once, unobtrusively, and keeps what it sees." : "Coupler no longer looks at creatures by itself."))
                .catch((err) => onError(String(err)));
            }}
          />
          <span>Look at each new creature once, unobtrusively (the room isn't told), and keep what's seen without showing it</span>
        </label>
        <div className="hooks-tabs" role="tablist" aria-label="Creatures or items">
          {TABS.map((t) => (
            <button
              key={t.key}
              type="button"
              role="tab"
              id={`${id}-tab-${t.key}`}
              className={`hooks-tab${tab === t.key ? " selected" : ""}`}
              data-testid={`bestiary-tab-${t.key}`}
              aria-selected={tab === t.key}
              aria-controls={`${id}-panel`}
              tabIndex={tab === t.key ? 0 : -1}
              onClick={() => {
                setTab(t.key);
                setRows(null);
              }}
              onKeyDown={onTabKey}
            >
              <span className="hooks-tab-mark" aria-hidden="true">
                •
              </span>
              {t.name}
            </button>
          ))}
        </div>
        <div className="hooks-search" role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-tab-${tab}`}>
          <label htmlFor={`${id}-search`}>Search</label>
          <input
            id={`${id}-search`}
            type="text"
            data-testid="bestiary-search"
            aria-describedby={`${id}-summary`}
            placeholder={creatures ? "A name, a race or an area" : "A name, what it's made of, or an area"}
            value={query}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            onChange={(e) => setQuery(e.currentTarget.value)}
          />
        </div>
        <p id={`${id}-summary`} className="hooks-summary" data-testid="bestiary-summary">
          {summary}
        </p>
        {/* The columns' names, for the eye; each row says its own. */}
        <div className="hooks-head bestiary-row" aria-hidden="true">
          <span>Name</span>
          <span>{what}</span>
          <span>Found in</span>
          <span />
        </div>
        <ul className="hooks-list bestiary-list" data-testid="bestiary-list" tabIndex={0} aria-label={`${creatures ? "Creatures" : "Items"}: name, ${what.toLowerCase()}, and where found`}>
          {open &&
            shown.map((r) => (
              <li key={r.key} className="bestiary-row">
                <span className="hook-key bestiary-name">
                  {r.styled.length === 0
                    ? r.name
                    : r.styled.map((span, i) => {
                        const { color, background } = mud.spanColors(span, colors);
                        return (
                          <span key={i} style={{ color, background }}>
                            {span.text}
                          </span>
                        );
                      })}
                </span>
                <span className="hook-value">{r.summary}</span>
                <span className="hook-value">
                  <span className="sr-only">Found in </span>
                  {r.areas || "Nowhere known"}
                </span>
                <button type="button" className="hook-edit" data-testid="bestiary-show" aria-label={`Show everything known of ${r.name}`} onClick={() => setShowing(r.key)}>
                  Show…
                </button>
              </li>
            ))}
        </ul>
      </Dialog>
      {/* Beside the list's dialog, not in it, as with the hook editor. */}
      <Dialog
        open={open && showing !== null && entry !== null}
        onClose={() => setShowing(null)}
        title={entry?.name ?? (creatures ? "Creature" : "Item")}
        className="dialog-wide"
        actions={
          <button type="button" className="primary" onClick={() => setShowing(null)}>
            Close
          </button>
        }
      >
        <div className="bestiary-entry">
          {entry?.picture && (
            <div className="hook-ansi bestiary-picture" data-testid="bestiary-picture" aria-hidden="true" style={{ width: entry.picture.columns * 8 }}>
              {entry.picture.lines.map((line, row) => (
                <div key={row} className="hook-ansi-row">
                  {line.map((span, i) => {
                    const { color, background } = mud.spanColors(span);
                    return (
                      <span key={i} style={{ color, background }}>
                        {span.text}
                      </span>
                    );
                  })}
                </div>
              ))}
            </div>
          )}
          <ul className="bestiary-facts" data-testid="bestiary-facts" tabIndex={0} aria-label={`Everything known of ${entry?.name ?? "it"}`}>
            {entry?.facts.map((f, i) => (
              <li key={i}>{f}</li>
            ))}
          </ul>
        </div>
      </Dialog>
    </>
  );
});

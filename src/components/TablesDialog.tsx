/**
 * **Tables** (gear menu, Cmd+Shift+T; the phone's Here sheet): every
 * table of the world last played, kept by src-tauri/src/tables.rs.
 * Coupler's own come first (Loot: what AUTOLOOT took off a body, filled
 * as the game says it, src-tauri/src/loot.rs); then the player's, made
 * here with a name and up to five columns, rows added here or by `#add`.
 * `#check <table>` says one; `#check loot`, the last fight's loot.
 *
 * One table shows at a time, chosen from a list, its rows the latest
 * first; each row's Delete comes first (the row's own buttons lead). The
 * columns' names head the list for the eye; each row's name for VoiceOver
 * says them with its cells. The list isn't a live region: loot coming in
 * while it's open is read in quietly.
 */
import { useCallback, useEffect, useId, useState, type CSSProperties } from "react";
import { prefix } from "../lib/commands";
import * as mud from "../lib/mud";
import { Dialog } from "./Dialog";

interface TablesDialogProps {
  open: boolean;
  onClose: () => void;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

/** Rows shown at most, the latest first. */
const SHOWN = 500;

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** When, short: "Oct 24, 14:02". */
const when = (ms: number) => {
  const d = new Date(ms);
  return `${d.toLocaleDateString(undefined, { month: "short", day: "numeric" })}, ${d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}`;
};

/** A row in words, for VoiceOver: "Item a club, From the body of an orc, …". */
const rowWords = (table: mud.Table, row: mud.TableRow) =>
  [...table.columns.map((c, i) => (row.cells[i] ? `${c} ${row.cells[i]}` : "")).filter(Boolean), when(row.at)].join(", ");

export function TablesDialog({ open, onClose, onStatus, onError }: TablesDialogProps) {
  const id = useId();
  const [list, setList] = useState<mud.TableOverview[] | null>(null);
  const [chosen, setChosen] = useState<string>(mud.LOOT_TABLE);
  const [table, setTable] = useState<mud.Table | null>(null);
  const [cells, setCells] = useState<string[]>([]);
  const [making, setMaking] = useState(false);
  const [name, setName] = useState("");
  const [columns, setColumns] = useState("");
  /** Deleting or clearing the table shown, asked first. */
  const [asking, setAsking] = useState<"delete" | "clear" | null>(null);
  const [busy, setBusy] = useState(false);

  const read = useCallback(() => {
    mud.tablesList().then(setList, (e) => onError(String(e)));
  }, [onError]);

  useEffect(() => {
    if (!open) return;
    read();
    const unlisten = mud.onTablesChanged(read);
    return () => void unlisten.then((f) => f());
  }, [open, read]);

  // The table chosen, again whenever the list says something changed.
  useEffect(() => {
    if (!open || !list) return;
    const there = list.some((t) => t.id === chosen) ? chosen : (list[0]?.id ?? null);
    if (there !== chosen && there) setChosen(there);
    if (!there) return setTable(null);
    let stale = false;
    mud.tableGet(there).then(
      (t) => !stale && setTable(t),
      (e) => onError(String(e)),
    );
    return () => {
      stale = true;
    };
  }, [open, list, chosen, onError]);

  useEffect(() => setCells([]), [chosen]);

  async function change(c: mud.TableChange, done: string): Promise<string | null> {
    setBusy(true);
    try {
      const made = await mud.tableChange(c);
      onStatus(done);
      return made;
    } catch (e) {
      onError(String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }

  async function create() {
    const cols = columns.split(",").map((c) => c.trim()).filter(Boolean);
    const made = await change({ kind: "create", name, columns: cols }, `Made the table ${name.trim()}.`);
    if (made) {
      setMaking(false);
      setName("");
      setColumns("");
      // The list with it in, and it chosen, at once: chosen before the list has it, it would fall back to the first.
      const all = await mud.tablesList().catch(() => null);
      if (all) setList(all);
      setChosen(made);
    }
  }

  async function addRow() {
    if (!table) return;
    setBusy(true);
    try {
      await mud.tableChange({ kind: "add", id: table.id, cells });
      onStatus(`Added a row to ${table.name}.`);
      setCells([]);
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const rows = table ? table.rows.slice(-SHOWN).reverse() : [];
  const summary = !list
    ? "Reading the tables…"
    : !table
      ? ""
      : table.rows.length === 0
        ? table.id === mud.LOOT_TABLE
          ? "No loot yet. With AUTOLOOT on in the game, everything you take off a body is listed here the moment it's taken."
          : `${table.name} has no rows yet: add one below, or type ${prefix()}add ${table.name.toLowerCase()} and its cells split by |.`
        : `${plural(table.rows.length, "row", "rows")}${table.rows.length > SHOWN ? `, the latest ${SHOWN} shown` : ""}, the latest first.`;
  const grid = { "--table-columns": table?.columns.length ?? 1 } as CSSProperties;
  const own = table !== null && !table.coupler;

  return (
    <>
      <Dialog
        open={open}
        onClose={onClose}
        title="Tables"
        className="dialog-wide dialog-hooks"
        covered={making || asking !== null}
        actions={
          <>
            <button type="button" data-testid="tables-new" onClick={() => setMaking(true)}>
              New Table…
            </button>
            {table && (
              <button type="button" data-testid="tables-forget" disabled={busy || (!own && table.rows.length === 0)} onClick={() => setAsking(own ? "delete" : "clear")}>
                {own ? "Delete Table…" : `Clear ${table.name}…`}
              </button>
            )}
            <button type="button" className="primary" onClick={onClose}>
              Close
            </button>
          </>
        }
      >
        <p>
          Coupler's tables of what it reads in the game, and your own. Loot is what you take off a body with AUTOLOOT: {prefix()}check loot (or LOOK
          LOOT) says what your last fight brought. {prefix()}check and a table's name says any table; {prefix()}add adds a row to one of yours.
        </p>
        <div className="hook-edit-row tables-choose">
          <label htmlFor={`${id}-table`}>Table</label>
          <select id={`${id}-table`} className="hook-asset" data-testid="tables-choose" value={chosen} onChange={(e) => setChosen(e.currentTarget.value)}>
            {(list ?? []).map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
                {t.coupler ? " (Coupler's)" : ""}, {plural(t.rows, "row", "rows")}
              </option>
            ))}
          </select>
        </div>
        <p id={`${id}-summary`} className="hooks-summary" data-testid="tables-summary">
          {summary}
        </p>
        {table && (
          <>
            {/* The columns' names, for the eye; each row says its own. */}
            <div className="hooks-head tables-row" style={grid} aria-hidden="true">
              <span />
              {table.columns.map((c) => (
                <span key={c}>{c}</span>
              ))}
              <span>When</span>
            </div>
            <ul className={`hooks-list tables-list${own ? " tables-list-own" : ""}`} style={grid} data-testid="tables-rows" tabIndex={0} aria-label={`${table.name}: ${table.columns.join(", ")}, and when`} aria-describedby={`${id}-summary`}>
              {open &&
                rows.map((r) => (
                  <li key={r.id} className="tables-row">
                    <button
                      type="button"
                      className="hook-edit"
                      data-testid="tables-row-delete"
                      disabled={busy}
                      aria-label={`Delete: ${rowWords(table, r)}`}
                      onClick={() => void change({ kind: "deleteRow", id: table.id, row: r.id }, `Deleted a row of ${table.name}.`)}
                    >
                      Delete
                    </button>
                    {table.columns.map((c, i) => (
                      <span key={c} className={i === 0 ? "hook-key" : "hook-value"}>
                        {r.cells[i] ?? ""}
                      </span>
                    ))}
                    <span className="hook-value">{when(r.at)}</span>
                  </li>
                ))}
            </ul>
            {own && (
              <fieldset className="radio-group tables-add" data-testid="tables-add">
                <legend>Add a row</legend>
                <div className="tables-add-cells" style={grid}>
                  {table.columns.map((c, i) => (
                    <div key={c} className="tables-add-cell">
                      <label htmlFor={`${id}-cell-${i}`}>{c}</label>
                      <input
                        id={`${id}-cell-${i}`}
                        type="text"
                        className="hook-asset"
                        autoComplete="off"
                        value={cells[i] ?? ""}
                        onChange={(e) => {
                          const next = [...cells];
                          next[i] = e.currentTarget.value;
                          setCells(next);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") void addRow();
                        }}
                      />
                    </div>
                  ))}
                </div>
                <button type="button" data-testid="tables-add-row" disabled={busy || !cells.some((c) => c.trim() !== "")} onClick={() => void addRow()}>
                  Add Row
                </button>
              </fieldset>
            )}
          </>
        )}
      </Dialog>
      <Dialog
        open={open && making}
        onClose={() => setMaking(false)}
        title="New Table"
        actions={
          <>
            <button type="button" onClick={() => setMaking(false)}>
              Cancel
            </button>
            <button type="button" className="primary" data-testid="tables-create" disabled={busy || name.trim() === "" || columns.trim() === ""} onClick={() => void create()}>
              Make It
            </button>
          </>
        }
      >
        <p>A table of your own: its name, and the names of its columns (up to {mud.MOST_TABLE_COLUMNS}), split by commas.</p>
        <div className="hook-edit-rows">
          <div className="hook-edit-row">
            <label htmlFor={`${id}-name`}>Name</label>
            <input id={`${id}-name`} type="text" className="hook-asset" data-testid="tables-name" autoComplete="off" value={name} placeholder="Quests" onChange={(e) => setName(e.currentTarget.value)} />
          </div>
          <div className="hook-edit-row">
            <label htmlFor={`${id}-columns`}>Columns</label>
            <input
              id={`${id}-columns`}
              type="text"
              className="hook-asset"
              data-testid="tables-columns"
              autoComplete="off"
              value={columns}
              placeholder="Who, Wants, Reward"
              onChange={(e) => setColumns(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && name.trim() !== "" && columns.trim() !== "") void create();
              }}
            />
          </div>
        </div>
      </Dialog>
      <Dialog
        open={open && asking !== null && table !== null}
        onClose={() => setAsking(null)}
        title={asking === "delete" ? "Delete Table" : "Clear Table"}
        actions={
          <>
            <button type="button" onClick={() => setAsking(null)}>
              Keep It
            </button>
            <button
              type="button"
              className="primary"
              data-testid="tables-forget-yes"
              disabled={busy}
              onClick={() => {
                if (!table) return;
                const what = asking;
                setAsking(null);
                void change(what === "delete" ? { kind: "delete", id: table.id } : { kind: "clear", id: table.id }, what === "delete" ? `Deleted the table ${table.name}.` : `Cleared ${table.name}.`);
              }}
            >
              {asking === "delete" ? "Delete It" : "Clear It"}
            </button>
          </>
        }
      >
        <p>
          {table && asking === "delete"
            ? `Delete ${table.name} and its ${plural(table.rows.length, "row", "rows")}? It can't be brought back, but by a Coupler Backup made before.`
            : table
              ? `Clear all ${plural(table.rows.length, "row", "rows")} of ${table.name}? Coupler goes on filling it.`
              : ""}
        </p>
      </Dialog>
    </>
  );
}

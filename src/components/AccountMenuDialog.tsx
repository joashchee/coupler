/**
 * The account menu as a dialog (src-tauri/src/account.rs reads the
 * game's): the account's characters, each a button that plays them, and
 * what else the account can do, instead of CoffeeMUD's screen of letters
 * and a command typed at "Command or Name (?)".
 *
 * Every way to play: the narrator says the menu in a few words when it
 * opens, and the characters once their list is read (lib/account.ts);
 * VoiceOver reads each button in full. Keyboard alone: Tab through the
 * characters and the actions; Esc or Type Instead hides it until the menu
 * comes back, and focus goes to the command line. New, Delete and Quit
 * ask here, in words, then answer the game's own y/N for the player.
 * Password, e-mail, import and export are the game's questions on the
 * command line, where a password is hidden.
 */
import { useEffect, useRef, useState } from "react";
import { Dialog } from "./Dialog";
import { characterLabel, describeCharacter, type Confirm } from "../lib/account";
import type { AccountMenu } from "../lib/mud";

interface AccountMenuDialogProps {
  open: boolean;
  menu: AccountMenu | null;
  /** Sends a line to the game; `confirm` answers yes to the game's question that follows. */
  onSend: (line: string, confirm?: Confirm) => void;
  /** A command whose questions are answered on the command line. */
  onTyped: (line: string, what: string) => void;
  onTypeInstead: () => void;
}

type View = { kind: "main" } | { kind: "new" } | { kind: "delete"; name: string | null } | { kind: "quit" };

export function AccountMenuDialog({ open, menu, onSend, onTyped, onTypeInstead }: AccountMenuDialogProps) {
  const [view, setView] = useState<View>({ kind: "main" });
  const [name, setName] = useState("");
  const first = useRef<HTMLButtonElement>(null);
  const nameBox = useRef<HTMLInputElement>(null);

  // Each time it opens, from the start.
  useEffect(() => {
    if (!open) return;
    setView({ kind: "main" });
    setName("");
  }, [open]);
  // Focus on the first character (or New) whenever the view changes.
  useEffect(() => {
    if (!open) return;
    const t = window.setTimeout(() => (view.kind === "new" ? nameBox.current : first.current)?.focus(), 0);
    return () => window.clearTimeout(t);
  }, [open, view.kind, menu?.characters?.length]);

  const characters = menu?.characters ?? null;
  const remaining = menu?.remaining;
  const full = remaining === "0";

  const actions =
    view.kind === "main" ? (
      <>
        <button type="button" data-testid="account-type-instead" onClick={onTypeInstead}>
          Type Instead
        </button>
        <button type="button" data-testid="account-quit" onClick={() => setView({ kind: "quit" })}>
          Quit
        </button>
      </>
    ) : (
      <button type="button" data-testid="account-back" onClick={() => setView({ kind: "main" })}>
        Back
      </button>
    );

  return (
    <Dialog open={open} onClose={onTypeInstead} title="Account Menu" className="dialog-wide" actions={actions}>
      {view.kind === "main" && (
        <>
          <h3 className="about-section-title">Your characters</h3>
          {characters === null ? (
            <p className="map-muted" role="status">
              Reading your characters…
            </p>
          ) : characters.length === 0 ? (
            <p className="map-muted">No characters yet. Make a new one to begin.</p>
          ) : (
            <ul className="account-chars" data-testid="account-characters">
              {characters.map((c, i) => (
                <li key={c.name}>
                  <button type="button" ref={i === 0 ? first : undefined} aria-label={characterLabel(c)} onClick={() => onSend(c.name)}>
                    <span className="account-name">{c.name.padEnd(16)}</span>
                    {describeCharacter(c) || " "}
                    {c.online ? " (playing now)" : ""}
                  </button>
                </li>
              ))}
            </ul>
          )}
          {menu?.totalHours != null && <p className="about-section-desc">{`${menu.totalHours} ${menu.totalHours === 1 ? "hour" : "hours"} played in all.`}</p>}
          <h3 className="about-section-title">Your account</h3>
          <div className="account-actions">
            <button
              type="button"
              ref={characters !== null && characters.length > 0 ? undefined : first}
              data-testid="account-new"
              disabled={full}
              onClick={() => setView({ kind: "new" })}
            >
              {remaining && remaining !== "Unlimited" ? `New Character (${remaining} left)` : "New Character"}
            </button>
            {characters !== null && characters.length > 0 && (
              <button type="button" data-testid="account-delete" onClick={() => setView({ kind: "delete", name: null })}>
                Retire a Character…
              </button>
            )}
            <button type="button" onClick={() => onTyped("PASSWORD", "Change the password")}>
              Change Password
            </button>
            {menu?.email && (
              <button type="button" onClick={() => onTyped("EMAIL", "Change the e-mail address")}>
                Change E-mail
              </button>
            )}
            {menu?.import && (
              <button type="button" onClick={() => onTyped("IMPORT", "Import a character")}>
                Import
              </button>
            )}
            {menu?.export && (
              <button type="button" onClick={() => onTyped("EXPORT", "Export a character")}>
                Export
              </button>
            )}
            <button type="button" onClick={() => onSend("L")}>
              List Again
            </button>
          </div>
        </>
      )}

      {view.kind === "new" && (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            const n = name.trim();
            if (n) onSend(`NEW ${n}`, "new");
          }}
        >
          <p id="account-new-help">One word, letters only: the name everyone in the game will see. Or let the game pick a random one.</p>
          <div className="account-actions">
            <label htmlFor="account-new-name">Name</label>
            <input
              id="account-new-name"
              ref={nameBox}
              value={name}
              aria-describedby="account-new-help"
              autoComplete="off"
              spellCheck={false}
              maxLength={20}
              onChange={(e) => setName(e.target.value.replace(/[^A-Za-z]/g, ""))}
            />
            <button type="submit" className="primary" disabled={name.trim() === ""}>
              Make This Character
            </button>
            <button type="button" onClick={() => onSend("NEW *", "new")}>
              A Random Name
            </button>
          </div>
        </form>
      )}

      {view.kind === "delete" &&
        (view.name === null ? (
          <>
            <p>Which character should retire? They're deleted for good.</p>
            <ul className="account-chars">
              {(characters ?? []).map((c, i) => (
                <li key={c.name}>
                  <button type="button" ref={i === 0 ? first : undefined} onClick={() => setView({ kind: "delete", name: c.name })}>
                    {c.name}
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <>
            <p role="alert">Retire and delete {view.name}? This can't be undone.</p>
            <div className="account-actions">
              <button type="button" ref={first} className="danger" data-testid="account-delete-confirm" onClick={() => onSend(`DELETE ${view.name}`, "delete")}>
                Delete {view.name}
              </button>
            </div>
          </>
        ))}

      {view.kind === "quit" && (
        <>
          <p>Log out of the account and hang up?</p>
          <div className="account-actions">
            <button type="button" ref={first} className="primary" data-testid="account-quit-confirm" onClick={() => onSend("QUIT", "quit")}>
              Quit
            </button>
          </div>
        </>
      )}
    </Dialog>
  );
}

import { useEffect, useId, useRef, type ReactNode } from "react";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  actions?: ReactNode;
  /** Extra class on the box, e.g. `dialog-wide` for a dialog with a list in it. */
  className?: string;
  /** Another dialog is open over this one: it's inert until that closes. */
  covered?: boolean;
}

/** The dialogs open now, oldest first: Escape closes only the last. */
const openDialogs: symbol[] = [];

/**
 * Themed dialog (Fontastik's docs/ui-defaults.md §9) — replaces
 * window.alert/confirm so every confirmation/prompt in the app shares one
 * styling and animation. Stays mounted while closed (opacity/pointer-events
 * toggled via `.open`) so it can transition in rather than popping.
 *
 * Coupler's accessibility rule (docs/accessibility.md) adds what the
 * shared version lacked, and the sister apps should take it back: a
 * closed dialog is `inert` (a screen reader and the Tab key can't wander
 * into it), opening moves focus inside, Escape closes, and closing hands
 * focus back to whatever had it. A dialog can open over another (the
 * hook editor over the Hooks list): Escape closes only the one on top,
 * and the one under it is `covered`, so inert.
 */
export function Dialog({ open, onClose, title, children, actions, className, covered = false }: DialogProps) {
  const titleId = useId();
  const box = useRef<HTMLDivElement>(null);
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    if (!open) return;
    const before = document.activeElement as HTMLElement | null;
    const me = Symbol(title);
    openDialogs.push(me);
    box.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && openDialogs[openDialogs.length - 1] === me) {
        e.stopImmediatePropagation();
        close.current();
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      openDialogs.splice(openDialogs.indexOf(me), 1);
      before?.focus?.();
    };
    // The title is only the stack entry's name; a new one isn't a new dialog.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  return (
    <div
      className={`dialog-overlay${open ? " open" : ""}`}
      inert={!open || covered}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div ref={box} tabIndex={-1} className={`dialog-box${className ? ` ${className}` : ""}`} role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <h2 id={titleId}>{title}</h2>
        {/* The box is the frame and doesn't scroll (in the ANSIapps theme
            its title sits in the top edge); the body does. */}
        <div className="dialog-body">{children}</div>
        <div className="dialog-actions">
          {actions ?? (
            <button type="button" className="primary" onClick={onClose}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

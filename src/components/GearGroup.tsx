import type { ReactNode } from "react";

interface GearGroupProps {
  /** The group's name in `data-testid`s and ids. */
  id: string;
  title: string;
  open: boolean;
  onToggle: (id: string) => void;
  children: ReactNode;
}

/**
 * A branch of the gear menu: a heading that opens and closes the items
 * under it, in place and indented one cell, so the menu is a few headings
 * and one group's items instead of every item at once. App.tsx keeps
 * which one is open (one at a time). Closed groups aren't in the page, so
 * keyboard and VoiceOver meet only the headings; the heading says whether
 * its group is open (`aria-expanded`), and the arrow is a picture of that.
 */
export function GearGroup({ id, title, open, onToggle, children }: GearGroupProps) {
  const body = `gear-group-${id}`;
  return (
    <div className="menu-nest">
      <button type="button" className="menu-item menu-item-head" data-testid={`gear-group-${id}`} aria-expanded={open} aria-controls={open ? body : undefined} onClick={() => onToggle(id)}>
        <span className="menu-arrow" aria-hidden="true">
          {open ? "▼" : "►"}
        </span>
        <span>{title}</span>
      </button>
      {open && (
        <div id={body} role="group" aria-label={title} className="menu-nest-body">
          {children}
        </div>
      )}
    </div>
  );
}

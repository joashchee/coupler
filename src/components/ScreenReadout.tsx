/**
 * The status bar's readout: the screen's size in character cells, and
 * the cell under the pointer (column X, row Y, counted from 1 at the
 * top left), for placing things on the fixed screen (lib/stage.ts).
 *
 * It keeps its own state, so moving the mouse redraws these few
 * characters and nothing else. The pointer's cell is hidden from
 * assistive technology: it says nothing to someone not using a mouse.
 */
import { useEffect, useState, type RefObject } from "react";
import { CELL_HEIGHT, CELL_WIDTH, STAGE_COLUMNS, STAGE_ROWS, STAGE_WIDTH } from "../lib/stage";

export function ScreenReadout({ stage }: { stage: RefObject<HTMLDivElement | null> }) {
  const [cell, setCell] = useState<[number, number] | null>(null);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      const el = stage.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      // The stage may be scaled (full screen); cells are counted in its own pixels.
      const scale = rect.width / STAGE_WIDTH;
      const x = Math.floor((e.clientX - rect.left) / scale / CELL_WIDTH) + 1;
      const y = Math.floor((e.clientY - rect.top) / scale / CELL_HEIGHT) + 1;
      const inside = x >= 1 && x <= STAGE_COLUMNS && y >= 1 && y <= STAGE_ROWS;
      setCell((was) => (!inside ? null : was && was[0] === x && was[1] === y ? was : [x, y]));
    };
    const onLeave = () => setCell(null);
    document.addEventListener("mousemove", onMove);
    document.documentElement.addEventListener("mouseleave", onLeave);
    return () => {
      document.removeEventListener("mousemove", onMove);
      document.documentElement.removeEventListener("mouseleave", onLeave);
    };
  }, [stage]);

  // Padded to a fixed width, so the bar doesn't shuffle as the pointer moves.
  const x = cell ? String(cell[0]).padStart(3) : "  -";
  const y = cell ? String(cell[1]).padStart(2) : " -";
  return (
    <p className="screen-readout" data-testid="screen-readout">
      <span aria-label={`Screen: ${STAGE_COLUMNS} columns by ${STAGE_ROWS} rows`} role="img" title="The screen's size, in characters">
        {STAGE_COLUMNS}×{STAGE_ROWS}
      </span>
      <span aria-hidden="true" title="The character under the pointer: column X, row Y">
        {`  X ${x} Y ${y}`}
      </span>
    </p>
  );
}

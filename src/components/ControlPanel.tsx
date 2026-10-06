/**
 * Terminal's Control Panel: the right of the screen, beside the game
 * output, for turning Coupler's panels and buttons on (the map, the
 * gauges, the journal and log buttons…) and putting them where they
 * suit (lib/controlPanel.ts).
 *
 * For every player: each is a checkbox, named for VoiceOver with what it
 * shows; arranging is a checkbox too. With it on, each thing on the
 * screen has a Move corner (top left) and a Resize corner (bottom
 * right), laid over it (components/Movable.tsx): drag them, or Tab to
 * them and press the arrow keys, one character a press, Home for the
 * default; the status bar (and so the screen reader) says where it went.
 * The help at the top says so.
 */
import { offered, type ControlId, type Controls } from "../lib/controlPanel";

interface ControlPanelProps {
  controls: Controls;
  onToggle: (id: ControlId, on: boolean) => void;
  arranging: boolean;
  onArranging: (on: boolean) => void;
  onReset: () => void;
}

export function ControlPanel({ controls, onToggle, arranging, onArranging, onReset }: ControlPanelProps) {
  return (
    <section className="control-panel panel" aria-labelledby="control-title" data-testid="control-panel">
      <h2 id="control-title">Control Panel</h2>
      <div className="control-body">
        <p className="control-help" id="control-help">
          Tick what to show beside the game. To arrange, tick Arrange: then drag a thing's top-left corner to move it, its bottom-right corner to resize
          it, or Tab to its Move or Resize corner and press the arrow keys (Home puts it back). The status bar says where it went. Esc returns to the
          command line.
        </p>
        <fieldset className="control-list" aria-describedby="control-help">
          <legend className="sr-only">Show beside the game</legend>
          {offered().map((c) => (
            <label key={c.id} className="check-row">
              <input type="checkbox" checked={controls[c.id]} data-testid={`control-${c.id}`} onChange={(e) => onToggle(c.id, e.target.checked)} />
              <span>
                {c.name}
                <span className="control-about">: {c.about}</span>
              </span>
            </label>
          ))}
        </fieldset>
        <div className="control-arrange">
          <label className="check-row">
            <input type="checkbox" checked={arranging} data-testid="control-arrange" onChange={(e) => onArranging(e.target.checked)} />
            <span>Arrange: move and resize what's shown</span>
          </label>
          <button type="button" className="small" data-testid="control-reset" onClick={onReset}>
            Reset Places
          </button>
        </div>
      </div>
    </section>
  );
}

/**
 * The map panel: where you are, the picture, then naming a landmark and
 * finding a room to walk to. The picture is hidden from assistive
 * technology, and everything it shows is beside it as words: the room
 * and its exits in "Where you are" (the exits are a sentence only a
 * screen reader meets; sighted players read them in the game output and
 * the picture), and every other room through "Find a room"
 * (docs/accessibility.md).
 *
 * The picture is a fixed size with the player in the middle, so nothing
 * below it moves as the map grows (the window is a fixed 1280 by 720).
 * Each explored room is a tile in its terrain's color (lib/terrain.ts),
 * with a key below for the terrains in view.
 *
 * The map itself is built in Rust from the game's GMCP (mapper.rs); this
 * only draws the snapshot it's handed.
 */
import { useEffect, useMemo, useState } from "react";
import * as mud from "../lib/mud";
import { terrainStyle, terrainWords } from "../lib/terrain";

interface MapPanelProps {
  snapshot: mud.MapSnapshot | null;
  connected: boolean;
  walking: boolean;
  /** Runs a map action with the app's usual feedback: a status on success, an error on failure. */
  run: (label: string, work: () => Promise<string | void>) => void;
  onAskClear: () => void;
}

/** Each room is three characters wide with one between, and one row between rows, for the links. */
const STEP_X = 4;
const STEP_Y = 2;
/** How far the picture reaches from the player, in rooms: session.rs's MAP_RADIUS. */
const VIEW_RX = 5;
const VIEW_RY = 3;
/** A margin of one link all round, so exits off the edge still show. */
const VIEW_WIDTH = 2 * VIEW_RX * STEP_X + 3 + 2;
const VIEW_HEIGHT = 2 * VIEW_RY * STEP_Y + 1 + 2;

interface Glyph {
  ch: string;
  cell?: mud.MapCell;
}

/**
 * Lays the cells out as rows of characters: `[@]` rooms joined by
 * box-drawing links. Always the same size, the player's room (0, 0) in
 * the middle.
 */
function drawMap(cells: mud.MapCell[]): Glyph[][] {
  if (cells.length === 0) return [];
  const [minX, minY, width, height] = [-VIEW_RX, -VIEW_RY, VIEW_WIDTH, VIEW_HEIGHT];
  cells = cells.filter((c) => Math.abs(c.x) <= VIEW_RX && Math.abs(c.y) <= VIEW_RY);
  const grid: Glyph[][] = Array.from({ length: height }, () => Array.from({ length: width }, () => ({ ch: " " })));
  const put = (x: number, y: number, ch: string) => {
    if (y < 0 || y >= height || x < 0 || x >= width) return;
    const was = grid[y][x].ch;
    // Two diagonals crossing in the same gap.
    grid[y][x] = { ch: (was === "/" && ch === "\\") || (was === "\\" && ch === "/") || was === "X" ? "X" : ch };
  };
  for (const cell of cells) {
    const cx = (cell.x - minX) * STEP_X + 2;
    const cy = (cell.y - minY) * STEP_Y + 1;
    for (const dir of cell.exits) {
      if (dir === "N") put(cx, cy - 1, "│");
      else if (dir === "S") put(cx, cy + 1, "│");
      else if (dir === "E") put(cx + 2, cy, "─");
      else if (dir === "W") put(cx - 2, cy, "─");
      else if (dir === "NE") put(cx + 2, cy - 1, "/");
      else if (dir === "SW") put(cx - 2, cy + 1, "/");
      else if (dir === "NW") put(cx - 2, cy - 1, "\\");
      else if (dir === "SE") put(cx + 2, cy + 1, "\\");
    }
  }
  for (const cell of cells) {
    const cx = (cell.x - minX) * STEP_X + 2;
    const cy = (cell.y - minY) * STEP_Y + 1;
    const mark = cell.current ? "@" : !cell.visited ? "?" : cell.landmark ? "*" : cell.up && cell.down ? "↕" : cell.up ? "↑" : cell.down ? "↓" : " ";
    const [open, close] = !cell.visited ? [" ", " "] : cell.otherZone ? ["{", "}"] : ["[", "]"];
    grid[cy][cx - 1] = { ch: open, cell };
    grid[cy][cx] = { ch: mark, cell };
    grid[cy][cx + 1] = { ch: close, cell };
  }
  return grid;
}

export function MapPanel({ snapshot, connected, walking, run, onAskClear }: MapPanelProps) {
  const room = snapshot?.room ?? null;
  const grid = useMemo(() => drawMap(snapshot?.cells ?? []), [snapshot]);
  /** The terrains on the picture now, with a color, for the key. */
  const terrainsInView = useMemo(
    () =>
      [...new Set((snapshot?.cells ?? []).filter((c) => c.visited && Math.abs(c.x) <= VIEW_RX && Math.abs(c.y) <= VIEW_RY).map((c) => c.terrain))]
        .filter((t) => terrainStyle(t))
        .sort(),
    [snapshot],
  );
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<mud.RoomRef[]>([]);
  const [landmark, setLandmark] = useState("");
  /** A room picked on the picture with the mouse. */
  const [picked, setPicked] = useState<mud.MapCell | null>(null);

  // The landmark field follows the room the player is in.
  useEffect(() => setLandmark(room?.landmark ?? ""), [room?.id, room?.landmark]);

  // With nothing typed the list shows the landmarks; it refreshes as the map grows.
  useEffect(() => {
    let stale = false;
    mud
      .mapFind(query)
      .then((found) => {
        if (!stale) setResults(found);
      })
      .catch(() => {
        if (!stale) setResults([]);
      });
    return () => {
      stale = true;
    };
  }, [query, snapshot?.roomsKnown, room?.landmark]);

  const placeName = (r: { name: string; landmark: string }) => (r.landmark ? `${r.landmark} (${r.name})` : r.name);
  const walkTo = (id: string, name: string) => run(`Finding the way to ${name}…`, async () => `Walking to ${name}: ${await mud.mapWalk(id)}.`);
  const directionsTo = (id: string, name: string) => run(`Finding the way to ${name}…`, async () => `To ${name}: ${await mud.mapDirections(id)}.`);

  return (
    <aside className="map-panel panel" aria-labelledby="map-title" data-testid="map-panel">
      <h2 id="map-title">Map</h2>

      {/* The frame doesn't scroll (its title sits in the top edge); this does. */}
      <div className="map-body">
      <section aria-labelledby="map-here-title">
        <h3 id="map-here-title" className="map-heading">
          Where you are
        </h3>
        {room ? (
          <p className="map-here" data-testid="map-here">
            <span className="map-room-name">{room.name}</span>
            {room.landmark && <span> ({room.landmark})</span>}
            <br />
            <span className="map-muted">
              {room.zone}
              {room.terrain ? `, ${terrainWords(room.terrain)}` : ""}
              {room.travel && room.travel !== "normal" ? `, ${room.travel}` : ""}
            </span>
            <span className="sr-only"> {mud.describeExits(room)}</span>
          </p>
        ) : (
          <p className="map-muted" data-testid="map-here">
            {connected
              ? "Once you're in the game, the map starts as you move. Every room you enter is remembered."
              : "Not connected. The rooms you've explored are kept: the map picks up where you left it when you connect."}
          </p>
        )}
      </section>

      {walking && (
        <p className="map-walking">
          <span>Walking…</span>
          <button type="button" className="small" data-testid="map-stop" onClick={() => run("Stopping…", () => mud.mapStop())}>
            Stop
          </button>
        </p>
      )}

      {/* Always here, even before there's a map, so what's below keeps its place. */}
      <section>
          {/* Hidden from assistive technology: "Where you are" above and
              "Find a room" below say everything this shows. */}
          <pre className="map-picture" aria-hidden="true" data-testid="map-picture">
            {grid.map((row, y) => (
              <div key={y}>
                {row.map((g, x) =>
                  g.cell ? (
                    <span
                      key={x}
                      className={`map-cell${g.cell.current ? " current" : ""}${g.cell.visited ? "" : " stub"}${picked?.id === g.cell.id ? " picked" : ""}`}
                      style={g.cell.visited && picked?.id !== g.cell.id ? (terrainStyle(g.cell.terrain) ?? undefined) : undefined}
                      title={g.cell.visited ? (g.cell.terrain ? `${g.cell.name}, ${terrainWords(g.cell.terrain)}` : g.cell.name) : "Not explored"}
                      onClick={() => setPicked(g.cell?.visited && !g.cell.current ? g.cell : null)}
                    >
                      {g.ch}
                    </span>
                  ) : (
                    g.ch
                  ),
                )}
              </div>
            ))}
          </pre>
          <p className="map-muted map-legend" aria-hidden="true">
            @ you · * landmark · ? not explored · ↑ ↓ stairs · {"{ }"} another area
          </p>
          {/* Always two lines, so what's below keeps its place. */}
          <p className="map-muted map-terrains" aria-hidden="true" data-testid="map-terrains">
            {terrainsInView.map((t) => (
              <span key={t} className="map-terrain">
                <span className="map-swatch" style={terrainStyle(t) ?? undefined}>
                  {"   "}
                </span>{" "}
                {terrainWords(t)}
              </span>
            ))}
          </p>
          {picked && (
            <p className="map-picked">
              <span>{picked.name}</span>
              <span className="map-row">
                <button type="button" className="small" disabled={!connected} onClick={() => walkTo(picked.id, picked.name)}>
                  Walk
                </button>
                <button type="button" className="small" onClick={() => directionsTo(picked.id, picked.name)}>
                  Directions
                </button>
              </span>
            </p>
          )}
      </section>

      {room && (
        <section aria-labelledby="map-landmark-title">
          <h3 id="map-landmark-title" className="map-heading">
            Landmark
          </h3>
          <form
            className="map-row"
            onSubmit={(e) => {
              e.preventDefault();
              const name = landmark.trim();
              run("Saving the landmark…", async () => {
                await mud.mapSetLandmark(room.id, name);
                return name ? `This room is now the landmark "${name}".` : "Landmark removed from this room.";
              });
            }}
          >
            <input
              type="text"
              data-testid="map-landmark"
              aria-label="Your name for this room"
              placeholder="Name this room (bank, home…)"
              value={landmark}
              maxLength={40}
              autoComplete="off"
              spellCheck={false}
              onChange={(e) => setLandmark(e.currentTarget.value)}
            />
            <button type="submit" className="small">
              Save
            </button>
          </form>
        </section>
      )}

      <section aria-labelledby="map-find-title">
        <h3 id="map-find-title" className="map-heading">
          Find a room
        </h3>
        <input
          type="search"
          className="map-find"
          data-testid="map-find"
          aria-label="Find a room you've visited, by room, landmark or area name"
          placeholder="Room, landmark or area"
          value={query}
          autoComplete="off"
          spellCheck={false}
          onChange={(e) => setQuery(e.currentTarget.value)}
        />
        <p className="map-muted" role="status">
          {query.trim() === ""
            ? results.length === 0
              ? "No landmarks yet. Type to search the rooms you've visited."
              : `Your landmarks: ${results.length}.`
            : results.length === 0
              ? "No visited room matches."
              : `${results.length === 50 ? "First 50 rooms" : results.length === 1 ? "1 room" : `${results.length} rooms`} found.`}
        </p>
        {results.length > 0 && (
          <ul className="map-results" data-testid="map-results">
            {results.map((r) => (
              <li key={r.id}>
                <span>
                  {placeName(r)}
                  <span className="map-muted">
                    {" "}
                    {r.zone}
                    {r.terrain ? `, ${terrainWords(r.terrain)}` : ""}
                  </span>
                </span>
                <span className="map-row">
                  <button type="button" className="small" disabled={!connected} aria-label={`Walk to ${placeName(r)}`} onClick={() => walkTo(r.id, placeName(r))}>
                    Walk
                  </button>
                  <button type="button" className="small" aria-label={`Directions to ${placeName(r)}`} onClick={() => directionsTo(r.id, placeName(r))}>
                    Directions
                  </button>
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <p className="map-muted map-tally" data-testid="map-tally">
        {snapshot && snapshot.roomsKnown > 0
          ? `${snapshot.roomsKnown} ${snapshot.roomsKnown === 1 ? "room" : "rooms"} explored in ${snapshot.zonesKnown} ${snapshot.zonesKnown === 1 ? "area" : "areas"}.`
          : "Nothing explored yet."}
      </p>
      {snapshot && snapshot.roomsKnown > 0 && (
        <button type="button" className="small" data-testid="map-clear" onClick={onAskClear}>
          Start the map over…
        </button>
      )}
      </div>
    </aside>
  );
}

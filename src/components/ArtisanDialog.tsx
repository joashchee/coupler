/**
 * Artisan Skills (gear menu, Cmd+Shift+A): the Artisan class's skill
 * tree and its mentor (lib/artisan.ts, from CoffeeMUD's Artisan.java).
 *
 * The mentor is first: name a skill (or ask "how do I get
 * blacksmithing?") and it answers in a few words what to learn first, in
 * order, how well, the base stats and the level. The answer is written
 * in a live region, so a screen reader reads it, and Coupler's voice
 * says it when it's on.
 *
 * Under it the tree, for the eye: a column a step from the start (step 1
 * needs nothing), each skill a button. Choosing one (or asking the
 * mentor about it) marks what it needs (◄, light cyan) and what it opens
 * up (►, light green), draws the lines between them, and says it. The
 * lines are a picture, hidden from screen readers; the same facts are
 * in each button's name and in the answer above. One Tab stop: the
 * arrows move between skills, Return chooses.
 */
import { useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import * as artisan from "../lib/artisan";
import * as voice from "../lib/voice";
import { CELL_HEIGHT, CELL_WIDTH } from "../lib/stage";
import { Dialog } from "./Dialog";

interface ArtisanDialogProps {
  open: boolean;
  onClose: () => void;
  /** Coupler's voice speaks (Immersive, or Workshop with it on). */
  voiced: boolean;
}

type Mark = "chosen" | "need" | "opens" | null;

const MARKS: Record<Exclude<Mark, null>, { glyph: string; words: string }> = {
  chosen: { glyph: "■ ", words: "chosen" },
  need: { glyph: "◄ ", words: "needed first" },
  opens: { glyph: "► ", words: "it opens up" },
};

interface Edge {
  from: string;
  to: string;
  kind: "need" | "opens" | "all";
}

const FIRST_WORDS = "Ask me about any skill, and I'll tell you what you need first.";

export function ArtisanDialog({ open, onClose, voiced }: ArtisanDialogProps) {
  const id = useId();
  const cols = useMemo(() => artisan.columns(), []);
  const [query, setQuery] = useState("");
  const [chosen, setChosen] = useState<string | null>(null);
  const [focusId, setFocusId] = useState(cols[0][0].id);
  // A changed string is what makes a screen reader read it again.
  const [words, setWords] = useState(FIRST_WORDS);
  const field = useRef<HTMLInputElement>(null);
  const chart = useRef<HTMLDivElement>(null);
  const nodes = useRef(new Map<string, HTMLButtonElement>());
  const [lines, setLines] = useState<{ d: string; kind: Edge["kind"] }[]>([]);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const snapTimer = useRef(0);

  // Opening puts the cursor in the question, after Dialog's own focus.
  useEffect(() => {
    if (!open) return;
    const t = window.setTimeout(() => field.current?.focus(), 0);
    return () => window.clearTimeout(t);
  }, [open]);

  const needs = useMemo(() => (chosen ? artisan.ancestors(chosen) : new Map<string, number>()), [chosen]);
  const opens = useMemo(() => (chosen ? artisan.descendants(chosen) : new Set<string>()), [chosen]);
  const markOf = useCallback((sid: string): Mark => (sid === chosen ? "chosen" : needs.has(sid) ? "need" : opens.has(sid) ? "opens" : null), [chosen, needs, opens]);

  const edges = useMemo(() => {
    const out: Edge[] = [];
    for (const s of artisan.SKILLS)
      for (const [need] of s.needs) {
        if (!chosen) out.push({ from: need, to: s.id, kind: "all" });
        else if ((s.id === chosen || needs.has(s.id)) && needs.has(need)) out.push({ from: need, to: s.id, kind: "need" });
        else if (opens.has(s.id) && (need === chosen || opens.has(need))) out.push({ from: need, to: s.id, kind: "opens" });
      }
    return out;
  }, [chosen, needs, opens]);

  // The lines, from where each name ends to where the next begins, measured once laid out.
  useLayoutEffect(() => {
    if (!open || !chart.current) return;
    const box = chart.current;
    // Whole cells, so the picture is on the grid too (Check the Grid measures it).
    setSize({ w: Math.ceil(box.scrollWidth / CELL_WIDTH) * CELL_WIDTH, h: Math.ceil(box.scrollHeight / CELL_HEIGHT) * CELL_HEIGHT });
    setLines(
      edges.flatMap((e) => {
        const a = nodes.current.get(e.from);
        const b = nodes.current.get(e.to);
        if (!a || !b) return [];
        const x1 = a.offsetLeft + a.offsetWidth;
        const y1 = a.offsetTop + a.offsetHeight / 2;
        const x2 = b.offsetLeft;
        const y2 = b.offsetTop + b.offsetHeight / 2;
        const bend = Math.max(24, (x2 - x1) / 3);
        return [{ d: `M${x1},${y1} C${x1 + bend},${y1} ${x2 - bend},${y2} ${x2},${y2}`, kind: e.kind }];
      }),
    );
  }, [open, edges]);

  const say = useCallback(
    (text: string) => {
      setWords((was) => (was === text ? `${text} ` : text));
      if (voiced) voice.speak(text, true);
    },
    [voiced],
  );

  /** Chooses a skill: marks its lines, says what it needs and what it opens up. */
  const choose = useCallback(
    (sid: string, scroll: boolean) => {
      setChosen(sid);
      setFocusId(sid);
      say(`${artisan.advice(sid)} ${artisan.opensWords(sid)}`);
      if (scroll) nodes.current.get(sid)?.scrollIntoView({ block: "center", inline: "center" });
    },
    [say],
  );

  const ask = () => {
    const a = artisan.answer(query);
    if (a.skill) choose(a.skill.id, true);
    else say(a.words);
  };

  // Where each skill is, for the arrows.
  const at = useMemo(() => {
    const m = new Map<string, [number, number]>();
    cols.forEach((col, c) => col.forEach((s, r) => m.set(s.id, [c, r])));
    return m;
  }, [cols]);

  const onChartKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const [c, r] = at.get(focusId) ?? [0, 0];
    let next: [number, number] | null = null;
    if (e.key === "ArrowUp") next = [c, Math.max(0, r - 1)];
    else if (e.key === "ArrowDown") next = [c, Math.min(cols[c].length - 1, r + 1)];
    else if (e.key === "ArrowLeft" && c > 0) next = [c - 1, Math.min(cols[c - 1].length - 1, r)];
    else if (e.key === "ArrowRight" && c < cols.length - 1) next = [c + 1, Math.min(cols[c + 1].length - 1, r)];
    else if (e.key === "Home") next = [c, 0];
    else if (e.key === "End") next = [c, cols[c].length - 1];
    if (!next) return;
    e.preventDefault();
    const sid = cols[next[0]][next[1]].id;
    setFocusId(sid);
    nodes.current.get(sid)?.focus();
  };

  const chosenSkill = chosen ? artisan.skill(chosen) : undefined;

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Artisan Skills"
      className="dialog-artisan"
      actions={
        <>
          <button
            type="button"
            data-testid="artisan-clear"
            disabled={chosen === null}
            onClick={() => {
              setChosen(null);
              say("Nothing chosen. Every line is shown.");
            }}
          >
            Show Every Line
          </button>
          <button type="button" className="primary" onClick={onClose}>
            Done
          </button>
        </>
      }
    >
      <p>
        The Artisan's {artisan.SKILLS.length} skills, as CoffeeMUD gives them. A skill is gained from a guildmaster once every skill it needs is known well enough: practice and use
        them to get there. Ask the mentor, or choose a skill below (arrow keys, then Return) to see what it needs and what it opens up.
      </p>
      <form
        className="hooks-search artisan-ask"
        onSubmit={(e) => {
          e.preventDefault();
          ask();
        }}
      >
        <label htmlFor={`${id}-ask`}>Ask the mentor</label>
        <input
          ref={field}
          id={`${id}-ask`}
          type="text"
          data-testid="artisan-ask"
          placeholder="Which skill do you want? Blacksmithing, Master Baking..."
          value={query}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          aria-describedby={`${id}-answer`}
          onChange={(e) => setQuery(e.currentTarget.value)}
        />
        <button type="submit" data-testid="artisan-ask-button">
          Ask
        </button>
      </form>
      <p id={`${id}-answer`} className="artisan-answer" data-testid="artisan-answer" role="status" aria-live="polite">
        {words}
      </p>
      {/* The key, for the eye: the same is in each skill's name for a screen reader. */}
      <p className="artisan-key" aria-hidden="true">
        <span className="artisan-mark-chosen">{MARKS.chosen.glyph}chosen</span>
        <span className="artisan-mark-need">{MARKS.need.glyph}needed first</span>
        <span className="artisan-mark-opens">{MARKS.opens.glyph}it opens up</span>
        <span className="artisan-kind-other">not a craft</span>
        <span>(30) the level, when it's above 1</span>
        <span>{chosenSkill ? `Showing: ${chosenSkill.name}` : "Showing every line"}</span>
      </p>
      <div
        ref={chart}
        className="artisan-chart"
        data-testid="artisan-chart"
        role="group"
        aria-label={`The skill tree: ${cols.length} steps from the start, the arrow keys move between skills, Return chooses one`}
        onKeyDown={onChartKey}
        onScroll={(e) => {
          // Sideways scrolling comes to rest on a whole cell, as lib/grid.ts's
          // row snap does downwards, so the names stay on the grid.
          const box = e.currentTarget;
          window.clearTimeout(snapTimer.current);
          if (document.documentElement.dataset.theme !== "ansiapps") return;
          snapTimer.current = window.setTimeout(() => {
            const snapped = Math.round(box.scrollLeft / CELL_WIDTH) * CELL_WIDTH;
            if (snapped !== box.scrollLeft) box.scrollLeft = snapped;
          }, 120);
        }}
        style={{ gridTemplateColumns: `repeat(${cols.length}, max-content)` }}
      >
        <svg className="artisan-lines" aria-hidden="true" width={size.w} height={size.h}>
          {lines.map((l, i) => (
            <path key={i} d={l.d} className={`artisan-line artisan-line-${l.kind}`} />
          ))}
        </svg>
        {cols.map((_, c) => (
          <div key={`head-${c}`} className="artisan-step" style={{ gridColumn: c + 1, gridRow: 1 }} aria-hidden="true">
            {c === 0 ? "Step 1: needs nothing" : `Step ${c + 1}`}
          </div>
        ))}
        {cols.map((col, c) =>
          col.map((s, r) => {
            const mark = markOf(s.id);
            const level = s.level > 1 ? ` (${s.level})` : "";
            return (
              <button
                key={s.id}
                ref={(el) => {
                  if (el) nodes.current.set(s.id, el);
                  else nodes.current.delete(s.id);
                }}
                type="button"
                tabIndex={s.id === focusId ? 0 : -1}
                className={`artisan-node${mark ? ` artisan-mark-${mark}` : ""}${s.kind === "craft" ? "" : " artisan-kind-other"}`}
                style={{ gridColumn: c + 1, gridRow: r + 2 }}
                data-testid={`artisan-node-${s.id}`}
                aria-pressed={mark === "chosen"}
                aria-label={`${s.name}, step ${c + 1}${s.level > 1 ? `, level ${s.level}` : ""}${mark ? `, ${MARKS[mark].words}` : ""}${s.kind === "craft" ? "" : ", not a craft"}. ${artisan.needsShort(s.id)}`}
                onFocus={() => setFocusId(s.id)}
                onClick={() => choose(s.id, false)}
              >
                {mark ? MARKS[mark].glyph : "  "}
                {s.name}
                {level}
              </button>
            );
          }),
        )}
      </div>
    </Dialog>
  );
}

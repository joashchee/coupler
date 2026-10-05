/**
 * The hooks list: every key and value the game has sent by GMCP, each
 * pair once (src-tauri/src/hooks.rs), opened from the total in the
 * status bar. Two tabs: the hooks list, and the Filtered list of keys
 * taken out of it. Ticking a row's box in the hooks list moves its key
 * (and so every row with that key) to the Filtered list; unticking it
 * there brings it back. A search narrows either; a long list is cut
 * short with the full count above it, so the dialog stays quick however
 * many there are.
 *
 * Beside each pair in the hooks list: a summary of what it sets off
 * each time the game sends it (sound effect, music, picture, background
 * noise, weather sound, from the Assets folder; or Nothing), and an
 * Edit button that opens the hook editor over the list
 * (HookEditor.tsx) with every choice. Assets are listed by name,
 * without their folder or file type (the type is added only to tell two
 * of the same name apart). Add Assets opens a file chooser, the
 * keyboard's way to do what dropping files on the window does. This
 * Room searches for the room the player is in, to give it its own
 * background noise.
 *
 * A third tab, Assets, lists every file in the Assets folder with how
 * many hooks use it: a green bullet beside each in use (on its own black
 * cell in the ANSIapps theme), and the same in words. Reading the
 * folder also clears any asset whose file has gone from every hook that
 * named it (assets_list), and says so in the status bar. Each row ends
 * with Play (Stop while it plays: one asset at a time, lib/assets.ts
 * `playPreview`) or, for a picture, Show… (AssetPreview.tsx). A MIDI
 * file's row chooses what plays it: Neumetik, Coupler's own synthesizer,
 * unless the player picks one of their SoundFonts for it (a SoundFont
 * that's gone falls back to Neumetik). A SoundFont's Play plays a
 * sample tune through it.
 *
 * Create Asset, beside Add Assets, makes a picture or a loop of music
 * from the player's words (CreateAssetDialog.tsx).
 *
 * The list is a fixed height and scrolls on its own (the dialog's frame
 * never does), and it takes the keyboard's focus so the arrow keys
 * scroll it. It isn't a live region: new hooks arrive all the time while
 * playing, and reading each out would bury the game. The chosen tab is
 * marked with a bullet as well as a color.
 *
 * The rows (up to 500) are drawn only while
 * the dialog is open, and the dialog is memoized: it stays mounted when
 * closed (Dialog), and App re-renders on every key typed in the command
 * line and every line of output, which used to redraw the whole list.
 */
import { memo, useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import type { ActivityUpdate } from "../lib/activity";
import * as sound from "../lib/assets";
import * as mud from "../lib/mud";
import { AssetPreviewDialog } from "./AssetPreview";
import { CreateAssetDialog } from "./CreateAssetDialog";
import { Dialog } from "./Dialog";
import { HookEditor } from "./HookEditor";

interface HooksDialogProps {
  open: boolean;
  onClose: () => void;
  /** The total in the status bar: when it changes, the open list is read again. */
  count: number;
  /** Bumped when the Assets folder changes, so its files are read again. */
  assetsVersion: number;
  /** The room the player is in (its id), for This Room; empty when it isn't known. */
  roomId: string;
  /** Opens the file chooser and adds what's chosen to the Assets folder. */
  onAddAssets: () => Promise<void>;
  /** The Assets folder changed here (an asset made, a SoundFont chosen): what was decoded is forgotten. */
  onAssetsChanged: () => void;
  runActivity: <T>(label: string, task: (update: ActivityUpdate) => Promise<T>, key?: string) => Promise<T>;
  onStatus: (message: string) => void;
  onError: (message: string) => void;
}

type Tab = "hooks" | "filtered" | "assets";
const TABS: [Tab, string][] = [
  ["hooks", "Hooks list"],
  ["filtered", "Filtered list"],
  ["assets", "Assets"],
];

const KIND_NAMES: Record<mud.AssetKind, string> = { sfx: "SFX", bgm: "BGM", art: "ART", soundfont: "SoundFont" };

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** A pair in words, for the names of its row's controls. */
const said = (p: mud.HookPair) => `${p.key} ${p.value}`;

/** What a trigger sets off, in short: "SFX door 40%, BGM town, loop, 75%, ART gate at 10,5, fades 8s", or Nothing. */
function summaryOf(t: mud.Trigger | undefined): string {
  if (!t) return "Nothing";
  const parts = [
    t.sfx && `SFX ${mud.assetTitle(t.sfx)} ${t.sfxVolume}%`,
    t.bgm && `BGM ${mud.assetTitle(t.bgm)}${t.bgmLoop ? ", loop," : ""} ${t.bgmVolume}%`,
    t.art && `ART ${mud.assetTitle(t.art)} at ${t.artX},${t.artY}${t.artFade > 0 ? `, fades ${t.artFade}s` : ""}`,
    t.bgn && `BGN ${mud.assetTitle(t.bgn)} ${t.bgnVolume}%`,
    t.bgw && `BGW ${mud.assetTitle(t.bgw)} ${t.bgwVolume}%`,
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(", ") : "Nothing";
}

export const HooksDialog = memo(function HooksDialog({ open, onClose, count, assetsVersion, roomId, onAddAssets, onAssetsChanged, runActivity, onStatus, onError }: HooksDialogProps) {
  const [tab, setTab] = useState<Tab>("hooks");
  const [query, setQuery] = useState("");
  const [listing, setListing] = useState<mud.HookListing | null>(null);
  const [filtered, setFiltered] = useState<mud.FilteredListing | null>(null);
  /** Bumped after a box is ticked or unticked, so the list is read again. */
  const [version, setVersion] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);
  /** The row whose box was just used: focus goes to the row that takes its place. */
  const focusRow = useRef<number | null>(null);
  const id = useId();
  const [assets, setAssets] = useState<mud.Asset[]>([]);
  /** Each MIDI file played through a SoundFont the player chose, and the SoundFont. */
  const [fonts, setFonts] = useState<Record<string, string>>({});
  /** The pair open in the hook editor, with what it sets off as last saved. */
  const [editing, setEditing] = useState<mud.HookPair | null>(null);
  /** The picture open in Show…, and whether Create Asset is open. */
  const [showing, setShowing] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  /** The asset being tried (Play), from the moment it's asked for. */
  const [trying, setTrying] = useState<string | null>(sound.previewing());
  useEffect(() => sound.onPreview(setTrying), []);
  // Closing the list closes what's over it too, and stops what it was playing.
  useEffect(() => {
    if (open) return;
    setEditing(null);
    setShowing(null);
    setCreating(false);
    sound.stopPreview();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    let stale = false;
    mud
      .assetsList()
      .then(({ assets, cleared, fonts }) => {
        if (stale) return;
        setAssets(assets);
        setFonts(fonts);
        if (cleared.length > 0) {
          onStatus(mud.clearedInWords(cleared));
          setVersion((v) => v + 1);
        }
      })
      .catch((e) => onError(String(e)));
    return () => {
      stale = true;
    };
    // `version` too: a changed trigger changes how many hooks use an asset.
  }, [open, assetsVersion, version, onStatus, onError]);

  /** Each kind's assets, each with the name the list shows: its name, and its type only if another of its kind shares the name. */
  const byKind = useMemo(() => {
    const kinds = new Map<mud.AssetKind, { path: string; kind: mud.AssetKind; shown: string; uses: number }[]>();
    for (const kind of ["sfx", "bgm", "art", "soundfont"] as const) {
      const ofKind = assets.filter((a) => a.kind === kind);
      const titles = new Map<string, number>();
      for (const a of ofKind) titles.set(mud.assetTitle(a.path), (titles.get(mud.assetTitle(a.path)) ?? 0) + 1);
      kinds.set(
        kind,
        ofKind.map((a) => {
          const title = mud.assetTitle(a.path);
          return { path: a.path, kind, uses: a.uses, shown: (titles.get(title) ?? 0) > 1 ? `${title} (${a.path.slice(0, a.path.indexOf("/"))})` : title };
        }),
      );
    }
    return kinds;
  }, [assets]);

  /** Saves a change to what a pair sets off, and says what it now does. */
  async function setTrigger(p: mud.HookPair, change: Partial<mud.Trigger>) {
    const trigger = { ...(p.trigger ?? mud.NO_TRIGGER), ...change };
    try {
      await mud.hooksSetTrigger(p.key, p.value, trigger);
      const parts = [
        trigger.sfx && `plays ${mud.assetTitle(trigger.sfx)} at ${trigger.sfxVolume}%`,
        trigger.bgm && `plays ${mud.assetTitle(trigger.bgm)}${trigger.bgmLoop ? " in a loop" : " once"} at ${trigger.bgmVolume}%`,
        trigger.art &&
          `shows ${mud.assetTitle(trigger.art)} at column ${trigger.artX}, row ${trigger.artY}, ${trigger.artFade > 0 ? `fading after ${trigger.artFade} seconds` : "until clicked away"}`,
        trigger.bgn && `loops ${mud.assetTitle(trigger.bgn)} as background noise at ${trigger.bgnVolume}%`,
        trigger.bgw && `loops ${mud.assetTitle(trigger.bgw)} as the weather's sound at ${trigger.bgwVolume}%`,
      ].filter(Boolean);
      onStatus(parts.length > 0 ? `${p.key} ${p.value} ${parts.join(", ")}.` : `${p.key} ${p.value} sets nothing off now.`);
      setEditing((e) => (e && e.key === p.key && e.value === p.value ? { ...e, trigger } : e));
      setVersion((v) => v + 1);
    } catch (e) {
      onError(String(e));
    }
  }

  useEffect(() => {
    if (!open || tab === "assets") return;
    let stale = false;
    const read = tab === "hooks" ? mud.hooksList(query).then((l) => !stale && setListing(l)) : mud.hooksFiltered(query).then((l) => !stale && setFiltered(l));
    read.catch((e) => onError(String(e)));
    return () => {
      stale = true;
    };
  }, [open, tab, query, count, version, onError]);

  // The ticked row has gone to the other list; keep the keyboard where it was.
  useEffect(() => {
    const row = focusRow.current;
    const list = listRef.current;
    if (row === null || !list) return;
    focusRow.current = null;
    const boxes = list.querySelectorAll<HTMLInputElement>('li > input[type="checkbox"]');
    (boxes[Math.min(row, boxes.length - 1)] ?? list).focus();
  }, [listing, filtered]);

  async function move(key: string, toFiltered: boolean, row: number) {
    try {
      await mud.hooksSetFiltered(key, toFiltered);
      focusRow.current = row;
      onStatus(toFiltered ? `${key} moved to the Filtered list.` : `${key} is back in the hooks list.`);
      setVersion((v) => v + 1);
    } catch (e) {
      onError(String(e));
    }
  }

  /** Plays an asset to try it, or stops it if it's the one playing. */
  async function playOrStop(a: { path: string; kind: mud.AssetKind; shown: string }) {
    if (trying === a.path) {
      sound.stopPreview();
      onStatus(`${a.shown} stopped.`);
      return;
    }
    try {
      // Music and SoundFonts are rendered first, which can take a moment.
      const play = () => sound.playPreview(a.path, a.kind);
      if (a.kind === "sfx") await play();
      else await runActivity(a.kind === "soundfont" ? `Playing a sample tune through ${a.shown}…` : `Getting ${a.shown} ready to play…`, play, `play:${a.path}`);
    } catch (e) {
      onError(String(e));
    }
  }

  /** Chooses what plays a MIDI file: a SoundFont, or Neumetik (empty). */
  async function chooseFont(path: string, font: string) {
    try {
      await mud.assetSetFont(path, font || null);
      if (trying === path) sound.stopPreview();
      onAssetsChanged();
      setFonts((all) => {
        const next = { ...all };
        if (font) next[path] = font;
        else delete next[path];
        return next;
      });
      onStatus(font ? `${mud.assetTitle(path)} now plays through the SoundFont ${mud.assetTitle(font)}.` : `${mud.assetTitle(path)} now plays through Neumetik, Coupler's own synthesizer.`);
    } catch (e) {
      onError(String(e));
    }
  }

  function onTabKey(e: KeyboardEvent<HTMLButtonElement>) {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    const at = TABS.findIndex(([which]) => which === tab);
    const other = TABS[(at + (e.key === "ArrowRight" ? 1 : TABS.length - 1)) % TABS.length][0];
    setTab(other);
    document.getElementById(`${id}-tab-${other}`)?.focus();
  }

  const searching = query.trim() !== "";
  /** The Assets tab's rows: every kind, in the folder's order, narrowed by Search. */
  const shownAssets = [...byKind.values()]
    .flat()
    .filter((a) => !searching || a.path.toLowerCase().includes(query.trim().toLowerCase()))
    .sort((a, b) => a.path.toLowerCase().localeCompare(b.path.toLowerCase()));
  let summary: string;
  if (tab === "hooks") {
    const shown = listing?.pairs.length ?? 0;
    summary = !listing
      ? "Reading the hooks…"
      : listing.total === 0
        ? "No hooks here yet. They're recorded as you play."
        : listing.matching === 0
          ? `None of the ${plural(listing.total, "hook matches", "hooks match")}.`
          : shown < listing.matching
            ? `Showing the first ${shown} of ${listing.matching}. Type in Search to narrow them down.`
            : searching
              ? `${listing.matching} of ${plural(listing.total, "hook", "hooks")} match.`
              : `${plural(listing.total, "hook", "hooks")}. Tick one to move its name to the Filtered list.`;
  } else if (tab === "assets") {
    const used = assets.filter((a) => a.uses > 0).length;
    const fontsHere = byKind.get("soundfont") ?? [];
    summary =
      assets.length === 0
        ? "The Assets folder is empty. Drop sound, music and picture files on Coupler's window, or use Add Assets."
        : searching
          ? `${shownAssets.length} of ${plural(assets.length, "asset", "assets")} match.`
          : `${plural(assets.length, "asset", "assets")}, ${used} in use by a hook (green bullet). MIDI plays through Neumetik${fontsHere.length > 0 ? " unless you choose a SoundFont" : ""}.`;
  } else {
    const shown = filtered?.keys.length ?? 0;
    summary = !filtered
      ? "Reading the filtered names…"
      : filtered.total === 0
        ? "Nothing is filtered. Tick a hook in the hooks list to move its name here."
        : filtered.matching === 0
          ? `None of the ${plural(filtered.total, "filtered name matches", "filtered names match")}.`
          : shown < filtered.matching
            ? `Showing the first ${shown} of ${filtered.matching}. Type in Search to narrow them down.`
            : searching
              ? `${filtered.matching} of ${plural(filtered.total, "filtered name", "filtered names")} match.`
              : `${plural(filtered.total, "filtered name", "filtered names")}. Untick one to bring it back to the hooks list.`;
  }

  return (
    <>
      <Dialog open={open} onClose={onClose} title="Hooks" className="dialog-wide dialog-hooks" covered={editing !== null || showing !== null || creating}>
        <p>
          Every name and value CoffeeMUD has sent by GMCP, each pair once, on this computer only, plus Coupler's own: coupler.room.type (indoors or
          outdoors), coupler.weather (from the game's weather lines; AUTOWEATHER tells it sooner) and coupler.time (dawn, day, dusk or night, from the
          game's clock). Edit chooses what a pair sets off each time it comes: a sound, music, a
          picture, background noise or weather. Assets come from the Assets folder: drop files on the window, or use Add Assets.
        </p>
        <div className="hooks-tabs" role="tablist" aria-label="Which list">
          {TABS.map(([which, label]) => (
            <button
              key={which}
              type="button"
              role="tab"
              id={`${id}-tab-${which}`}
              className={`hooks-tab${tab === which ? " selected" : ""}`}
              data-testid={`hooks-tab-${which}`}
              aria-selected={tab === which}
              aria-controls={`${id}-panel`}
              tabIndex={tab === which ? 0 : -1}
              onClick={() => setTab(which)}
              onKeyDown={onTabKey}
            >
              <span className="hooks-tab-mark" aria-hidden="true">
                •
              </span>
              {label}
            </button>
          ))}
        </div>
        <div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-tab-${tab}`}>
          <div className="hooks-search">
            {tab !== "filtered" && (
              <>
                <button type="button" className="hooks-add" data-testid="hooks-add-assets" onClick={() => void onAddAssets()}>
                  Add Assets…
                </button>
                <button type="button" className="hooks-add" data-testid="hooks-create-asset" onClick={() => setCreating(true)}>
                  Create Asset…
                </button>
              </>
            )}
            {tab === "hooks" && (
              <button
                type="button"
                className="hooks-add"
                data-testid="hooks-this-room"
                disabled={!roomId}
                title={roomId ? undefined : "Coupler doesn't know which room you're in yet."}
                onClick={() => {
                  setQuery(roomId);
                  onStatus(`Showing the hooks for the room you're in, ${roomId}. Its room.info.id row's Edit button gives it its own background noise.`);
                }}
              >
                This Room
              </button>
            )}
            <label htmlFor={`${id}-search`}>Search</label>
            <input
              id={`${id}-search`}
              type="text"
              data-testid="hooks-search"
              aria-describedby={`${id}-summary`}
              placeholder={tab === "hooks" ? "Part of a name or a value" : tab === "assets" ? "Part of a file's name or type" : "Part of a name"}
              value={query}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck={false}
              onChange={(e) => setQuery(e.currentTarget.value)}
            />
          </div>
          <p id={`${id}-summary`} className="hooks-summary" data-testid="hooks-summary">
            {summary}
          </p>
          {tab === "assets" ? (
            <>
              <div className="hooks-head assets-row" aria-hidden="true">
                <span />
                <span>Name</span>
                <span>Kind</span>
                <span>File</span>
                <span>Plays through</span>
                <span>Used</span>
                <span />
              </div>
              <ul ref={listRef} className="hooks-list assets-list" data-testid="assets-list" tabIndex={0} aria-label="Assets: name, kind, file, what plays it, how many hooks use it, then Play or Show">
                {open &&
                  shownAssets.map((a) => {
                    const chosen = fonts[a.path] ?? "";
                    const fontList = byKind.get("soundfont") ?? [];
                    const missing = chosen !== "" && !fontList.some((f) => f.path === chosen);
                    const isPlaying = trying === a.path;
                    return (
                      <li key={a.path} className={a.uses > 0 ? "in-use" : undefined}>
                        <span className="asset-dot" aria-hidden="true">
                          {a.uses > 0 ? "•" : ""}
                        </span>
                        <span className="hook-key">{a.shown}</span>
                        <span className="hook-value">{KIND_NAMES[a.kind]}</span>
                        <span className="hook-value">{a.path}</span>
                        {mud.isMidi(a.path) ? (
                          <select
                            className="hook-asset"
                            data-testid="asset-font"
                            value={chosen}
                            aria-label={`${a.shown} plays through`}
                            title={missing ? "That SoundFont isn't in the Assets folder any more, so Neumetik plays it." : undefined}
                            onChange={(e) => void chooseFont(a.path, e.currentTarget.value)}
                          >
                            <option value="">Neumetik</option>
                            {missing && <option value={chosen}>{mud.assetTitle(chosen)} (missing: Neumetik)</option>}
                            {fontList.map((f) => (
                              <option key={f.path} value={f.path}>
                                {f.shown}
                              </option>
                            ))}
                          </select>
                        ) : (
                          <span className="hook-value">{a.kind === "art" ? "" : a.kind === "soundfont" ? "a sample tune" : "as it is"}</span>
                        )}
                        <span className="hook-key">{a.uses > 0 ? `in use by ${plural(a.uses, "hook", "hooks")}` : "not in use"}</span>
                        {a.kind === "art" ? (
                          <button type="button" className="hook-edit" data-testid="asset-show" aria-label={`Show ${a.shown}`} onClick={() => setShowing(a.path)}>
                            Show…
                          </button>
                        ) : (
                          <button
                            type="button"
                            className="hook-edit"
                            data-testid="asset-play"
                            aria-label={`${isPlaying ? "Stop" : "Play"} ${a.kind === "soundfont" ? `a sample tune through ${a.shown}` : a.shown}`}
                            aria-pressed={isPlaying}
                            onClick={() => void playOrStop(a)}
                          >
                            {isPlaying ? "Stop" : "Play"}
                          </button>
                        )}
                      </li>
                    );
                  })}
              </ul>
            </>
          ) : tab === "hooks" ? (
            <>
              {/* The columns' names, for the eye; each control says its own. */}
              <div className="hooks-head" aria-hidden="true">
                <span />
                <span>Name</span>
                <span>Value</span>
                <span>Sets off</span>
                <span />
              </div>
              <ul ref={listRef} className="hooks-list hooks-list-assets" data-testid="hooks-list" tabIndex={0} aria-label="Hooks: name, value, then what each sets off">
                {open &&
                  listing?.pairs.map((p, row) => (
                    <li key={`${p.key}\n${p.value}`}>
                      <input type="checkbox" checked={false} aria-label={`Move ${p.key} to the Filtered list`} onChange={() => void move(p.key, true, row)} />
                      <span className="hook-key">{p.key}</span>
                      <span className="hook-value">{p.value}</span>
                      <span className={p.trigger ? "hook-sets-off" : "hook-value"}>{summaryOf(p.trigger)}</span>
                      <button type="button" className="hook-edit" data-testid="hook-edit" aria-label={`Edit what ${said(p)} sets off: ${summaryOf(p.trigger)}`} onClick={() => setEditing(p)}>
                        Edit…
                      </button>
                    </li>
                  ))}
              </ul>
            </>
          ) : (
            <ul ref={listRef} className="hooks-list" data-testid="hooks-filtered-list" tabIndex={0} aria-label="Filtered names">
              {open && filtered?.keys.map((k, row) => (
                <li key={k.key}>
                  <input type="checkbox" checked aria-label={`${k.key}, filtered. Untick to bring it back to the hooks list`} onChange={() => void move(k.key, false, row)} />
                  <span className="hook-key">{k.key}</span>
                  <span className="hook-value">{plural(k.values, "value", "values")}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Dialog>
      {/* Beside the list's dialog, not in it: an overlay inside a moved box would be placed by it. */}
      <HookEditor
        pair={open ? editing : null}
        choices={(kind) => byKind.get(kind) ?? []}
        onChange={(change) => (editing ? setTrigger(editing, change) : Promise.resolve())}
        onClose={() => setEditing(null)}
      />
      <AssetPreviewDialog path={open ? showing : null} onClose={() => setShowing(null)} onError={onError} />
      <CreateAssetDialog
        open={open && creating}
        onClose={() => setCreating(false)}
        runActivity={runActivity}
        onMade={() => {
          onAssetsChanged();
          setVersion((v) => v + 1);
        }}
        onStatus={onStatus}
        onError={onError}
      />
    </>
  );
});

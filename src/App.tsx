import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { ActivityStatus } from "./components/ActivityStatus";
import { AppTesting } from "./components/AppTesting";
import { Dialog } from "./components/Dialog";
import { HeardPanel } from "./components/HeardPanel";
import { HookPicture } from "./components/HookPicture";
import { HooksDialog } from "./components/HooksDialog";
import { AppMarkIcon, ChecklistIcon, GearIcon, InfoIcon } from "./components/icons";
import { LicensesDialog } from "./components/LicensesDialog";
import { MapPanel } from "./components/MapPanel";
import { MixerDialog } from "./components/MixerDialog";
import { MusicEditorDialog } from "./components/MusicEditorDialog";
import { ArrangeContext, KeptContext, Movable, PICTURE_Z } from "./components/Movable";
import { ScenePanel } from "./components/ScenePanel";
import { ScreenReadout } from "./components/ScreenReadout";
import { ShrinkDialog } from "./components/ShrinkDialog";
import { RestoreDialog, type RestoreOffer } from "./components/RestoreDialog";
import * as backup from "./lib/backup";
import { SpeechDialog } from "./components/SpeechDialog";
import { TutorialDialog } from "./components/TutorialDialog";
import { CreationDialog } from "./components/CreationDialog";
import { CuesDialog } from "./components/CuesDialog";
import { ArtisanDialog } from "./components/ArtisanDialog";
import { EchoesDialog } from "./components/EchoesDialog";
import { DisplayDialog } from "./components/DisplayDialog";
import { PicturePanel } from "./components/PicturePanel";
import { VitalsPanel } from "./components/VitalsPanel";
import { PicturesDialog } from "./components/PicturesDialog";
import { loadPictures, pictureShows, savePictures, type PictureSettings } from "./lib/pictures";
import { CastDialog } from "./components/CastDialog";
import { JournalDialog } from "./components/JournalDialog";
import { SpeakingPopup } from "./components/SpeakingPopup";
import { describeUnheard, useJournal } from "./lib/journal";
import { StartupScreen } from "./components/StartupScreen";
import { hiddenLine, Terminal, type Hidden, type TermLine } from "./components/Terminal";
import { WorkshopDialog } from "./components/WorkshopDialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { DESKTOP_ONLY, DESKTOP_ONLY_KEYS, WEB } from "./lib/web";
import { open as openFiles, save as saveFile } from "@tauri-apps/plugin-dialog";
import { useActivities } from "./lib/activity";
import * as sound from "./lib/assets";
import * as creation from "./lib/creation";
import * as earcons from "./lib/earcons";
import * as echoes from "./lib/echoes";
import { useImmersive } from "./lib/immersive";
import * as mud from "./lib/mud";
import * as updates from "./lib/updates";
import { checkGrid, installRowSnap } from "./lib/grid";
import { loadDisplay, saveDisplay, type Display, type TextSize } from "./lib/display";
import { loadSpoken, saveSpoken, type Spoken, type SpokenKind } from "./lib/speech";
import { forgetLayout, layoutText } from "./lib/layout";
import { stageScale } from "./lib/stage";
import { applyTheme, loadTheme, type Theme } from "./lib/theme";
import { loadLayers, loadShown, loadUx, storeLayers, storeShown, storeUx, UXES, type Layers, type Ux } from "./lib/ux";
import * as voice from "./lib/voice";
import "./App.css";

/**
 * Launch-time work, each shown on the startup screen until it answers
 * (CLAUDE.md's "An app loading screen" convention). New launch-time work
 * joins this list with its own label.
 */
const STARTUP_STEPS = ["server", "map", "hooks"] as const;
type StartupStep = (typeof STARTUP_STEPS)[number];
const STARTUP_LABELS: Record<StartupStep, string> = {
  server: "Getting ready…",
  map: "Unfolding the map…",
  hooks: "Counting the hooks and checking their assets…",
};

/** Lines kept in the scrollback; older ones drop off the top. */
const SCROLLBACK = 5000;
/** Lines kept in the screen reader's hidden live region: it reads additions only, so a few will do. */
const HEARD_KEPT = 50;
/**
 * The default layout of each way to play (lib/ux.ts). Terminal and
 * Immersive always have theirs; Workshop's is where the player's
 * arrangement starts.
 *
 * Terminal: the game output 80 columns wide in the middle of the screen
 * and every row it can have (43 in the ANSIapps theme: the command line
 * on row 44, one row of messages on 45), the gear beside its top right
 * corner. Offline, the ways to play take its top 12 rows.
 *
 * Immersive: the menu bar; Here (the room, the compass, the vitals, the
 * fight, the sound layers) in the left 75 columns; the game output (80
 * by 25) on the right with Heard (what the voice said) under it; the
 * command line, its hint and the message bar across the bottom.
 * Offline, the ways to play take Here's place.
 */
function uxLayout(ux: Ux, theme: Theme, showing: Showing, textSize: TextSize = 1): Record<string, Usual> {
  if (ux === "workshop") return defaultLayout(theme, showing);
  const ansi = theme === "ansiapps";
  if (ux === "terminal" && textSize > 1) return bigTextLayout(theme, showing, textSize);
  if (ux === "terminal") {
    return ansi
      ? {
          terminal: { at: { x: 304, y: showing.connected ? 0 : 208 } },
          ports: { at: { x: 304, y: 0 }, size: { width: 672, height: showing.output ? 192 : undefined } },
          command: { at: { x: 304, y: 688 }, size: { width: 672, height: 16 } },
          messages: { at: { x: 304, y: 704 }, size: { width: 672, height: 16 } },
          gear: { at: { x: 984, y: 0 } },
        }
      : {
          terminal: { at: { x: 286, y: showing.connected ? 12 : 180 } },
          ports: { at: { x: 286, y: 12 }, size: { width: 708, height: showing.output ? 160 : undefined } },
          command: { at: { x: 286, y: 640 }, size: { width: 708 } },
          messages: { at: { x: 286, y: 686 }, size: { width: 708, height: 28 } },
          gear: { at: { x: 1004, y: 12 } },
        };
  }
  const workshop = defaultLayout(theme, showing);
  const keep = ["menu", "mode", "state", "hangup", "where", "mixer", "gear"];
  const bar = Object.fromEntries(keep.map((id) => [id, workshop[id]]));
  return ansi
    ? {
        ...bar,
        scene: { at: { x: 0, y: 16 }, size: { width: 600, height: 640 } },
        ports: { at: { x: 0, y: 16 }, size: { width: 600, height: 640 } },
        terminal: { at: { x: 608, y: 16 } },
        heard: { at: { x: 608, y: 432 }, size: { width: 672, height: 224 } },
        command: { at: { x: 0, y: 656 }, size: { width: 1280, height: 16 } },
        journal: { at: { x: 1144, y: 688 } },
        log: { at: { x: 1144, y: 704 } },
        hint: { at: { x: 0, y: 672 }, size: { width: 1280, height: 16 } },
        messages: { at: { x: 8, y: 688 }, size: { width: 1128, height: 32 } },
      }
    : {
        ...bar,
        scene: { at: { x: 12, y: 62 }, size: { width: 541, height: 545 } },
        ports: { at: { x: 12, y: 62 }, size: { width: 541, height: 545 } },
        terminal: { at: { x: 561, y: 62 } },
        heard: { at: { x: 561, y: 544 }, size: { width: 707, height: 63 } },
        command: { at: { x: 12, y: 615 }, size: { width: 1256 } },
        hint: { at: { x: 12, y: 654 }, size: { width: 1256 } },
        messages: { at: { x: 12, y: 672 }, size: { width: 1120, height: 48 } },
        journal: { at: { x: 1144, y: 672 } },
        log: { at: { x: 1144, y: 696 } },
      };
}

/**
 * Terminal with big text (lib/display.ts): the gear alone on the top
 * row, then the game output the screen's width, the command line one
 * big row, and the messages under it. In the ANSIapps theme every row
 * is a whole number of the big cells; offline, the ways to play take
 * the rows above the game output.
 */
function bigTextLayout(theme: Theme, showing: Showing, size: TextSize): Record<string, Usual> {
  const rows = terminalRows(theme, showing.connected, size);
  if (theme === "ansiapps") {
    const cell = 16 * size;
    const top = showing.connected ? 16 : 208;
    const command = top + rows * cell;
    return {
      gear: { at: { x: 1240, y: 0 } },
      terminal: { at: { x: 0, y: top } },
      ports: { at: { x: 0, y: 16 }, size: { width: 1280, height: showing.output ? 192 : undefined } },
      command: { at: { x: 0, y: command }, size: { width: 1280, height: cell } },
      messages: { at: { x: 0, y: command + cell }, size: { width: 1280, height: 720 - command - cell } },
    };
  }
  const line = 14 * size * 1.3;
  const top = showing.connected ? 12 : 180;
  const command = Math.round(top + rows * line + 27);
  const field = Math.round(14 * size * 1.2 + 20);
  return {
    gear: { at: { x: 1227, y: 12 } },
    terminal: { at: { x: 12, y: top } },
    ports: { at: { x: 12, y: 12 }, size: { width: 1256, height: showing.output ? 160 : undefined } },
    command: { at: { x: 12, y: command }, size: { width: 1256 } },
    messages: { at: { x: 12, y: command + field + 4 }, size: { width: 1256, height: 720 - command - field - 4 } },
  };
}

/** What each way to play has on its screen; Workshop has everything, less what the player hid. */
const UX_PARTS: Record<Exclude<Ux, "workshop">, Set<string>> = {
  terminal: new Set(["terminal", "ports", "command", "messages", "gear"]),
  immersive: new Set(["menu", "mode", "state", "hangup", "where", "mixer", "gear", "scene", "terminal", "heard", "ports", "command", "hint", "messages", "journal", "log"]),
};
/** The game output's rows in Terminal: all the screen has, or what's left under the ways to play; fewer, bigger rows with big text. */
function terminalRows(theme: Theme, connected: boolean, size: TextSize = 1): number {
  if (size === 2) return theme === "ansiapps" ? (connected ? 19 : 13) : connected ? 15 : 10;
  if (size === 3) return theme === "ansiapps" ? (connected ? 11 : 7) : connected ? 9 : 6;
  if (theme === "ansiapps") return connected ? 43 : 30;
  return connected ? 33 : 24;
}

/**
 * Workshop's default layout: where everything on the screen is, and how big,
 * until the user moves or resizes it in Workshop mode
 * (components/Movable.tsx), and again after Reset to Default Layout. In
 * stage pixels from the screen's top left. A width or height left out
 * is the size of what's in it.
 *
 * It's the screen as it was before things could be moved. In the
 * ANSIapps theme, by rows of the 45: the menu bar (1: the name and the
 * music playing on the left; the connection state, Hang Up, Where am
 * I?, Map, Mixer, BGM and the gear on the right), the ways to play from row 2, the game output's 25 from
 * row 15, a blank row, the command line (42), its hint (43), the map in
 * the last 52 columns beside all of those, the message bar (44, 45)
 * with the Journal and Log buttons, one above the other, then Hooks and
 * the screen readout at its right end. While connected, the You panel
 * (the gauges) takes the rows the ways to play had, above the game
 * output. The modern theme has the same arrangement in its own sizes.
 *
 * A few places depend on what's showing, as they did then: the command
 * line and the ways to play are the full width with the map hidden, the
 * ways to play keep to the rows above the game output when it's
 * showing too, and the connection state sits against Hang Up or, with
 * no Hang Up, against Where am I?.
 */
type Usual = { at: { x: number; y: number }; size?: { width?: number; height?: number } };
interface Showing {
  map: boolean;
  connected: boolean;
  /** The game output is on screen. */
  output: boolean;
}
function defaultLayout(theme: Theme, showing: Showing): Record<string, Usual> {
  if (theme === "ansiapps") {
    const main = showing.map ? 864 : 1280;
    return {
      menu: { at: { x: 0, y: 0 }, size: { width: 1280, height: 16 } },
      mode: { at: { x: 112, y: 0 }, size: { width: 112, height: 16 } },
      nowPlaying: { at: { x: 232, y: 0 } },
      state: { at: { x: showing.connected ? 648 : 728, y: 0 }, size: { width: 232, height: 16 } },
      hangup: { at: { x: 888, y: 0 } },
      where: { at: { x: 968, y: 0 } },
      mapToggle: { at: { x: 1080, y: 0 } },
      mixer: { at: { x: 1128, y: 0 } },
      bgm: { at: { x: 1192, y: 0 } },
      gear: { at: { x: 1240, y: 0 } },
      terminal: { at: { x: 0, y: 224 } },
      ports: { at: { x: 0, y: 16 }, size: { width: main, height: showing.output ? 192 : undefined } },
      map: { at: { x: 864, y: 16 }, size: { width: 416, height: 672 } },
      command: { at: { x: 0, y: 656 }, size: { width: main, height: 16 } },
      hint: { at: { x: 0, y: 672 }, size: { width: main, height: 16 } },
      messages: { at: { x: 8, y: 688 }, size: { width: 848, height: 32 } },
      journal: { at: { x: 864, y: 688 } },
      log: { at: { x: 864, y: 704 } },
      hooks: { at: { x: 1000, y: 688 } },
      readout: { at: { x: 1128, y: 688 } },
      scene: { at: { x: 864, y: 16 }, size: { width: 416, height: 672 } },
      heard: { at: { x: 0, y: 16 }, size: { width: main, height: 192 } },
      picture: { at: { x: 864, y: 16 }, size: { width: 416, height: 256 } },
      vitals: { at: { x: 0, y: 16 }, size: { width: 704, height: 64 } },
    };
  }
  const main = showing.map ? 844 : 1256;
  return {
    menu: { at: { x: 12, y: 12 }, size: { width: 1256 } },
    mode: { at: { x: 140, y: 21 }, size: { width: 112 } },
    nowPlaying: { at: { x: 264, y: 17 } },
    state: { at: { x: showing.connected ? 616 : 699, y: 21 }, size: { width: 230 } },
    hangup: { at: { x: 854, y: 17 } },
    where: { at: { x: 937, y: 17 } },
    mapToggle: { at: { x: 1040, y: 17 } },
    mixer: { at: { x: 1096, y: 17 } },
    bgm: { at: { x: 1171, y: 17 } },
    gear: { at: { x: 1227, y: 17 } },
    terminal: { at: { x: 12, y: 133 } },
    ports: { at: { x: 12, y: 62 }, size: { width: main, height: showing.output ? 128 : undefined } },
    map: { at: { x: 868, y: 62 }, size: { width: 400, height: 610 } },
    command: { at: { x: 12, y: 615 }, size: { width: main } },
    hint: { at: { x: 12, y: 654 }, size: { width: main } },
    messages: { at: { x: 12, y: 672 }, size: { width: 840, height: 48 } },
    journal: { at: { x: 864, y: 672 } },
    log: { at: { x: 864, y: 696 } },
    hooks: { at: { x: 1012, y: 683 } },
    readout: { at: { x: 1138, y: 688 } },
    scene: { at: { x: 868, y: 62 }, size: { width: 400, height: 610 } },
    heard: { at: { x: 12, y: 62 }, size: { width: main, height: 128 } },
    picture: { at: { x: 868, y: 62 }, size: { width: 400, height: 260 } },
    vitals: { at: { x: 12, y: 62 }, size: { width: 720, height: 66 } },
  };
}
/** The smallest a framed panel can be made: room for its frame and a few lines. */
const PANEL_LEAST = { width: 208, height: 128 };
/** You is two lines of gauges in its frame. */
const VITALS_LEAST = { width: 208, height: 64 };
/** Whether Workshop mode is on (things can be moved and resized): only "on" means it is. */
const ARRANGE_KEY = "coupler.arrange";
/** Commands kept for Up/Down, this session only. Passwords never are. */
const HISTORY = 200;
/** The way to play chosen last time (a port's ID), so it's offered first. */
const PORT_KEY = "coupler.port";
/** Whether the map panel is showing: "off" hides it. */
const MAP_KEY = "coupler.map";
/** Whether Coupler fills the screen: "on" means it does, and does again next launch. */
const FULLSCREEN_KEY = "coupler.fullscreen";

/**
 * Shown in the Keyboard dialog. Every one works from anywhere in the window.
 * Keys are written as words: the ANSIapps theme's font has no ⌘, ⇧ or ⌃,
 * and a character from another font would sit off the theme's grid.
 */
const ALL_SHORTCUTS: [string, string][] = [
  ["Go to the command line", "Esc"],
  ["Terminal: the plain terminal", "Ctrl+Cmd+1"],
  ["Immersive: the game in sound", "Ctrl+Cmd+2"],
  ["Workshop: your own screen", "Ctrl+Cmd+3"],
  ["Say where you are and the exits", "Cmd+Shift+L"],
  ["Say your health, mana and movement", "Cmd+Shift+V"],
  ["Say who you're fighting", "Cmd+Shift+E"],
  ["Say who's online", "Cmd+Shift+W"],
  ["Say what the game said since your last command", "Cmd+Shift+O"],
  ["Stop Coupler's voice and everything waiting to be said", "Cmd+Period"],
  ["Say what's not yet heard in the journal and the log", "Cmd+Shift+U"],
  ["Play the journal's unheard lines, oldest first", "Cmd+Shift+N"],
  ["Show the journal: what's said in the game", "Cmd+Shift+J"],
  ["Show the log: OOC, INFO and the other channels", "Cmd+Shift+K"],
  ["Review the output: the line before, the line after", "Option+Up, Option+Down"],
  ["Review from the oldest line", "Option+Home"],
  ["Back to live output", "Option+End"],
  ["Show the guide to making a character, while making one", "Cmd+Shift+G"],
  ["Show or hide the map", "Cmd+Shift+M"],
  ["Show the hooks list", "Cmd+Shift+H"],
  ["Show the Artisan's skills and ask the mentor", "Cmd+Shift+A"],
  ["Paint the room's picture, or paint it again in a new look", "Cmd+Shift+P"],
  ["Stop the sounds and music, hide the pictures", "Cmd+Shift+S"],
  ["Full screen on or off", "Ctrl+Cmd+F"],
  ["Earlier and later commands", "↑ ↓"],
  ["Send the command", "Return"],
];
const SHORTCUTS = WEB ? ALL_SHORTCUTS.filter(([, keys]) => !(keys.startsWith("Cmd+Shift+") && DESKTOP_ONLY_KEYS.includes(keys.slice(-1).toLowerCase()))) : ALL_SHORTCUTS;

/** A line's words, without the spaces around them. */
function lineText(l: TermLine): string {
  return l.line.map((span) => span.text).join("").trim();
}

/** "1 line", "12 lines". */
function countLines(n: number): string {
  return `${n} ${n === 1 ? "line" : "lines"}`;
}

function stored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function store(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // localStorage unavailable: the choice just won't persist across launches.
  }
}

function App() {
  const [startupPending, setStartupPending] = useState<StartupStep[]>([...STARTUP_STEPS]);
  const [theme, setTheme] = useState<Theme>(loadTheme);
  const [gearOpen, setGearOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [licensesOpen, setLicensesOpen] = useState(false);
  const [keysOpen, setKeysOpen] = useState(false);
  const [clearMapOpen, setClearMapOpen] = useState(false);
  const [appTestingOpen, setAppTestingOpen] = useState(false);
  const [server, setServer] = useState<mud.ServerInfo | null>(null);
  const [connected, setConnected] = useState(false);
  /** The way to play that's connected, or was chosen last. */
  const [portId, setPortId] = useState<string | null>(() => stored(PORT_KEY));
  const [lines, setLines] = useState<TermLine[]>([]);
  const [partial, setPartial] = useState<mud.Line | null>(null);
  /** The server echoes, so what's typed is a password. */
  const [serverEchoes, setServerEchoes] = useState(false);
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  /** Checking for a newer Coupler by itself (lib/updates.ts), off until turned on; and the newer version found, offered in the gear menu. */
  const [autoUpdates, setAutoUpdates] = useState(updates.loadAuto);
  const [newRelease, setNewRelease] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [mapOpen, setMapOpen] = useState(() => stored(MAP_KEY) !== "off");
  const [snapshot, setSnapshot] = useState<mud.MapSnapshot | null>(null);
  /** The fight (src-tauri/src/combat.rs): Combat mode while there's one, Explore mode otherwise. */
  const [opponent, setOpponent] = useState<mud.Opponent | null>(null);
  const [walking, setWalking] = useState(false);
  /** How many unique GMCP pairs the hooks list holds (filtered keys aren't counted). */
  const [hooksCount, setHooksCount] = useState(0);
  const [hooksOpen, setHooksOpen] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  /** The way to play: Terminal, Immersive or Workshop (lib/ux.ts). */
  const [ux, setUx] = useState<Ux>(loadUx);
  /** Workshop's choices: what shows, and Immersive's layers there. */
  const [shown, setShown] = useState(loadShown);
  const [layers, setLayers] = useState<Layers>(loadLayers);
  const [workshopOpen, setWorkshopOpen] = useState(false);
  const [cuesOpen, setCuesOpen] = useState(false);
  /** The commands' one-line answers said by the narrator, not written, in Immersive (lib/echoes.ts), and Narrator's Answers. */
  const [echoesSaid, setEchoesSaid] = useState(echoes.echoesOn);
  useEffect(() => echoes.onEchoesChanged(() => setEchoesSaid(echoes.echoesOn())), []);
  const [echoesOpen, setEchoesOpen] = useState(false);
  const [artisanOpen, setArtisanOpen] = useState(false);
  /** Which kinds of line the screen reader is given (lib/speech.ts), and the Speech dialog. */
  const [spoken, setSpoken] = useState<Spoken>(loadSpoken);
  const [speechOpen, setSpeechOpen] = useState(false);
  /** Before You Play: the practice game that teaches Immersive (components/TutorialDialog.tsx). */
  const [tutorialOpen, setTutorialOpen] = useState(false);
  /** Game colors, text size and sound captions (lib/display.ts), and the Display dialog. */
  const [display, setDisplay] = useState<Display>(loadDisplay);
  const [displayOpen, setDisplayOpen] = useState(false);
  /** The room's picture: what it shows (src-tauri/src/paint.rs), who paints it and how (lib/pictures.ts). */
  const [pictureScene, setPictureScene] = useState<mud.Scene | null>(null);
  const [pictureSettings, setPictureSettings] = useState<PictureSettings>(loadPictures);
  const [picturesOpen, setPicturesOpen] = useState(false);
  /** The room whose picture was asked for by key (Cmd+Shift+P), where pictures don't show in every room. */
  const [pictureAskedFor, setPictureAskedFor] = useState<string | null>(null);
  const [castOpen, setCastOpen] = useState(false);
  const [journalOpen, setJournalOpen] = useState(false);
  const [logOpen, setLogOpen] = useState(false);
  /** What the screen reader is given to read: the hidden live region's lines. */
  const [heard, setHeard] = useState<{ id: number; text: string }[]>([]);
  /** The character's health, mana and movement (src-tauri/src/senses.rs). */
  const [vitals, setVitals] = useState<mud.Vitals | null>(null);
  /** The question the game's asking while a character is made (src-tauri/src/creation.rs), and whether the player hid the guide to type instead. */
  const [creationStep, setCreationStep] = useState<mud.CreationStep | null>(null);
  /** Coupler's guide or CoffeeMUD's own way (lib/creation.ts): the guide starts hidden in the standard mode. */
  const [creationMode, setCreationMode] = useState<creation.CreationMode>(creation.loadMode);
  const creationModeRef = useRef(creationMode);
  creationModeRef.current = creationMode;
  const [guideHidden, setGuideHidden] = useState(() => creation.loadMode() === "standard");
  const guideOpen = creationStep !== null && !guideHidden && connected;
  /** What's typed is a password: the server echoes, or the guide knows it's asking for one (CoffeeMUD doesn't hide the account's). */
  const secret = serverEchoes || (creationStep?.secret ?? false);
  const creationRef = useRef<mud.CreationStep | null>(null);
  /** Kept at once when a question comes, so the output of the same read is already the guide's. */
  const guideOpenRef = useRef(guideOpen);
  guideOpenRef.current = guideOpen;
  const guideHiddenRef = useRef(guideHidden);
  guideHiddenRef.current = guideHidden;
  /** A character is in the game (it sent `room.info` or `char.vitals`), past the login, whose questions always show. */
  const [inGame, setInGame] = useState(false);
  /** Immersive's cues and voice: always there, in Workshop when chosen, never in Terminal. */
  const cues = ux === "immersive" || (ux === "workshop" && layers.cues);
  const voiced = ux === "immersive" || (ux === "workshop" && layers.voice);
  /** The first line of output after the last command sent, for Cmd+Shift+O. */
  const sentAt = useRef(0);
  const linesRef = useRef<TermLine[]>([]);
  /**
   * Review mode: the line being read back (its id), or null for live.
   * `newest` was the newest line when review began; `told` is how many
   * newer lines the player was last told of.
   */
  const [reviewId, setReviewId] = useState<number | null>(null);
  const reviewRef = useRef<{ newest: number; told: number }>({ newest: 0, told: 0 });
  const voicedRef = useRef(voiced);
  voicedRef.current = voiced;
  const cuesRef = useRef(cues);
  cuesRef.current = cues;
  const spokenRef = useRef(spoken);
  spokenRef.current = spoken;
  const reviewingRef = useRef(false);
  reviewingRef.current = reviewId !== null;
  /** The prompt last given to the screen reader, so an unchanged one isn't read again. */
  const lastPrompt = useRef("");
  const nextHeardId = useRef(0);
  /** The fixed screen's scale: 1 in the window, more in full screen. */
  const [scale, setScale] = useState(stageScale);
  const stageRef = useRef<HTMLDivElement>(null);
  const history = useRef<string[]>([]);
  /** Where Up/Down is in `history`; history.length means the new line. */
  const historyAt = useRef(0);
  const nextLineId = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const gearRef = useRef<HTMLDivElement>(null);
  const firstPortRef = useRef<HTMLButtonElement>(null);
  const { activities, runActivity, isBusy } = useActivities();

  const finishStep = (step: StartupStep) => setStartupPending((list) => list.filter((s) => s !== step));

  useEffect(() => applyTheme(theme), [theme]);
  // The ANSIapps theme's grid: scrolling rests on whole rows.
  useEffect(installRowSnap, []);

  useEffect(() => {
    mud
      .serverInfo()
      .then((info) => {
        setServer(info);
        setConnected(info.connected);
        if (info.portId) setPortId(info.portId);
      })
      .catch((e) => setError(String(e)))
      .finally(() => finishStep("server"));
    mud
      .mapSnapshot()
      .then(setSnapshot)
      .catch((e) => setError(String(e)))
      .finally(() => finishStep("map"));
    mud
      .hooksCount()
      .then(setHooksCount)
      // Any asset whose file has gone is cleared from the hooks now, not when it next fails to play.
      .then(() => mud.assetsList())
      .then(({ cleared }) => {
        if (cleared.length > 0) setStatus(mud.clearedInWords(cleared));
      })
      .catch((e) => setError(String(e)))
      .finally(() => finishStep("hooks"));
  }, []);

  // The window is one fixed size; full screen scales the same screen up.
  // The system's own controls can leave full screen too, so ask after any resize.
  useEffect(() => {
    const onResize = () => {
      setScale(stageScale());
      mud
        .isFullscreen()
        .then(setFullscreen)
        .catch(() => {
          // The window is going away; nothing to update.
        });
    };
    window.addEventListener("resize", onResize);
    onResize();
    if (stored(FULLSCREEN_KEY) === "on") mud.setFullscreen(true).catch((e) => setError(String(e)));
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const append = useCallback((added: TermLine[]) => {
    setLines((list) => {
      const next = list.concat(added);
      return next.length > SCROLLBACK ? next.slice(next.length - SCROLLBACK) : next;
    });
  }, []);

  /**
   * Gives a line to the screen reader, if its kind is one the player
   * keeps on. Not while Coupler's own voice speaks (it would be said
   * twice) or while reviewing (the output holds still).
   */
  const hear = useCallback((text: string, kind: SpokenKind) => {
    if (voicedRef.current || reviewingRef.current || !spokenRef.current[kind] || text.trim() === "") return;
    // While the guide to making a character is open, it says the question in few words: not the screens of text.
    if (guideOpenRef.current && (kind === "game" || kind === "prompt")) return;
    setHeard((list) => list.slice(-(HEARD_KEPT - 1)).concat({ id: nextHeardId.current++, text: voice.pronounce(text) }));
  }, []);

  const note = useCallback(
    (text: string, kind: TermLine["kind"] = "note") => {
      append([{ id: nextLineId.current++, kind, line: [{ text }] }]);
      hear(text, kind ?? "note");
    },
    [append, hear],
  );

  useEffect(() => {
    const subscriptions = [
      mud.onOutput((e) => {
        if (e.lines.length > 0) append(e.lines.map((line, i) => ({ id: nextLineId.current++, line, talk: e.kinds[i] === "talk", time: e.kinds[i] === "time", echo: e.kinds[i] === "echo", prompt: e.kinds[i] === "prompt" })));
        setPartial(e.partial);
        // A finished prompt was already read as it came.
        e.lines.forEach((line, i) => {
          const kind = e.kinds[i] ?? "game";
          if (kind !== "prompt") hear(line.map((span) => span.text).join(""), kind);
        });
        // A long look's hidden details are shown only by color: they're said in words after the room.
        if (e.hidden) hear(e.hidden.words.length > 0 ? `Hidden details: ${mud.hiddenList(e.hidden.words)}.` : "No hidden details.", "game");
        const prompt = e.partial ? e.partial.map((span) => span.text).join("").trim() : "";
        if (prompt !== lastPrompt.current) {
          lastPrompt.current = prompt;
          hear(prompt, "prompt");
        }
      }),
      mud.onEcho(setServerEchoes),
      mud.onCreation((step) => {
        const before = creationRef.current;
        creationRef.current = step;
        guideOpenRef.current = step !== null && !guideHiddenRef.current;
        setCreationStep(step);
        if (step) return;
        setGuideHidden(creationModeRef.current === "standard");
        // The rules were the last question: the new character's in the game.
        if (before?.kind === "rules") {
          if (cuesRef.current) earcons.creationDone();
          setError(null);
          setStatus("Your new character is in the game. LOOK shows where you are; HELP explains the rest.");
        }
      }),
      mud.onGmcp((e) => {
        const p = e.package.toLowerCase();
        if (p === "room.info" || p === "char.vitals") setInGame(true);
      }),
      mud.onCombat(setOpponent),
      mud.onVitals(setVitals),
      mud.onClosed((reason) => {
        setConnected(false);
        setOpponent(null);
        setVitals(null);
        setServerEchoes(false);
        setPartial(null);
        setInGame(false);
        creationRef.current = null;
        setCreationStep(null);
        setGuideHidden(creationModeRef.current === "standard");
        lastPrompt.current = "";
        // Off the game, nothing it set off goes on: the sounds stop and the pictures close.
        sound.stopAll();
        setPictures([]);
        if (reason) {
          setStatus(reason);
          note(`*** ${reason}`);
          if (voicedRef.current) voice.speak(reason, true);
        }
      }),
      // A logout or a switch: the old character's sounds fade out and their pictures close.
      mud.onCharacterLeft((message) => {
        setInGame(false);
        sound.fadeAll();
        setPictures([]);
        setError(null);
        setStatus(message);
      }),
      // Someone logged on or off: the game's announcement isn't written, a doorbell says it instead.
      mud.onWho((changes) => changes.slice(0, 4).forEach((c, i) => earcons.loggedOnOff(c.name, c.on, i * 0.8))),
      mud.onMapChanged(setSnapshot),
      mud.onHooksChanged(setHooksCount),
      mud.onMapWalk((e) => {
        setWalking(e.walking);
        setError(null);
        setStatus(e.message);
      }),
    ];
    return () => subscriptions.forEach((s) => void s.then((unlisten) => unlisten()));
  }, [append, note, hear]);

  const onSize = useCallback((columns: number, rows: number) => {
    mud.resize(columns, rows).catch(() => {
      // Only fails if the socket just dropped; the closed event reports that.
    });
  }, []);

  const announcePanel = useCallback((message: string) => {
    setError(null);
    setStatus(message);
  }, []);
  const showTerminal = connected || lines.length > 0;
  const pictureLandmark = !!pictureScene && snapshot?.room?.id === pictureScene.room && !!snapshot.room.landmark;
  const pictureOn = pictureShows(pictureSettings, pictureScene, pictureLandmark, pictureAskedFor);
  /** Big text is Terminal's: the other ways to play have too much on the screen for it. */
  const textSize: TextSize = ux === "terminal" ? display.textSize : 1;
  const usual = uxLayout(ux, theme, { map: mapOpen && ux === "workshop", connected, output: showTerminal }, textSize);
  /** Whether a thing is on this way to play's screen. */
  const has = (id: string) => !(WEB && DESKTOP_ONLY.has(id)) && (ux === "workshop" ? (shown[id] ?? true) : UX_PARTS[ux].has(id));
  linesRef.current = lines;
  /** In Immersive, talk isn't in the game output (it's in the journal and the log, and the pop-up shows it while it's said), nor the time of day (the narrator says it). */
  /** Nor the player's prompt, once in the game: Immersive has no need of it to look at. */
  const hide: Hidden = useMemo(() => ({ talk: ux === "immersive", prompt: ux === "immersive" && inGame, echo: ux === "immersive" && echoesSaid }), [ux, inGame, echoesSaid]);
  const hideRef = useRef(hide);
  hideRef.current = hide;
  const [arranging, setArranging] = useState(() => stored(ARRANGE_KEY) === "on");
  const [usualLayoutOpen, setUsualLayoutOpen] = useState(false);
  function toggleArranging() {
    setArranging((was) => {
      store(ARRANGE_KEY, was ? "off" : "on");
      return !was;
    });
  }
  /** Dev-only: the arrangement on screen as text, for making it the default layout. */
  function devCopyLayout() {
    setGearOpen(false);
    navigator.clipboard.writeText(`${theme} theme\n${layoutText()}`).then(
      () => showStatus("Dev: the layout was copied."),
      () => showError("Dev: the layout couldn't be copied."),
    );
  }

  const ports = server?.ports ?? [];
  const port = ports.find((p) => p.id === portId) ?? null;
  // The way to play chosen last time is offered first; a new player gets Standard.
  const orderedPorts = port ? [port, ...ports.filter((p) => p !== port)] : ports;
  const startupDone = startupPending.length === 0;

  /**
   * The greeting, once the launch screen is gone: the times played with
   * Coupler on this Mac, and how many are online now in the game last
   * chosen (asked of the game itself, by MSSP), and a warning when the
   * game runs another CoffeeMUD version than Coupler was made for, and a
   * newer Coupler when the player has it checked for. In the status bar, where
   * the screen reader reads it, and said by the narrator when Coupler's
   * voice is on. Not once a game's under way: it'd talk over the login.
   */
  const greeted = useRef(false);
  const connectedRef = useRef(connected);
  connectedRef.current = connected;
  useEffect(() => {
    if (!startupDone || greeted.current || WEB) return;
    greeted.current = true;
    // Only asked of GitHub when the player turned automatic checks on,
    // once a day at most; a failed check says nothing.
    const release = updates.autoDue() ? updates.check().catch(() => null) : Promise.resolve(null);
    // No counts, no greeting: playing doesn't need it.
    void Promise.all([mud.launchCounts(portId ?? "standard").catch(() => null), release]).then(([counts, found]) => {
      const newer = found?.newer ? updates.describe(found) : null;
      if (found?.newer) setNewRelease(found.latest);
      if (connectedRef.current) return;
      const text = [counts && mud.describeLaunchCounts(counts), newer].filter(Boolean).join(" ");
      if (text) setStatus(text);
      // Another CoffeeMUD than the one Coupler was made for: a warning
      // in the status bar (an alert, so the screen reader says it too),
      // until the next connect clears it.
      const warning = counts && mud.describeVersionFit(counts);
      if (warning) setError(warning);
      const said = [text, warning].filter(Boolean).join(" ");
      if (said && voicedRef.current) voice.speak(said);
    });
  }, [startupDone, portId]);

  /** Gear → Check for Updates Now: asks GitHub whether a newer Coupler is out. */
  async function checkUpdates() {
    setError(null);
    try {
      const found = await runActivity("Checking for a newer Coupler…", () => updates.check(), "update");
      setNewRelease(found.newer ? found.latest : null);
      const text = updates.describe(found);
      setStatus(text);
      if (voicedRef.current) voice.speak(text);
    } catch (e) {
      setError(String(e));
    }
  }

  /** Gear → Check for Updates Automatically. */
  function toggleAutoUpdates(on: boolean) {
    updates.saveAuto(on);
    setAutoUpdates(on);
    setError(null);
    setStatus(on ? "Coupler will ask GitHub for a newer version once a day, at launch." : "Coupler won't check for updates by itself.");
  }

  // Once the launch screen is gone, the keyboard starts on the first way
  // to play, so Return connects without hunting for it.
  useEffect(() => {
    if (startupDone && !connected) firstPortRef.current?.focus();
  }, [startupDone, connected]);

  async function handleConnect(to: mud.Port) {
    setError(null);
    setStatus(null);
    try {
      await runActivity(`Dialing CoffeeMUD ${to.name}…`, () => mud.connect(to.id), "connect");
      setPortId(to.id);
      store(PORT_KEY, to.id);
      setConnected(true);
      setStatus(`Connected to CoffeeMUD ${to.name}.`);
      if (voiced) voice.speak(`Connected to CoffeeMUD ${to.name}.`, true);
      // The input is disabled until this render lands.
      window.setTimeout(() => inputRef.current?.focus(), 0);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDisconnect() {
    setError(null);
    try {
      await runActivity("Hanging up…", () => mud.disconnect(), "connect");
      setStatus("Disconnected from CoffeeMUD.");
      note("*** Disconnected.");
    } catch (e) {
      setError(String(e));
    }
  }

  /** An answer from the guide to making a character: written in the output (a password as stars), never in the history. */
  const sendAnswer = useCallback((line: string, secret: boolean) => {
    sentAt.current = nextLineId.current;
    append([{ id: nextLineId.current++, kind: "sent", line: [{ text: secret ? "********" : line }] }]);
    mud.sendLine(line).catch((e) => setError(String(e)));
  }, [append]);
  const hideGuide = useCallback(() => {
    setGuideHidden(true);
    setError(null);
    setStatus("The guide is hidden: type your answers. Cmd+Shift+G shows it again.");
  }, []);
  /** Coupler's guide or CoffeeMUD's own way, from now on (the guide's button and the gear menu). */
  const chooseCreationMode = useCallback((mode: creation.CreationMode) => {
    creation.storeMode(mode);
    setCreationMode(mode);
    setGuideHidden(mode === "standard");
    setError(null);
    const text =
      mode === "standard"
        ? "Standard mode: CoffeeMUD's own screens, answers typed. Cmd+Shift+G or the gear menu brings Coupler's guide back."
        : "Guide mode: Coupler asks the questions in few words.";
    setStatus(text);
    if (voicedRef.current) voice.speak(text, true);
  }, []);

  const send = useCallback(async (line: string) => {
    try {
      await mud.sendLine(line);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  async function handleSend() {
    const line = input;
    setInput("");
    sentAt.current = nextLineId.current;
    reviewLive(true);
    if (secret) {
      note("********", "sent");
    } else {
      note(line, "sent");
      if (line.trim() !== "" && history.current[history.current.length - 1] !== line) {
        history.current.push(line);
        if (history.current.length > HISTORY) history.current.shift();
      }
    }
    historyAt.current = history.current.length;
    await send(line);
  }

  /** A map action: a bar while it runs, a status on success, an error on failure. */
  const runMap = useCallback(
    (label: string, work: () => Promise<string | void>) => {
      setError(null);
      runActivity(label, work, "map")
        .then((message) => {
          if (message) setStatus(message);
        })
        .catch((e) => setError(String(e)));
    },
    [runActivity],
  );

  /**
   * The answer to a say key: in the status bar, where a screen reader
   * reads it, and spoken by Coupler's voice when it's on, cutting off
   * whatever it was saying.
   */
  const sayNow = useCallback(
    (text: string) => {
      setError(null);
      // A changed string is what makes a screen reader speak it again.
      setStatus((was) => (was === text ? `${text} ` : text));
      if (voiced) voice.speak(text, true);
    },
    [voiced],
  );

  /** The journal and the log as heard (lib/journal.ts): talk spoken as it comes, tones for what's waiting. */
  const journal = useJournal({ cues, voiced, logOpen });
  const { touchLog } = journal;
  const sayUnheard = useCallback(() => {
    touchLog();
    sayNow(describeUnheard(journal.unheard));
  }, [sayNow, journal.unheard, touchLog]);
  const { playUnheard: playJournal } = journal;
  const playUnheard = useCallback(() => {
    playJournal("journal").then(
      (n) => setStatus(n === 0 ? "Everything in the journal has been heard." : `Playing ${n} unheard ${n === 1 ? "line" : "lines"}. Cmd+Period stops.`),
      (e) => setError(String(e)),
    );
  }, [playJournal]);

  const whereAmI = useCallback(() => sayNow(mud.describeRoom(snapshot?.room ?? null)), [sayNow, snapshot]);
  const sayVitals = useCallback(() => sayNow(mud.describeVitals(vitals)), [sayNow, vitals]);
  const sayOpponent = useCallback(() => sayNow(mud.describeOpponent(opponent)), [sayNow, opponent]);
  const sayWho = useCallback(() => {
    mud.whoNow().then(
      (report) => sayNow(mud.describeWho(report)),
      (e) => setError(String(e)),
    );
  }, [sayNow]);
  /** What the game said since the last command: its words, not its art, the last 15 lines at most. */
  const sayRecent = useCallback(() => {
    const said = linesRef.current
      .filter((l) => l.id >= sentAt.current && !l.kind && !hiddenLine(l, hideRef.current))
      .map((l) => l.line.map((span) => span.text).join("").trim())
      .filter(voice.speakable)
      .slice(-15);
    sayNow(said.length > 0 ? said.join(" ") : "The game hasn't said anything since your last command.");
  }, [sayNow]);

  /**
   * Review mode, by line: Option+Up reads the newest line and then each
   * before it, Option+Down each after, Option+Home the oldest. Blank
   * lines are skipped, art is called art. While reviewing the output
   * holds still and isn't announced; what comes in meanwhile is counted
   * and said once per change. Option+End, Option+Down past the newest,
   * or sending a command goes back to live.
   */
  const reviewLive = useCallback(
    (quietly = false) => {
      if (reviewId === null) {
        if (!quietly) sayNow("Already live.");
        return;
      }
      const after = Math.max(reviewId, reviewRef.current.newest);
      const missed = linesRef.current.filter((l) => l.id > after && lineText(l) !== "" && !hiddenLine(l, hideRef.current)).length;
      setReviewId(null);
      if (!quietly) sayNow(missed > 0 ? `Live. ${countLines(missed)} came in after the line you were on.` : "Live.");
    },
    [reviewId, sayNow],
  );
  const reviewStep = useCallback(
    (step: "earlier" | "later" | "oldest") => {
      const list = linesRef.current.filter((l) => lineText(l) !== "" && !hiddenLine(l, hideRef.current));
      if (list.length === 0) {
        sayNow("There's no output to review yet.");
        return;
      }
      let at: number;
      let edge = "";
      if (reviewId === null) {
        reviewRef.current = { newest: list[list.length - 1].id, told: 0 };
        if (step === "later") {
          sayNow("Already live. Option+Up reviews the output.");
          return;
        }
        at = step === "oldest" ? 0 : list.length - 1;
      } else {
        // The line under review may have gone off the top of the scrollback.
        const found = list.findIndex((l) => l.id >= reviewId);
        const here = found === -1 ? list.length - 1 : found;
        if (step === "oldest") at = 0;
        else if (step === "earlier") at = Math.max(here - 1, 0);
        else at = here + 1;
        if (step === "earlier" && here === 0) edge = "Oldest line. ";
        if (at >= list.length) {
          reviewLive();
          return;
        }
      }
      const line = list[at];
      const newer = list.filter((l) => l.id > Math.max(line.id, reviewRef.current.newest)).length;
      const news = newer > 0 && newer !== reviewRef.current.told ? `${newer} new ${newer === 1 ? "line" : "lines"} below. ` : "";
      reviewRef.current.told = newer;
      setReviewId(line.id);
      const text = lineText(line);
      sayNow(`${edge}${news}${voice.speakable(text) ? text : "A line of symbols."}`);
    },
    [reviewId, sayNow, reviewLive],
  );

  // Immersive's cues and voice (lib/immersive.ts).
  useImmersive({ cues, voice: voiced, connected, snapshot, opponent, vitals, guideHidden });

  /** Chooses a way to play: the screen changes, and it's said. */
  const chooseUx = useCallback(
    (next: Ux) => {
      setGearOpen(false);
      const info = UXES.find((u) => u.id === next)!;
      if (next === ux) {
        sayNow(`Already in ${info.name}.`);
        return;
      }
      storeUx(next);
      setUx(next);
      if (next === "immersive" || (next === "workshop" && layers.cues)) earcons.modeChanged();
      const intro =
        next === "immersive"
          ? connected
            ? "Immersive. Listen as you move: the exits sound where they lead. Command Shift L says where you are, Command Period stops me."
            : "Immersive. Press Return to play, Tab to hear the other ways to play, or Shift Tab for Before You Play, a practice game that teaches Immersive."
          : `${info.name}. ${info.summary}`;
      setError(null);
      setStatus(intro);
      if (next === "immersive" || (next === "workshop" && layers.voice)) voice.speak(intro, true);
      else voice.hush();
      if (next !== "immersive" && !(next === "workshop" && layers.cues)) earcons.stopCues();
      // The screen is built afresh: the keyboard goes back where it belongs.
      window.setTimeout(() => (connected ? inputRef.current?.focus() : firstPortRef.current?.focus()), 0);
    },
    [ux, layers, connected, sayNow],
  );

  function changeShown(id: string, on: boolean) {
    setShown((was) => {
      const next = { ...was, [id]: on };
      storeShown(next);
      return next;
    });
  }

  function changeLayers(next: Layers) {
    storeLayers(next);
    setLayers(next);
    if (!next.cues) earcons.stopCues();
    if (!next.voice) voice.hush();
    showStatus(`Sound cues ${next.cues ? "on" : "off"}, Coupler's voice ${next.voice ? "on" : "off"}, in Workshop.`);
  }

  const toggleMap = useCallback(() => {
    store(MAP_KEY, mapOpen ? "off" : "on");
    setStatus(mapOpen ? "Map hidden." : "Map shown.");
    setMapOpen(!mapOpen);
  }, [mapOpen]);

  const toggleFullscreen = useCallback(() => {
    const on = !fullscreen;
    setError(null);
    mud
      .setFullscreen(on)
      .then(() => {
        store(FULLSCREEN_KEY, on ? "on" : "off");
        setFullscreen(on);
        setStatus(on ? "Full screen on." : "Full screen off.");
      })
      .catch((e) => setError(String(e)));
  }, [fullscreen]);

  function handleKey(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleSend();
      // With Option, Up and Down are review mode's, handled with the other shortcuts.
    } else if ((e.key === "ArrowUp" || e.key === "ArrowDown") && !secret && !e.altKey) {
      e.preventDefault();
      const h = history.current;
      const at = Math.max(0, Math.min(h.length, historyAt.current + (e.key === "ArrowUp" ? -1 : 1)));
      historyAt.current = at;
      setInput(at < h.length ? h[at] : "");
    }
  }

  const showError = useCallback((message: string) => setError(message), []);
  /** Stable, so the memoized hooks list isn't redrawn on every key typed. */
  const closeHooks = useCallback(() => setHooksOpen(false), []);
  const showStatus = useCallback((message: string) => {
    setError(null);
    setStatus(message);
  }, []);

  // ---- Assets: what the hooks' triggers play and show (lib/assets.ts) ----

  /**
   * The pictures triggers are showing, oldest first (so the newest is in
   * front): each at a column and row of the screen, from 1 like the
   * readout. A new one at a picture's place replaces it. `fade` is its
   * seconds before it fades away (0: it stays); `shownAt` is bumped when
   * its trigger goes off again, so that wait starts over.
   */
  const [pictures, setPictures] = useState<({ id: number; path: string; x: number; y: number; fade: number; shownAt: number } & sound.Shown)[]>([]);
  const nextPicture = useRef(0);
  /** The music playing now (an asset path), for the menu bar and the Mixer. */
  const [nowPlaying, setNowPlaying] = useState<string | null>(null);
  const [mixerOpen, setMixerOpen] = useState(false);
  const [musicOpen, setMusicOpen] = useState(false);
  useEffect(() => sound.onMusic(setNowPlaying), []);
  /** Bumped when the Assets folder changes, so the hooks list reads it again. */
  const [assetsVersion, setAssetsVersion] = useState(0);
  /** WAVs just added that could be smaller: the question open, or null. */
  const [shrinkOffer, setShrinkOffer] = useState<mud.Added[] | null>(null);
  /** A Coupler Backup chosen or dropped, waiting for a yes. */
  const [restoreOffer, setRestoreOffer] = useState<RestoreOffer | null>(null);
  const [restoring, setRestoring] = useState(false);
  const picturesRef = useRef(pictures);
  picturesRef.current = pictures;
  /** Each failure is said once a run: a broken trigger goes off in every room. */
  const saidFailures = useRef(new Set<string>());

  /** A trigger's sound or picture that failed: said once a run, and a file gone from Assets cleared from the hooks. */
  const triggerFailed = useCallback((e: unknown) => {
    const message = String(e);
    if (saidFailures.current.has(message)) return;
    saidFailures.current.add(message);
    setError(message);
    // Most likely its file has left the Assets folder: clear it from the hooks.
    mud
      .assetsList()
      .then(({ cleared }) => {
        if (cleared.length === 0) return;
        setError(null);
        setStatus(mud.clearedInWords(cleared));
        setAssetsVersion((v) => v + 1);
      })
      .catch(() => {});
  }, []);

  const onFired = useCallback((fired: mud.Fired[]) => {
    const failed = triggerFailed;
    // BGN and BGW aren't set off here: they loop while they apply (the `ambient` event).
    for (const { trigger } of fired) {
      if (trigger.sfx) sound.playEffect(trigger.sfx, trigger.sfxVolume).catch(failed);
      if (trigger.bgm) sound.playMusic(trigger.bgm, trigger.bgmLoop, trigger.bgmVolume).catch(failed);
      const art = trigger.art;
      if (art) {
        const { artX: x, artY: y, artFade: fade } = trigger;
        sound
          .picture(art)
          .then((ready) => {
            const same = (p: { path: string; x: number; y: number }) => p.path === art && p.x === x && p.y === y;
            // Already showing there: it stays, its fade starts over, and nothing is said again.
            if (picturesRef.current.some(same)) {
              setPictures((list) => list.map((p) => (same(p) ? { ...p, fade, shownAt: p.shownAt + 1 } : p)));
              return;
            }
            setPictures((list) => [...list.filter((p) => p.x !== x || p.y !== y), { id: nextPicture.current++, path: art, x, y, fade, shownAt: 0, ...ready }]);
            setError(null);
            setStatus(
              fade > 0
                ? `Picture: ${mud.assetTitle(art)}, for ${fade} second${fade === 1 ? "" : "s"}. Click it to close it, or Cmd+Shift+S for all.`
                : `Picture: ${mud.assetTitle(art)}. Click it to close it, or Cmd+Shift+S for all.`,
            );
          })
          .catch(failed);
      }
    }
  }, [triggerFailed]);

  // The room's picture: Rust says when the room, its weather or the time changes.
  useEffect(() => {
    mud.pictureNow().then(setPictureScene).catch(() => setPictureScene(null));
    const unlisten = mud.onPicture(setPictureScene);
    return () => void unlisten.then((f) => f());
  }, []);

  // Background noise and weather: Rust says what should loop (src-tauri/src/ambient.rs).
  useEffect(() => {
    const apply = (a: mud.Ambience) => {
      sound.setLoop("bgn", a.bgn).catch(triggerFailed);
      sound.setLoop("bgw", a.bgw).catch(triggerFailed);
    };
    mud.ambienceNow().then(apply).catch(triggerFailed);
    const unlisten = mud.onAmbient(apply);
    return () => void unlisten.then((f) => f());
  }, [triggerFailed]);

  /**
   * Cmd+Shift+P: where the room's picture isn't showing (landmarks only,
   * or when asked), it shows; where it is, it's painted again in a new
   * look the room keeps (src-tauri/src/paint.rs, `Looks`).
   */
  const paintRoom = useCallback(() => {
    if (pictureSettings.engine === "none") {
      showStatus("Pictures are off. Gear, then Pictures, turns them on.");
      return;
    }
    if (!pictureScene) {
      showStatus("Once you're in a room, Cmd+Shift+P paints it.");
      return;
    }
    if (pictureScene.art) {
      showStatus(`This room shows your own picture, ${mud.assetTitle(pictureScene.art)}. The hooks list changes it.`);
      return;
    }
    if (!pictureOn) {
      setPictureAskedFor(pictureScene.room);
      showStatus(`A picture of ${pictureScene.name || "this room"}, until you leave it.`);
      return;
    }
    runActivity("Painting the room again…", () => mud.pictureAgain(), "picture")
      .then((scene) => showStatus(scene ? `A new look for ${scene.name || "this room"}, kept for next time. Gear, then Pictures, has First Looks Back.` : "You're not in a room Coupler knows yet."))
      .catch((e) => setError(String(e)));
  }, [pictureSettings.engine, pictureScene, pictureOn, runActivity, showStatus]);

  const stopEverything = useCallback(() => {
    sound.stopAll();
    earcons.stopCues();
    voice.hush();
    setPictures([]);
    setError(null);
    setStatus("Sounds, music, background noise and weather stopped, and the pictures closed. Background noise and weather start again when what should play next changes.");
  }, []);

  /**
   * Copies files into the Assets folder, the bar filling by bytes read
   * and its label saying which file and what's being done to it (once a
   * step, not every chunk: the activity area is a live region), then
   * says what became of each file.
   */
  const importAssets = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      try {
        const done = await runActivity(
          `Adding ${paths.length === 1 ? "a file" : `${paths.length} files`} to the Assets folder…`,
          (update) => {
            let said = "";
            return mud.assetsImport(paths, (p) => {
              const step = {
                reading: "reading it",
                checking: "checking it isn't already there",
                saving: "saving the copy",
                compressing: "seeing how small it can be made",
              }[p.step];
              const label = `Adding file ${p.file} of ${p.files} to the Assets folder, ${p.name}: ${step}… (${mud.sizeInWords(p.doneBytes)} of ${mud.sizeInWords(p.totalBytes)} so far)`;
              const value = p.totalBytes > 0 ? p.doneBytes / p.totalBytes : (p.file - 1) / p.files;
              const stepSaid = `${p.file} ${p.step}`;
              update(stepSaid === said ? { value } : { value, label });
              said = stepSaid;
            });
          },
          "assets",
        );
        sound.forgetDecoded();
        saidFailures.current.clear();
        setAssetsVersion((v) => v + 1);
        const each = done.added.map((a) =>
          a.how === "new"
            ? `${a.from} added as ${a.path}`
            : a.how === "renamed"
              ? `${a.from} added as ${a.path} (another ${a.from} was already there)`
              : `${a.from} was already there, as ${a.path}`,
        );
        const of = `${done.added.length} of ${paths.length} ${paths.length === 1 ? "file" : "files"}`;
        const added = done.added.length > 0 ? `Assets: ${of} taken. ${each.join("; ")}.` : "";
        if (done.skipped.length > 0) setError(`${added} Not added: ${done.skipped.join(" ")}`.trim());
        else showStatus(added);
        const offers = done.added.filter((a) => a.smaller);
        // Added to any question still open (files dropped while it's up).
        if (offers.length > 0) setShrinkOffer((open) => [...(open ?? []), ...offers]);
      } catch (e) {
        setError(String(e));
      }
    },
    [runActivity, showStatus],
  );

  /** No to the smaller copies: the WAVs stay, and the copies are forgotten. */
  const keepWavs = useCallback(() => {
    if (!shrinkOffer) return;
    setShrinkOffer(null);
    mud
      .assetsKeep(shrinkOffer.map((a) => a.path))
      .then(() => showStatus(shrinkOffer.length === 1 ? `${shrinkOffer[0].name} was kept as a WAV.` : `The ${shrinkOffer.length} WAVs were kept as they are.`))
      .catch((e) => setError(String(e)));
  }, [shrinkOffer, showStatus]);

  /** Yes: each smaller copy takes its WAV's place in the Assets folder, and the WAV is deleted. */
  const makeSmaller = useCallback(async () => {
    if (!shrinkOffer) return;
    setShrinkOffer(null);
    try {
      const done = await runActivity(
        shrinkOffer.length === 1 ? `Making ${shrinkOffer[0].name} smaller…` : `Making ${shrinkOffer.length} sounds smaller…`,
        () => mud.assetsCompress(shrinkOffer.map((a) => a.path)),
        "assets",
      );
      sound.forgetDecoded();
      setAssetsVersion((v) => v + 1);
      const saved = done.done.reduce((sum, d) => sum + d.before - d.after, 0);
      const each = done.done.map((d) => `${d.from} is now ${d.path}`);
      const made = done.done.length > 0 ? `${mud.sizeInWords(saved)} saved: ${each.join("; ")}.` : "";
      if (done.skipped.length > 0) setError(`${made} Kept as a WAV: ${done.skipped.join(" ")}`.trim());
      else showStatus(made);
    } catch (e) {
      setError(String(e));
    }
  }, [shrinkOffer, runActivity, showStatus]);

  /** An asset made, or a SoundFont chosen, in the Hooks dialog: what was decoded may be out of date. */
  const assetsChangedHere = useCallback(() => {
    sound.forgetDecoded();
    saidFailures.current.clear();
  }, []);

  /** The keyboard's way to add assets: a file chooser. */
  const chooseAssets = useCallback(async () => {
    try {
      const chosen = await openFiles({ multiple: true, title: "Add to Coupler's Assets", filters: [{ name: "Sounds, music, pictures and SoundFonts", extensions: mud.ASSET_TYPES }] });
      if (chosen) await importAssets(Array.isArray(chosen) ? chosen : [chosen]);
    } catch (e) {
      setError(String(e));
    }
  }, [importAssets]);

  // ---- Coupler Backup (lib/backup.ts, src-tauri/src/backup.rs) ----

  /** Writes everything Coupler keeps to one file the player names. */
  const exportBackup = useCallback(async () => {
    try {
      const path = await saveFile({ title: "Export a Coupler Backup", defaultPath: backup.defaultName(), filters: [{ name: "Coupler Backup", extensions: [backup.EXTENSION] }] });
      if (!path) return;
      const started = Date.now();
      const made = await runActivity(
        "Making a Coupler Backup…",
        (update) =>
          mud.backupExport(path, backup.settings(), (p) =>
            update({ value: p.total > 0 ? p.done / p.total : undefined, label: `Making a Coupler Backup: ${mud.sizeInWords(p.done)} of ${mud.sizeInWords(p.total)} packed…` }),
          ),
        "backup",
      );
      const m = made.manifest;
      const seconds = Math.max(1, Math.round((Date.now() - started) / 1000));
      showStatus(
        `Backed up ${m.files} ${m.files === 1 ? "file" : "files"} and ${m.settings} settings (${mud.sizeInWords(m.bytes)}) into ${mud.sizeInWords(made.size)}, in ${seconds} ${seconds === 1 ? "second" : "seconds"}: ${path.split(/[\\/]/).pop()}.`,
      );
    } catch (e) {
      setError(String(e));
    }
  }, [runActivity, showStatus]);

  /** Reads a backup's start and asks before restoring it. */
  const offerRestore = useCallback(
    async (path: string) => {
      if (connected) {
        setError("Hang up first, then restore the Coupler Backup: restoring starts Coupler again.");
        return;
      }
      try {
        setRestoreOffer({ path, manifest: await mud.backupInspect(path) });
      } catch (e) {
        setError(String(e));
      }
    },
    [connected],
  );

  /** The keyboard's way to restore: a file chooser. */
  const chooseBackup = useCallback(async () => {
    try {
      const chosen = await openFiles({ multiple: false, title: "Restore a Coupler Backup", filters: [{ name: "Coupler Backup", extensions: [backup.EXTENSION] }] });
      if (typeof chosen === "string") await offerRestore(chosen);
    } catch (e) {
      setError(String(e));
    }
  }, [offerRestore]);

  /** Yes: the backup is unpacked and checked, then Coupler starts again to put it in place. */
  const restoreBackup = useCallback(async () => {
    if (!restoreOffer || restoring) return;
    setRestoring(true);
    try {
      await runActivity(
        "Restoring the Coupler Backup…",
        (update) =>
          mud.backupRestore(restoreOffer.path, (p) =>
            update({ value: p.total > 0 ? p.done / p.total : undefined, label: `Restoring the Coupler Backup: ${mud.sizeInWords(p.done)} of ${mud.sizeInWords(p.total)} unpacked…` }),
          ),
        "backup",
      );
      showStatus("The Coupler Backup is ready. Coupler is starting again to put it in place.");
      if (voiced) voice.speak("Backup restored. Coupler is starting again.", true);
      sound.fadeAll();
      window.setTimeout(() => void mud.appRestart(), 1500);
    } catch (e) {
      setRestoring(false);
      setRestoreOffer(null);
      setError(String(e));
    }
  }, [restoreOffer, restoring, runActivity, showStatus, voiced]);

  // Files dropped anywhere on the window go into the Assets folder; a
  // Coupler Backup dropped is offered for restoring.
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "enter") {
        setError(null);
        const paths = e.payload.paths;
        setStatus(paths.length === 1 && backup.isBackup(paths[0]) ? "Drop to restore this Coupler Backup." : "Drop to add these to Coupler's Assets folder.");
      } else if (e.payload.type === "leave") {
        setStatus(null);
      } else if (e.payload.type === "drop") {
        const backups = e.payload.paths.filter(backup.isBackup);
        if (backups.length === 0) void importAssets(e.payload.paths);
        else if (e.payload.paths.length === 1) {
          setStatus(null);
          void offerRestore(backups[0]);
        } else setError("Drop a Coupler Backup on its own to restore it.");
      }
    });
    return () => void unlisten.then((f) => f());
  }, [importAssets, offerRestore]);

  useEffect(() => {
    const unlisten = mud.onHookFired(onFired);
    // WebKit only lets sound start once the page has been used.
    const wake = () => sound.wakeAudio();
    document.addEventListener("pointerdown", wake, { once: true });
    document.addEventListener("keydown", wake, { once: true });
    return () => {
      void unlisten.then((f) => f());
      document.removeEventListener("pointerdown", wake);
      document.removeEventListener("keydown", wake);
    };
  }, [onFired]);

  const anyDialogOpen = aboutOpen || licensesOpen || keysOpen || clearMapOpen || usualLayoutOpen || hooksOpen || mixerOpen || appTestingOpen || workshopOpen || cuesOpen || echoesOpen || artisanOpen || speechOpen || tutorialOpen || guideOpen || displayOpen || picturesOpen || castOpen || journalOpen || logOpen || shrinkOffer !== null || restoreOffer !== null || musicOpen;

  // With Coupler's voice on, a problem is heard: a low buzz, then the words.
  useEffect(() => {
    if (!error) return;
    if (cues) earcons.problem();
    if (voiced) voice.speak(error, true);
    // Only when a new error comes, not when the voice is turned on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [error]);
  // A walk is heard arriving.
  useEffect(() => {
    if (cues && !walking && status?.startsWith("Arrived")) earcons.arrived();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [walking]);

  // The shortcuts in SHORTCUTS that aren't the input's own.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // The quiet key works over a dialog too: the Journal's Play is in one.
      if (e.metaKey && !e.ctrlKey && !e.shiftKey && !e.altKey && e.key === ".") {
        e.preventDefault();
        voice.hush();
        return;
      }
      if (anyDialogOpen) return;
      const chord = (e.metaKey || e.ctrlKey) && e.shiftKey && !e.altKey;
      if (WEB && chord && DESKTOP_ONLY_KEYS.includes(e.key.toLowerCase())) return;
      const way = e.metaKey && e.ctrlKey && !e.shiftKey && !e.altKey ? { "1": "terminal", "2": "immersive", "3": "workshop" }[e.code.replace("Digit", "")] : undefined;
      if (way) {
        e.preventDefault();
        chooseUx(way as Ux);
      } else if (e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey && ["ArrowUp", "ArrowDown", "Home", "End"].includes(e.key)) {
        e.preventDefault();
        if (e.key === "End") reviewLive();
        else reviewStep(e.key === "ArrowUp" ? "earlier" : e.key === "ArrowDown" ? "later" : "oldest");
      } else if (chord && e.key.toLowerCase() === "v") {
        e.preventDefault();
        sayVitals();
      } else if (chord && e.key.toLowerCase() === "e") {
        e.preventDefault();
        sayOpponent();
      } else if (chord && e.key.toLowerCase() === "w") {
        e.preventDefault();
        sayWho();
      } else if (chord && e.key.toLowerCase() === "o") {
        e.preventDefault();
        sayRecent();
      } else if (chord && e.key.toLowerCase() === "u") {
        e.preventDefault();
        sayUnheard();
      } else if (chord && e.key.toLowerCase() === "n") {
        e.preventDefault();
        playUnheard();
      } else if (chord && e.key.toLowerCase() === "j") {
        e.preventDefault();
        setJournalOpen(true);
      } else if (chord && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setLogOpen(true);
      } else if (chord && e.key.toLowerCase() === "l") {
        e.preventDefault();
        whereAmI();
      } else if (chord && e.key.toLowerCase() === "g") {
        e.preventDefault();
        if (creationStep) setGuideHidden(false);
        else showStatus("No character is being made: the guide opens when you make one.");
      } else if (chord && e.key.toLowerCase() === "m") {
        e.preventDefault();
        toggleMap();
      } else if (chord && e.key.toLowerCase() === "h") {
        e.preventDefault();
        setHooksOpen(true);
      } else if (chord && e.key.toLowerCase() === "p") {
        e.preventDefault();
        paintRoom();
      } else if (chord && e.key.toLowerCase() === "s") {
        e.preventDefault();
        stopEverything();
      } else if (chord && e.key.toLowerCase() === "a") {
        e.preventDefault();
        setArtisanOpen(true);
      } else if (e.metaKey && e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        toggleFullscreen();
      } else if (e.key === "Escape" && !gearOpen && connected) {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [anyDialogOpen, gearOpen, connected, creationStep, showStatus, whereAmI, sayUnheard, playUnheard, sayVitals, sayOpponent, sayWho, sayRecent, reviewStep, reviewLive, chooseUx, toggleMap, toggleFullscreen, stopEverything, paintRoom]);

  useEffect(() => {
    if (!gearOpen) return;
    const onDown = (e: MouseEvent) => {
      if (gearRef.current && !gearRef.current.contains(e.target as Node)) setGearOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setGearOpen(false);
        gearRef.current?.querySelector("button")?.focus();
      }
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [gearOpen]);

  /**
   * Dev-only: is everything showing on the ANSIapps theme's grid? It
   * measures the screen as it is, open menu or dialog included.
   */
  function devGridCheck() {
    if (theme !== "ansiapps" || !stageRef.current) {
      setError("The grid belongs to the ANSIapps theme. Switch to it, then check.");
      return;
    }
    const { checked, off } = checkGrid(stageRef.current);
    setError(off.length > 0 ? `Dev grid check: ${off.length} of ${checked} off the grid, outlined. ${off.slice(0, 4).join("; ")}` : null);
    if (off.length === 0) setStatus(`Dev grid check: all ${checked} texts and controls showing are on the grid.`);
    setGearOpen(false);
  }
  const gridCheckRef = useRef<(() => void) | null>(null);
  // Only in a dev build, so a production build drops the check entirely.
  if (import.meta.env.DEV) gridCheckRef.current = devGridCheck;
  // Its key works with a dialog open, which the gear menu can't reach.
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.ctrlKey && e.key.toLowerCase() === "g") {
        e.preventDefault();
        gridCheckRef.current?.();
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, []);

  const dialing = isBusy("connect");
  const host = server?.host ?? "CoffeeMUD";

  return (
    <div className="stage-frame">
    <div className="app client" ref={stageRef} style={{ transform: `scale(${scale})` }}>
      <StartupScreen
        progress={1 - startupPending.length / STARTUP_STEPS.length}
        label={startupPending.length > 0 ? STARTUP_LABELS[startupPending[0]] : ""}
        done={startupDone}
      />
      {/* The desk: everything on the screen is placed in here, each thing
          in a Movable, where the user left it or in its default place
          (defaultLayout). The order here is the Tab and reading order,
          whatever the arrangement. */}
      <KeptContext.Provider value={ux === "workshop"}>
      <ArrangeContext.Provider value={arranging && ux === "workshop"}>
      {/* Built afresh for each way to play: each has its own layout. */}
      <main className={`desk ux-${ux}${textSize > 1 ? ` text-${textSize}x` : ""}`} key={`${ux}-${textSize}`}>
        {/* The menu bar's strip, behind the things that sit on it. */}
        {has("menu") && (
        <Movable id="menu" label="menu bar" usual={usual.menu} className="menu-bar" backdrop announce={announcePanel}>
        <h1 className="menu-app" aria-label={`Coupler ${__APP_VERSION__}`}>
          <span className="app-mark" aria-hidden="true">
            <AppMarkIcon />
          </span>
          <span aria-hidden="true">
            Coupler<span className="accent">.</span>
          </span>
        </h1>
        </Movable>
        )}
        {/* Combat or Explore mode, while a character is in the game (the
            map knows their room). Said by VoiceOver when it changes. */}
        {has("mode") && connected && (snapshot?.room || opponent) && (
          <Movable id="mode" label="play mode" usual={usual.mode} className="menu-bar menu-chip" announce={announcePanel}>
            <span className={`play-mode${opponent ? " combat" : ""}`} data-testid="play-mode" role="status">
              {opponent ? "Combat mode" : "Explore mode"}
            </span>
          </Movable>
        )}
        {has("state") && (
        <Movable id="state" label="connection state" usual={usual.state} className="menu-bar menu-chip" announce={announcePanel}>
          <span className={`connection-state${connected ? " on" : ""}`} data-testid="connection-state">
            {connected ? `Online: ${port?.name ?? host}` : "Offline"}
          </span>
        </Movable>
        )}
        {has("hangup") && connected && (
          <Movable id="hangup" label="Hang Up button" usual={usual.hangup} className="menu-bar menu-chip" announce={announcePanel}>
            <button type="button" className="menu-title" data-testid="disconnect-button" disabled={dialing} onClick={() => void handleDisconnect()}>
              Hang Up
            </button>
          </Movable>
        )}
        {has("where") && (
        <Movable id="where" label="Where am I? button" usual={usual.where} className="menu-bar menu-chip" announce={announcePanel}>
          <button type="button" className="menu-title" data-testid="where-button" title="Say where you are and the exits (Cmd+Shift+L)" onClick={whereAmI}>
            Where am I?
          </button>
        </Movable>
        )}
        {/* What music is playing, while it plays: it opens the Mixer. */}
        {has("nowPlaying") && nowPlaying && (
          <Movable id="nowPlaying" label="music playing" usual={usual.nowPlaying} className="menu-bar menu-chip" announce={announcePanel}>
            <button type="button" className="menu-title now-playing" data-testid="now-playing" title="The music playing. Click for the Mixer" onClick={() => setMixerOpen(true)}>
              {`♪ ${mud.assetTitle(nowPlaying)}`}
            </button>
          </Movable>
        )}
        {has("mapToggle") && (
        <Movable id="mapToggle" label="Map button" usual={usual.mapToggle} className="menu-bar menu-chip" announce={announcePanel}>
          <button type="button" className={`menu-title${mapOpen ? " open" : ""}`} data-testid="map-toggle" aria-pressed={mapOpen} title="Show or hide the map (Cmd+Shift+M)" onClick={toggleMap}>
            Map
          </button>
        </Movable>
        )}
        {has("mixer") && (
        <Movable id="mixer" label="Mixer button" usual={usual.mixer} className="menu-bar menu-chip" announce={announcePanel}>
          <button type="button" className="menu-title" data-testid="mixer-button" title="The volumes of the sounds and music" onClick={() => setMixerOpen(true)}>
            Mixer
          </button>
        </Movable>
        )}
        {has("bgm") && (
        <Movable id="bgm" label="BGM button" usual={usual.bgm} className="menu-bar menu-chip" announce={announcePanel}>
          <button type="button" className="menu-title" data-testid="bgm-button" title="The Music Editor: the MIDI files in the Assets folder" onClick={() => setMusicOpen(true)}>
            BGM
          </button>
        </Movable>
        )}
        <Movable id="gear" label="gear button" usual={usual.gear} className="menu-bar menu-chip" front={gearOpen} announce={announcePanel}>
          <div className="gear-menu-wrap" ref={gearRef}>
            <button
              type="button"
              className="small icontext-btn gear-btn"
              data-testid="gear-button"
              title="Settings and about"
              aria-label="Settings and about"
              aria-haspopup="true"
              aria-expanded={gearOpen}
              onClick={() => setGearOpen((v) => !v)}
            >
              <GearIcon width={16} height={16} aria-hidden="true" />
            </button>
            <div className={`gear-menu${gearOpen ? " open" : ""}`}>
              {/* The three ways to play (lib/ux.ts). */}
              <div role="radiogroup" aria-label="Way to play" className="menu-group">
                {UXES.map((u) => (
                  <label key={u.id} className="menu-item menu-item-checkbox">
                    <input type="radio" name="ux" data-testid={`ux-${u.id}`} checked={ux === u.id} onChange={() => chooseUx(u.id)} />
                    <span>{`${u.name} (${u.key})`}</span>
                  </label>
                ))}
              </div>
              <div className="menu-sep" />
              <label className="menu-item menu-item-checkbox">
                <input
                  type="checkbox"
                  data-testid="ansiapps-theme-toggle"
                  checked={theme === "ansiapps"}
                  onChange={(e) => setTheme(e.currentTarget.checked ? "ansiapps" : "modern")}
                />
                <span>ANSIapps theme (old-school DOS look)</span>
              </label>
              <label className="menu-item menu-item-checkbox">
                <input type="checkbox" data-testid="fullscreen-toggle" checked={fullscreen} onChange={toggleFullscreen} />
                <span>Full screen (Ctrl+Cmd+F)</span>
              </label>
              <div className="menu-sep" />
              {/* Making a character: Coupler's guide or CoffeeMUD's own way (lib/creation.ts). */}
              <div role="radiogroup" aria-label="Making a character" className="menu-group">
                <label className="menu-item menu-item-checkbox">
                  <input type="radio" name="creation-mode" data-testid="creation-mode-guide" checked={creationMode === "guide"} onChange={() => chooseCreationMode("guide")} />
                  <span>Make characters with Coupler's guide</span>
                </label>
                <label className="menu-item menu-item-checkbox">
                  <input type="radio" name="creation-mode" data-testid="creation-mode-standard" checked={creationMode === "standard"} onChange={() => chooseCreationMode("standard")} />
                  <span>Make characters CoffeeMUD's way</span>
                </label>
              </div>
              {ux === "workshop" && (
                <>
                  <label className="menu-item menu-item-checkbox">
                    <input type="checkbox" data-testid="arrange-toggle" checked={arranging} onChange={toggleArranging} />
                    <span>Move and resize everything</span>
                  </label>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="workshop-button"
                    onClick={() => {
                      setGearOpen(false);
                      setWorkshopOpen(true);
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Customize the Screen…</span>
                  </button>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="cues-button"
                    onClick={() => {
                      setGearOpen(false);
                      setCuesOpen(true);
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Cues…</span>
                  </button>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="echoes-button"
                    onClick={() => {
                      setGearOpen(false);
                      setEchoesOpen(true);
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Narrator's Answers…</span>
                  </button>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="usual-layout-button"
                    onClick={() => {
                      setGearOpen(false);
                      setUsualLayoutOpen(true);
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Reset to Default Layout</span>
                  </button>
                </>
              )}
              <div className="menu-sep" />
              {!connected && (
                <button
                  type="button"
                  className="menu-item"
                  data-testid="tutorial-menu"
                  onClick={() => {
                    setGearOpen(false);
                    setTutorialOpen(true);
                  }}
                >
                  <ChecklistIcon aria-hidden="true" />
                  <span>Before You Play: a Tutorial…</span>
                </button>
              )}
              <button
                type="button"
                className="menu-item"
                data-testid="speech-button"
                onClick={() => {
                  setGearOpen(false);
                  setSpeechOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Speech…</span>
              </button>
              <button
                type="button"
                className="menu-item"
                data-testid="display-button"
                onClick={() => {
                  setGearOpen(false);
                  setDisplayOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Display…</span>
              </button>
              <button
                type="button"
                className="menu-item"
                data-testid="artisan-button"
                onClick={() => {
                  setGearOpen(false);
                  setArtisanOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Artisan Skills and Mentor (Cmd+Shift+A)…</span>
              </button>
              {!WEB && (
              <button
                type="button"
                className="menu-item"
                data-testid="pictures-button"
                onClick={() => {
                  setGearOpen(false);
                  setPicturesOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Pictures…</span>
              </button>
              )}
              {!WEB && (
              <button
                type="button"
                className="menu-item"
                data-testid="journal-menu"
                onClick={() => {
                  setGearOpen(false);
                  setJournalOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Journal… (Cmd+Shift+J)</span>
              </button>
              )}
              {!WEB && (
              <button
                type="button"
                className="menu-item"
                data-testid="log-menu"
                onClick={() => {
                  setGearOpen(false);
                  setLogOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Log… (Cmd+Shift+K)</span>
              </button>
              )}
              {!WEB && (
              <button
                type="button"
                className="menu-item"
                data-testid="cast-button"
                onClick={() => {
                  setGearOpen(false);
                  setCastOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Characters' Voices…</span>
              </button>
              )}
              {!WEB && (
                <>
                  <div className="menu-sep" />
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="backup-export"
                    onClick={() => {
                      setGearOpen(false);
                      void exportBackup();
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Export Coupler Backup…</span>
                  </button>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="backup-restore"
                    onClick={() => {
                      setGearOpen(false);
                      void chooseBackup();
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Restore Coupler Backup…</span>
                  </button>
                  <div className="menu-sep" />
                  {/* A newer Coupler (lib/updates.ts): asked of GitHub only by the button or once turned on. */}
                  <label className="menu-item menu-item-checkbox">
                    <input type="checkbox" data-testid="auto-updates-toggle" checked={autoUpdates} onChange={(e) => toggleAutoUpdates(e.currentTarget.checked)} />
                    <span>Check for updates automatically</span>
                  </label>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="update-check"
                    disabled={isBusy("update")}
                    onClick={() => {
                      setGearOpen(false);
                      void checkUpdates();
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Check for Updates Now</span>
                  </button>
                  {newRelease && (
                    <button
                      type="button"
                      className="menu-item"
                      data-testid="update-get"
                      onClick={() => {
                        setGearOpen(false);
                        updates
                          .openPage()
                          .then(() => setStatus(`Coupler ${newRelease}'s page is open in your browser.`))
                          .catch((e) => setError(String(e)));
                      }}
                    >
                      <ChecklistIcon aria-hidden="true" />
                      <span>{`Get Coupler ${newRelease}… (opens the browser)`}</span>
                    </button>
                  )}
                  <div className="menu-sep" />
                </>
              )}
              <button
                type="button"
                className="menu-item"
                data-testid="keys-button"
                onClick={() => {
                  setGearOpen(false);
                  setKeysOpen(true);
                }}
              >
                <ChecklistIcon aria-hidden="true" />
                <span>Keyboard</span>
              </button>
              <button
                type="button"
                className="menu-item"
                onClick={() => {
                  setGearOpen(false);
                  setAboutOpen(true);
                }}
              >
                <InfoIcon aria-hidden="true" />
                <span>About Coupler</span>
              </button>
              {import.meta.env.DEV && (
                <>
                  <div className="menu-sep" />
                  <button
                    type="button"
                    className="menu-item"
                    onClick={() => {
                      setGearOpen(false);
                      setAppTestingOpen(true);
                    }}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>App Testing</span>
                  </button>
                  <button
                    type="button"
                    className="menu-item"
                    data-testid="grid-check"
                    onClick={devGridCheck}
                  >
                    <ChecklistIcon aria-hidden="true" />
                    <span>Check the Grid (Ctrl+Cmd+G)</span>
                  </button>
                  <button type="button" className="menu-item" data-testid="copy-layout" onClick={devCopyLayout}>
                    <ChecklistIcon aria-hidden="true" />
                    <span>Copy the Layout</span>
                  </button>
                </>
              )}
            </div>
          </div>
        </Movable>

        {/* What the screen reader reads of the game, by kind (lib/speech.ts):
            the output itself isn't a live region, so it's never read twice. */}
        <div className="sr-only" role="log" aria-live="polite" aria-relevant="additions" aria-label="Game output, as read" data-testid="heard-live">
          {heard.map((h) => (
            <div key={h.id}>{h.text}</div>
          ))}
        </div>

        {/* The game output comes before the ways to play so they, which
            share its place until one is moved, are in front of it. */}
        {showTerminal && (
          <Movable id="terminal" label="game output" usual={usual.terminal} resizable={false} tier={1} announce={announcePanel}>
            <Terminal
              lines={lines}
              partial={partial}
              onSize={onSize}
              rows={ux === "terminal" ? terminalRows(theme, connected, textSize) : 25}
              colors={display.colors}
              hide={hide}
              reviewing={reviewId}
              empty={<>Connected. Waiting for CoffeeMUD to answer…</>}
            />
          </Movable>
        )}
        {/* Back in front each time it comes back (after a disconnect), even
            if the game output was brought in front of it since. */}
        {!connected && (
          <Movable id="ports" label="ways to play" usual={usual.ports} least={PANEL_LEAST} tier={1} frontOnShow announce={announcePanel}>
            <section className="ports panel" aria-labelledby="ports-title" data-testid="ports">
              <h2 id="ports-title">Choose how to play</h2>
              {/* The frame doesn't scroll (its title sits in the top edge); this does. */}
              <div className="ports-body">
              <p className="desc">
                CoffeeMUD runs several games side by side at {host}. Each button connects to one. New players: start with Standard, and create a character
                once connected.
              </p>
              <div className="tutorial-offer">
                <button
                  type="button"
                  data-testid="tutorial-button"
                  aria-describedby="tutorial-offer-desc"
                  onFocus={() => {
                    if (voiced) voice.speak("Before You Play. A short practice game that teaches Immersive, before the real one.", true);
                  }}
                  onClick={() => setTutorialOpen(true)}
                >
                  Before You Play
                </button>
                <p id="tutorial-offer-desc">New to Immersive? A short practice game in Midgaard teaches its sounds and keys. Nothing connects.</p>
              </div>
              <ul className="port-list">
                {orderedPorts.map((p, i) => (
                  <li key={p.id}>
                    <button
                      type="button"
                      ref={i === 0 ? firstPortRef : undefined}
                      className={i === 0 ? "primary" : undefined}
                      data-testid={`connect-${p.id}`}
                      aria-describedby={`port-${p.id}`}
                      disabled={dialing}
                      onFocus={() => {
                        if (voiced) voice.speak(`${p.name}. ${p === port ? "Played last time. " : ""}${p.summary}`, true);
                      }}
                      onClick={() => void handleConnect(p)}
                    >
                      {p.name}
                    </button>
                    <p id={`port-${p.id}`}>
                      {p === port && <span className="port-last">Played last time. </span>}
                      {p.summary} <span className="port-number">Port {p.port}.</span>
                    </p>
                  </li>
                ))}
              </ul>
              </div>
            </section>
          </Movable>
        )}

        <Movable id="command" tier={2} label="command line" usual={usual.command} announce={announcePanel}>
          <div className="input-row">
            <input
              ref={inputRef}
              type={secret ? "password" : "text"}
              data-testid="command-input"
              aria-label={secret ? "Password" : "Command"}
              aria-describedby="input-hint"
              placeholder={connected ? (secret ? "Password (hidden, not kept)" : "Type a command and press Return") : "Choose a way to play to start"}
              value={input}
              disabled={!connected}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck={false}
              onChange={(e) => setInput(e.currentTarget.value)}
              onKeyDown={handleKey}
            />
          </div>
        </Movable>
        {has("hint") && (
        <Movable id="hint" tier={2} label="command line's hint" usual={usual.hint} announce={announcePanel}>
          <div className="input-hint" id="input-hint">
            {!connected || secret
              ? ""
              : ux === "immersive"
                ? "Cmd+Shift+L where you are. Cmd+Shift+V health. Cmd+Shift+E the fight. Cmd+Shift+O what the game said. Cmd+Period quiet."
                : "Up and Down go through what you've typed. Esc comes back here from anywhere."}
          </div>
        </Movable>
        )}

        {has("scene") && (ux !== "immersive" || connected) && (
          <Movable id="scene" label="Here panel" usual={usual.scene} least={PANEL_LEAST} tier={1} announce={announcePanel}>
            <ScenePanel
              snapshot={snapshot}
              vitals={vitals}
              opponent={opponent}
              music={nowPlaying}
              cues={cues}
              voice={voiced}
              picture={!WEB && ux === "immersive" && pictureSettings.engine !== "none" && pictureScene ? { scene: pictureScene, settings: pictureSettings, shows: pictureOn } : null}
            />
          </Movable>
        )}
        {has("heard") && (
          <Movable id="heard" label="Heard panel" usual={usual.heard} least={PANEL_LEAST} tier={1} announce={announcePanel}>
            <HeardPanel captions={display.captions} />
          </Movable>
        )}

        {ux === "workshop" && has("vitals") && connected && (
          <Movable id="vitals" label="You panel" usual={usual.vitals} least={VITALS_LEAST} tier={1} announce={announcePanel}>
            <VitalsPanel vitals={vitals} opponent={opponent} />
          </Movable>
        )}

        {!WEB && ux === "workshop" && has("picture") && (
          <Movable id="picture" label="Picture panel" usual={usual.picture} least={PANEL_LEAST} tier={1} announce={announcePanel}>
            <PicturePanel scene={pictureScene} settings={pictureSettings} shows={pictureOn} />
          </Movable>
        )}

        {ux === "workshop" && mapOpen && (
          <Movable id="map" label="map" usual={usual.map} least={PANEL_LEAST} tier={1} announce={announcePanel}>
            <MapPanel snapshot={snapshot} connected={connected} walking={walking} run={runMap} onAskClear={() => setClearMapOpen(true)} />
          </Movable>
        )}

        {/* The message bar: always here, always this size, so nothing moves
            when a message comes or goes. A message too long for its two
            lines is cut short on screen; hovering shows all of it, and a
            screen reader reads all of it. */}
        {/* The triggers' pictures (the hooks list sets them), each at its
            column and row: in front of the panels, the newest in front of
            the others, behind the bars and controls. Clicking one closes
            it, one with a fade fades away on its own, Cmd+Shift+S closes
            them all, and so does leaving the game. */}
        {pictures.map((picture) => {
          const place = { left: (picture.x - 1) * 8, top: (picture.y - 1) * 16, maxWidth: 1280 - (picture.x - 1) * 8, maxHeight: 720 - (picture.y - 1) * 16, zIndex: PICTURE_Z };
          const name = mud.assetTitle(picture.path);
          const close = () => setPictures((list) => list.filter((p) => p.id !== picture.id));
          return <HookPicture key={picture.id} picture={picture} name={name} style={place} fade={picture.fade} shownAt={picture.shownAt} onClose={close} />;
        })}

        {/* Immersive: the line of talk being said, while it's said. */}
        {ux === "immersive" && (
          <div style={{ position: "absolute", left: 0, top: 0, zIndex: PICTURE_Z }}>
            <SpeakingPopup />
          </div>
        )}

        <Movable id="messages" tier={2} label="message bar" usual={usual.messages} announce={announcePanel}>
          <div className="status-bar">
        <div className="status-bar-message">
          <ActivityStatus activities={activities} />
          {error && (
            <p className="error" role="alert" title={error}>
              {error}
            </p>
          )}
          {status && !error && (
            <p className="status-message" role="status" data-testid="status-message" title={status}>
              {status}
            </p>
          )}
        </div>
          </div>
        </Movable>
        {(["journal", "log"] as const).map((book) => {
          const n = book === "journal" ? journal.unheard.journal : journal.unheard.log;
          const name = book === "journal" ? "Journal" : "Log";
          return (
            has(book) && (
              <Movable key={book} id={book} label={`${name} button`} usual={usual[book]} announce={announcePanel}>
                {/* The count is padded and the bullet keeps its cell, so the button never changes width. */}
                <button
                  type="button"
                  className="small journal-button"
                  data-testid={`${book}-button`}
                  aria-label={`${name}: ${n === 0 ? "all heard" : `${n} not heard yet`}. Show it`}
                  title={`${book === "journal" ? "What's said in the game" : "OOC, INFO and the other channels"}; a green bullet when some isn't heard yet (Cmd+Shift+${book === "journal" ? "J" : "K"})`}
                  onClick={() => (book === "journal" ? setJournalOpen(true) : setLogOpen(true))}
                >
                  {`${name.padEnd(7)} ${String(Math.min(n, 9999)).padStart(4)} `}
                  <span className="asset-dot">{n > 0 ? "•" : " "}</span>
                </button>
              </Movable>
            )
          );
        })}
        {has("hooks") && (
        <Movable id="hooks" label="Hooks button" usual={usual.hooks} announce={announcePanel}>
        {/* The count is padded to a fixed width, so the button keeps its place as it grows. */}
        <button
          type="button"
          className="small hooks-button"
          data-testid="hooks-button"
          aria-label={`Hooks: ${hooksCount}. Show the list`}
          title="What the game has sent behind its text, each name and value once. Click for the list (Cmd+Shift+H)"
          onClick={() => setHooksOpen(true)}
        >
          {`Hooks ${String(hooksCount).padStart(6)}`}
        </button>
        </Movable>
        )}
        {has("readout") && (
        <Movable id="readout" label="screen readout" usual={usual.readout} announce={announcePanel}>
          <ScreenReadout stage={stageRef} />
        </Movable>
        )}
      </main>
      </ArrangeContext.Provider>
      </KeptContext.Provider>

      <Dialog
        open={aboutOpen}
        onClose={() => setAboutOpen(false)}
        title="About Coupler"
        covered={licensesOpen}
        actions={
          <>
            <button type="button" data-testid="licenses-button" onClick={() => setLicensesOpen(true)}>
              Open-Source Licenses
            </button>
            <button type="button" className="primary" onClick={() => setAboutOpen(false)}>
              Close
            </button>
          </>
        }
      >
        <p>
          Coupler {__APP_VERSION__} is a client made for one game: CoffeeMUD. It connects to {host} and nowhere else, and keeps nothing about you or your
          characters anywhere but this computer. The map it draws as you explore is kept on this computer too.
        </p>
        <p>Coupler is free and open source, under the Apache License 2.0.</p>
        <h3 className="about-section-title">Credits</h3>
        <p className="about-section-desc" data-testid="about-credits">
          CoffeeMUD is by Bo Zimmerman and its contributors (coffeemud.net). Coupler's handling of the game's protocols follows Sip, CoffeeMUD's own client by
          Bo Zimmerman (Apache License 2.0). Coupler paints its own race and class portraits; CoffeeMUD's own, made into ANSI art, show instead when chosen in Pictures, as
          CoffeeMUD is distributed (Apache License 2.0).
        </p>
        <p className="about-section-desc">
          The ANSIapps theme's font is IBM VGA 8x16 from The Ultimate Oldschool PC Font Pack by VileR (int10h.org/oldschool-pc-fonts), licensed under CC BY-SA 4.0 and
          included unmodified.
        </p>
        <p className="about-section-desc">
          Opus and WavPack sounds are played and made with libopus (Xiph.Org Foundation and contributors, BSD licence), WavPack (David Bryant, BSD licence), and
          the Rust ogg and rubato libraries (BSD and MIT licences). The built-in Flite voice is flite-rs (Apache License 2.0) with Carnegie Mellon University's
          voice data. The Pocket TTS voices are Kyutai's model (CC BY 4.0) with voices from the VCTK corpus (University of Edinburgh), Alba MacKenna and
          LibriVox readers. Open-Source Licenses has every library's license and credit in full.
        </p>
      </Dialog>

      <LicensesDialog open={licensesOpen} onClose={() => setLicensesOpen(false)} />

      <Dialog open={keysOpen} onClose={() => setKeysOpen(false)} title="Keyboard">
        <p>Everything in Coupler can be done from the keyboard. Tab moves between controls; these work from anywhere in the window.</p>
        <table className="keys-table" data-testid="keys-table">
          <tbody>
            {SHORTCUTS.map(([what, key]) => (
              <tr key={what}>
                <th scope="row">{what}</th>
                <td>{key}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Dialog>

      <Dialog
        open={clearMapOpen}
        onClose={() => setClearMapOpen(false)}
        title="Start the map over?"
        actions={
          <>
            <button type="button" onClick={() => setClearMapOpen(false)}>
              Keep the Map
            </button>
            <button
              type="button"
              className="danger-fill"
              data-testid="map-clear-confirm"
              onClick={() => {
                setClearMapOpen(false);
                runMap("Forgetting the map…", async () => {
                  await mud.mapClear();
                  return "The map was started over.";
                });
              }}
            >
              Forget Every Room
            </button>
          </>
        }
      >
        <p>
          Coupler will forget every room and landmark it has mapped for this game{snapshot ? ` (${snapshot.roomsKnown} rooms)` : ""}. This can't be undone. The map
          builds again as you explore.
        </p>
      </Dialog>

      <Dialog
        open={usualLayoutOpen}
        onClose={() => setUsualLayoutOpen(false)}
        title="Reset to the default layout?"
        actions={
          <>
            <button type="button" onClick={() => setUsualLayoutOpen(false)}>
              Keep My Layout
            </button>
            <button
              type="button"
              className="danger-fill"
              data-testid="usual-layout-confirm"
              onClick={() => {
                setUsualLayoutOpen(false);
                forgetLayout();
                showStatus("Everything is back in its default place and size.");
              }}
            >
              Reset Layout
            </button>
          </>
        }
      >
        <p>Everything you've moved or resized on the screen goes back to its default place and size. This can't be undone.</p>
      </Dialog>

      <WorkshopDialog
        open={workshopOpen}
        onClose={() => setWorkshopOpen(false)}
        arranging={arranging}
        onArranging={toggleArranging}
        shown={shown}
        onShown={changeShown}
        mapOpen={mapOpen}
        onMap={toggleMap}
        layers={layers}
        onLayers={changeLayers}
        onReset={() => {
          setWorkshopOpen(false);
          setUsualLayoutOpen(true);
        }}
        onCues={() => setCuesOpen(true)}
        covered={cuesOpen}
      />

      <CuesDialog open={cuesOpen} onClose={() => setCuesOpen(false)} onStatus={showStatus} />
      <EchoesDialog open={echoesOpen} onClose={() => setEchoesOpen(false)} onStatus={showStatus} />
      <ArtisanDialog open={artisanOpen} onClose={() => setArtisanOpen(false)} voiced={voiced} />

      <CreationDialog open={guideOpen} step={creationStep} voiced={voiced} cues={cues} portraits={pictureSettings.portraits} onSend={sendAnswer} onClose={hideGuide} onStandard={() => chooseCreationMode("standard")} />

      <TutorialDialog
        open={tutorialOpen}
        onClose={() => setTutorialOpen(false)}
        onPlayImmersive={() => {
          setTutorialOpen(false);
          // Once the tutorial's closed and hushed its voice, so the intro is heard.
          window.setTimeout(() => chooseUx("immersive"), 0);
        }}
      />

      <SpeechDialog
        open={speechOpen}
        onClose={() => setSpeechOpen(false)}
        spoken={spoken}
        onSpoken={(next) => {
          setSpoken(next);
          saveSpoken(next);
        }}
      />

      <DisplayDialog
        open={displayOpen}
        onClose={() => setDisplayOpen(false)}
        display={display}
        onDisplay={(next) => {
          setDisplay(next);
          saveDisplay(next);
        }}
      />

      {!WEB && (
      <PicturesDialog
        open={picturesOpen}
        onClose={() => setPicturesOpen(false)}
        settings={pictureSettings}
        onSettings={(next) => {
          setPictureSettings(next);
          savePictures(next);
        }}
        onForgetLooks={() => {
          mud
            .pictureForgetLooks()
            .then(() => showStatus("Every room is back to its first look."))
            .catch((e) => setError(String(e)));
        }}
      />
      )}

      {!WEB && (
      <JournalDialog book="journal" open={journalOpen} onClose={() => setJournalOpen(false)} unheard={journal.unheard} onPlay={journal.play} onStatus={showStatus} onError={showError} />
      )}
      {!WEB && (
      <JournalDialog book="log" open={logOpen} onClose={() => setLogOpen(false)} unheard={journal.unheard} onPlay={journal.play} onStatus={showStatus} onError={showError} />
      )}

      {!WEB && (
      <CastDialog open={castOpen} onClose={() => setCastOpen(false)} onStatus={showStatus} onError={showError} />
      )}

      {!WEB && (
      <ShrinkDialog offers={shrinkOffer} onKeep={keepWavs} onShrink={() => void makeSmaller()} />
      )}

      {!WEB && (
      <RestoreDialog offer={restoreOffer} busy={restoring} onCancel={() => !restoring && setRestoreOffer(null)} onRestore={() => void restoreBackup()} />
      )}

      <MixerDialog open={mixerOpen} onClose={() => setMixerOpen(false)} playing={nowPlaying} onStatus={showStatus} />

      {!WEB && (
      <MusicEditorDialog
        open={musicOpen}
        onClose={() => setMusicOpen(false)}
        assetsVersion={assetsVersion}
        onSaved={() => {
          assetsChangedHere();
          setAssetsVersion((v) => v + 1);
        }}
        onStatus={showStatus}
        onError={showError}
      />
      )}

      {!WEB && (
      <HooksDialog
        open={hooksOpen}
        onClose={closeHooks}
        count={hooksCount}
        assetsVersion={assetsVersion}
        roomId={snapshot?.room?.id ?? ""}
        onAddAssets={chooseAssets}
        onAssetsChanged={assetsChangedHere}
        runActivity={runActivity}
        onStatus={showStatus}
        onError={showError}
      />
      )}

      {import.meta.env.DEV && <AppTesting open={appTestingOpen} onClose={() => setAppTestingOpen(false)} />}
    </div>
    </div>
  );
}

export default App;

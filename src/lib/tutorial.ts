/**
 * **Before You Play**: the tutorial (components/TutorialDialog.tsx), a
 * short practice game that teaches Immersive before the real one. It
 * never connects: the game is simulated here, a few rooms of CoffeeMUD's
 * Midgaard (where its players start), and every sound is Immersive's own
 * (lib/immersive.ts's `arrivalCues`, `bodyCues` and `fightCues`, the
 * same `describe*` sentences the say keys use), so what's learned here
 * is what the game will sound like.
 *
 * The game's lines are CoffeeMUD's own wherever its source says them
 * (reference/CoffeeMud): the default prompt (`DefaultPlayerStats`), the
 * starting health, mana and movement (`coffeemud.ini`'s STARTHP and
 * friends), the damage words and the dusk line (`lists.ini`), "is
 * DEAD!!!" and "You gain … experience points." (`MUDFight`,
 * `CoffeeLevels`), says, tells and channels (`Say`, `CommonMsgs`,
 * `CMChannels`), "Huh?!", "You can't go that way." and "I don't see …
 * here.", and the sit and stand answers (src-tauri/src/echo.rs). The
 * rooms are Midgaard's, with the IDs `room.info` gives them
 * (docs/coffeemud-gmcp.md: the Temple of Mota is Midgaard#3001); their
 * descriptions are the classic Midgaard text, CoffeeMUD's copy not being
 * in the source snapshot, so check them against the live game. Aldric,
 * Morgana and Brin are made up.
 *
 * Each lesson tells, plays a scene, then asks for one thing: a key or a
 * command. The narrator (lib/voice.ts) teaches in the player's narrator
 * voice; the lesson's words are written too, and every sound's caption
 * and every line the voice says for the game go in the dialog's Heard
 * list, so it works with eyes, ears or both.
 */
import * as earcons from "./earcons";
import * as echoes from "./echoes";
import { arrivalCues, bodyCues, fightCues, fightOf, NO_FIGHT, type Fight } from "./immersive";
import { FightTalk } from "./fightTalk";
import * as mud from "./mud";
import * as voice from "./voice";

// ---- The game's lines ----

const fg = (index: number): mud.Color => ({ kind: "index", index });
/** A line in one of the 16 colors (7 is the game's light gray). */
const plain = (text: string, index = 7): mud.Line => [{ text, fg: fg(index) }];

/** The default prompt, `^N%E<^h%hhp ^m%mm ^v%vmv^N>`. */
function promptLine(v: mud.Vitals): mud.Line {
  return [
    { text: "<" },
    { text: `${v.hp}hp`, fg: fg(10) },
    { text: " " },
    { text: `${v.mana}m`, fg: fg(13) },
    { text: " " },
    { text: `${v.moves}mv`, fg: fg(14) },
    { text: ">" },
  ];
}

// ---- Midgaard ----

const ZONE = "Midgaard";

interface Place {
  id: string;
  name: string;
  desc: string;
  terrain: string;
  exits: { dir: string; to: string; door?: boolean }[];
  /** A landmark's name, as the player would give it on the map. */
  landmark?: string;
}

const PLACES: Record<string, Place> = {
  "Midgaard#3001": {
    id: "Midgaard#3001",
    name: "The Temple of Mota",
    terrain: "stone",
    desc: "You are in the southern end of the temple hall in the Temple of Mota. The temple has been constructed from giant marble blocks, eternal in appearance, and most of the walls are covered by ancient wall paintings picturing gods, giants and peasants. Large steps lead down through the grand temple gate, descending the huge mound upon which the temple is built and ends on the temple square below.",
    exits: [
      { dir: "N", to: "Midgaard#3054" },
      { dir: "D", to: "Midgaard#3005" },
    ],
  },
  "Midgaard#3054": {
    id: "Midgaard#3054",
    name: "By the Temple Altar",
    terrain: "stone",
    desc: "You are by the temple altar in the northern end of the Temple of Mota. A huge altar made from white polished marble is standing in front of you and behind it is a ten foot tall sitting statue of Mota, the King of the Gods.",
    exits: [{ dir: "S", to: "Midgaard#3001" }],
  },
  "Midgaard#3005": {
    id: "Midgaard#3005",
    name: "The Temple Square",
    terrain: "city",
    desc: "You are standing on the temple square. Huge marble steps lead up to the temple gate. The entrance to the Clerics' Guild is to the west, and the old Grunting Boar Inn is to the east. Just south of here you see the market square, the center of Midgaard.",
    exits: [
      { dir: "E", to: "Midgaard#3006" },
      { dir: "S", to: "Midgaard#3014" },
      { dir: "W", to: "Midgaard#3004", door: true },
      { dir: "U", to: "Midgaard#3001" },
    ],
  },
  "Midgaard#3014": {
    id: "Midgaard#3014",
    name: "Market Square",
    terrain: "city",
    landmark: "the market",
    desc: "You are standing on the market square, the famous Square of Midgaard. A large, peculiar looking statue is standing in the middle of the square. Roads lead in every direction, north to the temple square, south to the common square, east and west to the main street.",
    exits: [
      { dir: "N", to: "Midgaard#3005" },
      { dir: "E", to: "Midgaard#3015" },
      { dir: "S", to: "Midgaard#3025" },
      { dir: "W", to: "Midgaard#3013" },
    ],
  },
};

const DIR_NAMES: Record<string, string> = {
  N: "north", NE: "northeast", E: "east", SE: "southeast", S: "south", SW: "southwest", W: "west", NW: "northwest", U: "up", D: "down",
};

/** The movement commands and their abbreviations, as the game takes them. */
const MOVES: Record<string, string> = {
  n: "N", north: "N", e: "E", east: "E", s: "S", south: "S", w: "W", west: "W", u: "U", up: "U", d: "D", down: "D",
  ne: "NE", northeast: "NE", se: "SE", southeast: "SE", sw: "SW", southwest: "SW", nw: "NW", northwest: "NW",
};

/** Where the practice game's start: a new character, at full health. */
const START: mud.Vitals = { hp: 20, maxHp: 20, mana: 100, maxMana: 100, moves: 100, maxMoves: 100 };

/** The fido, Midgaard's oldest foe. */
const FIDO = { name: "the beastly fido", health: 24 };

/** Made-up voices for the made-up players, as the cast would make them. */
const ALDRIC: mud.Voice = { gender: "masculine", pitch: 92, rate: 100, engine: null, voiceName: null, seed: 1093, quiet: false };
const MORGANA: mud.Voice = { gender: "feminine", pitch: 108, rate: 104, engine: null, voiceName: null, seed: 5521, quiet: false };

// ---- The stage ----

/** What the player did: a key Coupler answers, or a command typed. */
export type TutorialKey = "where" | "vitals" | "enemy" | "hush" | "review";
export type Act = { key: TutorialKey } | { command: string };

export interface StageHooks {
  /** Lines for the practice game's output. */
  write: (lines: mud.Line[]) => void;
  /** A line Coupler's voice says for the game (talk, the time, an answer), for the Heard list. */
  heard: (text: string) => void;
  /** The lesson's task is done. */
  done: () => void;
}

/** One step of a scene: the narrator says something (waited for), something happens, then a pause. */
export interface Step {
  say?: string;
  then?: () => void;
  /** Milliseconds after it. */
  wait?: number;
}

/**
 * The practice game: where the player is, what they've seen, their body
 * and the fight. It writes the game's lines, plays Immersive's cues for
 * what changes, and answers the say keys as the game would.
 */
export class Stage {
  here = "Midgaard#3001";
  visited = new Set<string>(["Midgaard#3001"]);
  vitals: mud.Vitals = { ...START };
  opponent: mud.Opponent | null = null;
  sitting = false;
  /** Whether the fido's in the room. */
  fido = false;
  /** Whether the player has sat down (the narrator lesson). */
  sat = false;
  /** Option+Up presses (the review lesson). */
  reviews = 0;
  private fight: Fight = NO_FIGHT;
  private timers: number[] = [];
  /** The fight told as the game tells it, waiting behind the lesson rather than cutting it (lib/fightTalk.ts). */
  private fightTalk = new FightTalk(false);
  /** Bumped by `stop`: a scene's steps from before stop there. */
  private run = 0;

  constructor(private hooks: StageHooks) {}

  /** The room as the map would have it, its exits marked visited or not. */
  room(id = this.here): mud.MapRoom {
    const p = PLACES[id];
    return {
      id: p.id,
      name: p.name,
      zone: ZONE,
      terrain: p.terrain,
      travel: "",
      landmark: p.landmark ?? "",
      exits: p.exits.map((e) => {
        const visited = this.visited.has(e.to);
        return {
          dir: e.dir,
          dirName: DIR_NAMES[e.dir],
          to: e.to,
          toName: visited ? PLACES[e.to]?.name ?? "" : "",
          visited,
          door: Boolean(e.door),
          open: false,
          locked: false,
        };
      }),
    };
  }

  write(...lines: mud.Line[]) {
    this.hooks.write(lines);
    lines.forEach((line) => this.fightTalk.heard(line.map((span) => span.text).join("")));
  }

  /** The room as the game writes it: its name, description, brief exits, who's here, the prompt. */
  look() {
    const p = PLACES[this.here];
    const exits = p.exits.map((e) => e.dir).join(" ");
    this.write(
      plain(p.name, 14),
      plain(p.desc),
      [{ text: "[Exits: ", fg: fg(10) }, { text: `${exits} `, fg: fg(10) }, { text: "]", fg: fg(10) }],
      ...(this.fido ? [plain("A beastly fido is mucking through the garbage looking for food.", 13)] : []),
    );
    this.prompt();
  }

  prompt() {
    this.write(promptLine(this.vitals));
  }

  /** Walks to `id`: the room written, and the cues Immersive plays for it. */
  go(id: string) {
    const before = { id: this.here, zone: ZONE, known: this.visited.size };
    this.here = id;
    this.visited.add(id);
    this.look();
    arrivalCues(this.room(), before, this.visited.size);
  }

  /** Plays the room's exits as if just arrived (the first room has no footstep). */
  arrive() {
    arrivalCues(this.room(), { id: null, zone: null, known: this.visited.size }, this.visited.size);
  }

  setVitals(change: Partial<mud.Vitals>) {
    const before = this.vitals;
    this.vitals = { ...before, ...change };
    earcons.heartbeat(mud.percentOf(this.vitals.hp, this.vitals.maxHp));
    bodyCues(before, this.vitals);
    this.fightTalk.setVitals(this.vitals);
  }

  setOpponent(opponent: mud.Opponent | null) {
    const before = this.fight;
    this.opponent = opponent;
    this.fightTalk.setOpponent(opponent);
    this.fight = fightOf(opponent);
    fightCues(before, opponent);
  }

  /** The narrator, after what's queued; `then` once it's said (or cut off). */
  say(text: string, then?: () => void) {
    const run = this.run;
    voice.speak(text, false, null, { onDone: () => run === this.run && then?.() });
  }

  /** A key's answer: at once, cutting off anything being said, as the say keys do. */
  answer(text: string) {
    voice.speak(text, true);
  }

  /** Talk as Immersive says it: a ping first for a tell, the narrator says who, the speaker says the words. Not written. */
  talk(text: string, how: mud.Voice, tell = false) {
    if (tell) earcons.tell();
    this.hooks.heard(text);
    voice.speakTalk(text, how);
  }

  /**
   * A command that can't be done: written, with the problem buzz, and
   * said by the narrator at once, as Immersive says a lone one-line answer.
   */
  refuse(text: string) {
    this.write(plain(text));
    earcons.problem();
    this.hooks.heard(text);
    voice.speak(text, true);
  }

  /** A line the narrator says for the game, instead of its being written. */
  narrate(text: string) {
    this.hooks.heard(text);
    voice.speak(text);
  }

  later(ms: number, f: () => void) {
    const run = this.run;
    this.timers.push(
      window.setTimeout(() => {
        if (run === this.run) f();
      }, ms),
    );
  }

  /** A scene's steps, one after another. */
  steps(list: Step[], then?: () => void) {
    const [first, ...rest] = list;
    if (!first) return then?.();
    const next = () => {
      first.then?.();
      this.later(first.wait ?? 0, () => this.steps(rest, then));
    };
    if (first.say) this.say(first.say, next);
    else next();
  }

  done() {
    this.hooks.done();
  }

  /** Ends any fight without a sound: a lesson left mid-fight doesn't carry it into the next. */
  calm() {
    this.fightTalk.stop();
    this.opponent = null;
    this.fight = NO_FIGHT;
  }

  /** Stops every scene and timer, and the heartbeat. */
  stop() {
    this.run++;
    this.timers.forEach((t) => window.clearTimeout(t));
    this.timers = [];
    this.fightTalk.stop();
    earcons.heartbeat(null);
  }

  /** The say keys, answered as the game's are (App.tsx's whereAmI, sayVitals, sayOpponent). */
  key(k: TutorialKey) {
    if (k === "where") this.answer(mud.describeRoom(this.room()));
    else if (k === "vitals") this.answer(mud.describeVitals(this.vitals));
    else if (k === "enemy") this.answer(mud.describeOpponent(this.opponent));
  }

  /** The commands every lesson knows: moving, looking, saying, sitting and standing. */
  command(text: string) {
    const typed = text.trim();
    const [word = "", ...rest] = typed.split(/\s+/);
    const verb = word.toLowerCase();
    const said = typed.startsWith("'") ? typed.slice(1).trim() : verb === "say" ? rest.join(" ") : null;
    this.write([{ text: typed, fg: fg(14) }]);
    if (verb in MOVES) {
      const exit = PLACES[this.here].exits.find((e) => e.dir === MOVES[verb]);
      if (this.sitting) this.refuse("You need to stand up first.");
      else if (this.opponent) this.refuse("You are fighting! Flee first.");
      else if (!exit) this.refuse("You can't go that way.");
      else if (exit.door) this.refuse("The door is closed.");
      else if (!PLACES[exit.to]) this.refuse("That way leads out of this practice game. Try another way.");
      else return this.go(exit.to);
      return this.prompt();
    }
    if (verb === "l" || verb === "look") return this.look();
    if (said !== null) {
      if (said === "") this.refuse("Say what?");
      else this.hooks.heard(`You say '${said}'`);
      return this.prompt();
    }
    if (verb === "sit" || verb === "rest" || verb === "r") {
      this.echo(this.sitting ? "sit.already" : "sit.down", this.sitting ? "You are already sitting!" : "You sit down and take a rest.", this.sitting ? "Already sitting." : "Sitting.");
      this.sitting = true;
      this.sat = true;
      return this.prompt();
    }
    if (verb === "stand" || verb === "st") {
      this.echo(this.sitting ? "stand.up" : "stand.already", this.sitting ? "You stand up." : "You are already standing!", this.sitting ? "Standing." : "Already standing.");
      this.sitting = false;
      return this.prompt();
    }
    if (verb === "kill" || verb === "k" || verb === "attack") {
      this.refuse(`I don't see '${rest.join(" ")}' here.`);
      return this.prompt();
    }
    this.refuse("Huh?!");
    this.prompt();
  }

  /** A command's one-line answer: in Immersive, not written, and the narrator says it in short (lib/echoes.ts). */
  private echo(id: string, line: string, says: string) {
    if (!echoes.echoesOn()) {
      this.write(plain(line));
      return;
    }
    this.narrate(echoes.spoken({ line: 0, lines: 1, id, values: [], says }));
  }

  // ---- The fight ----

  /** The fido fight, a round every two and a half seconds; `then` when it's over. */
  fightFido(then: () => void) {
    const max = FIDO.health;
    let health = max;
    const hit = (word: string, damage: number) => {
      this.write(plain(`You ${word} ${FIDO.name}.`, 15));
      health = Math.max(0, health - damage);
      this.setOpponent({ name: FIDO.name, health, healthMax: max, percent: Math.round((health * 100) / max), range: 0 });
    };
    const bitten = (word: string, damage: number) => {
      this.write(plain(`${cap(FIDO.name)} ${word} you.`, 9));
      this.setVitals({ hp: Math.max(1, (this.vitals.hp ?? 0) - damage) });
    };
    const rounds: (() => void)[] = [
      () => {
        hit("graze", 4);
        bitten("scratches", 2);
      },
      () => hit("hit", 6),
      () => {
        this.write(plain(`${cap(FIDO.name)} attacks you and misses.`, 9));
        hit("cut", 5);
      },
      () => {
        bitten("hurts", 5);
        hit("hit", 5);
      },
      () => hit("cut", 4),
    ];
    this.write(plain(`You attack ${FIDO.name}!`, 15));
    this.setOpponent({ name: FIDO.name, health, healthMax: max, percent: 100, range: 0 });
    const round = (i: number) => {
      if (i < rounds.length) {
        rounds[i]();
        this.prompt();
        this.later(2500, () => round(i + 1));
        return;
      }
      this.write(plain(`${cap(FIDO.name)} is DEAD!!!`, 9), plain("You gain 25 experience points.", 15));
      this.fido = false;
      this.setOpponent(null);
      this.prompt();
      then();
    };
    this.later(1200, () => round(0));
  }
}

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

// ---- The lessons ----

export interface Lesson {
  id: string;
  title: string;
  /** What the narrator teaches first: written and said. */
  teach: string;
  /** What to do: written, and said after the scene. */
  task: string;
  /** Said once it's done. */
  well: string;
  /** Sets the scene quietly: where the player is, what's written. */
  begin?: (s: Stage) => void;
  /** Plays the scene, once the teaching's said; `then` once it's done, to say the task. */
  play?: (s: Stage, then: () => void) => void;
  /** Takes a command before the game would, for the lesson's own; true when it did. */
  command?: (text: string, s: Stage) => boolean;
  /** Whether what was just done finishes the task. */
  finished: (a: Act, s: Stage) => boolean;
}

/** Puts the player in a room with these already explored, nothing written yet. */
function place(s: Stage, id: string, visited: string[], vitals: Partial<mud.Vitals> = {}) {
  s.calm();
  s.sitting = false;
  s.sat = false;
  s.reviews = 0;
  s.fido = false;
  s.here = id;
  s.visited = new Set([id, ...visited]);
  s.vitals = { ...START, ...vitals };
}

const isKey = (a: Act, k: TutorialKey) => "key" in a && a.key === k;

export const LESSONS: Lesson[] = [
  {
    id: "welcome",
    title: "Welcome",
    teach:
      "Welcome to Coupler. This is a practice game: nothing goes to CoffeeMUD, and nobody else hears it. In Immersive, Coupler plays the game to you as sounds, and speaks only what a sound can't say. Each lesson tells you something, then asks you to try it. Type commands in the box as you would in the game, and use Coupler's keys. Return on an empty line says the task again. Once it's done, Return goes on. Esc leaves the tutorial.",
    task: "First, the quiet key. Press Cmd+Period while I'm talking: it stops Coupler's voice at once.",
    well: "That's it. Whenever Coupler goes on too long, Cmd+Period.",
    finished: (a) => isKey(a, "hush"),
  },
  {
    id: "exits",
    title: "Where You Are",
    teach:
      "You're in the Temple of Mota, in Midgaard, the city where CoffeeMUD's players begin. The game writes the room, and Immersive plays its exits as notes, one each, clockwise from north. North is high and south low. East is in your right ear, west in your left. Up glides up, down glides down. A bright note with a sparkle is a way you haven't been; a plain note, a way you have. Here, listen: north, then down, both new.",
    task: "Press Cmd+Shift+L to hear where you are and the exits in words.",
    well: "Cmd+Shift+L works anywhere in the game.",
    begin: (s) => {
      place(s, "Midgaard#3001", []);
      s.look();
    },
    play: (s, then) => {
      s.arrive();
      s.later(1200, then);
    },
    finished: (a) => isKey(a, "where"),
  },
  {
    id: "walking",
    title: "Walking",
    teach:
      "To walk, type a direction: north, or just n. Each step is a footstep. A rising sparkle means the map has learned a new room. Then you hear the new room's exits. Walk back the way you came and there's no sparkle, since you've been there, and the way back is a plain note.",
    task: "Type north, then south to come back.",
    well: "You walked there and back. The map remembers every room you visit.",
    begin: (s) => {
      place(s, "Midgaard#3001", []);
      s.look();
    },
    finished: (_a, s) => s.here === "Midgaard#3001" && s.visited.has("Midgaard#3054"),
  },
  {
    id: "doors",
    title: "Doors and Landmarks",
    teach:
      "Down the temple steps is the Temple Square. A closed door knocks instead of playing a note: west, the Clerics' Guild's door is shut. A locked door knocks twice, low. South of the square is the Market Square. I've named it a landmark, as you can from the map, so a bell rings when you arrive.",
    task: "Type down, then south, to reach the Market Square.",
    well: "You're in the Market Square, the heart of Midgaard.",
    begin: (s) => {
      place(s, "Midgaard#3001", ["Midgaard#3054"]);
      s.look();
    },
    finished: (_a, s) => s.here === "Midgaard#3014",
  },
  {
    id: "body",
    title: "Your Body",
    teach:
      "In the game, your prompt shows your health, mana and movement. Immersive doesn't write it: your body is sounds. Listen to each.",
    task: "Press Cmd+Shift+V to hear your health, mana and movement in words.",
    well: "Cmd+Shift+V, any time. The heartbeat stops once you're over half.",
    begin: (s) => {
      place(s, "Midgaard#3014", ["Midgaard#3001", "Midgaard#3054", "Midgaard#3005"]);
      s.look();
    },
    play: (s, then) =>
      s.steps(
        [
          { say: "A blow worth noticing is a thud.", then: () => s.setVitals({ hp: 16 }), wait: 900 },
          { say: "Under half your health, a heartbeat, faster the lower you go.", then: () => s.setVitals({ hp: 9 }), wait: 3000 },
          { say: "Healing is a shimmer.", then: () => s.setVitals({ hp: 18 }), wait: 1200 },
          { say: "Mana running low is a glassy tone.", then: () => s.setVitals({ mana: 15 }), wait: 1200 },
          { say: "Movement running low is a breath.", then: () => s.setVitals({ moves: 12 }), wait: 1200 },
        ],
        then,
      ),
    finished: (a) => isKey(a, "vitals"),
  },
  {
    id: "fight",
    title: "A Fight",
    teach:
      "A beastly fido is in the square, an easy first fight. A fight is Combat mode. It begins with a low rising call. Each time your opponent's health crosses a quarter, a plucked note plays, lower as they weaken. A chord when it's over. The fight's lines are still written; Cmd+Shift+E says who you're fighting and how they're doing.",
    task: "Type kill fido. While you fight, press Cmd+Shift+E.",
    well: "You won your first fight. Combat mode is over, and you're back in Explore mode.",
    begin: (s) => {
      place(s, "Midgaard#3014", ["Midgaard#3001", "Midgaard#3054", "Midgaard#3005"]);
      s.fido = true;
      s.look();
    },
    command: (text, s) => {
      const [verb = "", ...rest] = text.trim().toLowerCase().split(/\s+/);
      if (!["kill", "k", "attack"].includes(verb)) return false;
      const target = rest.join(" ");
      if (!s.fido || s.opponent || !(target && ("beastly fido".includes(target) || target === "dog"))) return false;
      s.write([{ text: text.trim(), fg: fg(14) }]);
      s.fightFido(() => s.later(800, () => s.done()));
      return true;
    },
    finished: () => false,
  },
  {
    id: "talk",
    title: "Talk",
    teach:
      "In Immersive, what people say isn't written: it's spoken. The narrator says who, then they speak in their own voice. A tell to you, or your group, pings first. The channels, like OOC, only blip, each its own note: they go in the log to read when you like. Everything said is kept in the journal, and Cmd+Shift+J shows it.",
    task: "Answer Aldric: type say, a space, then your words.",
    well: "In the game, anyone in the room hears you.",
    begin: (s) => {
      place(s, "Midgaard#3014", ["Midgaard#3001", "Midgaard#3054", "Midgaard#3005"], { hp: 13 });
      s.write(plain("Aldric arrives from the north."));
      s.prompt();
    },
    play: (s, then) =>
      s.steps(
        [
          { say: "Someone saying something in the room:", then: () => s.talk("Aldric says 'Well fought! Welcome to Midgaard.'", ALDRIC), wait: 500 },
          { say: "A tell, to you alone:", then: () => s.talk("Morgana tells you 'Need a hand, newcomer?'", MORGANA, true), wait: 500 },
          { say: "And a line on the OOC channel, heard only as a blip:", then: () => earcons.logLine("OOC"), wait: 900 },
        ],
        () => {
          s.write([{ text: "Brin OOCs 'has anyone seen my horse?'", fg: fg(13) }]);
          s.prompt();
          then();
        },
      ),
    finished: (a, s) => {
      const said = "command" in a && /^(say\s+\S|'\s*\S)/i.test(a.command.trim());
      if (said) s.later(1500, () => s.talk("Aldric says 'Good luck out there!'", ALDRIC));
      return said;
    },
  },
  {
    id: "narrator",
    title: "The Narrator",
    teach:
      "Some lines the narrator says instead of writing. The time of day, when the sun rises or sets. And the short answers to commands: when you sit, the game writes, You sit down and take a rest, and the narrator says just, Sitting. You can change those words in Workshop.",
    task: "Type sit, then stand.",
    well: "Short answers mean fewer words to listen to.",
    begin: (s) => {
      place(s, "Midgaard#3014", ["Midgaard#3001", "Midgaard#3054", "Midgaard#3005"], { hp: 15 });
    },
    play: (s, then) => {
      // lists.ini's TOD_CHANGE_OUTSIDE, dusk.
      s.narrate("The sun begins to set in the east.");
      s.later(600, then);
    },
    finished: (a, s) => "command" in a && /^(stand|st)\b/i.test(a.command.trim()) && !s.sitting && s.sat,
  },
  {
    id: "review",
    title: "Reading Back",
    teach:
      "Missed something? Review the output a line at a time. Option+Up reads the line before, Option+Down the line after, and Option+End goes back to live. In the game, sending a command goes back to live too. Cmd+Shift+O says everything the game wrote since your last command.",
    task: "Press Option+Up two times to read back through the practice game.",
    well: "That's review. It holds the output still while you read, and counts what comes in.",
    finished: (a, s) => isKey(a, "review") && ++s.reviews >= 2,
  },
  {
    id: "ready",
    title: "Ready to Play",
    teach:
      "That's Immersive. The keys again: Cmd+Shift+L where you are, Cmd+Shift+V your body, Cmd+Shift+E the fight, Cmd+Shift+W who's online, Option+Up to read back, and Cmd+Period for quiet. Every key is in the gear menu, under Keyboard. Ctrl+Cmd+2 switches to Immersive whenever you like, and you can come back to this tutorial from the gear menu.",
    task: "Choose Play in Immersive, or Close to keep the way you play now.",
    well: "",
    finished: () => false,
  },
];


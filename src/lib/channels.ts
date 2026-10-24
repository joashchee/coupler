/**
 * The game's channels as CoffeeMUD last listed them (src-tauri/src/channels.rs):
 * turned off and on with its own commands (NO<NAME>, <NAME> ON), so the
 * game stops sending a channel for this character, in any client. The
 * Log's Mute Channel and the Channels dialog share this list.
 */
import * as mud from "./mud";

let listing: mud.ChannelListing | null = null;
const listeners = new Set<(listing: mud.ChannelListing | null) => void>();
let listening = false;

function told(next: mud.ChannelListing | null) {
  listing = next;
  listeners.forEach((f) => f(listing));
}

/** The list as last read, or null when none was read this connection. */
export const current = () => listing;

/** Calls `f` with each new list; returns the way to stop. */
export function onListing(f: (listing: mud.ChannelListing | null) => void): () => void {
  if (!listening) {
    listening = true;
    void mud.onChannels(told);
  }
  listeners.add(f);
  return () => {
    listeners.delete(f);
  };
}

/** Asks the game for its channels. Not in the game: the list is forgotten, and the reason given. */
export async function ask(): Promise<string | null> {
  try {
    await mud.channelsAsk();
    return null;
  } catch (e) {
    told(null);
    return String(e);
  }
}

/** Whether `name` is on as the game last said, or null when it's not in the list. */
export function isOn(name: string): boolean | null {
  const found = listing?.channels.find((c) => c.name.toLowerCase() === name.trim().toLowerCase());
  return found ? found.on : null;
}

/** The game's next list, or a rejection when none comes in `ms`. */
function nextListing(ms = 12_000): { promise: Promise<mud.ChannelListing>; cancel: () => void } {
  let stop = () => {};
  const promise = new Promise<mud.ChannelListing>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      stop();
      reject("The game didn't answer about its channels. Try again in a moment.");
    }, ms);
    const off = onListing((l) => {
      if (!l) return;
      stop();
      resolve(l);
    });
    stop = () => {
      window.clearTimeout(timer);
      off();
    };
  });
  return { promise, cancel: () => stop() };
}

/** Sends `send` and waits for the list the game gives after it. */
async function sent(send: () => Promise<void>): Promise<mud.ChannelListing> {
  const next = nextListing();
  try {
    await send();
  } catch (e) {
    next.cancel();
    throw String(e);
  }
  return next.promise;
}

/**
 * Turns `name` on or off and says what came of it, in words for the
 * status bar; throws the game's refusal (or why nothing could be sent).
 */
export async function turn(name: string, on: boolean): Promise<string> {
  const l = await sent(() => mud.channelSet(name, on));
  if (l.refused) throw l.refused;
  const now = isOn(name);
  if (now === null) throw l.none ?? `${name} isn't one of your channels.`;
  if (now !== on) throw `The game left ${name} ${now ? "on" : "off"}.`;
  return on ? `${name} is on: the game sends it again.` : `${name} is off: the game no longer sends it.`;
}

/** Turns every channel on or off; says what came of it. */
export async function all(on: boolean): Promise<string> {
  const l = await sent(() => mud.channelsAll(on));
  if (l.refused) throw l.refused;
  if (on) return l.none ?? `Every channel is on: ${l.channels.length} in all.`;
  return "Every channel is off: the game sends none of them.";
}

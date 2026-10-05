/**
 * The web build's stand-in for `@tauri-apps/api/event`: the events the
 * WebAssembly core sends (`src/web/core.ts`), by the desktop's names and
 * payloads, so `lib/mud.ts` listens the same way in both builds.
 */
export type UnlistenFn = () => void;

type Handler = (event: { event: string; payload: unknown }) => void;
const handlers = new Map<string, Set<Handler>>();

export async function listen<T>(event: string, handler: (event: { event: string; payload: T }) => void): Promise<UnlistenFn> {
  const set = handlers.get(event) ?? new Set<Handler>();
  handlers.set(event, set);
  set.add(handler as Handler);
  return () => set.delete(handler as Handler);
}

export function emit(event: string, payload: unknown) {
  for (const h of handlers.get(event) ?? []) h({ event, payload });
}

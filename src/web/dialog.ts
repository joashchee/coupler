/**
 * The web build's stand-in for `@tauri-apps/plugin-dialog`. A browser
 * hands a page files, never paths: a file chosen (or dropped, `webview.ts`)
 * is kept here under a name of its own, which the desktop's commands take
 * as their path (`core.ts` reads it back with `chosen`). Saving is the
 * browser's download, so `save` answers with the name offered.
 */

const files = new Map<string, File>();
let next = 0;

/** Keeps a file the player gave the page; its path for the commands. */
export function keep(file: File): string {
  next += 1;
  const path = `chosen-${next}/${file.name}`;
  files.set(path, file);
  return path;
}

/** A file kept by `keep`. */
export function chosen(path: string): File {
  const file = files.get(path);
  if (!file) throw "That file is no longer at hand. Choose it again.";
  return file;
}

interface OpenOptions {
  multiple?: boolean;
  filters?: { name: string; extensions: string[] }[];
}

/** The browser's file chooser: the chosen file's path, or null. Called while the player's click is still being handled, as browsers ask. */
export function open(options?: OpenOptions): Promise<string | string[] | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = options?.multiple ?? false;
    input.accept = (options?.filters ?? []).flatMap((f) => f.extensions.map((e) => `.${e}`)).join(",");
    input.addEventListener("change", () => {
      const picked = [...(input.files ?? [])].map(keep);
      resolve(picked.length === 0 ? null : input.multiple ? picked : picked[0]);
    });
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}

/** Saving is a download: the name offered, which the browser puts where it puts downloads. */
export async function save(options?: { defaultPath?: string }): Promise<string | null> {
  return options?.defaultPath ?? null;
}

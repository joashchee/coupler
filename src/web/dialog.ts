/** The web build's stand-in for `@tauri-apps/plugin-dialog`: no file choosers, as nothing is imported or saved. */
export async function open(_options?: unknown): Promise<null> {
  return null;
}

export async function save(_options?: unknown): Promise<null> {
  return null;
}

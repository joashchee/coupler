/** The web build's stand-in for `@tauri-apps/api/webview`: nothing is dropped into the web build's Assets (it has the mirror's). */
export function getCurrentWebview() {
  return { onDragDropEvent: async (_handler: unknown) => () => {} };
}

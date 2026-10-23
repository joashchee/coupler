/**
 * The web build's stand-in for `@tauri-apps/api/webview`: files dropped
 * on the page, as the desktop's drag-and-drop events. A browser says
 * nothing of what's being dragged until it's dropped, so `enter` carries
 * no paths; each dropped file is kept (`dialog.ts`) under the path given.
 */
import { keep } from "./dialog";

type DragDrop = { payload: { type: "enter" | "over" | "leave" | "drop"; paths: string[] } };

export function getCurrentWebview() {
  return {
    async onDragDropEvent(handler: (e: DragDrop) => void) {
      // Inside the page, enters and leaves come for every element crossed.
      let depth = 0;
      const files = (e: DragEvent) => [...(e.dataTransfer?.types ?? [])].includes("Files");
      const enter = (e: DragEvent) => {
        if (!files(e)) return;
        e.preventDefault();
        if (depth++ === 0) handler({ payload: { type: "enter", paths: [] } });
      };
      const over = (e: DragEvent) => {
        if (files(e)) e.preventDefault();
      };
      const leave = (e: DragEvent) => {
        if (!files(e)) return;
        if (--depth <= 0) {
          depth = 0;
          handler({ payload: { type: "leave", paths: [] } });
        }
      };
      const drop = (e: DragEvent) => {
        if (!files(e)) return;
        e.preventDefault();
        depth = 0;
        handler({ payload: { type: "drop", paths: [...(e.dataTransfer?.files ?? [])].map(keep) } });
      };
      document.addEventListener("dragenter", enter);
      document.addEventListener("dragover", over);
      document.addEventListener("dragleave", leave);
      document.addEventListener("drop", drop);
      return () => {
        document.removeEventListener("dragenter", enter);
        document.removeEventListener("dragover", over);
        document.removeEventListener("dragleave", leave);
        document.removeEventListener("drop", drop);
      };
    },
  };
}

import { invoke } from "@tauri-apps/api/core";
import type { WhiteboardMeta, WhiteboardData } from "$lib/types";

/**
 * Warstwa nad komendami tablic Excalidraw. Scena żyje zaszyfrowana w
 * `whiteboards/{id}.rune`; edycja odbywa się w osobnym oknie Tauri.
 */
export const whiteboardApi = {
  listWhiteboards: (noteId: string) =>
    invoke<WhiteboardMeta[]>("list_whiteboards", { noteId }),

  loadWhiteboard: (whiteboardId: string) =>
    invoke<WhiteboardData>("load_whiteboard", { whiteboardId }),

  saveWhiteboard: (
    whiteboardId: string,
    noteId: string,
    title: string,
    excalidrawJson: string,
  ) =>
    invoke<WhiteboardMeta>("save_whiteboard", {
      whiteboardId,
      noteId,
      title,
      excalidrawJson,
    }),

  deleteWhiteboard: (whiteboardId: string) =>
    invoke<void>("delete_whiteboard", { whiteboardId }),

  /** Otwiera (lub fokusuje) okno Excalidraw dla tablicy. */
  openWhiteboardWindow: (whiteboardId: string, noteId: string, title: string) =>
    invoke<void>("open_whiteboard_window", { whiteboardId, noteId, title }),
};

import { invoke } from "@tauri-apps/api/core";
import type { Note } from "$lib/types";

/** Kształt `NoteData` oczekiwany przez Rust (daty jako ISO 8601). */
function toDto(note: Note) {
  return {
    id: note.id,
    title: note.title,
    content: note.content,
    tags: note.tags,
    folderId: note.folderId,
    pinned: note.pinned,
    createdAt: note.createdAt.toISOString(),
    updatedAt: note.updatedAt.toISOString(),
  };
}

/**
 * Sterowanie indeksem pełnotekstowym (SQLite FTS5 w pamięci RAM po stronie
 * Rusta). Indeks budowany po odblokowaniu, aktualizowany przy zapisie/usuwaniu.
 */
export const searchApi = {
  /** Buduje indeks od zera (po unlock). Emituje event `search_index_ready`. */
  buildSearchIndex: () => invoke<void>("build_search_index"),

  /** Aktualizuje wpis indeksu po zapisie notatki. */
  updateSearchIndex: (note: Note) =>
    invoke<void>("update_search_index", { note: toDto(note) }),

  /** Usuwa notatkę z indeksu po jej skasowaniu. */
  removeFromSearchIndex: (id: string) =>
    invoke<void>("remove_from_search_index", { id }),
};

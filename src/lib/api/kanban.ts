import { invoke } from "@tauri-apps/api/core";
import type { KanbanData } from "$lib/types";

/**
 * Warstwa nad komendami tablicy Kanban. Stan (kolumny + przypisania) jest
 * szyfrowany w `kanban.rune`; mutacje zwracają zaktualizowaną tablicę.
 */
export const kanbanApi = {
  /** Wczytuje tablicę (przy pierwszym uruchomieniu tworzy domyślne kolumny). */
  loadKanban: () => invoke<KanbanData>("load_kanban"),

  /** Zapisuje cały stan tablicy. */
  saveKanban: (data: KanbanData) => invoke<void>("save_kanban", { data }),

  /** Dodaje notatkę do kolumny. */
  addNoteToKanban: (noteId: string, columnId: string) =>
    invoke<KanbanData>("add_note_to_kanban", { noteId, columnId }),

  /** Przenosi kartę do kolumny na wskazaną pozycję. */
  moveKanbanCard: (noteId: string, targetColumnId: string, targetOrder: number) =>
    invoke<KanbanData>("move_kanban_card", { noteId, targetColumnId, targetOrder }),

  /** Usuwa notatkę z tablicy (notatka pozostaje). */
  removeNoteFromKanban: (noteId: string) =>
    invoke<KanbanData>("remove_note_from_kanban", { noteId }),
};

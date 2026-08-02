import { get, writable } from "svelte/store";
import type { KanbanData } from "$lib/types";
import { kanbanApi } from "$lib/api/kanban";

/** Stan tablicy Kanban aktywnego vaultu (kolumny + przypisania). */
export const kanbanData = writable<KanbanData>({ columns: [], assignments: [] });

/** Czy trwa pierwsze ładowanie tablicy (dla stanu „loading" w widoku). */
export const kanbanLoading = writable(false);

/** Wczytuje tablicę z dysku (tworzy domyślne kolumny przy pierwszym razie). */
export async function loadKanban(): Promise<void> {
  kanbanLoading.set(true);
  try {
    const data = await kanbanApi.loadKanban();
    kanbanData.set(data);
  } catch {
    // Brak Tauri / vault zablokowany — zostaw pustą tablicę.
    kanbanData.set({ columns: [], assignments: [] });
  } finally {
    kanbanLoading.set(false);
  }
}

/** Zapisuje bieżący stan tablicy (fire-and-forget). */
function persist(): void {
  void kanbanApi.saveKanban(get(kanbanData)).catch(() => {});
}

// ── Kolumny ──────────────────────────────────────────────────────────────────

/** Tworzy nową kolumnę na końcu i zwraca jej id (do natychmiastowego rename). */
export function addColumn(name = "New column"): string {
  const id = crypto.randomUUID();
  kanbanData.update((d) => {
    const order = d.columns.reduce((max, c) => Math.max(max, c.order), -1) + 1;
    return { ...d, columns: [...d.columns, { id, name, order, color: null }] };
  });
  persist();
  return id;
}

export function renameColumn(id: string, name: string): void {
  const value = name.trim();
  if (!value) return;
  kanbanData.update((d) => ({
    ...d,
    columns: d.columns.map((c) => (c.id === id ? { ...c, name: value } : c)),
  }));
  persist();
}

export function setColumnColor(id: string, color: string | null): void {
  kanbanData.update((d) => ({
    ...d,
    columns: d.columns.map((c) => (c.id === id ? { ...c, color } : c)),
  }));
  persist();
}

/**
 * Usuwa kolumnę. `moveTo` ≠ null → karty trafiają na koniec wskazanej kolumny;
 * `moveTo` === null → karty znikają z tablicy (notatki pozostają nienaruszone).
 */
export function deleteColumn(id: string, moveTo: string | null): void {
  kanbanData.update((d) => {
    let assignments = d.assignments;
    if (moveTo) {
      const base =
        assignments
          .filter((a) => a.columnId === moveTo)
          .reduce((max, a) => Math.max(max, a.order), -1) + 1;
      let i = 0;
      assignments = assignments.map((a) =>
        a.columnId === id ? { ...a, columnId: moveTo, order: base + i++ } : a,
      );
    } else {
      assignments = assignments.filter((a) => a.columnId !== id);
    }
    return { ...d, columns: d.columns.filter((c) => c.id !== id), assignments };
  });
  persist();
}

// ── Karty (przypisania notatek) ──────────────────────────────────────────────

export function addCard(noteId: string, columnId: string): void {
  kanbanData.update((d) => {
    if (d.assignments.some((a) => a.noteId === noteId)) return d;
    const order =
      d.assignments
        .filter((a) => a.columnId === columnId)
        .reduce((max, a) => Math.max(max, a.order), -1) + 1;
    return { ...d, assignments: [...d.assignments, { noteId, columnId, order }] };
  });
  persist();
}

export function removeCard(noteId: string): void {
  kanbanData.update((d) => ({
    ...d,
    assignments: d.assignments.filter((a) => a.noteId !== noteId),
  }));
  persist();
}

/** Przenosi kartę do `targetColumnId` na pozycję `targetIndex` (z przeliczeniem). */
export function moveCard(noteId: string, targetColumnId: string, targetIndex: number): void {
  kanbanData.update((d) => {
    const moving = d.assignments.find((a) => a.noteId === noteId);
    if (!moving) return d;

    const rest = d.assignments.filter((a) => a.noteId !== noteId);
    const col = rest
      .filter((a) => a.columnId === targetColumnId)
      .sort((a, b) => a.order - b.order);

    const idx = Math.max(0, Math.min(targetIndex, col.length));
    col.splice(idx, 0, { ...moving, columnId: targetColumnId });
    col.forEach((a, i) => (a.order = i));

    const others = rest.filter((a) => a.columnId !== targetColumnId);
    return { ...d, assignments: [...others, ...col] };
  });
  persist();
}

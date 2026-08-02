import { writable } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import type { WhiteboardMeta } from "$lib/types";
import { whiteboardApi } from "$lib/api/whiteboard";

/** Tablice przypisane do aktualnie otwartej w panelu notatki. */
export const whiteboards = writable<WhiteboardMeta[]>([]);

/** Notatka, dla której panel ostatnio wczytał listę (do odświeżeń po evencie). */
let currentNoteId: string | null = null;

/** Wczytuje listę tablic danej notatki. */
export async function loadWhiteboardsFor(noteId: string): Promise<void> {
  currentNoteId = noteId;
  try {
    const list = await whiteboardApi.listWhiteboards(noteId);
    if (currentNoteId === noteId) whiteboards.set(list);
  } catch {
    if (currentNoteId === noteId) whiteboards.set([]);
  }
}

async function reloadCurrent(): Promise<void> {
  if (currentNoteId) await loadWhiteboardsFor(currentNoteId);
}

/**
 * Tworzy nową tablicę i otwiera jej okno. Aktualizacja optymistyczna: karta
 * pojawia się natychmiast (a meta jest utrwalana pustą sceną przed otwarciem).
 */
export async function createWhiteboard(
  noteId: string,
  title = "Untitled board",
): Promise<void> {
  const id = crypto.randomUUID();
  const now = Date.now();
  if (currentNoteId === noteId) {
    whiteboards.update((l) => [
      ...l,
      { id, noteId, title, createdAt: now, updatedAt: now },
    ]);
  }
  try {
    await whiteboardApi.saveWhiteboard(id, noteId, title, "{}");
    await whiteboardApi.openWhiteboardWindow(id, noteId, title);
    await reloadCurrent();
  } catch {
    // Brak Tauri (podgląd) — karta optymistyczna zostaje.
  }
}

/** Otwiera okno istniejącej tablicy. */
export async function openWhiteboard(meta: WhiteboardMeta): Promise<void> {
  try {
    await whiteboardApi.openWhiteboardWindow(meta.id, meta.noteId, meta.title);
  } catch {
    // Brak Tauri — nic nie zrobimy w podglądzie.
  }
}

/** Zmienia nazwę tablicy (zachowując scenę). */
export async function renameWhiteboard(
  id: string,
  noteId: string,
  title: string,
): Promise<void> {
  const value = title.trim();
  if (!value) return;
  whiteboards.update((l) => l.map((w) => (w.id === id ? { ...w, title: value } : w)));
  try {
    const data = await whiteboardApi.loadWhiteboard(id);
    await whiteboardApi.saveWhiteboard(id, noteId, value, data.excalidrawJson);
    await reloadCurrent();
  } catch {
    // Brak Tauri — zostaje zmiana optymistyczna.
  }
}

/** Usuwa tablicę (kartę + plik sceny na dysku). */
export async function removeWhiteboard(id: string): Promise<void> {
  whiteboards.update((l) => l.filter((w) => w.id !== id));
  try {
    await whiteboardApi.deleteWhiteboard(id);
    await reloadCurrent();
  } catch {
    // Brak Tauri — zostaje zmiana optymistyczna.
  }
}

let unlistenSaved: (() => void) | null = null;

/** Nasłuch eventu `rune://whiteboard-saved` — odświeża „updated" karty. */
export async function initWhiteboardEvents(): Promise<void> {
  if (unlistenSaved) return;
  try {
    unlistenSaved = await listen("rune://whiteboard-saved", () => {
      void reloadCurrent();
    });
  } catch {
    // Brak środowiska Tauri — pomiń.
  }
}

import { writable } from "svelte/store";

/** Id notatki, dla której otwarty jest modal „View time log" (null = zamknięty). */
export const timeLogNoteId = writable<string | null>(null);

export function openTimeLog(noteId: string): void {
  timeLogNoteId.set(noteId);
}

export function closeTimeLog(): void {
  timeLogNoteId.set(null);
}

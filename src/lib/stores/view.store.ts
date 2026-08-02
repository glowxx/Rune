import { writable } from "svelte/store";

/** Aktywny widok główny: lista, tablica Kanban albo kalendarz. */
export type ViewMode = "list" | "kanban" | "calendar";

export const viewMode = writable<ViewMode>("list");

export function setView(mode: ViewMode): void {
  viewMode.set(mode);
}

export function showList(): void {
  viewMode.set("list");
}

export function showKanban(): void {
  viewMode.set("kanban");
}

export function showCalendar(): void {
  viewMode.set("calendar");
}

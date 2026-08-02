import { get, writable } from "svelte/store";
import type { TimeEntry } from "$lib/types";
import { timeTrackingApi } from "$lib/api/timeTracking";
import { activeTimerNoteId } from "$lib/stores/app.store";

/** Wszystkie wpisy czasu aktywnego vaultu (źródło prawdy dla komponentów). */
export const timeEntries = writable<TimeEntry[]>([]);

/** Synchronizuje wskaźnik „biegnącego" licznika z aktualną listą wpisów. */
function syncActive(entries: TimeEntry[]): void {
  const running = entries.find((e) => e.endedAt === null);
  activeTimerNoteId.set(running ? running.noteId : null);
}

/** Wczytuje wpisy z dysku i synchronizuje stan biegnącego licznika. */
export async function refreshTimeEntries(): Promise<void> {
  try {
    const entries = await timeTrackingApi.loadTimeEntries();
    timeEntries.set(entries);
    syncActive(entries);
  } catch {
    // Brak Tauri / vault zablokowany — zostaw bieżący stan lokalny.
  }
}

/**
 * Startuje licznik dla notatki. Aktualizacja optymistyczna: natychmiast zamyka
 * każdy biegnący wpis i dodaje nowy (UI tyka od razu), po czym uzgadnia z
 * backendem. Tylko jeden licznik może biec naraz.
 */
export async function startTimer(noteId: string): Promise<void> {
  const now = Date.now();
  timeEntries.update((list) => {
    const next = list.map((e) =>
      e.endedAt === null
        ? { ...e, endedAt: now, durationMs: Math.max(0, now - e.startedAt) }
        : e,
    );
    next.push({
      id: crypto.randomUUID(),
      noteId,
      startedAt: now,
      endedAt: null,
      durationMs: 0,
    });
    return next;
  });
  activeTimerNoteId.set(noteId);

  try {
    await timeTrackingApi.startTimer(noteId);
    await refreshTimeEntries(); // backend jest autorytatywny (właściwe id/czasy)
  } catch {
    // Brak Tauri — zostaw wpis optymistyczny, UI dalej działa.
  }
}

/** Zatrzymuje biegnący licznik notatki (optymistycznie + uzgodnienie). */
export async function stopTimer(noteId: string): Promise<void> {
  const now = Date.now();
  timeEntries.update((list) =>
    list.map((e) =>
      e.noteId === noteId && e.endedAt === null
        ? { ...e, endedAt: now, durationMs: Math.max(0, now - e.startedAt) }
        : e,
    ),
  );
  if (get(activeTimerNoteId) === noteId) activeTimerNoteId.set(null);

  try {
    await timeTrackingApi.stopTimer(noteId);
    await refreshTimeEntries();
  } catch {
    // Brak Tauri — stan lokalny już zaktualizowany.
  }
}

/** Usuwa wpis czasu (optymistycznie + uzgodnienie). */
export async function deleteTimeEntry(id: string): Promise<void> {
  timeEntries.update((list) => list.filter((e) => e.id !== id));
  syncActive(get(timeEntries));
  try {
    await timeTrackingApi.deleteTimeEntry(id);
    await refreshTimeEntries();
  } catch {
    // Brak Tauri — stan lokalny już zaktualizowany.
  }
}

// ── Selektory (czyste, na podstawie listy wpisów) ────────────────────────────

/** Wpisy danej notatki, najnowsze pierwsze. */
export function entriesForNote(entries: TimeEntry[], noteId: string): TimeEntry[] {
  return entries
    .filter((e) => e.noteId === noteId)
    .sort((a, b) => b.startedAt - a.startedAt);
}

/** Suma zamkniętych sesji notatki (ms). Biegnąca sesja nie jest wliczana. */
export function totalMsForNote(entries: TimeEntry[], noteId: string): number {
  return entries
    .filter((e) => e.noteId === noteId && e.endedAt !== null)
    .reduce((sum, e) => sum + e.durationMs, 0);
}

/** Biegnący wpis danej notatki (lub null). */
export function runningEntryForNote(
  entries: TimeEntry[],
  noteId: string,
): TimeEntry | null {
  return entries.find((e) => e.noteId === noteId && e.endedAt === null) ?? null;
}

// ── Formatowanie ─────────────────────────────────────────────────────────────

/** `HH:MM:SS` z milisekund (do bieżącej sesji). */
export function formatHMS(ms: number): string {
  const total = Math.floor(Math.max(0, ms) / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(h)}:${p(m)}:${p(s)}`;
}

/** Czytelny czas: `Xh Ym`, `Ym`, albo `< 1m`. */
export function formatDuration(ms: number): string {
  if (ms < 60_000) return "< 1m";
  const totalMin = Math.floor(ms / 60_000);
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

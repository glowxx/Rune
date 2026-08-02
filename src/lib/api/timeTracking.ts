import { invoke } from "@tauri-apps/api/core";
import type { TimeEntry, NoteSummary } from "$lib/types";

/**
 * Warstwa nad komendami śledzenia czasu. Wpisy są szyfrowane w
 * `time_tracking.rune`; w danym momencie biegnie najwyżej jeden licznik.
 */
export const timeTrackingApi = {
  loadTimeEntries: () => invoke<TimeEntry[]>("load_time_entries"),

  startTimer: (noteId: string) => invoke<TimeEntry>("start_timer", { noteId }),

  stopTimer: (noteId: string) => invoke<TimeEntry>("stop_timer", { noteId }),

  deleteTimeEntry: (entryId: string) =>
    invoke<void>("delete_time_entry", { entryId }),

  getTimeSummary: () => invoke<NoteSummary[]>("get_time_summary"),
};

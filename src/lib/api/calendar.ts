import { invoke } from "@tauri-apps/api/core";
import type { CalendarNote } from "$lib/types";

/**
 * Warstwa nad komendą kalendarza. Treść notatek nie opuszcza Rusta — backend
 * zwraca tylko daty (z `created_at` i z treści) oraz licznik zadań.
 */
export const calendarApi = {
  getCalendarNotes: () => invoke<CalendarNote[]>("get_calendar_notes"),
};

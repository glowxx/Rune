import { get, writable } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import type { SearchHit } from "$lib/types";
import { appState } from "$lib/stores/app.store";
import { notesApi } from "$lib/api/notes";
import { searchApi } from "$lib/api/search";

/** Czy pasek wyszukiwania zastępuje normalny widok Sidebar. */
export const searchActive = writable(false);
/** Aktualny tekst zapytania. */
export const searchQuery = writable("");
/** Wyniki wyszukiwania (po debounce). */
export const searchResults = writable<SearchHit[]>([]);
/** Trwa wyszukiwanie (między naciśnięciem a wynikiem). */
export const searchLoading = writable(false);
/** Trwa budowa indeksu FTS (po odblokowaniu) — UI pokazuje „Indexing…". */
export const searchIndexing = writable(false);

let unlistenReady: (() => void) | null = null;

/**
 * Rejestruje nasłuch eventu `search_index_ready` (wysyłanego przez Rust po
 * zbudowaniu indeksu). Wołane raz przy starcie aplikacji.
 */
export async function initSearchIndexEvents(): Promise<void> {
  if (unlistenReady) return;
  try {
    unlistenReady = await listen("search_index_ready", () => searchIndexing.set(false));
  } catch {
    // Brak środowiska Tauri (podgląd przeglądarki) — pomiń.
  }
}

/** Buduje indeks pełnotekstowy w tle (po odblokowaniu vaultu). */
export async function rebuildSearchIndex(): Promise<void> {
  searchIndexing.set(true);
  try {
    await searchApi.buildSearchIndex();
    // Event `search_index_ready` zgasi „Indexing…". Brak eventu (brak Tauri)
    // i tak nie zaszkodzi — flaga jest tylko wskaźnikiem.
  } catch {
    searchIndexing.set(false);
  }
}

const DEBOUNCE = 200; // lokalne dane — krótki debounce wystarczy
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

export function openSearch(): void {
  searchActive.set(true);
}

export function closeSearch(): void {
  if (debounceTimer) clearTimeout(debounceTimer);
  searchActive.set(false);
  searchQuery.set("");
  searchResults.set([]);
  searchLoading.set(false);
}

/** Live search — wywoływane na każde naciśnięcie klawisza w pasku. */
export function runSearch(query: string): void {
  searchQuery.set(query);
  if (debounceTimer) clearTimeout(debounceTimer);

  const q = query.trim();
  if (!q) {
    searchResults.set([]);
    searchLoading.set(false);
    return;
  }

  searchLoading.set(true);
  debounceTimer = setTimeout(() => void doSearch(q), DEBOUNCE);
}

async function doSearch(q: string): Promise<void> {
  try {
    // Opcja B: pełnotekstowo po stronie Rusta (tytuł + tagi + treść + snippet).
    const hits = await notesApi.searchNotes(q);
    searchResults.set(hits);
  } catch {
    // Fallback (Opcja A): bez Tauri przeszukaj tytuł i tagi z metadanych w pamięci.
    const lower = q.toLowerCase();
    const hits: SearchHit[] = get(appState)
      .notes.filter(
        (n) =>
          n.title.toLowerCase().includes(lower) ||
          n.tags.some((t) => t.toLowerCase().includes(lower)),
      )
      .map((n) => ({
        id: n.id,
        title: n.title,
        tags: n.tags,
        folderId: n.folderId,
        pinned: n.pinned,
        snippet: "",
      }));
    searchResults.set(hits);
  } finally {
    searchLoading.set(false);
  }
}

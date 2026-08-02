import { get, writable } from "svelte/store";
import type { AppState, Folder, Note, NoteMetadata } from "$lib/types";
import { notesApi } from "$lib/api/notes";
import { foldersApi } from "$lib/api/folders";
import { searchApi } from "$lib/api/search";
import { vaultApi } from "$lib/api/vault";
import { setPhase } from "$lib/stores/vault.store";

// Świeży vault startuje bez folderów i notatek — wszystko tworzy użytkownik.
const initialState: AppState = {
  folders: [],
  notes: [],
  activeNoteId: null,
  activeFolderId: null,
};

export const appState = writable<AppState>(initialState);

/** Status autozapisu pokazywany w TopBar. */
export type SaveStatus = "idle" | "saving" | "saved";
export const saveStatus = writable<SaveStatus>("idle");

/**
 * Żądanie wejścia w inline rename folderu (np. po „New folder"). Sidebar to
 * obserwuje i otwiera input dla danego id, po czym resetuje na null.
 */
export const pendingFolderRename = writable<string | null>(null);

/** Identyfikatory notatek, których treść została już pobrana z dysku. */
const contentLoaded = new Set<string>();

// ── Persystencja ───────────────────────────────────────────────────────────

/** Pobiera metadane notatek z dysku i wypełnia store (treść leniwie). */
export async function loadNotes(): Promise<void> {
  let meta: NoteMetadata[];
  try {
    meta = await notesApi.listNotes();
  } catch {
    // Brak środowiska Tauri (podgląd przeglądarki) — zostaw stan pustym.
    return;
  }

  const notes: Note[] = meta.map((m) => ({
    id: m.id,
    title: m.title,
    content: "",
    tags: m.tags,
    folderId: m.folderId,
    pinned: m.pinned,
    createdAt: new Date(m.createdAt),
    updatedAt: new Date(m.updatedAt),
  }));

  contentLoaded.clear();

  appState.update((s) => {
    const firstId = notes[0]?.id ?? null;
    const first = notes[0] ?? null;
    return {
      ...s,
      notes,
      activeNoteId: firstId,
      activeFolderId: first?.folderId ?? s.activeFolderId,
    };
  });

  // Wczytaj treść aktywnej notatki od razu.
  const active = get(appState).activeNoteId;
  if (active) void ensureContent(active);
}

/** Dociąga treść notatki z dysku, jeśli jeszcze niewczytana. */
export async function ensureContent(id: string): Promise<void> {
  if (contentLoaded.has(id)) return;
  try {
    const full = await notesApi.loadNote(id);
    contentLoaded.add(id);
    appState.update((s) => ({
      ...s,
      notes: s.notes.map((n) => (n.id === id ? { ...n, content: full.content } : n)),
    }));
  } catch {
    // Notatka mogła zostać usunięta lub vault zablokowany — ignoruj.
  }
}

// ── Debounced autosave ───────────────────────────────────────────────────────

const SAVE_DELAY = 1500;
const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
let savedResetTimer: ReturnType<typeof setTimeout> | undefined;

/** Planuje zapis notatki po 1,5 s bezczynności. */
function scheduleSave(id: string): void {
  saveStatus.set("saving");
  const existing = saveTimers.get(id);
  if (existing) clearTimeout(existing);

  saveTimers.set(
    id,
    setTimeout(() => {
      saveTimers.delete(id);
      void flushSave(id);
    }, SAVE_DELAY),
  );
}

/** Natychmiastowy zapis notatki o danym id. */
async function flushSave(id: string): Promise<void> {
  const note = get(appState).notes.find((n) => n.id === id);
  if (!note) return;
  try {
    await notesApi.saveNote(note);
    contentLoaded.add(id);
    // Zaktualizuj indeks FTS (no-op, jeśli jeszcze niezbudowany).
    void searchApi.updateSearchIndex(note).catch(() => {});
    saveStatus.set("saved");
    if (savedResetTimer) clearTimeout(savedResetTimer);
    savedResetTimer = setTimeout(() => saveStatus.set("idle"), 2000);
  } catch {
    saveStatus.set("idle");
  }
}

// ── Selekcja / nawigacja ─────────────────────────────────────────────────────

/** Rozwija/zwija folder i ustawia go jako aktywny. */
export function toggleFolder(id: string): void {
  appState.update((s) => ({
    ...s,
    activeFolderId: id,
    folders: s.folders.map((f) =>
      f.id === id ? { ...f, isExpanded: !f.isExpanded } : f,
    ),
  }));
}

/** Ustawia aktywną notatkę i dociąga jej treść. */
export function selectNote(id: string): void {
  appState.update((s) => {
    const note = s.notes.find((n) => n.id === id);
    return {
      ...s,
      activeNoteId: id,
      activeFolderId: note?.folderId ?? s.activeFolderId,
    };
  });
  void ensureContent(id);
}

// ── Mutacje notatek ──────────────────────────────────────────────────────────

export function updateNoteTitle(id: string, title: string): void {
  appState.update((s) => ({
    ...s,
    notes: s.notes.map((n) =>
      n.id === id ? { ...n, title, updatedAt: new Date() } : n,
    ),
  }));
  scheduleSave(id);
}

export function updateNoteContent(id: string, content: string): void {
  appState.update((s) => ({
    ...s,
    notes: s.notes.map((n) =>
      n.id === id ? { ...n, content, updatedAt: new Date() } : n,
    ),
  }));
  scheduleSave(id);
}

/** Znajdzie id notatki po tytule (bez rozróżniania wielkości liter). */
export function findNoteIdByTitle(title: string): string | null {
  const target = title.trim().toLowerCase();
  return (
    get(appState).notes.find((n) => n.title.trim().toLowerCase() === target)?.id ?? null
  );
}

/**
 * Tworzy notatkę w danym folderze, zapisuje ją od razu i zwraca jej id.
 * `createdAt` można nadpisać (np. „Nowa notatka na ten dzień" z kalendarza).
 */
export function createNote(
  folderId: string | null,
  title = "Untitled",
  createdAt?: Date,
): string {
  const id = crypto.randomUUID();
  const now = new Date();
  const created = createdAt ?? now;
  const note: Note = {
    id,
    title,
    content: "",
    tags: [],
    folderId,
    pinned: false,
    createdAt: created,
    updatedAt: now,
  };

  contentLoaded.add(id);
  appState.update((s) => ({
    ...s,
    notes: [note, ...s.notes],
    folders: s.folders.map((f) =>
      f.id === folderId ? { ...f, isExpanded: true } : f,
    ),
  }));

  void flushSave(id);
  return id;
}

/** Zmienia tytuł notatki (z zapisem). */
export function renameNote(id: string, title: string): void {
  updateNoteTitle(id, title);
}

/** Przenosi notatkę do innego folderu (z zapisem). */
export function moveNote(id: string, folderId: string | null): void {
  appState.update((s) => ({
    ...s,
    notes: s.notes.map((n) =>
      n.id === id ? { ...n, folderId, updatedAt: new Date() } : n,
    ),
    folders: s.folders.map((f) =>
      f.id === folderId ? { ...f, isExpanded: true } : f,
    ),
  }));
  void flushSave(id);
}

/** Dodaje tag do notatki (po normalizacji) i planuje zapis. */
export function addNoteTag(id: string, raw: string): void {
  // Bez wiodącego „#", przycięte, bez spacji wewnętrznych łamiących układ.
  const tag = raw.trim().replace(/^#+/, "").trim();
  if (!tag) return;
  appState.update((s) => ({
    ...s,
    notes: s.notes.map((n) => {
      if (n.id !== id) return n;
      if (n.tags.includes(tag)) return n; // bez duplikatów
      return { ...n, tags: [...n.tags, tag], updatedAt: new Date() };
    }),
  }));
  scheduleSave(id);
}

/** Usuwa tag z notatki i planuje zapis. */
export function removeNoteTag(id: string, tag: string): void {
  appState.update((s) => ({
    ...s,
    notes: s.notes.map((n) =>
      n.id === id
        ? { ...n, tags: n.tags.filter((t) => t !== tag), updatedAt: new Date() }
        : n,
    ),
  }));
  scheduleSave(id);
}

/**
 * Id notatki z aktualnie biegnącym licznikiem czasu (lub null). Ustawiany przez
 * time-tracking store; Sidebar pokazuje przy tej notatce zieloną kropkę.
 */
export const activeTimerNoteId = writable<string | null>(null);

/** Aktywny filtr po tagu (null = brak filtra). Ustawiany z panelu Tags. */
export const activeTagFilter = writable<string | null>(null);

export function setTagFilter(tag: string): void {
  activeTagFilter.set(tag);
}

export function clearTagFilter(): void {
  activeTagFilter.set(null);
}

/** Przełącza przypięcie notatki (z zapisem). */
export function togglePin(id: string): void {
  appState.update((s) => ({
    ...s,
    notes: s.notes.map((n) =>
      n.id === id ? { ...n, pinned: !n.pinned, updatedAt: new Date() } : n,
    ),
  }));
  void flushSave(id);
}

/** Duplikuje notatkę (kopiuje też treść), zapisuje i zwraca id kopii. */
export async function duplicateNote(id: string): Promise<string | null> {
  await ensureContent(id);
  const src = get(appState).notes.find((n) => n.id === id);
  if (!src) return null;

  const newId = crypto.randomUUID();
  const now = new Date();
  const copy: Note = {
    ...src,
    id: newId,
    title: `Copy of ${src.title}`,
    pinned: false,
    createdAt: now,
    updatedAt: now,
  };

  contentLoaded.add(newId);
  appState.update((s) => {
    const idx = s.notes.findIndex((n) => n.id === id);
    const notes = [...s.notes];
    notes.splice(idx + 1, 0, copy);
    return { ...s, notes };
  });

  await flushSave(newId);
  return newId;
}

/** Usuwa notatkę z dysku i ze store. */
export async function deleteNote(id: string): Promise<void> {
  // Anuluj oczekujący zapis tej notatki.
  const timer = saveTimers.get(id);
  if (timer) {
    clearTimeout(timer);
    saveTimers.delete(id);
  }
  contentLoaded.delete(id);

  appState.update((s) => {
    const notes = s.notes.filter((n) => n.id !== id);
    const activeNoteId =
      s.activeNoteId === id ? (notes[0]?.id ?? null) : s.activeNoteId;
    return { ...s, notes, activeNoteId };
  });

  try {
    await notesApi.deleteNote(id);
    void searchApi.removeFromSearchIndex(id).catch(() => {});
  } catch {
    // Vault zablokowany lub brak pliku — store i tak już zaktualizowany.
  }
}

// ── Foldery (z persystencją do folders.rune) ─────────────────────────────────

/** Zapisuje całą listę folderów na dysk (fire-and-forget). */
function persistFolders(): void {
  const folders = get(appState).folders;
  void foldersApi.saveFolders(folders).catch(() => {
    // Vault zablokowany lub brak Tauri — store i tak aktualny.
  });
}

/** Wczytuje foldery z dysku przy wejściu do edytora. */
export async function loadFolders(): Promise<void> {
  let folders: Folder[];
  try {
    folders = await foldersApi.loadFolders();
  } catch {
    return; // brak Tauri / vault zablokowany
  }
  folders.sort((a, b) => a.order - b.order);
  appState.update((s) => ({ ...s, folders }));
}

/** Liczba notatek w folderze. */
export function folderNoteCount(state: AppState, folderId: string): number {
  return state.notes.filter((n) => n.folderId === folderId).length;
}

/** Tworzy folder, zapisuje listę i zwraca jego id. */
export function createFolder(name: string): string {
  const id = crypto.randomUUID();
  const now = new Date();
  appState.update((s) => {
    const order = s.folders.reduce((max, f) => Math.max(max, f.order), -1) + 1;
    const folder: Folder = { id, name, isExpanded: true, order, createdAt: now };
    return { ...s, folders: [...s.folders, folder] };
  });
  persistFolders();
  return id;
}

/** Zmienia nazwę folderu i zapisuje. */
export function renameFolder(id: string, name: string): void {
  appState.update((s) => ({
    ...s,
    folders: s.folders.map((f) => (f.id === id ? { ...f, name } : f)),
  }));
  persistFolders();
}

/**
 * Usuwa folder. Notatki z tego folderu trafiają do sekcji „bez folderu"
 * (`folderId = null`) i są zapisywane ponownie. Lista folderów zapisana.
 */
export function deleteFolder(id: string): void {
  const movedIds = get(appState)
    .notes.filter((n) => n.folderId === id)
    .map((n) => n.id);

  appState.update((s) => {
    const folders = s.folders.filter((f) => f.id !== id);
    const notes = s.notes.map((n) =>
      n.folderId === id ? { ...n, folderId: null, updatedAt: new Date() } : n,
    );
    return {
      ...s,
      folders,
      notes,
      activeFolderId: s.activeFolderId === id ? null : s.activeFolderId,
    };
  });

  persistFolders();
  // Zapisz przeniesione notatki z nowym folderId = null.
  for (const noteId of movedIds) void flushSave(noteId);
}

/** Sortuje foldery A→Z (po nazwie), przepisuje `order` i zapisuje. */
export function sortFoldersAZ(): void {
  appState.update((s) => {
    const folders = [...s.folders]
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
      .map((f, i) => ({ ...f, order: i }));
    return { ...s, folders };
  });
  persistFolders();
}

/** Zwija wszystkie foldery (stan UI, bez persystencji). */
export function collapseAllFolders(): void {
  appState.update((s) => ({
    ...s,
    folders: s.folders.map((f) => ({ ...f, isExpanded: false })),
  }));
}

/** Czyści wybór notatki (deselect) — Ctrl+W. */
export function deselectNote(): void {
  appState.update((s) => ({ ...s, activeNoteId: null }));
}

// ── Lock / sesja ─────────────────────────────────────────────────────────────

/**
 * Natychmiast zapisuje aktywną notatkę (pomija debounce). Używane przed
 * zablokowaniem vaultu, żeby nie zgubić zmian.
 */
export async function forceSaveCurrentNote(): Promise<void> {
  const id = get(appState).activeNoteId;
  if (!id) return;
  const t = saveTimers.get(id);
  if (t) {
    clearTimeout(t);
    saveTimers.delete(id);
  }
  await flushSave(id);
}

/** Czyści dane sesji z pamięci (po zablokowaniu lub przełączeniu vaultu). */
export function clearSession(): void {
  contentLoaded.clear();
  activeTagFilter.set(null);
  activeTimerNoteId.set(null);
  appState.set({ folders: [], notes: [], activeNoteId: null, activeFolderId: null });
}

/**
 * Blokuje vault: zapisuje bieżącą notatkę, zeruje klucz w Rust, czyści sesję
 * i wraca do ekranu odblokowania. Wspólna logika auto-lock i ręcznego locka.
 */
export async function lockSession(): Promise<void> {
  await forceSaveCurrentNote();
  try {
    await vaultApi.lockVault();
  } catch {
    // Brak Tauri / już zablokowany — i tak wracamy do ekranu unlock.
  }
  clearSession();
  setPhase("unlock");
}

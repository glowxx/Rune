export interface Note {
  id: string;
  title: string;
  content: string;
  tags: string[];
  folderId: string | null;
  pinned: boolean;
  createdAt: Date;
  updatedAt: Date;
}

/** Metadane notatki bez treści — do listy w sidebarze. */
export interface NoteMetadata {
  id: string;
  title: string;
  tags: string[];
  folderId: string | null;
  pinned: boolean;
  createdAt: string; // ISO 8601
  updatedAt: string; // ISO 8601
}

/** Wynik wyszukiwania — metadane notatki + fragment treści wokół dopasowania. */
export interface SearchHit {
  id: string;
  title: string;
  tags: string[];
  folderId: string | null;
  pinned: boolean;
  snippet: string;
}

/** Notatka linkująca do bieżącej (backlink) + linia kontekstu. */
export interface BacklinkHit {
  id: string;
  title: string;
  folderId: string | null;
  pinned: boolean;
  context: string;
}

/** Węzeł grafu notatek. */
export interface GraphNode {
  id: string;
  title: string;
  folderId: string | null;
}

/** Krawędź grafu (źródło → cel po id notatki). */
export interface GraphEdge {
  source: string;
  target: string;
}

/** Dane grafu notatek z `[[wikilinków]]`. */
export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

/** Wynik importu vaultu (kompatybilny z Rust `ImportResult`). */
export interface ImportResult {
  noteCount: number;
  folderCount: number;
  attachmentCount: number;
}

/** Wynik backupu (kompatybilny z Rust `BackupResult`). */
export interface BackupResult {
  backupPath: string;
  filesCopied: number;
}

/** Informacje o jednym backupie (kompatybilne z Rust `BackupInfo`). */
export interface BackupInfo {
  path: string;
  createdAt: string;
  sizeBytes: number;
}

/** Metadane zaszyfrowanego załącznika (kompatybilne z Rust `AttachmentMeta`). */
export interface AttachmentMeta {
  id: string;
  originalFilename: string;
  mimeType: string;
  sizeBytes: number;
  noteId: string;
  createdAt: string;
}

export interface Folder {
  id: string;
  name: string;
  isExpanded: boolean;
  order: number;
  createdAt: Date;
}

/** Folder zapisywany na dysku (kompatybilny z Rust `FolderData`). */
export interface FolderData {
  id: string;
  name: string;
  createdAt: string; // ISO 8601
  order: number;
}

export interface AppState {
  folders: Folder[];
  notes: Note[];
  activeNoteId: string | null;
  activeFolderId: string | null;
}

/** Parametry kosztu Argon2id (kompatybilne z Rust `ArgonParams`). */
export interface ArgonParams {
  memory_kib: number;
  iterations: number;
  parallelism: number;
}

/** Faza aplikacji sterująca tym, który ekran widać. */
export type AppPhase = "loading" | "vault-select" | "create" | "unlock" | "editor";

/** Wpis listy vaultów (kompatybilny z Rust `VaultInfo`). Bez danych wrażliwych. */
export interface VaultInfo {
  id: string;
  name: string;
  path: string;
  lastOpened: string; // ISO 8601
}

/** Kolumna tablicy Kanban (kompatybilna z Rust `KanbanColumn`). */
export interface KanbanColumn {
  id: string;
  name: string;
  order: number;
  color: string | null;
}

/** Przypisanie notatki do kolumny Kanban (kompatybilne z Rust `KanbanCardAssignment`). */
export interface KanbanCardAssignment {
  noteId: string;
  columnId: string;
  order: number;
}

/** Pełny stan tablicy Kanban (kompatybilny z Rust `KanbanData`). */
export interface KanbanData {
  columns: KanbanColumn[];
  assignments: KanbanCardAssignment[];
}

/** Notatka w ujęciu kalendarza (kompatybilna z Rust `CalendarNote`). */
export interface CalendarNote {
  id: string;
  title: string;
  createdAt: string; // ISO 8601
  dates: string[]; // YYYY-MM-DD z treści
  tags: string[];
  hasTasks: boolean;
  tasksDone: number;
  tasksTotal: number;
}

/** Sesja śledzenia czasu (kompatybilna z Rust `TimeEntry`). */
export interface TimeEntry {
  id: string;
  noteId: string;
  startedAt: number; // unix ms
  endedAt: number | null; // null = wciąż biegnie
  durationMs: number;
}

/** Suma czasu dla notatki (kompatybilna z Rust `NoteSummary`). */
export interface NoteSummary {
  noteId: string;
  totalMs: number;
}

/** Metadane tablicy Excalidraw (kompatybilne z Rust `WhiteboardMeta`). */
export interface WhiteboardMeta {
  id: string;
  noteId: string;
  title: string;
  createdAt: number; // unix ms
  updatedAt: number; // unix ms
}

/** Pełna tablica (metadane + scena) — kompatybilna z Rust `WhiteboardData`. */
export interface WhiteboardData {
  meta: WhiteboardMeta;
  excalidrawJson: string;
}

/** Pozycja w menu kontekstowym. */
export interface ContextMenuItem {
  label?: string; // wymagane dla zwykłych opcji; pomijane dla separatorów
  action?: () => void;
  danger?: boolean; // czerwony kolor (np. „Usuń")
  disabled?: boolean;
  separator?: boolean; // pozioma linia zamiast opcji
}

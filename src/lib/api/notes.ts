import { invoke } from "@tauri-apps/api/core";
import type { Note, NoteMetadata, SearchHit, BacklinkHit, GraphData } from "$lib/types";

/** Kształt wysyłany do Rusta (`NoteData`) — daty jako ISO 8601. */
interface NoteDataDto {
  id: string;
  title: string;
  content: string;
  tags: string[];
  folderId: string | null;
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
}

function toDto(note: Note): NoteDataDto {
  return {
    id: note.id,
    title: note.title,
    content: note.content,
    tags: note.tags,
    folderId: note.folderId,
    pinned: note.pinned,
    createdAt: note.createdAt.toISOString(),
    updatedAt: note.updatedAt.toISOString(),
  };
}

function fromDto(dto: NoteDataDto): Note {
  return {
    id: dto.id,
    title: dto.title,
    content: dto.content,
    tags: dto.tags,
    folderId: dto.folderId,
    pinned: dto.pinned,
    createdAt: new Date(dto.createdAt),
    updatedAt: new Date(dto.updatedAt),
  };
}

/**
 * Warstwa nad komendami notatek. Treść notatek nigdy nie jest trzymana
 * w stanie Rusta — szyfruje się ją kluczem z odblokowanego vaultu.
 */
export const notesApi = {
  /** Szyfruje i zapisuje notatkę na dysk. */
  saveNote: (note: Note) => invoke<void>("save_note", { note: toDto(note) }),

  /** Odczytuje i deszyfruje pełną notatkę. */
  loadNote: async (id: string): Promise<Note> =>
    fromDto(await invoke<NoteDataDto>("load_note", { id })),

  /** Lista metadanych wszystkich notatek (bez treści). */
  listNotes: () => invoke<NoteMetadata[]>("list_notes"),

  /** Pełnotekstowe wyszukiwanie (tytuł + tagi + treść) z fragmentem. */
  searchNotes: (query: string) => invoke<SearchHit[]>("search_notes", { query }),

  /** Notatki linkujące `[[Tytuł]]` do podanego tytułu (backlinki). */
  findBacklinks: (title: string) => invoke<BacklinkHit[]>("find_backlinks", { title }),

  /** Graf notatek: węzły + krawędzie z `[[wikilinków]]`. */
  getGraphData: () => invoke<GraphData>("get_graph_data"),

  /** Usuwa plik notatki z dysku. */
  deleteNote: (id: string) => invoke<void>("delete_note", { id }),
};

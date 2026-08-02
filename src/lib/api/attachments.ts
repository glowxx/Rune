import { invoke } from "@tauri-apps/api/core";
import type { AttachmentMeta } from "$lib/types";

/**
 * Warstwa nad komendami załączników. Pliki są szyfrowane kluczem vaultu po
 * stronie Rusta — frontend nigdy nie trzyma klucza ani plaintextu na dysku.
 */
export const attachmentsApi = {
  /**
   * Szyfruje i zapisuje plik. Zwraca wygenerowane `attachmentId`.
   * `bytes` przekazujemy jako zwykłą tablicę liczb (mapuje się na Rust `Vec<u8>`).
   */
  saveAttachment: (
    bytes: Uint8Array,
    originalFilename: string,
    mimeType: string,
    noteId: string,
  ): Promise<string> =>
    invoke<string>("save_attachment", {
      fileBytes: Array.from(bytes),
      originalFilename,
      mimeType,
      noteId,
    }),

  /** Surowe bajty po deszyfrowaniu (rzadziej potrzebne — preferuj data URL). */
  loadAttachment: (id: string): Promise<number[]> =>
    invoke<number[]>("load_attachment", { id }),

  /** Gotowy `data:` URL (mime + base64) zbudowany w Ruście — do <img src>. */
  loadAttachmentDataUrl: (id: string): Promise<string> =>
    invoke<string>("load_attachment_data_url", { id }),

  /** Usuwa plik załącznika i jego wpis z indeksu. */
  deleteAttachment: (id: string): Promise<void> =>
    invoke<void>("delete_attachment", { id }),

  /** Metadane załączników danej notatki. */
  listAttachmentsForNote: (noteId: string): Promise<AttachmentMeta[]> =>
    invoke<AttachmentMeta[]>("list_attachments_for_note", { noteId }),

  /** Sprząta załączniki osierocone (notatka już nie istnieje). Zwraca liczbę. */
  cleanupOrphanedAttachments: (): Promise<number> =>
    invoke<number>("cleanup_orphaned_attachments"),

  /** Deszyfruje do pliku tymczasowego i otwiera domyślną aplikacją (PDF). */
  openAttachment: (id: string): Promise<void> =>
    invoke<void>("open_attachment", { id }),

  /** Sprząta pliki podglądu z katalogu temp. */
  cleanupTempPreviews: (): Promise<void> => invoke<void>("cleanup_temp_previews"),
};

import { invoke } from "@tauri-apps/api/core";
import type { ImportResult } from "$lib/types";

/** Eksport/import vaultu (.vault) i eksport pojedynczej notatki (DOCX/HTML). */
export const exportApi = {
  /** Eksportuje cały vault do zaszyfrowanego pliku. Zwraca liczbę notatek. */
  exportVault: (exportPassword: string, destPath: string): Promise<number> =>
    invoke<number>("export_vault", { exportPassword, destPath }),

  /** Importuje (merge) vault z pliku `.vault`. */
  importVault: (importPassword: string, srcPath: string): Promise<ImportResult> =>
    invoke<ImportResult>("import_vault", { importPassword, srcPath }),

  /** Eksportuje notatkę do pliku DOCX. */
  exportNoteToDocx: (noteId: string, destPath: string): Promise<void> =>
    invoke<void>("export_note_to_docx", { noteId, destPath }),

  /** Zapisuje HTML notatki do pliku tymczasowego i otwiera go (druk do PDF). */
  exportNoteHtml: (html: string): Promise<void> =>
    invoke<void>("export_note_html", { html }),
};

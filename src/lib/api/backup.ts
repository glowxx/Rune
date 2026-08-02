import { invoke } from "@tauri-apps/api/core";
import type { BackupResult, BackupInfo } from "$lib/types";

/** Lokalny backup — kopia zaszyfrowanych plików vaultu do wybranego folderu. */
export const backupApi = {
  /** Wykonuje backup i przycina do `keepLast` najnowszych. */
  performBackup: (backupDir: string, keepLast: number): Promise<BackupResult> =>
    invoke<BackupResult>("perform_backup", { backupDir, keepLast }),

  /** Lista istniejących backupów (najnowsze pierwsze). */
  getBackupList: (backupDir: string): Promise<BackupInfo[]> =>
    invoke<BackupInfo[]>("get_backup_list", { backupDir }),
};

import { invoke } from "@tauri-apps/api/core";
import type { ArgonParams, VaultInfo } from "$lib/types";

/**
 * Wiele niezależnych vaultów. Każdy w osobnym podkatalogu `%APPDATA%/rune/{id}`
 * z własnym hasłem. Sekrety żyją po stronie Rusta — tu wymieniamy tylko metadane.
 */
export const vaultsApi = {
  /** Lista zarejestrowanych vaultów (z `vaults.json`). */
  listVaults: () => invoke<VaultInfo[]>("list_vaults"),

  /** Tworzy nowy vault i od razu go odblokowuje (klucz w stanie Rusta). */
  createNewVault: (name: string, password: string, params: ArgonParams) =>
    invoke<VaultInfo>("create_new_vault", { name, password, params }),

  /**
   * Przełącza aktywny vault (blokuje bieżący). Zwraca `true`, gdy docelowy ma
   * już `vault.rune` — front pokaże wtedy ekran Unlock.
   */
  switchVault: (vaultId: string) => invoke<boolean>("switch_vault", { vaultId }),

  /** Usuwa vault wraz z danymi na dysku. Nie można usunąć aktywnego. */
  deleteVault: (vaultId: string) => invoke<void>("delete_vault", { vaultId }),

  /** Zmienia wyświetlaną nazwę vaultu. */
  renameVault: (vaultId: string, newName: string) =>
    invoke<void>("rename_vault", { vaultId, newName }),
};

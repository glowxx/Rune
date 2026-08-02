import { invoke } from "@tauri-apps/api/core";
import type { ArgonParams } from "$lib/types";

/**
 * Cienka warstwa nad komendami Tauri vaultu. Wszystkie sekrety (hasło, klucz)
 * żyją po stronie Rusta — frontend nigdy nie trzyma klucza.
 */
export const vaultApi = {
  /** Czy `vault.rune` istnieje na dysku. */
  vaultExists: () => invoke<boolean>("vault_exists"),

  /** Tworzy nowy vault i odblokowuje go (klucz ląduje w stanie Rusta). */
  createVault: (password: string, params: ArgonParams) =>
    invoke<void>("create_vault", { password, params }),

  /**
   * Próbuje odblokować vault. Zwraca `true` przy poprawnym haśle,
   * `false` przy błędnym (lub uszkodzonych danych — nierozróżnialne).
   * Rzuca tylko przy braku pliku / katalogu.
   */
  unlockVault: (password: string) =>
    invoke<boolean>("unlock_vault", { password }),

  /** Zeruje klucz w pamięci Rusta. */
  lockVault: () => invoke<void>("lock_vault"),

  /** Czy vault jest aktualnie odblokowany (klucz w pamięci). */
  isVaultUnlocked: () => invoke<boolean>("is_vault_unlocked"),

  /** Parametry Argon2 zapisane w headerze vaultu (lub `null` gdy brak vaultu). */
  getVaultParams: () => invoke<ArgonParams | null>("get_vault_params"),

  /**
   * Ustawia hasło duress. Wymaga prawdziwego hasła do uwierzytelnienia.
   * Po sukcesie wpisanie hasła duress na ekranie Unlock pokazuje pusty wabik.
   */
  setDuressPassword: (realPassword: string, duressPassword: string) =>
    invoke<void>("set_duress_password", { realPassword, duressPassword }),

  /** Czy hasło duress jest skonfigurowane (bez ujawniania samego hasła). */
  duressConfigured: () => invoke<boolean>("duress_configured"),
};

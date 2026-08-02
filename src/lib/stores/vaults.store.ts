import { writable } from "svelte/store";
import type { ArgonParams, VaultInfo } from "$lib/types";
import { vaultsApi } from "$lib/api/vaults";
import { setPhase } from "$lib/stores/vault.store";
import { forceSaveCurrentNote, clearSession } from "$lib/stores/app.store";

/** Lista zarejestrowanych vaultów (z `vaults.json`). */
export const vaults = writable<VaultInfo[]>([]);

/** Id aktualnie aktywnego (otwartego lub wybranego) vaultu. */
export const activeVaultId = writable<string>("default");

/** Odświeża listę vaultów z dysku. Zwraca ją też dla wygody wołającego. */
export async function refreshVaults(): Promise<VaultInfo[]> {
  try {
    const list = await vaultsApi.listVaults();
    vaults.set(list);
    return list;
  } catch {
    vaults.set([]);
    return [];
  }
}

/** Nazwa vaultu o danym id (do wyświetlenia w switcherze). */
export function vaultName(list: VaultInfo[], id: string): string {
  return list.find((v) => v.id === id)?.name ?? "Vault";
}

/**
 * Wybiera vault na ekranie startowym / w switcherze: zapisuje bieżącą notatkę,
 * blokuje aktualny vault (po stronie Rusta), czyści sesję i przechodzi do
 * ekranu odblokowania wybranego vaultu.
 */
export async function switchToVault(id: string): Promise<void> {
  await forceSaveCurrentNote();
  let exists = true;
  try {
    exists = await vaultsApi.switchVault(id);
  } catch {
    // Brak środowiska Tauri — zmień tylko stan lokalny.
  }
  activeVaultId.set(id);
  clearSession();
  setPhase(exists ? "unlock" : "create");
}

/**
 * Tworzy nowy vault i wchodzi do niego (jest od razu odblokowany po stronie
 * Rusta). Rzuca przy błędzie — wołający pokazuje komunikat.
 */
export async function createAndEnterVault(
  name: string,
  password: string,
  params: ArgonParams,
): Promise<void> {
  const info = await vaultsApi.createNewVault(name, password, params);
  vaults.update((l) => [...l.filter((v) => v.id !== info.id), info]);
  activeVaultId.set(info.id);
  clearSession();
  setPhase("editor");
}

/** Usuwa vault i odświeża listę. Rzuca przy błędzie (np. próba usunięcia aktywnego). */
export async function removeVault(id: string): Promise<void> {
  await vaultsApi.deleteVault(id);
  await refreshVaults();
}

/** Zmienia nazwę vaultu i odświeża listę. */
export async function renameVaultById(id: string, newName: string): Promise<void> {
  await vaultsApi.renameVault(id, newName);
  await refreshVaults();
}

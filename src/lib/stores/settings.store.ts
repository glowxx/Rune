import { writable } from "svelte/store";

/** Czy modal ustawień jest otwarty. */
export const settingsOpen = writable<boolean>(false);

export function openSettings(): void {
  settingsOpen.set(true);
}

export function closeSettings(): void {
  settingsOpen.set(false);
}

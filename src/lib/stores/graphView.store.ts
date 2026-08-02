import { writable } from "svelte/store";

/** Czy pełnoekranowy graf notatek jest otwarty (overlay). */
export const graphViewOpen = writable(false);

export function openGraphView(): void {
  graphViewOpen.set(true);
}

export function closeGraphView(): void {
  graphViewOpen.set(false);
}

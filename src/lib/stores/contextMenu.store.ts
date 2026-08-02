import { writable } from "svelte/store";
import type { ContextMenuItem } from "$lib/types";

export interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}

const initial: ContextMenuState = { visible: false, x: 0, y: 0, items: [] };

export const contextMenu = writable<ContextMenuState>(initial);

/** Otwiera menu kontekstowe w punkcie (x, y) z podanymi opcjami. */
export function openContextMenu(x: number, y: number, items: ContextMenuItem[]): void {
  contextMenu.set({ visible: true, x, y, items });
}

/** Zamyka menu kontekstowe. */
export function closeContextMenu(): void {
  contextMenu.update((s) => ({ ...s, visible: false }));
}

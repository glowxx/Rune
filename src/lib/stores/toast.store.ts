import { writable } from "svelte/store";

export type ToastType = "success" | "error" | "info";

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

/** Kolejka toastów (maks. 3 widoczne naraz). */
export const toasts = writable<Toast[]>([]);

const MAX = 3;
let nextId = 1;

export function showToast(message: string, type: ToastType = "info", durationMs = 3000): void {
  const id = nextId++;
  toasts.update((list) => [...list, { id, type, message }].slice(-MAX));
  setTimeout(() => dismissToast(id), durationMs);
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((t) => t.id !== id));
}

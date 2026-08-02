import { writable } from "svelte/store";
import type { AppPhase, ArgonParams } from "$lib/types";

/** Bieżąca faza aplikacji — steruje routingiem w App.svelte. */
export const appPhase = writable<AppPhase>("loading");

/** Ustawia fazę aplikacji. */
export function setPhase(phase: AppPhase): void {
  appPhase.set(phase);
}

/** Preset Argon2 z metadanymi do UI (suwak + szacowany czas). */
export interface ArgonPreset {
  id: string;
  label: string;
  description: string;
  /** Szacowany czas odblokowania (tekst do wyświetlenia). */
  estimate: string;
  params: ArgonParams;
}

/** Presety odpowiadające `ArgonParams::{fast,balanced,strong,maximum}` w Ruście. */
export const ARGON_PRESETS: ArgonPreset[] = [
  {
    id: "fast",
    label: "Fast",
    description: "For older devices",
    estimate: "~0.5s",
    params: { memory_kib: 19456, iterations: 2, parallelism: 1 },
  },
  {
    id: "balanced",
    label: "Balanced",
    description: "Default — recommended",
    estimate: "~2s",
    params: { memory_kib: 65536, iterations: 3, parallelism: 4 },
  },
  {
    id: "strong",
    label: "Strong",
    description: "Higher cracking cost",
    estimate: "~5s",
    params: { memory_kib: 262144, iterations: 4, parallelism: 4 },
  },
  {
    id: "maximum",
    label: "Maximum",
    description: "Highest security",
    estimate: "~15s",
    params: { memory_kib: 1048576, iterations: 6, parallelism: 4 },
  },
];

/** Domyślny indeks presetu (Balans). */
export const DEFAULT_PRESET_INDEX = 1;

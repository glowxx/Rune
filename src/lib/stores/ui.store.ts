import { writable } from "svelte/store";

/** Tryb edytora — preferencja UI, nie dana wrażliwa (localStorage). */
export type EditorMode = "edit" | "preview" | "split";

const MODE_KEY = "rune.editorMode";
const IDLE_KEY = "rune.idleMinutes";
const CAPTURE_KEY = "rune.captureProtection";
const FOCUS_KEY = "rune.focusMode";
const TIME_TRACKING_KEY = "rune.timeTrackingEnabled";

function readMode(): EditorMode {
  if (typeof localStorage === "undefined") return "edit";
  const v = localStorage.getItem(MODE_KEY);
  return v === "preview" || v === "split" ? v : "edit";
}

function readIdle(): number {
  if (typeof localStorage === "undefined") return 5;
  const raw = localStorage.getItem(IDLE_KEY);
  if (raw === null) return 5; // brak zapisu → domyślne 5 min (Number(null) === 0!)
  const v = Number(raw);
  return [0, 1, 5, 15, 30].includes(v) ? v : 5;
}

export const editorMode = writable<EditorMode>(readMode());
editorMode.subscribe((m) => {
  if (typeof localStorage !== "undefined") localStorage.setItem(MODE_KEY, m);
});

/** Minuty bezczynności do auto-lock. 0 = nigdy. */
export const idleMinutes = writable<number>(readIdle());
idleMinutes.subscribe((n) => {
  if (typeof localStorage !== "undefined") localStorage.setItem(IDLE_KEY, String(n));
});

export const IDLE_OPTIONS = [
  { value: 1, label: "1 min" },
  { value: 5, label: "5 min" },
  { value: 15, label: "15 min" },
  { value: 30, label: "30 min" },
  { value: 0, label: "Never" },
];

/**
 * Anti-screenshot — czy ukrywać okno przed przechwytywaniem ekranu, gdy vault
 * jest odblokowany. Preferencja UI (nie dana wrażliwa), domyślnie włączona.
 */
function readCaptureProtection(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(CAPTURE_KEY) !== "false"; // brak zapisu → domyślnie on
}

export const captureProtection = writable<boolean>(readCaptureProtection());
captureProtection.subscribe((on) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(CAPTURE_KEY, String(on));
  }
});

/**
 * Focus mode — ukrywa sidebar i topbar, zostawiając sam edytor. Preferencja UI
 * (nie dana wrażliwa), domyślnie wyłączona; zapamiętywana między sesjami.
 */
function readFocusMode(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(FOCUS_KEY) === "true";
}

export const focusMode = writable<boolean>(readFocusMode());
focusMode.subscribe((on) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(FOCUS_KEY, String(on));
  }
});

export function toggleFocusMode(): void {
  focusMode.update((on) => !on);
}

export function exitFocusMode(): void {
  focusMode.set(false);
}

/**
 * Czy funkcja śledzenia czasu jest włączona. Preferencja UI (nie dana wrażliwa),
 * domyślnie WYŁĄCZONA — także dla świeżych instalacji i użytkowników, którzy nigdy
 * nie dotknęli tego ustawienia (brak zapisu → false).
 */
function readTimeTracking(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(TIME_TRACKING_KEY) === "true"; // brak zapisu → off
}

export const timeTrackingEnabled = writable<boolean>(readTimeTracking());
timeTrackingEnabled.subscribe((on) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(TIME_TRACKING_KEY, String(on));
  }
});

/**
 * Wstrzymuje odliczanie auto-lock, gdy trwa blokująca operacja kryptograficzna
 * (tworzenie/import/odblokowanie vaultu). Zapobiega temu, by aktualnie otwarty
 * vault został wymuszenie zablokowany w trakcie długiego Argon2. Nie persystowane.
 */
export const autoLockSuspended = writable<boolean>(false);

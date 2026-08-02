import { writable } from "svelte/store";

/** Częstotliwość auto-backupu. */
export type BackupFrequency = "hourly" | "6h" | "daily" | "weekly";

/** Konfiguracja backupu — to NIE są dane wrażliwe (sam opis ustawień). */
export interface BackupConfig {
  enabled: boolean;
  location: string | null;
  frequency: BackupFrequency;
  keepLast: number;
}

const CONFIG_KEY = "rune.backupConfig";
const LAST_KEY = "rune.lastBackupAt";

const DEFAULT: BackupConfig = {
  enabled: false,
  location: null,
  frequency: "daily",
  keepLast: 5,
};

function readConfig(): BackupConfig {
  if (typeof localStorage === "undefined") return { ...DEFAULT };
  try {
    const raw = localStorage.getItem(CONFIG_KEY);
    if (!raw) return { ...DEFAULT };
    return { ...DEFAULT, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT };
  }
}

export const backupConfig = writable<BackupConfig>(readConfig());
backupConfig.subscribe((c) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(CONFIG_KEY, JSON.stringify(c));
  }
});

export const BACKUP_FREQUENCIES: { value: BackupFrequency; label: string; ms: number }[] = [
  { value: "hourly", label: "Every hour", ms: 60 * 60 * 1000 },
  { value: "6h", label: "Every 6 hours", ms: 6 * 60 * 60 * 1000 },
  { value: "daily", label: "Daily", ms: 24 * 60 * 60 * 1000 },
  { value: "weekly", label: "Weekly", ms: 7 * 24 * 60 * 60 * 1000 },
];

export function frequencyMs(f: BackupFrequency): number {
  return BACKUP_FREQUENCIES.find((x) => x.value === f)?.ms ?? DEFAULT.keepLast;
}

export function getLastBackupAt(): number {
  if (typeof localStorage === "undefined") return 0;
  return Number(localStorage.getItem(LAST_KEY)) || 0;
}

export function setLastBackupAt(ts: number): void {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(LAST_KEY, String(ts));
  }
}

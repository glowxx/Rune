import { writable } from "svelte/store";

/**
 * Motywy. Każdy motyw to zestaw wartości zmiennych bazowych z `variables.css`
 * (sekcja BASE). Nadpisujemy je na `document.documentElement`, a reszta UI
 * (nakładki, tinty akcentu, skala szarości) przelicza się sama, bo jest
 * wyrażona względem tych zmiennych.
 *
 * To NIE są dane wrażliwe — sam wybór wyglądu — więc localStorage jest OK.
 */

/** Zmienne bazowe, które definiuje motyw. */
export interface ThemeColors {
  "--bg-app": string;
  "--bg-nav": string;
  "--bg-elevated": string;
  "--accent": string;
  "--accent-rgb": string; // np. "91, 138, 240"
  "--accent-hover": string;
  "--text-primary": string;
  "--fg-rgb": string; // "255, 255, 255" (ciemny) lub "0, 0, 0" (jasny)
}

export interface Theme {
  id: string;
  name: string;
  colors: ThemeColors;
}

/** Wejście edytora motywu własnego (hex + wybór atramentu UI). */
export interface CustomThemeInput {
  bgApp: string;
  bgNav: string;
  bgElevated: string;
  accent: string;
  accentHover: string;
  textPrimary: string;
  /** „light" = jasny atrament (białe nakładki), „dark" = ciemny atrament. */
  ink: "light" | "dark";
}

const THEME_KEY = "rune.themeId";
const CUSTOM_KEY = "rune.customTheme";

export const BUILT_IN_THEMES: Theme[] = [
  {
    id: "rune-dark",
    name: "Rune Dark",
    colors: {
      "--bg-app": "#131211",
      "--bg-nav": "#191715",
      "--bg-elevated": "#211f1d",
      "--accent": "#5b8af0",
      "--accent-rgb": "91, 138, 240",
      "--accent-hover": "#7aa0f4",
      "--text-primary": "#e0deda",
      "--fg-rgb": "255, 255, 255",
    },
  },
  {
    id: "midnight",
    name: "Midnight",
    colors: {
      "--bg-app": "#0d0d0f",
      "--bg-nav": "#0a0a0c",
      "--bg-elevated": "#16161a",
      "--accent": "#7c6af0",
      "--accent-rgb": "124, 106, 240",
      "--accent-hover": "#8f7ff5",
      "--text-primary": "#f0f0f2",
      "--fg-rgb": "255, 255, 255",
    },
  },
  {
    id: "slate",
    name: "Slate",
    colors: {
      "--bg-app": "#1e2130",
      "--bg-nav": "#181b27",
      "--bg-elevated": "#252939",
      "--accent": "#5b9cf6",
      "--accent-rgb": "91, 156, 246",
      "--accent-hover": "#74aef8",
      "--text-primary": "#e2e8f0",
      "--fg-rgb": "255, 255, 255",
    },
  },
  {
    id: "forest",
    name: "Forest",
    colors: {
      "--bg-app": "#1a1f1a",
      "--bg-nav": "#161b16",
      "--bg-elevated": "#202a20",
      "--accent": "#4ead6b",
      "--accent-rgb": "78, 173, 107",
      "--accent-hover": "#5ec07c",
      "--text-primary": "#e8f0e8",
      "--fg-rgb": "255, 255, 255",
    },
  },
  {
    id: "light",
    name: "Light",
    colors: {
      "--bg-app": "#f6f6f4",
      "--bg-nav": "#ececea",
      "--bg-elevated": "#e2e2df",
      "--accent": "#5c4ee8",
      "--accent-rgb": "92, 78, 232",
      "--accent-hover": "#4a3dd4",
      "--text-primary": "#1b1b1a",
      "--fg-rgb": "0, 0, 0",
    },
  },
];

export const CUSTOM_THEME_ID = "custom";

const DEFAULT_CUSTOM: CustomThemeInput = {
  bgApp: "#131211",
  bgNav: "#191715",
  bgElevated: "#211f1d",
  accent: "#5b8af0",
  accentHover: "#7aa0f4",
  textPrimary: "#e0deda",
  ink: "light",
};

/** Zamienia hex (#rgb lub #rrggbb) na „r, g, b". Błędny hex → null. */
export function hexToRgbString(hex: string): string | null {
  let h = hex.trim().replace(/^#/, "");
  if (h.length === 3) h = h.split("").map((c) => c + c).join("");
  if (!/^[0-9a-fA-F]{6}$/.test(h)) return null;
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `${r}, ${g}, ${b}`;
}

/** Buduje pełny zestaw zmiennych motywu z wejścia własnego. */
export function customThemeFromInput(input: CustomThemeInput): Theme {
  return {
    id: CUSTOM_THEME_ID,
    name: "Custom",
    colors: {
      "--bg-app": input.bgApp,
      "--bg-nav": input.bgNav,
      "--bg-elevated": input.bgElevated,
      "--accent": input.accent,
      "--accent-rgb": hexToRgbString(input.accent) ?? "91, 138, 240",
      "--accent-hover": input.accentHover,
      "--text-primary": input.textPrimary,
      "--fg-rgb": input.ink === "dark" ? "0, 0, 0" : "255, 255, 255",
    },
  };
}

function readCustomInput(): CustomThemeInput {
  if (typeof localStorage === "undefined") return { ...DEFAULT_CUSTOM };
  try {
    const raw = localStorage.getItem(CUSTOM_KEY);
    if (!raw) return { ...DEFAULT_CUSTOM };
    return { ...DEFAULT_CUSTOM, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_CUSTOM };
  }
}

function readThemeId(): string {
  if (typeof localStorage === "undefined") return BUILT_IN_THEMES[0].id;
  return localStorage.getItem(THEME_KEY) ?? BUILT_IN_THEMES[0].id;
}

/** Stan reaktywny: aktywne id motywu + bieżące wejście własne. */
export const activeThemeId = writable<string>(readThemeId());
export const customThemeInput = writable<CustomThemeInput>(readCustomInput());

/** Zwraca pełny obiekt motywu dla danego id (z uwzględnieniem własnego). */
export function resolveTheme(id: string, custom: CustomThemeInput): Theme {
  if (id === CUSTOM_THEME_ID) return customThemeFromInput(custom);
  return BUILT_IN_THEMES.find((t) => t.id === id) ?? BUILT_IN_THEMES[0];
}

/** Nakłada zmienne motywu na `:root` (działa od razu, bez przeładowania). */
export function applyThemeColors(colors: ThemeColors): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  for (const [key, value] of Object.entries(colors)) {
    root.style.setProperty(key, value);
  }
}

/** Ustawia aktywny motyw (preset lub własny) i utrwala wybór. */
export function setTheme(id: string, custom?: CustomThemeInput): void {
  const input = custom ?? readCustomInput();
  applyThemeColors(resolveTheme(id, input).colors);
  activeThemeId.set(id);
  if (typeof localStorage !== "undefined") localStorage.setItem(THEME_KEY, id);
}

/** Zapisuje i nakłada motyw własny (po edycji w color-pickerach). */
export function updateCustomTheme(input: CustomThemeInput): void {
  customThemeInput.set(input);
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(CUSTOM_KEY, JSON.stringify(input));
  }
  applyThemeColors(customThemeFromInput(input).colors);
  activeThemeId.set(CUSTOM_THEME_ID);
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(THEME_KEY, CUSTOM_THEME_ID);
  }
}

/** Nakłada zapisany motyw przy starcie aplikacji. */
export function initTheme(): void {
  const id = readThemeId();
  applyThemeColors(resolveTheme(id, readCustomInput()).colors);
}

import { invoke } from "@tauri-apps/api/core";

/**
 * Anti-screenshot. Na Windows wyklucza okno z przechwytywania ekranu
 * (`SetWindowDisplayAffinity`). Na innych platformach komendy są no-op.
 */
export const securityApi = {
  /** Czy platforma wspiera ochronę przed przechwytywaniem (tylko Windows). */
  captureProtectionSupported: () =>
    invoke<boolean>("capture_protection_supported"),

  /** Włącza/wyłącza wykluczenie okna ze zrzutów i nagrań ekranu. */
  toggleCaptureProtection: (enabled: boolean) =>
    invoke<void>("toggle_capture_protection", { enabled }),
};

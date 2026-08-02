<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { closeSettings } from "$lib/stores/settings.store";
  import { captureProtection, timeTrackingEnabled } from "$lib/stores/ui.store";
  import { timeEntries, stopTimer } from "$lib/stores/timeTracking.store";
  import { vaultApi } from "$lib/api/vault";
  import { securityApi } from "$lib/api/security";
  import { exportApi } from "$lib/api/export";
  import { backupApi } from "$lib/api/backup";
  import {
    backupConfig,
    BACKUP_FREQUENCIES,
    setLastBackupAt,
  } from "$lib/stores/backup.store";
  import { showToast } from "$lib/stores/toast.store";
  import { loadNotes, loadFolders } from "$lib/stores/app.store";
  import {
    BUILT_IN_THEMES,
    CUSTOM_THEME_ID,
    activeThemeId,
    customThemeInput,
    setTheme,
    updateCustomTheme,
  } from "$lib/stores/theme.store";
  import type { CustomThemeInput } from "$lib/stores/theme.store";

  // ── Appearance / themes ───────────────────────────────────────────────────
  let showCustom = $state(get(activeThemeId) === CUSTOM_THEME_ID);

  function selectBuiltIn(id: string) {
    setTheme(id);
    showCustom = false;
  }

  function openCustom() {
    showCustom = true;
    updateCustomTheme(get(customThemeInput)); // nałóż własny od razu
  }

  function setCustomField<K extends keyof CustomThemeInput>(
    key: K,
    value: CustomThemeInput[K],
  ) {
    updateCustomTheme({ ...get(customThemeInput), [key]: value });
  }

  // ── Duress password ──────────────────────────────────────────────────────
  let realPw = $state("");
  let duressPw = $state("");
  let duressBusy = $state(false);
  let duressConfigured = $state(false);
  let duressMsg = $state<{ kind: "ok" | "err"; text: string } | null>(null);

  let canSetDuress = $derived(!duressBusy && realPw.length > 0 && duressPw.length > 0);

  async function refreshDuressStatus() {
    try {
      duressConfigured = await vaultApi.duressConfigured();
    } catch {
      // Brak Tauri (podgląd) — zostaw status jak jest.
    }
  }

  async function handleSetDuress() {
    if (!canSetDuress) return;
    duressMsg = null;
    duressBusy = true;
    try {
      await vaultApi.setDuressPassword(realPw, duressPw);
      realPw = "";
      duressPw = "";
      duressMsg = { kind: "ok", text: "Duress password saved." };
      await refreshDuressStatus();
    } catch (e) {
      duressMsg = {
        kind: "err",
        text: typeof e === "string" ? e : "Could not set duress password",
      };
    } finally {
      duressBusy = false;
    }
  }

  // ── Productivity: time tracking ──────────────────────────────────────────
  async function onToggleTimeTracking() {
    const next = !get(timeTrackingEnabled);
    if (!next) {
      // Wyłączanie: najpierw zatrzymaj (i zapisz) biegnącą sesję, potem ukryj.
      const running = get(timeEntries).find((e) => e.endedAt === null);
      if (running) {
        try {
          await stopTimer(running.noteId);
        } catch {
          // Brak Tauri / vault zablokowany — i tak wyłączamy funkcję.
        }
      }
    }
    timeTrackingEnabled.set(next);
  }

  // ── Anti-screenshot ──────────────────────────────────────────────────────
  let captureSupported = $state(true);

  async function refreshCaptureSupport() {
    try {
      captureSupported = await securityApi.captureProtectionSupported();
    } catch {
      captureSupported = false; // brak Tauri → traktuj jak niewspierane
    }
  }

  // ── Data: export / import vaultu ─────────────────────────────────────────
  let exportPw = $state("");
  let exportPw2 = $state("");
  let exportBusy = $state(false);
  let importPath = $state<string | null>(null);
  let importPw = $state("");
  let importBusy = $state(false);

  async function handleExport() {
    if (exportPw.length === 0) {
      showToast("Enter an export password", "error");
      return;
    }
    if (exportPw !== exportPw2) {
      showToast("Passwords don't match", "error");
      return;
    }
    const today = new Date().toISOString().slice(0, 10);
    let dest: string | null;
    try {
      dest = await save({
        defaultPath: `rune-export-${today}.vault`,
        filters: [{ name: "Rune vault", extensions: ["vault"] }],
      });
    } catch {
      showToast("File dialog unavailable", "error");
      return;
    }
    if (!dest) return; // anulowano
    exportBusy = true;
    try {
      const count = await exportApi.exportVault(exportPw, dest);
      exportPw = "";
      exportPw2 = "";
      showToast(`Exported ${count} notes`, "success");
      void revealItemInDir(dest).catch(() => {});
    } catch (e) {
      showToast(typeof e === "string" ? e : "Export failed", "error");
    } finally {
      exportBusy = false;
    }
  }

  async function pickImportFile() {
    try {
      const sel = await open({
        multiple: false,
        filters: [{ name: "Rune vault", extensions: ["vault"] }],
      });
      if (typeof sel === "string") importPath = sel;
    } catch {
      showToast("File dialog unavailable", "error");
    }
  }

  async function handleImport() {
    if (!importPath) {
      showToast("Choose a .vault file first", "error");
      return;
    }
    importBusy = true;
    try {
      const r = await exportApi.importVault(importPw, importPath);
      importPw = "";
      importPath = null;
      showToast(
        `Imported ${r.noteCount} notes, ${r.folderCount} folders, ${r.attachmentCount} attachments`,
        "success",
      );
      await loadFolders();
      await loadNotes();
    } catch (e) {
      showToast(typeof e === "string" ? e : "Import failed", "error");
    } finally {
      importBusy = false;
    }
  }

  // ── Backup ───────────────────────────────────────────────────────────────
  let backupBusy = $state(false);

  async function pickBackupLocation() {
    try {
      const dir = await open({ directory: true, multiple: false });
      if (typeof dir === "string") {
        backupConfig.update((c) => ({ ...c, location: dir }));
      }
    } catch {
      showToast("Folder dialog unavailable", "error");
    }
  }

  async function backupNow() {
    const cfg = get(backupConfig);
    if (!cfg.location) {
      showToast("Choose a backup location first", "error");
      return;
    }
    backupBusy = true;
    try {
      const r = await backupApi.performBackup(cfg.location, cfg.keepLast);
      setLastBackupAt(Date.now());
      showToast(`Backed up ${r.filesCopied} files`, "success");
    } catch (e) {
      showToast(typeof e === "string" ? e : "Backup failed", "error");
    } finally {
      backupBusy = false;
    }
  }

  onMount(() => {
    void refreshDuressStatus();
    void refreshCaptureSupport();

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") closeSettings();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={closeSettings}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <header class="modal-head">
      <h2>Settings</h2>
      <button class="close" type="button" aria-label="Close settings" onclick={closeSettings}>×</button>
    </header>

    <div class="body">
      <!-- Appearance -->
      <section class="block">
        <h3>Appearance</h3>
        <p class="hint">Pick a theme. Changes apply instantly.</p>

        <div class="theme-grid">
          {#each BUILT_IN_THEMES as t (t.id)}
            <button
              class="theme-card"
              class:active={$activeThemeId === t.id}
              type="button"
              onclick={() => selectBuiltIn(t.id)}
            >
              <svg class="preview" viewBox="0 0 80 50" aria-hidden="true">
                <rect width="80" height="50" rx="4" fill={t.colors["--bg-app"]} />
                <rect width="27" height="50" rx="4" fill={t.colors["--bg-nav"]} />
                <rect x="27" width="4" height="50" fill={t.colors["--bg-nav"]} />
                <rect x="36" y="11" width="34" height="4" rx="2" fill={t.colors["--accent"]} />
                <rect x="36" y="22" width="30" height="3" rx="1.5" fill={t.colors["--text-primary"]} opacity="0.55" />
                <rect x="36" y="29" width="24" height="3" rx="1.5" fill={t.colors["--text-primary"]} opacity="0.3" />
                <rect x="6" y="11" width="15" height="3" rx="1.5" fill={t.colors["--text-primary"]} opacity="0.4" />
                <rect x="6" y="18" width="12" height="3" rx="1.5" fill={t.colors["--text-primary"]} opacity="0.25" />
              </svg>
              <span class="theme-name">{t.name}</span>
            </button>
          {/each}

          <button
            class="theme-card custom-card"
            class:active={$activeThemeId === CUSTOM_THEME_ID}
            type="button"
            onclick={openCustom}
          >
            <div class="custom-plus">+</div>
            <span class="theme-name">Custom</span>
          </button>
        </div>

        {#if showCustom}
          <div class="custom-panel">
            <label class="color-row">
              <span>Background</span>
              <input type="color" value={$customThemeInput.bgApp}
                oninput={(e) => setCustomField("bgApp", e.currentTarget.value)} />
            </label>
            <label class="color-row">
              <span>Sidebar</span>
              <input type="color" value={$customThemeInput.bgNav}
                oninput={(e) => setCustomField("bgNav", e.currentTarget.value)} />
            </label>
            <label class="color-row">
              <span>Surfaces</span>
              <input type="color" value={$customThemeInput.bgElevated}
                oninput={(e) => setCustomField("bgElevated", e.currentTarget.value)} />
            </label>
            <label class="color-row">
              <span>Accent</span>
              <input type="color" value={$customThemeInput.accent}
                oninput={(e) => setCustomField("accent", e.currentTarget.value)} />
            </label>
            <label class="color-row">
              <span>Accent (hover)</span>
              <input type="color" value={$customThemeInput.accentHover}
                oninput={(e) => setCustomField("accentHover", e.currentTarget.value)} />
            </label>
            <label class="color-row">
              <span>Text</span>
              <input type="color" value={$customThemeInput.textPrimary}
                oninput={(e) => setCustomField("textPrimary", e.currentTarget.value)} />
            </label>
            <label class="color-row">
              <span>UI tone</span>
              <select
                value={$customThemeInput.ink}
                onchange={(e) => setCustomField("ink", e.currentTarget.value as "light" | "dark")}
              >
                <option value="light">Light text on dark</option>
                <option value="dark">Dark text on light</option>
              </select>
            </label>
          </div>
        {/if}
      </section>

      <div class="divider"></div>

      <!-- Duress password -->
      <section class="block">
        <div class="block-head">
          <h3>Duress password</h3>
          <span class="status" class:on={duressConfigured}>
            {duressConfigured ? "Configured" : "Not configured"}
          </span>
        </div>
        <p class="hint">
          If someone forces you to unlock the vault, enter this password instead.
          It will show an empty vault while your real data stays hidden.
        </p>

        <label class="field">
          <span>Your real password</span>
          <input
            type="password"
            autocomplete="current-password"
            bind:value={realPw}
            disabled={duressBusy}
            placeholder="Required to confirm"
          />
        </label>
        <label class="field">
          <span>Duress password</span>
          <input
            type="password"
            autocomplete="new-password"
            bind:value={duressPw}
            disabled={duressBusy}
            placeholder="Different from your real password"
          />
        </label>

        {#if duressMsg}
          <p class="msg" class:err={duressMsg.kind === "err"}>{duressMsg.text}</p>
        {/if}

        <button class="action" disabled={!canSetDuress} onclick={handleSetDuress}>
          {duressBusy ? "Saving…" : duressConfigured ? "Change duress password" : "Set duress password"}
        </button>
      </section>

      <div class="divider"></div>

      <!-- Anti-screenshot -->
      <section class="block">
        <div class="toggle-row">
          <div class="toggle-text">
            <h3>Hide app content from screen recording/sharing</h3>
            <p class="hint">
              {#if captureSupported}
                Windows only. Excludes the window from screenshots and screen
                capture while unlocked (still visible to you on screen).
              {:else}
                Not available on this platform.
              {/if}
            </p>
          </div>
          <label class="switch" class:disabled={!captureSupported}>
            <input
              type="checkbox"
              bind:checked={$captureProtection}
              disabled={!captureSupported}
            />
            <span class="track"></span>
          </label>
        </div>
      </section>

      <div class="divider"></div>

      <!-- Productivity -->
      <section class="block">
        <h3>Productivity</h3>
        <div class="toggle-row" style="margin-top: 10px;">
          <div class="toggle-text">
            <h3>Enable time tracking</h3>
            <p class="hint">Adds a stopwatch to track time spent on each note.</p>
          </div>
          <label class="switch">
            <input
              type="checkbox"
              checked={$timeTrackingEnabled}
              onchange={onToggleTimeTracking}
            />
            <span class="track"></span>
          </label>
        </div>
      </section>

      <div class="divider"></div>

      <!-- Data: export / import -->
      <section class="block">
        <h3>Data</h3>
        <p class="hint">
          Export your whole vault to a portable encrypted <code>.vault</code> file,
          or import one (existing data is kept — import merges).
        </p>

        <div class="sub">Export vault</div>
        <label class="field">
          <span>Export password</span>
          <input type="password" autocomplete="new-password" bind:value={exportPw}
            disabled={exportBusy} placeholder="Can differ from your main password" />
        </label>
        <label class="field">
          <span>Confirm password</span>
          <input type="password" autocomplete="new-password" bind:value={exportPw2} disabled={exportBusy} />
        </label>
        <button class="action" disabled={exportBusy} onclick={handleExport}>
          {exportBusy ? "Exporting…" : "Export vault"}
        </button>

        <div class="sub">Import vault</div>
        <div class="file-row">
          <button class="ghost" type="button" disabled={importBusy} onclick={pickImportFile}>
            Choose .vault file
          </button>
          <span class="file-name">{importPath ?? "No file selected"}</span>
        </div>
        <label class="field">
          <span>Import password</span>
          <input type="password" autocomplete="off" bind:value={importPw} disabled={importBusy} />
        </label>
        <button class="action" disabled={importBusy || !importPath} onclick={handleImport}>
          {importBusy ? "Importing…" : "Import vault"}
        </button>
      </section>

      <div class="divider"></div>

      <!-- Backup -->
      <section class="block">
        <div class="toggle-row">
          <div class="toggle-text">
            <h3>Auto-backup</h3>
            <p class="hint">
              Periodically copy your encrypted vault to a folder. Files are already
              encrypted — no separate backup password needed.
            </p>
          </div>
          <label class="switch">
            <input
              type="checkbox"
              checked={$backupConfig.enabled}
              onchange={() => backupConfig.update((c) => ({ ...c, enabled: !c.enabled }))}
            />
            <span class="track"></span>
          </label>
        </div>

        <div class="file-row">
          <button class="ghost" type="button" onclick={pickBackupLocation}>
            Backup location
          </button>
          <span class="file-name">{$backupConfig.location ?? "Not set"}</span>
        </div>

        <div class="row-2">
          <label class="field">
            <span>Frequency</span>
            <select
              value={$backupConfig.frequency}
              onchange={(e) =>
                backupConfig.update((c) => ({ ...c, frequency: e.currentTarget.value as typeof c.frequency }))}
            >
              {#each BACKUP_FREQUENCIES as f}
                <option value={f.value}>{f.label}</option>
              {/each}
            </select>
          </label>
          <label class="field">
            <span>Keep last</span>
            <input
              type="number"
              min="1"
              value={$backupConfig.keepLast}
              onchange={(e) =>
                backupConfig.update((c) => ({
                  ...c,
                  keepLast: Math.max(1, Number(e.currentTarget.value) || 1),
                }))}
            />
          </label>
        </div>

        <button class="action" disabled={backupBusy} onclick={backupNow}>
          {backupBusy ? "Backing up…" : "Back up now"}
        </button>
      </section>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 6000;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .modal {
    width: 100%;
    max-width: 440px;
    max-height: 88vh;
    overflow-y: auto;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
  }
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--line);
  }
  .modal-head h2 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.2px;
  }
  .close {
    width: 28px;
    height: 28px;
    border: 1px solid var(--fg-10);
    border-radius: 8px;
    background: transparent;
    color: var(--fg-40);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .close:hover {
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .body {
    padding: 18px 20px 22px;
  }
  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .block h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .status {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--fg-40);
    border: 1px solid var(--fg-10);
    border-radius: 999px;
    padding: 2px 9px;
    white-space: nowrap;
  }
  .status.on {
    color: var(--accent);
    border-color: var(--accent-border);
    background: var(--accent-bg);
  }
  .hint {
    margin-top: 6px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--fg-44);
  }

  /* ── Appearance / themes ── */
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin-top: 12px;
  }
  .theme-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    padding: 6px;
    background: var(--bg-hover-subnote);
    border: 2px solid transparent;
    border-radius: 9px;
    cursor: pointer;
    font-family: var(--font-sans);
    transition: border-color 0.12s;
  }
  .theme-card:hover {
    border-color: var(--fg-15);
  }
  .theme-card.active {
    border-color: var(--accent);
  }
  .preview {
    width: 100%;
    height: auto;
    display: block;
    border-radius: 4px;
  }
  .theme-name {
    font-size: 11px;
    text-align: center;
    color: var(--text-primary);
  }
  .custom-card {
    align-items: center;
    justify-content: center;
  }
  .custom-plus {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: 44px;
    font-size: 22px;
    color: var(--fg-30);
    border-radius: 4px;
    background: var(--bg-app);
  }
  .custom-panel {
    margin-top: 12px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 14px;
  }
  .color-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .color-row span {
    font-size: 12px;
    color: var(--fg-44);
  }
  .color-row input[type="color"] {
    width: 38px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--fg-10);
    border-radius: 5px;
    background: transparent;
    cursor: pointer;
  }
  .color-row select {
    flex: 1;
    min-width: 0;
    background: rgba(var(--fg-rgb), 0.06);
    border: 1px solid var(--fg-10);
    border-radius: 6px;
    padding: 5px 8px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 11.5px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-top: 12px;
  }
  .field span {
    font-size: 12px;
    color: var(--fg-44);
  }
  .field input {
    box-sizing: border-box;
    width: 100%;
    background: rgba(var(--fg-rgb), 0.06);
    border: 1px solid var(--fg-10);
    border-radius: 8px;
    padding: 9px 12px;
    color: var(--text-primary);
    font-family: var(--font-sans, sans-serif);
    font-size: 13px;
    transition: border-color 0.12s;
  }
  .field input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .msg {
    margin-top: 10px;
    font-size: 12px;
    color: #6fcf97;
  }
  .msg.err {
    color: #e05c5c;
  }

  .action {
    margin-top: 14px;
    width: 100%;
    padding: 10px;
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sans, sans-serif);
    cursor: pointer;
    transition: background 0.12s;
  }
  .action:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .action:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .divider {
    height: 1px;
    background: var(--line);
    margin: 20px 0;
  }

  .sub {
    margin-top: 16px;
    margin-bottom: 2px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-36);
  }
  .hint code {
    background: rgba(var(--fg-rgb), 0.08);
    padding: 1px 5px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.9em;
  }
  .file-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
  }
  .file-name {
    font-size: 11.5px;
    color: var(--fg-40);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .ghost {
    flex-shrink: 0;
    border: 1px solid var(--fg-15);
    background: transparent;
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: var(--font-sans);
    padding: 7px 12px;
    border-radius: 7px;
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
  }
  .ghost:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .row-2 {
    display: flex;
    gap: 12px;
    margin-top: 12px;
  }
  .row-2 .field {
    flex: 1;
    margin-top: 0;
  }
  .field select,
  .field input[type="number"] {
    box-sizing: border-box;
    width: 100%;
    background: rgba(var(--fg-rgb), 0.06);
    border: 1px solid var(--fg-10);
    border-radius: 8px;
    padding: 9px 12px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 13px;
  }
  .field select:focus,
  .field input[type="number"]:focus {
    outline: none;
    border-color: var(--accent);
  }

  .toggle-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }
  .toggle-text {
    flex: 1;
    min-width: 0;
  }

  .switch {
    position: relative;
    flex-shrink: 0;
    width: 38px;
    height: 22px;
    cursor: pointer;
  }
  .switch.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .track {
    position: absolute;
    inset: 0;
    background: rgba(var(--fg-rgb), 0.16);
    border-radius: 999px;
    transition: background 0.15s;
  }
  .track::before {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.15s;
  }
  .switch input:checked + .track {
    background: var(--accent);
  }
  .switch input:checked + .track::before {
    transform: translateX(16px);
  }
</style>

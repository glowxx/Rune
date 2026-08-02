<script lang="ts">
  import { marked } from "marked";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    appState,
    saveStatus,
    lockSession,
    forceSaveCurrentNote,
  } from "$lib/stores/app.store";
  import { editorMode, idleMinutes, IDLE_OPTIONS } from "$lib/stores/ui.store";
  import type { EditorMode } from "$lib/stores/ui.store";
  import { openGraphView } from "$lib/stores/graphView.store";
  import { openSettings } from "$lib/stores/settings.store";
  import { focusMode, toggleFocusMode } from "$lib/stores/ui.store";
  import { exportApi } from "$lib/api/export";
  import { showToast } from "$lib/stores/toast.store";
  import { viewMode, showList, showKanban, showCalendar } from "$lib/stores/view.store";
  import { sanitizeMarkdownHtml } from "$lib/utils/markdownSanitizer";

  const activeNote = $derived(
    $appState.notes.find((n) => n.id === $appState.activeNoteId) ?? null,
  );

  // ── Eksport notatki (PDF / DOCX) ───────────────────────────────────────────
  let exportMenuOpen = $state(false);

  function safeFileName(name: string): string {
    return (name.trim() || "note").replace(/[\\/:*?"<>|]/g, "_").slice(0, 80);
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  /** Buduje samodzielny HTML notatki do druku do PDF (z systemowej przeglądarki). */
  function buildPrintHtml(title: string, content: string): string {
    const body = sanitizeMarkdownHtml(
      marked.parse(content, { breaks: true, gfm: true }) as string,
    );
    return `<!DOCTYPE html><html><head><meta charset="utf-8"><title>${escapeHtml(title)}</title>
<style>
  body { font-family: system-ui, -apple-system, "Segoe UI", sans-serif; color: #1a1a1a;
    max-width: 720px; margin: 40px auto; padding: 0 24px; line-height: 1.6; }
  h1, h2, h3, h4 { line-height: 1.3; }
  pre { background: #f4f4f4; padding: 12px; border-radius: 6px; overflow-x: auto; }
  code { background: #f0f0f0; padding: 2px 5px; border-radius: 4px; font-family: ui-monospace, Consolas, monospace; }
  pre code { background: none; padding: 0; }
  blockquote { border-left: 3px solid #5c4ee8; padding-left: 12px; color: #555; margin-left: 0; }
  img { max-width: 100%; }
  table { border-collapse: collapse; } th, td { border: 1px solid #ccc; padding: 6px 10px; }
</style></head><body><h1>${escapeHtml(title)}</h1>${body}</body></html>`;
  }

  async function exportPdf() {
    exportMenuOpen = false;
    if (!activeNote) return;
    await forceSaveCurrentNote();
    try {
      await exportApi.exportNoteHtml(buildPrintHtml(activeNote.title, activeNote.content));
      showToast("Opened in browser — use Print → Save as PDF", "info", 4000);
    } catch (e) {
      showToast(typeof e === "string" ? e : "PDF export failed", "error");
    }
  }

  async function exportDocx() {
    exportMenuOpen = false;
    if (!activeNote) return;
    await forceSaveCurrentNote();
    let dest: string | null;
    try {
      dest = await save({
        defaultPath: `${safeFileName(activeNote.title)}.docx`,
        filters: [{ name: "Word document", extensions: ["docx"] }],
      });
    } catch {
      showToast("File dialog unavailable", "error");
      return;
    }
    if (!dest) return;
    try {
      await exportApi.exportNoteToDocx(activeNote.id, dest);
      showToast("Exported to DOCX", "success");
    } catch (e) {
      showToast(typeof e === "string" ? e : "DOCX export failed", "error");
    }
  }
  const folderId = $derived(activeNote?.folderId ?? $appState.activeFolderId);
  const folder = $derived(
    $appState.folders.find((f) => f.id === folderId) ?? null,
  );

  const saveLabel = $derived(
    $saveStatus === "saving" ? "Saving…" : $saveStatus === "saved" ? "Saved" : "",
  );

  const modes: { id: EditorMode; label: string }[] = [
    { id: "edit", label: "Edit" },
    { id: "preview", label: "Preview" },
    { id: "split", label: "Split" },
  ];
</script>

<header class="toolbar">
  <nav class="breadcrumb">
    {#if folder}
      <span class="crumb-folder">{folder.name}</span>
    {/if}
    {#if folder && activeNote}
      <svg class="crumb-chevron" width="7" height="7" viewBox="0 0 8 8" fill="none" aria-hidden="true">
        <path d="M2.5 1.5L5.5 4L2.5 6.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    {/if}
    {#if activeNote}
      <span class="crumb-note">{activeNote.title}</span>
    {/if}
  </nav>

  <div class="actions">
    <!-- Przełącznik widoku: Lista / Kanban -->
    <div class="view-switch" role="group" aria-label="View mode">
      <button
        class="view-btn"
        class:active={$viewMode === "list"}
        type="button"
        title="List view"
        onclick={showList}
      >
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M2.5 4h11M2.5 8h11M2.5 12h11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
        </svg>
        List
      </button>
      <button
        class="view-btn"
        class:active={$viewMode === "kanban"}
        type="button"
        title="Kanban board"
        onclick={showKanban}
      >
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <rect x="2" y="2.5" width="3.4" height="11" rx="1" stroke="currentColor" stroke-width="1.3" />
          <rect x="6.3" y="2.5" width="3.4" height="7.5" rx="1" stroke="currentColor" stroke-width="1.3" />
          <rect x="10.6" y="2.5" width="3.4" height="9" rx="1" stroke="currentColor" stroke-width="1.3" />
        </svg>
        Kanban
      </button>
      <button
        class="view-btn"
        class:active={$viewMode === "calendar"}
        type="button"
        title="Calendar"
        onclick={showCalendar}
      >
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <rect x="2.5" y="3" width="11" height="10.5" rx="1.4" stroke="currentColor" stroke-width="1.3" />
          <path d="M2.5 6h11M5.5 2v2.4M10.5 2v2.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        </svg>
        Calendar
      </button>
    </div>

    {#if $viewMode === "list"}
    {#if saveLabel}
      <span class="save-status" class:saving={$saveStatus === "saving"}>{saveLabel}</span>
    {/if}

    <!-- Tryb edytora -->
    <div class="mode-switch" role="group" aria-label="Editor mode">
      {#each modes as m}
        <button
          class="mode-btn"
          class:active={$editorMode === m.id}
          type="button"
          onclick={() => editorMode.set(m.id)}
        >
          {m.label}
        </button>
      {/each}
    </div>

    <!-- Auto-lock -->
    <select
      class="idle-select"
      title="Auto-lock after inactivity"
      aria-label="Auto-lock after inactivity"
      bind:value={$idleMinutes}
    >
      {#each IDLE_OPTIONS as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>

    <!-- Eksport notatki -->
    {#if activeNote}
      <div class="export-wrap">
        <button
          class="icon-btn"
          class:active={exportMenuOpen}
          type="button"
          title="Export note"
          aria-label="Export note"
          onclick={() => (exportMenuOpen = !exportMenuOpen)}
        >
          <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M8 2v8M8 2 5.3 4.7M8 2l2.7 2.7" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M3 10v3.2a.8.8 0 0 0 .8.8h8.4a.8.8 0 0 0 .8-.8V10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
          </svg>
        </button>
        {#if exportMenuOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="menu-backdrop" onclick={() => (exportMenuOpen = false)}></div>
          <div class="export-menu" role="menu">
            <button class="export-item" type="button" role="menuitem" onclick={exportPdf}>Export as PDF</button>
            <button class="export-item" type="button" role="menuitem" onclick={exportDocx}>Export as DOCX</button>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Graph view -->
    <button
      class="icon-btn"
      type="button"
      title="Graph view"
      aria-label="Open graph view"
      onclick={openGraphView}
    >
      <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <circle cx="3.5" cy="4" r="1.8" stroke="currentColor" stroke-width="1.3" />
        <circle cx="12.5" cy="5" r="1.8" stroke="currentColor" stroke-width="1.3" />
        <circle cx="7.5" cy="12" r="1.8" stroke="currentColor" stroke-width="1.3" />
        <path d="M5 4.6L11 5M4.6 5.6L6.6 10.4M9 11L11.2 6.6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
      </svg>
    </button>

    <!-- Focus mode -->
    <button
      class="icon-btn"
      class:active={$focusMode}
      type="button"
      title="Focus mode (Ctrl+Shift+F)"
      aria-label="Toggle focus mode"
      onclick={toggleFocusMode}
    >
      <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M2.5 5.5V3a.5.5 0 0 1 .5-.5h2.5M13.5 5.5V3a.5.5 0 0 0-.5-.5h-2.5M2.5 10.5V13a.5.5 0 0 0 .5.5h2.5M13.5 10.5V13a.5.5 0 0 1-.5.5h-2.5"
          stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
    {/if}

    <!-- Settings -->
    <button
      class="icon-btn"
      type="button"
      title="Settings"
      aria-label="Open settings"
      onclick={openSettings}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>

    <!-- Lock now -->
    <button
      class="icon-btn"
      type="button"
      title="Lock now"
      aria-label="Lock now"
      onclick={() => void lockSession()}
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <rect x="3" y="7" width="10" height="6.5" rx="1.4" stroke="currentColor" stroke-width="1.4" />
        <path d="M5 7V5.2C5 3.7 6.3 2.5 8 2.5C9.7 2.5 11 3.7 11 5.2V7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
      </svg>
    </button>
  </div>
</header>

<style>
  .toolbar {
    height: var(--topbar-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px 0 28px;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    min-width: 0;
    overflow: hidden;
  }
  .crumb-folder {
    color: var(--fg-24);
    white-space: nowrap;
  }
  .crumb-chevron {
    color: var(--fg-18);
    flex-shrink: 0;
  }
  .crumb-note {
    color: var(--fg-44);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .save-status {
    font-size: 11px;
    color: var(--fg-30);
    font-family: var(--font-mono);
    transition: opacity 0.2s;
  }
  .save-status.saving {
    color: var(--fg-22);
  }

  .view-switch {
    display: flex;
    background: var(--bg-nav);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }
  .view-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    border: none;
    background: transparent;
    color: var(--fg-36);
    font-size: 11.5px;
    font-family: var(--font-sans);
    padding: 3px 9px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.12s;
  }
  .view-btn:hover {
    color: var(--text-primary);
  }
  .view-btn.active {
    background: var(--accent-bg);
    color: var(--accent);
  }

  .mode-switch {
    display: flex;
    background: var(--bg-nav);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }
  .mode-btn {
    border: none;
    background: transparent;
    color: var(--fg-36);
    font-size: 11.5px;
    font-family: var(--font-sans);
    padding: 3px 9px;
    border-radius: var(--radius-sm);
    transition: all 0.12s;
  }
  .mode-btn:hover {
    color: var(--text-primary);
  }
  .mode-btn.active {
    background: var(--accent-bg);
    color: var(--accent);
  }

  .idle-select {
    background: var(--bg-nav);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    color: var(--fg-36);
    font-size: 11.5px;
    font-family: var(--font-sans);
    padding: 4px 6px;
    cursor: pointer;
    outline: none;
  }
  .idle-select:hover {
    color: var(--text-primary);
  }

  .icon-btn {
    background: none;
    border: none;
    padding: 6px;
    border-radius: var(--radius-lg);
    color: var(--fg-26);
    display: flex;
    align-items: center;
  }
  .icon-btn:hover {
    color: var(--accent);
  }
  .icon-btn.active {
    color: var(--accent);
    background: var(--accent-bg);
  }

  .export-wrap {
    position: relative;
    display: flex;
  }
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
  }
  .export-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 100;
    min-width: 160px;
    background: #232323;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
  }
  .export-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: #d0d0d0;
    font-size: 12.5px;
    font-family: var(--font-sans);
    padding: 8px 12px;
    border-radius: 5px;
  }
  .export-item:hover {
    background: rgba(92, 78, 232, 0.18);
    color: #fff;
  }
</style>

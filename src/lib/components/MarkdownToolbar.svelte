<script lang="ts">
  import type { EditorView } from "@codemirror/view";
  import {
    wrapSelection,
    togglePrefix,
    setHeading,
    insertLink,
    toggleCode,
  } from "$lib/utils/markdownCommands";

  interface Props {
    view: EditorView | undefined;
    onPickFile?: () => void;
  }

  let { view, onPickFile }: Props = $props();

  // Każda akcja działa na żywym EditorView. Po wykonaniu komenda sama oddaje
  // fokus edytorowi (view.focus()), więc kliknięcie nie gubi miejsca pisania.
  function run(fn: (v: EditorView) => void) {
    if (view) fn(view);
  }
</script>

<div class="md-toolbar" role="toolbar" aria-label="Markdown formatting">
  <!-- Formatowanie tekstu -->
  <div class="group">
    <button class="tb-btn" type="button" title="Bold (Ctrl+B)" aria-label="Bold"
      onclick={() => run((v) => wrapSelection(v, "**", "**"))}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 2.5h4.2a2.6 2.6 0 0 1 0 5.2H4.5zM4.5 7.7h4.8a2.7 2.7 0 0 1 0 5.4H4.5z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/></svg>
    </button>
    <button class="tb-btn" type="button" title="Italic (Ctrl+I)" aria-label="Italic"
      onclick={() => run((v) => wrapSelection(v, "*", "*"))}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10.5 2.8H6.7M9.3 13.2H5.5M9.4 2.8 6.6 13.2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
    </button>
    <button class="tb-btn mono" type="button" title="Code (Ctrl+Shift+C)" aria-label="Code"
      onclick={() => run(toggleCode)}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5.5 4.5 2.5 8l3 3.5M10.5 4.5 13.5 8l-3 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
    <button class="tb-btn" type="button" title="Link (Ctrl+K)" aria-label="Link"
      onclick={() => run(insertLink)}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6.8 9.2a2.6 2.6 0 0 0 3.7 0l2-2a2.6 2.6 0 0 0-3.7-3.7l-1 1M9.2 6.8a2.6 2.6 0 0 0-3.7 0l-2 2a2.6 2.6 0 0 0 3.7 3.7l1-1" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
    </button>
    <button class="tb-btn" type="button" title="Insert image or PDF" aria-label="Insert image or PDF"
      onclick={() => onPickFile?.()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2" y="3" width="12" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3"/><circle cx="5.5" cy="6.5" r="1.1" fill="currentColor"/><path d="M3 11.5 6.5 8l2 2L11 7.5l2 2.2" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
  </div>

  <div class="sep" role="separator"></div>

  <!-- Nagłówki -->
  <div class="group">
    <button class="tb-btn text" type="button" title="Heading (Ctrl+Alt+1/2/3)" aria-label="Heading"
      onclick={() => run((v) => setHeading(v, 2))}>H</button>
  </div>

  <div class="sep" role="separator"></div>

  <!-- Listy i cytat -->
  <div class="group">
    <button class="tb-btn" type="button" title="Bullet list" aria-label="Bullet list"
      onclick={() => run((v) => togglePrefix(v, "- "))}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="3" cy="4.5" r="1" fill="currentColor"/><circle cx="3" cy="8" r="1" fill="currentColor"/><circle cx="3" cy="11.5" r="1" fill="currentColor"/><path d="M6 4.5h7M6 8h7M6 11.5h7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
    </button>
    <button class="tb-btn" type="button" title="Numbered list" aria-label="Numbered list"
      onclick={() => run((v) => togglePrefix(v, "1. "))}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 4.5h7M6 8h7M6 11.5h7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><text x="1.4" y="6" font-size="5" fill="currentColor">1</text><text x="1.4" y="9.7" font-size="5" fill="currentColor">2</text><text x="1.4" y="13.4" font-size="5" fill="currentColor">3</text></svg>
    </button>
    <button class="tb-btn" type="button" title="Task list" aria-label="Task list"
      onclick={() => run((v) => togglePrefix(v, "- [ ] "))}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="2" y="2.5" width="5" height="5" rx="1" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M3 5 4 6 6 3.7" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/><path d="M9.5 5h4.5M9.5 11h4.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><rect x="2" y="9" width="5" height="5" rx="1" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>
    </button>
    <button class="tb-btn" type="button" title="Quote" aria-label="Quote"
      onclick={() => run((v) => togglePrefix(v, "> "))}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6.5 4.5C4.8 5.2 4 6.4 4 8.2v3.3h3.3V8.2H5.7c0-1 .4-1.7 1.4-2.2zM12.5 4.5c-1.7.7-2.5 1.9-2.5 3.7v3.3h3.3V8.2h-1.6c0-1 .4-1.7 1.4-2.2z" fill="currentColor"/></svg>
    </button>
  </div>
</div>

<style>
  .md-toolbar {
    height: 36px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-app);
  }
  .group {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .sep {
    width: 1px;
    height: 18px;
    background: rgba(255, 255, 255, 0.1);
    margin: 0 4px;
  }
  .tb-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: rgba(255, 255, 255, 0.6);
    transition: background 0.12s, color 0.12s;
  }
  .tb-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #e8e8e8;
  }
  .tb-btn svg {
    width: 16px;
    height: 16px;
  }
  .tb-btn.text {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--font-sans);
  }
</style>

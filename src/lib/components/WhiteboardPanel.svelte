<script lang="ts">
  import {
    whiteboards,
    loadWhiteboardsFor,
    createWhiteboard,
    openWhiteboard,
    renameWhiteboard,
    removeWhiteboard,
  } from "$lib/stores/whiteboard.store";
  import { openContextMenu } from "$lib/stores/contextMenu.store";
  import type { ContextMenuItem, WhiteboardMeta } from "$lib/types";

  interface Props {
    noteId: string;
  }
  let { noteId }: Props = $props();

  let collapsed = $state(true);
  let renamingId = $state<string | null>(null);
  let renameValue = $state("");

  // Wczytaj tablice przy zmianie notatki.
  $effect(() => {
    const id = noteId;
    void loadWhiteboardsFor(id);
  });

  function relativeTime(ms: number): string {
    const diff = Date.now() - ms;
    if (diff < 60_000) return "just now";
    const min = Math.floor(diff / 60_000);
    if (min < 60) return `${min}m ago`;
    const h = Math.floor(min / 60);
    if (h < 24) return `${h}h ago`;
    const d = Math.floor(h / 24);
    return `${d}d ago`;
  }

  function startRename(wb: WhiteboardMeta) {
    renamingId = wb.id;
    renameValue = wb.title;
  }
  function commitRename() {
    const id = renamingId;
    const value = renameValue.trim();
    renamingId = null;
    if (id && value) void renameWhiteboard(id, noteId, value);
  }
  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitRename();
    } else if (e.key === "Escape") {
      e.preventDefault();
      renamingId = null;
    }
  }
  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function openCardMenu(e: MouseEvent, wb: WhiteboardMeta) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: "Rename", action: () => startRename(wb) },
      { separator: true },
      { label: "Delete", danger: true, action: () => void removeWhiteboard(wb.id) },
    ];
    openContextMenu(e.clientX, e.clientY, items);
  }
</script>

<div class="wb">
  <button class="wb-header" type="button" onclick={() => (collapsed = !collapsed)}>
    <svg class="wb-chevron" class:open={!collapsed} width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
      <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
    <svg class="wb-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <rect x="2" y="3" width="12" height="10" rx="1.6" stroke="currentColor" stroke-width="1.4" />
      <path d="M5 6.5L7 8.5L11 5.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
    <span class="wb-label">Whiteboards{$whiteboards.length > 0 ? ` (${$whiteboards.length})` : ""}</span>
  </button>

  {#if !collapsed}
    <div class="wb-body">
      {#each $whiteboards as wb (wb.id)}
        {#if renamingId === wb.id}
          <div class="wb-card rename">
            <input
              class="wb-rename-input"
              bind:value={renameValue}
              onkeydown={onRenameKey}
              onblur={commitRename}
              use:focusSelect
            />
          </div>
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
          <div
            class="wb-card"
            role="button"
            tabindex="0"
            ondblclick={() => startRename(wb)}
            onclick={() => void openWhiteboard(wb)}
            oncontextmenu={(e) => openCardMenu(e, wb)}
          >
            <span class="wb-card-title">{wb.title}</span>
            <span class="wb-card-time">Updated {relativeTime(wb.updatedAt)}</span>
          </div>
        {/if}
      {/each}

      <button class="wb-new" type="button" onclick={() => void createWhiteboard(noteId)}>
        <svg width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
          <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
        </svg>
        New
      </button>
    </div>
  {/if}
</div>

<style>
  .wb {
    flex-shrink: 0;
    border-top: 1px solid var(--line);
    background: var(--bg-app);
    padding: 8px 40px 12px;
  }
  .wb-header {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    max-width: var(--editor-max);
    margin: 0 auto;
    padding: 4px 0;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--fg-30);
  }
  .wb-header:hover {
    color: var(--fg-50);
  }
  .wb-chevron {
    transition: transform 0.15s;
    flex-shrink: 0;
  }
  .wb-chevron.open {
    transform: rotate(90deg);
  }
  .wb-icon {
    flex-shrink: 0;
  }
  .wb-label {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.4px;
    text-transform: uppercase;
  }

  .wb-body {
    max-width: var(--editor-max);
    margin: 8px auto 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .wb-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 150px;
    min-height: 48px;
    justify-content: center;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg-nav);
    cursor: pointer;
    transition: border-color 0.12s;
    box-sizing: border-box;
  }
  .wb-card:hover {
    border-color: var(--accent-border);
  }
  .wb-card.rename {
    cursor: default;
  }
  .wb-card-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wb-card-time {
    font-size: 10.5px;
    color: var(--fg-30);
    font-family: var(--font-mono);
  }
  .wb-rename-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-app);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    padding: 5px 8px;
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: var(--font-sans);
    outline: none;
  }
  .wb-new {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 16px;
    min-height: 48px;
    border: 1px dashed var(--fg-18);
    border-radius: 8px;
    background: transparent;
    color: var(--fg-30);
    font-size: 12px;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
  }
  .wb-new:hover {
    border-color: var(--accent-border);
    color: var(--accent);
  }
</style>

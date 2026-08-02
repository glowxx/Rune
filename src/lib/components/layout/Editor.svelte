<script lang="ts">
  import {
    appState,
    updateNoteTitle,
    updateNoteContent,
    createNote,
    selectNote,
    addNoteTag,
    removeNoteTag,
  } from "$lib/stores/app.store";
  import { editorMode, timeTrackingEnabled } from "$lib/stores/ui.store";
  import MarkdownEditor from "$lib/components/MarkdownEditor.svelte";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import BacklinksPanel from "$lib/components/BacklinksPanel.svelte";
  import TimeTracker from "$lib/components/TimeTracker.svelte";
  import WhiteboardPanel from "$lib/components/WhiteboardPanel.svelte";

  const dateFmt = new Intl.DateTimeFormat("en-US", {
    day: "numeric",
    month: "long",
    year: "numeric",
  });

  const activeNote = $derived(
    $appState.notes.find((n) => n.id === $appState.activeNoteId) ?? null,
  );

  // Tytuły wszystkich notatek — do autouzupełniania `[[` w edytorze.
  const noteTitles = $derived($appState.notes.map((n) => n.title));

  // Statystyka zadań (task lists) bieżącej notatki — do licznika + paska postępu.
  const taskStats = $derived.by(() => {
    let total = 0;
    let done = 0;
    for (const line of (activeNote?.content ?? "").split("\n")) {
      const m = line.match(/^(\s*)[-*+]\s\[([ xX])\]/);
      if (m) {
        total++;
        if (m[2].toLowerCase() === "x") done++;
      }
    }
    return { total, done };
  });

  function handleCreate() {
    selectNote(createNote(null));
  }

  // ── Edycja tagów ─────────────────────────────────────────────────────────
  let tagDraft = $state("");
  let tagNoteId = $state<string | null>(null);

  // Czyść szkic taga przy zmianie aktywnej notatki (guard chroni przed pętlą).
  $effect(() => {
    const id = activeNote?.id ?? null;
    if (id !== tagNoteId) {
      tagNoteId = id;
      tagDraft = "";
    }
  });

  function onTagKey(e: KeyboardEvent) {
    if (!activeNote) return;
    if (e.key === "Enter") {
      e.preventDefault();
      if (tagDraft.trim()) {
        addNoteTag(activeNote.id, tagDraft);
        tagDraft = "";
      }
    } else if (e.key === "Backspace" && tagDraft === "" && activeNote.tags.length > 0) {
      // Backspace na pustym polu usuwa ostatni tag.
      e.preventDefault();
      removeNoteTag(activeNote.id, activeNote.tags[activeNote.tags.length - 1]);
    }
  }
</script>

<div class="editor-root">
  {#if activeNote}
    <div class="header">
      <div class="header-inner">
        <input
          class="title"
          type="text"
          value={activeNote.title}
          placeholder="Note title"
          oninput={(e) => updateNoteTitle(activeNote.id, e.currentTarget.value)}
        />
        <div class="meta">
          <span class="date">{dateFmt.format(activeNote.createdAt)}</span>
        </div>

        <div class="tags-row">
          {#each activeNote.tags as tag (tag)}
            <span class="tag-chip">
              <span class="tag-text">{tag}</span>
              <button
                class="tag-x"
                type="button"
                title="Remove tag"
                aria-label="Remove tag {tag}"
                onclick={() => removeNoteTag(activeNote.id, tag)}
              >×</button>
            </span>
          {/each}
          <input
            class="tag-input"
            type="text"
            placeholder={activeNote.tags.length === 0 ? "Add tag..." : ""}
            bind:value={tagDraft}
            onkeydown={onTagKey}
          />
        </div>

        {#if taskStats.total > 0}
          <div class="task-progress" title="{taskStats.done} of {taskStats.total} tasks done">
            <span class="task-count">{taskStats.done}/{taskStats.total} tasks done</span>
            <div class="task-bar">
              <div
                class="task-bar-fill"
                style="width: {Math.round((taskStats.done / taskStats.total) * 100)}%"
              ></div>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="body" class:split={$editorMode === "split"}>
      {#if $editorMode === "edit"}
        <div class="pane">
          <MarkdownEditor
            content={activeNote.content}
            onChange={(v) => updateNoteContent(activeNote.id, v)}
            titles={noteTitles}
            noteId={activeNote.id}
          />
        </div>
      {:else if $editorMode === "preview"}
        <div class="pane scroll-pane">
          <MarkdownPreview
            content={activeNote.content}
            onTaskToggle={(v) => updateNoteContent(activeNote.id, v)}
            noteId={activeNote.id}
          />
        </div>
      {:else}
        <div class="pane pane-left">
          <MarkdownEditor
            content={activeNote.content}
            onChange={(v) => updateNoteContent(activeNote.id, v)}
            titles={noteTitles}
            noteId={activeNote.id}
          />
        </div>
        <div class="pane scroll-pane">
          <MarkdownPreview
            content={activeNote.content}
            onTaskToggle={(v) => updateNoteContent(activeNote.id, v)}
            noteId={activeNote.id}
          />
        </div>
      {/if}
    </div>

    <BacklinksPanel noteId={activeNote.id} title={activeNote.title} />
    {#if $timeTrackingEnabled}
      <TimeTracker noteId={activeNote.id} />
    {/if}
    <WhiteboardPanel noteId={activeNote.id} />
  {:else}
    <div class="empty">
      <div class="empty-inner">
        <svg class="empty-icon" width="64" height="64" viewBox="0 0 64 64" fill="none" aria-hidden="true">
          <path
            d="M14 8.5h24l12 12v35a1 1 0 0 1-1 1H14a1 1 0 0 1-1-1V9.5a1 1 0 0 1 1-1Z"
            stroke="currentColor"
            stroke-width="2.2"
          />
          <path d="M38 8.5v12h12" stroke="currentColor" stroke-width="2.2" stroke-linejoin="round" />
          <path d="M21 32h22M21 40h22M21 48h14" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" />
        </svg>
        <p class="empty-title">No note selected</p>
        <p class="empty-sub">Select a note from the sidebar<br />or create a new one.</p>
        <button class="empty-btn" type="button" onclick={handleCreate}>New note</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor-root {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg-app);
  }

  .header {
    flex-shrink: 0;
    padding: 40px 40px 16px;
    border-bottom: 1px solid var(--line);
  }
  .header-inner {
    max-width: var(--editor-max);
    margin: 0 auto;
  }

  .title {
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    font-family: var(--font-sans);
    font-size: 27px;
    font-weight: 600;
    color: var(--text-title);
    letter-spacing: -0.7px;
    line-height: 1.2;
    margin-bottom: 10px;
    padding: 0;
  }
  .title::placeholder {
    color: var(--fg-15);
  }

  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
  }
  .date {
    font-size: 11px;
    color: var(--fg-22);
    font-family: var(--font-mono);
  }

  /* Edytowalny rząd tagów */
  .tags-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 12px;
  }
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    background: rgba(92, 78, 232, 0.15);
    border: 1px solid rgba(92, 78, 232, 0.3);
    color: #a89cf0;
    padding: 2px 10px;
    border-radius: 12px;
    font-size: 12px;
    line-height: 1.4;
  }
  .tag-text {
    white-space: nowrap;
  }
  .tag-x {
    border: none;
    background: transparent;
    color: #a89cf0;
    font-size: 14px;
    line-height: 1;
    padding: 0 0 0 2px;
    margin-left: 1px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .tag-chip:hover .tag-x {
    opacity: 0.7;
  }
  .tag-x:hover {
    opacity: 1 !important;
  }
  .tag-input {
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 12px;
    padding: 2px 2px;
    min-width: 90px;
    flex: 1;
  }
  .tag-input::placeholder {
    color: var(--fg-22);
  }

  /* Licznik zadań + pasek postępu */
  .task-progress {
    margin-top: 12px;
    max-width: 220px;
  }
  .task-count {
    font-size: 11px;
    color: var(--fg-36);
    font-family: var(--font-mono);
  }
  .task-bar {
    margin-top: 5px;
    height: 3px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    overflow: hidden;
  }
  .task-bar-fill {
    height: 100%;
    background: #5c4ee8;
    border-radius: 999px;
    transition: width 0.2s ease;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .pane {
    flex: 1;
    min-width: 0;
    height: 100%;
  }
  .scroll-pane {
    overflow-y: auto;
    padding: 28px 40px 80px;
  }
  .body.split .pane-left {
    border-right: 1px solid rgba(255, 255, 255, 0.08);
  }
  .body.split .scroll-pane {
    padding: 28px 28px 80px;
  }

  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .empty-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    color: rgba(255, 255, 255, 0.2);
  }
  .empty-icon {
    color: rgba(255, 255, 255, 0.15);
    margin-bottom: 18px;
  }
  .empty-title {
    font-size: 15px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.3);
  }
  .empty-sub {
    margin-top: 6px;
    font-size: 12.5px;
    line-height: 1.6;
    color: rgba(255, 255, 255, 0.2);
  }
  .empty-btn {
    margin-top: 20px;
    padding: 8px 18px;
    border: 1px solid var(--accent-border);
    background: var(--accent-bg);
    color: var(--accent);
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sans);
  }
  .empty-btn:hover {
    opacity: 0.8;
  }
</style>

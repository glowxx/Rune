<script lang="ts">
  import { onMount } from "svelte";
  import {
    appState,
    selectNote,
    createNote,
    ensureContent,
  } from "$lib/stores/app.store";
  import {
    kanbanData,
    kanbanLoading,
    loadKanban,
    addColumn,
    renameColumn,
    deleteColumn,
    setColumnColor,
    addCard,
    removeCard,
    moveCard,
  } from "$lib/stores/kanban.store";
  import { showList } from "$lib/stores/view.store";
  import { openContextMenu } from "$lib/stores/contextMenu.store";
  import type { ContextMenuItem, KanbanColumn, Note } from "$lib/types";

  // Presety koloru nagłówka kolumny.
  const COLUMN_COLORS: { label: string; value: string | null }[] = [
    { label: "Default", value: null },
    { label: "Blue", value: "#5b8af0" },
    { label: "Green", value: "#4ead6b" },
    { label: "Amber", value: "#d6a14e" },
    { label: "Red", value: "#d76b6b" },
    { label: "Purple", value: "#9b7bd4" },
  ];

  onMount(() => {
    void loadKanban();
  });

  // Mapa id → notatka (dla szybkiego dostępu w renderze).
  let noteById = $derived(new Map($appState.notes.map((n) => [n.id, n])));

  // Kolumny w kolejności `order`.
  let columns = $derived(
    [...$kanbanData.columns].sort((a, b) => a.order - b.order),
  );

  /** Karty (notatki) danej kolumny — tylko istniejące notatki, wg `order`. */
  function cardsOf(columnId: string): Note[] {
    return $kanbanData.assignments
      .filter((a) => a.columnId === columnId && noteById.has(a.noteId))
      .sort((a, b) => a.order - b.order)
      .map((a) => noteById.get(a.noteId)!) as Note[];
  }

  // Dociągnij treść przypisanych notatek (do paska postępu zadań na kartach).
  const requested = new Set<string>();
  $effect(() => {
    for (const a of $kanbanData.assignments) {
      if (!requested.has(a.noteId)) {
        requested.add(a.noteId);
        void ensureContent(a.noteId);
      }
    }
  });

  /** Statystyka zadań (checkboxów) notatki — taki sam parser jak w edytorze. */
  function taskStats(content: string): { total: number; done: number } {
    let total = 0;
    let done = 0;
    for (const line of (content ?? "").split("\n")) {
      const m = line.match(/^(\s*)[-*+]\s\[([ xX])\]/);
      if (m) {
        total++;
        if (m[2].toLowerCase() === "x") done++;
      }
    }
    return { total, done };
  }

  function openNote(note: Note) {
    selectNote(note.id);
    showList();
  }

  // ── Inline rename kolumny ────────────────────────────────────────────────
  let renamingColumn = $state<string | null>(null);
  let renameValue = $state("");

  function startColumnRename(col: KanbanColumn) {
    renamingColumn = col.id;
    renameValue = col.name;
  }
  function commitColumnRename() {
    const id = renamingColumn;
    const value = renameValue.trim();
    renamingColumn = null;
    if (id && value) renameColumn(id, value);
  }
  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitColumnRename();
    } else if (e.key === "Escape") {
      e.preventDefault();
      renamingColumn = null;
    }
  }
  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function handleAddColumn() {
    const id = addColumn();
    renamingColumn = id;
    renameValue = "New column";
  }

  // ── Menu kolumny (⋮) ─────────────────────────────────────────────────────
  function openColumnMenu(e: MouseEvent, col: KanbanColumn) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: "Rename", action: () => startColumnRename(col) },
      { label: "Change color", action: () => openColorMenu(col, e.clientX, e.clientY) },
      { separator: true },
      { label: "Delete column", danger: true, action: () => requestDeleteColumn(col, e.clientX, e.clientY) },
    ];
    openContextMenu(e.clientX, e.clientY, items);
  }

  function openColorMenu(col: KanbanColumn, x: number, y: number) {
    const items: ContextMenuItem[] = COLUMN_COLORS.map((c) => ({
      label: c.label,
      disabled: (col.color ?? null) === c.value,
      action: () => setColumnColor(col.id, c.value),
    }));
    openContextMenu(x, y, items);
  }

  /** Usuwanie kolumny: z kartami pyta co z nimi, pusta znika od razu. */
  function requestDeleteColumn(col: KanbanColumn, x: number, y: number) {
    const count = cardsOf(col.id).length;
    if (count === 0) {
      deleteColumn(col.id, null);
      return;
    }
    const others = columns.filter((c) => c.id !== col.id);
    const items: ContextMenuItem[] = [
      ...others.map((c) => ({
        label: `Move cards to “${c.name}”`,
        action: () => deleteColumn(col.id, c.id),
      })),
      { separator: true },
      { label: "Remove cards from board", danger: true, action: () => deleteColumn(col.id, null) },
    ];
    openContextMenu(x, y, items);
  }

  // ── Menu karty (prawy klik) ──────────────────────────────────────────────
  function openCardMenu(e: MouseEvent, note: Note) {
    e.preventDefault();
    e.stopPropagation();
    const x = e.clientX;
    const y = e.clientY;
    const items: ContextMenuItem[] = [
      { label: "Open note", action: () => openNote(note) },
      { label: "Move to column", action: () => openMoveMenu(note, x, y) },
      { separator: true },
      { label: "Remove from board", danger: true, action: () => removeCard(note.id) },
    ];
    openContextMenu(x, y, items);
  }

  function openMoveMenu(note: Note, x: number, y: number) {
    const current = $kanbanData.assignments.find((a) => a.noteId === note.id)?.columnId;
    const items: ContextMenuItem[] = columns.map((c) => ({
      label: c.name,
      disabled: c.id === current,
      action: () => moveCard(note.id, c.id, cardsOf(c.id).length),
    }));
    openContextMenu(x, y, items);
  }

  // ── Add card picker ──────────────────────────────────────────────────────
  let pickerColumn = $state<string | null>(null);
  let pickerQuery = $state("");

  function openPicker(columnId: string) {
    pickerColumn = columnId;
    pickerQuery = "";
  }
  function closePicker() {
    pickerColumn = null;
    pickerQuery = "";
  }

  // Id notatek już obecnych na tablicy.
  let onBoard = $derived(new Set($kanbanData.assignments.map((a) => a.noteId)));

  // Notatki dostępne do dodania (nie na tablicy), filtrowane po zapytaniu.
  let pickerResults = $derived.by(() => {
    const q = pickerQuery.trim().toLowerCase();
    return $appState.notes
      .filter((n) => !onBoard.has(n.id))
      .filter((n) => !q || n.title.toLowerCase().includes(q))
      .slice(0, 50);
  });

  function pickNote(noteId: string) {
    if (pickerColumn) addCard(noteId, pickerColumn);
    closePicker();
  }

  function createAndAdd() {
    if (!pickerColumn) return;
    const title = pickerQuery.trim() || "Untitled";
    const id = createNote(null, title);
    addCard(id, pickerColumn);
    closePicker();
  }

  function focusInput(node: HTMLInputElement) {
    node.focus();
  }

  // ── Drag & drop (natywne HTML5) ──────────────────────────────────────────
  let draggingId = $state<string | null>(null);
  let overColumn = $state<string | null>(null);
  let overIndex = $state<number>(-1);

  function onDragStart(e: DragEvent, noteId: string) {
    draggingId = noteId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", noteId);
    }
  }

  function onDragEnd() {
    draggingId = null;
    overColumn = null;
    overIndex = -1;
  }

  /** Ustala pozycję wstawienia na podstawie środka najechanej karty. */
  function onCardDragOver(e: DragEvent, columnId: string, index: number) {
    if (!draggingId) return;
    e.preventDefault();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const after = e.clientY > rect.top + rect.height / 2;
    overColumn = columnId;
    overIndex = after ? index + 1 : index;
  }

  /** Najechanie na pustą przestrzeń kolumny → wstawienie na koniec. */
  function onColumnDragOver(e: DragEvent, columnId: string, count: number) {
    if (!draggingId) return;
    e.preventDefault();
    if (overColumn !== columnId) {
      overColumn = columnId;
      overIndex = count;
    }
  }

  function onColumnDrop(e: DragEvent, columnId: string) {
    if (!draggingId) return;
    e.preventDefault();
    const idx = overColumn === columnId && overIndex >= 0 ? overIndex : cardsOf(columnId).length;
    moveCard(draggingId, columnId, idx);
    onDragEnd();
  }
</script>

<div class="board">
  {#if $kanbanLoading && columns.length === 0}
    <div class="board-loading">Loading board…</div>
  {:else}
    {#each columns as col (col.id)}
      {@const cards = cardsOf(col.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <section
        class="column"
        ondragover={(e) => onColumnDragOver(e, col.id, cards.length)}
        ondrop={(e) => onColumnDrop(e, col.id)}
      >
        <header class="col-head" style={col.color ? `border-top: 2px solid ${col.color}` : ""}>
          {#if renamingColumn === col.id}
            <input
              class="col-rename"
              bind:value={renameValue}
              onkeydown={onRenameKey}
              onblur={commitColumnRename}
              use:focusSelect
            />
          {:else}
            <button class="col-title" type="button" ondblclick={() => startColumnRename(col)}>
              {#if col.color}
                <span class="col-dot" style={`background: ${col.color}`}></span>
              {/if}
              <span class="col-name">{col.name}</span>
              <span class="col-count">{cards.length}</span>
            </button>
            <button
              class="col-menu"
              type="button"
              title="Column options"
              aria-label="Column options"
              onclick={(e) => openColumnMenu(e, col)}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                <circle cx="8" cy="3.5" r="1.2" fill="currentColor" />
                <circle cx="8" cy="8" r="1.2" fill="currentColor" />
                <circle cx="8" cy="12.5" r="1.2" fill="currentColor" />
              </svg>
            </button>
          {/if}
        </header>

        <div class="col-body">
          {#each cards as note, i (note.id)}
            {@const stats = taskStats(note.content)}
            {#if overColumn === col.id && overIndex === i}
              <div class="placeholder"></div>
            {/if}
            <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
            <div
              class="card"
              class:dragging={draggingId === note.id}
              draggable="true"
              role="button"
              tabindex="0"
              ondragstart={(e) => onDragStart(e, note.id)}
              ondragend={onDragEnd}
              ondragover={(e) => onCardDragOver(e, col.id, i)}
              onclick={() => openNote(note)}
              oncontextmenu={(e) => openCardMenu(e, note)}
            >
              <button
                class="card-remove"
                type="button"
                title="Remove from board"
                aria-label="Remove from board"
                onclick={(e) => { e.stopPropagation(); removeCard(note.id); }}
              >×</button>
              <div class="card-title">{note.title}</div>
              {#if note.tags.length > 0}
                <div class="card-tags">
                  {#each note.tags as tag (tag)}
                    <span class="card-tag">#{tag}</span>
                  {/each}
                </div>
              {/if}
              {#if stats.total > 0}
                <div class="card-progress" title="{stats.done} of {stats.total} tasks done">
                  <div class="card-progress-bar">
                    <div class="card-progress-fill" style="width: {Math.round((stats.done / stats.total) * 100)}%"></div>
                  </div>
                  <span class="card-progress-count">{stats.done}/{stats.total}</span>
                </div>
              {/if}
            </div>
          {/each}

          {#if overColumn === col.id && overIndex === cards.length}
            <div class="placeholder"></div>
          {/if}

          {#if cards.length === 0 && !(overColumn === col.id)}
            <div class="col-empty">No cards</div>
          {/if}
        </div>

        {#if pickerColumn === col.id}
          <div class="picker">
            <input
              class="picker-input"
              type="text"
              placeholder="Search notes…"
              bind:value={pickerQuery}
              use:focusInput
              onkeydown={(e) => { if (e.key === "Escape") closePicker(); }}
            />
            <div class="picker-list">
              {#each pickerResults as note (note.id)}
                <button class="picker-item" type="button" onclick={() => pickNote(note.id)}>
                  {note.title}
                </button>
              {/each}
              {#if pickerResults.length === 0}
                <div class="picker-empty">No notes available</div>
              {/if}
              <button class="picker-create" type="button" onclick={createAndAdd}>
                + Create “{pickerQuery.trim() || "Untitled"}”
              </button>
            </div>
            <button class="picker-close" type="button" onclick={closePicker}>Cancel</button>
          </div>
        {:else}
          <button class="add-card" type="button" onclick={() => openPicker(col.id)}>
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
              <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
            </svg>
            Add card
          </button>
        {/if}
      </section>
    {/each}

    <button class="add-column" type="button" onclick={handleAddColumn}>
      <svg width="13" height="13" viewBox="0 0 12 12" fill="none" aria-hidden="true">
        <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
      Add column
    </button>
  {/if}
</div>

<style>
  .board {
    display: flex;
    flex-direction: row;
    gap: 16px;
    padding: 20px;
    overflow-x: auto;
    overflow-y: hidden;
    width: 100%;
    height: 100%;
    background: var(--bg-app);
    box-sizing: border-box;
  }
  .board-loading {
    margin: auto;
    color: var(--fg-30);
    font-size: 13px;
  }

  .column {
    width: 280px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    max-height: 100%;
    background: var(--bg-nav);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 12px;
    box-sizing: border-box;
  }

  .col-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
    padding-top: 2px;
  }
  .col-title {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-family: var(--font-sans);
    padding: 0;
  }
  .col-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .col-name {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .col-count {
    font-size: 11px;
    color: var(--fg-30);
    font-family: var(--font-mono);
    background: var(--bg-elevated);
    border-radius: 10px;
    padding: 1px 7px;
  }
  .col-menu {
    flex-shrink: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--fg-30);
    display: flex;
    align-items: center;
    padding: 2px;
    border-radius: var(--radius-md);
  }
  .col-menu:hover {
    color: var(--text-primary);
  }
  .col-rename {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-app);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    padding: 4px 8px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-sans);
    outline: none;
  }

  .col-body {
    flex: 1;
    overflow-y: auto;
    min-height: 30px;
    display: flex;
    flex-direction: column;
  }
  .col-empty {
    padding: 14px 4px;
    text-align: center;
    font-size: 11.5px;
    font-style: italic;
    color: var(--fg-20);
  }

  .card {
    position: relative;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 8px;
    cursor: pointer;
    transition: border-color 0.12s, transform 0.05s;
  }
  .card:hover {
    border-color: var(--accent-border);
  }
  .card.dragging {
    opacity: 0.45;
  }
  .card-remove {
    position: absolute;
    top: 6px;
    right: 6px;
    border: none;
    background: transparent;
    color: var(--fg-30);
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
    padding: 0 3px;
    opacity: 0;
    transition: opacity 0.12s, color 0.12s;
  }
  .card:hover .card-remove {
    opacity: 1;
  }
  .card-remove:hover {
    color: var(--text-primary);
  }
  .card-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.35;
    padding-right: 14px;
    word-break: break-word;
  }
  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
  .card-tag {
    font-size: 10px;
    color: var(--accent);
    background: var(--accent-bg);
    border-radius: 10px;
    padding: 1px 7px;
  }
  .card-progress {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 9px;
  }
  .card-progress-bar {
    flex: 1;
    height: 4px;
    background: var(--fg-10);
    border-radius: 2px;
    overflow: hidden;
  }
  .card-progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.2s;
  }
  .card-progress-count {
    font-size: 10px;
    color: var(--fg-30);
    font-family: var(--font-mono);
  }

  .placeholder {
    height: 2px;
    background: var(--accent);
    border-radius: 2px;
    margin: 0 0 8px;
  }

  .add-card {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    margin-top: 4px;
    padding: 8px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--fg-30);
    font-size: 12px;
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .add-card:hover {
    background: var(--bg-hover-folder);
    color: var(--text-primary);
  }

  /* Picker dodawania karty */
  .picker {
    margin-top: 4px;
    background: var(--bg-app);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 8px;
  }
  .picker-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    padding: 6px 8px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
    outline: none;
    margin-bottom: 6px;
  }
  .picker-input:focus {
    border-color: var(--accent);
  }
  .picker-list {
    max-height: 220px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .picker-item {
    text-align: left;
    border: none;
    background: transparent;
    color: var(--fg-40);
    font-size: 12px;
    font-family: var(--font-sans);
    padding: 6px 8px;
    border-radius: var(--radius-md);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .picker-item:hover {
    background: var(--bg-hover-subnote);
    color: var(--text-primary);
  }
  .picker-empty {
    padding: 6px 8px;
    font-size: 11px;
    font-style: italic;
    color: var(--fg-20);
  }
  .picker-create {
    text-align: left;
    border: none;
    border-top: 1px solid var(--line);
    margin-top: 4px;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
    font-family: var(--font-sans);
    padding: 7px 8px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .picker-create:hover {
    background: var(--accent-bg);
  }
  .picker-close {
    width: 100%;
    margin-top: 6px;
    border: none;
    background: transparent;
    color: var(--fg-30);
    font-size: 11.5px;
    font-family: var(--font-sans);
    padding: 5px;
    cursor: pointer;
    border-radius: var(--radius-md);
  }
  .picker-close:hover {
    color: var(--text-primary);
  }

  .add-column {
    width: 280px;
    flex-shrink: 0;
    align-self: flex-start;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 14px;
    border: 1px dashed var(--fg-18);
    border-radius: 10px;
    background: transparent;
    color: var(--fg-30);
    font-size: 12.5px;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
  }
  .add-column:hover {
    border-color: var(--accent-border);
    color: var(--accent);
  }
</style>

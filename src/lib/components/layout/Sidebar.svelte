<script lang="ts">
  import {
    appState,
    toggleFolder,
    selectNote,
    createNote,
    createFolder,
    renameNote,
    renameFolder,
    moveNote,
    togglePin,
    duplicateNote,
    deleteNote,
    deleteFolder,
    sortFoldersAZ,
    collapseAllFolders,
    pendingFolderRename,
    activeTagFilter,
    activeTimerNoteId,
    setTagFilter,
    clearTagFilter,
  } from "$lib/stores/app.store";
  import { openTimeLog } from "$lib/stores/timeLog.store";
  import { createWhiteboard } from "$lib/stores/whiteboard.store";
  import logoUrl from "$lib/assets/logo.png";
  import {
    searchActive,
    searchQuery,
    searchResults,
    searchLoading,
    searchIndexing,
    openSearch,
    closeSearch,
    runSearch,
  } from "$lib/stores/search.store";
  import { openContextMenu } from "$lib/stores/contextMenu.store";
  import { timeTrackingEnabled } from "$lib/stores/ui.store";
  import VaultSwitcher from "$lib/components/layout/VaultSwitcher.svelte";
  import type { ContextMenuItem, Folder, Note, SearchHit } from "$lib/types";

  // Foldery zawsze renderowane w kolejności `order`.
  let folders = $derived([...$appState.folders].sort((a, b) => a.order - b.order));
  // Notatki bez folderu (sekcja „No folder").
  let ungrouped = $derived(sortNotes($appState.notes.filter((n) => n.folderId === null)));

  // Przypięte notatki — płaska lista nad folderami (najnowsze na górze).
  let pinned = $derived(
    [...$appState.notes.filter((n) => n.pinned)].sort(
      (a, b) => b.updatedAt.getTime() - a.updatedAt.getTime(),
    ),
  );

  // Unikalne tagi z całego vaultu + licznik, alfabetycznie.
  let allTags = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const note of $appState.notes) {
      for (const tag of note.tags) counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
    return [...counts.entries()]
      .map(([tag, count]) => ({ tag, count }))
      .sort((a, b) => a.tag.toLowerCase().localeCompare(b.tag.toLowerCase()));
  });

  // Notatki dla aktywnego filtra po tagu.
  let tagFiltered = $derived(
    $activeTagFilter
      ? sortNotes($appState.notes.filter((n) => n.tags.includes($activeTagFilter as string)))
      : [],
  );

  // Sekcja Tags — zwijana (stan lokalny UI).
  let tagsExpanded = $state(true);

  function sortNotes(list: Note[]): Note[] {
    return [...list].sort((a, b) => Number(b.pinned) - Number(a.pinned));
  }

  function notesOf(folderId: string): Note[] {
    return sortNotes($appState.notes.filter((n) => n.folderId === folderId));
  }

  function folderName(id: string | null): string | null {
    if (id === null) return null;
    return $appState.folders.find((f) => f.id === id)?.name ?? null;
  }

  function handleNewNote() {
    selectNote(createNote(null));
  }

  function handleNewFolder() {
    const id = createFolder("New folder");
    pendingFolderRename.set(id);
  }

  // ── Tagi / wyszukiwanie ──────────────────────────────────────────────────
  function pickTag(tag: string) {
    closeSearch(); // wyjdź z trybu search jeśli był aktywny
    setTagFilter(tag);
  }

  function startSearch() {
    clearTagFilter();
    openSearch();
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeSearch();
    }
  }

  function openHit(hit: SearchHit) {
    selectNote(hit.id);
    closeSearch();
  }

  function focusInput(node: HTMLInputElement) {
    node.focus();
  }

  // ── Inline rename ──────────────────────────────────────────────────────────
  let renaming = $state<{ kind: "folder" | "note"; id: string } | null>(null);
  let renameValue = $state("");

  function startRename(kind: "folder" | "note", id: string, current: string) {
    renaming = { kind, id };
    renameValue = current;
  }

  // Zewnętrzne żądanie rename folderu (np. po „New folder" lub Ctrl+Shift+N).
  $effect(() => {
    const id = $pendingFolderRename;
    if (!id) return;
    const f = $appState.folders.find((x) => x.id === id);
    if (f) startRename("folder", f.id, f.name);
    pendingFolderRename.set(null);
  });

  function commitRename() {
    if (!renaming) return;
    const value = renameValue.trim();
    if (value) {
      if (renaming.kind === "folder") renameFolder(renaming.id, value);
      else renameNote(renaming.id, value);
    }
    renaming = null;
  }

  function cancelRename() {
    renaming = null;
  }

  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitRename();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelRename();
    }
  }

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  // ── Context menus ────────────────────────────────────────────────────────────
  function openFolderMenu(e: MouseEvent, folder: Folder) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: "Rename", action: () => startRename("folder", folder.id, folder.name) },
      { label: "New note here", action: () => selectNote(createNote(folder.id)) },
      { separator: true },
      { label: "Delete folder", danger: true, action: () => deleteFolder(folder.id) },
    ];
    openContextMenu(e.clientX, e.clientY, items);
  }

  function openNoteMenu(e: MouseEvent, note: Note) {
    e.preventDefault();
    e.stopPropagation();
    const x = e.clientX;
    const y = e.clientY;
    const items: ContextMenuItem[] = [
      { label: "Rename", action: () => startRename("note", note.id, note.title) },
      { label: "Move to folder", action: () => openMoveMenu(note, x, y) },
      { label: note.pinned ? "Unpin" : "Pin", action: () => togglePin(note.id) },
      { label: "Duplicate", action: () => void duplicateNote(note.id) },
      { label: "View time log", action: () => openTimeLog(note.id) },
      { label: "New whiteboard", action: () => void createWhiteboard(note.id) },
      { separator: true },
      { label: "Delete", danger: true, action: () => void deleteNote(note.id) },
    ];
    openContextMenu(x, y, items);
  }

  function openMoveMenu(note: Note, x: number, y: number) {
    const items: ContextMenuItem[] = [
      { label: "No folder", disabled: note.folderId === null, action: () => moveNote(note.id, null) },
      ...folders.map((f) => ({
        label: f.name,
        disabled: f.id === note.folderId,
        action: () => moveNote(note.id, f.id),
      })),
    ];
    openContextMenu(x, y, items);
  }

  // Prawy klik na pustym obszarze Library (tylko sam kontener, nie dziecko).
  function openSidebarMenu(e: MouseEvent) {
    if (e.target !== e.currentTarget) return;
    e.preventDefault();
    const items: ContextMenuItem[] = [
      { label: "New note (no folder)", action: handleNewNote },
      { label: "New folder", action: handleNewFolder },
      { separator: true },
      { label: "Sort folders A→Z", action: sortFoldersAZ },
      { label: "Collapse all folders", action: collapseAllFolders },
    ];
    openContextMenu(e.clientX, e.clientY, items);
  }
</script>

<nav class="nav">
  <header class="logo">
    <div class="logo-mark">
      <img class="logo-img" src={logoUrl} alt="Rune logo" width="10" height="20" />
      <span class="logo-text">Rune</span>
    </div>
  </header>

  <div class="section-label">
    <span>{$searchActive ? "Search" : "Library"}</span>
    {#if !$searchActive}
      {#if $searchIndexing}
        <span class="indexing" title="Building search index…">
          <svg class="spinner" width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" stroke-opacity="0.25" />
            <path d="M14 8A6 6 0 0 0 8 2" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
          </svg>
          Indexing…
        </span>
      {:else}
        <button class="search-btn" type="button" title="Search notes" aria-label="Search notes" onclick={startSearch}>
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <circle cx="7" cy="7" r="4.5" stroke="currentColor" stroke-width="1.5" />
            <path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </button>
      {/if}
    {/if}
  </div>

  {#if $searchActive}
    <!-- ── Tryb wyszukiwania ── -->
    <div class="search-bar">
      <svg class="search-bar-icon" width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <circle cx="7" cy="7" r="4.5" stroke="currentColor" stroke-width="1.5" />
        <path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
      <input
        class="search-input"
        type="text"
        placeholder="Search notes..."
        value={$searchQuery}
        oninput={(e) => runSearch(e.currentTarget.value)}
        onkeydown={onSearchKey}
        use:focusInput
      />
      <button class="search-close" type="button" title="Close search" aria-label="Close search" onclick={closeSearch}>×</button>
    </div>

    <div class="tree">
      {#if $searchQuery.trim() === ""}
        <div class="search-hint">Type to search titles, tags and content.</div>
      {:else if $searchResults.length === 0 && !$searchLoading}
        <div class="search-empty">No notes found for “{$searchQuery.trim()}”</div>
      {:else}
        {#each $searchResults as hit (hit.id)}
          <button
            class="subnote root result"
            class:active={hit.id === $appState.activeNoteId}
            type="button"
            onclick={() => openHit(hit)}
          >
            <div class="result-line">
              {#if hit.pinned}
                <svg class="pin" width="9" height="9" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                  <path d="M6 1.5L7.4 4.3L10.5 4.7L8.2 6.9L8.8 10L6 8.5L3.2 10L3.8 6.9L1.5 4.7L4.6 4.3L6 1.5Z" fill="currentColor" />
                </svg>
              {:else}
                <svg class="dot" width="4" height="4" viewBox="0 0 4 4" fill="none" aria-hidden="true">
                  <circle cx="2" cy="2" r="1.5" fill="currentColor" />
                </svg>
              {/if}
              <span class="subnote-title">{hit.title}</span>
            </div>
            {#if hit.snippet}
              <!-- Snippet jest zescapowany po stronie Rusta; jedyne znaczniki to <mark>. -->
              <span class="result-snippet">{@html hit.snippet}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {:else if $activeTagFilter}
    <!-- ── Widok filtra po tagu ── -->
    <div class="filter-chip-row">
      <span class="filter-chip">
        <span class="filter-hash">#</span>{$activeTagFilter}
        <button class="filter-x" type="button" title="Clear filter" aria-label="Clear tag filter" onclick={clearTagFilter}>×</button>
      </span>
    </div>
    <div class="tree">
      {#if tagFiltered.length === 0}
        <div class="search-empty">No notes with this tag.</div>
      {:else}
        {#each tagFiltered as note (note.id)}
          <button
            class="subnote root"
            class:active={note.id === $appState.activeNoteId}
            type="button"
            onclick={() => selectNote(note.id)}
            oncontextmenu={(e) => openNoteMenu(e, note)}
          >
            {#if note.pinned}
              <svg class="pin" width="9" height="9" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                <path d="M6 1.5L7.4 4.3L10.5 4.7L8.2 6.9L8.8 10L6 8.5L3.2 10L3.8 6.9L1.5 4.7L4.6 4.3L6 1.5Z" fill="currentColor" />
              </svg>
            {:else}
              <svg class="dot" width="4" height="4" viewBox="0 0 4 4" fill="none" aria-hidden="true">
                <circle cx="2" cy="2" r="1.5" fill="currentColor" />
              </svg>
            {/if}
            <span class="subnote-title">{note.title}</span>
            {#if $timeTrackingEnabled && $activeTimerNoteId === note.id}
              <span class="timer-dot" title="Timer running" aria-hidden="true"></span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {:else}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tree" oncontextmenu={openSidebarMenu}>
    {#if $appState.notes.length === 0 && folders.length === 0}
      <div class="empty-hint">
        No notes yet.<br />Right-click to get started.
      </div>
    {/if}

    <!-- Przypięte notatki -->
    {#if pinned.length > 0}
      <div class="pinned-section">
        <div class="mini-label">Pinned</div>
        {#each pinned as note (note.id)}
          <button
            class="subnote root"
            class:active={note.id === $appState.activeNoteId}
            type="button"
            onclick={() => selectNote(note.id)}
            oncontextmenu={(e) => openNoteMenu(e, note)}
          >
            <svg class="pin" width="9" height="9" viewBox="0 0 12 12" fill="none" aria-hidden="true">
              <path d="M6 1.5L7.4 4.3L10.5 4.7L8.2 6.9L8.8 10L6 8.5L3.2 10L3.8 6.9L1.5 4.7L4.6 4.3L6 1.5Z" fill="currentColor" />
            </svg>
            <span class="subnote-title">{note.title}</span>
            {#if $timeTrackingEnabled && $activeTimerNoteId === note.id}
              <span class="timer-dot" title="Timer running" aria-hidden="true"></span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}

    <!-- Notatki bez folderu -->
    {#if ungrouped.length > 0}
      <div class="ungrouped">
        {#each ungrouped as note (note.id)}
          {#if renaming?.kind === "note" && renaming.id === note.id}
            <div class="rename-row">
              <input
                class="rename-input"
                bind:value={renameValue}
                onkeydown={onRenameKey}
                onblur={commitRename}
                use:focusSelect
              />
            </div>
          {:else}
            <button
              class="subnote root"
              class:active={note.id === $appState.activeNoteId}
              type="button"
              onclick={() => selectNote(note.id)}
              oncontextmenu={(e) => openNoteMenu(e, note)}
            >
              {#if note.pinned}
                <svg class="pin" width="9" height="9" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                  <path d="M6 1.5L7.4 4.3L10.5 4.7L8.2 6.9L8.8 10L6 8.5L3.2 10L3.8 6.9L1.5 4.7L4.6 4.3L6 1.5Z" fill="currentColor" />
                </svg>
              {:else}
                <svg class="dot" width="4" height="4" viewBox="0 0 4 4" fill="none" aria-hidden="true">
                  <circle cx="2" cy="2" r="1.5" fill="currentColor" />
                </svg>
              {/if}
              <span class="subnote-title">{note.title}</span>
            {#if $timeTrackingEnabled && $activeTimerNoteId === note.id}
              <span class="timer-dot" title="Timer running" aria-hidden="true"></span>
            {/if}
            </button>
          {/if}
        {/each}
      </div>
    {/if}

    {#each folders as folder (folder.id)}
      {@const folderNotes = notesOf(folder.id)}
      <div class="folder-group">
        {#if renaming?.kind === "folder" && renaming.id === folder.id}
          <div class="rename-row">
            <input
              class="rename-input"
              bind:value={renameValue}
              onkeydown={onRenameKey}
              onblur={commitRename}
              use:focusSelect
            />
          </div>
        {:else}
          <button
            class="folder-row"
            class:active={folder.id === $appState.activeFolderId}
            type="button"
            onclick={() => toggleFolder(folder.id)}
            oncontextmenu={(e) => openFolderMenu(e, folder)}
          >
            <svg
              class="chevron"
              class:open={folder.isExpanded}
              width="9"
              height="9"
              viewBox="0 0 10 10"
              fill="none"
              aria-hidden="true"
            >
              <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
            </svg>

            <svg class="folder-icon" width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path
                d="M1.5 5.5C1.5 4.948 1.948 4.5 2.5 4.5H6.086C6.351 4.5 6.605 4.605 6.793 4.793L7.207 5.207C7.395 5.395 7.649 5.5 7.914 5.5H13.5C14.052 5.5 14.5 5.948 14.5 6.5V12.5C14.5 13.052 14.052 13.5 13.5 13.5H2.5C1.948 13.5 1.5 13.052 1.5 12.5V5.5Z"
                stroke="currentColor"
                stroke-width="1.35"
                fill="none"
              />
            </svg>

            <span class="folder-label">{folder.name}</span>
            {#if folderNotes.length > 0}
              <span class="folder-count">{folderNotes.length}</span>
            {/if}
          </button>
        {/if}

        <div class="subnotes" class:open={folder.isExpanded}>
          <div class="subnotes-inner">
            {#if folderNotes.length === 0}
              <div class="empty-folder">Empty folder</div>
            {:else}
              {#each folderNotes as note (note.id)}
                {#if renaming?.kind === "note" && renaming.id === note.id}
                  <div class="rename-row subnote-rename">
                    <input
                      class="rename-input"
                      bind:value={renameValue}
                      onkeydown={onRenameKey}
                      onblur={commitRename}
                      use:focusSelect
                    />
                  </div>
                {:else}
                  <button
                    class="subnote"
                    class:active={note.id === $appState.activeNoteId}
                    type="button"
                    onclick={() => selectNote(note.id)}
                    oncontextmenu={(e) => openNoteMenu(e, note)}
                  >
                    {#if note.pinned}
                      <svg class="pin" width="9" height="9" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                        <path d="M6 1.5L7.4 4.3L10.5 4.7L8.2 6.9L8.8 10L6 8.5L3.2 10L3.8 6.9L1.5 4.7L4.6 4.3L6 1.5Z" fill="currentColor" />
                      </svg>
                    {:else}
                      <svg class="dot" width="4" height="4" viewBox="0 0 4 4" fill="none" aria-hidden="true">
                        <circle cx="2" cy="2" r="1.5" fill="currentColor" />
                      </svg>
                    {/if}
                    <span class="subnote-title">{note.title}</span>
            {#if $timeTrackingEnabled && $activeTimerNoteId === note.id}
              <span class="timer-dot" title="Timer running" aria-hidden="true"></span>
            {/if}
                  </button>
                {/if}
              {/each}
            {/if}
          </div>
        </div>
      </div>
    {/each}

    <!-- Tagi -->
    {#if allTags.length > 0}
      <div class="tags-section">
        <button class="tags-header" type="button" onclick={() => (tagsExpanded = !tagsExpanded)}>
          <svg class="chevron" class:open={tagsExpanded} width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
            <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <span class="tags-title">Tags</span>
        </button>
        {#if tagsExpanded}
          <div class="tags-list">
            {#each allTags as { tag, count } (tag)}
              <button class="tag-row" type="button" onclick={() => pickTag(tag)}>
                <span class="tag-hash">#</span>
                <span class="tag-name">{tag}</span>
                <span class="tag-count">{count}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
  {/if}

  <VaultSwitcher />

  <footer class="bottom">
    <div class="bottom-actions">
      <button
        class="ghost-btn"
        type="button"
        title="New folder (Ctrl+Shift+N)"
        aria-label="New folder"
        onclick={handleNewFolder}
      >
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M1.5 5.5C1.5 4.948 1.948 4.5 2.5 4.5H6.086C6.351 4.5 6.605 4.605 6.793 4.793L7.207 5.207C7.395 5.395 7.649 5.5 7.914 5.5H13.5C14.052 5.5 14.5 5.948 14.5 6.5V12.5C14.5 13.052 14.052 13.5 13.5 13.5H2.5C1.948 13.5 1.5 13.052 1.5 12.5V5.5Z"
            stroke="currentColor"
            stroke-width="1.35"
            fill="none"
          />
          <path d="M8 8V11M6.5 9.5H9.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        </svg>
      </button>
      <button class="new-note" type="button" title="New note (Ctrl+N)" onclick={handleNewNote}>
        <svg width="10" height="10" viewBox="0 0 12 12" fill="none" aria-hidden="true">
          <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
        New
      </button>
    </div>
  </footer>
</nav>

<style>
  .nav {
    width: var(--nav-width);
    height: 100%;
    background: var(--bg-nav);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
  }

  .logo {
    height: var(--topbar-height);
    padding: 0 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }
  .logo-mark {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .logo-img {
    width: auto;
    height: 20px;
    display: block;
    flex-shrink: 0;
    /* Keeps the transparent silver rune legible on light themes without a background box */
    filter: drop-shadow(0 0 0.5px rgba(0, 0, 0, 0.55));
  }
  .logo-text {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.3px;
    color: var(--text-logo);
  }

  .section-label {
    padding: 16px 10px 6px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .section-label span {
    font-size: 9px;
    font-weight: 500;
    color: var(--fg-18);
    letter-spacing: 1.1px;
    text-transform: uppercase;
  }
  .search-btn {
    background: none;
    border: none;
    padding: 3px;
    border-radius: var(--radius-md);
    color: var(--fg-26);
    display: flex;
    align-items: center;
    cursor: pointer;
  }
  .search-btn:hover {
    color: var(--accent);
  }
  .indexing {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--fg-30);
  }
  .spinner {
    color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Pasek wyszukiwania */
  .search-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 10px 6px;
    padding: 5px 8px;
    background: var(--bg-app);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
  }
  .search-bar-icon {
    flex-shrink: 0;
    color: var(--fg-26);
  }
  .search-input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: var(--font-sans);
  }
  .search-input::placeholder {
    color: var(--fg-22);
  }
  .search-close {
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--fg-30);
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
    padding: 0 2px;
  }
  .search-close:hover {
    color: var(--text-primary);
  }
  .search-hint,
  .search-empty {
    margin-top: 18px;
    text-align: center;
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--fg-30);
    padding: 0 14px;
  }

  /* Wynik wyszukiwania (z fragmentem treści) */
  .result {
    flex-direction: column;
    align-items: stretch;
    gap: 3px;
  }
  .result-line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .result-snippet {
    font-size: 10.5px;
    color: var(--fg-22);
    line-height: 1.4;
    padding-left: 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Podświetlenie dopasowanych słów w snippetcie (FTS5 <mark>). */
  .result-snippet :global(mark) {
    background: rgba(var(--accent-rgb), 0.3);
    color: var(--text-primary);
    border-radius: 2px;
    padding: 0 1px;
  }

  /* Chip aktywnego filtra po tagu */
  .filter-chip-row {
    padding: 0 12px 6px;
  }
  .filter-chip {
    display: inline-flex;
    align-items: center;
    gap: 1px;
    background: rgba(92, 78, 232, 0.15);
    border: 1px solid rgba(92, 78, 232, 0.3);
    color: #a89cf0;
    padding: 3px 8px 3px 9px;
    border-radius: 12px;
    font-size: 12px;
  }
  .filter-hash {
    opacity: 0.6;
    margin-right: 1px;
  }
  .filter-x {
    border: none;
    background: transparent;
    color: #a89cf0;
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    padding: 0 0 0 4px;
  }
  .filter-x:hover {
    color: #fff;
  }

  /* Sekcja Pinned */
  .pinned-section {
    margin-bottom: 6px;
  }
  .mini-label {
    padding: 4px 8px 3px;
    font-size: 9px;
    font-weight: 500;
    color: var(--fg-18);
    letter-spacing: 1px;
    text-transform: uppercase;
  }

  /* Sekcja Tags */
  .tags-section {
    margin-top: 10px;
    border-top: 1px solid var(--line);
    padding-top: 6px;
  }
  .tags-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 8px 4px 6px;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--fg-20);
  }
  .tags-header:hover {
    color: var(--fg-40);
  }
  .tags-title {
    font-size: 9px;
    font-weight: 500;
    color: var(--fg-18);
    letter-spacing: 1.1px;
    text-transform: uppercase;
  }
  .tags-list {
    padding-left: 6px;
  }
  .tag-row {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 3px 8px;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }
  .tag-row:hover {
    background: var(--bg-hover-subnote);
  }
  .tag-hash {
    color: var(--accent);
    font-size: 11px;
    opacity: 0.7;
  }
  .tag-name {
    flex: 1;
    font-size: 12px;
    color: var(--fg-36);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag-count {
    font-size: 10px;
    color: var(--fg-20);
    font-family: var(--font-mono);
  }

  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 2px 6px;
  }

  .empty-hint {
    margin-top: 24px;
    text-align: center;
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--fg-30);
    pointer-events: none;
  }

  .ungrouped {
    margin-bottom: 4px;
  }
  .subnote.root {
    padding-left: 8px;
  }

  .folder-row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 5px 8px 5px 6px;
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    cursor: pointer;
    margin-bottom: 1px;
    transition: background 0.12s;
    color: var(--fg-20);
    text-align: left;
    font-family: var(--font-sans);
  }
  .folder-row:hover {
    background: var(--bg-hover-folder);
  }
  .folder-row.active {
    background: var(--bg-active-folder);
    color: var(--fg-50);
  }

  .chevron {
    flex-shrink: 0;
    transition: transform 0.15s;
    transform: rotate(0deg);
  }
  .chevron.open {
    transform: rotate(90deg);
  }

  .folder-icon {
    flex-shrink: 0;
    color: var(--fg-26);
  }
  .folder-row.active .folder-icon {
    color: var(--accent);
  }

  .folder-label {
    font-size: 12.5px;
    color: var(--fg-36);
    font-weight: 400;
    flex: 1;
  }
  .folder-row.active .folder-label {
    color: var(--text-primary);
    font-weight: 500;
  }

  .folder-count {
    font-size: 10px;
    color: var(--fg-20);
    font-family: var(--font-mono);
  }

  /* Inline rename */
  .rename-row {
    padding: 3px 6px;
    margin-bottom: 1px;
  }
  .subnote-rename {
    padding-left: 22px;
  }
  .rename-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-app);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    padding: 4px 8px;
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: var(--font-sans);
    outline: none;
  }

  .subnotes {
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.2s ease;
  }
  .subnotes.open {
    max-height: 1000px;
  }
  .subnotes-inner {
    padding-left: 22px;
    padding-bottom: 4px;
  }

  .empty-folder {
    padding: 4px 8px 4px 4px;
    font-size: 11px;
    font-style: italic;
    color: var(--fg-20);
  }

  .subnote {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 8px;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background 0.1s;
    text-align: left;
    font-family: var(--font-sans);
  }
  .subnote:hover {
    background: var(--bg-hover-subnote);
  }
  .subnote.active {
    background: var(--accent-bg);
  }
  .dot {
    flex-shrink: 0;
    color: var(--fg-20);
  }
  .pin {
    flex-shrink: 0;
    color: var(--accent);
  }
  .subnote.active .dot {
    color: var(--accent);
  }
  .subnote-title {
    font-size: 12px;
    color: var(--fg-30);
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subnote.active .subnote-title {
    color: var(--text-subnote-active);
    font-weight: 500;
  }
  /* Zielona kropka — notatka z biegnącym licznikiem czasu. */
  .timer-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4ead6b;
    flex-shrink: 0;
    margin-left: 2px;
    box-shadow: 0 0 0 2px rgba(78, 173, 107, 0.2);
  }

  .bottom {
    padding: 10px 8px;
    border-top: 1px solid var(--line);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-shrink: 0;
  }
  .bottom-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .ghost-btn {
    background: none;
    border: none;
    padding: 6px;
    border-radius: var(--radius-lg);
    color: var(--fg-28);
    display: flex;
    align-items: center;
  }
  .ghost-btn:hover {
    opacity: 0.7;
  }
  .new-note {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-lg);
    padding: 4px 10px;
    color: var(--accent);
    font-size: 11.5px;
    font-family: var(--font-sans);
    font-weight: 500;
  }
  .new-note:hover {
    opacity: 0.7;
  }
</style>

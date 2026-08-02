<script lang="ts">
  import Sidebar from "./Sidebar.svelte";
  import TopBar from "./TopBar.svelte";
  import Editor from "./Editor.svelte";
  import KanbanView from "./KanbanView.svelte";
  import CalendarView from "$lib/components/CalendarView.svelte";
  import GraphView from "$lib/components/GraphView.svelte";
  import Settings from "./Settings.svelte";
  import TimeLogModal from "$lib/components/TimeLogModal.svelte";
  import { graphViewOpen } from "$lib/stores/graphView.store";
  import { settingsOpen } from "$lib/stores/settings.store";
  import { timeLogNoteId } from "$lib/stores/timeLog.store";
  import { focusMode, exitFocusMode } from "$lib/stores/ui.store";
  import { viewMode } from "$lib/stores/view.store";
</script>

{#if $viewMode === "kanban"}
  <!-- Widok Kanban: pełna szerokość, bez Sidebar; TopBar zostaje. -->
  <div class="kanban-shell">
    <div class="area-topbar">
      <TopBar />
    </div>
    <div class="area-kanban">
      <KanbanView />
    </div>
  </div>
{:else if $viewMode === "calendar"}
  <!-- Widok Kalendarza: pełna szerokość, bez Sidebar; TopBar zostaje. -->
  <div class="kanban-shell">
    <div class="area-topbar">
      <TopBar />
    </div>
    <div class="area-kanban">
      <CalendarView />
    </div>
  </div>
{:else}
  <div class="app" class:focus={$focusMode}>
    <div class="area-nav">
      <Sidebar />
    </div>
    <div class="area-topbar">
      <TopBar />
    </div>
    <div class="area-editor">
      <Editor />
    </div>
  </div>
{/if}

{#if $focusMode}
  <!-- Minimalny przycisk wyjścia z focus mode — prawie niewidoczny, pojawia
       się przy najechaniu na prawy górny róg. -->
  <button
    class="focus-exit"
    type="button"
    title="Exit focus mode (Esc)"
    aria-label="Exit focus mode"
    onclick={exitFocusMode}
  >×</button>
{/if}

{#if $graphViewOpen}
  <GraphView />
{/if}

{#if $settingsOpen}
  <Settings />
{/if}

{#if $timeLogNoteId}
  <TimeLogModal />
{/if}

<style>
  .app {
    display: grid;
    height: 100vh;
    width: 100vw;
    grid-template:
      "nav topbar" var(--topbar-height)
      "nav editor" 1fr
      / var(--nav-width) 1fr;
    overflow: hidden;
    background: var(--bg-app);
    transition:
      grid-template-columns 0.2s ease,
      grid-template-rows 0.2s ease;
  }

  /* Focus mode — zwija kolumnę nawigacji i wiersz topbara do zera. */
  .app.focus {
    grid-template-columns: 0 1fr;
    grid-template-rows: 0 1fr;
  }

  .area-nav {
    grid-area: nav;
    grid-row: 1 / -1;
    /* min-width/height: 0 pozwala kolumnie zwinąć się do 0 w focus mode
       (domyślne min-width:auto blokuje zejście poniżej szerokości treści). */
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .area-topbar {
    grid-area: topbar;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  /* Ledwo widoczny przycisk wyjścia w prawym górnym rogu, wyraźny przy hover. */
  .focus-exit {
    position: fixed;
    top: 10px;
    right: 14px;
    z-index: 4000;
    width: 30px;
    height: 30px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--fg-20);
    font-size: 20px;
    line-height: 1;
    opacity: 0.15;
    transition: opacity 0.2s ease, color 0.12s, border-color 0.12s;
  }
  .focus-exit:hover {
    opacity: 1;
    color: var(--text-primary);
    border-color: var(--accent-border);
    background: var(--bg-nav);
  }

  .area-editor {
    grid-area: editor;
    min-height: 0;
    min-width: 0;
    display: flex;
  }

  /* Widok Kanban — TopBar na górze, tablica wypełnia resztę. */
  .kanban-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--bg-app);
  }
  .kanban-shell .area-topbar {
    height: var(--topbar-height);
    flex-shrink: 0;
  }
  .area-kanban {
    flex: 1;
    min-height: 0;
    display: flex;
  }
</style>

<script lang="ts">
  import { appState, selectNote } from "$lib/stores/app.store";
  import { notesApi } from "$lib/api/notes";
  import type { BacklinkHit } from "$lib/types";

  interface Props {
    noteId: string;
    title: string;
  }

  let { noteId, title }: Props = $props();

  let backlinks = $state<BacklinkHit[]>([]);
  let collapsed = $state(false);

  // Backlinki zależą tylko od tytułu bieżącej notatki (kto linkuje DO niej).
  // Debounce, bo każde pobranie deszyfruje wszystkie notatki po stronie Rusta.
  $effect(() => {
    const t = title.trim();
    const currentId = noteId;
    if (!t) {
      backlinks = [];
      return;
    }
    let cancelled = false;
    const timer = setTimeout(() => {
      void (async () => {
        let hits: BacklinkHit[];
        try {
          hits = await notesApi.findBacklinks(t);
        } catch {
          hits = clientBacklinks(t, currentId);
        }
        if (!cancelled) backlinks = hits.filter((h) => h.id !== currentId);
      })();
    }, 350);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });

  /** Fallback bez Tauri — przeszukuje notatki z załadowaną treścią. */
  function clientBacklinks(target: string, currentId: string): BacklinkHit[] {
    const low = target.toLowerCase();
    const out: BacklinkHit[] = [];
    for (const n of $appState.notes) {
      if (n.id === currentId || !n.content) continue;
      let ctx: string | null = null;
      for (const line of n.content.split("\n")) {
        const matches = [...line.matchAll(/\[\[([^\]]+)\]\]/g)];
        if (matches.some((m) => m[1].trim().toLowerCase() === low)) {
          ctx = line.trim().slice(0, 120);
          break;
        }
      }
      if (ctx !== null) {
        out.push({
          id: n.id,
          title: n.title,
          folderId: n.folderId,
          pinned: n.pinned,
          context: ctx,
        });
      }
    }
    return out;
  }
</script>

{#if backlinks.length > 0}
  <div class="backlinks">
    <button class="bl-header" type="button" onclick={() => (collapsed = !collapsed)}>
      <svg class="bl-chevron" class:open={!collapsed} width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
        <path d="M3 2L7 5L3 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <span class="bl-label">Linked mentions ({backlinks.length})</span>
    </button>

    {#if !collapsed}
      <div class="bl-list">
        {#each backlinks as bl (bl.id)}
          <button class="bl-item" type="button" onclick={() => selectNote(bl.id)}>
            <span class="bl-title">{bl.title}</span>
            <span class="bl-ctx">{bl.context}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .backlinks {
    flex-shrink: 0;
    border-top: 1px solid var(--line);
    background: var(--bg-app);
    max-height: 28vh;
    overflow-y: auto;
    padding: 8px 40px 14px;
  }
  .bl-header {
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
  .bl-header:hover {
    color: var(--fg-50);
  }
  .bl-chevron {
    transition: transform 0.15s;
  }
  .bl-chevron.open {
    transform: rotate(90deg);
  }
  .bl-label {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.4px;
    text-transform: uppercase;
  }
  .bl-list {
    max-width: var(--editor-max);
    margin: 6px auto 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .bl-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    text-align: left;
    padding: 7px 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--bg-nav);
    cursor: pointer;
    transition: border-color 0.12s;
  }
  .bl-item:hover {
    border-color: var(--accent-border);
  }
  .bl-title {
    font-size: 12.5px;
    color: var(--text-primary);
    font-weight: 500;
  }
  .bl-ctx {
    font-size: 11px;
    color: var(--fg-30);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

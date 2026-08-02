<script lang="ts">
  import { onMount } from "svelte";
  import {
    timeEntries,
    refreshTimeEntries,
    startTimer,
    stopTimer,
    deleteTimeEntry,
    entriesForNote,
    totalMsForNote,
    runningEntryForNote,
    formatHMS,
    formatDuration,
  } from "$lib/stores/timeTracking.store";

  interface Props {
    noteId: string;
  }
  let { noteId }: Props = $props();

  let historyOpen = $state(false);
  let now = $state(Date.now());

  onMount(() => {
    void refreshTimeEntries();
  });

  let running = $derived(runningEntryForNote($timeEntries, noteId));
  let isRunning = $derived(running !== null);
  let total = $derived(totalMsForNote($timeEntries, noteId));
  let sessions = $derived(
    entriesForNote($timeEntries, noteId).filter((e) => e.endedAt !== null),
  );

  // Bieżąca sesja: tyka co sekundę, dopóki licznik biegnie.
  let elapsed = $derived(running ? Math.max(0, now - running.startedAt) : 0);

  $effect(() => {
    if (!isRunning) return;
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  function toggle() {
    if (isRunning) void stopTimer(noteId);
    else void startTimer(noteId);
  }

  function sessionDate(ms: number): string {
    return new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
</script>

<div class="tracker">
  <div class="tracker-row">
    <div class="timer" class:running={isRunning}>
      <span class="led" aria-hidden="true"></span>
      <span class="clock">{formatHMS(elapsed)}</span>
      <button class="toggle" class:stop={isRunning} type="button" onclick={toggle}>
        {isRunning ? "Stop" : "Start"}
      </button>
    </div>

    <span class="total">Total: <strong>{total > 0 ? formatDuration(total) : "—"}</strong></span>

    <button
      class="history-btn"
      class:open={historyOpen}
      type="button"
      onclick={() => (historyOpen = !historyOpen)}
      disabled={sessions.length === 0}
    >
      History
      <svg class="caret" width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
        <path d="M2 3.5L5 6.5L8 3.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  </div>

  {#if historyOpen && sessions.length > 0}
    <div class="history">
      {#each sessions.slice(0, 10) as entry (entry.id)}
        <div class="hist-row">
          <span class="hist-date">{sessionDate(entry.startedAt)}</span>
          <span class="hist-dur">{formatDuration(entry.durationMs)}</span>
          <button
            class="hist-del"
            type="button"
            title="Delete entry"
            aria-label="Delete time entry"
            onclick={() => void deleteTimeEntry(entry.id)}
          >×</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tracker {
    flex-shrink: 0;
    border-top: 1px solid var(--line);
    background: var(--bg-app);
    padding: 8px 40px 12px;
  }
  .tracker-row {
    display: flex;
    align-items: center;
    gap: 16px;
    max-width: var(--editor-max);
    margin: 0 auto;
  }

  .timer {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 4px 6px 4px 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--bg-nav);
  }
  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--fg-20);
    flex-shrink: 0;
  }
  .timer.running .led {
    background: #4ead6b;
    box-shadow: 0 0 0 3px rgba(78, 173, 107, 0.18);
  }
  .clock {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    min-width: 66px;
  }
  .toggle {
    border: none;
    border-radius: var(--radius-sm);
    padding: 3px 11px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    cursor: pointer;
    background: var(--accent-bg);
    color: var(--accent);
    letter-spacing: 0.4px;
  }
  .toggle:hover {
    background: var(--accent-border);
  }
  .toggle.stop {
    background: rgba(215, 107, 107, 0.16);
    color: #d76b6b;
  }
  .toggle.stop:hover {
    background: rgba(215, 107, 107, 0.26);
  }

  .total {
    font-size: 12px;
    color: var(--fg-36);
  }
  .total strong {
    color: var(--text-primary);
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .history-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--fg-36);
    border-radius: var(--radius-md);
    padding: 4px 9px;
    font-size: 11.5px;
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .history-btn:hover:not(:disabled) {
    color: var(--text-primary);
    border-color: var(--accent-border);
  }
  .history-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .caret {
    transition: transform 0.15s;
  }
  .history-btn.open .caret {
    transform: rotate(180deg);
  }

  .history {
    max-width: var(--editor-max);
    margin: 8px auto 0;
    max-height: 180px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--bg-nav);
  }
  .hist-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line);
  }
  .hist-row:last-child {
    border-bottom: none;
  }
  .hist-date {
    font-size: 11.5px;
    color: var(--fg-40);
    font-family: var(--font-mono);
    min-width: 52px;
  }
  .hist-dur {
    flex: 1;
    font-size: 11.5px;
    color: var(--text-primary);
  }
  .hist-del {
    border: none;
    background: transparent;
    color: var(--fg-30);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    padding: 0 3px;
  }
  .hist-del:hover {
    color: #d76b6b;
  }
</style>

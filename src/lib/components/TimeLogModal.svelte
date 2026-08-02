<script lang="ts">
  import { onMount } from "svelte";
  import { appState } from "$lib/stores/app.store";
  import { timeLogNoteId, closeTimeLog } from "$lib/stores/timeLog.store";
  import {
    timeEntries,
    refreshTimeEntries,
    startTimer,
    entriesForNote,
    totalMsForNote,
    runningEntryForNote,
    formatDuration,
  } from "$lib/stores/timeTracking.store";

  let noteId = $derived($timeLogNoteId as string);

  let title = $derived(
    $appState.notes.find((n) => n.id === noteId)?.title ?? "Untitled",
  );

  onMount(() => {
    void refreshTimeEntries();
  });

  let total = $derived(totalMsForNote($timeEntries, noteId));
  let isRunning = $derived(runningEntryForNote($timeEntries, noteId) !== null);
  let sessions = $derived(
    entriesForNote($timeEntries, noteId)
      .filter((e) => e.endedAt !== null)
      .slice(0, 5),
  );

  function sessionLabel(startedAt: number): string {
    return new Date(startedAt).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }

  function onStart() {
    void startTimer(noteId);
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) closeTimeLog();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") closeTimeLog();
  }
</script>

<svelte:window onkeydown={onKey} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={onBackdrop}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Time log">
    <header class="modal-head">
      <div class="head-text">
        <span class="modal-label">Time log</span>
        <span class="modal-title">{title}</span>
      </div>
      <button class="close" type="button" title="Close" aria-label="Close" onclick={closeTimeLog}>×</button>
    </header>

    <div class="total-box">
      <span class="total-label">Total tracked</span>
      <span class="total-value">{total > 0 ? formatDuration(total) : "No time yet"}</span>
    </div>

    <div class="sessions">
      <div class="sessions-label">Recent sessions</div>
      {#if sessions.length === 0}
        <div class="sessions-empty">No sessions recorded.</div>
      {:else}
        {#each sessions as entry (entry.id)}
          <div class="session-row">
            <span class="session-date">{sessionLabel(entry.startedAt)}</span>
            <span class="session-dur">{formatDuration(entry.durationMs)}</span>
          </div>
        {/each}
      {/if}
    </div>

    {#if !isRunning}
      <button class="start-btn" type="button" onclick={onStart}>
        <span class="led" aria-hidden="true"></span>
        Start timer
      </button>
    {:else}
      <div class="running-note">
        <span class="led live" aria-hidden="true"></span>
        Timer running
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .modal {
    width: 100%;
    max-width: 360px;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
    padding: 18px;
  }
  .modal-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 14px;
  }
  .head-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .modal-label {
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 1.1px;
    text-transform: uppercase;
    color: var(--fg-30);
  }
  .modal-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--fg-30);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    padding: 0 2px;
  }
  .close:hover {
    color: var(--text-primary);
  }

  .total-box {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 12px 14px;
    background: var(--bg-nav);
    border: 1px solid var(--line);
    border-radius: 8px;
    margin-bottom: 14px;
  }
  .total-label {
    font-size: 12px;
    color: var(--fg-36);
  }
  .total-value {
    font-size: 16px;
    font-weight: 600;
    color: var(--accent);
    font-family: var(--font-mono);
  }

  .sessions {
    margin-bottom: 16px;
  }
  .sessions-label {
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 1.1px;
    text-transform: uppercase;
    color: var(--fg-22);
    margin-bottom: 6px;
  }
  .sessions-empty {
    font-size: 12px;
    font-style: italic;
    color: var(--fg-22);
    padding: 4px 0;
  }
  .session-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 2px;
    border-bottom: 1px solid var(--line);
  }
  .session-row:last-child {
    border-bottom: none;
  }
  .session-date {
    font-size: 12px;
    color: var(--fg-40);
    font-family: var(--font-mono);
  }
  .session-dur {
    font-size: 12px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .start-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 10px;
    border: 1px solid var(--accent-border);
    background: var(--accent-bg);
    color: var(--accent);
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .start-btn:hover {
    background: var(--accent-border);
  }
  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4ead6b;
    flex-shrink: 0;
  }
  .running-note {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 10px;
    font-size: 12.5px;
    color: var(--fg-40);
  }
  .led.live {
    box-shadow: 0 0 0 3px rgba(78, 173, 107, 0.18);
  }
</style>

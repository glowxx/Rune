<script lang="ts">
  import { onMount } from "svelte";
  import { selectNote, createNote } from "$lib/stores/app.store";
  import { showList } from "$lib/stores/view.store";
  import { calendarApi } from "$lib/api/calendar";
  import type { CalendarNote } from "$lib/types";

  let calendarNotes = $state<CalendarNote[]>([]);
  // Pierwszy dzień wyświetlanego miesiąca (godzina 12:00 — bezpieczny środek dnia).
  let currentMonth = $state(startOfMonth(new Date()));
  let selectedDay = $state<string | null>(null);

  const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const todayKey = ymd(new Date());

  onMount(() => {
    void load();
  });

  async function load() {
    try {
      calendarNotes = await calendarApi.getCalendarNotes();
    } catch {
      calendarNotes = []; // brak Tauri / vault zablokowany
    }
  }

  // ── Pomocnicze daty ────────────────────────────────────────────────────────
  function startOfMonth(d: Date): Date {
    return new Date(d.getFullYear(), d.getMonth(), 1, 12, 0, 0);
  }
  /** Lokalny `YYYY-MM-DD`. */
  function ymd(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }
  /** `YYYY-MM-DD` → Date (środek dnia, by uniknąć przeskoków stref). */
  function parseDay(key: string): Date {
    const [y, m, d] = key.split("-").map(Number);
    return new Date(y, m - 1, d, 12, 0, 0);
  }

  // ── Mapa dzień → notatki ───────────────────────────────────────────────────
  let notesByDay = $derived.by(() => {
    const map = new Map<string, CalendarNote[]>();
    const add = (key: string, note: CalendarNote) => {
      const list = map.get(key);
      if (list) {
        if (!list.some((n) => n.id === note.id)) list.push(note);
      } else {
        map.set(key, [note]);
      }
    };
    for (const note of calendarNotes) {
      const created = new Date(note.createdAt);
      if (!isNaN(created.getTime())) add(ymd(created), note);
      for (const d of note.dates) add(d, note);
    }
    return map;
  });

  // ── Siatka miesiąca (Mon–Sun, 6 rzędów = 42 komórki) ───────────────────────
  let monthLabel = $derived(
    currentMonth.toLocaleDateString(undefined, { month: "long", year: "numeric" }),
  );

  let grid = $derived.by(() => {
    const first = startOfMonth(currentMonth);
    const offset = (first.getDay() + 6) % 7; // ile dni wstecz do poniedziałku
    const start = new Date(first);
    start.setDate(first.getDate() - offset);

    const cells: { key: string; day: number; inMonth: boolean }[] = [];
    for (let i = 0; i < 42; i++) {
      const d = new Date(start);
      d.setDate(start.getDate() + i);
      cells.push({
        key: ymd(d),
        day: d.getDate(),
        inMonth: d.getMonth() === currentMonth.getMonth(),
      });
    }
    return cells;
  });

  // ── Nawigacja ──────────────────────────────────────────────────────────────
  function prevMonth() {
    currentMonth = new Date(currentMonth.getFullYear(), currentMonth.getMonth() - 1, 1, 12);
  }
  function nextMonth() {
    currentMonth = new Date(currentMonth.getFullYear(), currentMonth.getMonth() + 1, 1, 12);
  }
  function goToday() {
    currentMonth = startOfMonth(new Date());
    selectedDay = todayKey;
  }

  function notesFor(key: string): CalendarNote[] {
    return notesByDay.get(key) ?? [];
  }

  /** Kolor kropki: hue z pierwszego tagu, inaczej akcent 70%. */
  function dotColor(note: CalendarNote): string {
    if (note.tags.length === 0) return "rgba(var(--accent-rgb), 0.7)";
    let h = 0;
    for (const ch of note.tags[0]) h = (h * 31 + ch.charCodeAt(0)) % 360;
    return `hsl(${h} 58% 56%)`;
  }

  // ── Panel dnia ─────────────────────────────────────────────────────────────
  let selectedLabel = $derived(
    selectedDay
      ? parseDay(selectedDay).toLocaleDateString(undefined, {
          weekday: "long",
          day: "numeric",
          month: "long",
          year: "numeric",
        })
      : "",
  );
  let selectedNotes = $derived(selectedDay ? notesFor(selectedDay) : []);

  function openNote(note: CalendarNote) {
    selectNote(note.id);
    showList();
  }

  function newNoteForDay() {
    if (!selectedDay) return;
    const id = createNote(null, selectedDay, parseDay(selectedDay));
    selectNote(id);
    showList();
  }
</script>

<div class="calendar">
  <div class="cal-main">
    <header class="cal-head">
      <h2 class="cal-title">{monthLabel}</h2>
      <div class="cal-nav">
        <button class="nav-btn" type="button" title="Previous month" aria-label="Previous month" onclick={prevMonth}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M10 3L5 8L10 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <button class="today-btn" type="button" onclick={goToday}>Today</button>
        <button class="nav-btn" type="button" title="Next month" aria-label="Next month" onclick={nextMonth}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M6 3L11 8L6 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </div>
    </header>

    <div class="weekdays">
      {#each WEEKDAYS as wd (wd)}
        <div class="weekday">{wd}</div>
      {/each}
    </div>

    <div class="month-grid">
      {#each grid as cell (cell.key)}
        {@const dayNotes = notesFor(cell.key)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div
          class="day-cell"
          class:other-month={!cell.inMonth}
          class:today={cell.key === todayKey}
          class:selected={cell.key === selectedDay}
          role="button"
          tabindex="0"
          onclick={() => (selectedDay = cell.key)}
        >
          <span class="day-num">{cell.day}</span>
          {#if dayNotes.length > 0}
            <div class="day-dots">
              {#each dayNotes.slice(0, 3) as note (note.id)}
                <span class="dot" style={`background: ${dotColor(note)}`} title={note.title}></span>
              {/each}
              {#if dayNotes.length > 3}
                <span class="more">+{dayNotes.length - 3}</span>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  {#if selectedDay}
    <aside class="day-panel">
      <header class="panel-head">
        <span class="panel-date">{selectedLabel}</span>
        <button class="panel-close" type="button" title="Close" aria-label="Close day panel" onclick={() => (selectedDay = null)}>×</button>
      </header>

      <div class="panel-list">
        {#if selectedNotes.length === 0}
          <div class="panel-empty">No notes for this day.</div>
        {:else}
          {#each selectedNotes as note (note.id)}
            <button class="panel-note" type="button" onclick={() => openNote(note)}>
              <span class="panel-note-title">{note.title}</span>
              {#if note.tags.length > 0}
                <span class="panel-note-tags">
                  {#each note.tags as tag (tag)}
                    <span class="panel-tag">#{tag}</span>
                  {/each}
                </span>
              {/if}
              {#if note.hasTasks}
                <span class="panel-progress" title="{note.tasksDone} of {note.tasksTotal} tasks done">
                  <span class="panel-progress-bar">
                    <span
                      class="panel-progress-fill"
                      style="width: {note.tasksTotal > 0 ? Math.round((note.tasksDone / note.tasksTotal) * 100) : 0}%"
                    ></span>
                  </span>
                  <span class="panel-progress-count">{note.tasksDone}/{note.tasksTotal}</span>
                </span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <button class="new-day-note" type="button" onclick={newNoteForDay}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
          <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
        </svg>
        New note for this day
      </button>
    </aside>
  {/if}
</div>

<style>
  .calendar {
    display: flex;
    flex-direction: row;
    width: 100%;
    height: 100%;
    background: var(--bg-app);
    overflow: hidden;
  }
  .cal-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 20px 24px;
    box-sizing: border-box;
  }

  .cal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .cal-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .cal-nav {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--line);
    background: var(--bg-nav);
    color: var(--fg-36);
    border-radius: var(--radius-md);
    padding: 5px;
    cursor: pointer;
  }
  .nav-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent-border);
  }
  .today-btn {
    border: 1px solid var(--line);
    background: var(--bg-nav);
    color: var(--fg-40);
    border-radius: var(--radius-md);
    padding: 5px 12px;
    font-size: 12px;
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .today-btn:hover {
    color: var(--accent);
    border-color: var(--accent-border);
  }

  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 0;
    margin-bottom: 4px;
  }
  .weekday {
    padding: 4px 8px;
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--fg-30);
  }

  .month-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-auto-rows: 1fr;
    border-top: 1px solid var(--line);
    border-left: 1px solid var(--line);
  }
  .day-cell {
    min-height: 80px;
    border-right: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    padding: 6px 8px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: background 0.1s;
    outline: none;
  }
  .day-cell:hover {
    background: var(--bg-hover-folder);
  }
  .day-cell.other-month {
    opacity: 0.3;
  }
  .day-cell.today {
    border: 1px solid var(--accent);
  }
  .day-cell.selected {
    background: var(--accent-bg);
  }
  .day-num {
    font-size: 12.5px;
    color: var(--fg-44);
    font-variant-numeric: tabular-nums;
  }
  .day-cell.today .day-num {
    color: var(--accent);
    font-weight: 600;
  }
  .day-dots {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .more {
    font-size: 9.5px;
    color: var(--fg-30);
    font-family: var(--font-mono);
  }

  /* Panel dnia */
  .day-panel {
    width: 280px;
    flex-shrink: 0;
    border-left: 1px solid var(--line);
    background: var(--bg-nav);
    display: flex;
    flex-direction: column;
  }
  .panel-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    padding: 16px 16px 12px;
    border-bottom: 1px solid var(--line);
  }
  .panel-date {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.4;
  }
  .panel-close {
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: var(--fg-30);
    font-size: 17px;
    line-height: 1;
    cursor: pointer;
    padding: 0 2px;
  }
  .panel-close:hover {
    color: var(--text-primary);
  }
  .panel-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }
  .panel-empty {
    padding: 16px 8px;
    text-align: center;
    font-size: 12px;
    font-style: italic;
    color: var(--fg-22);
  }
  .panel-note {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    text-align: left;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px;
    margin-bottom: 8px;
    cursor: pointer;
    font-family: var(--font-sans);
    transition: border-color 0.12s;
  }
  .panel-note:hover {
    border-color: var(--accent-border);
  }
  .panel-note-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
    word-break: break-word;
  }
  .panel-note-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .panel-tag {
    font-size: 10px;
    color: var(--accent);
    background: var(--accent-bg);
    border-radius: 10px;
    padding: 1px 7px;
  }
  .panel-progress {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .panel-progress-bar {
    flex: 1;
    height: 4px;
    background: var(--fg-10);
    border-radius: 2px;
    overflow: hidden;
  }
  .panel-progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
  }
  .panel-progress-count {
    font-size: 10px;
    color: var(--fg-30);
    font-family: var(--font-mono);
  }
  .new-day-note {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    margin: 8px;
    padding: 10px;
    border: 1px solid var(--accent-border);
    background: var(--accent-bg);
    color: var(--accent);
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .new-day-note:hover {
    background: var(--accent-border);
  }
</style>

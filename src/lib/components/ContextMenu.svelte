<script lang="ts">
  import { contextMenu, closeContextMenu } from "$lib/stores/contextMenu.store";
  import type { ContextMenuItem } from "$lib/types";

  const MENU_WIDTH = 200;
  const ITEM_H = 33;
  const SEP_H = 9;

  let menuEl = $state<HTMLDivElement | null>(null);

  // Skorygowana pozycja, by menu nie wychodziło poza ekran.
  let adjusted = $derived.by(() => {
    const s = $contextMenu;
    if (!s.visible) return { x: s.x, y: s.y };

    const height = s.items.reduce(
      (acc, it) => acc + (it.separator ? SEP_H : ITEM_H),
      8, // pionowy padding menu
    );
    let x = s.x;
    let y = s.y;
    if (typeof window !== "undefined") {
      if (x + MENU_WIDTH > window.innerWidth - 8) x = window.innerWidth - MENU_WIDTH - 8;
      if (y + height > window.innerHeight - 8) y = window.innerHeight - height - 8;
    }
    return { x: Math.max(8, x), y: Math.max(8, y) };
  });

  function run(item: ContextMenuItem) {
    if (item.disabled || item.separator) return;
    const action = item.action;
    closeContextMenu();
    action?.();
  }

  // Zamknij przy kliknięciu POZA menu. Klik wewnątrz menu pomijamy, żeby
  // pointerdown (faza capture) nie usunął menu zanim zadziała onclick opcji.
  function onWindowPointerDown(e: PointerEvent) {
    if (!$contextMenu.visible) return;
    if (menuEl && e.target instanceof Node && menuEl.contains(e.target)) return;
    closeContextMenu();
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && $contextMenu.visible) closeContextMenu();
  }
</script>

<svelte:window
  onpointerdowncapture={onWindowPointerDown}
  onkeydown={onKeydown}
  onresize={closeContextMenu}
/>

{#if $contextMenu.visible}
  <div
    class="menu"
    bind:this={menuEl}
    style="left: {adjusted.x}px; top: {adjusted.y}px;"
    role="menu"
    tabindex="-1"
  >
    {#each $contextMenu.items as item, i (i)}
      {#if item.separator}
        <div class="sep" role="separator"></div>
      {:else}
        <button
          class="item"
          class:danger={item.danger}
          role="menuitem"
          disabled={item.disabled}
          onclick={() => run(item)}
        >
          {item.label}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    width: 200px;
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    padding: 4px;
    z-index: 9999;
    pointer-events: auto;
    transform-origin: top left;
    animation: pop 100ms ease;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: #e8e8e8;
    font-size: 13px;
    font-family: var(--font-sans);
    border-radius: 5px;
  }
  .item:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.06);
  }
  .item.danger {
    color: #e05c5c;
  }
  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .sep {
    height: 1px;
    background: rgba(255, 255, 255, 0.08);
    margin: 4px 0;
  }
</style>

<script lang="ts">
  import { toasts, dismissToast } from "$lib/stores/toast.store";
  import { fly } from "svelte/transition";
</script>

<div class="toast-stack" aria-live="polite">
  {#each $toasts as t (t.id)}
    <button
      class="toast {t.type}"
      type="button"
      onclick={() => dismissToast(t.id)}
      transition:fly={{ x: 40, duration: 200 }}
    >
      {t.message}
    </button>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 8000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-end;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    max-width: 320px;
    text-align: left;
    background: #232323;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-left: 3px solid #5c4ee8;
    color: #e8e8e8;
    font-size: 12.5px;
    font-family: var(--font-sans);
    padding: 10px 14px;
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    cursor: pointer;
  }
  .toast.success {
    border-left-color: #6fcf97;
  }
  .toast.error {
    border-left-color: #e05c5c;
  }
  .toast.info {
    border-left-color: #5c4ee8;
  }
</style>

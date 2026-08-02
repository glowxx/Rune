<script lang="ts">
  import { ARGON_PRESETS, DEFAULT_PRESET_INDEX } from "$lib/stores/vault.store";
  import { createAndEnterVault } from "$lib/stores/vaults.store";
  import { autoLockSuspended } from "$lib/stores/ui.store";
  import type { ArgonParams } from "$lib/types";

  let { onClose }: { onClose: () => void } = $props();

  let name = $state("");
  let password = $state("");
  let confirm = $state("");
  let presetIndex = $state(DEFAULT_PRESET_INDEX);
  let busy = $state(false);
  let error = $state<string | null>(null);

  let passwordError = $derived.by(() => {
    if (password.length === 0 && confirm.length === 0) return null;
    if (password.length < 8) return "Minimum 8 characters";
    if (confirm.length > 0 && password !== confirm) return "Passwords do not match";
    return null;
  });

  let canSubmit = $derived(
    !busy && name.trim().length > 0 && password.length >= 8 && password === confirm,
  );

  function paramSummary(p: ArgonParams): string {
    return `${Math.round(p.memory_kib / 1024)} MB · ${p.iterations} iter`;
  }

  async function handleCreate() {
    if (!canSubmit) return;
    error = null;
    busy = true;
    // Wstrzymaj auto-lock aktualnie otwartego vaultu na czas blokującego Argon2
    // (Maximum potrafi trwać sekundy) — inaczej mógłby wymusić lock w trakcie.
    autoLockSuspended.set(true);
    try {
      await createAndEnterVault(name.trim(), password, ARGON_PRESETS[presetIndex].params);
      // Faza zmieni się na „editor" — modal i tak zniknie.
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to create vault";
      busy = false;
    } finally {
      autoLockSuspended.set(false);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="head">
      <h2>New vault</h2>
      <p>A separate, independently encrypted notebook with its own password.</p>
    </div>

    <label class="field">
      <span>Vault name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" autofocus bind:value={name} placeholder="e.g. Work" />
    </label>

    <label class="field">
      <span>Password</span>
      <input
        type="password"
        autocomplete="new-password"
        bind:value={password}
        placeholder="min. 8 characters"
      />
    </label>
    <label class="field">
      <span>Confirm password</span>
      <input type="password" autocomplete="new-password" bind:value={confirm} />
    </label>

    <div class="presets">
      {#each ARGON_PRESETS as preset, i}
        <button
          type="button"
          class="preset"
          class:active={i === presetIndex}
          onclick={() => (presetIndex = i)}
        >
          <span class="preset-name">{preset.label}</span>
          <span class="preset-params">{paramSummary(preset.params)}</span>
        </button>
      {/each}
    </div>

    {#if passwordError}<p class="msg err">{passwordError}</p>{/if}
    {#if error}<p class="msg err">{error}</p>{/if}

    <div class="actions">
      <button class="ghost" type="button" onclick={onClose} disabled={busy}>Cancel</button>
      <button class="submit" disabled={!canSubmit} onclick={handleCreate}>
        {busy ? "Creating…" : "Create vault"}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 24px;
  }
  .modal {
    width: 100%;
    max-width: 420px;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .head h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .head p {
    margin-top: 4px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--fg-40);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field span {
    font-size: 12px;
    color: var(--fg-40);
  }
  .field input {
    width: 100%;
    box-sizing: border-box;
    background: rgba(var(--fg-rgb), 0.06);
    border: 1px solid var(--fg-10);
    border-radius: 8px;
    padding: 9px 13px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 13.5px;
  }
  .field input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .presets {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .preset {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    padding: 8px 10px;
    background: var(--bg-hover-subnote);
    border: 1px solid var(--fg-10);
    border-radius: 7px;
    color: var(--text-primary);
    font-family: var(--font-sans);
  }
  .preset.active {
    border-color: var(--accent);
    background: var(--accent-bg);
  }
  .preset-name {
    font-size: 12.5px;
    font-weight: 600;
  }
  .preset-params {
    font-size: 10.5px;
    color: var(--fg-40);
    font-family: var(--font-mono);
  }
  .msg.err {
    font-size: 12px;
    color: #e05c5c;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 2px;
  }
  .ghost {
    flex: 0 0 auto;
    padding: 10px 16px;
    border: 1px solid var(--fg-10);
    background: transparent;
    color: var(--text-primary);
    border-radius: 8px;
    font-family: var(--font-sans);
    font-size: 13px;
  }
  .ghost:hover:not(:disabled) {
    background: var(--bg-hover-folder);
  }
  .submit {
    flex: 1;
    padding: 10px;
    border: none;
    background: var(--accent);
    color: #fff;
    border-radius: 8px;
    font-size: 13.5px;
    font-weight: 500;
    font-family: var(--font-sans);
  }
  .submit:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .submit:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>

<script lang="ts">
  import {
    vaults,
    activeVaultId,
    refreshVaults,
    vaultName,
    switchToVault,
    removeVault,
    renameVaultById,
  } from "$lib/stores/vaults.store";
  import { openContextMenu } from "$lib/stores/contextMenu.store";
  import { showToast } from "$lib/stores/toast.store";
  import type { ContextMenuItem, VaultInfo } from "$lib/types";
  import NewVaultModal from "$lib/components/vault/NewVaultModal.svelte";

  let open = $state(false);
  let showNew = $state(false);
  let renamingId = $state<string | null>(null);
  let renameValue = $state("");

  let currentName = $derived(vaultName($vaults, $activeVaultId));

  function toggle() {
    open = !open;
    if (open) void refreshVaults();
  }

  function close() {
    open = false;
    renamingId = null;
  }

  function pick(v: VaultInfo) {
    if (v.id === $activeVaultId) return; // już aktywny
    close();
    void switchToVault(v.id);
  }

  function startRename(v: VaultInfo) {
    renamingId = v.id;
    renameValue = v.name;
  }

  async function commitRename() {
    const id = renamingId;
    const value = renameValue.trim();
    renamingId = null;
    if (!id || !value) return;
    try {
      await renameVaultById(id, value);
    } catch (e) {
      showToast(typeof e === "string" ? e : "Could not rename vault", "error");
    }
  }

  async function confirmDelete(v: VaultInfo) {
    if (v.id === $activeVaultId) {
      showToast("Switch to another vault before deleting this one", "error");
      return;
    }
    if (!confirm(`Delete vault “${v.name}” and all its data? This cannot be undone.`)) return;
    try {
      await removeVault(v.id);
      showToast(`Vault “${v.name}” deleted`, "success");
    } catch (e) {
      showToast(typeof e === "string" ? e : "Could not delete vault", "error");
    }
  }

  function openRowMenu(e: MouseEvent, v: VaultInfo) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: "Rename", action: () => startRename(v) },
      { separator: true },
      { label: "Delete", danger: true, disabled: v.id === $activeVaultId, action: () => void confirmDelete(v) },
    ];
    openContextMenu(e.clientX, e.clientY, items);
  }

  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void commitRename();
    } else if (e.key === "Escape") {
      e.preventDefault();
      renamingId = null;
    }
  }

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="switcher">
  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="backdrop" onclick={close}></div>
    <div class="popover" role="menu">
      <div class="pop-label">Vaults</div>
      {#each $vaults as v (v.id)}
        {#if renamingId === v.id}
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
          <div class="vault-row" class:active={v.id === $activeVaultId}>
            <button class="vault-pick" type="button" onclick={() => pick(v)}>
              {#if v.id === $activeVaultId}
                <svg class="ico check" width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path d="M3 8.5L6.5 12L13 4.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              {:else}
                <svg class="ico" width="12" height="12" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <rect x="5" y="11" width="14" height="9" rx="2" stroke="currentColor" stroke-width="1.7" />
                  <path d="M8 11V8C8 5.79 9.79 4 12 4C14.21 4 16 5.79 16 8V11" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
                </svg>
              {/if}
              <span class="vault-name">{v.name}</span>
            </button>
            <button class="more" type="button" title="Vault options" aria-label="Vault options" onclick={(e) => openRowMenu(e, v)}>
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                <circle cx="3.5" cy="8" r="1.2" fill="currentColor" />
                <circle cx="8" cy="8" r="1.2" fill="currentColor" />
                <circle cx="12.5" cy="8" r="1.2" fill="currentColor" />
              </svg>
            </button>
          </div>
        {/if}
      {/each}
      <button class="new-row" type="button" onclick={() => { close(); showNew = true; }}>
        <svg width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
          <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
        </svg>
        New vault
      </button>
    </div>
  {/if}

  <button class="trigger" type="button" onclick={toggle} title="Switch vault">
    <svg class="ico" width="12" height="12" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <rect x="5" y="11" width="14" height="9" rx="2" stroke="currentColor" stroke-width="1.7" />
      <path d="M8 11V8C8 5.79 9.79 4 12 4C14.21 4 16 5.79 16 8V11" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
    </svg>
    <span class="trigger-name">{currentName}</span>
    <svg class="chevron" class:up={open} width="9" height="9" viewBox="0 0 10 10" fill="none" aria-hidden="true">
      <path d="M2 3.5L5 6.5L8 3.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
</div>

{#if showNew}
  <NewVaultModal onClose={() => (showNew = false)} />
{/if}

<style>
  .switcher {
    position: relative;
    border-top: 1px solid var(--line);
    background: var(--bg-elevated);
    flex-shrink: 0;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 9px 12px;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--fg-36);
    font-family: var(--font-sans);
    text-align: left;
  }
  .trigger:hover {
    background: var(--bg-hover-folder);
  }
  .trigger .ico {
    flex-shrink: 0;
    color: var(--accent);
  }
  .trigger-name {
    flex: 1;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chevron {
    flex-shrink: 0;
    color: var(--fg-30);
    transition: transform 0.15s;
  }
  .chevron.up {
    transform: rotate(180deg);
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .popover {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 6px;
    right: 6px;
    z-index: 41;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 5px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .pop-label {
    padding: 5px 8px 4px;
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 1.1px;
    text-transform: uppercase;
    color: var(--fg-18);
  }
  .vault-row {
    display: flex;
    align-items: center;
    border-radius: var(--radius-md);
  }
  .vault-row:hover {
    background: var(--bg-hover-subnote);
  }
  .vault-row.active {
    background: var(--accent-bg);
  }
  .vault-pick {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 1;
    min-width: 0;
    padding: 6px 4px 6px 8px;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-family: var(--font-sans);
  }
  .vault-pick .ico {
    flex-shrink: 0;
    color: var(--fg-30);
  }
  .vault-pick .ico.check {
    color: var(--accent);
  }
  .vault-name {
    flex: 1;
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    flex-shrink: 0;
    padding: 6px 8px;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--fg-30);
    display: flex;
    align-items: center;
    border-radius: var(--radius-md);
  }
  .more:hover {
    color: var(--text-primary);
  }
  .new-row {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    margin-top: 2px;
    padding: 7px 8px;
    border: none;
    border-top: 1px solid var(--line);
    background: transparent;
    cursor: pointer;
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
  }
  .new-row:hover {
    background: var(--bg-hover-folder);
  }
  .rename-row {
    padding: 4px 6px;
  }
  .rename-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-app);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    padding: 5px 8px;
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: var(--font-sans);
    outline: none;
  }
</style>

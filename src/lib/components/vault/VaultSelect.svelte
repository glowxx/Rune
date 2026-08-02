<script lang="ts">
  import { vaults, switchToVault } from "$lib/stores/vaults.store";
  import NewVaultModal from "./NewVaultModal.svelte";
  import ImportVaultModal from "./ImportVaultModal.svelte";

  let showNew = $state(false);
  let showImport = $state(false);

  /** „Last opened" w czytelnej formie (data ISO → lokalna data). */
  function lastOpened(iso: string): string {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
  }

  // Najnowniej otwierane na górze.
  let sorted = $derived(
    [...$vaults].sort((a, b) => (a.lastOpened < b.lastOpened ? 1 : -1)),
  );
</script>

<div class="screen">
  <div class="inner">
    <header class="brand">
      <h1 class="brand-name">Rune</h1>
      <p class="brand-sub">Choose a vault</p>
    </header>

    <div class="list">
      {#each sorted as v (v.id)}
        <button class="vault-card" type="button" onclick={() => switchToVault(v.id)}>
          <span class="lock" aria-hidden="true">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <rect x="4.5" y="10.5" width="15" height="10" rx="2.2" stroke="var(--accent)" stroke-width="1.6" />
              <path d="M7.5 10.5V7.5C7.5 5.015 9.515 3 12 3C14.485 3 16.5 5.015 16.5 7.5V10.5" stroke="var(--accent)" stroke-width="1.6" stroke-linecap="round" />
            </svg>
          </span>
          <span class="vault-meta">
            <span class="vault-name">{v.name}</span>
            {#if lastOpened(v.lastOpened)}
              <span class="vault-date">Last opened {lastOpened(v.lastOpened)}</span>
            {/if}
          </span>
          <svg class="go" width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M6 3L11 8L6 13" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      {/each}
    </div>

    <div class="divider"></div>

    <button class="new-vault" type="button" onclick={() => (showNew = true)}>
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
        <path d="M6 2V10M2 6H10" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
      </svg>
      Create new vault
    </button>

    <button class="import-vault" type="button" onclick={() => (showImport = true)}>
      <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M8 2v7M8 9 5.3 6.3M8 9l2.7-2.7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
        <path d="M3 10v3.2a.8.8 0 0 0 .8.8h8.4a.8.8 0 0 0 .8-.8V10" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
      </svg>
      Import vault (.vault)
    </button>
  </div>
</div>

{#if showNew}
  <NewVaultModal onClose={() => (showNew = false)} />
{/if}
{#if showImport}
  <ImportVaultModal onClose={() => (showImport = false)} />
{/if}

<style>
  .screen {
    height: 100vh;
    width: 100vw;
    background: var(--bg-nav);
    overflow-y: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .inner {
    width: 100%;
    max-width: 400px;
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }
  .brand {
    text-align: center;
    margin-bottom: 28px;
  }
  .brand-name {
    font-size: 28px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: -0.5px;
  }
  .brand-sub {
    margin-top: 4px;
    font-size: 13px;
    color: var(--fg-40);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }
  .vault-card {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 14px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 10px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.12s, background 0.12s;
  }
  .vault-card:hover {
    border-color: var(--accent-border);
    background: var(--bg-active-folder);
  }
  .lock {
    flex-shrink: 0;
    display: flex;
  }
  .vault-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .vault-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .vault-date {
    font-size: 11px;
    color: var(--fg-40);
  }
  .go {
    flex-shrink: 0;
    color: var(--fg-30);
  }
  .divider {
    height: 1px;
    background: var(--line);
    margin: 6px 0 12px;
  }
  .new-vault {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 11px;
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
    border-radius: 10px;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .new-vault:hover {
    background: var(--accent-border);
  }
  .import-vault {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-top: 8px;
    padding: 11px;
    background: transparent;
    border: 1px solid var(--line);
    border-radius: 10px;
    color: var(--fg-50);
    font-size: 13px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
  }
  .import-vault:hover {
    border-color: var(--accent-border);
    color: var(--accent);
  }
</style>

<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { ARGON_PRESETS, DEFAULT_PRESET_INDEX } from "$lib/stores/vault.store";
  import { vaultsApi } from "$lib/api/vaults";
  import { vaultApi } from "$lib/api/vault";
  import { exportApi } from "$lib/api/export";
  import { refreshVaults } from "$lib/stores/vaults.store";
  import { autoLockSuspended } from "$lib/stores/ui.store";
  import { showToast } from "$lib/stores/toast.store";

  let { onClose }: { onClose: () => void } = $props();

  let name = $state("");
  let password = $state("");
  let confirm = $state("");
  let exportPassword = $state("");
  let filePath = $state<string | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  let passwordError = $derived.by(() => {
    if (password.length === 0 && confirm.length === 0) return null;
    if (password.length < 8) return "Minimum 8 characters";
    if (confirm.length > 0 && password !== confirm) return "Passwords do not match";
    return null;
  });

  let canSubmit = $derived(
    !busy &&
      !!filePath &&
      name.trim().length > 0 &&
      password.length >= 8 &&
      password === confirm &&
      exportPassword.length > 0,
  );

  async function pickFile() {
    try {
      const sel = await open({
        multiple: false,
        filters: [{ name: "Rune vault", extensions: ["vault"] }],
      });
      if (typeof sel === "string") filePath = sel;
    } catch {
      error = "File dialog unavailable";
    }
  }

  async function handleImport() {
    if (!canSubmit || !filePath) return;
    error = null;
    busy = true;
    autoLockSuspended.set(true);
    try {
      // 1. Utwórz nowy, pusty vault (od razu odblokowany po stronie Rusta).
      await vaultsApi.createNewVault(
        name.trim(),
        password,
        ARGON_PRESETS[DEFAULT_PRESET_INDEX].params,
      );
      // 2. Wmerguj zawartość pliku .vault do świeżego (pustego) vaultu.
      const r = await exportApi.importVault(exportPassword, filePath);
      // 3. Zablokuj nowy vault i wróć do ekranu startowego z odświeżoną listą.
      await vaultApi.lockVault().catch(() => {});
      await refreshVaults();
      showToast(
        `Imported vault “${name.trim()}” — ${r.noteCount} notes, ${r.folderCount} folders`,
        "success",
      );
      onClose();
    } catch (e) {
      // Nowy vault mógł już powstać (pusty) — użytkownik zobaczy go na liście
      // i może go usunąć. Najczęstsza przyczyna błędu to złe hasło eksportu.
      await vaultApi.lockVault().catch(() => {});
      await refreshVaults().catch(() => {});
      error = typeof e === "string" ? e : "Import failed — check the export password";
      busy = false;
    } finally {
      autoLockSuspended.set(false);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  function fileLabel(p: string): string {
    const parts = p.split(/[\\/]/);
    return parts[parts.length - 1] || p;
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="head">
      <h2>Import vault</h2>
      <p>Restore an encrypted <code>.vault</code> file into a new, separate vault.</p>
    </div>

    <div class="file-row">
      <button class="ghost" type="button" disabled={busy} onclick={pickFile}>
        Choose .vault file
      </button>
      <span class="file-name">{filePath ? fileLabel(filePath) : "No file selected"}</span>
    </div>

    <label class="field">
      <span>New vault name</span>
      <input type="text" bind:value={name} placeholder="e.g. Imported" disabled={busy} />
    </label>

    <label class="field">
      <span>Export password</span>
      <input
        type="password"
        autocomplete="off"
        bind:value={exportPassword}
        placeholder="Password the .vault was exported with"
        disabled={busy}
      />
    </label>

    <label class="field">
      <span>New vault password</span>
      <input
        type="password"
        autocomplete="new-password"
        bind:value={password}
        placeholder="min. 8 characters"
        disabled={busy}
      />
    </label>
    <label class="field">
      <span>Confirm new password</span>
      <input type="password" autocomplete="new-password" bind:value={confirm} disabled={busy} />
    </label>

    {#if passwordError}<p class="msg err">{passwordError}</p>{/if}
    {#if error}<p class="msg err">{error}</p>{/if}

    <div class="actions">
      <button class="ghost" type="button" onclick={onClose} disabled={busy}>Cancel</button>
      <button class="submit" disabled={!canSubmit} onclick={handleImport}>
        {busy ? "Importing…" : "Import vault"}
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
    gap: 12px;
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
  .head code {
    background: rgba(var(--fg-rgb), 0.08);
    padding: 1px 5px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.9em;
  }
  .file-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .file-name {
    font-size: 11.5px;
    color: var(--fg-40);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
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
    cursor: pointer;
  }
  .ghost:hover:not(:disabled) {
    background: var(--bg-hover-folder);
  }
  .ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
    cursor: pointer;
  }
  .submit:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .submit:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>

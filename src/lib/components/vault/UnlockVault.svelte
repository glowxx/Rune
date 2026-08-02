<script lang="ts">
  import { vaultApi } from "$lib/api/vault";
  import { setPhase } from "$lib/stores/vault.store";

  let password = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  // Gdy użytkownik cofnie się w trakcie odblokowywania, ignorujemy wynik po jego
  // rozwiązaniu — a jeśli hasło było poprawne (klucz już w Ruście), blokujemy vault
  // z powrotem, żeby nawigacja „wstecz" nie zostawiła po cichu odblokowanego vaultu.
  let cancelled = false;

  let canSubmit = $derived(!busy && password.length > 0);

  async function handleUnlock() {
    if (!canSubmit) return;
    error = null;
    busy = true;
    cancelled = false;
    const pw = password;
    try {
      const ok = await vaultApi.unlockVault(pw);
      if (cancelled) {
        // Wrócono do wyboru vaultu w trakcie — cofnij ewentualne odblokowanie.
        if (ok) await vaultApi.lockVault().catch(() => {});
        return;
      }
      if (ok) {
        password = "";
        setPhase("editor");
      } else {
        error = "Invalid password or corrupted vault";
        busy = false;
      }
    } catch (e) {
      if (cancelled) return;
      error = typeof e === "string" ? e : "Failed to unlock vault";
      busy = false;
    }
  }

  /** Wróć do ekranu wyboru vaultu bez odblokowywania (anuluje trwającą próbę). */
  function goBack() {
    cancelled = true;
    busy = false;
    setPhase("vault-select");
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleUnlock();
  }
</script>

<div class="screen">
  <button class="back" type="button" onclick={goBack} title="Switch vault">
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M10 3.5L5.5 8L10 12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
    Switch vault
  </button>
  <div class="inner">
    <header class="brand">
      <h1 class="brand-name">Rune</h1>
      <p class="brand-sub">Encrypted notebook</p>
    </header>

    <div class="card">
      <div class="lock" aria-hidden="true">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
          <rect
            x="4.5"
            y="10.5"
            width="15"
            height="10"
            rx="2.2"
            stroke="var(--accent)"
            stroke-width="1.6"
          />
          <path
            d="M7.5 10.5V7.5C7.5 5.015 9.515 3 12 3C14.485 3 16.5 5.015 16.5 7.5V10.5"
            stroke="var(--accent)"
            stroke-width="1.6"
            stroke-linecap="round"
          />
          <circle cx="12" cy="15" r="1.4" fill="var(--accent)" />
        </svg>
      </div>

      <label class="field">
        <span>Password</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="password"
          autocomplete="current-password"
          autofocus
          bind:value={password}
          onkeydown={onKeydown}
          disabled={busy}
        />
      </label>

      {#if error}<p class="msg err">{error}</p>{/if}

      <button class="submit" disabled={!canSubmit} onclick={handleUnlock}>
        {busy ? "Decrypting…" : "Unlock"}
      </button>

      <p class="warn">Forgot your password? Data cannot be recovered.</p>
    </div>
  </div>
</div>

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

  /* Subtelny powrót do wyboru vaultu (nie akcja główna). */
  .back {
    position: absolute;
    top: 18px;
    left: 18px;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--fg-40);
    font-size: 12.5px;
    font-family: var(--font-sans);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: color 0.12s, background 0.12s;
  }
  .back:hover {
    color: var(--text-primary);
    background: var(--bg-hover-folder);
  }

  .inner {
    width: 100%;
    max-width: 480px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .brand {
    text-align: center;
    margin-bottom: 32px;
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

  .card {
    width: 100%;
    max-width: 360px;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .lock {
    display: flex;
    justify-content: center;
    margin-bottom: 2px;
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
    padding: 10px 14px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 14px;
    text-align: center;
    transition: border-color 0.12s;
  }
  .field input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .msg {
    font-size: 12px;
    text-align: center;
  }
  .msg.err {
    color: #e05c5c;
  }

  .submit {
    width: 100%;
    padding: 11px;
    border: none;
    background: var(--accent);
    color: #fff;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    font-family: var(--font-sans);
    transition: background 0.12s;
  }
  .submit:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .submit:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .warn {
    font-size: 11px;
    color: var(--fg-30);
    text-align: center;
    margin-top: 4px;
  }
</style>

<script lang="ts">
  import { vaultApi } from "$lib/api/vault";
  import { setPhase, ARGON_PRESETS, DEFAULT_PRESET_INDEX } from "$lib/stores/vault.store";
  import type { ArgonParams } from "$lib/types";

  type Tab = "calculator" | "advanced";

  let tab = $state<Tab>("calculator");

  let presetIndex = $state(DEFAULT_PRESET_INDEX);
  let selectedPreset = $derived(ARGON_PRESETS[presetIndex]);

  let memoryKib = $state(ARGON_PRESETS[DEFAULT_PRESET_INDEX].params.memory_kib);
  let iterations = $state(ARGON_PRESETS[DEFAULT_PRESET_INDEX].params.iterations);
  let parallelism = $state(ARGON_PRESETS[DEFAULT_PRESET_INDEX].params.parallelism);

  let password = $state("");
  let confirm = $state("");

  let busy = $state(false);
  let error = $state<string | null>(null);

  /** Short parameter summary for a tile, e.g. "64 MB · 3 iter". */
  function paramSummary(p: ArgonParams): string {
    const mb = Math.round(p.memory_kib / 1024);
    return `${mb} MB · ${p.iterations} iter`;
  }

  function pickPreset(i: number) {
    presetIndex = i;
    const p = ARGON_PRESETS[i].params;
    memoryKib = p.memory_kib;
    iterations = p.iterations;
    parallelism = p.parallelism;
  }

  let passwordError = $derived.by(() => {
    if (password.length === 0 && confirm.length === 0) return null;
    if (password.length < 8) return "Minimum 8 characters";
    if (confirm.length > 0 && password !== confirm) return "Passwords do not match";
    return null;
  });

  let canSubmit = $derived(!busy && password.length >= 8 && password === confirm);

  function currentParams(): ArgonParams {
    return tab === "advanced"
      ? {
          memory_kib: Number(memoryKib),
          iterations: Number(iterations),
          parallelism: Number(parallelism),
        }
      : selectedPreset.params;
  }

  async function handleCreate() {
    if (!canSubmit) return;
    error = null;
    busy = true;
    const pw = password;
    try {
      await vaultApi.createVault(pw, currentParams());
      password = "";
      confirm = "";
      setPhase("editor");
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to create vault";
      busy = false;
    }
  }
</script>

<div class="screen">
  <div class="inner">
    <header class="brand">
      <h1 class="brand-name">Rune</h1>
      <p class="brand-sub">Encrypted notebook</p>
    </header>

    <div class="card">
      <div class="card-head">
        <h2>Create vault</h2>
        <p>Set a master password — it encrypts all notes locally.</p>
      </div>

      <div class="pills" role="tablist">
        <button
          class="pill"
          class:active={tab === "calculator"}
          role="tab"
          aria-selected={tab === "calculator"}
          onclick={() => (tab = "calculator")}
        >
          Calculator
        </button>
        <button
          class="pill"
          class:active={tab === "advanced"}
          role="tab"
          aria-selected={tab === "advanced"}
          onclick={() => (tab = "advanced")}
        >
          Advanced
        </button>
      </div>

      {#if tab === "calculator"}
        <div class="panel">
          <input
            class="slider"
            type="range"
            min="0"
            max="3"
            step="1"
            bind:value={presetIndex}
            oninput={() => pickPreset(presetIndex)}
            aria-label="Poziom kosztu"
          />

          <div class="grid">
            {#each ARGON_PRESETS as preset, i}
              <button
                type="button"
                class="tile"
                class:active={i === presetIndex}
                onclick={() => pickPreset(i)}
              >
                <span class="tile-name">{preset.label}</span>
                <span class="tile-params">{paramSummary(preset.params)}</span>
              </button>
            {/each}
          </div>

          <p class="estimate">
            Estimated unlock time: <strong>{selectedPreset.estimate}</strong>
          </p>
        </div>
      {:else}
        <div class="panel">
          <label class="field">
            <span>Memory (KiB)</span>
            <input type="number" min="8192" bind:value={memoryKib} />
          </label>
          <label class="field">
            <span>Iterations</span>
            <input type="number" min="1" bind:value={iterations} />
          </label>
          <label class="field">
            <span>Parallelism</span>
            <input type="number" min="1" max="16" bind:value={parallelism} />
          </label>
        </div>
      {/if}

      <div class="panel pw">
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
        {#if passwordError}<p class="msg err">{passwordError}</p>{/if}
        {#if error}<p class="msg err">{error}</p>{/if}
      </div>

      <button class="submit" disabled={!canSubmit} onclick={handleCreate}>
        {busy ? "Creating vault…" : "Create vault"}
      </button>

      {#if busy}
        <p class="hint">Argon2 key derivation may take a few seconds.</p>
      {/if}
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
    background: var(--bg-elevated);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .card-head h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .card-head p {
    margin-top: 4px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--fg-40);
  }

  .pills {
    display: flex;
    gap: 6px;
  }
  .pill {
    flex: 1;
    padding: 8px 10px;
    border: 1px solid var(--fg-10);
    background: transparent;
    color: var(--fg-50);
    font-size: 13px;
    font-family: var(--font-sans);
    border-radius: 8px;
    transition: all 0.12s;
  }
  .pill.active {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .slider {
    width: 100%;
    accent-color: var(--accent);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 3px;
    text-align: left;
    padding: 11px 12px;
    background: var(--bg-hover-subnote);
    border: 1px solid var(--fg-10);
    border-radius: 8px;
    color: var(--text-primary);
    transition: all 0.12s;
  }
  .tile.active {
    border-color: var(--accent);
    background: var(--accent-bg);
  }
  .tile-name {
    font-size: 13px;
    font-weight: 600;
  }
  .tile-params {
    font-size: 11px;
    color: var(--fg-40);
    font-family: var(--font-mono);
  }

  .estimate {
    font-size: 12px;
    color: var(--fg-40);
  }
  .estimate strong {
    color: var(--text-primary);
    font-weight: 600;
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
    transition: border-color 0.12s;
  }
  .field input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .pw {
    border-top: 1px solid var(--line);
    padding-top: 18px;
  }

  .msg {
    font-size: 12px;
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

  .hint {
    font-size: 12px;
    color: var(--fg-40);
    text-align: center;
  }
</style>

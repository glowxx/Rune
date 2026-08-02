<script lang="ts">
  import { onMount } from "svelte";
  import "./lib/styles/variables.css";
  import { vaultApi } from "$lib/api/vault";
  import { appPhase, setPhase } from "$lib/stores/vault.store";
  import {
    loadNotes,
    loadFolders,
    createNote,
    createFolder,
    selectNote,
    deselectNote,
    lockSession,
    pendingFolderRename,
  } from "$lib/stores/app.store";
  import {
    idleMinutes,
    captureProtection,
    focusMode,
    toggleFocusMode,
    exitFocusMode,
    autoLockSuspended,
    timeTrackingEnabled,
  } from "$lib/stores/ui.store";
  import { closeContextMenu } from "$lib/stores/contextMenu.store";
  import {
    searchActive,
    closeSearch,
    initSearchIndexEvents,
    rebuildSearchIndex,
  } from "$lib/stores/search.store";
  import { closeGraphView } from "$lib/stores/graphView.store";
  import { securityApi } from "$lib/api/security";
  import { attachmentsApi } from "$lib/api/attachments";
  import { backupApi } from "$lib/api/backup";
  import {
    backupConfig,
    frequencyMs,
    getLastBackupAt,
    setLastBackupAt,
  } from "$lib/stores/backup.store";
  import { showToast } from "$lib/stores/toast.store";
  import { initTheme } from "$lib/stores/theme.store";
  import { refreshTimeEntries } from "$lib/stores/timeTracking.store";
  import { initWhiteboardEvents } from "$lib/stores/whiteboard.store";
  import Toasts from "$lib/components/Toasts.svelte";
  import { listen } from "@tauri-apps/api/event";
  import { get } from "svelte/store";
  import { vaultsApi } from "$lib/api/vaults";
  import { vaults, activeVaultId } from "$lib/stores/vaults.store";
  import CreateVault from "$lib/components/vault/CreateVault.svelte";
  import UnlockVault from "$lib/components/vault/UnlockVault.svelte";
  import VaultSelect from "$lib/components/vault/VaultSelect.svelte";
  import EditorShell from "$lib/components/layout/EditorShell.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";

  // Po wejściu do edytora pobierz foldery i notatki z dysku (raz na przejście).
  // Reset przy opuszczeniu edytora, by po ponownym odblokowaniu wczytać świeże.
  let dataLoaded = false;
  $effect(() => {
    if ($appPhase === "editor" && !dataLoaded) {
      dataLoaded = true;
      void loadFolders();
      void loadNotes();
      // Zbuduj indeks pełnotekstowy w tle (po odblokowaniu vaultu).
      void rebuildSearchIndex();
      // Wczytaj wpisy czasu tylko gdy funkcja włączona (domyślnie OFF) —
      // inaczej nie uruchamiamy liczników ani nie pokazujemy zielonej kropki.
      if (get(timeTrackingEnabled)) void refreshTimeEntries();
      // Sprzątanie w tle (nieblokująco): osierocone załączniki + pliki temp.
      void attachmentsApi.cleanupOrphanedAttachments().catch(() => {});
      void attachmentsApi.cleanupTempPreviews().catch(() => {});
      void maybeAutoBackup();
    } else if ($appPhase !== "editor") {
      dataLoaded = false;
    }
  });

  /** Auto-backup po odblokowaniu, jeśli minął skonfigurowany interwał. */
  async function maybeAutoBackup() {
    const cfg = get(backupConfig);
    if (!cfg.enabled || !cfg.location) return;
    if (Date.now() - getLastBackupAt() < frequencyMs(cfg.frequency)) return;
    try {
      await backupApi.performBackup(cfg.location, cfg.keepLast);
      setLastBackupAt(Date.now());
      showToast("Backup completed", "success");
    } catch {
      // Brak Tauri / błąd kopiowania — pomiń (nie blokuj wejścia do edytora).
    }
  }

  // ── Auto-lock po bezczynności ──────────────────────────────────────────────
  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  let lastReset = 0;

  function armIdleTimer() {
    if (idleTimer) clearTimeout(idleTimer);
    const mins = $idleMinutes;
    if (mins <= 0) return; // „Never"
    idleTimer = setTimeout(() => void triggerAutoLock(), mins * 60 * 1000);
  }

  // Throttle: maks. raz na sekundę, żeby nie resetować na każdy piksel ruchu.
  function onActivity() {
    if ($appPhase !== "editor") return;
    const now = Date.now();
    if (now - lastReset < 1000) return;
    lastReset = now;
    armIdleTimer();
  }

  async function triggerAutoLock() {
    if ($appPhase !== "editor") return;
    // Nie blokuj w trakcie blokującej operacji krypto (tworzenie/import/odblokowanie
    // innego vaultu) — przezbrój licznik i spróbuj ponownie po jej zakończeniu.
    if (get(autoLockSuspended)) {
      armIdleTimer();
      return;
    }
    await lockSession();
  }

  // Przezbrój licznik gdy zmieni się ustawienie lub wejdziemy do edytora.
  $effect(() => {
    void $idleMinutes;
    if ($appPhase === "editor") armIdleTimer();
    else if (idleTimer) clearTimeout(idleTimer);
  });

  // ── Anti-screenshot ─────────────────────────────────────────────────────────
  // Włącz ochronę przed przechwytywaniem tylko gdy vault jest odblokowany
  // (ekran Unlock może być widoczny w screen-share — nie jest wrażliwy).
  $effect(() => {
    const protect = $appPhase === "editor" && $captureProtection;
    securityApi.toggleCaptureProtection(protect).catch(() => {
      // Brak Tauri (podgląd przeglądarki) lub platforma bez wsparcia — ignoruj.
    });
  });

  // ── Panic button ───────────────────────────────────────────────────────────
  /**
   * Natychmiastowy „panic lock" (Ctrl+Shift+L). Różni się od zwykłego locka tym,
   * że najpierw błyskawicznie zasłania edytor i zamyka wszystkie panele
   * nawigacji (search, graph, menu kontekstowe), żeby nie został ślad, która
   * notatka była otwarta. Potem zapisuje notatkę i zeruje klucz.
   */
  async function panicLock() {
    setPhase("unlock"); // natychmiast zasłoń zawartość
    closeSearch();
    closeGraphView();
    closeContextMenu();
    await lockSession(); // force-save + lock_vault + czyszczenie sesji
  }

  // ── Globalne skróty klawiszowe (tylko w edytorze) ──────────────────────────
  function isTyping(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
  }

  function onKeydown(e: KeyboardEvent) {
    // Panic lock — sprawdzany przed jakimkolwiek guardem, bo musi zadziałać
    // zawsze, także gdy fokus jest w polu input/textarea.
    if (e.ctrlKey && e.shiftKey && (e.key === "L" || e.key === "l")) {
      if ($appPhase === "editor") {
        e.preventDefault();
        void panicLock();
      }
      return;
    }

    // Focus mode toggle — Ctrl+Shift+F. Dozwolone także podczas pisania
    // (wygodne przy wchodzeniu w skupienie wprost z edytora).
    if (e.ctrlKey && e.shiftKey && (e.key === "F" || e.key === "f")) {
      if ($appPhase === "editor") {
        e.preventDefault();
        toggleFocusMode();
      }
      return;
    }

    if ($appPhase !== "editor") return;

    if (e.key === "Escape") {
      closeContextMenu();
      if (get(searchActive)) closeSearch();
      if (get(focusMode)) exitFocusMode();
      return;
    }

    // Skróty z modyfikatorem nie działają podczas pisania.
    if (isTyping(e.target)) return;
    const mod = e.ctrlKey || e.metaKey;
    if (!mod) return;
    const key = e.key.toLowerCase();

    if (key === "n" && e.shiftKey) {
      e.preventDefault();
      const id = createFolder("New folder");
      pendingFolderRename.set(id);
    } else if (key === "n") {
      e.preventDefault();
      selectNote(createNote(null));
    } else if (key === "w") {
      e.preventDefault();
      deselectNote();
    }
  }

  // Dev-only: pozwala wymusić fazę z konsoli przeglądarki (bez backendu Tauri).
  // Wykluczone z buildu produkcyjnego przez `import.meta.env.DEV`.
  if (import.meta.env.DEV && typeof window !== "undefined") {
    (window as unknown as { __rune?: unknown }).__rune = { setPhase };
  }

  async function resolveInitialPhase() {
    try {
      const list = await vaultsApi.listVaults();
      vaults.set(list);

      // Jeśli sesja jest już odblokowana (np. hot-reload), wejdź do edytora.
      const unlocked = await vaultApi.isVaultUnlocked();
      if (unlocked) {
        if (list.length === 1) activeVaultId.set(list[0].id);
        setPhase("editor");
        return;
      }

      // Ekran startowy z listą vaultów pokazujemy ZAWSZE, gdy istnieje ≥1 vault —
      // także dla pojedynczego (stąd można utworzyć lub zaimportować kolejny).
      // Wybór konkretnego vaultu przełącza go i przechodzi do ekranu Unlock.
      if (list.length >= 1) {
        setPhase("vault-select");
        return;
      }

      // Brak zarejestrowanych vaultów: świeża instalacja albo dane sprzed migracji.
      const exists = await vaultApi.vaultExists();
      setPhase(exists ? "unlock" : "create");
    } catch {
      // Brak środowiska Tauri (np. czysty podgląd przeglądarki) — pokaż create.
      setPhase("create");
    }
  }

  onMount(() => {
    initTheme(); // nałóż zapisany motyw zanim cokolwiek się wyrenderuje
    void initSearchIndexEvents(); // nasłuch eventu gotowości indeksu FTS
    void initWhiteboardEvents(); // nasłuch zapisów tablic z okien Excalidraw

    const events = ["mousemove", "keydown", "mousedown", "scroll"] as const;
    events.forEach((evt) => window.addEventListener(evt, onActivity, { passive: true }));

    // Backend zgłasza, gdy pominął uszkodzone pliki notatek przy ładowaniu —
    // reszta vaultu ładuje się normalnie, użytkownik dostaje krótki komunikat.
    let unlistenNotes: (() => void) | undefined;
    listen<number>("rune://notes-load-issues", (e) => {
      const n = typeof e.payload === "number" ? e.payload : 0;
      if (n > 0) {
        showToast(
          `${n} note${n === 1 ? "" : "s"} could not be loaded (possibly corrupted)`,
          "error",
          5000,
        );
      }
    })
      .then((u) => (unlistenNotes = u))
      .catch(() => {});

    void resolveInitialPhase();

    return () => {
      events.forEach((evt) => window.removeEventListener(evt, onActivity));
      if (idleTimer) clearTimeout(idleTimer);
      unlistenNotes?.();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

{#if $appPhase === "loading"}
  <div class="loading">
    <div class="logo">Rune</div>
  </div>
{:else if $appPhase === "vault-select"}
  <VaultSelect />
{:else if $appPhase === "create"}
  <CreateVault />
{:else if $appPhase === "unlock"}
  <UnlockVault />
{:else}
  <EditorShell />
{/if}

<ContextMenu />
<Toasts />

<style>
  .loading {
    height: 100vh;
    width: 100vw;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-app);
  }

  .logo {
    font-family: var(--font-mono);
    font-size: 13px;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--accent);
    opacity: 0.5;
  }
</style>

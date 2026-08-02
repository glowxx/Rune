<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, keymap, placeholder } from "@codemirror/view";
  import { EditorState, Annotation } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
  import {
    autocompletion,
    completionKeymap,
    type CompletionContext,
    type CompletionResult,
  } from "@codemirror/autocomplete";
  import MarkdownToolbar from "$lib/components/MarkdownToolbar.svelte";
  import {
    wrapSelection,
    insertLink,
    toggleCode,
    setHeading,
  } from "$lib/utils/markdownCommands";
  import { attachmentsApi } from "$lib/api/attachments";

  interface Props {
    content: string;
    onChange: (value: string) => void;
    titles?: string[];
    noteId?: string;
  }

  let { content, onChange, titles = [], noteId }: Props = $props();

  let host: HTMLDivElement;
  let fileInput = $state<HTMLInputElement>();
  let attachError = $state<string | null>(null);
  let errorTimer: ReturnType<typeof setTimeout> | undefined;
  // $state, żeby pasek narzędzi dostał żywy `view` po jego utworzeniu w onMount.
  let view = $state<EditorView>();

  function showError(msg: string) {
    attachError = msg;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (attachError = null), 4000);
  }

  /** Zapisuje plik jako zaszyfrowany załącznik i wstawia odwołanie w treści. */
  async function handleFile(file: File): Promise<void> {
    if (!noteId || !view) return;
    if (file.size > 20 * 1024 * 1024) {
      showError("File too large (max 20MB)");
      return;
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const mime = file.type || guessMime(file.name);
      const id = await attachmentsApi.saveAttachment(bytes, file.name, mime, noteId);
      insertAttachmentRef(id, file.name, mime);
    } catch (e) {
      showError(typeof e === "string" ? e : "Could not attach file");
    }
  }

  function guessMime(name: string): string {
    const ext = name.toLowerCase().split(".").pop() ?? "";
    const map: Record<string, string> = {
      png: "image/png",
      jpg: "image/jpeg",
      jpeg: "image/jpeg",
      gif: "image/gif",
      webp: "image/webp",
      svg: "image/svg+xml",
      pdf: "application/pdf",
    };
    return map[ext] ?? "application/octet-stream";
  }

  /** Wstawia `![…](attachment://id)` (obraz) lub `[pdf:…](attachment://id)`. */
  function insertAttachmentRef(id: string, filename: string, mime: string): void {
    if (!view) return;
    const isPdf = mime === "application/pdf" || filename.toLowerCase().endsWith(".pdf");
    const safe = filename.replace(/[[\]()]/g, ""); // nie psuj składni Markdown
    const md = isPdf ? `[pdf:${safe}](attachment://${id})` : `![${safe}](attachment://${id})`;
    view.dispatch(view.state.replaceSelection(md + "\n"));
    view.focus();
  }

  function pickFile(): void {
    fileInput?.click();
  }

  function onFilePicked(e: Event): void {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files ? [...input.files] : [];
    for (const f of files) void handleFile(f);
    input.value = ""; // pozwól wybrać ten sam plik ponownie
  }

  /** Paste obrazu (np. screenshot Ctrl+V). Zwraca true gdy obsłużono. */
  function handlePaste(event: ClipboardEvent): boolean {
    const items = event.clipboardData?.items;
    if (!items) return false;
    for (const it of items) {
      if (it.kind === "file" && it.type.startsWith("image/")) {
        const file = it.getAsFile();
        if (file) {
          event.preventDefault();
          void handleFile(file);
          return true;
        }
      }
    }
    return false;
  }

  /** Drop obrazu/PDF na edytor. Zwraca true gdy obsłużono. */
  function handleDrop(event: DragEvent): boolean {
    const files = event.dataTransfer?.files;
    if (!files || files.length === 0) return false;
    const accepted = [...files].filter(
      (f) => f.type.startsWith("image/") || f.type === "application/pdf",
    );
    if (accepted.length === 0) return false;
    event.preventDefault();
    for (const f of accepted) void handleFile(f);
    return true;
  }

  // Skróty formatowania — te same komendy co przyciski paska narzędzi.
  const formatKeymap = keymap.of([
    { key: "Mod-b", run: (v) => (wrapSelection(v, "**", "**"), true) },
    { key: "Mod-i", run: (v) => (wrapSelection(v, "*", "*"), true) },
    { key: "Mod-k", run: (v) => (insertLink(v), true) },
    { key: "Mod-Shift-c", run: (v) => (toggleCode(v), true) },
    { key: "Mod-Alt-1", run: (v) => (setHeading(v, 1), true) },
    { key: "Mod-Alt-2", run: (v) => (setHeading(v, 2), true) },
    { key: "Mod-Alt-3", run: (v) => (setHeading(v, 3), true) },
  ]);

  // Najświeższa lista tytułów do autouzupełniania `[[`. Źródło completion jest
  // tworzone raz (onMount), więc czyta przez tę zmienną aktualizowaną efektem.
  let currentTitles: string[] = [];
  $effect(() => {
    currentTitles = titles;
  });

  /** Podpowiedzi notatek po wpisaniu `[[` (fuzzy: substring po tytule). */
  function wikilinkSource(context: CompletionContext): CompletionResult | null {
    const before = context.matchBefore(/\[\[([^\]]*)$/);
    if (!before) return null;
    // Nie pokazuj pustej listy zaraz po `[[`, chyba że są jakieś tytuły.
    if (before.from === before.to && !context.explicit) return null;
    const typed = before.text.slice(2).toLowerCase();
    const options = currentTitles
      .filter((t) => t.toLowerCase().includes(typed))
      .slice(0, 8)
      .map((t) => ({ label: t, type: "text", apply: `[[${t}]]` }));
    if (options.length === 0) return null;
    return { from: before.from, options, filter: false };
  }

  // Oznacza transakcje pochodzące z synchronizacji (nie od użytkownika),
  // żeby nie wywoływać onChange w pętli.
  const External = Annotation.define<boolean>();

  // Motyw dopasowany do aplikacji.
  const theme = EditorView.theme(
    {
      "&": {
        height: "100%",
        backgroundColor: "#1c1c1c",
        color: "#e8e8e8",
        fontSize: "14px",
      },
      ".cm-scroller": {
        fontFamily:
          "'JetBrains Mono', 'Fira Code', ui-monospace, 'Cascadia Code', Consolas, monospace",
        lineHeight: "1.6",
        padding: "8px 0",
      },
      ".cm-content": { caretColor: "#5c4ee8", maxWidth: "720px", margin: "0 auto", width: "100%" },
      "&.cm-focused": { outline: "none" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#5c4ee8" },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "rgba(92, 78, 232, 0.25)" },
      ".cm-gutters": {
        backgroundColor: "#1c1c1c",
        color: "rgba(255,255,255,0.2)",
        border: "none",
      },
      ".cm-activeLine": { backgroundColor: "rgba(255,255,255,0.02)" },
      ".cm-activeLineGutter": { backgroundColor: "transparent" },
      ".cm-placeholder": { color: "rgba(255,255,255,0.15)" },
      ".cm-tooltip": {
        backgroundColor: "#232323",
        border: "1px solid rgba(255,255,255,0.1)",
        borderRadius: "6px",
        overflow: "hidden",
      },
      ".cm-tooltip-autocomplete ul li": {
        color: "#d0d0d0",
        padding: "4px 10px",
        fontFamily: "'JetBrains Mono', ui-monospace, Consolas, monospace",
        fontSize: "12.5px",
      },
      ".cm-tooltip-autocomplete ul li[aria-selected]": {
        backgroundColor: "rgba(92,78,232,0.35)",
        color: "#fff",
      },
    },
    { dark: true },
  );

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: content,
        extensions: [
          history(),
          formatKeymap, // wyższy priorytet niż domyślny keymap
          keymap.of([...defaultKeymap, ...historyKeymap, ...completionKeymap]),
          markdown(),
          syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
          EditorView.lineWrapping,
          autocompletion({ override: [wikilinkSource], activateOnTyping: true }),
          placeholder("Start writing... (Markdown supported)"),
          // Wklejanie/przeciąganie obrazów i PDF jako zaszyfrowane załączniki.
          EditorView.domEventHandlers({
            paste: (event) => handlePaste(event),
            drop: (event) => handleDrop(event),
            dragover: (event) => {
              // Pozwól na drop (domyślnie przeglądarka by go zablokowała).
              if (event.dataTransfer?.types.includes("Files")) {
                event.preventDefault();
              }
              return false;
            },
          }),
          theme,
          EditorView.updateListener.of((u) => {
            if (
              u.docChanged &&
              !u.transactions.some((t) => t.annotation(External))
            ) {
              onChange(u.state.doc.toString());
            }
          }),
        ],
      }),
    });
  });

  onDestroy(() => view?.destroy());

  // Synchronizacja zewnętrznych zmian (przełączenie notatki, doładowanie treści).
  $effect(() => {
    const incoming = content;
    if (view && incoming !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: incoming },
        annotations: External.of(true),
      });
    }
  });
</script>

<div class="editor-wrap">
  <MarkdownToolbar {view} onPickFile={pickFile} />
  <div class="cm-host" bind:this={host}></div>
  <!-- Ukryty picker plików dla przycisku obrazka w pasku narzędzi. -->
  <input
    bind:this={fileInput}
    class="file-input"
    type="file"
    accept="image/*,application/pdf"
    multiple
    onchange={onFilePicked}
  />
  {#if attachError}
    <div class="attach-error" role="alert">{attachError}</div>
  {/if}
</div>

<style>
  .editor-wrap {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
  }
  .cm-host {
    flex: 1;
    width: 100%;
    min-height: 0;
    overflow: hidden;
  }
  .file-input {
    display: none;
  }
  .attach-error {
    position: absolute;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(229, 92, 92, 0.15);
    border: 1px solid rgba(229, 92, 92, 0.4);
    color: #e88;
    font-size: 12px;
    padding: 7px 14px;
    border-radius: 8px;
    z-index: 20;
  }
</style>

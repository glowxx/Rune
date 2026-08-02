<script module lang="ts">
  // Cache współdzielony między instancjami podglądu (sesja). Bajty załącznika są
  // niezmienne, więc data URL można trzymać do końca sesji; metadane notatki
  // odświeżamy, gdy pojawi się nieznane id (nowy załącznik).
  import type { AttachmentMeta } from "$lib/types";
  const imageCache = new Map<string, string>(); // attachmentId -> data URL
  const metaCache = new Map<string, Map<string, AttachmentMeta>>(); // noteId -> (id -> meta)
</script>

<script lang="ts">
  import { marked } from "marked";
  import {
    appState,
    selectNote,
    findNoteIdByTitle,
    createNote,
  } from "$lib/stores/app.store";
  import { attachmentsApi } from "$lib/api/attachments";
  import type { Note } from "$lib/types";
  import { sanitizeMarkdownHtml } from "$lib/utils/markdownSanitizer";
  import "$lib/styles/markdown.css";

  interface Props {
    content: string;
    /** Wywoływane gdy klik w checkbox zmieni `- [ ]`/`- [x]` w surowym tekście. */
    onTaskToggle?: (content: string) => void;
    /** Notatka, do której należą załączniki (do pobrania metadanych). */
    noteId?: string;
  }

  let { content, onTaskToggle, noteId }: Props = $props();

  marked.setOptions({ breaks: true, gfm: true });

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function processWikilinks(text: string, notes: Note[]): string {
    return text.replace(/\[\[([^\]]+)\]\]/g, (match, raw: string) => {
      const title = raw.trim();
      if (!title) return match;
      const exists = notes.some(
        (n) => n.title.trim().toLowerCase() === title.toLowerCase(),
      );
      const cls = exists ? "wikilink" : "wikilink-broken";
      const safe = escapeHtml(title);
      return `<a href="#" class="${cls}" data-title="${safe}">${safe}</a>`;
    });
  }

  function processTaskCheckboxes(html: string): string {
    let i = 0;
    return html.replace(/<input\b[^>]*type="checkbox"[^>]*>/gi, (tag) => {
      const idx = i++;
      const unlocked = tag.replace(/\sdisabled(?:="[^"]*")?/gi, "");
      return unlocked.replace(/\s*\/?>$/, ` data-task-index="${idx}">`);
    });
  }

  // ── Załączniki ─────────────────────────────────────────────────────────────

  /** Data URL obrazu (z cache, inaczej deszyfruj przez Rust). */
  async function getImageDataUrl(id: string): Promise<string | null> {
    const cached = imageCache.get(id);
    if (cached) return cached;
    try {
      const url = await attachmentsApi.loadAttachmentDataUrl(id);
      imageCache.set(id, url);
      return url;
    } catch {
      return null; // brak Tauri / usunięty plik
    }
  }

  /** Mapa metadanych dla notatki; odświeża gdy brakuje któregoś z `neededIds`. */
  async function getNoteMeta(
    nid: string,
    neededIds: string[],
  ): Promise<Map<string, AttachmentMeta>> {
    let m = metaCache.get(nid);
    const stale = !m || neededIds.some((id) => !m!.has(id));
    if (stale) {
      try {
        const list = await attachmentsApi.listAttachmentsForNote(nid);
        m = new Map(list.map((a) => [a.id, a]));
        metaCache.set(nid, m);
      } catch {
        m = m ?? new Map();
      }
    }
    return m ?? new Map();
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  /** Zamienia `![alt](attachment://id)` na obraz z data URL. */
  async function resolveImages(text: string): Promise<string> {
    const matches = [...text.matchAll(/!\[([^\]]*)\]\(attachment:\/\/([a-zA-Z0-9-]+)\)/g)];
    let result = text;
    for (const m of matches) {
      const [full, alt, id] = m;
      const url = await getImageDataUrl(id);
      if (url) result = result.replace(full, `![${alt}](${url})`);
    }
    return result;
  }

  /** Zamienia `[pdf:nazwa](attachment://id)` na kartę załącznika z „Open". */
  async function resolvePdfCards(text: string, nid: string | undefined): Promise<string> {
    const matches = [...text.matchAll(/\[pdf:([^\]]*)\]\(attachment:\/\/([a-zA-Z0-9-]+)\)/g)];
    if (matches.length === 0) return text;
    const meta = nid ? await getNoteMeta(nid, matches.map((m) => m[2])) : new Map();
    let result = text;
    for (const m of matches) {
      const [full, name, id] = m;
      const info = meta.get(id);
      const fname = escapeHtml(name || info?.originalFilename || "document.pdf");
      const size = info ? formatSize(info.sizeBytes) : "";
      const card =
        `<div class="pdf-card" data-attach-id="${id}">` +
        `<span class="pdf-ic" aria-hidden="true">📄</span>` +
        `<span class="pdf-info"><span class="pdf-name">${fname}</span>` +
        `<span class="pdf-size">${size}</span></span>` +
        `<button class="pdf-open" type="button" data-attach-id="${id}">Open</button>` +
        `</div>`;
      result = result.replace(full, card);
    }
    return result;
  }

  async function buildRendered(
    raw: string,
    notes: Note[],
    nid: string | undefined,
  ): Promise<string> {
    if (!raw.trim()) return "";
    let text = await resolveImages(raw);
    text = await resolvePdfCards(text, nid);
    text = processWikilinks(text, notes);
    let html = sanitizeMarkdownHtml(marked.parse(text) as string);
    html = processTaskCheckboxes(html);
    return html;
  }

  // Renderowanie jest async (deszyfrowanie załączników), więc wynik trzymamy
  // w reaktywnej zmiennej aktualizowanej przez efekt, z ochroną przed wyścigiem.
  let renderedHtml = $state("");
  $effect(() => {
    const raw = content;
    const notes = $appState.notes;
    const nid = noteId;
    let cancelled = false;
    void buildRendered(raw, notes, nid).then((html) => {
      if (!cancelled) renderedHtml = html;
    });
    return () => {
      cancelled = true;
    };
  });

  // ── Lightbox ───────────────────────────────────────────────────────────────
  let lightboxSrc = $state<string | null>(null);

  /** Delegacja kliknięć: wikilinki, obrazy (lightbox), przyciski „Open" w PDF. */
  function onPreviewClick(e: MouseEvent) {
    const target = e.target as HTMLElement;

    // PDF „Open" → deszyfruj i otwórz domyślną aplikacją.
    const openBtn = target.closest(".pdf-open") as HTMLElement | null;
    if (openBtn) {
      e.preventDefault();
      const id = openBtn.dataset.attachId;
      if (id) void attachmentsApi.openAttachment(id).catch(() => {});
      return;
    }

    // Obraz → lightbox.
    const img = target.closest(".markdown-body img") as HTMLImageElement | null;
    if (img) {
      e.preventDefault();
      lightboxSrc = img.src;
      return;
    }

    // Wikilink.
    const link = target.closest("a.wikilink, a.wikilink-broken") as HTMLElement | null;
    if (!link) return;
    e.preventDefault();
    const title = link.dataset.title ?? "";
    if (!title) return;
    if (link.classList.contains("wikilink")) {
      const id = findNoteIdByTitle(title);
      if (id) selectNote(id);
    } else {
      const ok = window.confirm(`Note "${title}" doesn't exist. Create it?`);
      if (ok) selectNote(createNote(null, title));
    }
  }

  function onCheckboxChange(e: Event) {
    const t = e.target;
    if (!(t instanceof HTMLInputElement) || t.type !== "checkbox") return;
    const attr = t.getAttribute("data-task-index");
    if (attr === null) return;
    const index = Number(attr);
    const checked = t.checked;

    let taskIndex = 0;
    const updated = content.split("\n").map((line) => {
      if (/^(\s*)[-*+]\s\[([ xX])\]/.test(line)) {
        if (taskIndex === index) {
          taskIndex++;
          return line.replace(/\[([ xX])\]/, checked ? "[x]" : "[ ]");
        }
        taskIndex++;
      }
      return line;
    });
    onTaskToggle?.(updated.join("\n"));
  }

  function previewInteractions(node: HTMLElement) {
    node.addEventListener("click", onPreviewClick);
    node.addEventListener("change", onCheckboxChange);
    return {
      destroy() {
        node.removeEventListener("click", onPreviewClick);
        node.removeEventListener("change", onCheckboxChange);
      },
    };
  }

  function onLightboxKey(e: KeyboardEvent) {
    if (e.key === "Escape") lightboxSrc = null;
  }
</script>

<svelte:window onkeydown={lightboxSrc ? onLightboxKey : undefined} />

{#if renderedHtml}
  <div class="markdown-body" use:previewInteractions>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html renderedHtml}
  </div>
{:else}
  <p class="markdown-empty">Nothing to preview yet.</p>
{/if}

{#if lightboxSrc}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="lightbox" onclick={() => (lightboxSrc = null)}>
    <img src={lightboxSrc} alt="Attachment preview" />
  </div>
{/if}

<style>
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 7000;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
    cursor: zoom-out;
  }
  .lightbox img {
    max-width: 92vw;
    max-height: 92vh;
    border-radius: 8px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
  }
</style>

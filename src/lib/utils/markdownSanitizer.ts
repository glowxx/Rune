import DOMPurify from "dompurify";

/**
 * Sanitize HTML produced from untrusted note content before it reaches Svelte's
 * `{@html}` block.  The HTML profile deliberately excludes SVG and MathML,
 * which have historically provided many alternate XSS execution paths.
 */
export function sanitizeMarkdownHtml(html: string): string {
  return DOMPurify.sanitize(html, {
    USE_PROFILES: { html: true },
    ALLOW_DATA_ATTR: true,
    FORBID_TAGS: ["script", "style", "iframe", "object", "embed", "form"],
    FORBID_ATTR: ["style", "srcdoc"],
  });
}

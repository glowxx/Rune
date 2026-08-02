// @vitest-environment jsdom

import { describe, expect, it } from "vitest";
import { sanitizeMarkdownHtml } from "./markdownSanitizer";

describe("sanitizeMarkdownHtml", () => {
  it("removes executable elements and event handlers", () => {
    const dirty = [
      '<script>globalThis.pwned = true</script>',
      '<img src="x" onerror="globalThis.pwned = true">',
      '<svg><a xlink:href="javascript:alert(1)">x</a></svg>',
      '<iframe srcdoc="<script>alert(1)</script>"></iframe>',
    ].join("");

    const clean = sanitizeMarkdownHtml(dirty);

    expect(clean).not.toMatch(/script|onerror|javascript:|srcdoc|iframe|svg/i);
  });

  it("blocks script URLs in quoted and unquoted attributes", () => {
    const clean = sanitizeMarkdownHtml(
      '<a href="javascript:alert(1)">one</a><a href=javascript:alert(2)>two</a>',
    );

    const container = document.createElement("div");
    container.innerHTML = clean;
    for (const link of container.querySelectorAll("a")) {
      expect(link.getAttribute("href") ?? "").not.toMatch(/^javascript:/i);
    }
  });

  it("keeps the safe markup used by previews", () => {
    const clean = sanitizeMarkdownHtml(
      '<a class="wikilink" data-title="Roadmap" href="#">Roadmap</a>' +
        '<input type="checkbox" disabled>' +
        '<img alt="attachment" src="data:image/png;base64,AA==">',
    );

    expect(clean).toContain('class="wikilink"');
    expect(clean).toContain('data-title="Roadmap"');
    expect(clean).toContain('type="checkbox"');
    expect(clean).toContain('src="data:image/png;base64,AA=="');
  });
});

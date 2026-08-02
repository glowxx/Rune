import { readFile, readdir } from "node:fs/promises";
import { extname, join } from "node:path";

const html = await readFile("build/whiteboard.html", "utf8");
if (/https?:\/\//i.test(html)) {
  throw new Error("Whiteboard build contains a remote URL");
}
if (!html.includes("whiteboard-assets/")) {
  throw new Error("Whiteboard build does not reference local assets");
}

const assets = await readdir("build/whiteboard-assets", { recursive: true });
const extensions = new Set(assets.map((file) => extname(file)));
for (const required of [".js", ".css", ".woff2"]) {
  if (!extensions.has(required)) {
    throw new Error(`Whiteboard build is missing ${required} assets`);
  }
}

for (const file of assets.filter((name) => [".js", ".css"].includes(extname(name)))) {
  const content = await readFile(join("build/whiteboard-assets", file), "utf8");
  if (/https:\/\/(unpkg\.com|esm\.sh)/i.test(content)) {
    throw new Error(`Whiteboard asset ${file} still references a CDN`);
  }
}

console.log(`Whiteboard bundle verified: ${assets.length} local assets`);

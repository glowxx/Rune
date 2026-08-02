import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";
import { cp, mkdir } from "node:fs/promises";

const whiteboardRoot = fileURLToPath(new URL("./whiteboard", import.meta.url));
const buildDir = fileURLToPath(new URL("./build", import.meta.url));
const fontSource = fileURLToPath(
  new URL("./node_modules/excalidraw-local/dist/prod/fonts", import.meta.url),
);
const fontTarget = fileURLToPath(
  new URL("./build/whiteboard-assets/fonts", import.meta.url),
);

const localizeExcalidrawAssets = {
  name: "localize-excalidraw-assets",
  enforce: "pre",
  transform(code, id) {
    const marker = '"ASSETS_FALLBACK_URL",';
    if (!id.includes("excalidraw-local") || !code.includes(marker)) return null;

    const markerAt = code.indexOf(marker);
    const valueStart = markerAt + marker.length;
    const valueEnd = code.indexOf(");", valueStart);
    if (valueEnd === -1) throw new Error("Could not localize Excalidraw asset fallback");

    return `${code.slice(0, valueStart)}"./whiteboard-assets/"${code.slice(valueEnd)}`;
  },
  async closeBundle() {
    await mkdir(fontTarget, { recursive: true });
    await cp(fontSource, fontTarget, { recursive: true });
  },
};

export default defineConfig({
  root: whiteboardRoot,
  base: "./",
  publicDir: false,
  plugins: [localizeExcalidrawAssets],
  build: {
    outDir: buildDir,
    emptyOutDir: false,
    copyPublicDir: false,
    assetsDir: "whiteboard-assets",
    rollupOptions: {
      input: fileURLToPath(new URL("./whiteboard/whiteboard.html", import.meta.url)),
    },
  },
});

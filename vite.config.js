import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  build: {
    rollupOptions: {
      output: {
        /** @param {string} id */
        manualChunks(id) {
          if (!id.includes("node_modules")) return undefined;
          if (
            id.includes("@codemirror/view") ||
            id.includes("@codemirror/state") ||
            id.includes("style-mod")
          ) {
            return "editor-core";
          }
          if (id.includes("@codemirror/lang") || id.includes("@lezer")) {
            return "editor-language";
          }
          if (id.includes("@codemirror")) return "editor-tools";
          if (id.includes("d3-") || id.includes("/d3/")) return "graph";
          if (id.includes("marked") || id.includes("dompurify")) return "markdown";
          return undefined;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));

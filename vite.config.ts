import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/rastro/**", "**/docs/**"] },
  },
  build: { target: "chrome120", outDir: "dist", emptyOutDir: true },
});

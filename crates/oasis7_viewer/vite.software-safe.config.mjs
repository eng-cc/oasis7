import { resolve } from "node:path";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

const tempOutDir = resolve(__dirname, ".software-safe-build");

export default defineConfig({
  plugins: [solid()],
  build: {
    target: "es2020",
    emptyOutDir: true,
    minify: false,
    sourcemap: false,
    outDir: tempOutDir,
    lib: {
      entry: resolve(__dirname, "software_safe_src/main.jsx"),
      formats: ["es"],
      fileName: () => "viewer"
    },
    rollupOptions: {
      output: {
        entryFileNames: "viewer.js",
        inlineDynamicImports: true
      }
    }
  }
});

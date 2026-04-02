import { resolve } from "node:path";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid()],
  build: {
    target: "es2020",
    emptyOutDir: false,
    minify: false,
    sourcemap: false,
    outDir: resolve(__dirname),
    lib: {
      entry: resolve(__dirname, "software_safe_src/main.jsx"),
      formats: ["es"],
      fileName: () => "software_safe"
    },
    rollupOptions: {
      output: {
        entryFileNames: "software_safe.js",
        inlineDynamicImports: true
      }
    }
  }
});

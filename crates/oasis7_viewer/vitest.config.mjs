import { defineConfig } from "vitest/config";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: "jsdom",
    globals: true,
    maxWorkers: 1,
    setupFiles: ["./test/setup.js"],
    include: [
      "software_safe_src/**/*.test.js",
      "software_safe_src/**/*.test.jsx",
    ],
    environmentOptions: {
      jsdom: {
        url: "http://127.0.0.1:4173/software_safe.html?test_api=1&connect=0&hosted_bootstrap=0&locale=en",
      },
    },
  },
});

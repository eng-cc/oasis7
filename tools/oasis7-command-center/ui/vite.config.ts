import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';
export default defineConfig({ plugins: [solid()], server: { port: 1420, strictPort: true }, clearScreen: false, test: { environment: 'jsdom', setupFiles: ['./src/test-setup.ts'] } });

import { resolve } from 'node:path';
import { defineConfig } from 'vite';

// A interface fica em ui/ e é empacotada em dist/ (lido pelo Tauri).
export default defineConfig({
  root: 'ui',
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: {
        menu: resolve(__dirname, 'ui/index.html'),
        toast: resolve(__dirname, 'ui/toast.html'),
      },
    },
  },
});

import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig(({ mode }) => ({
  root,
  base: './',
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    outDir: mode === 'mock' ? 'dist-mock' : 'dist',
    emptyOutDir: true,
    target: 'safari15',
    assetsInlineLimit: 0,
  },
}));

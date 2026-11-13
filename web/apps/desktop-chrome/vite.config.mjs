import { defineConfig } from 'vite';
import { appConfig } from '../../vite.app.mjs';

const base = appConfig('desktop-chrome');
// Two pages: the island (index.html, its own window) and the tab strip (strip.html, a webview at the
// top of the main window, ADR-0015).
export default defineConfig({
  ...base,
  build: {
    ...base.build,
    rollupOptions: {
      ...base.build.rollupOptions,
      input: { index: 'index.html', strip: 'strip.html' },
      output: { entryFileNames: '[name]-[hash].js', chunkFileNames: '[name]-[hash].js' },
    },
  },
});

// Shared Vite config for internal scyde:// pages (ADR-0005).
// Production builds get a strict CSP (CLAUDE.md rule 6): only same-origin scripts, no eval,
// no network except the host bridge. Inline style attributes are allowed (Svelte style: directives).
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

export const CSP = [
  "default-src 'self'",
  "script-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self'",
  "connect-src 'self'",
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
  // frame-ancestors only works as an HTTP header; hosts send it with the page (Phase 3–5).
].join('; ');

/** @param {string} app */
export function appConfig(app) {
  return {
    base: './',
    plugins: [
      svelte(),
      {
        name: 'scyde-csp',
        apply: 'build',
        transformIndexHtml: () => [{ tag: 'meta', attrs: { 'http-equiv': 'Content-Security-Policy', content: CSP }, injectTo: 'head-prepend' }],
      },
    ],
    server: { fs: { allow: [repo] } },
    build: {
      outDir: 'dist',
      emptyOutDir: true,
      target: ['chrome120', 'firefox128', 'safari16'],
      assetsInlineLimit: 0,
      modulePreload: { polyfill: false },
      reportCompressedSize: false,
      rollupOptions: { output: { entryFileNames: `${app}-[hash].js`, chunkFileNames: '[name]-[hash].js' } },
    },
  };
}

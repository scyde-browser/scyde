#!/usr/bin/env node
// WCAG 2.x contrast gate for the Aura tokens (DESIGN.md §2). Run after `pnpm tokens`.
// Usage: node tools/contrast-check/index.mjs [--json]
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const tokensPath = path.resolve(here, '../../web/packages/tokens/dist/json/tokens.json');
let tokens;
try {
  tokens = JSON.parse(readFileSync(tokensPath, 'utf8'));
} catch {
  console.error(`Missing ${tokensPath}. Run "pnpm tokens" first.`);
  process.exit(2);
}

export function luminance(hex) {
  const h = hex.replace('#', '').slice(0, 6);
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
  const lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}
export function contrast(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const v = (key) => {
  const t = tokens[key];
  if (!t) throw new Error(`Unknown token ${key}`);
  return t.value;
};

const AA = 4.5;
const AA_LARGE = 3;
const checks = [];
const add = (label, fg, bg, min = AA) => checks.push({ label, fg, bg, ratio: contrast(fg, bg), min });

const surfaces = ['canvas', 'chrome', 'sunken', 'raised'];
const accents = Object.keys(tokens)
  .filter((k) => /^accent\.[^.]+\.light\.fill$/.test(k))
  .map((k) => k.split('.')[1]);

for (const mode of ['light', 'dark']) {
  for (const s of surfaces) {
    const bg = v(`theme.${mode}.surface.${s}`);
    for (const t of ['primary', 'secondary', 'tertiary'])
      add(`${mode}: text.${t} on surface.${s}`, v(`theme.${mode}.text.${t}`), bg);
    for (const st of ['shield', 'warning', 'danger', 'info'])
      add(`${mode}: status.${st} on surface.${s}`, v(`theme.${mode}.status.${st}`), bg);
    for (const a of accents) {
      add(`${mode}: accent.${a}.ink on surface.${s}`, v(`accent.${a}.${mode}.ink`), bg);
      // Focus ring is a non-text UI component: 3:1 (WCAG 1.4.11).
      add(`${mode}: accent.${a}.ring on surface.${s}`, v(`accent.${a}.${mode}.ring`), bg, AA_LARGE);
    }
  }
  for (const a of accents) add(`${mode}: on-fill on accent.${a}.fill`, v(`accent.${a}.${mode}.on-fill`), v(`accent.${a}.${mode}.fill`));
  add(`${mode}: cta.text on cta.fill`, v(`theme.${mode}.cta.text`), v(`theme.${mode}.cta.fill`));
  add(`${mode}: cta.text on cta.fill-hover`, v(`theme.${mode}.cta.text`), v(`theme.${mode}.cta.fill-hover`));
}

const failed = checks.filter((c) => c.ratio < c.min);
if (process.argv.includes('--json')) {
  console.log(JSON.stringify({ checks, failed: failed.length }, null, 2));
} else {
  const worst = [...checks].sort((a, b) => a.ratio / a.min - b.ratio / b.min).slice(0, 8);
  console.log(`contrast-check: ${checks.length} pairs, ${failed.length} below threshold`);
  console.log('Tightest pairs:');
  for (const c of worst) console.log(`  ${c.ratio.toFixed(2).padStart(5)}:1 (min ${c.min})  ${c.label}`);
  for (const c of failed) console.error(`FAIL ${c.ratio.toFixed(2)}:1 < ${c.min}  ${c.label}  (${c.fg} on ${c.bg})`);
}
process.exit(failed.length ? 1 : 0);

#!/usr/bin/env node
// Bundle budgets (PLAN §8.1): gzipped JS per internal page. Fails when a budget is exceeded.
import { readdirSync, readFileSync, existsSync, appendFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const KB = 1024;
export const BUDGETS = { newtab: 120 * KB, settings: 120 * KB, shields: 120 * KB, library: 120 * KB, welcome: 120 * KB, 'desktop-chrome': 80 * KB };

let failed = false;
const rows = [];
for (const [app, budget] of Object.entries(BUDGETS)) {
  const dist = path.join(root, 'web/apps', app, 'dist');
  if (!existsSync(dist)) continue;
  const js = readdirSync(dist).filter((f) => f.endsWith('.js') && !f.startsWith('mock-'));
  const gz = js.reduce((n, f) => n + gzipSync(readFileSync(path.join(dist, f)), { level: 9 }).length, 0);
  const ok = gz <= budget;
  failed ||= !ok;
  rows.push(`| ${app} | ${(gz / KB).toFixed(1)} KB | ${budget / KB} KB | ${ok ? '✅' : '❌'} |`);
}
const md = ['### Bundle size (JS, gzip; the dev mock chunk excluded)', '| Page | Size | Budget | |', '|---|---|---|---|', ...rows].join('\n');
console.log(md);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, md + '\n');
if (failed) process.exit(1);

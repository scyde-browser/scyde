#!/usr/bin/env node
// Downloads the filter lists for the daily bundle (filters.yml, PLAN §5.3).
// Usage: node tools/filters/fetch.mjs [--lists tools/filters/lists.txt] [--out lists]
// Writes <out>/NN-<name>.txt in list order and resolves `!#include file` directives
// (uBO splits filters.txt into yearly files) relative to the including URL, same origin only.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const MAX_DEPTH = 3;
const MAX_BYTES = 50 * 1024 * 1024;

/** Parses `name url` lines, ignoring comments and blanks. */
export function parseSources(text) {
  return text
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('#'))
    .map((l) => {
      const [name, url] = l.split(/\s+/);
      if (!name || !url) throw new Error(`bad sources line: ${l}`);
      return { name, url };
    });
}

/** Downloads `url` and inlines `!#include` files from the same origin, recursively. */
export async function fetchList(url, { fetchImpl = fetch, depth = 0, seen = new Set() } = {}) {
  if (depth > MAX_DEPTH) throw new Error(`!#include nested too deep at ${url}`);
  if (seen.has(url)) return ''; // include cycles
  seen.add(url);
  let res;
  for (let attempt = 1; ; attempt++) {
    try {
      res = await fetchImpl(url, { headers: { 'user-agent': 'scyde-filters' } });
      if (res.ok) break;
      throw new Error(`HTTP ${res.status}`);
    } catch (e) {
      if (attempt >= 3) throw new Error(`${url}: ${e.message}`);
      await new Promise((r) => setTimeout(r, 1000 * 2 ** attempt));
    }
  }
  const text = await res.text();
  if (text.length > MAX_BYTES) throw new Error(`${url}: larger than ${MAX_BYTES} bytes`);
  const base = new URL(url);
  const out = [];
  for (const line of text.split('\n')) {
    const m = /^!#include\s+(\S+)/.exec(line.trim());
    if (!m) {
      out.push(line);
      continue;
    }
    const inc = new URL(m[1], base);
    if (inc.origin !== base.origin) {
      out.push(`! scyde: skipped cross-origin include ${m[1]}`);
      continue;
    }
    out.push(`! scyde: begin include ${m[1]}`);
    out.push(await fetchList(inc.href, { fetchImpl, depth: depth + 1, seen }));
    out.push(`! scyde: end include ${m[1]}`);
  }
  return out.join('\n');
}

async function main() {
  const args = process.argv.slice(2);
  const opt = (n, d) => (args.includes(n) ? args[args.indexOf(n) + 1] : d);
  const here = path.dirname(fileURLToPath(import.meta.url));
  const sources = parseSources(readFileSync(opt('--lists', path.join(here, 'lists.txt')), 'utf8'));
  const out = opt('--out', 'lists');
  mkdirSync(out, { recursive: true });
  let failed = 0;
  for (const [i, { name, url }] of sources.entries()) {
    try {
      const text = await fetchList(url);
      const file = path.join(out, `${String(i).padStart(2, '0')}-${name}.txt`);
      writeFileSync(file, text);
      const rules = text.split('\n').filter((l) => l.trim() && !l.startsWith('!') && !l.startsWith('[')).length;
      console.log(`${name.padEnd(14)} ${String(rules).padStart(7)} rules  ${file}`);
    } catch (e) {
      failed++;
      console.error(`::error::${name}: ${e.message}`);
    }
  }
  if (failed) process.exit(1);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();

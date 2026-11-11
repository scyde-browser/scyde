#!/usr/bin/env node
// Shield WASM benchmark (PLAN §8.1: WASM p99 < 200 µs; ADR-0004, ADR-0010).
// Usage: node bench.mjs <pkg-dir> [--rules 80000] [--out ../target/perf/shield-wasm.json]
// <pkg-dir> is wasm-bindgen output with --target nodejs. Runs in V8 as the CI proxy for
// SpiderMonkey (on-device numbers come from the GeckoView app, Phase 5).
// Exits non-zero on a wrong fixture decision or a budget miss.
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => (args.includes(n) ? args[args.indexOf(n) + 1] : d);
const pkg = path.resolve(args[0] ?? 'pkg');
const RULES = Number(opt('--rules', '80000'));
const OUT = path.resolve(opt('--out', path.join(here, '../target/perf/shield-wasm.json')));
const REQUESTS = 50_000;
const ROUNDS = 3;
const BUDGET = { p99_us: 200, wasm_gz_bytes: 1_000_000 };

const { ShieldEngine } = createRequire(import.meta.url)(path.join(pkg, 'shield_wasm.js'));
const fail = [];

// ---- 1. fixture decisions (qa/lists/fixture.txt) ----
const fixture = ShieldEngine.fromLists(readFileSync(path.join(here, '../../qa/lists/fixture.txt'), 'utf8'));
const base = 'http://127.0.0.1:8787/adblock/';
const page = base + 'index.html';
const expect = [
  ['ad-network/ad.js', 'script', true],
  ['ad-network/banner.gif', 'image', true],
  ['tracker/pixel.gif', 'image', true],
  ['analytics/collect.js', 'script', true],
  ['cdn/prebid-v8.js', 'script', true],
  ['ad-network/frame.html', 'sub_frame', true],
  ['cdn/prebid-allowed.js', 'script', false],
  ['index.html', 'main_frame', false],
];
for (const [p, t, want] of expect) {
  if (fixture.shouldBlock(base + p, page, t) !== want) fail.push(`fixture ${p} (${t}) should ${want ? '' : 'not '}block`);
}
const cos = JSON.parse(fixture.cosmetics(page));
const generic = JSON.parse(fixture.hiddenSelectors('["ad-banner","content"]', '["keep"]', JSON.stringify(cos.exceptions)));
if (!cos.hide.includes('.scyde-specific-ad')) fail.push('fixture host cosmetic missing');
if (!cos.hide.some((s) => s.includes('sponsor-'))) fail.push('fixture attribute cosmetic missing');
if (generic.join() !== '.ad-banner') fail.push(`fixture generic cosmetic: ${generic}`);

// ---- 2. synthetic EasyList-shaped list (same proportions as the native bench) ----
let seed = 0x5c1de5c1;
const rnd = () => {
  seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};
const below = (n) => Math.floor(rnd() * n);
const word = () => Array.from({ length: 3 + below(8) }, () => String.fromCharCode(97 + below(26))).join('');
const TLD = ['com', 'net', 'io', 'org', 'co', 'hr', 'de'];
const hosts = [];
const paths = [];
const lines = [];
for (let i = 0; i < RULES; i++) {
  const d = `${word()}${i}.${TLD[below(TLD.length)]}`;
  const roll = below(2000);
  if (roll === 0) lines.push(`/${word()}[0-9]{2,}\\.js/$script`);
  else if (roll < 900) { hosts.push(d); lines.push(`||${d}^`); }
  else if (roll < 1100) lines.push(`||${d}^$third-party`);
  else if (roll < 1400) { const p = `${word()}/${word()}`; paths.push(p); lines.push(`/${p}*.js$script`); }
  else if (roll < 1500) lines.push(`/${word()}-${word()}.$image,xmlhttprequest`);
  else if (roll < 1580) lines.push(`@@||${d}/${word()}/$script`);
  else if (roll < 1900) lines.push(`${d}##.${word()}-${word()}`);
  else lines.push(`##.${word()}-${i}`);
}
let t0 = performance.now();
const engine = ShieldEngine.fromLists(lines.join('\n'));
const buildMs = performance.now() - t0;

const TYPES = ['script', 'image', 'xmlhttprequest', 'stylesheet', 'sub_frame'];
const reqs = Array.from({ length: REQUESTS }, (_, i) => {
  const host = below(4) === 0 ? hosts[below(hosts.length)] : `static.${word()}${below(5000)}.com`;
  const p = below(20) === 0 ? paths[below(paths.length)] : `${word()}/${word()}`;
  return [`https://${host}/${p}-${word()}.js?v=${below(1000)}&id=${below(1e9)}`, `https://www.${word()}${below(500)}.hr/clanak/${i}`, TYPES[below(TYPES.length)]];
});
for (const [u, s, t] of reqs.slice(0, 2000)) engine.shouldBlock(u, s, t); // warm up

// ---- 3. latency: best of ROUNDS per request (ADR-0010) ----
const times = new Float64Array(REQUESTS);
const single = new Float64Array(REQUESTS);
let blocked = 0;
for (let i = 0; i < REQUESTS; i++) {
  const [u, s, t] = reqs[i];
  let best = Infinity;
  let b = false;
  for (let r = 0; r < ROUNDS; r++) {
    const a = performance.now();
    b = engine.shouldBlock(u, s, t);
    const e = (performance.now() - a) * 1000;
    if (r === 0) single[i] = e;
    if (e < best) best = e;
  }
  times[i] = best;
  if (b) blocked++;
}
times.sort();
single.sort();
const pct = (a, p) => a[Math.round((a.length - 1) * p)];
const p50 = pct(times, 0.5);
const p99 = pct(times, 0.99);
const singleP99 = pct(single, 0.99);

const wasmFile = readdirSync(pkg).find((f) => f.endsWith('.wasm'));
const wasm = readFileSync(path.join(pkg, wasmFile));
const gz = gzipSync(wasm, { level: 9 }).length;

console.log(`shield-wasm · ${RULES} synthetic rules · ${REQUESTS} requests (best of ${ROUNDS}) · V8 ${process.versions.v8}`);
console.log(`  fixture          ${fail.length ? 'FAIL' : 'ok'} (${expect.length} network + 3 cosmetic)`);
console.log(`  build            ${buildMs.toFixed(0).padStart(8)} ms`);
console.log(`  match p50        ${p50.toFixed(2).padStart(8)} µs`);
console.log(`  match p99        ${p99.toFixed(2).padStart(8)} µs   (budget < ${BUDGET.p99_us} µs)`);
console.log(`  single-shot p99  ${singleP99.toFixed(2).padStart(8)} µs   (info)`);
console.log(`  blocked          ${((blocked / REQUESTS) * 100).toFixed(1).padStart(8)} %`);
console.log(`  wasm             ${(wasm.length / 1048576).toFixed(2).padStart(8)} MB, ${(gz / 1024).toFixed(0)} KB gz   (budget ≤ ${BUDGET.wasm_gz_bytes / 1000} KB gz)`);

if (p99 >= BUDGET.p99_us) fail.push(`p99 ${p99.toFixed(1)} µs`);
if (gz > BUDGET.wasm_gz_bytes) fail.push(`wasm ${gz} B gz`);

mkdirSync(path.dirname(OUT), { recursive: true });
writeFileSync(OUT, JSON.stringify({
  suite: 'shield-wasm',
  rules: RULES,
  requests: REQUESTS,
  metrics: { match_p50_us: +p50.toFixed(3), match_p99_us: +p99.toFixed(3), match_p99_single_shot_us: +singleP99.toFixed(3), build_ms: +buildMs.toFixed(1), wasm_bytes: wasm.length, wasm_gz_bytes: gz },
  budgets: BUDGET,
}, null, 2) + '\n');
console.log(`  → ${OUT}`);
if (fail.length) {
  console.error('FAIL: ' + fail.join('; '));
  process.exit(1);
}

#!/usr/bin/env node
// Spike S3 / G3: WASM Shield decision latency (p50/p99) and fixture correctness.
// Usage: node bench.mjs <list.txt>... [--json out.json]
// Runs in Node (V8). Gecko's SpiderMonkey is measured on-device in the GeckoView app; this is the CI proxy.
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const { ShieldEngine } = require('./pkg/scyde_shield_wasm_spike.js');
const args = process.argv.slice(2);
const jsonOut = args.includes('--json') ? args[args.indexOf('--json') + 1] : null;
const lists = args.filter((a, i) => !a.startsWith('--') && args[i - 1] !== '--json');
if (!lists.length) throw new Error('pass at least one filter list');

// ---- 1. fixture correctness (qa/lists/fixture.txt must give exact decisions) ----
const fixture = new ShieldEngine(readFileSync(path.resolve(here, '../../../qa/lists/fixture.txt'), 'utf8'));
const page = 'http://127.0.0.1:8787/adblock/';
const expect = [
  ['http://127.0.0.1:8787/adblock/ad-network/ad.js', 'script', true],
  ['http://127.0.0.1:8787/adblock/ad-network/banner.gif', 'image', true],
  ['http://127.0.0.1:8787/adblock/tracker/pixel.gif', 'image', true],
  ['http://127.0.0.1:8787/adblock/analytics/collect.js', 'script', true],
  ['http://127.0.0.1:8787/adblock/cdn/prebid-v8.js', 'script', true],
  ['http://127.0.0.1:8787/adblock/ad-network/frame.html', 'sub_frame', true],
  ['http://127.0.0.1:8787/adblock/cdn/prebid-allowed.js', 'script', false],
  ['http://127.0.0.1:8787/adblock/index.html', 'main_frame', false],
];
const fixtureResults = expect.map(([u, t, want]) => ({ url: u, type: t, want, got: fixture.should_block(u, page, t) }));
const fixtureOk = fixtureResults.filter((r) => r.want === r.got).length;
const cos = JSON.parse(fixture.cosmetic(page));
const generic = fixture.hidden_selectors(['ad-banner', 'content'], ['sponsor-top', 'keep'], JSON.stringify(cos.exceptions));
console.log(`fixture network: ${fixtureOk}/${expect.length} correct`);
console.log(`fixture cosmetic: host-specific ${JSON.stringify(cos.hide)}, generic ${JSON.stringify(generic)}`);

// ---- 2. engine build from real lists ----
const text = lists.map((f) => readFileSync(f, 'utf8')).join('\n');
const ruleLines = text.split('\n').filter((l) => l && !l.startsWith('!') && !l.startsWith('['));
let t0 = performance.now();
const engine = new ShieldEngine(text);
const buildMs = performance.now() - t0;
const serialized = engine.serialized_len();
console.log(`lists: ${lists.map((l) => path.basename(l)).join(', ')} · ${ruleLines.length} rule lines · build ${buildMs.toFixed(0)} ms · engine.dat ≈ ${(serialized / 1e6).toFixed(2)} MB`);

// ---- 3. request corpus: rule hosts (should mostly block) + ordinary traffic ----
let seed = 42;
const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 2 ** 32);
const pick = (a) => a[Math.floor(rnd() * a.length)];
const adHosts = [...text.matchAll(/^\|\|([a-z0-9.-]+\.[a-z]{2,})\^/gm)].map((m) => m[1]);
const normalHosts = ['www.index.hr', 'cdn.jsdelivr.net', 'fonts.gstatic.com', 'hr.wikipedia.org', 'upload.wikimedia.org', 'www.jutarnji.hr', 'static.24sata.hr', 'i.ytimg.com', 'github.githubassets.com', 'www.reddit.com', 'cdnjs.cloudflare.com', 'images.net.hr'];
const paths = ['/static/js/app.', '/assets/img/', '/api/v1/', '/media/', '/css/main.', '/ads/', '/pixel', '/track', '/banner/', '/news/2026/'];
const types = ['script', 'image', 'stylesheet', 'xmlhttprequest', 'sub_frame', 'font', 'media', 'other'];
const N = 50000;
const corpus = Array.from({ length: N }, () => {
  const host = rnd() < 0.3 && adHosts.length ? pick(adHosts) : pick(normalHosts);
  const p = pick(paths) + Math.floor(rnd() * 1e6).toString(36) + pick(['.js', '.png', '.css', '', '?id=' + Math.floor(rnd() * 1e9)]);
  return [`https://${host}${p}`, `https://${pick(normalHosts)}/`, pick(types)];
});

// warm-up (JIT + engine caches), then measure every call
for (let i = 0; i < 2000; i++) engine.should_block(...corpus[i]);
const lat = new Float64Array(N);
let blocked = 0;
for (let i = 0; i < N; i++) {
  const s = process.hrtime.bigint();
  if (engine.should_block(...corpus[i])) blocked++;
  lat[i] = Number(process.hrtime.bigint() - s) / 1000; // µs
}
lat.sort();
const q = (p) => lat[Math.min(N - 1, Math.floor(p * N))];
const stats = { n: N, blocked, p50_us: q(0.5), p90_us: q(0.9), p99_us: q(0.99), p999_us: q(0.999), max_us: lat[N - 1] };
console.log(`decisions: ${N} · blocked ${blocked} · p50 ${stats.p50_us.toFixed(1)} µs · p99 ${stats.p99_us.toFixed(1)} µs · p99.9 ${stats.p999_us.toFixed(1)} µs`);
const g3 = stats.p99_us < 200 && fixtureOk === expect.length;
console.log(`G3 (WASM part): p99 < 200 µs and fixture exact → ${g3 ? 'PASS' : 'FAIL'}`);

const wasmBytes = readFileSync(path.join(here, 'pkg/scyde_shield_wasm_spike_bg.wasm')).length;
if (jsonOut)
  writeFileSync(jsonOut, JSON.stringify({ node: process.version, wasm_bytes: wasmBytes, rule_lines: ruleLines.length, build_ms: buildMs, engine_dat_bytes: serialized, fixture: { correct: fixtureOk, total: expect.length, results: fixtureResults, cosmetic_host: cos.hide, cosmetic_generic: generic }, latency: stats, g3_wasm_pass: g3 }, null, 2));
process.exit(g3 ? 0 : 1);

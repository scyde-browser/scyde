#!/usr/bin/env node
// Perf regression gate (PLAN §8.2: a regression > 10 % fails the job).
// Usage: node tools/perf/compare.mjs --base a.json[,b.json…] --head c.json[,d.json…] [--threshold 0.10]
// Each file is a perf.json ({suite, metrics, budgets}). Several runs per side → per-metric median,
// so base and head measured alternately on the same runner cancel most machine noise.
// Gated: every metric that has a budget (lower is better). Others are reported only.
// Writes a Markdown table to $GITHUB_STEP_SUMMARY when set; exits 1 on a gated regression.
import { readFileSync, appendFileSync } from 'node:fs';

export function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const m = Math.floor(s.length / 2);
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}

/** Per-metric medians over several perf.json objects. */
export function aggregate(runs) {
  const out = {};
  for (const r of runs) for (const [k, v] of Object.entries(r.metrics)) (out[k] ??= []).push(v);
  return Object.fromEntries(Object.entries(out).map(([k, v]) => [k, median(v)]));
}

/** Rows {metric, base, head, change, gated, regressed}. */
export function compare(baseRuns, headRuns, threshold = 0.1) {
  const base = aggregate(baseRuns);
  const head = aggregate(headRuns);
  const budgets = headRuns[0]?.budgets ?? {};
  return Object.keys(head).map((metric) => {
    const b = base[metric];
    const h = head[metric];
    const change = b ? (h - b) / b : 0;
    const gated = metric in budgets && b !== undefined;
    return { metric, base: b, head: h, change, gated, regressed: gated && change > threshold };
  });
}

function main() {
  const args = process.argv.slice(2);
  const opt = (n, d) => (args.includes(n) ? args[args.indexOf(n) + 1] : d);
  const load = (list) => list.split(',').filter(Boolean).map((f) => JSON.parse(readFileSync(f, 'utf8')));
  const threshold = Number(opt('--threshold', '0.10'));
  const head = load(opt('--head', ''));
  const base = load(opt('--base', ''));
  if (!head.length) throw new Error('--head is required');
  const rows = compare(base, head, threshold);
  const fmt = (x) => (x === undefined ? '–' : Number(x.toFixed(3)).toString());
  const md = [
    `### Perf: ${head[0].suite} (median of ${head.length}, threshold ${threshold * 100} %)`,
    '| Metric | Base | Head | Change | |',
    '|---|---|---|---|---|',
    ...rows.map((r) => `| ${r.metric} | ${fmt(r.base)} | ${fmt(r.head)} | ${r.base ? (r.change * 100).toFixed(1) + ' %' : '–'} | ${r.regressed ? '❌ regression' : r.gated ? '✅' : 'info'} |`),
  ].join('\n');
  console.log(md);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, md + '\n');
  if (!base.length) console.log('(no base runs: nothing to compare)');
  if (rows.some((r) => r.regressed)) process.exit(1);
}

if (import.meta.url === `file://${process.argv[1]}`) main();

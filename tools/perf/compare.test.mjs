import { test } from 'node:test';
import assert from 'node:assert/strict';
import { compare, median } from './compare.mjs';

const run = (p50, p99, extra = 0) => ({ suite: 's', metrics: { match_p50_us: p50, match_p99_us: p99, build_ms: 100 + extra }, budgets: { match_p50_us: 5, match_p99_us: 50 } });

test('median', () => {
  assert.equal(median([3, 1, 2]), 2);
  assert.equal(median([4, 1, 2, 3]), 2.5);
});

test('gates budgeted metrics only, on medians', () => {
  const base = [run(2, 4), run(2.1, 4.2), run(9, 40)]; // one noisy base run
  const ok = compare(base, [run(2.15, 4.3, 500), run(2.2, 4.1), run(2.1, 4.4)]);
  assert.ok(!ok.some((r) => r.regressed), 'within 10 %, build_ms is info only');
  const bad = compare(base, [run(2.5, 4.3), run(2.6, 4.2), run(2.4, 4.4)]);
  assert.deepEqual(bad.filter((r) => r.regressed).map((r) => r.metric), ['match_p50_us']);
  assert.ok(!compare([], [run(2, 4)]).some((r) => r.regressed), 'no base: nothing to gate');
});

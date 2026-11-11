import { test } from 'node:test';
import assert from 'node:assert/strict';
import { toPhysics, position, settleTime, toCssLinear } from './springs.mjs';

test('stiffness follows (2π/response)²', () => {
  const { stiffness } = toPhysics({ response: 0.5, damping: 1 });
  assert.ok(Math.abs(stiffness - (2 * Math.PI / 0.5) ** 2) < 1e-9);
});

test('starts at 0 and settles at 1', () => {
  for (const s of [{ response: 0.3, damping: 0.86 }, { response: 0.45, damping: 1 }, { response: 0.5, damping: 0.72 }]) {
    assert.ok(Math.abs(position(s, 0)) < 1e-9);
    assert.ok(Math.abs(1 - position(s, settleTime(s) / 1000)) <= 0.001);
  }
});

test('bouncy overshoots, smooth does not', () => {
  const peak = (s) => Math.max(...Array.from({ length: 1000 }, (_, i) => position(s, i / 1000)));
  assert.ok(peak({ response: 0.5, damping: 0.72 }) > 1.01);
  assert.ok(peak({ response: 0.45, damping: 1 }) <= 1);
});

test('linear() string ends at 1', () => {
  const { easing } = toCssLinear({ response: 0.3, damping: 0.86 });
  assert.match(easing, /^linear\(0, .*, 1\)$/);
});

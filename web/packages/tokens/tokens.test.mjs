import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const css = readFileSync(new URL('./dist/css/tokens.css', import.meta.url), 'utf8');
const json = JSON.parse(readFileSync(new URL('./dist/json/tokens.json', import.meta.url), 'utf8'));

test('theme selectors from DESIGN.md §9 exist', () => {
  for (const sel of ['[data-theme="light"]', '[data-theme="dark"]', '[data-density="compact"]', '[data-motion="reduced"]'])
    assert.ok(css.includes(sel), sel);
  for (const a of ['sky', 'periwinkle', 'lavender', 'mint', 'ember', 'onyx'])
    assert.ok(css.includes(`[data-theme="dark"][data-accent="${a}"]`), a);
});

test('semantic variables used by components are defined', () => {
  for (const v of ['--surface-canvas', '--surface-sunken', '--text-primary', '--accent-fill', '--accent-ink', '--accent-on-fill', '--accent-ring', '--cta-fill', '--elev-2', '--motion-spring-snappy'])
    assert.match(css, new RegExp(`${v}:`), v);
});

test('system preferences are honoured', () => {
  assert.ok(css.includes('@media (prefers-color-scheme: dark)'));
  assert.ok(css.includes('@media (prefers-reduced-motion: reduce)'));
});

test('no unresolved references leak into output', () => {
  const ref = /\{[a-z][a-z0-9-]*\.[a-z0-9.-]+\}/;
  assert.doesNotMatch(css, ref);
  for (const [k, t] of Object.entries(json)) assert.doesNotMatch(JSON.stringify(t.value), ref, k);
});

test('motion durations match DESIGN.md §6.2', () => {
  assert.equal(json['motion.duration.instant'].value, '90ms');
  assert.equal(json['motion.duration.choreo-max'].value, '420ms');
});

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { locales } from './index.mjs';

test('hr and en have identical keys', () => {
  assert.deepEqual(Object.keys(locales.hr).sort(), Object.keys(locales.en).sort());
});

test('no empty strings', () => {
  for (const [lang, dict] of Object.entries(locales))
    for (const [k, v] of Object.entries(dict)) assert.ok(v.trim().length > 0, `${lang}:${k}`);
});

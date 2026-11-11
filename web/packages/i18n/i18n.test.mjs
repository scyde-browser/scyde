import { test } from 'node:test';
import assert from 'node:assert/strict';
import { locales, defaultLocale } from './index.mjs';

test('English is the only shipped locale (ADR-0009)', () => {
  assert.deepEqual(Object.keys(locales), ['en']);
  assert.equal(defaultLocale, 'en');
});

test('no empty strings', () => {
  for (const [k, v] of Object.entries(locales.en)) assert.ok(v.trim().length > 0, k);
});

test('no Croatian diacritics left in UI copy', () => {
  for (const [k, v] of Object.entries(locales.en)) assert.doesNotMatch(v, /[čćđšžČĆĐŠŽ]/, k);
});

test('t() formats placeholders and plurals', async () => {
  const { t } = await import('./index.mjs');
  assert.equal(t('shield.blocked_count', { count: 1 }), '1 blocked');
  assert.equal(t('shield.blocked_count', { count: 1234 }), '1,234 blocked');
  assert.equal(t('omnibox.suggest.search', { q: 'kava' }), 'Search for “kava”');
  assert.equal(t('no.such.key'), 'no.such.key');
});

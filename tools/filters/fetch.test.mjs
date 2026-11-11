import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fetchList, parseSources } from './fetch.mjs';

const files = {
  'https://lists.example/filters/filters.txt': '! Title: x\n||a.example^\n!#include filters-2026.txt\n!#include https://evil.example/x.txt\n',
  'https://lists.example/filters/filters-2026.txt': '||b.example^\n!#include filters.txt\n',
};
const fakeFetch = async (url) =>
  url in files ? { ok: true, status: 200, text: async () => files[url] } : { ok: false, status: 404, text: async () => '' };

test('parses sources', () => {
  assert.deepEqual(parseSources('# c\n\na  https://x/a.txt\n'), [{ name: 'a', url: 'https://x/a.txt' }]);
  assert.throws(() => parseSources('lonely'));
});

test('inlines same-origin includes, skips cross-origin and cycles', async () => {
  const text = await fetchList('https://lists.example/filters/filters.txt', { fetchImpl: fakeFetch });
  assert.match(text, /\|\|a\.example\^/);
  assert.match(text, /\|\|b\.example\^/);
  assert.match(text, /skipped cross-origin include/);
  assert.equal(text.match(/\|\|a\.example\^/g).length, 1, 'cycle does not duplicate');
});

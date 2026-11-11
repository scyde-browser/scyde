import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createIndex, hostChain } from './src/cosmetics.js';

test('host chain', () => assert.deepEqual(hostChain('a.b.hr'), ['a.b.hr', 'b.hr', 'hr']));

test('host, generic and exceptions', () => {
  const idx = createIndex(
    { classes: { 'ad-banner': ['.ad-banner'] }, ids: { top: ['#top'] }, other: ['div[id^="sponsor-"]'], exceptions: ['.ok'] },
    { 'example.hr': { hide: ['.x'] }, 'news.example.hr': { unhide: ['.ad-banner', '.x'] } },
  );
  assert.deepEqual(idx.forHost('www.example.hr').sort(), ['.x', 'div[id^="sponsor-"]']);
  assert.deepEqual(idx.forHost('news.example.hr'), ['div[id^="sponsor-"]']);
  assert.deepEqual(idx.forClassesAndIds('example.hr', ['ad-banner', 'content'], ['top']), ['.ad-banner', '#top']);
  assert.deepEqual(idx.forClassesAndIds('news.example.hr', ['ad-banner'], []), []);
});

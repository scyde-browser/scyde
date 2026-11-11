#!/usr/bin/env node
// Native string catalogs from web/packages/i18n/locales/en.json (rule 10: one source of copy).
//   node tools/i18n-native/build.mjs --ios <Localizable.xcstrings> --android <values/strings.xml>
// Placeholders: {name} → %@ (iOS) / %s (Android, positional). Plural messages keep their "other" form.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const en = JSON.parse(readFileSync(path.join(here, '../../web/packages/i18n/locales/en.json'), 'utf8'));
const args = process.argv.slice(2);
const opt = (n) => (args.includes(n) ? args[args.indexOf(n) + 1] : undefined);

const flatten = (v) => v.replace(/\{(\w+), plural, (?:[^{}]*\{[^}]*\})*?\s*other \{([^}]*)\}\s*\}/g, (_, n, other) => other.replaceAll('#', `{${n}}`));

export function toIOS(v) {
  return flatten(v).replace(/\{\w+\}/g, '%@');
}
export function toAndroid(v) {
  let i = 0;
  return flatten(v)
    .replace(/\{\w+\}/g, () => `%${++i}$s`)
    .replace(/[\\']/g, (c) => `\\${c}`)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '\\"');
}

if (opt('--ios')) {
  const strings = Object.fromEntries(
    Object.entries(en).map(([k, v]) => [k, { localizations: { en: { stringUnit: { state: 'translated', value: toIOS(v) } } } }]),
  );
  mkdirSync(path.dirname(opt('--ios')), { recursive: true });
  writeFileSync(opt('--ios'), JSON.stringify({ sourceLanguage: 'en', strings, version: '1.0' }, null, 2));
}
if (opt('--android')) {
  const rows = Object.entries(en).map(([k, v]) => `  <string name="${k.replace(/[^a-zA-Z0-9_]/g, '_')}">${toAndroid(v)}</string>`);
  mkdirSync(path.dirname(opt('--android')), { recursive: true });
  writeFileSync(opt('--android'), `<?xml version="1.0" encoding="utf-8"?>\n<!-- Generated from web/packages/i18n/locales/en.json. Do not edit. -->\n<resources>\n${rows.join('\n')}\n</resources>\n`);
}

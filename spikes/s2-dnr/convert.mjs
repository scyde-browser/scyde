#!/usr/bin/env node
// Spike S2 option A: minimal ABP -> Chromium declarativeNetRequest converter, writes an unpacked MV3 extension.
// Handles the common network subset: ||host^, plain/path filters, @@ exceptions, $third-party/$first-party,
// resource-type options and $domain=. Everything else (regex, cosmetic, scriptlets, $redirect, $csp...) is
// counted as dropped. The real converter is part of shield-compiler (Phase 1).
// Usage: node convert.mjs --out <dir> [--per-ruleset 30000] [--max 330000] <list.txt>...
// Rules are split into static rulesets of --per-ruleset (Chromium guarantees 30k per extension, the rest comes
// from the shared global pool; at most 50 enabled rulesets).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
const opt = (n, d) => (args.includes(n) ? args[args.indexOf(n) + 1] : d);
const out = opt('--out', 'dnr-ext');
const max = Number(opt('--max', '330000'));
const per = Number(opt('--per-ruleset', '30000'));
const lists = args.filter((a, i) => !a.startsWith('--') && !['--out', '--max', '--per-ruleset'].includes(args[i - 1]));

const TYPES = { script: 'script', image: 'image', stylesheet: 'stylesheet', css: 'stylesheet', xmlhttprequest: 'xmlhttprequest', xhr: 'xmlhttprequest', subdocument: 'sub_frame', frame: 'sub_frame', font: 'font', media: 'media', websocket: 'websocket', ping: 'ping', other: 'other', object: 'object', document: 'main_frame' };
const stats = { total: 0, converted: 0, dropped: 0, reasons: {} };
const drop = (why) => { stats.dropped++; stats.reasons[why] = (stats.reasons[why] || 0) + 1; };

function convert(line) {
  if (!line || line.startsWith('!') || line.startsWith('[')) return null;
  stats.total++;
  if (line.includes('##') || line.includes('#@#') || line.includes('#?#') || line.includes('#$#') || line.includes('#+js')) return drop('cosmetic'), null;
  let allow = false;
  if (line.startsWith('@@')) { allow = true; line = line.slice(2); }
  let [pattern, options = ''] = line.split(/\$(?=[^/]*$)/);
  if (pattern.startsWith('/') && pattern.endsWith('/') && pattern.length > 2) return drop('regex'), null;
  const condition = {};
  const resourceTypes = [];
  const excludedResourceTypes = [];
  for (const o of options ? options.split(',') : []) {
    const neg = o.startsWith('~');
    const name = neg ? o.slice(1) : o;
    if (name === 'third-party' || name === '3p') condition.domainType = neg ? 'firstParty' : 'thirdParty';
    else if (name === 'first-party' || name === '1p') condition.domainType = neg ? 'thirdParty' : 'firstParty';
    else if (TYPES[name]) (neg ? excludedResourceTypes : resourceTypes).push(TYPES[name]);
    else if (name.startsWith('domain=')) {
      const doms = name.slice(7).split('|');
      const inc = doms.filter((d) => !d.startsWith('~'));
      const exc = doms.filter((d) => d.startsWith('~')).map((d) => d.slice(1));
      if (inc.length) condition.initiatorDomains = inc;
      if (exc.length) condition.excludedInitiatorDomains = exc;
    } else if (name === 'match-case') condition.isUrlFilterCaseSensitive = true;
    else if (name === 'important' || name === 'all') { /* ignore */ }
    else return drop(`option:${name.split('=')[0]}`), null;
  }
  if (!pattern || pattern === '*') return drop('empty'), null;
  if (!/^[\x20-\x7e]+$/.test(pattern)) return drop('non-ascii'), null;
  condition.urlFilter = pattern;
  if (resourceTypes.length) condition.resourceTypes = [...new Set(resourceTypes)];
  if (excludedResourceTypes.length) condition.excludedResourceTypes = [...new Set(excludedResourceTypes)];
  else if (!resourceTypes.length) condition.excludedResourceTypes = ['main_frame'];
  stats.converted++;
  return { priority: allow ? 2 : 1, action: { type: allow ? 'allow' : 'block' }, condition };
}

const rules = [];
for (const f of lists) for (const line of readFileSync(f, 'utf8').split('\n')) {
  const r = convert(line.trim());
  if (r) rules.push(r);
}
const kept = rules.slice(0, max).map((r, i) => ({ id: i + 1, ...r }));
stats.over_limit = Math.max(0, rules.length - max);
mkdirSync(out, { recursive: true });
const resources = [];
for (let i = 0; i * per < kept.length; i++) {
  const file = `rules-${i}.json`;
  writeFileSync(path.join(out, file), JSON.stringify(kept.slice(i * per, (i + 1) * per)));
  resources.push({ id: `scyde-${i}`, enabled: true, path: file });
}
writeFileSync(path.join(out, 'manifest.json'), JSON.stringify({
  manifest_version: 3,
  name: 'SCYDE Shield DNR (spike S2-A)',
  version: '0.0.1',
  permissions: ['declarativeNetRequest', 'declarativeNetRequestFeedback'],
  host_permissions: ['<all_urls>'],
  declarative_net_request: { rule_resources: resources },
}, null, 2));
writeFileSync(path.join(out, 'stats.json'), JSON.stringify({ ...stats, kept: kept.length, rulesets: resources.length }, null, 2));
console.log(JSON.stringify({ ...stats, kept: kept.length, rulesets: resources.length }));

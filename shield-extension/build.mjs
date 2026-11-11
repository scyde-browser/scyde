#!/usr/bin/env node
// Builds the Shield extension from a shield-compiler bundle.
//   node shield-extension/build.mjs --target chromium-mv3 --bundle <bundle dir> --out <dir>
// chromium-mv3: DNR static rulesets (≤ 30k each) + cosmetics worker/content script (WebView2).
// The gecko-mv2 target (webRequest + WASM engine) is built in Phase 5.
import { mkdirSync, readFileSync, writeFileSync, copyFileSync, rmSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (n, d) => (args.includes(n) ? args[args.indexOf(n) + 1] : d);
const target = opt('--target', 'chromium-mv3');
const bundle = path.resolve(opt('--bundle', ''));
const out = path.resolve(opt('--out', 'dist/' + target));
const version = opt('--version', '0.1.0');

const manifest = JSON.parse(readFileSync(path.join(bundle, 'manifest.json'), 'utf8'));
rmSync(out, { recursive: true, force: true });
mkdirSync(path.join(out, 'cosmetic'), { recursive: true });
mkdirSync(path.join(out, 'dnr'), { recursive: true });

if (target !== 'chromium-mv3') throw new Error(`unknown target ${target}`);

for (const f of ['generic.json', 'hosts.json']) copyFileSync(path.join(bundle, 'cosmetic', f), path.join(out, 'cosmetic', f));
const rulesets = manifest.dnr.map((r) => {
  copyFileSync(path.join(bundle, r.path), path.join(out, r.path));
  return { id: r.id, enabled: true, path: r.path };
});
for (const f of ['content.js', 'cosmetics.js', 'chromium-background.js']) copyFileSync(path.join(here, 'src', f), path.join(out, f));

writeFileSync(
  path.join(out, 'manifest.json'),
  JSON.stringify(
    {
      manifest_version: 3,
      name: 'SCYDE Shield',
      version,
      description: `Built-in ad, tracker and popup protection (filters ${manifest.version})`,
      permissions: ['declarativeNetRequest'],
      host_permissions: ['<all_urls>'],
      background: { service_worker: 'chromium-background.js', type: 'module' },
      content_scripts: [{ matches: ['http://*/*', 'https://*/*'], js: ['content.js'], run_at: 'document_start', all_frames: true }],
      declarative_net_request: { rule_resources: rulesets },
    },
    null,
    2,
  ),
);
console.log(`${target}: ${rulesets.length} DNR rulesets, filters ${manifest.version} → ${out}`);
console.log(readdirSync(out).join(' '));

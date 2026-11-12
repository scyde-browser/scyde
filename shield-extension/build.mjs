#!/usr/bin/env node
// Builds the Shield extension from a shield-compiler bundle.
//   node shield-extension/build.mjs --target chromium-mv3 --bundle <bundle dir> --out <dir>
// chromium-mv3: DNR static rulesets (≤ 30k each) + cosmetics worker/content script (WebView2).
// gecko-mv2: webRequestBlocking + WASM engine (engine.dat) + cosmetics + the internal pages
//   (--wasm <shield-wasm no-modules dir> --pages <dir with newtab/ settings/ shields/ library/>).
import { mkdirSync, readFileSync, writeFileSync, copyFileSync, rmSync, readdirSync, cpSync } from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { shardOf } from './src/shard.js';
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

if (target === 'gecko-mv2') {
  const wasm = path.resolve(opt('--wasm', ''));
  const pages = opt('--pages');
  mkdirSync(path.join(out, 'engine'), { recursive: true });
  for (const f of ['shield_wasm.js', 'shield_wasm_bg.wasm']) copyFileSync(path.join(wasm, f), path.join(out, 'engine', f));
  copyFileSync(path.join(bundle, 'resources.json'), path.join(out, 'resources.json'));
  // engine.dat is written by the same wasm32 engine that loads it (see ShieldEngine.toDat): the
  // bundle's file comes from a 64-bit host and loses generic class/id cosmetics in wasm32.
  const lists = opt('--lists');
  const wasmNode = opt('--wasm-node');
  if (!lists || !wasmNode) throw new Error('gecko-mv2 needs --lists <dir> and --wasm-node <dir> (core/target/shield-wasm/node)');
  const { ShieldEngine } = createRequire(import.meta.url)(path.resolve(wasmNode, 'shield_wasm.js'));
  const text = readdirSync(path.resolve(lists)).filter((f) => f.endsWith('.txt')).sort()
    .map((f) => readFileSync(path.resolve(lists, f), 'utf8')).join('\n');
  const dat = ShieldEngine.fromLists(text).toDat();
  writeFileSync(path.join(out, 'engine.dat'), dat);
  // Guard: a generic class rule from the lists must still match after loading the file.
  const probe = text.match(/^##\.([A-Za-z0-9_-]+)$/m)?.[1];
  if (probe && !JSON.parse(ShieldEngine.fromDat(dat).hiddenSelectors(JSON.stringify([probe]), '[]', '[]')).includes('.' + probe)) {
    throw new Error(`engine.dat lost generic cosmetics (probe .${probe})`);
  }
  for (const f of ['content.js', 'gecko-background.js', 'scroll-guard.js']) copyFileSync(path.join(here, 'src', f), path.join(out, f));
  if (pages) cpSync(path.resolve(pages), path.join(out, 'pages'), { recursive: true });
  rmSync(path.join(out, 'dnr'), { recursive: true, force: true });
  rmSync(path.join(out, 'cosmetic'), { recursive: true, force: true });
  writeFileSync(
    path.join(out, 'manifest.json'),
    JSON.stringify(
      {
        manifest_version: 2,
        name: 'SCYDE Shield',
        version,
        description: `Built-in ad, tracker and popup protection (filters ${manifest.version})`,
        browser_specific_settings: { gecko: { id: 'shield@scyde.dev' } },
        permissions: ['webRequest', 'webRequestBlocking', '<all_urls>', 'nativeMessaging', 'nativeMessagingFromContent', 'geckoViewAddons'],
        background: { scripts: ['engine/shield_wasm.js', 'gecko-background.js'] },
        content_scripts: [
          { matches: ['http://*/*', 'https://*/*'], js: ['content.js'], run_at: 'document_start', all_frames: true },
          { matches: ['http://*/*', 'https://*/*'], js: ['scroll-guard.js'], run_at: 'document_end' },
        ],
        content_security_policy: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'",
      },
      null,
      2,
    ),
  );
  console.log(`gecko-mv2: engine.dat ${(readFileSync(path.join(out, 'engine.dat')).length / 1e6).toFixed(1)} MB, filters ${manifest.version} → ${out}`);
  process.exit(0);
}
if (target !== 'chromium-mv3') throw new Error(`unknown target ${target}`);

// Cosmetics without a service worker (WebView2 did not deliver worker messages, ADR-0012):
// all generic selectors as one stylesheet the browser injects itself, host rules as shards.
const generic = JSON.parse(readFileSync(path.join(bundle, 'cosmetic', 'generic.json'), 'utf8'));
const hosts = JSON.parse(readFileSync(path.join(bundle, 'cosmetic', 'hosts.json'), 'utf8'));
const excepted = new Set(generic.exceptions ?? []);
const selectors = [...Object.values(generic.classes ?? {}).flat(), ...Object.values(generic.ids ?? {}).flat(), ...(generic.other ?? [])]
  .filter((s) => !excepted.has(s));
const css = [];
// One invalid selector drops its whole rule, so rules stay small.
for (let i = 0; i < selectors.length; i += 50) css.push(`${selectors.slice(i, i + 50).join(',\n')}{display:none!important}`);
writeFileSync(path.join(out, 'cosmetic', 'generic.css'), css.join('\n') + '\n');
mkdirSync(path.join(out, 'cosmetic', 'hosts'), { recursive: true });
const shards = new Map();
for (const [host, rules] of Object.entries(hosts)) {
  const s = shardOf(host);
  if (!shards.has(s)) shards.set(s, {});
  shards.get(s)[host] = { hide: rules.hide, unhide: rules.unhide };
}
for (const [s, index] of shards) writeFileSync(path.join(out, 'cosmetic', 'hosts', `${s}.json`), JSON.stringify(index));
const rulesets = manifest.dnr.map((r) => {
  copyFileSync(path.join(bundle, r.path), path.join(out, r.path));
  return { id: r.id, enabled: true, path: r.path };
});
copyFileSync(path.join(here, 'src', 'content-static.js'), path.join(out, 'content.js'));
copyFileSync(path.join(here, 'src', 'scroll-guard.js'), path.join(out, 'scroll-guard.js'));

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
      content_scripts: [
        { matches: ['http://*/*', 'https://*/*'], css: ['cosmetic/generic.css'], js: ['content.js'], run_at: 'document_start', all_frames: true },
        { matches: ['http://*/*', 'https://*/*'], js: ['scroll-guard.js'], run_at: 'document_end' },
      ],
      web_accessible_resources: [{ resources: ['cosmetic/hosts/*.json'], matches: ['http://*/*', 'https://*/*'] }],
      declarative_net_request: { rule_resources: rulesets },
    },
    null,
    2,
  ),
);
console.log(`${target}: ${rulesets.length} DNR rulesets, filters ${manifest.version} → ${out}`);
console.log(readdirSync(out).join(' '));

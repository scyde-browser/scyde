/* global wasm_bindgen, browser */
// GeckoView built-in Shield (ADR-0004): every request is decided by the WASM engine
// (core/shield-wasm) loaded from the bundle's engine.dat; cosmetics come from the same engine.
// Until the engine is ready requests pass; the app waits for `ready` before the first page load.
const NATIVE = 'scyde';
let engine = null;
let blocked = 0;
let reported = 0;
const exceptions = new Map(); // host → exception selectors for generic hiding

browser.webRequest.onBeforeRequest.addListener(
  (d) => {
    if (!engine || !/^(https?|wss?):/.test(d.url)) return {};
    const source = d.documentUrl || d.originUrl || d.url;
    if (engine.shouldBlock(d.url, source, d.type)) {
      blocked++;
      return { cancel: true };
    }
    return {};
  },
  { urls: ['<all_urls>'] },
  ['blocking'],
);

// Counter for the island and the New Tab (batched, every 2 s at most).
setInterval(() => {
  if (blocked !== reported) {
    reported = blocked;
    browser.runtime.sendNativeMessage(NATIVE, { type: 'shield.count', blocked }).catch(() => {});
  }
}, 2000);

browser.runtime.onMessage.addListener(async (m) => {
  if (!engine || !m?.host) return null;
  if (m.type === 'scyde.cosmetics') {
    const c = JSON.parse(engine.cosmetics(`https://${m.host}/`));
    exceptions.set(m.host, c.exceptions);
    if (c.script) return { hide: c.hide, script: c.script };
    return { hide: c.hide };
  }
  if (m.type === 'scyde.generic') {
    const ex = JSON.stringify(exceptions.get(m.host) ?? []);
    return { hide: JSON.parse(engine.hiddenSelectors(JSON.stringify(m.classes), JSON.stringify(m.ids), ex)) };
  }
  return null;
});

(async () => {
  const url = (p) => browser.runtime.getURL(p);
  try {
    await wasm_bindgen({ module_or_path: url('engine/shield_wasm_bg.wasm') });
    const dat = new Uint8Array(await (await fetch(url('engine.dat'))).arrayBuffer());
    engine = wasm_bindgen.ShieldEngine.fromDat(dat);
    try {
      engine.useResources(await (await fetch(url('resources.json'))).text());
    } catch {}
    await browser.runtime.sendNativeMessage(NATIVE, { type: 'ready' });
  } catch (e) {
    await browser.runtime.sendNativeMessage(NATIVE, { type: 'error', error: String(e) }).catch(() => {});
  }
})();

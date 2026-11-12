/* global wasm_bindgen, browser */
// GeckoView built-in Shield (ADR-0004): every request is decided by the WASM engine
// (core/shield-wasm) loaded from the bundle's engine.dat; cosmetics come from the same engine.
// Until the engine is ready requests pass; the app waits for `ready` before the first page load.
const NATIVE = 'scyde';
let engine = null;
let blocked = 0;
let reported = 0;
const exceptions = new Map(); // host → exception selectors for generic hiding
// Per-site Shield (shield.disabled_sites, sent by the app over a native port).
let disabled = new Set();
// Requests blocked per page host since its last page load; changed hosts are reported.
const perHost = new Map();
const changed = new Set();

const hostOf = (url) => {
  try {
    return new URL(url).hostname;
  } catch {
    return '';
  }
};
/** Shield is off for `host` when it or a parent domain is in the list (entries are eTLD+1). */
const off = (host) => {
  for (let h = host; h; h = h.slice(h.indexOf('.') + 1)) {
    if (disabled.has(h)) return true;
    if (!h.includes('.')) break;
  }
  return false;
};

try {
  const port = browser.runtime.connectNative(NATIVE);
  port.onMessage.addListener((m) => {
    // The app sends display names (IDNs in Unicode); requests carry punycode hosts.
    if (m?.type === 'allowlist') disabled = new Set((m.sites ?? []).map((s) => hostOf(`https://${s}/`) || s));
  });
} catch {}

browser.webRequest.onBeforeRequest.addListener(
  (d) => {
    if (!/^(https?|wss?):/.test(d.url)) return {};
    // The top page: the last frame ancestor, else the document itself (or the request for a page load).
    const top = d.type === 'main_frame' ? d.url : (d.frameAncestors?.at(-1)?.url ?? d.documentUrl ?? d.originUrl ?? d.url);
    const topHost = hostOf(top);
    if (d.type === 'main_frame') {
      perHost.set(topHost, 0);
      changed.add(topHost);
    }
    if (!engine || off(topHost)) return {};
    const source = d.documentUrl || d.originUrl || d.url;
    if (engine.shouldBlock(d.url, source, d.type)) {
      blocked++;
      perHost.set(topHost, (perHost.get(topHost) ?? 0) + 1);
      changed.add(topHost);
      return { cancel: true };
    }
    return {};
  },
  { urls: ['<all_urls>'] },
  ['blocking'],
);

// Counters for the island and the New Tab (batched, every 1 s at most).
setInterval(() => {
  if (blocked === reported && !changed.size) return;
  reported = blocked;
  const hosts = Object.fromEntries([...changed].map((h) => [h, perHost.get(h) ?? 0]));
  changed.clear();
  browser.runtime.sendNativeMessage(NATIVE, { type: 'shield.count', blocked, hosts }).catch(() => {});
}, 1000);

browser.runtime.onMessage.addListener(async (m) => {
  if (!engine || !m?.host) return null;
  if (off(m.top ?? m.host)) return { hide: [] };
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

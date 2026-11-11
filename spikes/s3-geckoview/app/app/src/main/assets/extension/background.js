/* global wasm_bindgen, browser */
// Spike S3 background: WASM Shield engine decides every request (webRequestBlocking).
const NATIVE = 'scyde';
const LISTS = ['fixture.txt', 'easylist.txt', 'easyprivacy.txt', 'filters.txt'];
let engine = null;
const counters = { checked: 0, blocked: 0 };

const send = (msg) => browser.runtime.sendNativeMessage(NATIVE, msg).catch((e) => console.error('native', e));

// Registered synchronously so no request slips through before the engine is ready (pass-through until then).
browser.webRequest.onBeforeRequest.addListener(
  (d) => {
    if (!engine || d.url.startsWith('moz-extension:') || d.url.startsWith('data:')) return {};
    const source = d.documentUrl || d.originUrl || d.url;
    counters.checked++;
    if (engine.should_block(d.url, source, d.type)) {
      counters.blocked++;
      return { cancel: true };
    }
    return {};
  },
  { urls: ['<all_urls>'] },
  ['blocking'],
);

browser.runtime.onMessage.addListener(async (m, sender) => {
  if (!engine) return null;
  if (m.type === 'cosmetic') return JSON.parse(engine.cosmetic(m.url));
  if (m.type === 'generic') return engine.hidden_selectors(m.classes, m.ids, JSON.stringify(m.exceptions || []));
  if (m.type === 'fixture-result') {
    await send({ type: 'fixture', url: sender.url, ...m.data, counters });
    return true;
  }
  return null;
});

function timerResolutionUs() {
  let min = Infinity;
  for (let i = 0; i < 2000; i++) {
    const a = performance.now();
    let b = performance.now();
    while (b === a) b = performance.now();
    min = Math.min(min, b - a);
  }
  return min * 1000;
}

// Per-request cost in SpiderMonkey. Timer precision in extension contexts may be coarse, so every
// request is repeated REPS times and timed as a block; p99 is over distinct requests.
function bench(text) {
  let seed = 42;
  const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 2 ** 32);
  const pick = (a) => a[Math.floor(rnd() * a.length)];
  const adHosts = [...text.matchAll(/^\|\|([a-z0-9.-]+\.[a-z]{2,})\^/gm)].map((m) => m[1]);
  const normal = ['www.index.hr', 'cdn.jsdelivr.net', 'fonts.gstatic.com', 'hr.wikipedia.org', 'www.jutarnji.hr', 'i.ytimg.com', 'cdnjs.cloudflare.com'];
  const paths = ['/static/js/app.', '/assets/img/', '/api/v1/', '/ads/', '/pixel', '/track', '/banner/', '/news/2026/'];
  const types = ['script', 'image', 'stylesheet', 'xmlhttprequest', 'sub_frame', 'font', 'media', 'other'];
  const N = 3000;
  const REPS = 50;
  const costs = new Float64Array(N);
  for (let i = 0; i < 500; i++) engine.should_block(`https://${pick(normal)}/warm${i}.js`, 'https://www.index.hr/', 'script');
  for (let i = 0; i < N; i++) {
    const host = rnd() < 0.3 && adHosts.length ? pick(adHosts) : pick(normal);
    const url = `https://${host}${pick(paths)}${Math.floor(rnd() * 1e6).toString(36)}.js`;
    const src = `https://${pick(normal)}/`;
    const type = pick(types);
    const t0 = performance.now();
    for (let r = 0; r < REPS; r++) engine.should_block(url, src, type);
    costs[i] = ((performance.now() - t0) * 1000) / REPS;
  }
  costs.sort();
  const q = (p) => costs[Math.min(N - 1, Math.floor(p * N))];
  return { n: N, reps: REPS, p50_us: q(0.5), p90_us: q(0.9), p99_us: q(0.99), max_us: costs[N - 1], timer_resolution_us: timerResolutionUs(), ad_hosts: adHosts.length };
}

(async () => {
  const t0 = performance.now();
  await wasm_bindgen({ module_or_path: browser.runtime.getURL('engine/scyde_shield_wasm_spike_bg.wasm') });
  const texts = await Promise.all(LISTS.map((f) => fetch(browser.runtime.getURL(`lists/${f}`)).then((r) => (r.ok ? r.text() : '')).catch(() => '')));
  const text = texts.join('\n');
  engine = new wasm_bindgen.ShieldEngine(text);
  const initMs = performance.now() - t0;
  await send({ type: 'ready', init_ms: initMs, rule_lines: text.split('\n').filter((l) => l && !l.startsWith('!')).length, ua: navigator.userAgent });
  await send({ type: 'bench', ...bench(text) });
})().catch((e) => send({ type: 'error', error: String(e && e.stack || e) }));

// Shield cosmetics without a background worker (chromium-mv3 / WebView2, ADR-0012).
// Generic rules come from cosmetic/generic.css, injected by the browser (manifest "css").
// Host rules: fetch the shard for each suffix of the host (web-accessible JSON), hide selectors.
(() => {
  if (!/^https?:$/.test(location.protocol)) return;
  const api = globalThis.chrome ?? globalThis.browser;
  const SHARDS = 256;
  const shardOf = (host) => {
    let h = 0x811c9dc5;
    for (let i = 0; i < host.length; i++) {
      h ^= host.charCodeAt(i);
      h = Math.imul(h, 0x01000193) >>> 0;
    }
    return (h % SHARDS).toString(16).padStart(2, '0');
  };
  const parts = location.hostname.split('.');
  const chain = parts.map((_, i) => parts.slice(i).join('.')).filter((h) => h.includes('.') || h === 'localhost');
  const byShard = new Map();
  for (const h of chain) {
    const s = shardOf(h);
    byShard.set(s, [...(byShard.get(s) ?? []), h]);
  }
  Promise.all(
    [...byShard].map(([s, hosts]) =>
      fetch(api.runtime.getURL(`cosmetic/hosts/${s}.json`))
        .then((r) => (r.ok ? r.json() : {}))
        .then((index) => hosts.map((h) => index[h]).filter(Boolean))
        .catch(() => []),
    ),
  ).then((found) => {
    const rules = found.flat();
    const unhide = new Set(rules.flatMap((r) => r.unhide ?? []));
    const hide = rules.flatMap((r) => r.hide ?? []).filter((s) => !unhide.has(s));
    const css = [];
    for (let i = 0; i < hide.length; i += 200) css.push(`${hide.slice(i, i + 200).join(',\n')}{display:none!important}`);
    // Generic selectors this host excepts (`example.com#@#.ad`) are shown again.
    if (unhide.size) css.push(`${[...unhide].join(',\n')}{display:revert!important}`);
    if (!css.length) return;
    const style = document.createElement('style');
    style.dataset.scyde = 'shield';
    style.textContent = css.join('\n');
    (document.head ?? document.documentElement).append(style);
  });
})();

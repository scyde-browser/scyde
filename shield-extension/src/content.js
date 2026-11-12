// Shield cosmetics (shared by both targets). Runs at document_start in every frame.
// 1. Host rules + generic "other" selectors as soon as the document starts.
// 2. Generic class/id selectors for what is actually on the page, once, at DOMContentLoaded,
//    then for added nodes in batches (MutationObserver, at most every 250 ms).
(() => {
  const api = globalThis.browser ?? globalThis.chrome;
  if (!api?.runtime?.sendMessage || !/^https?:$/.test(location.protocol)) return;
  const host = location.hostname;
  const seen = new Set();
  let style;

  function inject(selectors) {
    if (!selectors?.length) return;
    style ??= document.createElement('style');
    style.dataset.scyde = 'shield';
    // Chunks keep one invalid selector from disabling the rest.
    for (let i = 0; i < selectors.length; i += 500) {
      style.append(`${selectors.slice(i, i + 500).join(',\n')}{display:none!important}\n`);
    }
    (document.head ?? document.documentElement)?.append(style);
  }

  function collect(root) {
    const classes = [];
    const ids = [];
    for (const el of root.querySelectorAll?.('[class],[id]') ?? []) {
      if (el.id && !seen.has('#' + el.id)) (seen.add('#' + el.id), ids.push(el.id));
      for (const c of el.classList ?? []) if (!seen.has('.' + c)) (seen.add('.' + c), classes.push(c));
    }
    return { classes, ids };
  }

  async function ask(msg) {
    try {
      return await api.runtime.sendMessage(msg);
    } catch {
      return null;
    }
  }

  // The page's top host decides per-site Shield; an iframe uses its embedder from the referrer.
  let top = host;
  if (window.top !== window) {
    try {
      top = new URL(document.referrer).hostname || host;
    } catch {}
  }

  ask({ type: 'scyde.cosmetics', host, top }).then((r) => inject(r?.hide));

  let pending = false;
  const generic = async (root) => {
    const { classes, ids } = collect(root);
    if (!classes.length && !ids.length) return;
    const r = await ask({ type: 'scyde.generic', host, top, classes, ids });
    inject(r?.hide);
  };
  // Aura tint (DESIGN.md §9): the top page's theme-color, as #RRGGBB, for the island (GeckoView only).
  function themeColor() {
    if (window.top !== window || !api.runtime.sendNativeMessage) return;
    const metas = [...document.querySelectorAll('meta[name="theme-color" i]')];
    const meta = metas.find((m) => !m.media || matchMedia(m.media).matches);
    let hex = null;
    if (meta?.content) {
      const probe = document.createElement('i');
      probe.style.color = meta.content;
      document.documentElement.append(probe);
      const rgb = getComputedStyle(probe).color.match(/[\d.]+/g);
      probe.remove();
      if (rgb && rgb.length >= 3 && !(rgb.length > 3 && Number(rgb[3]) === 0)) {
        hex = '#' + rgb.slice(0, 3).map((x) => Math.round(Number(x)).toString(16).padStart(2, '0')).join('');
      }
    }
    api.runtime.sendNativeMessage('scyde', { type: 'theme', color: hex }).catch(() => {});
  }

  // Back/forward from the page cache and in-page navigations do not run this script again.
  addEventListener('pageshow', (e) => e.persisted && themeColor());
  addEventListener('popstate', themeColor);
  addEventListener('hashchange', themeColor);

  const start = () => {
    themeColor();
    generic(document);
    new MutationObserver(() => {
      if (pending) return;
      pending = true;
      setTimeout(() => ((pending = false), generic(document)), 250);
    }).observe(document.documentElement, { childList: true, subtree: true });
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true });
  else start();
})();

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

  ask({ type: 'scyde.cosmetics', host }).then((r) => inject(r?.hide));

  let pending = false;
  const generic = async (root) => {
    const { classes, ids } = collect(root);
    if (!classes.length && !ids.length) return;
    const r = await ask({ type: 'scyde.generic', host, classes, ids });
    inject(r?.hide);
  };
  const start = () => {
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

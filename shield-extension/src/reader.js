// Reader (own code): shows the article of the current page as clean text over the page, and
// removes it again when asked a second time. Shared by iOS (injected into an isolated content
// world, called by the app) and GeckoView (content script, asked by the background page).
// Everything is built with createElement/textContent: no page HTML is copied as HTML.
(() => {
  if (window.top !== window) return;
  const ID = 'scyde-reader';
  const SKIP = 'nav, aside, footer, form, header nav, [aria-hidden="true"], [hidden], figure figcaption ~ *';

  function pick() {
    let best = null;
    let score = 0;
    for (const el of document.querySelectorAll('article, main, [role="main"], [itemprop="articleBody"], section, div')) {
      let n = 0;
      for (const p of el.querySelectorAll(':scope > p, :scope > * > p')) {
        const len = p.textContent.trim().length;
        if (len >= 80) n += len;
      }
      if (el.tagName === 'ARTICLE' || el.getAttribute('itemprop') === 'articleBody') n *= 1.25;
      if (n > score) (best = el), (score = n);
    }
    return score >= 600 ? best : null;
  }

  function blocks(root) {
    const out = [];
    for (const el of root.querySelectorAll('h1, h2, h3, p, blockquote, li, pre, img')) {
      if (el.closest(SKIP)) continue;
      if (el.tagName === 'IMG') {
        const src = el.currentSrc || el.src;
        if (/^https?:/.test(src) && (el.naturalWidth || el.width) >= 300) out.push({ tag: 'img', src, alt: el.alt || '' });
        continue;
      }
      // A paragraph inside a quote or a list item is covered by its parent.
      if (el.tagName === 'P' && el.closest('blockquote, li')) continue;
      const text = el.textContent.replace(/\s+/g, ' ').trim();
      if (text.length < 2) continue;
      if (el.tagName === 'LI') {
        const links = [...el.querySelectorAll('a')].reduce((n, a) => n + a.textContent.length, 0);
        if (links * 2 > text.length) continue; // menus and "related" lists
      }
      out.push({ tag: el.tagName.toLowerCase(), text });
    }
    return out;
  }

  /** opts: { bg, fg, muted, accent, close } → true if the reader is now open or was closed, false if no article. */
  globalThis.__scydeReader = (opts = {}) => {
    const open = document.getElementById(ID);
    if (open) {
      open.remove();
      document.documentElement.style.overflow = open.dataset.overflow || '';
      return true;
    }
    const root = pick();
    const content = root ? blocks(root) : [];
    if (content.filter((b) => b.tag === 'p').length < 3) return false;

    const host = document.createElement('div');
    host.id = ID;
    host.dataset.overflow = document.documentElement.style.overflow;
    const shadow = host.attachShadow({ mode: 'closed' });
    const css = document.createElement('style');
    const color = (v, d) => (/^#[0-9a-f]{6}$/i.test(v || '') ? v : d);
    css.textContent = `
      :host { all: initial; position: fixed; inset: 0; z-index: 2147483647; overflow-y: auto; -webkit-overflow-scrolling: touch;
        background: ${color(opts.bg, '#F3F1EC')}; color: ${color(opts.fg, '#121214')}; font: 19px/1.6 -apple-system, system-ui, sans-serif; }
      main { box-sizing: border-box; max-width: 680px; margin: 0 auto; padding: 24px 20px 160px; }
      h1 { font-size: 1.7em; line-height: 1.2; margin: 0.4em 0 0.8em; }
      h2 { font-size: 1.3em; line-height: 1.25; margin: 1.6em 0 0.5em; }
      h3 { font-size: 1.1em; margin: 1.4em 0 0.4em; }
      p, li, blockquote, pre { margin: 0 0 1em; overflow-wrap: anywhere; }
      li { margin-left: 1.2em; display: list-item; }
      blockquote { padding-left: 16px; border-left: 3px solid ${color(opts.accent, '#F2603F')}; color: ${color(opts.muted, '#5B5A57')}; }
      pre { white-space: pre-wrap; font: 0.85em/1.5 ui-monospace, monospace; }
      img { display: block; max-width: 100%; height: auto; margin: 0 0 1em; border-radius: 12px; }
      small { display: block; color: ${color(opts.muted, '#5B5A57')}; font-size: 0.75em; margin-bottom: 0.5em; }
      button { position: sticky; top: 12px; float: right; width: 44px; height: 44px; border: 0; border-radius: 22px;
        background: ${color(opts.fg, '#121214')}; color: ${color(opts.bg, '#F3F1EC')}; font: 22px/1 system-ui, sans-serif; }
    `;
    const main = document.createElement('main');
    const close = document.createElement('button');
    close.textContent = '×';
    close.setAttribute('aria-label', typeof opts.close === 'string' ? opts.close.slice(0, 40) : 'Close');
    close.addEventListener('click', () => globalThis.__scydeReader());
    const site = document.createElement('small');
    site.textContent = location.hostname.replace(/^www\./, '');
    main.append(close, site);
    if (!content.some((b) => b.tag === 'h1')) {
      const h = document.createElement('h1');
      h.textContent = (document.querySelector('h1')?.textContent || document.title).replace(/\s+/g, ' ').trim();
      main.append(h);
    }
    for (const b of content) {
      const el = document.createElement(b.tag);
      if (b.tag === 'img') (el.src = b.src), (el.alt = b.alt), (el.referrerPolicy = 'no-referrer');
      else el.textContent = b.text;
      main.append(el);
    }
    shadow.append(css, main);
    document.documentElement.append(host);
    document.documentElement.style.overflow = 'hidden';
    return true;
  };

  // GeckoView: the app asks through the extension's background page.
  const api = globalThis.browser ?? globalThis.chrome;
  if (api?.runtime?.connect) {
    try {
      const port = api.runtime.connect({ name: 'scyde-reader' });
      port.onMessage.addListener((m) => {
        if (m?.type !== 'reader' || document.visibilityState !== 'visible') return;
        const ok = globalThis.__scydeReader(m.opts);
        if (!ok) port.postMessage({ type: 'reader.none' });
      });
    } catch {}
  }
})();

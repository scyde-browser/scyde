// Shared prototype runtime: i18n, icons, brand mark, theme switcher. No network, no framework.
(() => {
  const q = new URLSearchParams(location.search);
  const root = document.documentElement;
  const lang = q.get('lang') || 'hr';
  root.lang = lang;
  for (const k of ['theme', 'accent', 'density', 'motion']) if (q.get(k)) root.dataset[k] = q.get(k);
  if (!root.dataset.theme) root.dataset.theme = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';

  const dict = (window.SCYDE_STRINGS || {})[lang] || {};
  const fmtNum = (n) => new Intl.NumberFormat(lang === 'hr' ? 'hr-HR' : 'en-US').format(n);
  const t = (key, params = {}) => {
    let s = dict[key];
    if (s == null) {
      console.warn('missing i18n key', key);
      return key;
    }
    s = s.replace(/\{(\w+), plural,[^}]*\{[^}]*\}[^}]*\}\}/g, (m, v) => fmtNum(params[v])); // prototypes: plural shortcut
    return s.replace(/\{(\w+)\}/g, (_, v) => (typeof params[v] === 'number' ? fmtNum(params[v]) : params[v] ?? ''));
  };
  window.t = t;
  window.fmtNum = fmtNum;

  // Lucide-style 1.5 px line icons (ISC licence, paths redrawn on a 24 grid).
  const I = {
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
    mic: '<rect x="9" y="2" width="6" height="12" rx="3"/><path d="M19 10v1a7 7 0 0 1-14 0v-1"/><path d="M12 18v4"/>',
    image: '<rect x="3" y="3" width="18" height="18" rx="3"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.1-3.1a2 2 0 0 0-2.8 0L6 21"/>',
    shield: '<path d="M20 13c0 5-3.5 7.5-7.7 9a1 1 0 0 1-.6 0C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.2-2.7a1.2 1.2 0 0 1 1.6 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z"/>',
    'shield-check': '<path d="M20 13c0 5-3.5 7.5-7.7 9a1 1 0 0 1-.6 0C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.2-2.7a1.2 1.2 0 0 1 1.6 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>',
    plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
    x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    back: '<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>',
    forward: '<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>',
    reload: '<path d="M21 12a9 9 0 1 1-9-9c2.5 0 4.9 1 6.7 2.7L21 8"/><path d="M21 3v5h-5"/>',
    lock: '<rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>',
    more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    star: '<path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.3l-5.6 2.9 1.1-6.2L3 9.6l6.2-.9z"/>',
    sliders: '<path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15" cy="6" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="18" r="2"/>',
    news: '<path d="M4 21h15a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2H8a2 2 0 0 0-2 2v14a2 2 0 0 1-2 2Zm0 0a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2"/><path d="M10 7h7v4h-7zM10 15h7"/>',
    share: '<path d="M4 12v7a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-7"/><path d="m16 6-4-4-4 4"/><path d="M12 2v13"/>',
    tabs: '<rect x="8" y="8" width="13" height="13" rx="3"/><path d="M4 16a2 2 0 0 1-2-2V5a3 3 0 0 1 3-3h9a2 2 0 0 1 2 2"/>',
    moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>',
    'eye-off': '<path d="M10 10a3 3 0 1 0 4 4"/><path d="M10.7 5.1A10 10 0 0 1 12 5c7 0 10 7 10 7a13 13 0 0 1-1.7 2.7"/><path d="M6.6 6.6A13.5 13.5 0 0 0 2 12s3 7 10 7a9.7 9.7 0 0 0 5.4-1.6"/><path d="m2 2 20 20"/>',
    leaf: '<path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.5 19 2c1 2 2 4.2 2 8 0 5.5-4.8 10-10 10Z"/><path d="M2 21c0-3 1.9-5.4 5.1-6C9.5 14.5 12 13 13 12"/>',
    chevron: '<path d="m9 18 6-6-6-6"/>',
    panel: '<rect x="3" y="3" width="18" height="18" rx="3"/><path d="M9 3v18"/>',
    globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>',
    mail: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m3 7 9 6 9-6"/>',
    chat: '<path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12Z"/>',
    send: '<path d="m22 2-7 20-4-9-9-4z"/><path d="M22 2 11 13"/>',
    min: '<path d="M5 12h14"/>',
    max: '<rect x="5" y="5" width="14" height="14" rx="1"/>',
    home: '<path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
  };
  const svg = (name) =>
    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${I[name] || ''}</svg>`;
  window.icon = svg;

  // Brand mark "Aura S" as a CSS mask so the Ember gradient comes from the token (--gradient-ember).
  const S =
    "<svg xmlns='http://www.w3.org/2000/svg' viewBox='313 197 398 630'><g fill='none' stroke='black' stroke-width='118'>" +
    "<path d='M500 256A128 128 0 0 0 500 512'/><path d='M524 512A128 128 0 0 1 524 768'/></g>" +
    "<circle cx='500' cy='256' r='59'/><circle cx='524' cy='768' r='59'/></svg>";
  window.SCYDE_MARK_MASK = `url("data:image/svg+xml,${encodeURIComponent(S)}")`;
  root.style.setProperty('--brand-mark-mask', window.SCYDE_MARK_MASK);

  function apply(scope = document) {
    scope.querySelectorAll('[data-icon]').forEach((el) => (el.innerHTML = svg(el.dataset.icon)));
    scope.querySelectorAll('[data-i18n]').forEach((el) => {
      const params = el.dataset.i18nParams ? JSON.parse(el.dataset.i18nParams) : {};
      el.textContent = t(el.dataset.i18n, params);
    });
    scope.querySelectorAll('[data-i18n-attr]').forEach((el) => {
      for (const pair of el.dataset.i18nAttr.split(';')) {
        const [attr, key] = pair.split(':');
        el.setAttribute(attr, t(key));
      }
    });
  }
  window.applyProto = apply;

  function switcher() {
    if (q.has('clean')) return;
    const box = document.createElement('div');
    box.className = 'proto-switcher';
    const sel = (label, key, opts) =>
      `<label>${t(label)} <select data-k="${key}">${opts.map((o) => `<option ${root.dataset[key] === o ? 'selected' : ''}>${o}</option>`).join('')}</select></label>`;
    box.innerHTML =
      sel('proto.theme', 'theme', ['light', 'dark']) +
      sel('proto.accent', 'accent', ['sky', 'periwinkle', 'lavender', 'mint', 'ember', 'onyx']) +
      sel('proto.density', 'density', ['comfortable', 'compact']) +
      sel('proto.motion', 'motion', ['full', 'reduced']) +
      `<label>${t('proto.lang')} <select data-k="lang"><option ${lang === 'hr' ? 'selected' : ''}>hr</option><option ${lang === 'en' ? 'selected' : ''}>en</option></select></label>`;
    box.addEventListener('change', (e) => {
      const k = e.target.dataset.k;
      if (k === 'lang') {
        q.set('lang', e.target.value);
        location.search = q.toString();
        return;
      }
      const go = () => (root.dataset[k] = e.target.value);
      // Dissolve (DESIGN §6.3): theme change cross-fades via View Transitions where available.
      if (k === 'theme' && document.startViewTransition && root.dataset.motion !== 'reduced') document.startViewTransition(go);
      else go();
    });
    document.body.appendChild(box);
  }

  document.addEventListener('DOMContentLoaded', () => {
    apply();
    switcher();
  });
})();

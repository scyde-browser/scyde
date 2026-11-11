/* global browser */
// Spike S3 content script: cosmetic filtering + reporting the qa fixture result to the app.
(() => {
  const style = document.createElement('style');
  const hide = (selectors) => {
    if (!selectors || !selectors.length) return;
    style.textContent += `${selectors.join(',\n')}{display:none !important}\n`;
    (document.head || document.documentElement).appendChild(style);
  };
  let exceptions = [];
  browser.runtime.sendMessage({ type: 'cosmetic', url: location.href }).then((r) => {
    if (!r) return;
    exceptions = r.exceptions || [];
    hide(r.hide);
  });
  const generic = () => {
    const classes = new Set();
    const ids = new Set();
    for (const el of document.querySelectorAll('[class],[id]')) {
      el.classList.forEach((c) => classes.add(c));
      if (el.id) ids.add(el.id);
    }
    browser.runtime.sendMessage({ type: 'generic', classes: [...classes], ids: [...ids], exceptions }).then(hide);
  };
  document.addEventListener('DOMContentLoaded', generic, { once: true });

  // qa/adblock reports into window.__scydeQa; Gecko exposes page globals via wrappedJSObject.
  if (location.pathname.includes('/adblock/')) {
    let tries = 0;
    const poll = setInterval(() => {
      const qa = window.wrappedJSObject && window.wrappedJSObject.__scydeQa;
      if (qa || ++tries > 120) {
        clearInterval(poll);
        if (qa) browser.runtime.sendMessage({ type: 'fixture-result', data: JSON.parse(JSON.stringify(qa)) });
      }
    }, 250);
  }
})();

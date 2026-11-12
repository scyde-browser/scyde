// Scroll guard (all platforms). When Shield hides a site's overlay (an adblock wall, a consent
// modal), the scroll lock the site put on <html>/<body> stays behind and the page cannot scroll.
// The guard unlocks scrolling only while no visible overlay covers the viewport, so a real dialog
// keeps its lock. Main frame only; cheap: two computed styles and three hit tests per check.
(() => {
  if (window.top !== window || !/^https?:$/.test(location.protocol)) return;
  const root = document.documentElement;
  let applied = false;

  const lockedBy = (el) => {
    if (!el) return false;
    const s = getComputedStyle(el);
    return s.overflowY === 'hidden' || s.overflowY === 'clip' || (el === document.body && s.position === 'fixed');
  };

  /** A visible fixed layer over a large part of the viewport (a dialog the user can see). */
  const covered = () => {
    const w = innerWidth;
    const h = innerHeight;
    for (const [x, y] of [[w / 2, h / 2], [w / 4, h / 3], [(3 * w) / 4, (2 * h) / 3]]) {
      for (let el = document.elementFromPoint(x, y); el && el !== document.body && el !== root; el = el.parentElement) {
        const s = getComputedStyle(el);
        if (s.position !== 'fixed' && s.position !== 'sticky') continue;
        const r = el.getBoundingClientRect();
        if (r.width * r.height > 0.4 * w * h && s.visibility !== 'hidden' && Number(s.opacity) > 0.05) return true;
      }
    }
    return false;
  };

  const set = (on) => {
    for (const el of [root, document.body]) {
      if (!el) continue;
      if (on) el.style.setProperty('overflow-y', 'auto', 'important');
      else el.style.removeProperty('overflow-y');
    }
    if (document.body) {
      if (on && getComputedStyle(document.body).position === 'fixed') {
        document.body.dataset.scydeScroll = '1';
        document.body.style.setProperty('position', 'static', 'important');
      } else if (!on && document.body.dataset.scydeScroll) {
        delete document.body.dataset.scydeScroll;
        document.body.style.removeProperty('position');
      }
    }
    applied = on;
  };

  const check = () => {
    if (!document.body) return;
    if (applied) {
      if (covered()) set(false);
      return;
    }
    const tall = Math.max(root.scrollHeight, document.body.scrollHeight) > innerHeight + 48;
    if (tall && (lockedBy(root) || lockedBy(document.body)) && !covered()) set(true);
  };

  let queued = false;
  const soon = () => {
    if (queued) return;
    queued = true;
    setTimeout(() => {
      queued = false;
      check();
    }, 300);
  };
  const start = () => {
    check();
    const mo = new MutationObserver(soon);
    mo.observe(root, { attributes: true, attributeFilter: ['class', 'style'] });
    if (document.body) mo.observe(document.body, { attributes: true, attributeFilter: ['class', 'style'] });
    // Overlays often appear late (after ads fail): re-check for the first 30 s.
    let n = 0;
    const t = setInterval(() => {
      check();
      if (++n >= 30) clearInterval(t);
    }, 1000);
  };
  if (document.readyState === 'loading') addEventListener('DOMContentLoaded', start, { once: true });
  else start();
})();

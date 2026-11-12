import { test, expect } from '@playwright/test';
import { fileURLToPath } from 'node:url';

// shield-extension/src/scroll-guard.js in all three engines (it also ships in the iOS app).
const GUARD = fileURLToPath(new URL('../../../shield-extension/src/scroll-guard.js', import.meta.url));
const LONG = '<div style="height:4000px">content</div>';

async function load(page, body, style = '') {
  await page.route('http://qa.scyde.test/**', (r) =>
    r.fulfill({ contentType: 'text/html', body: `<!doctype html><style>${style}</style><body>${body}</body>` }),
  );
  await page.goto('http://qa.scyde.test/');
  await page.addScriptTag({ path: GUARD });
}
const overflow = (page) => page.evaluate(() => getComputedStyle(document.body).overflowY);

test('unlocks a scroll lock left behind by a hidden overlay', async ({ page }) => {
  await load(page, `${LONG}<div class="wall" style="position:fixed;inset:0;display:none">ad wall</div>`, 'html,body{overflow:hidden}');
  await expect.poll(() => overflow(page)).toBe('auto');
  await page.mouse.wheel(0, 800);
  await expect.poll(() => page.evaluate(() => scrollY)).toBeGreaterThan(0);
});

test('unlocks a position:fixed body lock', async ({ page }) => {
  await load(page, LONG, 'body{position:fixed;top:0;width:100%}');
  await expect.poll(() => page.evaluate(() => getComputedStyle(document.body).position)).toBe('static');
});

test('keeps the lock while a visible dialog covers the page', async ({ page }) => {
  await load(page, `${LONG}<div style="position:fixed;inset:0;background:#0008">dialog</div>`, 'body{overflow:hidden}');
  await page.waitForTimeout(1500);
  expect(await overflow(page)).toBe('hidden');
});

test('leaves app-like pages (inner scroll, no tall document) alone', async ({ page }) => {
  await load(page, '<div style="height:100vh;overflow:auto">app</div>', 'html,body{overflow:hidden;margin:0}');
  await page.waitForTimeout(1500);
  expect(await overflow(page)).toBe('hidden');
});

test('re-locks when the site opens a real dialog later', async ({ page }) => {
  await load(page, `${LONG}<div id="d" style="position:fixed;inset:0;display:none">dialog</div>`, 'body{overflow:hidden}');
  await expect.poll(() => overflow(page)).toBe('auto');
  await page.evaluate(() => {
    document.getElementById('d').style.display = 'block';
    document.body.className = 'modal-open';
  });
  await expect.poll(() => overflow(page)).toBe('hidden');
});

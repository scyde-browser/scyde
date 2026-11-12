import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mkdirSync } from 'node:fs';

const PAGES = ['newtab', 'settings', 'shields', 'library', 'welcome'];
const SHOTS = process.env.SHOTS_DIR ?? 'screenshots';

/** Collects console errors and CSP violations; both fail the test. */
function watch(page) {
  const errors = [];
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('pageerror', (e) => errors.push(String(e)));
  return errors;
}

for (const app of PAGES) {
  for (const theme of ['light', 'dark']) {
    test(`${app} · ${theme}: renders, no errors, axe clean, screenshots`, async ({ page, browserName }, info) => {
      const errors = watch(page);
      await page.emulateMedia({ colorScheme: theme, reducedMotion: 'reduce' });
      await page.goto(`/${app}/`);
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await expect(page.locator('main')).toBeVisible();
      await page.waitForLoadState('networkidle');
      await page.evaluate(() => document.fonts.ready);

      const axe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
      const serious = axe.violations.filter((v) => ['serious', 'critical'].includes(v.impact ?? ''));
      expect(serious.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(', ')}`)).toEqual([]);

      mkdirSync(SHOTS, { recursive: true });
      for (const [w, h] of [[1440, 900], [390, 844]]) {
        await page.setViewportSize({ width: w, height: h });
        await page.screenshot({ path: `${SHOTS}/${app}-${theme}-${w}-${browserName}.png`, fullPage: true });
      }
      expect(errors).toEqual([]);
    });
  }
}

test('newtab: typing and Enter goes through the bridge', async ({ page }) => {
  const errors = watch(page);
  await page.goto('/newtab/');
  const q = page.getByRole('textbox', { name: 'Search or type a URL' });
  await q.fill('example.com');
  await q.press('Enter');
  await expect(page.getByRole('list').first()).toBeVisible();
  await page.keyboard.press('Escape');
  await page.locator('body').click({ position: { x: 5, y: 5 } });
  await page.keyboard.press('/');
  await expect(q).toBeFocused();
  expect(errors).toEqual([]);
});

test('settings: switching theme applies at once and accent radios are exclusive', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('/settings/');
  await page.getByRole('radio', { name: 'Dark' }).first().click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('radio', { name: 'Mint' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-accent', 'mint');
  await expect(page.getByRole('radio', { name: 'Ember' })).toHaveAttribute('aria-checked', 'false');
  // custom search URL appears only for the custom engine
  await expect(page.getByLabel('Custom search URL')).toHaveCount(0);
  await page.getByRole('radio', { name: 'Custom' }).last().click();
  await expect(page.getByLabel('Custom search URL')).toBeVisible();
});

test('library: search filters history, delete removes, bookmarks open folders', async ({ page }) => {
  await page.goto('/library/');
  await page.getByRole('searchbox').fill('wiki');
  await expect(page.locator('.list li')).toHaveCount(1);
  await page.getByRole('button', { name: /Remove .* from history/ }).click();
  await expect(page.getByText('Nothing matches “wiki”.')).toBeVisible();
  await page.getByRole('tab', { name: 'Bookmarks' }).click();
  await page.getByRole('button', { name: 'News', exact: true }).click();
  await expect(page.getByText('Index', { exact: true })).toBeVisible();
});

test('shields: per-site switch toggles', async ({ page }) => {
  await page.goto('/shields/');
  const sw = page.getByRole('switch', { name: 'Shield on github.com' });
  await expect(sw).toHaveAttribute('aria-checked', 'false');
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'true');
});

#!/usr/bin/env node
// Screenshots a SCYDE prototype in every theme/width combination using the pre-installed Playwright Chromium.
// Usage: node .claude/skills/prototype/shot.mjs <file.html> [--widths 1440,390] [--themes light,dark]
//        [--accent sky] [--out <dir>] [--scale 2] [--height 0 (full page)] [--browser chromium|firefox|webkit]
import { createRequire } from 'node:module';
import { mkdirSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const pw = require('playwright');

const args = process.argv.slice(2);
const file = args.find((a) => !a.startsWith('--') && !args[args.indexOf(a) - 1]?.startsWith('--'));
if (!file) {
  console.error('usage: shot.mjs <file.html> [--widths 1440,390] [--themes light,dark] [--accent sky] [--out dir]');
  process.exit(2);
}
const opt = (n, d) => {
  const i = args.indexOf(`--${n}`);
  return i >= 0 ? args[i + 1] : d;
};
const widths = opt('widths', '1440,390').split(',').map(Number);
const themes = opt('themes', 'light,dark').split(',');
const accent = opt('accent', '');
const scale = Number(opt('scale', '2'));
const height = Number(opt('height', '0'));
const browserName = opt('browser', 'chromium');
const abs = path.resolve(file);
const out = path.resolve(opt('out', path.join(path.dirname(abs), 'shots')));
mkdirSync(out, { recursive: true });

const browser = await pw[browserName].launch();
const written = [];
try {
  for (const theme of themes) {
    for (const width of widths) {
      const isPhone = width < 600;
      const ctx = await browser.newContext({
        viewport: { width, height: isPhone ? 844 : 900 },
        deviceScaleFactor: scale,
        colorScheme: theme === 'dark' ? 'dark' : 'light',
        hasTouch: isPhone,
        isMobile: isPhone && browserName !== 'firefox',
      });
      const page = await ctx.newPage();
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      const url = new URL(pathToFileURL(abs));
      url.searchParams.set('clean', '');
      await page.goto(url.href);
      await page.evaluate(
        ({ theme, accent }) => {
          document.documentElement.dataset.theme = theme;
          if (accent) document.documentElement.dataset.accent = accent;
          document.documentElement.dataset.motion = 'reduced';
        },
        { theme, accent },
      );
      await page.waitForTimeout(400);
      const name = `${theme}-${width}${accent ? `-${accent}` : ''}.png`;
      const target = path.join(out, name);
      await page.screenshot({ path: target, fullPage: height === 0, ...(height ? { clip: { x: 0, y: 0, width, height } } : {}) });
      written.push(target);
      if (errors.length) console.warn(`page errors (${name}):\n  ${errors.join('\n  ')}`);
      await ctx.close();
    }
  }
} finally {
  await browser.close();
}
for (const w of written) console.log(path.relative(process.cwd(), w));

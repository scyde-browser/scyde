// Icon export pipeline (PLAN §11 Faza 0.3, DESIGN.md §7).
//   node export.mjs [--concept <id|all|brand>] (default: brand.json icon) [--round <n>, default latest] [--out <dir>] [--check]
// Output (default brand/icon/export/<id>/, gitignored; CI uploads it as an artifact):
//   windows/scyde.ico                        16–256 px, PNG-compressed entries
//   android/res/mipmap-*/ic_launcher*.png    legacy + adaptive foreground + monochrome
//   android/res/mipmap-anydpi-v26/ic_launcher(.xml|_round.xml)
//   android/res/values/ic_launcher_background.xml
//   ios/AppIcon.appiconset/                  1024 light, dark and tinted + Contents.json
//   preview.png                              contact sheet for review

import { Resvg } from '@resvg/resvg-js';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildConcept, conceptIds, roundOf, latestRound, SIZE } from './concepts.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const brand = JSON.parse(readFileSync(path.join(root, 'brand/brand.json'), 'utf8'));

const args = process.argv.slice(2);
const arg = (name, def) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : def;
};
const check = args.includes('--check');
const which = arg('concept', 'brand') === 'brand' ? brand.icon?.concept ?? 'all' : arg('concept', 'all');
const round = Number(arg('round', latestRound));
const ids = which === 'all' ? conceptIds.filter((id) => roundOf(id) === round) : [which];
for (const id of ids) if (!conceptIds.includes(id)) throw new Error(`Unknown concept "${id}". Known: ${conceptIds.join(', ')}`);

export const render = (svg, px) => Buffer.from(new Resvg(svg, { fitTo: { mode: 'width', value: px } }).render().asPng());

/** Scales the drawing about the centre (Android safe zone). */
const scaled = (svg, k) =>
  svg.replace(/(<\/defs>)([\s\S]*)(<\/svg>)/, (_, a, body, z) => `${a}<g transform="translate(${(SIZE * (1 - k)) / 2} ${(SIZE * (1 - k)) / 2}) scale(${k})">${body}</g>${z}`);

/** Builds a .ico with PNG entries (supported since Windows Vista). */
export function ico(pngs) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(pngs.length, 4);
  const entries = [];
  let offset = 6 + 16 * pngs.length;
  for (const { size, data } of pngs) {
    const e = Buffer.alloc(16);
    e.writeUInt8(size >= 256 ? 0 : size, 0);
    e.writeUInt8(size >= 256 ? 0 : size, 1);
    e.writeUInt8(0, 2);
    e.writeUInt8(0, 3);
    e.writeUInt16LE(1, 4);
    e.writeUInt16LE(32, 6);
    e.writeUInt32LE(data.length, 8);
    e.writeUInt32LE(offset, 12);
    offset += data.length;
    entries.push(e);
  }
  return Buffer.concat([header, ...entries, ...pngs.map((p) => p.data)]);
}

const write = (file, data) => {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, data);
};

const ANDROID_DENSITIES = { mdpi: 1, hdpi: 1.5, xhdpi: 2, xxhdpi: 3, xxxhdpi: 4 };
const ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];

function exportConcept(id, outRoot) {
  const c = buildConcept(id);
  const out = path.join(outRoot, id);
  const files = [];
  const put = (rel, data) => {
    write(path.join(out, rel), data);
    files.push(rel);
  };

  // Windows: squircle tile, all sizes in one .ico
  const win = c.windows ?? c.light; // unplated glyph when the concept defines one
  put('windows/scyde.ico', ico(ICO_SIZES.map((size) => ({ size, data: render(win, size) }))));
  put('windows/scyde-256.png', render(win, 256));

  // Android: adaptive (foreground + background colour + monochrome) and legacy fallbacks.
  // Foreground is 108 dp with a 66 dp safe zone; marks are drawn within ~62 %, scaled to 0.92 for margin.
  const fg = scaled(c.mark, 0.92);
  const mono = scaled(c.mono, 0.92);
  for (const [dpi, k] of Object.entries(ANDROID_DENSITIES)) {
    put(`android/res/mipmap-${dpi}/ic_launcher_foreground.png`, render(fg, Math.round(108 * k)));
    put(`android/res/mipmap-${dpi}/ic_launcher_monochrome.png`, render(mono, Math.round(108 * k)));
    put(`android/res/mipmap-${dpi}/ic_launcher.png`, render(c.light, Math.round(48 * k)));
  }
  const adaptive = `<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome" />
</adaptive-icon>
`;
  put('android/res/mipmap-anydpi-v26/ic_launcher.xml', adaptive);
  put('android/res/mipmap-anydpi-v26/ic_launcher_round.xml', adaptive);
  put(
    'android/res/values/ic_launcher_background.xml',
    `<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <color name="ic_launcher_background">${c.androidBackground ?? brand.colors.iconTileLight}</color>\n</resources>\n`,
  );
  put('android/playstore-512.png', render(c.fullBleedLight, 512));

  // iOS: single-size 1024 app icon with dark and tinted appearances (iOS 18+). iOS applies its own mask.
  put('ios/AppIcon.appiconset/icon-1024.png', render(c.fullBleedLight, 1024));
  put('ios/AppIcon.appiconset/icon-1024-dark.png', render(c.fullBleedDark, 1024));
  put('ios/AppIcon.appiconset/icon-1024-tinted.png', render(c.tinted, 1024));
  put(
    'ios/AppIcon.appiconset/Contents.json',
    JSON.stringify(
      {
        images: [
          { filename: 'icon-1024.png', idiom: 'universal', platform: 'ios', size: '1024x1024' },
          { appearances: [{ appearance: 'luminosity', value: 'dark' }], filename: 'icon-1024-dark.png', idiom: 'universal', platform: 'ios', size: '1024x1024' },
          { appearances: [{ appearance: 'luminosity', value: 'tinted' }], filename: 'icon-1024-tinted.png', idiom: 'universal', platform: 'ios', size: '1024x1024' },
        ],
        info: { author: 'scyde-icon-export', version: 1 },
      },
      null,
      2,
    ) + '\n',
  );

  // Masters
  for (const v of ['mark', 'mono', 'light', 'dark']) put(`svg/${id}.${v}.svg`, c[v]);
  return { c, out, files };
}

function previewSheet(results, file) {
  const cell = 220;
  const cols = ['light', 'dark', 'mono'];
  const w = 40 + cols.length * (cell + 30) + 200;
  const h = 40 + results.length * (cell + 90);
  let body = '';
  results.forEach(({ c }, row) => {
    const y = 30 + row * (cell + 90);
    cols.forEach((v, col) => {
      const x = 30 + col * (cell + 30);
      const b64 = render(c[v], cell * 2).toString('base64');
      body += `<image x="${x}" y="${y}" width="${cell}" height="${cell}" href="data:image/png;base64,${b64}"/>`;
    });
    // small sizes as they appear in a taskbar / tab
    let x = 30 + cols.length * (cell + 30);
    for (const s of [64, 32, 16]) {
      const b64 = render(c.light, s * 2).toString('base64');
      body += `<image x="${x}" y="${y + 40}" width="${s}" height="${s}" href="data:image/png;base64,${b64}"/>`;
      x += s + 16;
    }
    body += `<text x="30" y="${y + cell + 34}" font-family="sans-serif" font-size="20" font-weight="600" fill="#111216">${c.name}</text>`;
    body += `<text x="30" y="${y + cell + 60}" font-family="sans-serif" font-size="14" fill="#545866">${c.blurb.replace(/&/g, '&amp;')}</text>`;
  });
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><rect width="${w}" height="${h}" fill="#E9EBEF"/>${body}</svg>`;
  write(file, Buffer.from(new Resvg(svg, { font: { loadSystemFonts: true }, fitTo: { mode: 'width', value: w * 2 } }).render().asPng()));
}

const outRoot = path.resolve(arg('out', path.join(root, 'brand/icon/export')));
const results = ids.map((id) => exportConcept(id, outRoot));
previewSheet(results, path.join(outRoot, 'preview.png'));

if (check) {
  for (const { out, files } of results) {
    for (const f of files) if (!existsSync(path.join(out, f))) throw new Error(`missing ${f}`);
    const icoBuf = readFileSync(path.join(out, 'windows/scyde.ico'));
    if (icoBuf.readUInt16LE(2) !== 1 || icoBuf.readUInt16LE(4) !== ICO_SIZES.length) throw new Error('bad ico header');
  }
  console.log(`icon-export check ok: ${results.length} concept(s)`);
}
for (const { c, files } of results) console.log(`${c.id}: ${files.length} files`);
console.log(`preview: ${path.join(outRoot, 'preview.png')}`);

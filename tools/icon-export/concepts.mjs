// Generates the three original icon concept masters (DESIGN.md §7, PLAN §16) as SVG.
// Each concept produces:
//   <id>.mark.svg   colour mark on a transparent 1024 canvas (Android foreground, compositing)
//   <id>.mono.svg   single-colour mark (Android themed icon, iOS tinted)
//   <id>.light.svg  mark on the white squircle tile (preview / Windows)
//   <id>.dark.svg   mark on the #17181E squircle tile (iOS 18+ dark variant)
// SVG has no conic gradient, so the soft Ember conic is drawn as overlapping wedges and masked.
// Never two equal overlapping circles.

import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const outDir = path.join(root, 'brand/icon/concepts');
const brand = JSON.parse(readFileSync(path.join(root, 'brand/brand.json'), 'utf8'));

export const SIZE = 1024;
const C = SIZE / 2;
const [E300, E400, E500] = brand.colors.iconGradient;
const TILE_LIGHT = brand.colors.iconTileLight;
const TILE_DARK = brand.colors.iconTileDark;
const MONO = '#111216';

// ---------- colour helpers ----------
const hex2rgb = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const rgb2hex = (c) => '#' + c.map((v) => Math.round(v).toString(16).padStart(2, '0')).join('');
const mix = (a, b, t) => rgb2hex(hex2rgb(a).map((v, i) => v + (hex2rgb(b)[i] - v) * t));
function sampleStops(stops, t) {
  const seg = (stops.length - 1) * t;
  const i = Math.min(Math.floor(seg), stops.length - 2);
  return mix(stops[i], stops[i + 1], seg - i);
}

/** Wedges emulating `conic-gradient(from <fromDeg>, stops…)` around (cx, cy). */
function conic(cx, cy, fromDeg, stops, steps = 180) {
  const R = SIZE * 1.5;
  const out = [`<rect x="${-SIZE}" y="${-SIZE}" width="${SIZE * 3}" height="${SIZE * 3}" fill="${sampleStops(stops, 0.5)}"/>`];
  for (let i = 0; i < steps; i++) {
    // CSS conic: 0deg = up, clockwise. SVG angle: 0 = right, clockwise (y down).
    const a0 = ((fromDeg + (i / steps) * 360 - 90 - 0.6) * Math.PI) / 180;
    const a1 = ((fromDeg + ((i + 1) / steps) * 360 - 90 + 0.6) * Math.PI) / 180;
    const p = (a) => `${(cx + R * Math.cos(a)).toFixed(1)} ${(cy + R * Math.sin(a)).toFixed(1)}`;
    out.push(`<path d="M${cx} ${cy}L${p(a0)}L${p(a1)}Z" fill="${sampleStops(stops, (i + 0.5) / steps)}"/>`);
  }
  return out.join('');
}

/** Superellipse squircle path (n = 5) filling the canvas with a small inset. */
export function squircle(inset = 0) {
  const n = 5;
  const r = C - inset;
  const pts = [];
  for (let i = 0; i < 360; i++) {
    const t = (i / 360) * 2 * Math.PI;
    const ct = Math.cos(t);
    const st = Math.sin(t);
    const x = C + r * Math.sign(ct) * Math.abs(ct) ** (2 / n);
    const y = C + r * Math.sign(st) * Math.abs(st) ** (2 / n);
    pts.push(`${x.toFixed(1)} ${y.toFixed(1)}`);
  }
  return `M${pts.join('L')}Z`;
}

const polar = (cx, cy, r, deg) => {
  const a = (deg * Math.PI) / 180;
  return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
};
/** SVG arc path from a0 to a1 (degrees, SVG orientation), sweeping clockwise if cw. */
function arc(cx, cy, r, a0, a1, cw) {
  const [x0, y0] = polar(cx, cy, r, a0);
  const [x1, y1] = polar(cx, cy, r, a1);
  const sweepDeg = cw ? (a1 - a0 + 360) % 360 : (a0 - a1 + 360) % 360;
  const large = sweepDeg > 180 ? 1 : 0;
  return `M${x0.toFixed(1)} ${y0.toFixed(1)}A${r} ${r} 0 ${large} ${cw ? 1 : 0} ${x1.toFixed(1)} ${y1.toFixed(1)}`;
}

const GRAD = [E300, E400, E500, E300];

// ---------- concepts ----------
// Each returns { colour: svg fragment, mono: svg fragment } drawn on a 1024 canvas, mark within ~640 px.

const concepts = {
  orbit: {
    name: 'Orbit',
    blurb: 'Jedan prsten s otvorom kroz koji prolazi manja kugla. Kretanje i zaštita: kugla je "propuštena" kroz štit.',
    draw() {
      const r = 250;
      const sw = 92;
      const gapFrom = -78;
      const gapTo = -12;
      const ring = arc(C, C, r, gapTo, gapFrom + 360, true);
      const [bx, by] = polar(C, C, r, -45);
      const br = 70;
      const mask = `<mask id="m-orbit" maskUnits="userSpaceOnUse" x="0" y="0" width="${SIZE}" height="${SIZE}"><path d="${ring}" fill="none" stroke="#fff" stroke-width="${sw}" stroke-linecap="round"/></mask>`;
      const ball = `<radialGradient id="g-ball" cx="0.35" cy="0.32" r="0.75"><stop offset="0" stop-color="${E300}"/><stop offset="1" stop-color="${E500}"/></radialGradient>`;
      return {
        defs: mask + ball,
        colour: `<g mask="url(#m-orbit)">${conic(C, C, 210, GRAD)}</g><circle cx="${bx.toFixed(1)}" cy="${by.toFixed(1)}" r="${br}" fill="url(#g-ball)"/>`,
        mono: `<path d="${ring}" fill="none" stroke="${MONO}" stroke-width="${sw}" stroke-linecap="round"/><circle cx="${bx.toFixed(1)}" cy="${by.toFixed(1)}" r="${br}" fill="${MONO}"/>`,
      };
    },
  },
  'twin-arc': {
    name: 'Twin Arc',
    blurb: 'Apstraktno "S" od dva luka različitih radijusa (gornji manji, donji veći). Čita se kao slovo, a ne kao krugovi.',
    draw() {
      const sw = 96;
      const r1 = 112;
      const r2 = 150;
      const total = 2 * r1 + 2 * r2 + sw;
      const cy1 = C - total / 2 + sw / 2 + r1;
      const cy2 = cy1 + r1 + r2;
      const cx = C + 6;
      // top: from upper-right (-28°) counter-clockwise over the top to the bottom point (90°)
      const top = arc(cx, cy1, r1, -28, 90, false);
      // bottom: from its top point (-90°) clockwise round the right to lower-left (152°)
      const bottom = arc(cx, cy2, r2, -90, 152, true).replace(/^M[^A]+/, '');
      const d = `${top}${bottom}`;
      const mask = `<mask id="m-twin" maskUnits="userSpaceOnUse" x="0" y="0" width="${SIZE}" height="${SIZE}"><path d="${d}" fill="none" stroke="#fff" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round"/></mask>`;
      return {
        defs: mask,
        colour: `<g mask="url(#m-twin)">${conic(C, C, 210, GRAD)}</g>`,
        mono: `<path d="${d}" fill="none" stroke="${MONO}" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round"/>`,
      };
    },
  },
  'aura-drop': {
    name: 'Aura Drop',
    blurb: 'Jedan krug s pomaknutom unutarnjom svjetlinom. Mekani "sjaj" Aure; na 16 px postaje ekscentrični prsten.',
    draw() {
      const R = 290;
      const [lx, ly] = [C - 70, C - 78];
      const lr = 138;
      const defs =
        `<clipPath id="c-drop"><circle cx="${C}" cy="${C}" r="${R}"/></clipPath>` +
        `<radialGradient id="g-light" cx="${lx}" cy="${ly}" r="${lr * 1.35}" gradientUnits="userSpaceOnUse">` +
        `<stop offset="0" stop-color="#FFF4EC"/><stop offset="0.8" stop-color="#FFF4EC"/>` +
        `<stop offset="0.82" stop-color="#FFE6D6" stop-opacity="0.5"/><stop offset="0.9" stop-color="#FFD9C4" stop-opacity="0"/></radialGradient>` +
        `<mask id="m-drop-mono" maskUnits="userSpaceOnUse" x="0" y="0" width="${SIZE}" height="${SIZE}"><circle cx="${C}" cy="${C}" r="${R}" fill="#fff"/><circle cx="${lx}" cy="${ly}" r="${lr}" fill="#000"/></mask>`;
      return {
        defs,
        colour: `<g clip-path="url(#c-drop)">${conic(C, C, 210, GRAD)}<circle cx="${lx}" cy="${ly}" r="${lr * 1.35}" fill="url(#g-light)"/></g>`,
        mono: `<circle cx="${C}" cy="${C}" r="${R}" fill="${MONO}" mask="url(#m-drop-mono)"/>`,
      };
    },
  },
};

const svg = (defs, body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}"><defs>${defs}</defs>${body}</svg>\n`;

export function buildConcept(id) {
  const c = concepts[id];
  const { defs, colour, mono } = c.draw();
  const tile = (fill) => `<path d="${squircle(0)}" fill="${fill}"/>`;
  return {
    id,
    name: c.name,
    blurb: c.blurb,
    mark: svg(defs, colour),
    mono: svg(defs, mono),
    light: svg(defs, tile(TILE_LIGHT) + colour),
    dark: svg(defs, tile(TILE_DARK) + colour),
    // variants for platform exports
    fullBleedLight: svg(defs, `<rect width="${SIZE}" height="${SIZE}" fill="${TILE_LIGHT}"/>` + colour),
    fullBleedDark: svg(defs, `<rect width="${SIZE}" height="${SIZE}" fill="${TILE_DARK}"/>` + colour),
    tinted: svg(defs, `<rect width="${SIZE}" height="${SIZE}" fill="#000"/>` + mono.replaceAll(MONO, '#FFFFFF')),
  };
}

export const conceptIds = Object.keys(concepts);

if (import.meta.url === `file://${process.argv[1]}`) {
  mkdirSync(outDir, { recursive: true });
  for (const id of conceptIds) {
    const c = buildConcept(id);
    for (const v of ['mark', 'mono', 'light', 'dark']) writeFileSync(path.join(outDir, `${id}.${v}.svg`), c[v]);
    console.log(`concept ${id}: ${c.name}`);
  }
}

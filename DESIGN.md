# DESIGN.md: "Island", the visual language of SCYDE Browser

> **v2.0 (2026-11-11).** Replaces v1 "Aura" (ADR-0008). This is the **single source of truth** for look and motion.
> Every value exists as a token in `design/tokens/*.json` (DTCG). Code never hard-codes values.
> Contrast is checked by CI (`pnpm tokens && pnpm contrast`, 204 pairs, all ≥ AA). Fonts (OFL) are in `design/fonts/`.

---

## 1. Essence

**Content edge to edge, the interface as a single object.** SCYDE has no tab strip and no toolbar at the top. The whole interface is an **island**: one floating object at the bottom of the window that carries the mark, navigation, address, Shield and tabs, and expands when you need it.

- **Paper and ink.** The page rests on warm paper (light) or near-black ink (dark). **The island is always dark:** in the light theme it is ink on paper, and in the dark theme it is a dark island with a thin light border and a shadow, with no light bar flashing in your eyes at night (owner, 2026-11-11).
- **Ember is energy.** The orange-red Ember appears only where there is action: the caret, the active tab, the primary button, the mark.
- **Words instead of tiles.** New Tab does not start with a grid of icons but with the question "Where to today?", which you type into directly.
- **Real numbers.** Shield statistics come from the engine and sit next to the island, in the mono font.
- **Quiet while you read.** When you scroll down, the island steps aside and the page fills the whole screen, edge to edge (on phones the status bar hides too). It comes back when you scroll up, reach the end of the page, swipe up from the bottom edge, or (desktop) when the cursor approaches it.

### How it differs from other browsers
| Usual | SCYDE Island |
|---|---|
| Tab strip + toolbar at the top | Nothing at the top except tiny Windows caption buttons |
| Full-width address bar | The address lives in the island; the domain is emphasized, the path is mono and fades out with a fade mask |
| Tabs as cards in a row | Tabs as stacked circles (stack) in the island; a click fans them out into a shelf above the island |
| New Tab = logo + field + tile grid | New Tab = one question you type into + a "recent" list with times |
| Ad blocker as an icon and a number | Shield is an indicator light in the island + a mono sentence on New Tab |

---

## 2. Color

### 2.1 Paper and ink (surfaces and text)

| Token | Light | Dark | Usage |
|---|---|---|---|
| `surface.canvas` | `#F3F1EC` paper | `#0D0D0F` ink | background of New Tab and internal pages |
| `surface.sunken` | `#EEEBE5` | `#1A1A1D` | fields and rows inside panels |
| `surface.raised` | `#FFFFFF` | `#1F1F23` | panels that come out of the island |
| `border.hairline` | ink 10 % | paper 10 % | 1 px lines |
| `text.primary` | `#121214` | `#F1EEE7` | headings and text |
| `text.secondary` | `#5B5A57` | `#A6A39C` | descriptions, meta |
| `text.tertiary` | `#66645F` | `#8E8B84` | hints, times |

### 2.2 Island (always dark)

| Token | Light | Dark |
|---|---|---|
| `island.bg` | `#121214` | `#232328` |
| `island.raised` | `#2A2A2E` | `#313137` |
| `island.text` | `#F3F1EC` | `#F1EEE7` |
| `island.text-secondary` | `#A8A6A0` | `#A6A39C` |
| `island.shield` | `#3DDC97` | `#3DDC97` |
| `island.border` (edge) | – | paper 22 % (1 px) |
| `elev.island` | `0 18 50 -18 rgba(0,0,0,.45)` | `0 18 50 -12 rgba(0,0,0,.70)` |

The island has `box-shadow: var(--elev-island), inset 0 0 0 1px var(--island-border)`.

### 2.3 Accent (user's choice, default **Ember**)
The accent marks "you are here": the caret, the active tab, the focus ring and the selected row. Each accent has a role per surface:

| Role | On paper (light) | On paper (dark) | On the island |
|---|---|---|---|
| `accent.ink` (text, icons) | dark ink variant | pastel | – |
| `accent.fill` + `accent.on-fill` | pastel + `#121214` | pastel + `#121214` | – |
| `accent.island` | – | – | pastel on the dark island in both themes (≥ 3:1) |

Presets: **Ember** (default), Sky, Periwinkle, Lavender, Mint, Onyx. For a custom color, `core/theme` derives all roles in OKLCH and rejects combinations below AA.

### 2.4 Brand
- **Ember:** `ember.300 #FFB27A` → `ember.800 #B8322C`. The CTA is `ember.700` with white text (5.08:1), and hover is `ember.800`.
- **The SCYDE S mark** (the owner's logo, `brand/icon/scyde.svg`): an S made of two mirrored halves with a diagonal cut (a lightning bolt). The colors are `brand.mark` = `ember.500 #F2603F` and `brand.ink #0A0A0A`, so Ember remains the only energy color in the entire brand. The mark is **just the letter S in Ember, with no plate and no background**, both in the interface (24 px in the island) and in icons.
- **No Aura mesh backgrounds.** The paper is clean; color comes only from favicons, the accent and Ember.

### 2.5 Semantic
`status.shield` (light `#107640`, dark `#3DDC97`), `status.warning`, `status.danger`, `status.info`: as in the tokens, all ≥ AA on all surfaces.

---

## 3. Typography
- **Bricolage Grotesque** (OFL, variable, opsz 12–96): the voice of the brand. Used for the hero, UI and page titles. At large sizes it has character (tight rhythm, dark cuts), and at small sizes it is legible.
- **Geist Mono** (OFL): data. Shield numbers, times ("2 min"), paths in the address, key hints.
- **Numerals:** always `tabular-nums`.
- **Weights:** 400 and 500, plus 600 for the hero only. Never ALL CAPS, not even in mono hints.
- Mobile native UI (SwiftUI, Compose) uses the same fonts bundled in the app; Dynamic Type and font scale up to 200 % are respected.

| Token | Size / line | Tracking | Usage |
|---|---|---|---|
| `type.hero` | 104 / 102 | −0.045em | "Where to today?" (phone 54 / 56) |
| `type.display` | 56 / 60 | −0.02em | onboarding |
| `type.headline` | 32 / 38 | −0.015em | internal page titles |
| `type.title` | 20 / 26 | −0.01em | panel titles |
| `type.address` | 16 / 22 | −0.01em | address in the island |
| `type.body-lg` | 16 / 24 | 0 | "recent" rows (19 px on desktop) |
| `type.body` | 14 / 20 | 0 | text |
| `type.label` | 13 / 18 | +0.005em | buttons, menus |
| `type.data` | 13 / 20 | 0 | mono data |
| `type.caption` | 12 / 16 | +0.01em | meta |

---

## 4. Space, shape, sizes
- **Grid:** 4 pt. Spacing `2…80` as in the tokens.
- **Radii:** `xs 6` · `sm 10` · `md 14` (rows, fields) · `lg 22` (panels) · `xl 28` · **`island 30`** · `full`.
- **Island (desktop):** height 60, width 720 (resting) and 760 (expanded), 22 px from the bottom, centered. Buttons 44, tab circles 30 with a 9 px overlap, address 420.
- **Island (phone):** height 58, 12 px from the edges, above the safe area.
- **Caption buttons (Windows):** 34 × 24, floating top right without a bar, with hover. Snap Layouts: ADR-0007.
- **New Tab (desktop):** padding 72/96, "recent" column 360, rows 50.
- **Hit targets:** 32 px (mouse), 44 pt (iOS), 48 dp (Android).

---

## 5. Depth and materials
- Everything that comes out of the island (suggestions, tab shelves, the Shield panel) is `surface.raised` + `elev.2` and grows **from the island upward**.
- The island has one shadow (`elev.island`); glass is never stacked.
- **Windows:** no Mica behind the content (the page goes to the edge). The island is a separate transparent top-level window, as are the panels (CLAUDE.md, ADR-0007).
- **iOS:** the island is native SwiftUI; with Reduce Transparency it stays solid. **Android:** solid island, no blur.

---

## 6. Motion

### 6.1 Principles
1. The island is an object: it changes shape (morph), it does not appear out of nowhere.
2. Fast and interruptible. Frequent actions (Ctrl+Tab, typing, scrolling) are instant.
3. Only `transform` and `opacity` are animated. For the island, width is animated as `scaleX` on the wrapper, with counter-scaling of the content.
4. Reduced motion: everything becomes a 120 ms cross-fade.

### 6.2 Tokens
Durations (`instant 90`, `fast 140`, `base 200`, `slow 300`, `choreo-max 420`) and springs (`snappy`, `smooth`, `bouncy`) are unchanged from v1. Settling times: 420, 663 and 805 ms.

### 6.3 Named effects
| Effect | Description |
|---|---|
| **Press** | scale 0.97, 90 ms, snappy return |
| **Rise** | entrance: opacity 0→1, y 8→0, `ease.out`, 200 ms |
| **Swell** | the island expands (720→760) and a panel grows upward out of it: smooth spring |
| **Hide** | scroll down (or swipe down on the island): the island slides away and the page goes full screen, 200 ms; it comes back on scroll up, at the end of the page, with a tap or swipe up at the bottom edge, or when the cursor approaches (owner, 2026-11-12; replaces the pebble) |
| **Fan** | the tab circles fan out into a shelf of cards above the island, stagger 24 ms, at most 8 |
| **Caret** | the Ember caret in "Where to today?" blinks at 1.1 s; with reduced motion it is static |
| **Morph** | Shield counter: digit roll; bouncy only when jumping from 0 |
| **Dissolve** | theme change: View Transition cross-fade 200 ms |
| **Sleep** | sleeping tab: circle at 60 % + moon |

### 6.4 Choreographies
- **New Tab entrance** (≤ 420 ms): Shield row Rise (0) → hero Rise (28 ms) → "recent" rows Rise, stagger 24 ms. The field is interactive immediately.
- **Address focus:** island Swell, suggestions grow above it (Bloom from the island), 140 ms.
- **Tabs:** a click on the stack triggers Fan (shelves above the island); Esc or a click outside brings the circles back.

---

## 7. Iconography
- Line icons, 1.5 px stroke, 20/24 grid, Lucide (ISC) + custom.
- A favicon without an image gets a **monogram in Aura pastel** per host (stable).
- App icon: **SCYDE S, just the letter in Ember, no background** (owner, 2026-11-11). Windows `.ico`: transparent, the mark covering ~92 % of the area. Android: transparent adaptive background + Ember S in the safe zone; monochrome for themed icons. iOS: dark and tinted are transparent; **light must be opaque (Apple), so it has a white background**. Exports: `node tools/icon-export/export.mjs` (`brand.json → icon.concept = owner-s`).

---

## 8. Components (required states: default, hover, press, focus-visible, disabled, loading)

| Component | Key details |
|---|---|
| **Island** | Left to right: mark (menu) · back · \| · address · Shield pebble (dot + number) · \| · tab stack (+N). Desktop 720/60, phone full width − 24. |
| **Address in the island** | Resting: domain (eTLD+1) in `island.text`, path in mono in `island.text-secondary` with a fade mask. Focus: full URL selected, island Swell, suggestions above. Never hides the domain (PLAN §7). |
| **Suggestions** | Panel above the island, rows 48: favicon, title, domain in accent ink, source on the right (history, bookmark, search engine) and `Enter` on the selected one. |
| **Tab stack and shelf** | Circles 30 with overlap; the active one has an accent ring. Shelf (Fan): cards 168 × 112 with a snapshot and title, `+` at the end, keys 1–9. |
| **Shield pebble and panel** | Dot `island.shield` + number (Morph). Panel above the island: large toggle, two numbers, level, Zen, details. |
| **New Tab** | Shield mono row · hero question with Ember caret · key hints · "recent" column (favicon, host, time in mono). News is opt-in and shown as a second column of the list, not as cards. |
| **Caption buttons** | Windows: three tiny buttons top right, no bar; hover `surface.sunken`, close `status.danger`. |
| **Menu** | Panel from the mark in the island, rows 32, shortcuts on the right in mono. |
| **Toast** | Comes out of the island upward, one action, 4 s. |
| **Phone: island** | The same island at the bottom; swiping left and right on the island switches tabs, swiping up opens the tab grid (a full-screen shelf). |

---

## 9. Themes and customization
Root attributes: `data-theme="light|dark"` · `data-accent="ember|sky|periwinkle|lavender|mint|onyx|custom"` · `data-density` · `data-motion`.
Key variables: `--island-*`, `--accent-island`, `--accent-ink`, `--accent-fill`, `--accent-ring`, `--surface-*`, `--text-*`.
- **Adaptive tint (optional):** the island takes 8–12 % of the page's `theme-color`, with a contrast check; if the check fails, the tint is not applied.
- **New Tab background:** paper (default), solid color or a custom image with 0–40 % dimming. The hero text keeps AA contrast.

---

## 10. Accessibility
- The focus ring is `2px solid var(--accent-ring)` on paper, or `var(--accent-island)` on the island, with a 2 px offset.
- The island is a landmark (`<nav aria-label>`). The tab stack is a `tablist`, suggestions a `listbox`, panels a `dialog`.
- The Shield counter is `aria-live="polite"` with rate limiting.
- Dynamic Type, 200 %, Reduce Motion, Reduce Transparency, High Contrast and Increase Contrast (stronger hairline) are respected.

---

## 11. Do / Don't
**Do:** content to the edge · one object for the whole interface · paper and ink · Ember only on action · words instead of tiles · mono for data · real numbers · quiet while reading.

**Don't:** a tab strip at the top · grey "Chrome" interfaces · gradient backgrounds · glass on glass · heavy shadows (except the one on the island) · emoji · "AI sparkles" · 3D blobs · ALL CAPS · grids of identical tiles · more than two weights per view.

---

## 12. Tokens in code and changelog
Outputs: `web/packages/tokens/dist/{css,kotlin,swift,json}`. Generator: `pnpm tokens`. Gate: `pnpm contrast`.

### Changelog
- **v2.2 (2026-11-13):** on Windows a **tab strip** runs across the top (ADR-0015, owner): every tab with its icon, title and ×, then + and the caption buttons. The window has no native title bar. This replaces "no tab strip at the top" (§1, §11) on Windows; the island keeps navigation, address and Shield.
- **v2.1 (2026-11-12):** the owner asked for a full-screen page while reading: **Hide** replaces **Pebble** (the island no longer stays as a small pill, and on phones the status bar hides with it). On phones the page runs under the status bar with the page's `theme-color` behind it, so there is no band around the page.
- **v2.0 (2026-11-11):** the owner rejected v1 "Aura" ("it must not look like every other browser") and, from three directions (Island, Spine, Keys; `design/prototypes/redesign/`), chose **Island** (ADR-0008).
  - New paper-and-ink palette, the island as an inverse surface, accent role `island`, default accent Ember.
  - Fonts Bricolage Grotesque and Geist Mono (OFL) instead of Geist/Aeonik.
  - New island sizes, `radius.island`, `type.hero`, `type.data`, `type.address`.
  - New effects Swell, Pebble, Fan and Caret.
  - Aura mesh backgrounds have been removed.
  - The contrast gate has been extended to pairs on the island (204 pairs).
  - In the dark theme the island stays dark with a border (instead of a light island), at the owner's request.
  - Icon and mark: the owner's **SCYDE S** replaces the Aura S; at the owner's wish it is in **Ember** (lime at first, then switched), tokens `brand.mark` and `brand.ink`.
- **v1.0 (Phase 0.2):** "Aura" (in git history).

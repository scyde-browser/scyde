# ADR-0005: UI library for the desktop chrome and internal pages

- **Status:** Accepted
- **Date:** 2026-11-11
- **Decided by:** Claude Code (Phase 0.2)
- **Related:** PLAN §8.1 (JS budgets), §11 Phase 0.2, CLAUDE.md (architecture notes)

## Context
The desktop chrome (tab strip, toolbar, omnibox, popovers) and internal pages (`scyde://newtab`, `settings`, `shields`, `library`) are written in web technology and run in three engines: WebView2 (Chromium), GeckoView (Gecko) and WKWebView (WebKit).

Constraints:
- desktop chrome ≤ **80 KB gz** JS, New Tab ≤ **120 KB gz** (PLAN §8.1)
- New Tab must be displayed ≤ 120 ms after Ctrl+T on Windows
- the look is our own (DESIGN.md "Aura"); the generic look of UI kits is not acceptable (R1, R4)
- animations only on `transform`/`opacity`, via CSS, WAAPI and `linear()` springs (no GSAP)
- the `pick-ui-library` skill is not installed in this environment, so the decision was made manually against the same criteria (recorded in `docs/STATUS.md`)

## Options
| Option | Runtime (gz, approx.) | Pros | Cons |
|---|---|---|---|
| **Svelte 5 (runes) + our own components** | **14.2 KB** measured for a whole mini app (see below) | compiled, fine-grained reactivity without a virtual DOM, small runtime, scoped CSS, good TS | smaller ecosystem; runes are a newer API |
| Solid + our own components | ~7–9 KB | smallest runtime, fine-grained reactivity | JSX, smaller ecosystem than React; the team (Claude Code + owner) is already planning Svelte |
| Preact + our own components | ~4–5 KB + hooks | tiny, familiar API | VDOM diff on every keystroke in the omnibox; signals are an add-on |
| React 19 + UI kit (e.g. Radix/shadcn) | ~45–50 KB for React + kit alone | huge ecosystem, ready-made a11y primitives | uses ~60 % of the chrome budget before the first line of our code; the kit pulls toward a generic look |
| Lit (web components) | ~6 KB | standard custom elements | shadow DOM complicates tokens, focus and tests; more boilerplate |

## Decision
**Svelte 5 (runes) + TypeScript `strict` + our own components in `web/packages/ui`, no UI kit.** Styles exclusively through CSS variables from `@scyde/tokens`. A11y patterns (menu, listbox, dialog, tabs) are written following the WAI-ARIA APG and tested with axe and Playwright in all three engines.

## Consequences
- Every component has all the states from DESIGN §8 (default, hover, press, focus-visible, disabled, loading); we build these ourselves, without a kit.
- `pnpm size` (size-limit or equivalent) becomes a mandatory job in `web.yml` from Phase 2; budget: desktop-chrome 80 KB gz, newtab 120 KB gz.
- Animations: CSS transitions + WAAPI, springs as pre-generated `linear()` (`tools/springs`). No animation libraries.
- If Phase 2 shows that the Svelte runtime + our code exceeds 60 KB gz for the desktop chrome, we reopen this ADR (candidate: Solid).

## Measurements / evidence
Mini app (tab list with `$state`, `$derived`, keyed `{#each}`, `bind:value`, `{#if}`, event handlers), Svelte 5.57.3 + Vite 8.3.8, production build:

```
dist/assets/index-*.js  36.57 kB │ gzip: 14.35 kB
```

That is ~18 % of the desktop chrome budget (80 KB gz) and ~12 % of the New Tab budget (120 KB gz), leaving the rest for our components and logic.

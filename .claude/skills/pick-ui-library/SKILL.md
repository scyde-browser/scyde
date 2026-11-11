---
name: pick-ui-library
description: Decide or re-decide the UI library for SCYDE web surfaces (desktop chrome, internal scyde:// pages) against hard bundle budgets, by measuring real builds rather than quoting sizes. Produces or updates an ADR (ADR-0005). Trigger on "pick-ui-library", "which framework", "switch from Svelte", or when a bundle budget is at risk.
---

# pick-ui-library

Constraints come from PLAN §8.1 and CLAUDE.md: desktop chrome ≤ 80 KB gz JS, New Tab ≤ 120 KB gz, runs in WebView2 (Chromium), GeckoView (Gecko) and WKWebView (WebKit), no UI kit (custom Aura look), animations via CSS/WAAPI/`linear()` only, no GSAP. Current decision: ADR-0005 (Svelte 5 runes, 14.35 KB gz measured).

## Procedure

1. **State the trigger.** Why are we (re)deciding? Quote the budget numbers and the current `pnpm size` output if it exists.
2. **Candidates.** Always include the incumbent (Svelte 5) plus at most three of: Solid, Preact (+signals), Lit, vanilla + small helpers. React/Vue/Angular only with a written reason; UI kits are out (R1, R4).
3. **Measure, same app for every candidate.** Build the identical reference app in the scratchpad, never in the repo:
   - tab list (keyed list, add/remove/select), an input with derived filter, a conditional block, one popover with focus management
   - production build with Vite, then `gzip -9c <bundle> | wc -c`
   - record: gz bytes, cold render time of 50 tabs (Playwright `performance.now()` around mount, median of 5) in chromium, firefox and webkit
4. **Score** each candidate: bundle (40 %), runtime cost on typing/scroll (25 %), a11y/focus ergonomics (15 %), TS strictness and DX (10 %), maintenance and ecosystem (10 %).
5. **Decide** and write/update `docs/adr/0005-ui-library.md` with the template in `docs/adr/0000-template.md`: table of measured numbers, the exact versions used, and the re-open condition (e.g. "if chrome bundle > 60 KB gz").
6. **Wire the budget**: make sure `pnpm size` enforces 80/120 KB gz in `web.yml` once apps exist.

Never claim a size you did not measure in this session; write "not measured" instead.

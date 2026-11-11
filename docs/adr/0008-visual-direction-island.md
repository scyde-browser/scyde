# ADR-0008: Visual direction "Island" instead of "Aura"

- **Status:** Accepted
- **Date:** 2026-11-11
- **Decided by:** `[OWNER]` (choice), Claude Code (elaboration)
- **Related:** PLAN §1 (R1 unique, R4 premium), PLAN §10, DESIGN.md v2.0, ADR-0005, ADR-0007

## Context
The first visual direction ("Aura", DESIGN v1) kept the classic browser layout: tabs and toolbar at the top, an address bar, a New Tab with a logo, a field and a grid of tiles. The owner rejected it: "I want it to be unique and new, not to look like every other browser".

## Options
Three directions as exploratory prototypes (`design/prototypes/redesign/`, git `4922b7d`):
1. **A · Island:** no top bar; the entire interface is a single floating island at the bottom; New Tab is a question you type into.
2. **B · Spine:** editorial and typographic; tabs are vertical headings in a left spine; New Tab is a front page.
3. **C · Keys:** tabs are numbered keys; shortcuts are a QWERTY keyboard.

## Decision
**A · Island** (owner, 2026-11-11). DESIGN.md was rewritten as v2.0, and the tokens were reworked.

## Consequences
- **Windows (ADR-0007):** there is no chrome webview at the top. The island is a separate, pre-created transparent top-level window (the same mechanism as popovers), and the tab webviews take up the entire client area.
  - The caption buttons float at the top right, and Snap Layouts remains an open question (ADR-0007).
  - The island must follow window moves and resizes, which is extra work in Phase 3.
  - Pebble (collapsing on scroll) needs scroll events from the tab webviews via the bootstrap script, without access to page content.
- **Mobile:** the bottom bar from the plan becomes the island; gestures stay (swipe across the island switches tabs, swipe up opens the grid).
- **Fonts:** Bricolage Grotesque and Geist Mono (OFL) go into the apps. The Aeonik license question (PLAN §17, no. 4) is no longer relevant.
- **Tokens:** new paper-and-ink palette, `island.*`, accent role `island`, default accent Ember. The contrast gate covers 204 pairs.
- **Prototypes:** the v1 prototypes were replaced by the Island prototypes, and v1 remains in git history.
- **Risk:** the unusual layout may confuse users accustomed to Chrome. Mitigation: onboarding in Phase 6 and an "island always full" option (no Pebble).

## Measurements / evidence
Screenshots of all three directions: `design/prototypes/redesign/*/shots/` (git `4922b7d`); Island prototypes and critique: `design/prototypes/{newtab,desktop,mobile}/`.

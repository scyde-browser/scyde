---
name: impeccable
description: SCYDE design quality gate. Use to finalize or edit DESIGN.md, to critique any UI (prototype, component, page, icon) before a PR, and for audit/polish passes in Phase 6. Checks every finding against DESIGN.md "Aura" and the tokens, ranks findings P1–P3, then fixes them. Trigger on "impeccable", "critique", "design review", "audit", "polish", or any UI PR.
---

# impeccable: SCYDE design quality gate

DESIGN.md is the only source of truth. This skill turns it into a repeatable review. Never review "by taste" alone: every finding names the DESIGN.md section or token it breaks.

## Modes

| Mode | When | Output |
|---|---|---|
| `critique` | every UI PR, every prototype, every icon round | findings list + fixes applied + summary for the PR body |
| `audit` | Phase 6, end of a platform phase | findings across the whole UI, grouped by screen |
| `polish` | after critique/audit is clean | micro-detail pass (states, timing, optical alignment) |
| `finalize` | editing DESIGN.md itself | updated DESIGN.md + changelog entry in §12 |

## Procedure (critique / audit)

1. **Look, don't guess.** Render what you review. For web/prototypes use `node .claude/skills/prototype/shot.mjs <html>` (light + dark, 1440 and 390 px). For icons use `node tools/icon-export/export.mjs` and read `brand/icon/export/preview.png`. Read the screenshots before writing any finding.
2. **Run the machines first.**
   - `pnpm tokens && pnpm contrast`: must be 0 failures.
   - Grep the changed files for hard-coded values: `#[0-9a-fA-F]{3,8}`, `\d+px`, `\d+ms`, `cubic-bezier(` outside `design/tokens` and generated output. Each hit is a P1.
3. **Walk the checklist** below. For each item: pass, or a finding.
4. **Rank** each finding:
   - **P1** breaks a rule (contrast, tokens, a11y, R2 no-ads, hidden domain in omnibox, forbidden list in §11, "two overlapping circles").
   - **P2** visibly off the system (wrong hierarchy, spacing off-grid, missing state, motion out of budget).
   - **P3** polish (optical alignment, copy, tiny timing).
5. **Fix** all P1 and P2 in the same session. P3: fix or list as follow-up.
6. **Re-render and re-check.** A finding is closed only when the new screenshot shows it fixed.
7. **Report** in this exact shape (it goes into the PR body under "`impeccable` critique"):

```
### impeccable critique: <subject>
Rendered: <files/screens checked>
Machines: contrast <n>/<n> AA, hard-coded values <n>
| # | P | Finding | Rule | Fix | Status |
|---|---|---|---|---|---|
Verdict: ship | ship after P2 fixes | rework
```

## Checklist

**Hierarchy and layout (DESIGN §1, §4, §11)**
- One focus per screen. On New Tab that is search. Is anything competing with it?
- Spacing is on the 4 pt grid and uses `--space-*`. Generous space, never cramped.
- Not centred by default: centring must be a choice (hero only).
- Radii come from `--radius-*` and match the component table (tile 22, menu 14, tab 10, pill full).

**Colour (DESIGN §2)**
- Light: pastel is a fill with dark text; accent-coloured text uses `--accent-ink`. Dark: pastel directly.
- CTA is `--cta-fill` with `--cta-text`; hover `--cta-fill-hover`. Never `ember.600` behind normal text.
- At most one Aura mesh per screen; text over it keeps §2.1 contrast.
- Shield state uses `--status-shield` (on) / `--text-tertiary` (off).

**Type (DESIGN §3)**
- Max two weights per view (400/500; 600 only display). No ALL CAPS. Counters use `tabular-nums`.
- Desktop chrome default is `type.label` 13/18.

**Depth (DESIGN §5)**
- Light: `--elev-*` only, never heavier. Dark: depth from lighter surface + hairline, no drop shadows.
- No stacked glassmorphism.

**Motion (DESIGN §6)**
- Only `transform`/`opacity`. Durations/easings/springs from tokens. Interruptible. Reduced motion = 120 ms cross-fade.
- Frequent actions (Ctrl+Tab, typing, scroll) are instant.
- Named effects from §6.3 (Press, Lift, Rise, Bloom, Glide, Morph, Dissolve, Peek, Sleep). New effect = add it to §6.3 first.

**States and a11y (DESIGN §8, §10)**
- Every interactive component: default, hover, press, focus-visible, disabled, loading.
- Focus ring `2px solid var(--accent-ring)` + 2 px offset; never `outline: none` without replacement.
- Hit targets: 32 px desktop, 44 pt iOS, 48 dp Android. Labels on every icon button. i18n keys, no literal strings.

**Brand and icon (DESIGN §7, PLAN §16)**
- Original shape; never two equal overlapping circles; no copied reference logo/layout.
- Reads at 16 px (taskbar, tab) and in monochrome (Android themed, iOS tinted).
- Mark sits inside the Android adaptive safe zone (66/108). Soft conic Ember gradient, no seams or visible gradient apex.
- No 3D blob/gloss, no sparkle.

**Product rules**
- R2: no sponsored tiles, ads, affiliate links, rewards/wallet UI.
- Omnibox never hides the registrable domain.
- Numbers are real (engine data), or explicitly marked "≈" when estimated.

## finalize (DESIGN.md)
- Every number in DESIGN.md must be reproducible by a tool (`pnpm contrast`, `tools/springs`). Recompute, don't copy.
- Every value must exist as a token in `design/tokens/*.json`; add the token first.
- Add a dated entry to the §12 changelog describing what changed and why.

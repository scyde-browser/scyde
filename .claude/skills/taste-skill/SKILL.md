---
name: taste-skill
description: Anti-generic check for SCYDE. Use on prototypes, New Tab, onboarding, icons and any marketing page to make sure nothing looks template-made, "AI-made" or like a reskin of Chrome/Brave/Arc. Scores distinctiveness and names concrete changes. Trigger on "taste", "does this look generic", "make it unique", or before approving any visual direction.
---

# taste-skill: does this look like SCYDE, or like everything else?

R1 in PLAN.md says SCYDE must be unique and must not look like a reskin. This skill is the explicit check for that. It complements `impeccable` (rules) with judgement (character). Always render first and judge the pixels, not the code.

## Step 1: tells of generic / AI-made UI (each hit must be removed or justified)

- purple-to-blue hero gradient, neon glow, "aurora" blobs, 3D glossy spheres
- stacked glassmorphism, frosted cards on frosted cards
- sparkles, magic wands, "AI" badges
- emoji used as icons
- everything centred, symmetrical card grids with identical cards, no rhythm
- default Inter/system type with no tracking or size contrast; bold everywhere
- heavy drop shadows, borders AND shadows AND fills on the same element
- generic copy ("Supercharge your browsing", "Lightning fast", "Welcome back!")
- stock illustration people, isometric 3D scenes
- icons mixed from different families or stroke weights
- logo shaped like a known mark (two overlapping circles, a fox, a lion, a compass)
- looks like Chrome's New Tab, Brave's stats page or Arc's sidebar with colours swapped

## Step 2: score (1–5 each, write one sentence of evidence per line)

| Axis | 1 | 5 |
|---|---|---|
| Distinctiveness | could be any product | recognisably SCYDE with the logo covered |
| Restraint | everything shouts | one focus, calm, energy only where the action is (Ember) |
| Specificity | placeholder-ish | real content, real numbers, real states |
| Craft | rough edges | optical alignment, consistent rhythm, considered states |
| Coherence | parts disagree | light/dark/desktop/mobile feel like one product |

Pass bar: **every axis ≥ 4**, distinctiveness ≥ 4 is mandatory.

## Step 3: what makes it SCYDE (must be answerable)

Name at least two of these signature moves visible in the work, or propose where they go:
- **Aura atmosphere:** one low-saturation pastel mesh, pastel fills with ink text in light mode
- **Ember energy** only on the primary action and the brand mark
- **Quiet premium:** large spacing, soft sunken tiles, hairlines in dark, no heavy shadow
- **Real numbers:** live Shield counters with tabular figures and Morph roll
- **Original mark** with the soft conic Ember gradient on a white/onyx squircle

## Step 4: output

```
### taste check: <subject>
Generic tells found: <list or "none">
Scores: distinctiveness x/5, restraint x/5, specificity x/5, craft x/5, coherence x/5
Signature moves present: <list>
Changes to make: <ordered, concrete>
Verdict: pass | revise
```

Apply the changes, re-render and re-score. Record the final block in the PR body or `docs/STATUS.md`.

---
name: prototype
description: Build a fast, single-file HTML prototype of a SCYDE UI (New Tab, desktop chrome, omnibox, Shield panel, mobile toolbar, tab switcher) in design/prototypes/, wired to the real Aura tokens, with light/dark/accent/density/motion switches, then screenshot it in light and dark at desktop and phone widths. Use before implementing any new UI feature. Trigger on "prototype", "mock up", "try a layout".
---

# prototype

A prototype answers "is this the right layout and feel?" before real code. It is throwaway HTML, but it uses the **real tokens**, so what passes here carries straight into `web/`.

## Rules
- One folder per prototype: `design/prototypes/<name>/index.html` (+ optional `notes.md`). Single file, no build step, no framework, no external network calls (no CDN, no web fonts from Google). Images are inline SVG or local files.
- Styles **only** through tokens. Link the generated file with a relative path:
  `<link rel="stylesheet" href="../../../web/packages/tokens/dist/css/tokens.css">` (run `pnpm tokens` first). No hex, px or ms literals in the prototype CSS except `0`, `1px` hairlines and percentages.
- Put `data-theme`, `data-accent`, `data-density`, `data-motion` on `<html>`. Include a small, visually quiet switcher (bottom corner) so the owner can flip them; hide it with `?clean` for screenshots.
- Text comes from `web/packages/i18n/locales/{hr,en}.json` keys where they exist; add missing keys there first. Default language for prototypes: `hr`, with `?lang=en`.
- Realistic content: real-looking site names, real-shaped numbers ("1.284 blokirano"), long titles, empty states. No lorem ipsum.
- Motion uses token durations/easings/springs and the named effects from DESIGN §6.3. Respect `prefers-reduced-motion`.
- Icons: inline Lucide-style 1.5 px line icons (ISC) or our own; never emoji.
- Never copy a reference image's logo or exact layout (CLAUDE.md rule 7).

## Screenshots
```bash
pnpm tokens
node .claude/skills/prototype/shot.mjs design/prototypes/<name>/index.html
# options: --widths 1440,390  --themes light,dark  --accent sky  --out <dir>  --scale 2
```
Writes `design/prototypes/<name>/shots/<theme>-<width>.png` (commit them: they are the review artefact) and prints the paths. Read every screenshot before claiming the prototype is done.

## Then
1. Run `taste-skill` on the screenshots.
2. Run `impeccable` critique, fix P1/P2, re-shoot.
3. Record both result blocks in `design/prototypes/<name>/notes.md` and the PR body.
4. Mark the prototype in `docs/STATUS.md` (0.4) and ask `[VLASNIK]` for approval.

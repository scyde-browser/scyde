# design/prototypes: the "Island" direction (DESIGN.md v2.0)

Prototypes open in the browser (`file://`), on the real tokens (`pnpm tokens`) and i18n strings (`node design/prototypes/_shared/build-strings.mjs`).
Parameters: `?state=…`, `?theme=light|dark`, `?accent=ember|sky|…`, `?clean` (without switchers).

| Prototype | States | Screenshots |
|---|---|---|
| `desktop/` (1440 × 900) | `newtab`, `browse`, `omnibox`, `tabs`, `shield` | `desktop/shots/` |
| `mobile/` (390 × 844) | `newtab`, `browse`, `tabs`, `shield` | `mobile/shots/` |

Screenshots: `node .claude/skills/prototype/shot.mjs design/prototypes/desktop/index.html --widths 1440 --height 900 --query state=tabs --tag tabs`.
The v1 "Aura" prototypes and the exploration of three directions are in git history (`4922b7d`, ADR-0008).

## taste check: Island
Generic tells found: none. No tab strip, toolbar or tile grid; no gradient backgrounds or glass.
Scores: distinctiveness 5/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 5/5
Signature moves: the island as the only object of the interface, New Tab as a question you type into, paper and ink with an inverse island, mono for data, the owner's S mark in Ember.
Verdict: pass

## impeccable critique: Island (desktop + mobile)
Rendered: desktop {newtab, browse, omnibox, tabs, shield} × {light, dark}; mobile {newtab, browse, tabs, shield} × {light, dark}
Machines: contrast 204/204 AA; no hard-coded values in the prototypes' CSS (only 0, 1px, %); 390 px without horizontal overflow (verified)
| # | P | Finding | Rule | Fix | Status |
|---|---|---|---|---|---|
| 1 | P1 | 6 text pairs below AA on the new palette (sunken, tertiary, Shield on the light island) | §2, §10 | sunken `#EEEBE5`, tertiary `#66645F`, `island.shield` dark `#0B5E33` | ✅ |
| 2 | P2 | Captions under the Shield numbers have different heights | rhythm | fixed 2 lines | ✅ |
| 3 | P2 | The mark (gradient on ink) disappears on the light island in the dark theme | §2.4 | mark as an Ember plate with a black S, from tokens | ✅ |
| 4 | P3 | The Ember accent and the lime mark were two "energies" | §1 | owner: logo switched to Ember | ✅ |
| 5 | P3 | Desktop New Tab has a lot of empty space below the hint | §1 quiet | intentional; in Phase 6 consider news as a second column when enabled | – |
Verdict: ship

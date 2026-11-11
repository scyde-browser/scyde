# Prompts for Claude Code

Copy them into Claude Code in order. Each prompt is one session. Before each one, make sure the previous phase is closed (CI green, `docs/STATUS.md` up to date).

---

## 1. Kickoff: Phase 0 (foundations, design, spikes)

```
You are the lead engineer of the SCYDE Browser project. The repo contains PLAN.md, CLAUDE.md, DESIGN.md and design/references/.

1. Read PLAN.md, CLAUDE.md and DESIGN.md in full and look at all three images in design/references/.
2. Write a short work plan for Phase 0 (PLAN §11) as a checklist in docs/STATUS.md.
3. Carry out 0.1 (repo structure from PLAN §4, pinned toolchains, Renovate, skeleton CI workflows from PLAN §13, ADR template, docs/NETWORK.md) and write ADR-0001 (engine strategy from PLAN §2).
4. Carry out 0.2: using the `impeccable` skill, finalise DESIGN.md and generate design/tokens (DTCG) from it + a Style Dictionary build for CSS, Kotlin and Swift. Check that the contrasts from DESIGN.md pass (create tools/contrast-check). Run `taste-skill` on the direction and `pick-ui-library` (constraint: chrome UI ≤ 80 KB gz) → ADR-0005.
5. Carry out 0.3: three original icon concepts as SVG (not two overlapping circles, see PLAN §16 and DESIGN §7) + tools/icon-export. Stop and ask me which concept I choose.
6. Carry out 0.4: prototypes with the `prototype` skill in design/prototypes/ (New Tab light/dark, desktop chrome with omnibox suggestions and the Shield panel, mobile bottom toolbar + tab switcher). For each, run an `impeccable` critique and fix the findings.
7. Carry out 0.5: prepare spikes S1–S5 (PLAN §11, Phase 0) in spikes/ with CI workflows that build and run whatever can be automated. For anything that needs me (e.g. signing in to a Google account on a device), write the exact test steps in docs/STATUS.md.

Rules: work on the branch phase/0-foundations, small commits (Conventional Commits), open a PR when 0.1–0.2 is done and a second PR for 0.3–0.5. Mark decisions that need me with [OWNER]. Do not move on to Phase 1.
```

---

## 2. After gates G1–G5

```
Read docs/STATUS.md and the spike results. For each gate (G1–G5), write an ADR with the measurements and the decision
(ADR-0002 Google sign-in/engine, ADR-0003 Windows Shield option A/B/C, ADR-0004 GeckoView extension, ADR-0006 iOS rule lists, ADR-0007 Tauri multiwebview).
If a gate did not pass, propose an alternative from PLAN §2.3 with a cost estimate and stop until I decide.
If they all passed, delete spikes/ (keep the ADRs), close Phase 0 in STATUS.md and propose a plan for Phase 1.
```

---

## 3. Template for each subsequent phase

```
Start Phase <N> according to PLAN.md §11. Read CLAUDE.md and docs/STATUS.md.
- Break the phase down into tasks in docs/STATUS.md (checklist), each with the AC from the plan.
- Work on the branch phase/<N>-<name>; one PR per logical unit; CI must be green.
- For UI: first `prototype`, then implementation, then an `impeccable` critique. Animations go through `animate`/`choreograph`, and `animation-performance` checks each one.
- For Windows and iOS, use GitHub Actions (gh workflow run / gh run watch) and attach the screenshot artifacts to the PR.
- At the end of the phase, run `break-ui` with the scenarios from PLAN §14.3 and fix the P1/P2 findings.
- Update STATUS.md; deviations from the plan go into an ADR; mark decisions for me with [OWNER].
```

---

## 4. Polish: Phase 6

```
Phase 6. In order:
1) `find-animation-opportunities` across all platforms → list.
2) `improve-animations` → plan, then implementation with `choreograph` and `animation-performance`.
3) `review-animations` → critique and fixes.
4) `remove-animations` → remove everything that does not serve a purpose (and add a power-saver mode).
5) `impeccable` audit + polish across the whole UI; `taste-skill` on the New Tab and onboarding.
6) Use `awwwards-animations` only as guidelines for the New Tab hero / onboarding / reveal on theme change; implementation via CSS, WAAPI and linear() springs, without GSAP.
7) `break-ui` on all three platforms.
Then the unique features from PLAN §11 Phase 6, in the order I confirm.
```

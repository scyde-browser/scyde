# CLAUDE.md: SCYDE Browser

These are the working instructions for Claude Code in this repo. The complete plan is in **PLAN.md**, the visual system in **DESIGN.md**.
At the start of every session, read `docs/STATUS.md` and continue from the first open task of the current phase.

## What we are building
Our own browser (like Brave, but our own) for **Windows, Android and iOS**. Requirements:
- unique
- no ads, with a built-in ad, tracker and popup blocker
- Dark/Light and theme customization
- premium look
- highly optimized

| Platform | Engine and shell |
|---|---|
| Windows | Tauri 2 + WebView2 |
| Android | Kotlin/Compose + GeckoView |
| iOS | Swift + WKWebView, .ipa for Feather |

The shared Rust core is `core/`, the shared web UI is `web/`.

## Golden rules
1. **Phases and gates.** Work only on the current phase (PLAN §11). Do not skip gates G1–G5. When something depends on the owner, mark it `[OWNER]`, record it in `docs/STATUS.md` and continue with whatever does not depend on that decision.
2. **No ads and no telemetry.** Never add sponsored content, affiliate links, analytics or third-party SDKs that send data. Document every new network call made by the browser in `docs/NETWORK.md`.
3. **Performance is a requirement, not a wish.** Respect the budgets from PLAN §8. A PR that touches a hot path (startup, tab switching, Shield match, animations) comes with a measurement.
4. **Design only through tokens.** No hard-coded colors, spacing, radii or durations. Everything comes from `design/tokens` (CSS variables, Kotlin, Swift). A new value means a token first, and only then code.
5. **Motion:** animate only `transform` and `opacity` (web) and their equivalents on native platforms. Animations must be interruptible and respect reduced motion. Durations and springs come from DESIGN.md §6.
6. **Security:**
   - the native bridge accepts messages only from the `scyde://` origin (origin check + method allowlist)
   - strict CSP on internal pages
   - remote debugging only in debug builds
   - never disable TLS verification
   - the omnibox never hides the real domain
7. **Do not copy references.** The images in `design/references/` are a direction. Do not take logos, layouts or assets from other people's work. The icon must not be "two overlapping circles" (PLAN §16).
8. **Decisions go into an ADR.** Every deviation from PLAN.md gets a `docs/adr/NNNN-*.md`.
9. **Small PRs, green CI.** Conventional Commits. Branches `phase/N-*`, `feat/*`, `fix/*`. Every UI PR has screenshots (CI artifacts) and an `impeccable` critique in the description.
10. **English only, i18n-ready.** All UI copy is English (owner decision 2026-11-11, ADR-0009). No hard-coded strings: copy lives in `web/packages/i18n` (en), `android/.../strings.xml` and `ios/.../Localizable.xcstrings`, so more languages can be added later.
11. **Clean code of our own, not a fork.** SCYDE is not built the way Brave, Firefox or other browsers are: no fork or rebrand of Chromium, Brave, Firefox or their code. We write the shell, UI, Shield logic, omnibox, history, themes and all features ourselves. The render engine is the platform's (WebView2, WKWebView). Approved exceptions (owner, 2026-11-11): **GeckoView** as the Android engine and **adblock-rust** as the Shield matcher, both as libraries. Any new external library that carries another browser's code goes to the owner first `[OWNER]`.

## Environment and build
Claude Code usually runs on **Linux**. `core/`, `web/`, `shield-extension/` and `android/` work locally. **Windows and iOS are built and tested in GitHub Actions.** Iterate like this:
```bash
gh workflow run windows.yml --ref <branch>   # or ios.yml
gh run watch                                 # follow
gh run view --log-failed                     # if it fails
gh run download <run-id> -n screenshots      # visual check
```

### Commands (fill them in during Phase 0 and keep them up to date)
```bash
# core
cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace
cargo bench -p shield --bench match            # Shield latency
cargo run -p shield-compiler -- --lists qa/lists --out dist/filters

# web (pnpm workspace)
pnpm i && pnpm -r lint && pnpm -r typecheck && pnpm -r test
pnpm --filter @scyde/tokens build              # Style Dictionary → CSS/Kotlin/Swift
pnpm --filter newtab dev
pnpm e2e                                        # Playwright: chromium + firefox + webkit, light/dark
pnpm size                                       # bundle budgets

# shield-extension
pnpm --filter shield-extension build -- --target chromium-mv3
pnpm --filter shield-extension build -- --target gecko-mv2

# android
cd android && ./gradlew assembleDebug testDebugUnitTest
./gradlew :benchmark:connectedCheck            # Macrobenchmark (emulator: trend only)

# desktop (Windows / CI only)
cd desktop && pnpm tauri build

# ios (macOS / CI only)
cd ios && xcodegen && xcodebuild -scheme SCYDE -destination 'platform=iOS Simulator,name=<iPhone from xcrun simctl list>' test
```

## Skills: when to use them
The detailed map is in PLAN §12. In short:

| Situation | Skill |
|---|---|
| New UI feature | `prototype` → implementation → `impeccable` critique |
| Design system, DESIGN.md, audit, polish | `impeccable` |
| Checking that it does not look generic | `taste-skill` |
| Components and small details | `emil-design-eng` |
| iOS (and gesture and spring principles in general) | `apple-design` |
| Internal web pages on phones | `mobile-native` |
| New animation | `animate`; for multiple elements `choreograph` |
| Every animation (budget) | `animation-performance` |
| Naming effects | `animation-vocabulary` |
| Phase 6 | `find-animation-opportunities` → `improve-animations` → `review-animations` → `remove-animations` |
| End of a platform phase | `break-ui` |
| Choosing a UI library (Phase 0) | `pick-ui-library` |
| Hero, onboarding, reveal effects | `awwwards-animations`, **guidelines only**; no GSAP or its libraries |

If a skill is not installed, continue according to DESIGN.md and note it in STATUS.md.

## Architecture notes (most common pitfalls)
- **Tauri multiwebview** requires the `unstable` feature. All tab webviews must be on the **same WebView2 environment**. For APIs that Tauri does not expose, use `with_webview(|w| w.controller())`.
- **Popovers on desktop** (omnibox suggestions, menus, Shield panel) are separate transparent top-level windows, created in advance. Do not try to transparently overlay child webviews.
- **The WebView2 popup blocker is disabled** in WebView2. We do the blocking ourselves in `NewWindowRequested` based on `IsUserInitiated`.
- **Google sign-in:** do not change the UA on Windows (it must be identical to Edge). On iOS the UA must have the Safari suffix. Android uses GeckoView. After every engine change, run the G1 test.
- **iOS rule lists:** at most 150k rules per list. Compile once and cache, never on the main thread.
- **Android:** edge-to-edge and predictive back are mandatory. Limit the number of live `GeckoSession`s.
- **Svelte 5 runes**, TypeScript `strict`, no UI kit (ADR-0005). Components are in `web/packages/ui`.
- Fonts: OFL substitute until `[OWNER]` confirms the Aeonik license. The subset must cover č, ć, đ, š, ž.

## Definition of "done" for every task
- [ ] Code + tests; CI green
- [ ] Budgets not exceeded (measurement attached if it is a hot path)
- [ ] Light and dark checked (screenshots)
- [ ] a11y: focus, labels, AA contrast, hit targets (44 pt iOS / 48 dp Android)
- [ ] i18n keys (en)
- [ ] `docs/STATUS.md` updated; ADR if it is a decision

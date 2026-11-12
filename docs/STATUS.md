# STATUS.md

> At the start of every session, read this file and continue from the first open task of the current phase.

**Current phase:** Phases 2–5 done (PR #14), **[v0.2.0 published](https://github.com/scyde-browser/scyde/releases/tag/v0.2.0)** with the Windows `.exe`, the unsigned iOS `.ipa` and the Android APKs (2026-11-11). Phase 6 is waiting for the owner to confirm the feature order (PLAN §11); Phase 7 has started with release hardening (NOTICE, privacy policy, release rules). Phase 1 done (PRs #5–#12).
**Last updated:** 2026-11-11

## Phase 0: CLOSED (2026-11-11)

| Step | Result |
|---|---|
| 0.1 Repo | structure, pinned toolchains, Renovate, CI skeleton, ADR-0001 · PR #1 |
| 0.2 Design | DESIGN.md v1.0, DTCG tokens → CSS/Kotlin/Swift, contrast gate 168/168, ADR-0005 · PR #1 |
| 0.3 Icon | the owner's **SCYDE S** in Ember, reconstructed as a vector; `tools/icon-export` · PR #2 |
| 0.4 Prototypes | v1 "Aura" rejected → three new directions → the owner chose **Island** (ADR-0008, DESIGN.md v2.0); prototypes `design/prototypes/{desktop,mobile}` · PR #2 |
| 0.5 Spikes | all gates passed, spike code deleted (git `23451d6`) · PR #2 |

| Gate | Result | ADR |
|---|---|---|
| G1 Google sign-in | ✅ WebView2, GeckoView, WKWebView (owner) | 0002 |
| G2 Windows Shield | ✅ option A (DNR), B as fallback; 6/6 network; loading faster than baseline | 0003 |
| G3 GeckoView | ✅ WASM p99 40 µs (SpiderMonkey), fixture, native messaging | 0004 |
| G4 iOS rule lists | ✅ 263,790 rules / 2 lists, cached load 6 ms | 0006 |
| G5 Tauri multiwebview | ✅ switch p95 ≤ 0.39 ms, popover p95 0.10 ms | 0007 |

**Carried over to later phases:**
- Snap Layouts on Windows 11 (ADR-0007) → Phase 3
- lazy tab creation + pre-warmed tab for Ctrl+T (≈ 200 ms per webview) → Phase 3
- iOS list compilation only in the background, and shipping a precompiled snapshot → Phase 4
- GeckoView 160 requires compileSdk 37.2, AGP 9.5, Kotlin 2.4 → Phase 5

## Release v0.1.0: published (2026-11-11)
[SCYDE v0.1.0](https://github.com/scyde-browser/scyde/releases/tag/v0.1.0), a pre-release with 7 assets (icons, `.ico`, mark, tokens, fonts, prototypes, docs).
Tag pushes from the agent environment are still refused (HTTP 403), so `release.yml` gained a manual path: `workflow_dispatch` with `tag` (and optional `target`, default `main`) creates the tag through the API, then publishes (PR #4).
To release from an agent session, run the `release` workflow with `tag=vX.Y.Z` instead of pushing a tag.

## Phase 1: plan (confirmed)
Branch: the session's designated branch (agent sessions may only push there), one PR per logical unit, one PR per logical unit, CI green. The order was chosen so that the Shield pipeline (the riskiest part) is done first.

### 1.1 `core/shield`: engine
- [x] Wrapper around adblock-rust: `check`/`should_block`, cosmetics (`cosmetics`, `hidden_selectors`), scriptlets (`use_resources_json`)
- [x] `engine.dat`: SCYDE container (magic, format version, matcher tag) with typed errors
- [x] benchmark `cargo bench -p shield --bench match`: a percentile harness instead of criterion (ADR-0010), gated in CI, `perf.json` artifact
- **AC:** ✅ p50 1.6–3.3 µs, p99 2.5–4.4 µs (best of 3), deserialization ≈ 1 ms for 80k rules; fixture tests pass (6 blocked, 1 exception, 3 cosmetic)

### 1.2 `core/shield-compiler`: one source, all formats (PLAN §5.2, ADR-0011)
- [x] `engine.dat`
- [x] DNR rulesets ≤ 30k rules (our own converter), with a report of dropped rules (`$popup`, cosmetics, regex…)
- [x] iOS lists ≤ 150k, with a filter for regexes WebKit does not support (ADR-0006) and a report
- [x] cosmetic indexes, `resources.json`
- [x] `manifest.json` with hashes and an ed25519 signature; `verify` and `keygen` subcommands
- [x] Golden tests: `qa/lists/fixture.txt` → expected DNR / iOS / cosmetic output (`tests/golden/`); bundles are reproducible
- [x] macOS CI job `ios` → `shield-rulelists`: compiles the iOS output of real lists and the fixture on the simulator (tag/manual/`ios/**`)
- **AC:** ✅ DNR and iOS output within limits (tested), dropped rules reported in `report.json`, signature verified in a test (wrong key, tampered file and tampered manifest all rejected)
- **CI evidence (PR #6):** EasyList + EasyPrivacy + uBO filters → **132,009** WebKit rules in one list, compiled on the simulator in **6.0 s**, lookup 1 ms; fixture 10/11 (6/6 network, exception, generic and attribute cosmetics; the host-specific rule cannot match `file://`, as in G4)

### 1.3 `shield-wasm`: package for the GeckoView extension
- [x] `core/shield-wasm`: wasm-bindgen layer over `shield` (`fromDat`, `fromLists`, `useResources`, `shouldBlock`, `check`, `cosmetics`, `hiddenSelectors`); `build.sh` → `web` (extension) + `node` (bench)
- [x] CI job `core` → `wasm`: fixture decisions, p99 < 200 µs, ≤ 1000 KB gz; artifact `shield-wasm` (package + `perf.json`)
- **AC:** ✅ V8 p50 4.3 µs, p99 7.9–12.3 µs (best of 3; single-shot p99 50–72 µs), 80k rules build 149 ms; WASM 2.42 MB / **840 KB gz**

### 1.4 `filters.yml`: daily bundle
- [x] fetch lists (`tools/filters/fetch.mjs`, resolves uBO `!#include`) → golden tests → compile → check limits → sign → verify → release `filters-YYYYMMDD` + `filters-latest` (updated in place), prune > 14 days
- **Dry run on real lists (PR #8):** 169,280 filter lines → DNR **118,964** rules in 4 rulesets (none over the limit); iOS **148,676** rules (1 list, close to the 150k limit, so the compiler will split it into 2 when lists grow); cosmetic index of 32,686 hosts; `engine.dat` 8.5 MB, tarball 7.5 MB.
- [ ] `[OWNER]`: add the secret `FILTERS_SIGNING_KEY` and commit `brand/keys/filters.pub` (steps in `docs/filters-signing.md`). Until then the workflow builds and uploads an artifact but **does not publish**.
- [ ] `[OWNER]`: scriptlet resources (`resources.json`, uBO GPLv3, PLAN §16) are not fetched yet, so the bundle ships `[]`. The source and licence go with open question 3 (open/closed source).

### 1.5 `core/omnibox`
- [x] URL fixup (`scyde`, `localhost:3000`, `192.168.1.1`, `ftp://`), URL-or-search decision (PSL suffix check, https by default, `javascript:`/`data:` → search)
- [x] eTLD+1 display, punycode for mixed scripts and confusable IDNs (own policy: one script per label or an accepted CJK mix, Cyrillic/Greek lookalikes under a foreign TLD, Latin lookalike letters), elision that never cuts the domain, credentials never shown as the host
- [x] frecency ranking (`frecency`, `rank`, `inline_completion`)
- **AC:** ✅ 12 unit + 6 property tests (2,000 cases each); `аpple.com` → `xn--pple-43d.com`. Uses only crates already in the tree via adblock-rust (`idna`, `psl`, `icu_properties`); `proptest` is a dev-dependency.

### 1.6 `core/places`
- [x] SQLite (rusqlite bundled, WAL): history (visits, titles, delete URL / range / all), bookmarks tree (folders, move with renumbering, no cycles, built-in folders fixed), top sites (pinned + one tile per host by frecency, hidden hosts), suggestions (host prefix + trigram FTS5 → `omnibox::frecency::rank`), migrations via `user_version` (v1 core, v2 speed dial, v3 FTS), a newer schema is refused
- **AC:** ✅ test with 100k records (debug build, optimized SQLite): suggest host 4.1 ms, suggest title 0.7 ms (78 ms before FTS), history page 0.1 ms, history search 0.4 ms, top sites 0.2 ms, `record_visit` 0.3 ms

### 1.7 `core/settings`
- [x] JSON Schema as the source of truth (`core/settings/schema/settings.schema.json`), our own validator for exactly the keywords it uses (a test fails on any other keyword)
- [x] load = repair: invalid or unknown values fall back to schema defaults and are reported (never blocks startup); `set` validates; `reset`; migrations framework (`MIGRATIONS`, version in the file; newer files are read safely)
- [x] search templates without referral parameters; default search engine **DuckDuckGo** `[OWNER]` (open question 2); `top_stories` defaults to off (opt-in)
- **AC:** ✅ 6 tests; the accent enum is checked against `design/tokens/accent.json`

### 1.8 `core/theme`
- [x] tokens compiled in from `design/tokens/*.json` and resolved at runtime (the same source as CSS, Kotlin and Swift)
- [x] custom accent: seed → OKLCH → every role (light ink/ring, fill/island, dark ink = ring = fill, on-fill); if the seed fails, the **nearest passing lightness** (same hue, chroma clamped to sRGB) is used; `validate` rejects explicit roles below AA
- [x] same result as `tools/contrast-check`: all 204 pairs mirrored with the same labels and thresholds; in CI each pair is compared with the Node output (`THEME_CONTRAST_JSON`)
- **AC:** ✅ 10 tests; every hue × 4 lightness × 4 chroma (576 seeds) derives a valid accent; Ember's pastel is kept unchanged

### 1.9 `core/ffi`
- [x] UniFFI 0.32 (proc macros) → `libscyde_core`: `ShieldEngine`, `verify_filter_bundle`, omnibox (`omnibox_classify`, `omnibox_display_url`, `omnibox_elide_host`, `omnibox_search_url`), `PlacesStore`, `SettingsStore`, `theme_derive_accent`, `theme_contrast_ratio`, `core_version`; errors as `CoreError`; stores are `Send + Sync`
- [x] CI `android` → `core-ndk`: `ffi/build-android.sh` → `.so` for arm64-v8a, armeabi-v7a and x86_64 + Kotlin bindings (artifact `scyde-core-android`). The Kotlin bindings are compiled for real with the Android project in Phase 5.
- [x] CI `ios` → `core-apple` (tag/manual/`ios/**`/`core/ffi/**`): `ffi/build-apple.sh` → `ScydeCore.xcframework` + Swift package, and an XCTest that calls the core from Swift on the simulator (artifact `scyde-core-apple`)
- Note: bindings are generated from an unstripped host build, because the release profile strips the UniFFI metadata symbols. The shipped libraries stay stripped.

### 1.10 Measurement
- [x] `perf.json` from benchmarks (`shield`, `shield-wasm`), uploaded as artifacts
- [x] CI `core` → `perf` (PRs): the base commit's Shield bench and the PR's bench are alternated 3× on the same runner; a budgeted metric whose median is > 10 % worse fails (`tools/perf/compare.mjs`, with tests). A same-runner A/B, because absolute µs numbers swing ±50 % between shared runners.

**Phase 1 AC (PLAN §11):** `cargo test` green, clippy without warnings, budgets from 1.1, lists within limits, signature verified, omnibox tests pass.

| AC | Status |
|---|---|
| `cargo test` green, clippy `-D warnings` | ✅ CI `core` → `check` (pedantic lints, native and wasm32) |
| match p50 < 5 µs, p99 < 50 µs, deserialization < 50 ms | ✅ ADR-0010, gated in CI, A/B regression gate |
| iOS ≤ 150k per list, DNR within Chromium limits, dropped rules reported | ✅ real lists: iOS 148,676 (compiled on the simulator), DNR 118,964 in 4 rulesets; `report.json` |
| bundle signed and verified in a test | ✅ `shield-compiler` golden tests; publishing waits on `[OWNER]` key (1.4) |
| omnibox: URL or search, punycode/confusables, fixups | ✅ unit + property tests |

**Remaining `[OWNER]` items for Phase 1:**
- `FILTERS_SIGNING_KEY` and `brand/keys/filters.pub` (docs/filters-signing.md)
- the scriptlet resources source (resources.json, GPLv3; open question 3)
- default search engine (open question 2; DuckDuckGo until then)

## Phase 2: web design system and internal pages
- [x] `@scyde/ui`: Svelte 5 components from the Island prototype (BrandMark, Icon, Button, IconButton, Toggle, Segmented, Tabs, Favicon monogram, Kbd, Stat, Card, Skeleton, Toast), `base.css` (OFL fonts, tokens, focus, Rise), `applyAppearance` (data-theme/accent/density/motion), formatters
- [x] `@scyde/bridge`: typed API (13 methods, one allowlist), transports Tauri / WKWebView reply handler / GeckoView native messaging, trusted-origin check, timeouts, in-memory mock for dev and tests
- [x] Apps `newtab`, `settings` (rendered from `core/settings/schema/settings.schema.json`), `shields`, `library` (history + bookmarks); strict CSP meta in production builds
- [x] `@scyde/i18n`: `t()` with ICU plurals; all copy in `en.json`
- [x] Playwright (`web/e2e`): Chromium, Firefox, WebKit × light/dark × 1440/390 screenshots, axe (no serious/critical), no console/CSP errors, interaction tests; CI artifact `screenshots`
- [x] `pnpm size`: JS gz newtab 20.5 KB, settings 21.4 KB, shields 19.8 KB, library 24.7 KB (budget 120 KB)
- [ ] Lighthouse a11y ≥ 95: covered by axe in 3 engines for now; Lighthouse CI to be added
- Skills `emil-design-eng`, `animate`, `mobile-native` are not installed; DESIGN.md was followed directly

## Phase 3: Windows (ADR-0012), code on PR #14
- [x] `desktop/src-tauri`: Tauri 2.12.5 multiwebview, tabs on one WebView2 environment, island band (`web/apps/desktop-chrome`), omnibox (core), history (places), settings (settings), `scyde://` pages via a custom protocol with CSP headers, bridge with origin + allowlist checks
- [x] Shield: `shield-extension` chromium-mv3 (DNR + cosmetics) built from a fresh bundle; **11/11 on qa/adblock in Chromium locally**. The first CI smoke test in WebView2 scored 8/11 because the worker-based cosmetics did not run, so cosmetics now work without a service worker (a stylesheet plus host shards)
- [x] Popup blocker (`NewWindowRequested` + `IsUserInitiated`), shortcuts (`AcceleratorKeyPressed`)
- [x] `cargo check` + clippy pass for a Windows target locally (x86_64-pc-windows-gnu)
- [x] CI: NSIS installer → silent install → smoke run + screenshot
- [ ] Not yet: blocked-request counters (only popups are counted), per-site switches, suggestions popover, session restore, downloads, find, zoom, private window, default browser, updater (`TAURI_SIGNING_PRIVATE_KEY` `[OWNER]`), frameless window and floating island (Phase 6)

## Phase 4: iOS, code on PR #14
- [x] `ios/SCYDE` (xcodegen): SwiftUI island (back, address with eTLD+1, Shield, tabs, menu, swipes), tab grid, WKWebView tabs with a Safari UA suffix, `scyde://` scheme handler, bridge (main-frame `scyde://` only), ScydeCore (omnibox, places, settings), WKContentRuleLists from the bundle (compiled once, cached per version), WebKit popup gate + popups open as tabs
- [x] CI `ios` → `app`: simulator screenshots (New Tab light/dark, qa/adblock, settings) + **unsigned .ipa** for Feather
- [ ] `[OWNER]`: install via Feather on a real device, Google sign-in check (Phase 4 AC)
- [ ] Not yet: blocked counter estimate, per-site switches, downloads, find, private tabs

## Phase 5: Android, code on PR #14
- [x] `android/` Kotlin + Compose + GeckoView 160: edge-to-edge, predictive back, island (address with inline completion, Shield, tabs, menu, swipes), tab grid, ETP Strict + TCP, built-in extension (gecko-mv2: WASM engine from `engine.dat`, cosmetics, internal pages), bridge over native messaging (sender URL + environment + allowlist), popup prompts denied and counted, ScydeCore via UniFFI + JNA
- [x] CI `android` → `app`: per-ABI release APKs (owner keystore if the secrets exist, otherwise a per-build test key), emulator screenshots
- [ ] `[OWNER]`: release keystore secrets (`ANDROID_KEYSTORE_B64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`), so that updates install over earlier versions
- [ ] Not yet: Baseline Profile, live-session limit, downloads, private tabs

## Release
- [x] `release.yml` calls `windows`, `ios` and `android` as reusable workflows and attaches the `.exe`, `.ipa` and `.apk` files (a tag created with `GITHUB_TOKEN` does not trigger other workflows)
- [x] **v0.2.0 published** (release run 43109526691): `SCYDE-v0.2.0-windows-x64-setup.exe` (in the CI smoke test, `qa/adblock` scores 11/11 in WebView2 and the popup is blocked), `SCYDE-v0.2.0-ios-unsigned.ipa`, `scyde-0.2.0-{arm64-v8a,armeabi-v7a,x86_64}.apk`
- [x] Phase 7 (partial): `NOTICE.md`, `PRIVACY.md` (draft), `docs/release.md`; `NETWORK.md` records that v0.2.0 makes no calls of its own

- [x] **v0.2.1** (owner's phone test, 2026-11-12): scroll guard (all platforms), iOS island follows the active tab, De-AMP (core `omnibox::deamp`), internal pages as sheets (iOS sheet, Android sheet session with ×), island swipe in the Initial pass / simultaneous gesture, Android edge swipe back/forward (`EdgeSwipeLayout`). CI: Android emulator swipes `qa/scroll` and fails if the page does not move
- [ ] Open: redirect stubs (`$redirect` resources) and scriptlets. Blocking `gpt.js`/`gtm.js` without uBO-style stubs can break sites that call them; we need our own stub resources (not uBO's GPL files). `[OWNER]`: send sites that still break

## Phase 6 (owner, 2026-11-12: "implement all the features and more, better than Arc")
- [x] **v0.3.0, phones:**
  - island v2 (mark menu · back · address · Shield light/count · tabs)
  - full screen while reading (DESIGN v2.1 "Hide")
  - control center, command omnibox (open tabs, history, bookmarks, actions)
  - Shield per site (`shield.disabled_sites`; Android: allowlist over a native port + per-page counts)
  - private tabs, find in page, desktop site, bookmark star, Peek, Aura tint (`theme::aura`)
  - welcome on first run (`web/apps/welcome`, also on Windows)
  - prototype states in `design/prototypes/mobile`
- [x] **v0.4.0:**
  - phones (ADR-0013): session restore (`session.json`), sleeping tabs (`tabs.sleep_after_minutes`), pinned tabs, Spaces with their own colour
  - Windows (ADR-0014): the island floats in a transparent window over the page, with no black band
- [ ] v0.5.0:
  - Windows: island v2 (control center, command omnibox, hide while reading), Shield per site (dynamic DNR allow rules), Ctrl+K palette, split view, web panels
  - Zen mode (annoyance lists, `[OWNER]` licences)
- Known: the Windows smoke test once scored `qa/adblock` 10/11 (`allow:exception`) and 11/11 on rerun. On first launch the DNR rulesets do not all load at the same moment, so for a few hundred ms a block rule can win over its exception.
- Skills `animate`, `break-ui` and others from PLAN §12 are not installed; DESIGN §6 was followed directly

## Next (owner)
- `[OWNER]` Phase 6: confirm the order of the unique features (Aura tint, Shield pill, sleeping tabs, split view + Ctrl+K, web panels, Peek, Zen mode, Spaces)
- `[OWNER]` Phase 7: SCYDE Sync yes/no; licence of our own code; privacy policy contact
- `[OWNER]` keys: Android keystore, Windows code signing + `TAURI_SIGNING_PRIVATE_KEY`, `FILTERS_SIGNING_KEY`
- `[OWNER]` real-device tests: Windows install, Feather install on iOS, Android APK, Google sign-in on all three

## Skills
`impeccable`, `taste-skill`, `prototype` and `pick-ui-library` do not exist in the plugin catalog, so they are **implemented as project skills** in `.claude/skills/` (2026-11-11) and are loaded in every session in this repo:
- `impeccable`: quality gate against DESIGN.md (critique / audit / polish / finalize), findings P1–P3, mandatory format for PRs
- `taste-skill`: genericness check (list of "AI-made" signs, score 1–5 on 5 axes, threshold ≥ 4)
- `prototype`: HTML prototypes on the real tokens + `shot.mjs` (Playwright, light/dark × 1440/390)
- `pick-ui-library`: decision by measuring builds (ADR-0005)

The other skills from PLAN §12 (`emil-design-eng`, `apple-design`, `mobile-native`, `animate`, `choreograph`, `animation-*`, `break-ui`, `awwwards-animations`) are not installed yet; they will be built using the same pattern when a phase calls for them.

### taste-skill: Aura S icon
Generic signs: none (no 2 circles, no gloss or sparkle; R1 round 1 Aura Drop was rejected because of the 3D sphere).
Scores: distinctiveness 4/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 5/5 → **pass**.
Signature: S monogram (SCYDE), soft conic Ember gradient, white/onyx squircle tile.

### impeccable critique: Aura S icon
Rendered: `brand/icon/preview.png` (light, dark, mono, 64/32/16 px) + Android/iOS/Windows exports
| # | P | Finding | Rule | Fix | Status |
|---|---|---|---|---|---|
| 1 | P2 | The tip of the conic gradient fell on the bend of the S (dark spot) | DESIGN §7, no visible tip | gradient center moved outside the mark | ✅ |
| 2 | P3 | Mark too small within the tile (~55 %) | §7 legibility | stroke 108→118, radius 116→128 | ✅ |
| 3 | P3 | The bend turns into a step at 16 px | legibility | accepted: the S is still clear; the detail pays off at large sizes | – |
Verdict: **ship**.

## Open questions for the owner `[OWNER]`
Defaults apply until the owner decides (PLAN §17).

| # | Question | Default | Status |
|---|---|---|---|
| 1 | Name and home screen label | "SCYDE Browser" | open |
| 2 | Default search engine | DuckDuckGo | open |
| 3 | Open source or closed source (affects list licenses, PLAN §16) | – | open |
| 4 | Aeonik license or OFL replacement | Bricolage Grotesque + Geist Mono (OFL) | resolved (ADR-0008) |
| 5 | Windows code signing | unsigned (SmartScreen warns) | open |
| 6 | SCYDE Sync self-hosted | – | open (Phase 7) |
| 7 | Sources for Top Stories | off (opt-in) | open |
| 8 | Android distribution | GitHub Releases | open |
| 9 | Icon | – | **decided 2026-11-11**: the owner's SCYDE S in Ember (#F2603F + ink) |
| 12 | Brand energy: Ember or lime? | Ember | **decided 2026-11-11**: Ember, and the logo was switched to Ember |
| 11 | "Clean, our own code" rule: GeckoView and adblock-rust? | – | **decided 2026-11-11**: GeckoView stays (Android engine as a library), adblock-rust stays (Shield matcher). No browser fork. |
| 10 | Regional HR/EU filter lists | off | open |
| 13 | Language | – | **decided 2026-11-11**: English only (ADR-0009) |

## Owner's rule (2026-11-11)
> "It must not be built like Brave, Firefox and the others, clean, our own custom code."

Decision: **no fork or rebrand** of Chromium, Brave or Firefox. The shell, UI, Shield logic, omnibox, history and themes are our own code. The render engine is the platform one (WebView2 / WKWebView), because a custom engine is not realistic for v1. The owner confirmed two libraries as permitted: GeckoView (Android) and adblock-rust (Shield). The rule is also written into CLAUDE.md (rule 11).

## Blockers
- None that block the work. Open `[OWNER]` items are listed in the phase sections.

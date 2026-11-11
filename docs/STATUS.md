# STATUS.md

> At the start of every session, read this file and continue from the first open task of the current phase.

**Current phase:** Phase 1, Rust core and Shield pipeline (PLAN §11) · **plan proposed, awaiting confirmation `[OWNER]`**
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

## Phase 1: plan (proposal)
Branch `phase/1-core`, one PR per logical unit, CI green. The order was chosen so that the Shield pipeline (the riskiest part) is done first.

### 1.1 `core/shield`: engine
- [ ] Wrapper around adblock-rust: `should_block`, cosmetics (`url_cosmetic_resources`, `hidden_class_id_selectors`), scriptlets
- [ ] `engine.dat`: serialization and deserialization with a format version
- [ ] criterion benchmark `cargo bench -p shield --bench match`
- **AC:** native match p50 < 5 µs, p99 < 50 µs; deserialization < 50 ms (desktop); fixture `qa/lists/fixture.txt` yields correct decisions

### 1.2 `core/shield-compiler`: one source, all formats (PLAN §5.2)
- [ ] `engine.dat`
- [ ] DNR rulesets ≤ 30k rules, with a report of dropped rules (`$popup`, cosmetics, regex…)
- [ ] iOS lists ≤ 150k, with a filter for regexes WebKit does not support (ADR-0006) and a report
- [ ] cosmetic indexes, `resources.json`
- [ ] `manifest.json` with hashes and an ed25519 signature
- [ ] Golden tests: `qa/lists/fixture.txt` → expected DNR / iOS / cosmetic output
- [ ] macOS CI job that compiles the iOS output on the simulator (tag/manual)
- **AC:** DNR and iOS output are within limits, dropped rules are reported, the signature is verified in a test

### 1.3 `shield-wasm`: package for the GeckoView extension
- [ ] wasm32 build + wasm-bindgen (from S3), size within budget, p99 test in CI

### 1.4 `filters.yml`: daily bundle
- [ ] fetch lists → compile → golden tests → sign → release `filters-YYYYMMDD` + `filters-latest`
- `[OWNER]`: add the secret `FILTERS_SIGNING_KEY` (I will generate a key pair and instructions; the public key goes into `brand/keys/`)

### 1.5 `core/omnibox`
- [ ] URL fixup (`scyde`, `localhost:3000`, `192.168.1.1`, `ftp://`), URL-or-search decision
- [ ] eTLD+1 display, punycode for mixed scripts and confusable IDNs
- [ ] frecency ranking
- **AC:** property tests for parsing; the homograph `аpple.com` (Cyrillic "а") is displayed as punycode

### 1.6 `core/places`
- [ ] SQLite (rusqlite bundled): history, bookmarks, top sites, migrations; test with 100k records

### 1.7 `core/settings`
- [ ] JSON Schema as the source of truth, validation, migrations
- [ ] default search engine DuckDuckGo `[OWNER]`

### 1.8 `core/theme`
- [ ] custom accent: OKLCH → fill/ink, rejection below AA, nearest acceptable shade (DESIGN §2.2)
- [ ] same result as `tools/contrast-check`

### 1.9 `core/ffi`
- [ ] UniFFI → Swift and Kotlin
- [ ] CI: `.so` per ABI (cargo-ndk), `ScydeCore.xcframework` (macOS, tag/manual)

### 1.10 Measurement
- [ ] `perf.json` artifact from benchmarks; a regression > 10 % fails the job (PLAN §8.2)

**Phase 1 AC (PLAN §11):** `cargo test` green, clippy without warnings, budgets from 1.1, lists within limits, signature verified, omnibox tests pass.

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
None.

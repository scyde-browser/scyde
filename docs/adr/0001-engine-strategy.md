# ADR-0001: Engine strategy (our own shell around platform engines)

- **Status:** Accepted (G1–G5 passed, Phase 0 closed 2026-11-11)
- **Date:** 2026-11-11
- **Decided by:** Claude Code, per PLAN §2
- **Related:** PLAN.md §2.1–2.3, gates G1, G3, G4, G5

## Context
SCYDE Browser must run on Windows, Android and iOS, with built-in ad blocking, a premium look and strict performance budgets (PLAN §8). It is built by a single developer with Claude Code, and Windows and iOS are built exclusively in GitHub Actions. A browser is a security product, so the engine must receive security patches without any work on our side.

## Options
1. **Chromium fork (the Brave route).** Full control, one engine on desktop and Android.
   Cons: the build needs hundreds of GB and hours on powerful machines; GitHub hosted runners cannot handle it; Chromium patches every few weeks have to be rebased manually. On iOS it has to be WebKit anyway.
2. **Gecko desktop fork (the Zen/Floorp route) + GeckoView + WKWebView.** The same engine on desktop and Android.
   Cons: rebase onto Firefox every 4 weeks and a full build on a self-hosted runner.
3. **Our own shell around self-updating platform engines** + a shared Rust core:
   - Windows: Tauri 2 + WebView2 Evergreen
   - Android: Kotlin/Compose + GeckoView (not Android WebView, because of Google sign-in and `webRequestBlocking`)
   - iOS: Swift + WKWebView (the only option outside the EU)

## Decision
**Option 3.** All of the "brain" (Shield via adblock-rust, omnibox, places, settings, theme) lives in `core/` (Rust) and is shared through a crate (Windows), WASM (GeckoView extension) and UniFFI (Swift/Kotlin). A shared web UI (`web/`, Svelte 5) is used for the desktop chrome and internal pages.

The Windows part is **conditional**: if spike S1 shows that Google sign-in does not work in WebView2 (G1 fails), the fallback from PLAN §2.3 (Gecko desktop fork) is activated in a new ADR (ADR-0002), subject to an `[OWNER]` decision.

## Consequences
- Engine security patches: Windows and iOS automatically (Evergreen / OS); Android through a Renovate bump of GeckoView (every 4 weeks, security bump within 48 h).
- Shield has three integrations (DNR extension / webRequest extension / WKContentRuleList) from a single source (`shield-compiler`). This is the main cost of this decision.
- Page behavior is not identical across all three platforms (Chromium, Gecko, WebKit). Playwright tests therefore run on all three engines (PLAN §11, Phase 2).
- Tauri multiwebview requires the `unstable` feature → the version is pinned, and S5 measures stability (G5).
- Google sign-in is risk no. 1 → G1 on all three engines before Phase 1.

## Measurements / evidence
Spikes S1–S5 confirmed the strategy: ADR-0002 (G1), 0003 (G2), 0004 (G3), 0006 (G4), 0007 (G5).

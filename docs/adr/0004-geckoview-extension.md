# ADR-0004: Shield on Android: GeckoView built-in extension with a WASM engine

- **Status:** Accepted (Phase 0 closed 2026-11-11)
- **Date:** 2026-11-11
- **Decided by:** Claude Code, based on spike S3
- **Related:** PLAN §5.4, gate **G3**, `spikes/s3-geckoview`, CLAUDE.md rule 11 (GeckoView and adblock-rust approved as libraries)

## Context
On Android, Shield runs as a built-in WebExtension in GeckoView (MV2 `webRequestBlocking`), with the adblock-rust engine compiled to WASM. G3 requires the extension to block the fixture, the WASM decision to be p99 < 200 µs, and internal pages to talk to the app.

## Options
1. **Built-in MV2 extension + WASM engine** (the plan).
2. Only GeckoView ETP (Strict) without our own engine: too little coverage (no EasyList / uBO rules and no cosmetics).
3. Engine in Kotlin/Rust via native messaging for every decision: IPC per request, slower and more complex.

## Decision
**Option 1**, with ETP Strict and Total Cookie Protection as an additional layer.

## Consequences
- **GeckoView 160** requires compileSdk **37.2**, AGP 9.5+ and Kotlin 2.4 (kotlin-stdlib 2.4.21 comes with GeckoView). The Android project in Phase 5 starts from that.
- **Engine build:** from 139k lines it takes 1.7 s at startup (incl. fetching the lists). In Phase 1 a serialized `engine.dat` is loaded instead.
- **Until the engine is ready:** requests pass unfiltered, so in the spike the app waits for a "ready" message before the first load. In production this means: the first navigation waits for the engine, or the engine is loaded from `engine.dat` before anything is displayed.
- **Cosmetics:** the content script requests selectors from the background (`url_cosmetic_resources` + `hidden_class_id_selectors`). Works, incl. generic and attribute rules.
- **Size:** the WASM is 2.8 MB (844 KB gz) inside the APK, within budget (≤ 10 MB of our code).

## Measurements / evidence
CI run 43017809149 / 43019194222 (`s3-geckoview-app`, emulator API 34 x86_64, GeckoView 160.0):
- Fixture: **10/11**. All 6 network requests are blocked, the exception is let through, and generic and attribute cosmetics work. The host-specific cosmetic rule targeted 127.0.0.1, but the emulator uses 10.0.2.2; the rule has been added.
- WASM in SpiderMonkey: **p99 40 µs**, p90 20 µs, max 240 µs. Measured as 3000 requests × 50 repetitions, because the timer resolution in the extension is 1 ms → G3 ✅ (< 200 µs).
- WASM in V8 (CI proxy, 50k decisions): p50 5.5 µs, p99 14.8 µs.
- Internal page (`moz-extension://…/internal.html`, displayed as `scyde://newtab`) → `runtime.sendNativeMessage` → Kotlin `MessageDelegate` → reply back: ✅.
- UA: `Mozilla/5.0 (Android 14; Mobile; rv:160.0) Gecko/160.0 Firefox/160.0` (relevant for G1).

> The spike code was deleted at the close of Phase 0. The source code and CI workflow are in git history at commit `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

# ADR-0002: Google sign-in in embedded engines (G1)

- **Status:** Accepted (G1 ✅ on all three engines, owner 2026-11-11)
- **Date:** 2026-11-11
- **Related:** PLAN §2.2, §2.3, gate **G1**, `docs/STATUS.md` (S1 steps)

## Context
Google blocks OAuth sign-in in "embedded webviews". Without sign-in to Gmail and YouTube the browser is unusable. G1 requires sign-in to work on WebView2, GeckoView and WKWebView.

## Prepared for testing
| Engine | Build | UA |
|---|---|---|
| WebView2 | artifact `s1-windows-exe` (`scyde-spike-s5.exe`, mode `signin`) | unmodified WebView2 UA (= Edge) |
| GeckoView 160 | artifact `s3-geckoview-apk` | `Mozilla/5.0 (Android 14; Mobile; rv:160.0) Gecko/160.0 Firefox/160.0` |
| WKWebView | artifact `s1-ios-ipa` (Feather) | system + `Version/<iOS> Mobile/15E148 Safari/604.1` |

## Decision
**Sign-in works on all three engines.** The fallback from PLAN §2.3 is not needed. The strategy from ADR-0001 stands unchanged:
- Windows: Tauri 2 + WebView2, unmodified UA
- Android: GeckoView
- iOS: WKWebView with the Safari suffix in the UA

**Rule for all subsequent phases:** after every change of engine, UA or GeckoView version, repeat this test (CLAUDE.md).

## Measurements / evidence
| Engine | Google sign-in | Gmail | YouTube | Drive | "Sign in with Google" (3rd party) | Source |
|---|---|---|---|---|---|---|
| WebView2 (`scyde-spike-s5.exe`, unmodified UA) | ✅ | ✅ | ✅ | ✅ | ✅ | owner, 2026-11-11: "all sign-ins work in all of these open tabs" |
| GeckoView 160 (`s3-geckoview-apk`) | ✅ | ✅ | ✅ | ✅ | ✅ | owner, 2026-11-11: "everything has been tested and everything works" |
| WKWebView + Safari UA (`s1-ios-ipa`) | ✅ | ✅ | ✅ | ✅ | ✅ | owner, 2026-11-11 |

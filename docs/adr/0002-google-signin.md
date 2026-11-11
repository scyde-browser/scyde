# ADR-0002: Google prijava u embedded engineima (G1)

- **Status:** Prihvaćeno (G1 ✅ na sva tri enginea, vlasnik 2026-11-11)
- **Datum:** 2026-11-11
- **Veza:** PLAN §2.2, §2.3, gate **G1**, `docs/STATUS.md` (S1 koraci)

## Kontekst
Google blokira OAuth prijavu u "embedded webviewima". Bez prijave u Gmail i YouTube preglednik je neupotrebljiv. G1 traži da prijava radi na WebView2, GeckoViewu i WKWebViewu.

## Pripremljeno za test
| Engine | Build | UA |
|---|---|---|
| WebView2 | artefakt `s1-windows-exe` (`scyde-spike-s5.exe`, mod `signin`) | nepromijenjen WebView2 UA (= Edge) |
| GeckoView 160 | artefakt `s3-geckoview-apk` | `Mozilla/5.0 (Android 14; Mobile; rv:160.0) Gecko/160.0 Firefox/160.0` |
| WKWebView | artefakt `s1-ios-ipa` (Feather) | sistemski + `Version/<iOS> Mobile/15E148 Safari/604.1` |

## Odluka
**Prijava radi na sva tri enginea.** Fallback iz PLAN §2.3 nije potreban. Strategija iz ADR-0001 vrijedi bez promjena:
- Windows: Tauri 2 + WebView2, nepromijenjen UA
- Android: GeckoView
- iOS: WKWebView sa Safari sufiksom u UA

**Pravilo za sve sljedeće faze:** nakon svake promjene enginea, UA-a ili verzije GeckoViewa ponovi ovaj test (CLAUDE.md).

## Mjerenja / dokazi
| Engine | Google prijava | Gmail | YouTube | Drive | "Sign in with Google" (3rd party) | Izvor |
|---|---|---|---|---|---|---|
| WebView2 (`scyde-spike-s5.exe`, nepromijenjen UA) | ✅ | ✅ | ✅ | ✅ | ✅ | vlasnik, 2026-11-11: "rade sve prijave na svim ovim otvorenim tabovima" |
| GeckoView 160 (`s3-geckoview-apk`) | ✅ | ✅ | ✅ | ✅ | ✅ | vlasnik, 2026-11-11: "sve je testirano i sve radi" |
| WKWebView + Safari UA (`s1-ios-ipa`) | ✅ | ✅ | ✅ | ✅ | ✅ | vlasnik, 2026-11-11 |

# ADR-0002: Google prijava u embedded engineima (G1)

- **Status:** **Čeka ručni test `[VLASNIK]`**
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
Otvoreno. Ako WebView2 padne, aktivira se PLAN §2.3 (Gecko desktop fork) i Faza 1 ne počinje.

## Mjerenja / dokazi
(screenshotovi i rezultati vlasnika)

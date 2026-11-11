# Spike S5 (+ S2 opcija B, + S1 Windows): Tauri 2 multiwebview na WebView2

Jedna aplikacija, tri moda (`scyde-spike-s5.exe --mode …`):

| Mod | Gate | Što radi |
|---|---|---|
| `bench` | **G5** | 20 tab webviewa na istom WebView2 environmentu, 200 prebacivanja (hide/show/focus), unaprijed kreiran transparentni popover (50× show), Mica, vlastiti titlebar. Piše `s5.json` + screenshot. |
| `shield --shield none\|b` | **G2** | Učitava `qa/adblock` + 5 teških stranica (3 runde), mjeri PageLoad Started→Finished. `b` = `WebResourceRequested` filter + adblock-rust u hostu (403 za blokirano). Piše `s2-*.json`. |
| `signin` (default) | **G1** | Otvara Google prijavu, Gmail, YouTube, Drive i "Sign in with Google" na trećoj stranici, s nepromijenjenim WebView2 UA. Ručni test. |

CI: `.github/workflows/spikes.yml` → job `s5-windows`; artefakti `s5-s2-results` (JSON + PNG) i `s1-windows-exe`.

Verzije: tauri 2.12.5 (feature `unstable`), tauri-build 2.7.4, webview2-com 0.39.1, windows 0.62.

## Otvoreno
- **S2 opcija A** (MV3 ekstenzija s DNR rulesetovima kroz `browser_extensions_enabled` / `extensions_path`) i **opcija C** (CDP `Network.setBlockedURLs`): sljedeći korak. Treba DNR konverter (dolazi s `shield-compiler`om; za spike ograničena konverzija `||host^` pravila).
- **Snap Layouts:** child WebView2 HWND prima miš iznad caption gumba, pa `WM_NCHITTEST → HTMAXBUTTON` na glavnom prozoru ne vrijedi tamo. Opcije za ADR-0007: (a) tanki nativni overlay HWND iznad maximize gumba, (b) `tauri-plugin-decorum`, (c) nativni caption gumbi. Provjerava se ručno.
- Vizualno trajanje prebacivanja (frame) se mjeri ručno ili kroz ETW trace; CI mjeri host-side vrijeme.

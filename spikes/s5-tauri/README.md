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

## Rezultati: CI run 43017487417 (windows-latest, 2026-11-11)

### S5 (G5)
| Metrika | Vrijednost | Cilj |
|---|---|---|
| Prebacivanje taba (host: show + hide + focus), 186 prebacivanja među 20 tabova | p50 **0,06 ms**, p95 **0,16 ms**, max 0,86 ms | ≤ 16 ms ✅ |
| Prikaz unaprijed kreiranog popovera (50×) | p50 **0,02 ms**, p95 **0,10 ms**, max 0,14 ms | ≤ 16 ms ✅ |
| Kreiranje 20 tab webviewa (isti environment) | 3,7 s ukupno (≈ 185 ms po tabu) | – (lijeno kreiranje tabova, PLAN §8.2) |
| Učitano tabova | 20/20 | ✅ |

Host-side mjerenje; vizualni frame provjerava se na screenshotu i ručno. Mica i custom titlebar su na screenshotu `s5.png` (artefakt).

### S2 (G2): medijan od 3 učitavanja, PageLoad Started → Finished
| Stranica | Bez blokiranja | Opcija B | Blokirano po učitavanju (B) |
|---|---|---|---|
| qa/adblock fixture | 22 ms (2/11) | 20 ms (**7/11**) | 5 |
| index.hr | 1343 ms | **227 ms** | 14 |
| 24sata.hr | 780 ms | **158 ms** | 15 |
| jutarnji.hr | 1046 ms | **913 ms** | 44 |
| youtube.com | 960 ms | **674 ms** | 4 |
| edition.cnn.com | 555 ms | 561 ms | 9 |

Opcija B ne dodaje vrijeme; učitavanje je brže jer se reklamni zahtjevi ne izvršavaju. Fixture 7/11: kozmetika (3) se u B još ne radi (dolazi bootstrap skripta, PLAN §5.4), a subframe je propušten jer WebView2 iframe prijavljuje kao `DOCUMENT` (popravljeno u sljedećem commitu).
Opcija A u tom runu nije učitala ekstenziju (`extensions_path` traži podmape po ekstenziji; popravljeno). Rezultat A i C slijedi.

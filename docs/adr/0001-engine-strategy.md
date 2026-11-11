# ADR-0001: Strategija enginea (vlastita ljuska oko platformskih enginea)

- **Status:** Prihvaćeno (G1–G5 prošli, Faza 0 zatvorena 2026-11-11)
- **Datum:** 2026-11-11
- **Odlučuje:** Claude Code, prema PLAN §2
- **Veza:** PLAN.md §2.1–2.3, gateovi G1, G3, G4, G5

## Kontekst
SCYDE Browser mora raditi na Windowsu, Androidu i iOS-u, s ugrađenim blokiranjem reklama, premium izgledom i strogim performance budgetima (PLAN §8). Radi ga jedan developer s Claude Codeom, a Windows i iOS se grade isključivo u GitHub Actions. Preglednik je sigurnosni proizvod, pa engine mora dobivati sigurnosne zakrpe bez našeg rada.

## Opcije
1. **Chromium fork (Brave-put).** Puna kontrola, jedan engine na desktopu i Androidu.
   Mane: build traži stotine GB i sate na jakim strojevima; GitHub hosted runneri to ne podnose; zakrpe Chromiuma svakih nekoliko tjedana treba ručno rebaseati. Na iOS-u ionako mora biti WebKit.
2. **Gecko desktop fork (Zen/Floorp put) + GeckoView + WKWebView.** Isti engine na desktopu i Androidu.
   Mane: rebase na Firefox svaka 4 tjedna i full build na self-hosted runneru.
3. **Vlastita ljuska oko platformskih enginea koji se sami ažuriraju** + zajednička Rust jezgra:
   - Windows: Tauri 2 + WebView2 Evergreen
   - Android: Kotlin/Compose + GeckoView (ne Android WebView, zbog Google prijave i `webRequestBlocking`)
   - iOS: Swift + WKWebView (jedina opcija izvan EU-a)

## Odluka
**Opcija 3.** Sav "mozak" (Shield preko adblock-rust, omnibox, places, settings, theme) živi u `core/` (Rust) i dijeli se kroz crate (Windows), WASM (GeckoView ekstenzija) i UniFFI (Swift/Kotlin). Zajednički web UI (`web/`, Svelte 5) koristi se za desktop chrome i interne stranice.

Windows dio je **uvjetan**: ako spike S1 pokaže da Google prijava u WebView2 ne radi (G1 padne), aktivira se fallback iz PLAN §2.3 (Gecko desktop fork) u novom ADR-u (ADR-0002), uz odluku `[VLASNIK]`.

## Posljedice
- Sigurnosne zakrpe enginea: Windows i iOS automatski (Evergreen / OS); Android kroz Renovate bump GeckoViewa (svaka 4 tjedna, sigurnosni bump u 48 h).
- Shield ima tri integracije (DNR ekstenzija / webRequest ekstenzija / WKContentRuleList) iz jednog izvora (`shield-compiler`). To je glavni trošak ove odluke.
- Ponašanje stranica nije identično na sve tri platforme (Chromium, Gecko, WebKit). Playwright testovi zato idu na sva tri enginea (PLAN §11, Faza 2).
- Tauri multiwebview traži feature `unstable` → verzija se pina, a S5 mjeri stabilnost (G5).
- Google prijava je rizik br. 1 → G1 na sva tri enginea prije Faze 1.

## Mjerenja / dokazi
Spikeovi S1–S5 su potvrdili strategiju: ADR-0002 (G1), 0003 (G2), 0004 (G3), 0006 (G4), 0007 (G5).

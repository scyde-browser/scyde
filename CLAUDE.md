# CLAUDE.md: SCYDE Browser

Ovo je radna uputa za Claude Code u ovom repou. Kompletan plan je u **PLAN.md**, vizualni sustav u **DESIGN.md**.
Na početku svake sesije pročitaj `docs/STATUS.md` i nastavi od prvog otvorenog zadatka trenutne faze.

## Što gradimo
Vlastiti preglednik (kao Brave, ali vlastiti) za **Windows, Android i iOS**. Zahtjevi:
- unikatan
- bez reklama, s ugrađenim ad, tracker i popup blockerom
- Dark/Light i prilagodba tema
- premium izgled
- jako optimiziran

| Platforma | Engine i shell |
|---|---|
| Windows | Tauri 2 + WebView2 |
| Android | Kotlin/Compose + GeckoView |
| iOS | Swift + WKWebView, .ipa za Feather |

Zajednička Rust jezgra je `core/`, zajednički web UI je `web/`.

## Zlatna pravila
1. **Faze i gateovi.** Radi samo na trenutnoj fazi (PLAN §11). Ne preskači gateove G1–G5. Kad nešto ovisi o vlasniku, označi `[VLASNIK]`, upiši u `docs/STATUS.md` i nastavi s onim što ne ovisi o toj odluci.
2. **Bez reklama i bez telemetrije.** Nikad ne dodaj sponzorirani sadržaj, affiliate linkove, analitiku ni SDK-ove trećih strana koji šalju podatke. Svaki novi mrežni poziv preglednika dokumentiraj u `docs/NETWORK.md`.
3. **Performanse su zahtjev, ne želja.** Poštuj budgete iz PLAN §8. PR koji dira vrući put (startup, prebacivanje tabova, Shield match, animacije) uz sebe nosi mjerenje.
4. **Dizajn samo kroz tokene.** Nema hardkodiranih boja, razmaka, radijusa ni trajanja. Sve dolazi iz `design/tokens` (CSS varijable, Kotlin, Swift). Nova vrijednost znači prvo token, pa tek onda kod.
5. **Kretanje:** animiraj samo `transform` i `opacity` (web) i ekvivalente na nativnim platformama. Animacije moraju biti prekidive i poštovati reduced motion. Trajanja i springovi dolaze iz DESIGN.md §6.
6. **Sigurnost:**
   - native bridge prima poruke samo od `scyde://` origina (provjera origina + allowlista metoda)
   - stroga CSP na internim stranicama
   - remote debugging samo u debug buildovima
   - nikad ne gasi provjeru TLS-a
   - omnibox nikad ne skriva pravu domenu
7. **Ne kopiraj reference.** Slike u `design/references/` su smjer. Logo, layout i assete tuđih radova ne preuzimaj. Ikona ne smije biti "dva preklopljena kruga" (PLAN §16).
8. **Odluke idu u ADR.** Svako odstupanje od PLAN.md dobiva `docs/adr/NNNN-*.md`.
9. **Mali PR-ovi, zeleni CI.** Conventional Commits. Grane `phase/N-*`, `feat/*`, `fix/*`. Svaki UI PR ima screenshotove (CI artefakti) i `impeccable` critique u opisu.
10. **i18n od prvog dana.** Hrvatski i engleski. Nema hardkodiranih stringova u UI-ju. Ključevi su u `web/packages/i18n`, `android/.../strings.xml` i `ios/.../Localizable.xcstrings`.
11. **Čisti naš kod, ne fork.** SCYDE se ne gradi kao Brave, Firefox ili drugi preglednici: nema forka ni rebrandanja Chromiuma, Bravea, Firefoxa ni njihovog koda. Ljuska, UI, Shield logika, omnibox, povijest, teme i sve značajke pišemo sami. Render engine je platformski (WebView2, WKWebView). Svaka nova vanjska biblioteka koja nosi kod drugog preglednika ide prvo vlasniku `[VLASNIK]` (vidi `docs/STATUS.md`, pitanje 11).

## Okruženje i build
Claude Code obično radi na **Linuxu**. Lokalno radi `core/`, `web/`, `shield-extension/` i `android/`. **Windows i iOS se grade i testiraju u GitHub Actions.** Iteriraj ovako:
```bash
gh workflow run windows.yml --ref <grana>   # ili ios.yml
gh run watch                                 # prati
gh run view --log-failed                     # ako padne
gh run download <run-id> -n screenshots      # vizualna provjera
```

### Naredbe (popuni ih u Fazi 0 i drži ažurnima)
```bash
# core
cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace
cargo bench -p shield --bench match            # Shield latencija
cargo run -p shield-compiler -- --lists qa/lists --out dist/filters

# web (pnpm workspace)
pnpm i && pnpm -r lint && pnpm -r typecheck && pnpm -r test
pnpm --filter @scyde/tokens build              # Style Dictionary → CSS/Kotlin/Swift
pnpm --filter newtab dev
pnpm e2e                                        # Playwright: chromium + firefox + webkit, light/dark
pnpm size                                       # bundle budgeti

# shield-extension
pnpm --filter shield-extension build -- --target chromium-mv3
pnpm --filter shield-extension build -- --target gecko-mv2

# android
cd android && ./gradlew assembleDebug testDebugUnitTest
./gradlew :benchmark:connectedCheck            # Macrobenchmark (emulator: samo trend)

# desktop (samo Windows / CI)
cd desktop && pnpm tauri build

# ios (samo macOS / CI)
cd ios && xcodegen && xcodebuild -scheme SCYDE -destination 'platform=iOS Simulator,name=<iPhone iz xcrun simctl list>' test
```

## Skillovi: kada ih koristiti
Detaljna mapa je u PLAN §12. Ukratko:

| Situacija | Skill |
|---|---|
| Nova UI značajka | `prototype` → implementacija → `impeccable` critique |
| Design sustav, DESIGN.md, audit, polish | `impeccable` |
| Provjera da ne izgleda generički | `taste-skill` |
| Komponente i sitni detalji | `emil-design-eng` |
| iOS (i principi gesti i springova općenito) | `apple-design` |
| Interne web stranice na mobitelu | `mobile-native` |
| Nova animacija | `animate`; za više elemenata `choreograph` |
| Svaka animacija (budget) | `animation-performance` |
| Imenovanje efekata | `animation-vocabulary` |
| Faza 6 | `find-animation-opportunities` → `improve-animations` → `review-animations` → `remove-animations` |
| Kraj platformske faze | `break-ui` |
| Odabir UI biblioteke (Faza 0) | `pick-ui-library` |
| Hero, onboarding, reveal efekti | `awwwards-animations`, **samo smjernice**; bez GSAP-a i njegovih biblioteka |

Ako neki skill nije instaliran, nastavi po DESIGN.md i zabilježi to u STATUS.md.

## Arhitektonske napomene (najčešće zamke)
- **Tauri multiwebview** traži feature `unstable`. Svi tab webviewi moraju biti na **istom WebView2 environmentu**. Za API-je koje Tauri ne izlaže koristi `with_webview(|w| w.controller())`.
- **Popoveri na desktopu** (omnibox prijedlozi, meniji, Shield panel) su zasebni transparentni top-level prozori, unaprijed kreirani. Ne pokušavaj prozirno preklapati child webviewe.
- **WebView2 popup blocker je isključen** u WebView2. Blokiranje radimo sami u `NewWindowRequested` prema `IsUserInitiated`.
- **Google prijava:** ne mijenjaj UA na Windowsu (mora biti identičan Edgeu). Na iOS-u UA mora imati Safari sufiks. Android koristi GeckoView. Nakon svake promjene enginea pokreni G1 test.
- **iOS rule liste:** najviše 150k pravila po listi. Kompajliraj jednom i keširaj, nikad na main threadu.
- **Android:** edge-to-edge i predictive back su obavezni. Ograniči broj živih `GeckoSession`a.
- **Svelte 5 runes**, TypeScript `strict`, bez UI kita (ADR-0005). Komponente su u `web/packages/ui`.
- Fontovi: OFL zamjena dok `[VLASNIK]` ne potvrdi Aeonik licencu. Subset mora pokriti č, ć, đ, š, ž.

## Definicija "gotovo" za svaki zadatak
- [ ] Kod + testovi; CI zelen
- [ ] Budgeti nisu prekršeni (mjerenje priloženo ako je vrući put)
- [ ] Light i dark provjereni (screenshotovi)
- [ ] a11y: fokus, labele, kontrast AA, hit targeti (44 pt iOS / 48 dp Android)
- [ ] i18n ključevi HR i EN
- [ ] `docs/STATUS.md` ažuriran; ADR ako je odluka

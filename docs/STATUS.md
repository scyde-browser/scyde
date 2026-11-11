# STATUS.md

> Na početku svake sesije pročitaj ovu datoteku i nastavi od prvog otvorenog zadatka trenutne faze.

**Trenutna faza:** Faza 1, Rust jezgra i Shield pipeline (PLAN §11) · **plan predložen, čeka potvrdu `[VLASNIK]`**
**Zadnje ažurirano:** 2026-11-11

## Faza 0: ZATVORENA (2026-11-11)

| Korak | Rezultat |
|---|---|
| 0.1 Repo | struktura, pinani toolchainovi, Renovate, CI skeleton, ADR-0001 · PR #1 |
| 0.2 Dizajn | DESIGN.md v1.0, DTCG tokeni → CSS/Kotlin/Swift, contrast gate 168/168, ADR-0005 · PR #1 |
| 0.3 Ikona | **Aura S** + `tools/icon-export` · PR #2 |
| 0.4 Prototipovi | New Tab, desktop chrome, mobile + critique · PR #2 · odobrenje `[VLASNIK]` otvoreno |
| 0.5 Spikeovi | svi gateovi prošli, spike kod obrisan (git `23451d6`) · PR #2 |

| Gate | Rezultat | ADR |
|---|---|---|
| G1 Google prijava | ✅ WebView2, GeckoView, WKWebView (vlasnik) | 0002 |
| G2 Windows Shield | ✅ opcija A (DNR), B rezerva; 6/6 mreža; učitavanje brže od baselinea | 0003 |
| G3 GeckoView | ✅ WASM p99 40 µs (SpiderMonkey), fixture, native messaging | 0004 |
| G4 iOS rule liste | ✅ 263.790 pravila / 2 liste, keširani load 6 ms | 0006 |
| G5 Tauri multiwebview | ✅ switch p95 ≤ 0,39 ms, popover p95 0,10 ms | 0007 |

**Preneseno u kasnije faze:**
- Snap Layouts na Windows 11 (ADR-0007) → Faza 3
- lijeno kreiranje tabova + pre-warmed tab za Ctrl+T (≈ 200 ms po webviewu) → Faza 3
- iOS kompajliranje lista samo u pozadini i isporuka unaprijed kompajliranog snapshota → Faza 4
- GeckoView 160 traži compileSdk 37.2, AGP 9.5, Kotlin 2.4 → Faza 5

## Faza 1: plan (prijedlog)
Grana `phase/1-core`, PR po logičkoj cjelini, CI zelen. Redoslijed je izabran tako da se Shield pipeline (najrizičniji dio) radi prvi.

### 1.1 `core/shield`: engine
- [ ] Omotač oko adblock-rust: `should_block`, kozmetika (`url_cosmetic_resources`, `hidden_class_id_selectors`), scriptleti
- [ ] `engine.dat`: serijalizacija i deserijalizacija s verzijom formata
- [ ] criterion benchmark `cargo bench -p shield --bench match`
- **AC:** native match p50 < 5 µs, p99 < 50 µs; deserijalizacija < 50 ms (desktop); fixture `qa/lists/fixture.txt` daje točne odluke

### 1.2 `core/shield-compiler`: jedan izvor, svi formati (PLAN §5.2)
- [ ] `engine.dat`
- [ ] DNR rulesetovi ≤ 30k pravila, s izvješćem o odbačenim pravilima (`$popup`, kozmetika, regex…)
- [ ] iOS liste ≤ 150k, s filtrom za regexe koje WebKit ne podržava (ADR-0006) i izvješćem
- [ ] kozmetički indeksi, `resources.json`
- [ ] `manifest.json` s hashevima i ed25519 potpisom
- [ ] Golden testovi: `qa/lists/fixture.txt` → očekivani DNR / iOS / kozmetički izlaz
- [ ] macOS CI job koji kompajlira iOS izlaz na simulatoru (tag/ručno)
- **AC:** DNR i iOS izlaz su unutar limita, odbačena pravila su prijavljena, potpis je provjeren u testu

### 1.3 `shield-wasm`: paket za GeckoView ekstenziju
- [ ] wasm32 build + wasm-bindgen (iz S3), veličina u budgetu, p99 test u CI-ju

### 1.4 `filters.yml`: dnevni bundle
- [ ] dohvat lista → compile → golden testovi → potpis → release `filters-YYYYMMDD` + `filters-latest`
- `[VLASNIK]`: dodati secret `FILTERS_SIGNING_KEY` (generirat ću par ključeva i uputu; javni ključ ide u `brand/keys/`)

### 1.5 `core/omnibox`
- [ ] URL fixup (`scyde`, `localhost:3000`, `192.168.1.1`, `ftp://`), odluka URL ili pretraga
- [ ] eTLD+1 prikaz, punycode za miješana pisma i konfuzabilne IDN-ove
- [ ] frecency rangiranje
- **AC:** property testovi za parsiranje; homograf `аpple.com` (ćirilično "а") se prikazuje kao punycode

### 1.6 `core/places`
- [ ] SQLite (rusqlite bundled): povijest, oznake, top sites, migracije; test sa 100k zapisa

### 1.7 `core/settings`
- [ ] JSON Schema kao izvor istine, validacija, migracije
- [ ] default tražilica DuckDuckGo `[VLASNIK]`

### 1.8 `core/theme`
- [ ] vlastiti accent: OKLCH → fill/ink, odbijanje ispod AA, najbliža prihvatljiva nijansa (DESIGN §2.2)
- [ ] isti rezultat kao `tools/contrast-check`

### 1.9 `core/ffi`
- [ ] UniFFI → Swift i Kotlin
- [ ] CI: `.so` po ABI-ju (cargo-ndk), `ScydeCore.xcframework` (macOS, tag/ručno)

### 1.10 Mjerenje
- [ ] `perf.json` artefakt iz benchmarka; regresija > 10 % ruši job (PLAN §8.2)

**AC Faze 1 (PLAN §11):** `cargo test` zelen, clippy bez upozorenja, budgeti iz 1.1, liste unutar limita, potpis provjeren, omnibox testovi prolaze.

## Skillovi
`impeccable`, `taste-skill`, `prototype` i `pick-ui-library` ne postoje u katalogu pluginova, pa su **implementirani kao project skillovi** u `.claude/skills/` (2026-11-11) i učitavaju se u svakoj sesiji u ovom repou:
- `impeccable`: quality gate prema DESIGN.md (critique / audit / polish / finalize), nalazi P1–P3, obavezni format za PR
- `taste-skill`: provjera generičnosti (lista "AI-made" znakova, ocjena 1–5 na 5 osi, prag ≥ 4)
- `prototype`: HTML prototipovi na pravim tokenima + `shot.mjs` (Playwright, light/dark × 1440/390)
- `pick-ui-library`: odluka mjerenjem buildova (ADR-0005)

Ostali skillovi iz PLAN §12 (`emil-design-eng`, `apple-design`, `mobile-native`, `animate`, `choreograph`, `animation-*`, `break-ui`, `awwwards-animations`) još nisu instalirani; rade se po istom obrascu kad ih faza zatraži.

### taste-skill: ikona Aura S
Generički znakovi: nijedan (nema 2 kruga, nema glossa ni sparkla; R1 krug 1 Aura Drop je odbačen zbog 3D kugle).
Ocjene: distinctiveness 4/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 5/5 → **pass**.
Potpis: S monogram (SCYDE), meki conic Ember gradijent, bijela/onyx squircle pločica.

### impeccable critique: ikona Aura S
Rendered: `brand/icon/preview.png` (light, dark, mono, 64/32/16 px) + Android/iOS/Windows exporti
| # | P | Nalaz | Pravilo | Popravak | Status |
|---|---|---|---|---|---|
| 1 | P2 | Vrh conic gradijenta padao je na prijelom S-a (tamna točka) | DESIGN §7, bez vidljivog vrha | centar gradijenta pomaknut izvan znaka | ✅ |
| 2 | P3 | Znak premalen u pločici (~55 %) | §7 čitljivost | stroke 108→118, radijus 116→128 | ✅ |
| 3 | P3 | Prijelom na 16 px postaje stepenica | čitljivost | prihvaćeno: S je i dalje jasan; detalj vrijedi na velikim veličinama | – |
Verdict: **ship**.

## Otvorena pitanja za vlasnika `[VLASNIK]`
Zadane vrijednosti vrijede dok vlasnik ne odluči (PLAN §17).

| # | Pitanje | Default | Status |
|---|---|---|---|
| 1 | Naziv i label na home screenu | "SCYDE Browser" | otvoreno |
| 2 | Zadana tražilica | DuckDuckGo | otvoreno |
| 3 | Open source ili zatvoreni kod (utječe na licence lista, PLAN §16) | – | otvoreno |
| 4 | Aeonik licenca ili OFL zamjena | Geist (OFL) | otvoreno |
| 5 | Windows code signing | bez potpisa (SmartScreen upozorava) | otvoreno |
| 6 | SCYDE Sync self-hosted | – | otvoreno (Faza 7) |
| 7 | Izvori za Top Stories | isključeno (opt-in) | otvoreno |
| 8 | Distribucija za Android | GitHub Releases | otvoreno |
| 9 | Koncept ikone (0.3) | – | **odlučeno 2026-11-11**: Aura S |
| 11 | Pravilo "čisti naš kod": GeckoView i adblock-rust? | – | **odlučeno 2026-11-11**: GeckoView ostaje (Android engine kao biblioteka), adblock-rust ostaje (Shield matcher). Bez forka preglednika. |
| 10 | Regionalne HR/EU filter liste | isključeno | otvoreno |

## Pravilo vlasnika (2026-11-11)
> "Ne smije biti građen kao Brave, Firefox i ostali, čisti naš custom code."

Odluka: **nema forka ni rebrandanja** Chromiuma, Bravea ni Firefoxa. Ljuska, UI, Shield logika, omnibox, povijest i teme su naš kod. Render engine je platformski (WebView2 / WKWebView), jer vlastiti engine nije realan za v1. Vlasnik je potvrdio dvije biblioteke kao dopuštene: GeckoView (Android) i adblock-rust (Shield). Pravilo je upisano i u CLAUDE.md (pravilo 11).

## Blokade
Nema.

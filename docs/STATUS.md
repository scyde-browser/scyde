# STATUS.md

> Na početku svake sesije pročitaj ovu datoteku i nastavi od prvog otvorenog zadatka trenutne faze.

**Trenutna faza:** Faza 0, Temelji, dizajn i spikeovi (PLAN §11)
**Grana:** `phase/0-foundations`
**Zadnje ažurirano:** 2026-11-11

## Faza 0: checklista

### 0.1 Repo
- [x] Struktura iz PLAN §4 (`core/`, `web/`, `shield-extension/`, `desktop/`, `android/`, `ios/`, `qa/`, `spikes/`, `tools/`, `docs/`, `brand/`, `design/`)
- [x] Pinani toolchainovi: `rust-toolchain.toml` (1.97.0), `.nvmrc` (22), `packageManager` pnpm 10.28.0
- [ ] Gradle version catalog: dolazi sa spikeom S3 / Fazom 5 (tamo prvi put postoji Gradle projekt)
- [x] Renovate (`renovate.json`): GeckoView svaka 4 tjedna, Tauri ručno, sigurnosni bumpovi odmah
- [x] Skeleton CI workflowi iz PLAN §13 (`core`, `web`, `filters`, `windows`, `android`, `ios`)
- [x] ADR predložak (`docs/adr/0000-template.md`) i ADR-0001 (strategija enginea)
- [x] `docs/NETWORK.md`
- [x] PR predložak (screenshotovi + `impeccable` critique)

### 0.2 Dizajn temelj
- [x] DESIGN.md finaliziran (v1.0): izračunati kontrasti, token mapa, ispravci (vidi changelog u DESIGN.md)
- [x] `design/tokens/*.json` (DTCG) → Style Dictionary 5 → CSS, Kotlin (Compose), Swift (SwiftUI), JSON
- [x] `tools/springs`: response/damping → stiffness, CSS `linear()` + trajanje
- [x] `tools/contrast-check`: 168 parova, svi ≥ AA (najtijesniji 4,65:1)
- [x] ADR-0005: Svelte 5 + vlastite komponente (izmjereno 14,2 KB gz)
- [x] Smjer provjeren protiv generičkog izgleda (ručno, vidi "Skillovi" dolje)
- [ ] Prva kompilacija generiranog `ScydeTokens.kt` i `ScydeTokens.swift` u stvarnim projektima (S3/S4 ili Faza 4/5)

### 0.3 Brand i ikona
- [x] Tri originalna koncepta ikone (SVG): **Orbit**, **Twin Arc**, **Aura Drop** (`brand/icon/concepts/`), bez dva preklopljena kruga
- [x] `tools/icon-export`: Windows `.ico` 16–256, Android adaptive (foreground, background, monochrome) + legacy, iOS 1024 + dark + tinted, pregled `preview.png`
- [x] Ikona odabrana: **Aura S** (`brand/icon/scyde.svg`, `brand.json → icon`). Vlasnik je nakon dva kruga (6 koncepata) prepustio izbor; znak je doveden do kraja kroz `taste-skill` + `impeccable` critique (vidi dolje).

### 0.4 Prototipovi
- [x] New Tab light/dark (`design/prototypes/newtab/`)
- [x] Desktop chrome: tabovi, omnibox focus s prijedlozima, Shield panel (`design/prototypes/desktop-chrome/`)
- [x] Mobilni donji toolbar + tab switcher (`design/prototypes/mobile/`)
- [x] `taste-skill` + `impeccable` critique svakog prototipa; svi P1/P2 popravljeni (vidi `notes.md` u svakoj mapi)
- [ ] `[VLASNIK]` odobrava prototipove

### 0.5 Spikeovi
CI: `.github/workflows/spikes.yml` (pokreće se na PR-u ili ručno). Fixturei: `qa/` (`node qa/serve.mjs 8787`).
- [ ] S1 Google prijava na WebView2, GeckoView i WKWebView → **G1** (`[VLASNIK]`, ručni test: koraci ispod)
- [ ] S2 WebView2 Shield → **G2**: baseline i opcija B automatizirani (`s5-windows`); opcije A (DNR ekstenzija) i C (CDP) još nisu napravljene
- [ ] S3 GeckoView → **G3**: WASM dio ✅ (CI p99 14,8 µs, fixture 8/8); aplikacija + emulator test u izradi (`s3-geckoview-app`)
- [ ] S4 iOS rule liste ≥ 150k pravila → **G4**: konverter ✅ (132k stvarnih / 264k stres), XCTest na simulatoru u CI-ju (`s4-ios-rulelists`)
- [ ] S5 Tauri multiwebview → **G5**: 20 tabova, prebacivanje, popover, Mica, titlebar u CI-ju (`s5-windows`); Snap Layouts ručno

#### S1: koraci za vlasnika (Google prijava) `[VLASNIK]`
Cilj G1: prijava radi na sva tri enginea (screenshotovi idu u ADR-0002). Koristi **testni Google račun** s uključenom 2FA, ako ga imaš.

**Windows (WebView2):**
1. Na PR-u otvori zadnji uspješni run workflowa `spikes` i preuzmi artefakt `s1-windows-exe`.
2. Pokreni `scyde-spike-s5.exe` (bez argumenata). SmartScreen će upozoriti jer exe nije potpisan: "More info" → "Run anyway".
3. Prvi tab je `accounts.google.com`: prijavi se (lozinka + 2FA).
4. Klikni tabove Gmail, YouTube i Drive: moraš biti prijavljen bez ponovne prijave.
5. Zadnji tab (Stack Overflow login): odaberi "Log in with Google" i dovrši prijavu.
6. Za svaki korak zapiši: prošlo / poruka "This browser or app may not be secure" / drugo. Napravi screenshot (Win+Shift+S).

**Android (GeckoView):**
1. Preuzmi artefakt `s3-geckoview-apk` i instaliraj `app-debug.apk` na telefon (dopusti instalaciju iz nepoznatih izvora).
2. Otvori aplikaciju "SCYDE S3". Prikazuje internu stranicu s popisom linkova (ujedno dokazuje da interna stranica razgovara s aplikacijom).
3. Redom: accounts.google.com (prijava), Gmail, YouTube, Drive, Stack Overflow → "Log in with Google". Tipka Natrag vraća na popis.

**iOS (WKWebView):** koraci dolaze s malom S1 iOS aplikacijom (Safari UA sufiks) u sljedećem commitu.

Rezultate javi u chatu ili ih upiši ovdje, a ja ću napisati ADR-0002.

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
| 9 | Koncept ikone (0.3) | – | **odlučeno 2026-11-11**: Aura S (vlasnik: "generiraj ikonu") |
| 11 | Pravilo "čisti naš kod": GeckoView i adblock-rust? | – | **odlučeno 2026-11-11**: GeckoView ostaje (Android engine kao biblioteka), adblock-rust ostaje (Shield matcher). Bez forka preglednika. |
| 10 | Regionalne HR/EU filter liste | isključeno | otvoreno |

## Pravilo vlasnika (2026-11-11)
> "Ne smije biti građen kao Brave, Firefox i ostali, čisti naš custom code."

Odluka: **nema forka ni rebrandanja** Chromiuma, Bravea ni Firefoxa. Ljuska, UI, Shield logika, omnibox, povijest i teme su naš kod. Render engine je platformski (WebView2 / WKWebView), jer vlastiti engine nije realan za v1. Vlasnik je potvrdio dvije biblioteke kao dopuštene: GeckoView (Android) i adblock-rust (Shield). Pravilo je upisano i u CLAUDE.md (pravilo 11).

## Blokade
Nema.

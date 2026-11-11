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
- [ ] `[VLASNIK]` bira koncept (čeka odgovor)

### 0.4 Prototipovi
- [ ] New Tab light/dark
- [ ] Desktop chrome: tabovi, omnibox focus s prijedlozima, Shield panel
- [ ] Mobilni donji toolbar + tab switcher
- [ ] Critique svakog prototipa i popravci
- [ ] `[VLASNIK]` odobrava prototipove

### 0.5 Spikeovi
- [ ] S1 Google prijava na WebView2, GeckoView i WKWebView → **G1**
- [ ] S2 WebView2 Shield opcije A/B/C → **G2**
- [ ] S3 GeckoView ekstenzija + WASM + native messaging → **G3**
- [ ] S4 iOS rule liste ≥ 150k pravila → **G4**
- [ ] S5 Tauri multiwebview, 20 tabova, popoveri, Mica, snap → **G5**

## Skillovi
Skillovi `impeccable`, `taste-skill`, `pick-ui-library` i `prototype` **nisu instalirani** u ovom okruženju. Prema CLAUDE.md rad se nastavlja po DESIGN.md:
- DESIGN.md je finaliziran ručno: svaka tvrdnja o kontrastu je provjerena alatom, ne procjenom.
- Provjera generičnosti: ručno prema listi "Ne" iz DESIGN §11.
- Odabir UI biblioteke: ADR-0005 s mjerenjem bundlea.

Kad se skillovi instaliraju, pokreni `impeccable` critique nad DESIGN.md i prototipovima i upiši nalaze ovdje.

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
| 9 | Koncept ikone (0.3) | – | **čeka izbor**: Orbit / Twin Arc / Aura Drop |
| 11 | "Čisti naš custom code, ne građen kao Brave/Firefox" (vlasnik, 2026-11-11): vrijedi li i za **GeckoView** (Mozillin engine kao biblioteka na Androidu) i **adblock-rust** (Braveova biblioteka za Shield)? | nema forka nijednog preglednika (PLAN već tako postavlja Windows i iOS) | **otvoreno**: utječe na ADR-0001, S3 i Fazu 1 |
| 10 | Regionalne HR/EU filter liste | isključeno | otvoreno |

## Pravilo vlasnika (2026-11-11)
> "Ne smije biti građen kao Brave, Firefox i ostali, čisti naš custom code."

Tumačenje do odluke: **nema forka ni rebrandanja** Chromiuma, Bravea ni Firefoxa. Ljuska, UI, Shield logika, omnibox, povijest i teme su naš kod. Render engine je platformski (WebView2 / WKWebView), jer vlastiti engine nije realan za v1. Dvije posuđene komponente iz plana čekaju potvrdu (pitanje 11).

## Blokade
Nema.

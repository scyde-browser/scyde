# SCYDE Browser: master plan za Claude Code

> **Radni naziv:** SCYDE Browser · **repo/codename:** `scyde-browser` · **app id:** `dev.scyde.browser`
> Naziv, ikona i boje žive u `brand/brand.json` + `design/tokens/`, pa je rebrand jedna izmjena.
> **Platforme:** Windows 10/11 (x64, arm64 kasnije) · Android 8+ · iOS 16+ (.ipa za Feather / SideStore / TrollStore)
> **Status dokumenta:** v1, temelj za Fazu 0. Odluke koje Faza 0 potvrdi ili promijeni bilježe se kao ADR u `docs/adr/`.

---

## 0. Kako Claude Code koristi ovaj dokument

1. Redoslijed čitanja: **PLAN.md → CLAUDE.md → DESIGN.md → `design/references/`**.
2. Radi se strogo po fazama (§11). Faza se ne zatvara dok nisu ispunjeni svi njezini **kriteriji prihvaćanja (AC)**.
3. **Faza 0 ima gate-ove (G1–G5).** Ako neki gate padne, ne nastavljaj sa sljedećom fazom. Upiši nalaz u ADR, predloži alternativu iz §2.3 i označi odluku s `[VLASNIK]`.
4. Svaka arhitektonska odluka koja odstupa od ovog plana dobiva ADR (`docs/adr/NNNN-naslov.md`, predložak u §13.4).
5. Stanje rada uvijek je u `docs/STATUS.md`: što je gotovo, što je u tijeku, blokade i otvorena pitanja za vlasnika.
6. Windows i iOS artefakti grade se u **GitHub Actions** (Claude Code obično radi na Linuxu). Iteriraj kroz `gh run watch` i `gh run view --log-failed`. Za UI promjene CI uvijek prilaže screenshotove (§13.3).

---

## 1. Vizija i ne-pregovarajući zahtjevi

**Vizija:** brz, tih, premium preglednik koji štiti po defaultu. Sadržaj stranice je u prvom planu, a sučelje se osjeća kao jedan proizvod na sve tri platforme, ne kao tri aplikacije.

| # | Zahtjev | Što konkretno znači |
|---|---|---|
| R1 | **Unikatan** | Vlastiti vizualni jezik "Aura" (DESIGN.md), vlastita ikona, vlastiti Shield sustav i vlastite značajke (§11, Faza 6). Ne smije izgledati kao reskin Chromea ili Bravea. |
| R2 | **Nula reklama + ad/popup blocker** | (a) Shield blokira reklame, trackere i popupe na svim platformama, uključen po defaultu. (b) **Sam preglednik nikad ne prikazuje reklame:** nema sponzoriranih tileova, sponzoriranih pozadina, affiliate preusmjeravanja ni crypto/rewards programa. |
| R3 | **Dark/Light + teme** | Light, Dark i System. Uz to Aura accent (5 predložaka + vlastita boja s provjerom kontrasta), pozadine New Taba, gustoća sučelja, smanjeno kretanje i opcionalni adaptivni tint prema `theme-color` stranice. |
| R4 | **Premium izgled** | Prema DESIGN.md. Svaki UI PR prolazi `impeccable` critique. Animacije su fizikalne, prekidive i unutar frame budgeta. |
| R5 | **Jako optimiziran** | Tvrdi performance budgeti (§8) mjere se u CI-ju. Regresija budgeta blokira merge. |
| R6 | **Privatnost** | Bez telemetrije po defaultu. Crash izvješća su opt-in. Mrežni pozivi samog preglednika: provjera ažuriranja, filter bundle i (opt-in) prijedlozi pretrage i vijesti. Svaki takav poziv je dokumentiran u `docs/NETWORK.md`. |

**Izvan opsega v1:** crypto novčanik, BAT-style rewards (ikone novčanika i poklona iz reference se **ne** implementiraju), vlastiti tražilični indeks, sinkronizacija (dolazi u Fazi 7), AI chat (kasnije, opt-in).

---

## 2. Ključne arhitektonske odluke

### 2.1 Strategija enginea

Brave je fork Chromiuma. Za samostalnog developera s Claude Codeom to je pogrešan početak: build traži velike strojeve (stotine GB, sati kompajliranja), GitHub hosted runneri to ne podnose, a sigurnosne zakrpe Chromiuma izlaze svakih nekoliko tjedana i fork bi ih morao pratiti ručno. Zato v1 gradi **vlastitu ljusku (shell) oko platformskih enginea koji se sami sigurnosno ažuriraju**, a sav "mozak" (Shield, omnibox, povijest, teme) drži u zajedničkoj Rust jezgri.

| Platforma | Engine | Shell / UI | Zašto | Sigurnosna ažuriranja |
|---|---|---|---|---|
| **Windows** | **WebView2 (Evergreen)**, Chromium | **Tauri 2** (Rust) + chrome UI u web tehnologiji (Svelte 5) | Mali installer, dijeljeni runtime, Rust jezgra se linka direktno, puna kontrola dizajna kroz web UI | Automatski (Microsoft Evergreen runtime) |
| **Android** | **GeckoView** (Mozilla, MPL-2.0) | Kotlin + Jetpack Compose | Pravi browser engine: WebExtensions s blokirajućim `webRequest`, Enhanced Tracking Protection, Total Cookie Protection. Ne pogađa ga Googleova zabrana prijave u Android WebViewu (§2.2). | Bump verzije u Gradleu (Renovate PR svaka 4 tjedna + hitni) |
| **iOS** | **WKWebView** (WebKit, jedini dopušteni izvan EU-a) | Swift: SwiftUI + UIKit za browser view | Jedina realna opcija. Svi iOS preglednici (Chrome, Brave, Firefox) rade na WebKitu. | Automatski sa sustavom |

**Zajednička Rust jezgra (`core/`)** (isti pristup kao Mozillin *application-services*):
- `shield`: omotač oko **adblock-rust** (Braveov engine, MPL-2.0). Podržava EasyList/uBO sintaksu, kozmetičko filtriranje, scriptlete, WASM build i konverziju u Appleov content-blocking format.
- `shield-compiler`: CLI koji u CI-ju kompajlira filter liste u formate po platformi (§5).
- `omnibox`: URL fixup, URL ili pretraga, rangiranje prijedloga (frecency), sigurnosni prikaz URL-a.
- `places`: povijest, oznake, top sites (SQLite, `rusqlite` bundled).
- `settings`: shema postavki, validacija, migracije (JSON Schema kao izvor istine).
- `theme`: razrješavanje tokena, generiranje accent skale iz korisničke boje, provjera kontrasta.
- `ffi`: UniFFI bindings (Swift, Kotlin). Windows host koristi crateove direktno.

### 2.2 Rizik br. 1: prijava u Google račun u embedded engineima

Google blokira OAuth prijavu u "embedded webviewima". Android WebView i WKWebView su eksplicitno navedeni kao neusklađeni, a postoje i prijave blokade na desktop WebView2. Preglednik u kojem se ne možeš prijaviti u Gmail ili YouTube je neupotrebljiv. Zato:
- **Android → GeckoView**, ne Android WebView. Gecko je engine Firefoxa, nije Android WebView.
- **iOS → WKWebView s potpunim Safari-like User-Agentom** (`applicationNameForUserAgent = "Version/<x> Mobile/15E148 Safari/604.1"`), kao što rade svi iOS preglednici.
- **Windows → WebView2 je uvjetan.** Gate **G1** u Fazi 0 mora dokazati da prijava radi (accounts.google.com → Gmail, YouTube, Drive, i "Sign in with Google" na trećoj stranici). UA mora biti identičan stabilnom Edgeu. Nema `navigator.webdriver` ni automation zastavica.

### 2.3 Fallback ako G1 padne na Windowsu

1. **Gecko desktop fork** (pristup Zen / Floorp / LibreWolf). Isti engine kao na Androidu, chrome UI u HTML/CSS/JS, uBO-razina blokiranja. Cijena: rebase na Firefox svaka 4 tjedna i full build na self-hosted runneru (Proxmox homelab / vlastiti server). Za razvoj UI-ja koriste se "artifact buildovi" (samo frontend).
2. **CEF**: puni Chromium u paketu, ali Google ga u istoj politici navodi kao problematičnog, pa nije pravo rješenje.
3. **Chromium fork (Brave-put)**: samo ako projekt dobije build infrastrukturu i vrijeme za praćenje zakrpa. Nije za v1.

Odluka ide u ADR-0002 s rezultatima spikea.

---

## 3. Arhitektura

```
┌──────────────────────────── core/  (Rust: jedna istina) ────────────────────────────┐
│  shield (adblock-rust) · omnibox · places (SQLite) · settings · theme · ffi (UniFFI)  │
│  shield-compiler (CLI, radi u CI-ju)  ──►  potpisani filter bundle (§5.3)             │
└──────┬───────────────────────────┬──────────────────────────────┬────────────────────┘
       │ native (crate)            │ WASM                          │ UniFFI
       ▼                           ▼                               ▼
 WINDOWS                     shield-extension/              ANDROID: Kotlin + Compose
 Tauri 2 host (Rust)         · target "chromium-mv3"        · libscyde_core.so (places, omnibox, settings, theme)
  ├ chrome webview (UI)        (DNR pravila → WebView2)     · GeckoView + built-in shield ekstenzija (MV2, WASM engine)
  ├ tab webviews (WebView2)  · target "gecko-mv2"           IOS: Swift + SwiftUI/UIKit
  └ popover prozori            (webRequest → GeckoView)     · ScydeCore.xcframework (uklj. shield native)
                             · content scripts (kozmetika,  · WKContentRuleList + WKUserScript (izolirani world)
                               scriptleti, brojači)
       ▲                           ▲                               ▲
       └────────────── web/ (Svelte 5 + TS + Vite): dijeljeni dizajn sustav i interne stranice ─────┘
             newtab · settings (desktop) · shields panel · history/bookmarks · downloads · desktop chrome UI
```

**Interne stranice** imaju shemu `scyde://` (`newtab`, `settings`, `shields`, `history`, `bookmarks`, `downloads`, `about`). Po platformi:
- Windows: Tauri custom protocol / WebView2 virtual host. Omnibox prikazuje `scyde://…`.
- Android: stranice žive unutar built-in ekstenzije (`moz-extension://…`) i s aplikacijom komuniciraju preko native messaginga built-in ekstenzije. Omnibox prikazuje `scyde://…`.
- iOS: `WKURLSchemeHandler` za `scyde://`.

---

## 4. Monorepo

```
scyde-browser/
├─ PLAN.md · CLAUDE.md · DESIGN.md · README.md · LICENSE · NOTICE.md
├─ brand/                    brand.json (ime, app id, boje brenda), ikona (SVG master + exporti)
├─ design/
│  ├─ references/            referentne slike (samo inspiracija, vidi §16)
│  ├─ prototypes/            brzi HTML prototipovi (skill: prototype)
│  └─ tokens/                DTCG JSON tokeni → Style Dictionary → CSS / Kotlin / Swift
├─ core/                     Cargo workspace
│  ├─ shield/ · shield-compiler/ · omnibox/ · places/ · settings/ · theme/ · ffi/
│  └─ benches/               criterion benchmarki (shield match latencija itd.)
├─ web/                      pnpm workspace
│  ├─ packages/tokens/ · packages/ui/ · packages/bridge/   (tipizirani IPC klijent)
│  └─ apps/newtab/ · apps/settings/ · apps/shields/ · apps/library/ (history+bookmarks+downloads) · apps/desktop-chrome/
├─ shield-extension/         zajednički kod; build targeti chromium-mv3 i gecko-mv2
├─ desktop/                  Tauri 2 app (src-tauri/ + veza na web/apps/desktop-chrome)
├─ android/                  Gradle projekt (app/, core-bindings/, baselineprofile/, benchmark/)
├─ ios/                      project.yml (XcodeGen), App/, BrowserKit/, Tests/, UITests/
├─ qa/                       adblock i popup fixture stranice, break-ui skripte, popisi stranica za ručni QA
├─ spikes/                   Faza 0: jednokratni prototipovi enginea (brišu se nakon ADR-a)
├─ tools/                    ikona export, contrast check, perf skripte, release skripte
├─ docs/                     STATUS.md · NETWORK.md · adr/ · architecture/ · release.md
└─ .github/workflows/        vidi §13
```

**Alati:** Rust stable (rustup, `cargo-ndk`, `uniffi-bindgen`, `wasm-pack` ili `wasm-bindgen-cli`), Node LTS + pnpm, JDK 17+, Android SDK/NDK, Xcode zadnji stabilni (na CI-ju), XcodeGen. Verzije se pinaju u Fazi 0 (rust-toolchain.toml, .nvmrc, gradle version catalog).

---

## 5. Shield: ad, tracker i popup zaštita

### 5.1 Liste (default uključene)
EasyList, EasyPrivacy, uBlock filters (+ privacy, + unbreak), Peter Lowe's list. Opcionalno: cookie-banner/annoyance liste ("Zen" prekidač), regionalne HR/EU liste (`[VLASNIK]` potvrđuje izbor). Korisnik može dodati vlastitu listu (URL) i vlastita pravila.

### 5.2 Izlazi `shield-compiler`a (jedan izvor → svi formati)

| Izlaz | Koristi ga | Napomena |
|---|---|---|
| `engine.dat` (serijalizirani adblock-rust engine) | iOS native, WASM u gecko ekstenziji, Windows host (kozmetika) | Brzi start: deserijalizacija umjesto parsiranja lista |
| `dnr/*.json` (Chromium declarativeNetRequest static rulesets) | Windows (opcija A, §5.4) | Chromium jamči 30.000 statičkih pravila po ekstenziji plus globalni pool. Najviše 50 uključenih rulesetova od 100 deklariranih. Kompajler dijeli liste i prijavljuje ono što ne stane. |
| `ios/*.json` (WKContentRuleList) | iOS | Najviše 150.000 pravila po listi. Kompajler dijeli u više lista. |
| `cosmetic/` (generički + per-host indeksi) | sve platforme | Za brzi lookup u content skriptama |
| `resources.json` (scriptleti/redirect resursi, uBO-kompatibilni) | sve | Licenca: GPLv3, vidi §16 |
| `manifest.json` (verzija, hashevi, potpis) | updater | ed25519 / minisign |

### 5.3 Ažuriranje lista
- `filters.yml` (GitHub Actions, dnevno): preuzmi liste → `shield-compiler` → testovi (golden fixtures) → potpiši → objavi kao release `filters-YYYYMMDD` i kao `filters-latest` (GitHub Releases ili vlastiti CDN/server).
- Klijent provjerava jednom dnevno (ETag/If-None-Match), provjerava potpis, priprema novi engine **izvan main threada** i radi atomski swap. Ako bilo što pukne, ostaje stari bundle.
- Installer, APK i IPA nose snapshot bundlea, pa Shield radi i offline od prve sekunde.

### 5.4 Integracija po platformi

**Windows (WebView2).** Konačni izbor radi spike S2 prema mjerljivim kriterijima:
- **A, preferirano:** vlastita **MV3 ekstenzija** (učitana kroz `browser_extensions_enabled` + `extensions_path` u Tauriju, odnosno `AddBrowserExtension`) s **DNR statičkim rulesetovima**. Blokiranje se odvija unutar mrežnog sloja Chromiuma, bez IPC-a prema hostu. Per-site iznimke idu u dinamička pravila.
- **B:** `WebResourceRequested` filter i Rust engine u hostu. Jednostavno, ali svaki zahtjev ide na UI thread hosta (dodatni process hop), pa je moguća latencija.
- **C:** CDP `Network.setBlockedURLs` kroz `CallDevToolsProtocolMethod`, s brojačima iz `Network.loadingFailed`. Samo kao mjerenje.
- **Kozmetika i scriptleti (v1):** host ubacuje bootstrap skriptu (`AddScriptToExecuteOnDocumentCreated`) koja traži kozmetiku od hosta preko WebMessagea (`url_cosmetic_resources`, `hidden_class_id_selectors` u batchu iz MutationObservera). CSS se primjenjuje kroz `adoptedStyleSheets`. Ako spike pokaže da je isolated world ekstenzije stabilniji, kozmetika se seli u ekstenziju (ADR).
- **Glavni okvir (popunder/redirect):** `NavigationStarting` provjerava engine. Blokirana domena dobiva internu interstitial stranicu "Shield je zaustavio ovu stranicu" (Nastavi jednom / Uvijek dopusti).

**Android (GeckoView).**
- Built-in ekstenzija (`WebExtensionController.ensureBuiltIn`, nije potreban potpis) s **MV2 `webRequest` + `webRequestBlocking`**, a engine radi kao **WASM** u background skripti.
- Content scripti rade kozmetiku i scriptlete. Brojače šalju aplikaciji preko native messaginga (`setMessageDelegate`).
- Uz to uključi GeckoView **ETP Strict** i **Total Cookie Protection** (ContentBlocking postavke).

**iOS (WKWebView).**
- Mreža i generička kozmetika: **WKContentRuleList** (kompajlira se jednom, kešira u `WKContentRuleListStore`, rekompajlira se u pozadini nakon updatea).
- Specifična kozmetika: `WKUserScript` u vlastitom `WKContentWorld` (izolirano od stranice). Upit ide prema native engineu (UniFFI) preko `WKScriptMessageHandlerWithReply`.
- Scriptleti: page world, `atDocumentStart`.
- **Brojač je procjena:** WKContentRuleList ne prijavljuje blokirane zahtjeve. Content script skuplja URL-ove resursa iz DOM-a i pita engine, a UI prikazuje "≈".

**Zajedničko:**
- Per-site Shield panel: on/off za stranicu, razina (Standard / Agresivno), blokiranje cookie bannera, broj blokiranih, ušteda podataka i vremena (procjena).
- Globalna statistika na New Tabu.
- **Očekivanja:** YouTube i slični servisi su igra mačke i miša. Blokiranje je best-effort i ovisi o svježim scriptletima, pa dnevni bundle nije opcionalan.

### 5.5 Popup blocker

| Platforma | Mehanizam |
|---|---|
| Windows | WebView2 isključuje Edgeov popup blocker, pa ga radimo sami: `NewWindowRequested`. Ako `IsUserInitiated == false`, postavi `Handled = true` i prikaži "Popup blokiran" pill u omniboxu (Dopusti jednom / Uvijek za ovu stranicu). Popup koji je pokrenuo korisnik otvara se kao novi tab u našem UI-ju (`NewWindow` = naš webview, s Deferralom). |
| Android | Gecko interni popup blocker + `PromptDelegate.onPopupPrompt` (naš pill i odluka) + `NavigationDelegate.onNewSession` (novi tab). |
| iOS | `WKPreferences.javaScriptCanOpenWindowsAutomatically = false`. `WKUIDelegate.createWebViewWith` otvara novi tab samo za geste korisnika. |
| Sve | Navigacije glavnog okvira prema blokiranim domenama (popunder, redirect lanci) idu na interstitial. Obavijesti i automatsko puštanje zvuka su po defaultu "tiho" ili isključeni. |

---

## 6. Teme i dizajn sustav (sažetak, detalji u DESIGN.md)

- **Izvor istine:** `design/tokens/*.json` (DTCG format) → Style Dictionary → `web/packages/tokens` (CSS custom properties), `android/…/theme/Tokens.kt`, `ios/BrowserKit/Theme/Tokens.swift`.
- **Opcije korisnika:**
  - mod (Light / Dark / System)
  - Aura accent (Sky, Periwinkle, Lavender, Mint, Ember ili vlastita boja; `core/theme` generira skalu i "ink" varijante te odbija kombinacije ispod WCAG AA)
  - pozadina New Taba (Aura gradijenti, puna boja ili vlastita slika; slika se lokalno skalira i kešira)
  - gustoća (Comfortable / Compact)
  - veličina teksta
  - smanjeno kretanje (prati sustav, ručno se može nadjačati)
  - adaptivni tint (opcija)
- **Primjena:** desktop chrome UI i interne stranice čitaju CSS varijable. Promjena teme je cross-fade od 200 ms bez reflowa. Native UI (Compose, SwiftUI) čita generirane tokene.

---

## 7. Sigurnost (preglednik je sigurnosni proizvod)

- **Omnibox:** naglasi registrabilnu domenu (eTLD+1). Miješana pisma i konfuzabilne IDN domene prikazuju se kao punycode. Duge poddomene se skraćuju od početka, nikad tako da sakriju domenu. Lokot i stanje veze dolaze isključivo iz enginea.
- **Interne stranice:** stroga CSP, bez udaljenih skripti. Native bridge je dostupan **samo** `scyde://` originu (provjera origina na svakoj poruci, allowlista metoda, tipizirane poruke u `web/packages/bridge`). Web sadržaj nikad ne dobiva host objekte.
- **Dozvole** (kamera, mikrofon, lokacija, obavijesti, clipboard): native prompt s pamćenjem po stranici i pregledom u postavkama.
- **Privatni tabovi/prozori:**
  - Windows: WebView2 InPrivate profil (`incognito`)
  - Android: GeckoSession s `usePrivateMode`
  - iOS: `WKWebsiteDataStore.nonPersistent()`
- **Ažuriranja:** Tauri updater s potpisom. Filter bundle s potpisom. GeckoView bump preko Renovatea, a sigurnosni bump se mergea u roku od 48 h.
- Safe Browsing / SmartScreen: politika privatnost vs. zaštita ide u ADR `[VLASNIK]`.

---

## 8. Performanse

### 8.1 Budgeti (ciljevi; Faza 0 mjeri baseline i ADR ih može korigirati)

| Metrika | Windows (laptop srednje klase, SSD) | Android (Pixel 6a klasa) | iOS (iPhone 12 klasa) |
|---|---|---|---|
| Hladni start do interaktivnog chromea | ≤ 600 ms (cilj 400) | ≤ 900 ms | ≤ 500 ms |
| Prikaz New Taba nakon Ctrl+T / + | ≤ 120 ms | ≤ 200 ms | ≤ 150 ms |
| Prebacivanje taba (živ tab) | ≤ 1 frame (16 ms) | ≤ 100 ms | ≤ 100 ms |
| Animacije | 60 fps, 0 izgubljenih frameova u tab open/close i omnibox focus | 60/90/120 fps prema zaslonu | 60/120 fps (ProMotion) |
| Shield odluka po zahtjevu | native p99 < 50 µs | WASM p99 < 200 µs | rule list (WebKit) |
| Veličina paketa | installer ≤ 15 MB (Evergreen runtime) | GeckoView baseline + ≤ 10 MB našeg koda, po ABI-ju | ≤ 30 MB |
| JS bundle desktop chromea | ≤ 80 KB gz | – | – |
| JS bundle New Taba | ≤ 120 KB gz (bez slika) | isto | isto |
| Memorija ljuske (bez renderera) | ≤ 150 MB | mjeri se u Fazi 0 | mjeri se u Fazi 0 |

### 8.2 Tehnike
- **Windows:**
  - jedan dijeljeni `CoreWebView2Environment` za sve tabove
  - prozor odmah pokazuje boju teme i native skeleton (bez bijelog bljeska), a chrome webview se pojavljuje kad je spreman
  - lijeno vraćanje tabova (tab se kreira tek kad postane vidljiv)
  - **sleeping tabs**: pozadinski tab dobiva `MemoryUsageTargetLevel = Low`, a nakon N minuta neaktivnosti `TrySuspendAsync` (UI prikazuje "spava · ušteđeno X MB")
  - popover prozori su unaprijed kreirani i skriveni
  - nema WebView2 za splash
- **Android:**
  - `GeckoRuntime` kao singleton s ranom asinkronom inicijalizacijom
  - najviše N živih `GeckoSession`a (default 5), ostali spremaju `SessionState` i vraćaju se na dodir
  - Baseline Profiles + Macrobenchmark, R8 full mode, ABI splits, Compose stabilnost (bez nepotrebnih rekompozicija; provjera kroz Layout Inspector i compiler metrics)
- **iOS:**
  - ograničen broj živih `WKWebView`a; ostali tabovi su snapshot + `interactionState` (iOS 15+)
  - content rule liste se kompajliraju jednom i keširaju
  - nema teškog posla na main threadu (Swift 6 concurrency)
- **Web UI:**
  - Svelte 5 (kompajlirano, mali runtime)
  - animiraju se samo `transform` i `opacity`; `content-visibility` za liste; virtualizacija povijesti
  - fontovi kao subsetani variable WOFF2 (Latin + Latin Ext za č, ć, đ, š, ž)
  - slike u AVIF/WebP, lijeno učitavanje
- **Mjerenje u CI-ju:** criterion benchmarki (`core/benches`), Lighthouse-CI/perf skripta za interne stranice, Macrobenchmark (Android, na emulatoru samo trend), XCTest `measure` (iOS), startup skripta na Windows runneru (trend). Rezultati se spremaju kao artefakt `perf.json`. Regresija veća od 10 % ruši job.

---

## 9. Platforme u detalje

### 9.1 Windows (Tauri 2 + WebView2)

**Prozor:**
- `decorations: false`, Mica/Acrylic pozadina (Tauri window effects), vlastiti titlebar s tabovima
- Windows 11 **Snap Layouts** na maximize gumbu: `WM_NCHITTEST → HTMAXBUTTON` kroz subclass prozora ili evaluacija `tauri-plugin-decorum`
- caption gumbi u Windows stilu (ne macOS semafor iz reference)

**Slojevi:**
1. `chrome` webview: tab strip, toolbar i lijevi rail za web panele
2. `tab-*` webviewi (Tauri multiwebview, feature `unstable`, svi na **istom environmentu**), vidljiv je samo aktivni
3. **popover prozori**: transparentni borderless prozori bez taskbar ikone, u vlasništvu glavnog prozora, za omnibox prijedloge, menije, Shield panel, command palette i toastove. Child webviewi se ne mogu prozirno slagati jedan preko drugoga, pa popoveri žive u zasebnim top-level prozorima.

**Escape hatch:** `Webview::with_webview(|w| w.controller())` daje sirovi `ICoreWebView2Controller` za API-je koje Tauri ne izlaže (`NewWindowRequested` s Deferralom, `MemoryUsageTargetLevel`, `TrySuspend`, `AddBrowserExtension`, download eventi).

**Značajke v1:**
- tabovi (pin, mute, drag reorder, drag-out u novi prozor kasnije), omnibox s prijedlozima, back/forward/reload
- find-in-page, zoom, preuzimanja (vlastiti panel), print
- povijest, oznake + traka oznaka, uvoz iz Chromea, Edgea i Firefoxa (Faza 7)
- Shield, popup blocker, New Tab, postavke, teme
- privatni prozor, "Postavi kao zadani preglednik" (registry `RegisteredApplications`, `StartMenuInternet`, URL i datotečne asocijacije, zatim otvori `ms-settings:defaultapps`)
- prečaci kao u Chromeu (Ctrl+T/W/L/Tab, Ctrl+Shift+T, Ctrl+K command palette)

**Paket:**
- NSIS per-user installer (bez admina) + MSI za IT
- WebView2 bootstrapper ako runtime nedostaje
- Tauri updater (GitHub Releases, potpis)
- code signing (Azure Trusted Signing ili OV cert) `[VLASNIK]`, inače SmartScreen upozorava

### 9.2 Android (Kotlin + Compose + GeckoView)

- **Arhitektura:** single-activity, `BrowserStore` (StateFlow) za tabove, aktivni tab, nav stanje i Shield brojače. `EngineSession` omot oko `GeckoSession` (sučelje dopušta zamjenu enginea). `TabManager` s limitom živih sesija. Core preko UniFFI (`cargo-ndk`, arm64-v8a + armeabi-v7a + x86_64 za emulator). Postavke u DataStoreu (shema iz `core/settings`).
- **UI:**
  - **donji toolbar** s omniboxom (zona palca); swipe po toolbaru mijenja tab; swipe gore otvara tab switcher (grid sa shared-element tranzicijom)
  - Shield bottom sheet; kontekstni meni za linkove (long-press, peek preview u Fazi 6)
  - **edge-to-edge** (obavezno za targetSdk 35+), **predictive back** (povijest webviewa → zatvaranje sheeta → izlaz)
  - Material 3 je samo tehnička baza; vizualno je sve iz DESIGN.md (bez generičkog Material izgleda)
- **Interne stranice:** web build se pakira u built-in ekstenziju (assets). New Tab je `moz-extension` stranica prikazana kao `scyde://newtab`.
- **Paket:** potpisani APK po ABI-ju (GitHub Releases, Obtainium-kompatibilno), kasnije AAB za Play i F-Droid `[VLASNIK]`. `applicationId dev.scyde.browser`.

### 9.3 iOS (Swift + WKWebView, .ipa za Feather)

- **Arhitektura:**
  - SwiftUI za app shell, postavke i sheetove; UIKit `BrowserViewController` za WKWebView (geste, performanse, `UIScrollView` kontrola)
  - Swift 6 strict concurrency
  - `ScydeCore.xcframework` (Rust, targeti `aarch64-apple-ios` + `aarch64-apple-ios-sim`) se gradi u CI-ju
  - XcodeGen `project.yml` (bez ručnog `.pbxproj`, pa nema merge konflikata)
- **UI:**
  - **donja adresna traka** (kao Safari), swipe po traci mijenja tab, tab grid sa zoom tranzicijom
  - Shield sheet s detentima, haptika
  - sistemski materijali (na iOS 26+ Liquid Glass gdje prirodno paše, inače `ultraThinMaterial`); sve prema `apple-design` skillu
  - Dynamic Type
- **Engine postavke:** Safari-like UA (§2.2). `WKWebpagePreferences.allowsContentJavaScript` po stranici (opcija). Privatni tab koristi `nonPersistent` data store. `isInspectable` samo u debug buildu.
- **Paket:** CI na macOS runneru: `xcodebuild archive CODE_SIGNING_ALLOWED=NO`, zatim `Payload/<App>.app` zipan u `.ipa` i release artefakt. Feather potpisuje na uređaju. Opcija: AltStore-kompatibilan JSON izvor (GitHub Pages) za ažuriranja kroz Feather.
- **Očekivana ograničenja sideloada:**
  - aplikacija ne može biti zadani preglednik (entitlement `com.apple.developer.web-browser` dodjeljuje Apple)
  - bez push notifikacija
  - App Groups i share extension mogu ovisiti o certifikatu u Featheru, pa share extension ostaje opcionalan i ne smije biti obavezan za rad aplikacije

---

## 10. New Tab (prema referenci, ali vlastito)

- **Hero:** logo SCYDE + tri "privacy chipa" sa **živim brojkama** (npr. "Blokirano danas 1.284", "Ušteđeno 42 MB", "≈ 3 min").
- **Pretraga:** veliko polje s pill CTA gumbom (Ember). Prijedlozi iz povijesti, oznaka i tražilice (opt-out). Glasovni unos ide preko native speech API-ja kroz bridge. Pretraga slikom (Faza 6) šalje sliku na reverse-image stranicu odabrane tražilice, uz upozorenje o privatnosti.
- **Speed dial:** grid tileova (top sites + prikvačeni), drag reorder, "Dodaj". Favicon se dohvaća i kešira lokalno. Fallback je monogram tile u Aura boji.
- **Top Stories:** **opt-in** RSS agregator (kategorije: Vijesti, Tech, Sport, Zabava, Putovanja…). Izvore bira `[VLASNIK]` (HR + svjetski). Dohvat je direktan, bez trackera, s keširanjem. Kategorije koriste vlastite ikone (ne emoji kao u referenci).
- **Desktop lijevi rail:** web paneli (Gmail, Discord, Telegram, WhatsApp Web, LinkedIn… koje korisnik sam doda). Svaki panel je zaseban webview koji ostaje prijavljen.
- **Pozadina:** Aura gradijent ili vlastita slika. Bez sponzoriranih slika, ikad (R2).
- **Prazna stanja i break-ui:** 0 i 100 tileova, bez interneta, s isključenim vijestima.

---

## 11. Faze

> Veličine su relativne (S/M/L/XL). Faze 4 i 5 mogu ići paralelno nakon Faze 3 (zasebne grane i sesije).

### Faza 0: Temelji, dizajn i spikeovi (L) 🔒 gate
**Koraci:**
1. **0.1 Repo:** struktura iz §4, pinani toolchainovi, `CLAUDE.md`, `docs/STATUS.md`, `docs/NETWORK.md`, ADR predložak, Renovate, osnovni CI (lint i build prazni skeleton).
2. **0.2 Dizajn temelj:** skill `impeccable` finalizira **DESIGN.md**. `taste-skill` provjerava smjer protiv generičkog izgleda. `pick-ui-library` potvrđuje *Svelte 5 + vlastite komponente (bez UI kita)* unutar JS budgeta → ADR-0005.
3. **0.3 Brand i ikona:** tri originalna koncepta ikone (SVG) prema smjeru iz `design/references/03-app-icon-direction.png`, ali **bez dva preklopljena kruga** (vidi §16). Export pipeline `tools/icon-export`: Windows `.ico` 16–256, Android adaptive (foreground, background, monochrome za themed icons), iOS 1024 + dark + tinted varijante. `[VLASNIK]` bira koncept.
4. **0.4 Prototipovi** (`prototype` skill, `design/prototypes/`): New Tab light/dark, desktop chrome (tabovi, omnibox focus s prijedlozima, Shield panel), mobilni donji toolbar + tab switcher. `impeccable critique` na svakom.
5. **0.5 Spikeovi** (`spikes/`, jednokratni):
   - **S1 Google prijava** na sva tri enginea → **G1**
   - **S2 WebView2 Shield:** opcije A/B/C iz §5.4 na 5 teških stranica (npr. index.hr, 24sata.hr, jutarnji.hr, youtube.com, cnn.com) s mjerenjem vremena učitavanja u odnosu na baseline bez blokiranja → **G2**
   - **S3 GeckoView:** built-in ekstenzija, webRequestBlocking, WASM engine, native messaging s internih stranica → **G3**
   - **S4 iOS:** kompajliranje ≥ 150k pravila podijeljenih u više lista na simulatoru; mjerenje kompajliranja i keširanog loada → **G4**
   - **S5 Tauri multiwebview:** 20 tabova na jednom environmentu, prebacivanje, pre-warm popover prozora, Mica, custom titlebar + snap → **G5**

**AC / gateovi:**
- **G1:** Prijava u Google (Gmail, YouTube) radi na WebView2, GeckoView i WKWebViewu (screenshotovi u ADR-0002). Ako ne radi na WebView2, aktiviraj §2.3 i stani.
- **G2:** Shield na Windowsu dodaje ≤ 3 % (cilj 0) na median vremena učitavanja i blokira ≥ 95 % fixture zahtjeva (`qa/adblock-fixtures`). Odabrana opcija ide u ADR-0003.
- **G3:** GeckoView ekstenzija blokira fixture, WASM odluka p99 < 200 µs, interne stranice razgovaraju s aplikacijom.
- **G4:** Rule liste se kompajliraju. Keširani load je < 100 ms. Fixture je blokiran.
- **G5:** Prebacivanje taba ≤ 16 ms, popover ≤ 16 ms, snap layout radi.
- DESIGN.md je finaliziran, ikona odabrana, prototipovi odobreni od `[VLASNIK]`.

### Faza 1: Rust jezgra i Shield pipeline (L)
- Crateovi `shield`, `shield-compiler`, `omnibox`, `places`, `settings`, `theme` i `ffi` (UniFFI → Swift, Kotlin). WASM paket za ekstenziju.
- `filters.yml` (dnevno, potpis, objava). Golden testovi (fixture liste → očekivani DNR / iOS / kozmetički izlaz).
- Benchmarki: match latencija, deserijalizacija enginea, veličina bundlea.

**AC:**
- `cargo test` zelen; clippy bez upozorenja
- native match p50 < 5 µs, p99 < 50 µs; deserijalizacija < 50 ms (desktop)
- iOS liste ≤ 150k po listi; DNR izlaz unutar Chromium limita (s izvješćem o odbačenim pravilima)
- bundle potpisan i potpis provjeren u testu
- `omnibox` prolazi testove: URL ili pretraga, punycode i konfuzabilni nazivi, fixup (`scyde`, `localhost:3000`, `192.168.1.1`, `ftp://`)

### Faza 2: Web dizajn sustav i interne stranice (L)
- `web/packages/tokens` (iz Style Dictionaryja), `packages/ui` (Button, IconButton, Chip, Tile, Card, Menu, Popover, Tooltip, Toast, Toggle, Slider, Tabs, Sheet, SearchField, Kbd, Skeleton), `packages/bridge` (tipiziran, s mockom za testove).
- Aplikacije: `newtab`, `settings`, `shields`, `library`.
- Skillovi: `emil-design-eng` (detalji komponenti), `animate` + `choreograph` (ulazak New Taba, stagger tileova), `animation-performance`, `mobile-native` (interne stranice na telefonu: safe areas, touch targeti, overscroll, `100dvh`, bez tap highlighta), `awwwards-animations` (samo smjernice za hero i onboarding; implementacija kroz CSS i WAAPI, bez GSAP-a).
- Playwright vizualni testovi u **Chromiumu, Firefoxu i WebKitu** (sva tri enginea koja koristimo!), light i dark, 3 širine.

**AC:**
- bundle budgeti (§8)
- Lighthouse a11y ≥ 95
- svi kontrasti ≥ AA (automatski test)
- `prefers-reduced-motion` poštovan
- screenshotovi kao artefakti

### Faza 3: Windows MVP (XL)
- Tauri ljuska prema §9.1: tabovi, omnibox, navigacija, Shield (odabrana opcija), popup blocker, interne stranice, teme, sleeping tabs, preuzimanja, find, zoom, privatni prozor, prečaci, zadani preglednik, installer + updater.
- Skillovi: `emil-design-eng` (tab strip, omnibox, meniji), `choreograph` (otvaranje/zatvaranje taba, omnibox expand), `animation-performance`, `break-ui` na kraju.

**AC:**
- budgeti §8 (Windows stupac)
- G1 i dalje zelen
- 500 tabova bez rušenja (sleeping)
- `qa/` adblock ≥ 95 %, popup fixture 100 %
- updater testiran s dvije verzije
- `break-ui` izvješće bez P1 nalaza

### Faza 4: iOS MVP (L)
- §9.3: tabovi, donja traka, tab grid, Shield (rule liste + kozmetika + procjena brojača), popup, interne stranice, teme, privatni tab, dijeljenje, preuzimanja (Files), find, čitač (kasnije).
- Skillovi: `apple-design` (sheetovi, springovi, materijali, tipografija, geste), `mobile-native` (interne stranice), `break-ui`.
- CI: unsigned `.ipa` + XCUITest screenshotovi (light/dark, iPhone SE i Pro Max veličine).

**AC:**
- budgeti §8 (iOS)
- instalacija preko Feathera na stvarnom uređaju `[VLASNIK]` potvrđuje
- Google prijava radi
- adblock/popup fixture

### Faza 5: Android MVP (XL)
- §9.2: GeckoView, Compose UI, donji toolbar, tab switcher, Shield ekstenzija, ETP + TCP, popup, interne stranice, teme, privatni tabovi, preuzimanja, predictive back.
- Skillovi: `apple-design` (principi springova i gesti prevedeni u Compose), `mobile-native`, `animation-performance`, `break-ui`.

**AC:**
- budgeti §8 (Android)
- Baseline Profile aktivan (Macrobenchmark izvješće)
- APK po ABI-ju u releaseu
- Google prijava radi
- fixturei prolaze

### Faza 6: Premium polish i unikatne značajke (L)
- Animacijski audit: `find-animation-opportunities` → `improve-animations` → `review-animations` → `remove-animations` (makni sve što ne služi). Pa `impeccable audit` + `polish`, pa `break-ui` na svim platformama.
- **Unikatne značajke** (redoslijed potvrđuje `[VLASNIK]`):
  1. **Aura adaptivni tint** chromea prema `theme-color` ili dominantnoj boji favicona (uz zaštitu kontrasta)
  2. **Shield pill** s morph brojačem i "ušteđeno" statistikom
  3. **Sleeping tabs** s vidljivom uštedom RAM-a
  4. **Split view** (2 taba jedan uz drugi) + **command palette** (Ctrl+K: tabovi, povijest, akcije, postavke) na desktopu
  5. **Web paneli** u lijevom railu (desktop)
  6. **Peek** (long-press link preview) na mobitelu
  7. **Zen mod** po stranici (cookie banneri, annoyance liste, čitač)
  8. **Spaces** (grupe tabova s vlastitom Aura bojom)
- Onboarding (3 ekrana: tema, tražilica, uvoz), bez dark patterna.

**AC:**
- 0 animacija izvan budgeta
- `impeccable audit` bez kritičnih nalaza
- svaka značajka ima on/off i test

### Faza 7: Podaci, sinkronizacija, izdanje (L)
- Uvoz s Windowsa iz Chromea, Edgea i Firefoxa (oznake, povijest; lozinke samo ako je sigurno i transparentno, inače se savjetuje izvoz CSV-a).
- Upravitelj lozinki: v1 koristi OS (Windows Hello / Credential Manager, iOS AutoFill, Android Credential Manager), vlastiti tek nakon sigurnosnog pregleda.
- **SCYDE Sync** (opcija `[VLASNIK]`): E2E enkriptirana sinkronizacija oznaka, povijesti, postavki i tabova. Self-hosted server kao **Docker Compose** aplikacija (isti obrazac kao Mediarr/Streamero), klijent u `core/sync`.
- Release hardening: code signing, crash izvješća opt-in (self-hosted), `NOTICE.md` s licencama, privacy policy, pravila izdanja (`docs/release.md`), Play/F-Droid priprema `[VLASNIK]`.

---

## 12. Mapa skillova (kada se koji koristi)

| Skill | Kada | Rezultat |
|---|---|---|
| `impeccable` | Faza 0 (DESIGN.md), **svaki UI PR** (critique), Faza 6 (audit, polish) | DESIGN.md je živi dokument; nalazi idu u PR opis |
| `taste-skill` (design-taste-frontend) | prototipovi, New Tab, onboarding, marketing stranica | provjera da ništa ne izgleda generički ili "AI-made" |
| `prototype` | prije svake nove UI značajke | HTML prototip u `design/prototypes/` |
| `pick-ui-library` | Faza 0 | ADR-0005 (Svelte 5 + vlastite komponente ili obrazložena alternativa) |
| `emil-design-eng` | komponente: tabovi, omnibox, meniji, toastovi, tooltipovi, sheetovi | mikro-detalji, stanja, fokus, timing |
| `apple-design` | iOS faza; principi springova i gesti za Android i desktop | sheetovi, materijali, tipografija, geste |
| `mobile-native` | interne web stranice na Androidu i iOS-u | izgledaju i ponašaju se nativno |
| `animate` | nova animacija od nule | implementacija prema motion tokenima |
| `animation-vocabulary` | imenovanje efekata u DESIGN.md i PR-ovima | zajednički jezik |
| `choreograph` | sekvence više elemenata (New Tab ulaz, tab switcher, omnibox) | stagger i redoslijed |
| `animation-performance` | svaki PR s animacijom | compositor-only, bez layout thrashinga, u budgetu |
| `find-animation-opportunities` | Faza 6 | popis mjesta gdje kretanje nedostaje |
| `improve-animations` | Faza 6 | plan poboljšanja |
| `review-animations` | Faza 6, prije releasea | kritika |
| `remove-animations` | Faza 6, power-saver mod | uklanjanje suvišnog |
| `break-ui` | kraj Faza 3, 4, 5 i 6 | izvješće sa scenarijima iz §14.3 |
| `awwwards-animations` | New Tab hero, onboarding, reveal kod promjene teme | **samo smjernice**; bez GSAP-a i njegovih biblioteka (nema licence); implementacija kroz CSS, WAAPI i `linear()` springove |

---

## 13. CI/CD i GitHub

### 13.1 Workflowi (`.github/workflows/`)

| Workflow | Runner | Okidač | Radi |
|---|---|---|---|
| `core.yml` | ubuntu | promjene u `core/**` | fmt, clippy, test, bench smoke, WASM build |
| `web.yml` | ubuntu | `web/**`, `shield-extension/**`, `design/tokens/**` | lint, typecheck, vitest, Playwright (3 enginea) + screenshotovi, bundle-size check |
| `filters.yml` | ubuntu | cron dnevno + ručno | compile, test, sign, publish filter bundle |
| `windows.yml` | windows | `desktop/**` + ovisnosti; tag `v*` | build NSIS/MSI, smoke start + screenshot; release s updater artefaktima na tagu |
| `android.yml` | ubuntu | `android/**` + ovisnosti; tag | assemble, unit testovi, (opcija) emulator smoke + screenshotovi; potpisani APK na tagu |
| `ios.yml` | macos | `ios/**` + ovisnosti; **tag ili ručno** | xcframework, xcodegen, simulator build + XCUITest screenshotovi; unsigned .ipa na tagu |

- Path filteri, `concurrency: cancel-in-progress`, keš (rust-cache, Gradle, pnpm, SPM).
- **Trošak:** na privatnom repou macOS (i Windows) minute se naplaćuju višestruko skuplje od Linuxa, pa iOS build ide samo na tag ili ručno, a PR-ovi grade simulator samo kad se mijenja `ios/**`.

### 13.2 Secrets
`TAURI_SIGNING_PRIVATE_KEY` (+ `_PASSWORD`), `ANDROID_KEYSTORE_B64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`, `FILTERS_SIGNING_KEY`, kasnije Windows code signing. Javni ključevi za provjeru potpisa su u repou (`brand/keys/`).

### 13.3 Vizualna verifikacija bez uređaja
Svaki UI PR prilaže artefakte: Playwright screenshotove internih stranica (3 enginea × light/dark × 3 širine), Windows smoke screenshot, iOS simulator screenshotove i Android emulator screenshotove (gdje je job aktivan). Claude Code linka artefakte u PR opisu.

### 13.4 Konvencije
- Grane `phase/N-naziv`, `feat/…`, `fix/…`. Conventional Commits. Mali PR-ovi. Squash merge.
- Verzioniranje SemVer `v0.x.y` do javnog izdanja. Jedan tag gradi sve platforme.
- **ADR predložak:** Kontekst · Opcije · Odluka · Posljedice · Mjerenja/dokazi · Status.

---

## 14. Testiranje i QA

### 14.1 Automatski
- **core:** unit, property testovi (omnibox parsing), golden testovi kompajlera, benchmarki
- **web:** vitest, Playwright, axe a11y, vizualni diff
- **Windows:** integracijski testovi kroz WebView2 CDP u dev buildu (remote debugging port **samo u debugu**)
- **Android:** JUnit, Compose UI testovi, Macrobenchmark
- **iOS:** XCTest, XCUITest

### 14.2 `qa/` fixturei (determinističko, lokalno posluženo)
- `adblock/`: stranica s poznatim oglasnim i tracker URL-ovima (network), elementima za kozmetiku (`##.ad-banner`, generički i specifični) i scriptlet testom
- `popup/`: `window.open` bez geste, s gestom, `target=_blank`, popunder, redirect lanac, `window.open` u `setTimeout` nakon klika
- `perf/`: teška stranica za mjerenje
- ručni popis stvarnih stranica: youtube.com, index.hr, 24sata.hr, jutarnji.hr, net.hr, cnn.com, forbes.com, reddit.com, twitch.tv, gmail.com

### 14.3 `break-ui` scenariji (obavezni)
- 500 tabova; naslov od 300 znakova; RTL i emoji naslovi; nedostaje favicon
- URL od 4.000 znakova; IDN homograf (npr. ćirilični "а" u `аpple.com`)
- spora mreža i offline; preuzimanje od 5 GB
- 0 i 100 speed-dial tileova
- skaliranje 200 % (Windows), mobilni tekst na 200 %, prozor širine 500 px
- Windows High Contrast; Dark ↔ Light usred animacije
- vrlo dugi popisi povijesti (100k zapisa)

---

## 15. Rizici

| Rizik | Vjerojatnost | Utjecaj | Mitigacija |
|---|---|---|---|
| Google blokira prijavu u WebView2 | srednja | kritičan | G1 u Fazi 0; Edge-identičan UA; fallback §2.3 |
| DNR u WebView2 ekstenziji ograničen ili nestabilan | srednja | visok | S2 mjeri opcije A/B/C; odluka u ADR |
| Tauri multiwebview (feature `unstable`) se mijenja | srednja | srednji | pin verzije, escape hatch na sirovi WebView2 COM, testovi u S5 |
| GeckoView API promjene između verzija | srednja | srednji | Renovate + CI smoke, `EngineSession` apstrakcija |
| YouTube i anti-adblock skripte | visoka | srednji | dnevni bundle, scriptleti, brzi hotfix kanal |
| iOS sideload ograničenja | sigurno | nizak/srednji | dokumentirano (§9.3), bez ovisnosti o entitlementima |
| Opseg (3 platforme, 1 developer) | visoka | visok | stroge faze, AC, MVP prije polisha, paralelizacija Faza 4 i 5 |
| Licence (liste, scriptleti, font) | srednja | srednji | §16, `NOTICE.md`, odluka o open-sourceu |

---

## 16. Licence i pravno

- **adblock-rust:** MPL-2.0 (copyleft na razini datoteke; izmjene u MPL datotekama se objavljuju). **GeckoView:** MPL-2.0.
- **Filter liste:** EasyList i EasyPrivacy su dvojno licencirane (GPLv3 / CC BY-SA). uBlock filteri i **scriptlet resursi su GPLv3**. Ako aplikacija ostane zatvorenog koda, liste i resursi se distribuiraju kao **zaseban podatkovni bundle** s atribucijom i linkom na izvor. Prije javnog izdanja potreban je pravni pregled `[VLASNIK]`.
- **Font Aeonik** (CoType Foundry) je komercijalan; za ugradnju u aplikaciju treba app licenca. Do tada se koristi OFL zamjena (DESIGN.md §3).
- **Ikone:** Lucide (ISC) ili vlastite.
- **Referentne slike** u `design/references/` su inspiracija. **Ne kopiraj** crveni logo ni točan layout tuđeg Dribbble/Behance rada, i ne koristi njihove assete.
- **Ikona:** dva preklopljena narančasto-crvena kruga su vizualno vrlo blizu poznatom zaštitnom znaku kartične kuće. Zadrži osjećaj (meki conic gradijent, bijela squircle pločica), a oblik napravi originalnim (Faza 0, korak 0.3).
- Prije javnog izdanja provjeri zaštitni znak za naziv "SCYDE Browser".

---

## 17. Otvorena pitanja za vlasnika (`[VLASNIK]`)

Zadane vrijednosti vrijede dok vlasnik ne odluči.

1. **Naziv i label na home screenu.** Default: "SCYDE Browser". Pazi na sukob s postojećom aplikacijom čiji je label "SCYDE".
2. **Zadana tražilica.** Default: DuckDuckGo. Alternative: Brave Search, Startpage, Google, Bing, Ecosia.
3. **Open source ili zatvoreni kod?** Utječe na §16.
4. **Aeonik licenca ili OFL zamjena?**
5. **Windows code signing** (budžet, SmartScreen).
6. **SCYDE Sync** self-hosted na vlastitoj infrastrukturi: da/ne.
7. **Izvori za Top Stories** (HR + svjetski), opt-in.
8. **Distribucija za Android:** samo GitHub Releases ili i Play/F-Droid.

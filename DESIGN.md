# DESIGN.md: "Otok", vizualni jezik SCYDE Browsera

> **v2.0 (2026-11-11).** Zamjenjuje v1 "Aura" (ADR-0008). Ovo je **jedini izvor istine** za izgled i kretanje.
> Svaka vrijednost postoji kao token u `design/tokens/*.json` (DTCG). Kod nikad ne hardkodira vrijednosti.
> Kontraste provjerava CI (`pnpm tokens && pnpm contrast`, 204 para, svi ≥ AA). Fontovi (OFL) su u `design/fonts/`.

---

## 1. Esencija

**Sadržaj od ruba do ruba, sučelje kao jedan predmet.** SCYDE nema traku s tabovima ni toolbar na vrhu. Cijelo sučelje je **otok**: jedan plutajući objekt na dnu prozora koji nosi znak, navigaciju, adresu, Shield i tabove, a širi se kad ga trebaš.

- **Papir i tinta.** Stranica počiva na toplom papiru (light) ili gotovo crnoj tinti (dark). **Otok je uvijek inverz:** tinta na papiru, papir na tinti. Zato ga se uvijek odmah nađe.
- **Ember je energija.** Narančasto-crveni Ember pojavljuje se samo tamo gdje je radnja: kursor, aktivni tab, glavni gumb, znak.
- **Riječi umjesto pločica.** New Tab ne počinje mrežom ikona nego pitanjem "Kamo danas?", u koje se izravno tipka.
- **Stvarne brojke.** Shield statistika dolazi iz enginea i stoji uz otok, mono fontom.
- **Tišina dok čitaš.** Kad se skrola, otok se smanjuje na sitni "kamenčić" i vraća se pri skrolu prema gore ili kad mu se kursor približi.

### Po čemu nije kao drugi preglednici
| Uobičajeno | SCYDE Otok |
|---|---|
| Tab strip + toolbar na vrhu | Nema ničega na vrhu osim sitnih Windows caption gumba |
| Adresna traka cijelom širinom | Adresa živi u otoku; domena je naglašena, put je mono i gasi se fade maskom |
| Tabovi kao kartice u retku | Tabovi kao složeni krugovi (stack) u otoku; klik ih rasklapa u policu iznad otoka |
| New Tab = logo + polje + mreža pločica | New Tab = jedno pitanje u koje se tipka + popis "nedavno" s vremenima |
| Ad blocker kao ikona i broj | Shield je kontrolna lampica u otoku + mono rečenica na New Tabu |

---

## 2. Boje

### 2.1 Papir i tinta (površine i tekst)

| Token | Light | Dark | Upotreba |
|---|---|---|---|
| `surface.canvas` | `#F3F1EC` papir | `#0D0D0F` tinta | pozadina New Taba i internih stranica |
| `surface.sunken` | `#EEEBE5` | `#1A1A1D` | polja i retci unutar panela |
| `surface.raised` | `#FFFFFF` | `#1F1F23` | paneli koji izlaze iz otoka |
| `border.hairline` | ink 10 % | papir 10 % | 1 px linije |
| `text.primary` | `#121214` | `#F1EEE7` | naslovi i tekst |
| `text.secondary` | `#5B5A57` | `#A6A39C` | opisi, meta |
| `text.tertiary` | `#66645F` | `#8E8B84` | hintovi, vremena |

### 2.2 Otok (inverzna površina)

| Token | Light (tinta) | Dark (papir) |
|---|---|---|
| `island.bg` | `#121214` | `#F1EEE7` |
| `island.raised` | `#2A2A2E` | `#DAD6CD` |
| `island.text` | `#F3F1EC` | `#121214` |
| `island.text-secondary` | `#A8A6A0` | `#5B5A57` |
| `island.shield` | `#3DDC97` | `#0B5E33` |
| `elev.island` | `0 18 50 -18 rgba(0,0,0,.45)` | `… rgba(0,0,0,.60)` |

### 2.3 Accent (korisnik bira, default **Ember**)
Accent obilježava "ovdje si": kursor, aktivni tab, fokus prsten i odabrani redak. Svaki accent ima ulogu po površini:

| Uloga | Na papiru (light) | Na papiru (dark) | Na otoku |
|---|---|---|---|
| `accent.ink` (tekst, ikone) | tamna ink varijanta | pastel | – |
| `accent.fill` + `accent.on-fill` | pastel + `#121214` | pastel + `#121214` | – |
| `accent.island` | – | – | light: pastel na tinti; dark: ink na papiru (≥ 3:1) |

Predlošci: **Ember** (default), Sky, Periwinkle, Lavender, Mint, Onyx. Kod vlastite boje `core/theme` izvodi sve uloge u OKLCH-u i odbija kombinacije ispod AA.

### 2.4 Brend
- **Ember:** `ember.300 #FFB27A` → `ember.800 #B8322C`. CTA je `ember.700` s bijelim tekstom (5,08:1), a hover `ember.800`.
- **Znak SCYDE S** (vlasnikov logo, `brand/icon/scyde.svg`): S od dva zrcalna dijela s dijagonalnim rezom (munja). Boje su `brand.mark` = `ember.500 #F2603F` i `brand.ink #0A0A0A`, pa Ember ostaje jedina boja energije u cijelom brendu. Znak je **samo slovo S u Emberu, bez pločice i pozadine**, i u sučelju (24 px u otoku) i u ikonama.
- **Bez Aura mesh pozadina.** Papir je čist; boja dolazi samo od faviconâ, accenta i Embera.

### 2.5 Semantičke
`status.shield` (light `#107640`, dark `#3DDC97`), `status.warning`, `status.danger`, `status.info`: kao u tokenima, sve ≥ AA na svim površinama.

---

## 3. Tipografija
- **Bricolage Grotesque** (OFL, variable, opsz 12–96): glas brenda. Koristi se za hero, UI i nazive stranica. Na velikim veličinama ima karakter (stisnuti ritam, tamni rezovi), a na malim je čitljiv.
- **Geist Mono** (OFL): podaci. Shield brojke, vremena ("2 min"), putovi u adresi, hintovi za tipke.
- **Brojke:** uvijek `tabular-nums`.
- **Težine:** 400 i 500, plus 600 samo za hero. Nikad VELIKA SLOVA, pa ni u mono hintovima.
- Mobilni nativni UI (SwiftUI, Compose) koristi iste fontove ugrađene u aplikaciju; Dynamic Type i font scale do 200 % se poštuju.

| Token | Veličina / red | Tracking | Upotreba |
|---|---|---|---|
| `type.hero` | 104 / 102 | −0,045em | "Kamo danas?" (mobitel 54 / 56) |
| `type.display` | 56 / 60 | −0,02em | onboarding |
| `type.headline` | 32 / 38 | −0,015em | naslovi internih stranica |
| `type.title` | 20 / 26 | −0,01em | naslovi panela |
| `type.address` | 16 / 22 | −0,01em | adresa u otoku |
| `type.body-lg` | 16 / 24 | 0 | redci "nedavno" (19 px na desktopu) |
| `type.body` | 14 / 20 | 0 | tekst |
| `type.label` | 13 / 18 | +0,005em | gumbi, meniji |
| `type.data` | 13 / 20 | 0 | mono podaci |
| `type.caption` | 12 / 16 | +0,01em | meta |

---

## 4. Prostor, oblik, veličine
- **Grid:** 4 pt. Razmaci `2…80` kao u tokenima.
- **Radijusi:** `xs 6` · `sm 10` · `md 14` (redci, polja) · `lg 22` (paneli) · `xl 28` · **`island 30`** · `full`.
- **Otok (desktop):** visina 60, širina 720 (mirovanje) i 760 (prošireno), 22 px od dna, centriran. Gumbi 44, tab-krugovi 30 s preklopom od 9 px, adresa 420.
- **Otok (mobitel):** visina 58, 12 px od rubova, iznad safe-area.
- **Caption gumbi (Windows):** 34 × 24, plutaju gore desno bez trake, s hoverom. Snap Layouts: ADR-0007.
- **New Tab (desktop):** padding 72/96, stupac "nedavno" 360, redci 50.
- **Hit targeti:** 32 px (miš), 44 pt (iOS), 48 dp (Android).

---

## 5. Dubina i materijali
- Sve što izlazi iz otoka (prijedlozi, police s tabovima, Shield panel) je `surface.raised` + `elev.2` i raste **iz otoka prema gore**.
- Otok ima jednu sjenu (`elev.island`); nikad se ne slažu stakla.
- **Windows:** bez Mice iza sadržaja (stranica ide do ruba). Otok je zaseban transparentni top-level prozor, kao i paneli (CLAUDE.md, ADR-0007).
- **iOS:** otok je nativni SwiftUI; uz Reduce Transparency ostaje pun. **Android:** puni otok, bez blura.

---

## 6. Kretanje

### 6.1 Principi
1. Otok je predmet: mijenja oblik (morph), ne pojavljuje se niotkuda.
2. Brzo i prekidivo. Česte radnje (Ctrl+Tab, tipkanje, skrol) su trenutne.
3. Animiraju se samo `transform` i `opacity`. Kod otoka je to širina kao `scaleX` na omotaču, uz kontra-skaliranje sadržaja.
4. Reduced motion: sve postaje cross-fade od 120 ms.

### 6.2 Tokeni
Trajanja (`instant 90`, `fast 140`, `base 200`, `slow 300`, `choreo-max 420`) i springovi (`snappy`, `smooth`, `bouncy`) su nepromijenjeni u odnosu na v1. Vremena smirivanja: 420, 663 i 805 ms.

### 6.3 Imenovani efekti
| Efekt | Opis |
|---|---|
| **Press** | scale 0,97, 90 ms, snappy povratak |
| **Rise** | ulazak: opacity 0→1, y 8→0, `ease.out`, 200 ms |
| **Swell** | otok se širi (720→760) i iz njega raste panel prema gore: smooth spring |
| **Pebble** | skrol prema dolje: otok se sažima u kamenčić (znak + Shield točka), 200 ms; vraća se pri skrolu gore ili kad se kursor približi |
| **Fan** | tab-krugovi se rasklapaju u policu kartica iznad otoka, stagger 24 ms, najviše 8 |
| **Caret** | Ember kursor u "Kamo danas?" trepće 1,1 s; uz reduced motion je statičan |
| **Morph** | Shield brojač: roll znamenki; bouncy samo pri skoku s 0 |
| **Dissolve** | promjena teme: View Transition cross-fade 200 ms |
| **Sleep** | uspavani tab: krug na 60 % + mjesec |

### 6.4 Koreografije
- **New Tab ulazak** (≤ 420 ms): Shield redak Rise (0) → hero Rise (28 ms) → redci "nedavno" Rise, stagger 24 ms. Polje je odmah interaktivno.
- **Fokus adrese:** otok Swell, prijedlozi rastu iznad njega (Bloom iz otoka), 140 ms.
- **Tabovi:** klik na stack izaziva Fan (police iznad otoka); Esc ili klik izvan vraća krugove.

---

## 7. Ikonografija
- Linijske ikone, potez 1,5 px, grid 20/24, Lucide (ISC) + vlastite.
- Favicon bez slike dobiva **monogram u Aura pastelu** po hostu (stabilno).
- Ikona aplikacije: **SCYDE S, samo slovo u Emberu, bez pozadine** (vlasnik, 2026-11-11). Windows `.ico`: prozirno, znak preko ~92 % površine. Android: prozirna adaptive pozadina + Ember S u safe zoni; monochrome za themed icons. iOS: dark i tinted prozirni; **light mora biti neproziran (Apple), pa ima bijelu pozadinu**. Exporti: `node tools/icon-export/export.mjs` (`brand.json → icon.concept = owner-s`).

---

## 8. Komponente (stanja obavezna: default, hover, press, focus-visible, disabled, loading)

| Komponenta | Ključni detalji |
|---|---|
| **Otok** | Slijeva nadesno: znak (izbornik) · natrag · \| · adresa · Shield kamenčić (točka + broj) · \| · tab stack (+N). Desktop 720/60, mobitel puna širina − 24. |
| **Adresa u otoku** | Mirovanje: domena (eTLD+1) u `island.text`, put mono u `island.text-secondary` s fade maskom. Fokus: puni URL selektiran, otok Swell, prijedlozi iznad. Nikad ne skriva domenu (PLAN §7). |
| **Prijedlozi** | Panel iznad otoka, redci 48: favicon, naslov, domena u accent inku, desno izvor (povijest, oznaka, tražilica) i `Enter` na odabranom. |
| **Tab stack i polica** | Krugovi 30 s preklopom; aktivni ima accent prsten. Polica (Fan): kartice 168 × 112 sa snapshotom i naslovom, `+` na kraju, tipke 1–9. |
| **Shield kamenčić i panel** | Točka `island.shield` + broj (Morph). Panel iznad otoka: veliki prekidač, dvije brojke, razina, Zen, detalji. |
| **New Tab** | Shield mono redak · hero pitanje s Ember kursorom · hintovi tipki · stupac "nedavno" (favicon, host, vrijeme mono). Vijesti su opt-in i prikazuju se kao drugi stupac popisa, ne kao kartice. |
| **Caption gumbi** | Windows: tri sitna gumba gore desno, bez trake; hover `surface.sunken`, zatvaranje `status.danger`. |
| **Meni** | Panel iz znaka u otoku, redci 32, prečaci desno u mono. |
| **Toast** | Izlazi iz otoka prema gore, jedna radnja, 4 s. |
| **Mobitel: otok** | Isti otok na dnu; swipe lijevo i desno po otoku mijenja tab, swipe gore otvara tab grid (polica preko cijelog ekrana). |

---

## 9. Teme i prilagodba
Root atributi: `data-theme="light|dark"` · `data-accent="ember|sky|periwinkle|lavender|mint|onyx|custom"` · `data-density` · `data-motion`.
Ključne varijable: `--island-*`, `--accent-island`, `--accent-ink`, `--accent-fill`, `--accent-ring`, `--surface-*`, `--text-*`.
- **Adaptivni tint (opcija):** otok uzima 8–12 % `theme-color` stranice uz provjeru kontrasta; ako provjera padne, tint se ne primjenjuje.
- **Pozadina New Taba:** papir (default), puna boja ili vlastita slika uz zatamnjenje 0–40 %. Hero tekst zadržava AA kontrast.

---

## 10. Pristupačnost
- Fokus prsten je `2px solid var(--accent-ring)` na papiru, odnosno `var(--accent-island)` na otoku, uz offset od 2 px.
- Otok je landmark (`<nav aria-label>`). Tab stack je `tablist`, prijedlozi `listbox`, paneli `dialog`.
- Shield brojač je `aria-live="polite"` s ograničenjem učestalosti.
- Poštuju se Dynamic Type, 200 %, Reduce Motion, Reduce Transparency, High Contrast i Increase Contrast (jači hairline).

---

## 11. Da / Ne
**Da:** sadržaj do ruba · jedan predmet za sve sučelje · papir i tinta · Ember samo na radnji · riječi umjesto pločica · mono za podatke · stvarne brojke · tišina dok se čita.

**Ne:** traka s tabovima na vrhu · siva "Chrome" sučelja · gradijentne pozadine · staklo na staklu · teške sjene (osim jedne na otoku) · emoji · "AI iskrice" · 3D blobovi · VELIKA SLOVA · mreže jednakih pločica · više od dvije težine po prikazu.

---

## 12. Tokeni u kodu i changelog
Izlazi: `web/packages/tokens/dist/{css,kotlin,swift,json}`. Generator: `pnpm tokens`. Gate: `pnpm contrast`.

### Changelog
- **v2.0 (2026-11-11):** vlasnik je odbacio v1 "Aura" ("ne smije ličiti na svaki drugi browser") i između tri smjera (Otok, Kralježnica, Tipke; `design/prototypes/redesign/`) izabrao **Otok** (ADR-0008).
  - Nova paleta papir i tinta, otok kao inverzna površina, accent uloga `island`, default accent Ember.
  - Fontovi Bricolage Grotesque i Geist Mono (OFL) umjesto Geist/Aeonik.
  - Nove veličine otoka, `radius.island`, `type.hero`, `type.data`, `type.address`.
  - Novi efekti Swell, Pebble, Fan i Caret.
  - Aura mesh pozadine su uklonjene.
  - Contrast gate je proširen na parove na otoku (204 para).
  - Ikona i znak: vlasnikov **SCYDE S** zamjenjuje Aura S; po želji vlasnika je u **Ember** boji (prvo lime, pa prebačeno), tokeni `brand.mark` i `brand.ink`.
- **v1.0 (Faza 0.2):** "Aura" (u git povijesti).

# DESIGN.md: "Aura", vizualni jezik SCYDE Browsera

> Ovo je početna verzija. U Fazi 0 je finalizira skill `impeccable`, a nakon toga je **jedini izvor istine** za izgled i kretanje.
> Svaka vrijednost ovdje postoji kao token u `design/tokens/*.json` (DTCG). Kod nikad ne hardkodira vrijednosti.
> Svi kontrasti u ovom dokumentu su izračunati (WCAG 2.x). Nakon promjene tokena pokreni `tools/contrast-check`.

---

## 1. Esencija

**"Tihi premium."** Sadržaj je u prvom planu. Sučelje je mirno, mekano i precizno, a energija se pojavljuje samo tamo gdje je radnja.

- **Aura** (pastelne boje) je *atmosfera*: accenti, odabrana stanja, pozadine New Taba.
- **Ember** (topli narančasto-crveni gradijent iz ikone) je *energija*: glavni CTA, brend i Shield trenuci.
- Jedan fokus po ekranu. Na New Tabu je to pretraga.
- Brojke su stvarne. Shield statistika dolazi iz enginea, ne iz marketinga.

### Što uzimamo iz referenci, a što ne

| Uzimamo | Ne uzimamo |
|---|---|
| Light/Dark kao ravnopravne teme (split prikaz iz reference 01) | Crveni "Santa" logo i "Ho-ho-ho" (tuđi rad) |
| Meke "sunken" tileove, pill CTA, velik razmak | macOS semafor gumbe na Windowsu |
| Lijevi rail s web panelima | Emoji kao ikone kategorija |
| Pastelnu paletu iz reference 02 kao Aura accente | Ikone novčanika i poklona (crypto/rewards izvan opsega) |
| Osjećaj ikone iz reference 03 (bijela squircle pločica, meki conic gradijent) | Oblik "dva preklopljena kruga" (blizak poznatom zaštitnom znaku) |

---

## 2. Boje

### 2.1 Neutralne (površine i tekst)

| Token | Light | Dark | Upotreba |
|---|---|---|---|
| `surface.canvas` | `#F1F2F5` | `#1C1D24` | pozadina stranica (New Tab, postavke) |
| `surface.chrome` | `#F7F8FA` | `#17181E` | tab strip i toolbar (na Windowsu ispod je Mica) |
| `surface.sunken` | `#E9EBEF` | `#23252D` | tileovi, polje pretrage, omnibox, chipovi |
| `surface.raised` | `#FFFFFF` | `#2A2C35` | popoveri, meniji, sheetovi, kartice vijesti |
| `border.hairline` | `rgba(17,18,22,.08)` | `rgba(255,255,255,.07)` | 1 px rubovi |
| `text.primary` | `#111216` | `#F2F3F5` | naslovi, tekst |
| `text.secondary` | `#545866` | `#A3A7B3` | opisi, meta |
| `text.tertiary` | `#626777` | `#9398A6` | placeholder, hintovi |

Izmjereni kontrasti (najgori slučaj po površini):
- Light: primary ≥ 15,7:1, secondary ≥ 5,9:1, tertiary ≥ 4,7:1.
- Dark: primary ≥ 12,5:1, secondary ≥ 5,8:1, tertiary ≥ 4,8:1.

Sve je **AA i za normalni tekst**.

### 2.2 Aura accenti (korisnik bira, default **Sky**)

Pastelne boje imaju nizak kontrast na svijetloj pozadini (Sky na `#F1F2F5` je samo 1,4:1). Zato vrijedi:

> **Light mod:** pastel je **ispuna** (fill) s tamnim tekstom, a za tekst i ikone u accent boji koristi se **ink** varijanta.
> **Dark mod:** pastel se koristi direktno kao tekst i ikona (8–13:1) i kao ispuna s tamnim tekstom.

| Accent | Fill (pastel) | Ink (light: tekst i ikone) | Ink na `sunken` | Tamni tekst na fillu |
|---|---|---|---|---|
| **Sky** (default) | `#ABD5E9` | `#236F93` | 4,67:1 | 12,6:1 |
| **Periwinkle** | `#A6C0F1` | `#1E60DE` | 4,65:1 | 10,7:1 |
| **Lavender** | `#C6A5F1` | `#853BE5` | 4,69:1 | 9,4:1 |
| **Mint** | `#A6F1C9` | `#107640` | 4,77:1 | 15,0:1 |
| **Ember** | `#FF9F7E` | `#B8322C` | 4,98:1 | 9,8:1 |
| **Onyx** (#000 iz palete) | light `#111216`, dark `#F2F3F5` | isto | ≥ 15:1 | – |

**Vlastita boja:** `core/theme` iz korisničkog hexa izvodi fill i ink (OKLCH, mijenja se lightness uz očuvan hue) i odbija kombinacije ispod 4,5:1. UI tada pokazuje najbližu prihvatljivu nijansu.

### 2.3 Brend: Ember

| Token | Hex | Napomena |
|---|---|---|
| `ember.300` | `#FFB27A` | početak gradijenta ikone |
| `ember.400` | `#FF8E5C` | |
| `ember.500` | `#F2603F` | hover sjaj, ilustracije (ne za tekst na svijetlom) |
| `ember.600` | `#D9433A` | bijeli tekst = 4,36:1, **samo za velik/bold tekst** |
| `ember.700` | `#C93A33` | **CTA ispuna** (bijeli tekst 5,08:1) |
| `ember.800` | `#B8322C` | ink, danger tekst (light) |

- `gradient.ember`: `conic-gradient(from 210deg, #FFB27A, #FF8E5C, #F2603F, #FFB27A)` za ikonu, logo i Shield "bloom" trenutak.
- `gradient.aura.*`: 3–4 meka mesh gradijenta od Aura boja **niske zasićenosti** za pozadinu New Taba. Najviše jedan gradijent po ekranu.

### 2.4 Semantičke

| Token | Light | Dark |
|---|---|---|
| `status.shield` / `success` | `#107640` | `#A6F1C9` |
| `status.warning` | `#8A5300` | `#F5C26B` |
| `status.danger` | `#B8322C` | `#FF8A7A` |
| `status.info` | `#1E60DE` | `#A6C0F1` |

Shield ikona u toolbaru je `status.shield` kad je zaštita aktivna, a `text.tertiary` kad je isključena za tu stranicu.

---

## 3. Tipografija

- **Brend i display:** *Aeonik* (CoType Foundry) **samo uz kupljenu app licencu** `[VLASNIK]`. Do tada je zamjena **Geist** (SIL OFL), a fallback je sistemski font. Provjeri glifove č, ć, đ, š, ž (`tools/font-check`).
- **UI:**
  - Desktop: brend font (Geist/Aeonik); fallback `"Segoe UI Variable", system-ui`
  - iOS: **SF Pro** (sistem; Dynamic Type)
  - Android: **Roboto Flex / sistem**
  - Brend font se na mobitelu koristi samo za display naslove na internim stranicama.
- **Brojke:** `font-variant-numeric: tabular-nums` za sve brojače (Shield, preuzimanja, vrijeme).
- **Težine:** 400 i 500 (iznimno 600 za display). Najviše dvije težine po prikazu.

| Token | Veličina / visina reda | Tracking | Upotreba |
|---|---|---|---|
| `type.display` | 56 / 60 | −0,02em | onboarding hero |
| `type.headline` | 32 / 38 | −0,015em | "Top Stories", naslovi sekcija |
| `type.title` | 20 / 26 | −0,01em | naslovi panela i sheetova |
| `type.body-lg` | 16 / 24 | 0 | polje pretrage na New Tabu, mobilni tekst |
| `type.body` | 14 / 20 | 0 | standardni tekst |
| `type.label` | 13 / 18 | +0,005em | **desktop chrome default** (tabovi, omnibox, meniji) |
| `type.caption` | 12 / 16 | +0,01em | meta, tile labele |
| `type.micro` | 11 / 14 | +0,02em | badgevi (rijetko) |

---

## 4. Prostor, oblik, veličine

- **Grid:** 4 pt. Razmaci: `2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80`.
- **Radijusi:** `xs 6` (kbd) · `sm 10` (tab, mali gumbi) · `md 14` (input, meni, toast) · `lg 22` (tile, kartica) · `xl 28` (sheet, dialog) · `full` (chip, pill CTA, omnibox na New Tabu). Kutove prozora na Windows 11 određuje sustav; ne mijenjaj ih.
- **Desktop chrome:** tab strip 44, tab 34 (min širina 72, max 240), toolbar 48, omnibox 34 (radius full), rail 72 s gumbima od 44 (krug), ikone 20 px.
- **New Tab:** max širina sadržaja 1120; polje pretrage 56 (radius full) s CTA od 48; tile 272×132 (radius 22, razmak 12, grid 4 stupca na desktopu i 2 na mobitelu); chip 36.
- **Gustoća Compact:** visine × 0,875, razmaci jedan korak niže.
- **Hit targeti:** ≥ 44 pt (iOS), ≥ 48 dp (Android), ≥ 32 px (desktop miš, s hover stanjem).

---

## 5. Dubina i materijali

- **Light:** sjene su nježne i dvoslojne.
  - `elev.1`: `0 1px 2px rgba(17,18,22,.06)`
  - `elev.2` (popover): `0 8px 24px -6px rgba(17,18,22,.18), 0 0 0 1px rgba(17,18,22,.06)`
  - `elev.3` (sheet, dialog): `0 24px 64px -12px rgba(17,18,22,.28)`
- **Dark:** dubina dolazi od **svjetlije površine i hairlinea**, ne od sjene. Uz to `inset 0 1px 0 rgba(255,255,255,.04)` kao gornji highlight.
- **Windows:** Mica ispod tab stripa i toolbara. Popoveri su **puna** `surface.raised` + hairline (bez Acrylica, zbog performansi). Kad je "Transparency effects" isključen, nema ni Mice.
- **iOS:** sistemski materijali (Liquid Glass na iOS 26+ gdje prirodno paše, inače `.ultraThinMaterial`) za donju traku i sheetove. Reduce Transparency znači pune boje.
- **Android:** pune površine i scrim. Blur ne koristimo (trošak na slabijim uređajima).

---

## 6. Kretanje

### 6.1 Principi
1. **Svrha prije ukrasa.** Animacija objašnjava odakle je nešto došlo i kamo je otišlo. Ako to ne radi, briše se (`remove-animations`).
2. **Brzo i prekidivo.** Svaka animacija se može prekinuti novom radnjom bez skoka.
3. **Česte radnje se ne animiraju teatralno.** Ctrl+Tab, tipkanje u omnibox i scroll ostaju trenutni.
4. **Samo compositor svojstva:** `transform` i `opacity` (web); native ekvivalenti na mobitelu.
5. **Reduced motion:** sve postaje `opacity` cross-fade od 120 ms, a parallax i stagger se gase.

### 6.2 Tokeni

| Token | Vrijednost | Upotreba |
|---|---|---|
| `motion.duration.instant` | 90 ms | hover, press |
| `motion.duration.fast` | 140 ms | tooltip, mala stanja, omnibox popover |
| `motion.duration.base` | 200 ms | meni, promjena teme (cross-fade) |
| `motion.duration.slow` | 300 ms | sheet, tab switcher |
| `motion.duration.choreo-max` | 420 ms | **gornja granica** cijele koreografije (npr. ulazak New Taba) |
| `motion.ease.out` | `cubic-bezier(0.23, 1, 0.32, 1)` | ulasci |
| `motion.ease.in-out` | `cubic-bezier(0.77, 0, 0.175, 1)` | pomaci na ekranu |
| `motion.ease.exit` | `cubic-bezier(0.4, 0, 1, 1)` | izlasci (trajanje × 0,7) |
| `motion.spring.snappy` | response 0,30 s · damping 0,86 | toggle, press, pill |
| `motion.spring.smooth` | response 0,45 s · damping 1,0 | sheetovi, tab switcher, drag reorder |
| `motion.spring.bouncy` | response 0,50 s · damping 0,72 | **samo** Shield counter "pop" |
| `motion.stagger` | 24–32 ms, najviše 8 elemenata | liste i tileovi |

**Springovi na sve tri platforme** (`tools/springs`): stiffness `k = (2π / response)²` (masa 1), damping ratio `ζ = damping`.
- Web: unaprijed generirani CSS `linear()` stringovi
- SwiftUI: `.spring(response:dampingFraction:)`
- Compose: `spring(dampingRatio = ζ, stiffness = k)`

### 6.3 Imenovani efekti (zajednički rječnik; `animation-vocabulary`)

| Efekt | Opis |
|---|---|
| **Press** | scale 0,97, 90 ms, snappy povratak |
| **Lift** | tile na hover: `translateY(-2px)` + prijelaz na `elev.2`, 140 ms |
| **Rise** | ulazak: opacity 0→1, y 8→0, `ease.out`, 200 ms |
| **Bloom** | popover i meni: scale 0,96→1 iz točke okidača + opacity, 140 ms |
| **Glide** | reorder tabova i tileova (FLIP), smooth spring |
| **Morph** | Shield brojač: vertikalni "roll" znamenki (tabular), bouncy samo kad brojač skoči s 0 |
| **Dissolve** | promjena teme: View Transition cross-fade 200 ms (progresivno; fallback je trenutna promjena) |
| **Peek** | mobitel: long-press link preview, smooth spring + haptika |
| **Sleep** | tab koji zaspi: favicon prelazi u 60 % opacity + mali "z" badge, 300 ms |

### 6.4 Koreografije (`choreograph`)
- **New Tab ulazak** (≤ 420 ms): logo Rise (0 ms) → privacy chipovi Rise, stagger 28 ms → polje pretrage je **odmah** interaktivno (bez kašnjenja fokusa) → tileovi Rise, stagger 24 ms, najviše 8. Vijesti se ne animiraju.
- **Otvaranje taba (desktop):** novi tab raste iz širine 0 (transform scaleX na wrapperu + clip), susjedi Glide, sadržaj se pojavljuje kad je prvi paint gotov. **Zatvaranje:** tab Dissolve 120 ms, susjedi Glide; širina se zaključava dok je kursor iznad tab stripa (kao u Chromeu) da tab ne pobjegne ispod miša.
- **Fokus omniboxa:** polje dobiva fokus prsten, a popover s prijedlozima Bloom odozgo, 140 ms.
- **Tab switcher (mobitel):** zoom iz trenutne kartice u grid, smooth spring. Zatvaranje kartice swipeom ima fizikalnu brzinu.

---

## 7. Ikonografija
- Linijske ikone, potez **1,5 px**, grid 20 px (desktop) i 24 px (mobitel), zaobljeni krajevi. Baza je **Lucide** (ISC), vlastite ikone za Shield, Aura i Sleep.
- Ikone kategorija vijesti su vlastite linijske, **ne emoji**.
- Faviconi u speed dialu: originalna boja na `surface.sunken`. Ako favicon ne postoji, prikazuje se monogram u Aura fillu.

### Ikona aplikacije (smjer)
- Bijela squircle pločica (light) i `#17181E` pločica (dark varijanta na iOS 18+). Znak ima meki **conic Ember gradijent**.
- Oblik je **originalan**. Ideje za Fazu 0.3 (vlasnik bira):
  - (a) "orbit": jedan prsten s otvorom kroz koji prolazi manja kugla
  - (b) apstraktno "S" od dva luka različitih radijusa
  - (c) "aura kap": krug s pomaknutom unutarnjom svjetlinom
- Nikako dva jednaka kruga koji se preklapaju.
- Exporti: Windows `.ico` (16–256), Android adaptive (foreground, background, **monochrome** za themed icons), iOS 1024 + dark + tinted.

---

## 8. Komponente (stanja su obavezna: default, hover, press, focus-visible, disabled, loading)

| Komponenta | Ključni detalji |
|---|---|
| **Tab (desktop)** | favicon 16, naslov `type.label` s fade maskom umjesto "…", gumb za zatvaranje na hover/aktivnom, indikator zvuka i spavanja. Aktivni tab ima `surface.canvas` i spaja se s toolbarom. |
| **Omnibox** | Stanja: idle (prikazuje eTLD+1 naglašeno), focus (puni URL, selektiran), tipkanje (prijedlozi), sigurno/nesigurno/interno (`scyde://` s Aura točkom), učitavanje (tanka progress linija ispod, Ember→accent). Pillovi "Popup blokiran" i Shield desno. |
| **Shield gumb i panel** | Ikona štita + mali brojač (Morph). Panel (Bloom): veliki prekidač za stranicu, brojke (blokirano, ušteda), razina, Zen, link na detalje. |
| **Speed dial tile** | `surface.sunken`, radius 22, ikona 48, labela `type.caption`. Lift na hover, kontekstni meni (Uredi, Ukloni, Prikvači). |
| **Privacy chip** | ikona + labela + živa brojka; `surface.sunken` ili transparentan s hairlineom. |
| **Polje pretrage (New Tab)** | 56 px pill, ikona lupe, placeholder `text.tertiary`, ikone za glas i sliku, **Ember CTA** pill (`ember.700`, bijeli tekst). |
| **Chip filtera (vijesti)** | odabrani: Aura fill + tamni tekst (light) / pastel tekst na `sunken` (dark). |
| **Kartica vijesti** | `surface.raised`, slika 16:9 s radiusom 16 unutar kartice, izvor i vrijeme `type.caption`. |
| **Rail (desktop)** | krugovi od 44, aktivni ima Aura ring; tooltip desno (fast). Postavke na dnu. |
| **Meni / popover** | `surface.raised`, `elev.2`, radius 14, stavke visoke 32, prečaci desno u `Kbd`. |
| **Toast** | dno-centar (desktop), iznad donje trake (mobitel); jedna radnja; auto-dismiss 4 s, pauza na hover. |
| **Sheet (mobitel)** | detenti (medium/large), grabber, smooth spring, scrim 40 %. |
| **Donja traka (mobitel)** | omnibox pill + 4 akcije; skriva se pri scrollu prema dolje, a vraća pri scrollu prema gore ili dodiru na dnu. |
| **Tab switcher (mobitel)** | grid 2 stupca, kartice sa snapshotom, swipe za zatvaranje, "+" FAB s Ember gradijentom. |

---

## 9. Teme i prilagodba

Root atributi određuju temu, a komponente koriste samo semantičke varijable:
`data-theme="light|dark"` · `data-accent="sky|periwinkle|lavender|mint|ember|onyx|custom"` · `data-density="comfortable|compact"` · `data-motion="full|reduced"`

Ključne varijable: `--accent-fill`, `--accent-ink`, `--accent-on-fill`, `--accent-ring` te površine i tekst iz §2.

- **Adaptivni tint (opcija):** chrome uzima `theme-color` stranice ili dominantnu boju favicona, miješa je 8–12 % u `surface.chrome` i automatski provjerava kontrast teksta. Ako provjera padne, tint se ne primjenjuje.
- **Pozadina New Taba:** Aura gradijent (4 predloška), puna boja ili vlastita slika (lokalno skalirana; zatamnjenje 0–40 % za čitljivost).

---

## 10. Pristupačnost
- Fokus prsten `2px solid var(--accent-ink)` (dark: `--accent-fill`) + `2px` offset u boji podloge. **Nikad** `outline: none` bez zamjene.
- Potpuna navigacija tipkovnicom na desktopu (tab strip, omnibox, paneli, meniji kao ARIA meni/listbox).
- Labele za čitače ekrana na svim ikonama. Shield brojač je `aria-live="polite"` s ograničenjem učestalosti.
- Poštuje se: Dynamic Type i font scale do 200 %, Reduce Motion, Reduce Transparency, Windows High Contrast (sistemske boje), Increase Contrast (jači hairlinei).

---

## 11. Da / Ne

**Da:**
- velik razmak
- jedan fokus po ekranu
- meke površine
- pill CTA
- hairline u darku
- stvarne brojke
- tišina dok korisnik čita

**Ne:**
- generički ljubičasto-plavi hero gradijenti
- slaganje glassmorphism slojeva
- teške sjene
- emoji kao ikone
- "AI iskrice"
- 3D blobovi
- više od dvije težine fonta
- tekst VELIKIM SLOVIMA
- centriranje svega po defaultu
- animacija svega što se miče

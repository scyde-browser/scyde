# Promptovi za Claude Code

Kopiraj u Claude Code redom. Svaki prompt je jedna sesija. Prije svakog se uvjeri da je prethodna faza zatvorena (CI zelen, `docs/STATUS.md` ažuran).

---

## 1. Kickoff: Faza 0 (temelji, dizajn, spikeovi)

```
Ti si glavni inženjer projekta SCYDE Browser. U repou su PLAN.md, CLAUDE.md, DESIGN.md i design/references/.

1. Pročitaj PLAN.md, CLAUDE.md i DESIGN.md u cijelosti te pogledaj sve tri slike u design/references/.
2. Napravi kratak plan rada za Fazu 0 (PLAN §11) kao checklistu u docs/STATUS.md.
3. Izvrši 0.1 (repo struktura iz PLAN §4, pinani toolchainovi, Renovate, skeleton CI workflowi iz PLAN §13, ADR predložak, docs/NETWORK.md) i zapiši ADR-0001 (strategija enginea iz PLAN §2).
4. Izvrši 0.2: pomoću skilla `impeccable` finaliziraj DESIGN.md i iz njega generiraj design/tokens (DTCG) + Style Dictionary build za CSS, Kotlin i Swift. Provjeri da kontrasti iz DESIGN.md prolaze (napravi tools/contrast-check). Pokreni `taste-skill` nad smjerom i `pick-ui-library` (constraint: chrome UI ≤ 80 KB gz) → ADR-0005.
5. Izvrši 0.3: tri ORIGINALNA koncepta ikone kao SVG (ne dva preklopljena kruga, vidi PLAN §16 i DESIGN §7) + tools/icon-export. Stani i pitaj me koji koncept biram.
6. Izvrši 0.4: prototipovi skillom `prototype` u design/prototypes/ (New Tab light/dark, desktop chrome s omnibox prijedlozima i Shield panelom, mobilni donji toolbar + tab switcher). Za svaki pokreni `impeccable` critique i popravi nalaze.
7. Izvrši 0.5: pripremi spikeove S1–S5 (PLAN §11, Faza 0) u spikes/ s CI workflowima koji grade i pokreću ono što se može automatizirati. Za ono što traži mene (npr. prijava u Google račun na uređaju) napiši točne korake testiranja u docs/STATUS.md.

Pravila: radi na grani phase/0-foundations, mali commitovi (Conventional Commits), otvori PR kad je 0.1–0.2 gotovo i drugi PR za 0.3–0.5. Odluke koje traže mene označi s [VLASNIK]. Ne prelazi na Fazu 1.
```

---

## 2. Nakon gateova G1–G5

```
Pročitaj docs/STATUS.md i rezultate spikeova. Za svaki gate (G1–G5) napiši ADR s mjerenjima i odlukom
(ADR-0002 Google prijava/engine, ADR-0003 Windows Shield opcija A/B/C, ADR-0004 GeckoView ekstenzija, ADR-0006 iOS rule liste, ADR-0007 Tauri multiwebview).
Ako neki gate nije prošao, predloži alternativu iz PLAN §2.3 s procjenom troška i stani dok ne odlučim.
Ako su svi prošli, obriši spikes/ (zadrži ADR-ove), zatvori Fazu 0 u STATUS.md i predloži plan Faze 1.
```

---

## 3. Predložak za svaku sljedeću fazu

```
Započni Fazu <N> prema PLAN.md §11. Pročitaj CLAUDE.md i docs/STATUS.md.
- Razbij fazu na zadatke u docs/STATUS.md (checklist), svaki s AC iz plana.
- Radi na grani phase/<N>-<naziv>; PR po logičkoj cjelini; CI mora biti zelen.
- Za UI: prvo `prototype`, zatim implementacija, zatim `impeccable` critique. Animacije idu kroz `animate`/`choreograph`, a `animation-performance` provjerava svaku.
- Za Windows i iOS koristi GitHub Actions (gh workflow run / gh run watch) i priloži screenshot artefakte u PR.
- Na kraju faze pokreni `break-ui` sa scenarijima iz PLAN §14.3 i popravi P1/P2 nalaze.
- Ažuriraj STATUS.md; odstupanja od plana idu u ADR; odluke za mene označi [VLASNIK].
```

---

## 4. Polish: Faza 6

```
Faza 6. Redom:
1) `find-animation-opportunities` nad svim platformama → popis.
2) `improve-animations` → plan, pa implementacija uz `choreograph` i `animation-performance`.
3) `review-animations` → kritika i popravci.
4) `remove-animations` → ukloni sve što ne služi (i dodaj power-saver mod).
5) `impeccable` audit + polish nad cijelim UI-jem; `taste-skill` nad New Tabom i onboardingom.
6) `awwwards-animations` koristi SAMO kao smjernice za New Tab hero / onboarding / reveal kod promjene teme; implementacija kroz CSS, WAAPI i linear() springove, bez GSAP-a.
7) `break-ui` na sve tri platforme.
Zatim unikatne značajke iz PLAN §11 Faza 6, redoslijedom koji potvrdim.
```

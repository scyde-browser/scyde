# Mobilni prototip (Faza 0.4)

Stanja: `?state=browse` (donji toolbar), `?state=tabs` (tab switcher). Screenshotovi u `shots/` (light/dark, 390×844).
Isti raspored vrijedi za Android (Compose) i iOS (SwiftUI); ovo je referenca za oba.

## Odluke
- Donja traka u zoni palca: omnibox pill 44 (domena centrirano, Shield brojač desno) + 5 radnji (natrag, naprijed, podijeli, kartice, više). Hit targeti 48 (Android) / 44 (iOS).
- Tab switcher: grid 2 stupca, kartice sa snapshotom, aktivna s accent ringom, uspavana na 60 %. Segment Kartice / Privatno.
- "+" je FAB s Ember gradijentom (DESIGN §8), "Zatvori sve" je tiši (secondary), "Gotovo" u accent inku.

## taste check: mobile
Generic tells found: none.
Scores: distinctiveness 4/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 5/5
Verdict: pass

## impeccable critique: mobile
Rendered: shots/browse-{light,dark}-390, tabs-{light,dark}-390
| # | P | Finding | Rule | Fix | Status |
|---|---|---|---|---|---|
| 1 | P1 | Desni stupac tab switchera izlazio je izvan ekrana, gumb ✕ odrezan | §4 mobile, hit target | `minmax(0,1fr)` za grid i karticu | ✅ |
| 2 | P2 | "Zatvori sve" bio je naglašen kao primarna radnja | §1 energija samo na radnji | `text.secondary` | ✅ |
| 3 | P3 | U darku su donji rubovi snapshot kartica slabo vidljivi | §5 dark dubina | provjeriti na OLED uređaju u Fazi 4/5 | otvoreno |
Verdict: ship

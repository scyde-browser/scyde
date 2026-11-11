# ADR-0008: Vizualni smjer "Otok" umjesto "Aure"

- **Status:** Prihvaćeno
- **Datum:** 2026-11-11
- **Odlučuje:** `[VLASNIK]` (izbor), Claude Code (razrada)
- **Veza:** PLAN §1 (R1 unikatan, R4 premium), PLAN §10, DESIGN.md v2.0, ADR-0005, ADR-0007

## Kontekst
Prvi vizualni smjer ("Aura", DESIGN v1) zadržao je klasičan raspored preglednika: tabovi i toolbar na vrhu, adresna traka, New Tab s logom, poljem i mrežom pločica. Vlasnik ga je odbio: "želim da je unikatan i nov, ne da liči na svaki drugi browser".

## Opcije
Tri smjera kao istraživački prototipovi (`design/prototypes/redesign/`, git `4922b7d`):
1. **A · Otok:** nema gornje trake; sve sučelje je jedan plutajući otok na dnu; New Tab je pitanje u koje se tipka.
2. **B · Kralježnica:** uredničko-tipografski; tabovi su okomiti naslovi u lijevoj kralježnici; New Tab je naslovnica.
3. **C · Tipke:** tabovi su numerirane tipke; prečaci su QWERTY tipkovnica.

## Odluka
**A · Otok** (vlasnik, 2026-11-11). DESIGN.md je prepisan u v2.0, a tokeni su prerađeni.

## Posljedice
- **Windows (ADR-0007):** nema chrome webviewa na vrhu. Otok je zaseban, unaprijed kreiran transparentni top-level prozor (isti mehanizam kao popoveri), a tab webviewi zauzimaju cijeli klijentski prostor.
  - Caption gumbi plutaju gore desno, a Snap Layouts ostaje otvoreno pitanje (ADR-0007).
  - Otok mora pratiti pomicanje i promjenu veličine prozora, što je dodatni posao u Fazi 3.
  - Pebble (sažimanje pri skrolu) traži događaje skrola iz tab webviewa preko bootstrap skripte, bez pristupa sadržaju stranice.
- **Mobitel:** donja traka iz plana postaje otok; geste ostaju (swipe po otoku mijenja tab, swipe gore otvara grid).
- **Fontovi:** Bricolage Grotesque i Geist Mono (OFL) ulaze u aplikacije. Pitanje licence za Aeonik (PLAN §17, br. 4) više nije relevantno.
- **Tokeni:** nova paleta papir i tinta, `island.*`, accent uloga `island`, default accent Ember. Contrast gate pokriva 204 para.
- **Prototipovi:** v1 prototipovi su zamijenjeni Otok prototipovima, a v1 ostaje u git povijesti.
- **Rizik:** neuobičajeni raspored može zbuniti korisnike naviknute na Chrome. Mitigacija: onboarding u Fazi 6 i opcija "otok uvijek pun" (bez Pebblea).

## Mjerenja / dokazi
Screenshotovi sva tri smjera: `design/prototypes/redesign/*/shots/` (git `4922b7d`); Otok prototipovi i critique: `design/prototypes/{newtab,desktop,mobile}/`.

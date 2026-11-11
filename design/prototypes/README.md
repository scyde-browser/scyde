# design/prototypes: smjer "Otok" (DESIGN.md v2.0)

Prototipovi se otvaraju u pregledniku (`file://`), na pravim tokenima (`pnpm tokens`) i i18n stringovima (`node design/prototypes/_shared/build-strings.mjs`).
Parametri: `?state=…`, `?theme=light|dark`, `?accent=ember|sky|…`, `?lang=hr|en`, `?clean` (bez switchera).

| Prototip | Stanja | Screenshotovi |
|---|---|---|
| `desktop/` (1440 × 900) | `newtab`, `browse`, `omnibox`, `tabs`, `shield` | `desktop/shots/` |
| `mobile/` (390 × 844) | `newtab`, `browse`, `tabs`, `shield` | `mobile/shots/` |

Screenshotovi: `node .claude/skills/prototype/shot.mjs design/prototypes/desktop/index.html --widths 1440 --height 900 --query state=tabs --tag tabs`.
v1 "Aura" prototipovi i istraživanje tri smjera su u git povijesti (`4922b7d`, ADR-0008).

## taste check: Otok
Generic tells found: none. Nema tab stripa, toolbara ni mreže pločica; nema gradijentnih pozadina ni stakla.
Scores: distinctiveness 5/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 5/5
Signature moves: otok kao jedini predmet sučelja, New Tab kao pitanje u koje se tipka, papir i tinta s inverznim otokom, mono za podatke, vlasnikov S znak u Emberu.
Verdict: pass

## impeccable critique: Otok (desktop + mobile)
Rendered: desktop {newtab, browse, omnibox, tabs, shield} × {light, dark}; mobile {newtab, browse, tabs, shield} × {light, dark}
Machines: contrast 204/204 AA; nijedna hardkodirana vrijednost u CSS-u prototipova (samo 0, 1px, %); 390 px bez horizontalnog prelijevanja (provjereno)
| # | P | Nalaz | Pravilo | Popravak | Status |
|---|---|---|---|---|---|
| 1 | P1 | 6 parova teksta ispod AA na novoj paleti (sunken, tertiary, Shield na svijetlom otoku) | §2, §10 | sunken `#EEEBE5`, tertiary `#66645F`, `island.shield` dark `#0B5E33` | ✅ |
| 2 | P2 | Opisi ispod Shield brojki različite visine | ritam | fiksno 2 reda | ✅ |
| 3 | P2 | Znak (gradijent na tinti) nestaje na svijetlom otoku u dark temi | §2.4 | znak kao Ember pločica s crnim S-om, iz tokena | ✅ |
| 4 | P3 | Ember accent i lime znak bili su dvije "energije" | §1 | vlasnik: logo prebačen u Ember | ✅ |
| 5 | P3 | Desktop New Tab ima puno praznog prostora ispod hinta | §1 tišina | namjerno; u Fazi 6 razmotriti vijesti kao drugi stupac kad su uključene | – |
Verdict: ship

# New Tab prototip (Faza 0.4)

Otvori `index.html` u pregledniku (radi s `file://`). Parametri: `?theme=dark`, `?accent=mint`, `?lang=en`, `?clean` (bez switchera).
Screenshotovi: `shots/` (light/dark × 1440/390), `node .claude/skills/prototype/shot.mjs design/prototypes/newtab/index.html`.

## Odluke
- Jedan fokus: polje pretrage je odmah fokusirano (fokus prsten u accent ringu), sve ostalo je tiše.
- Hero je centriran namjerno; tileovi i vijesti su poravnati na mrežu od 1120 px.
- Privacy chipovi s hairlineom (ne sunken) da ne konkuriraju polju pretrage; brojevi `tabular-nums`.
- Vijesti su **isključene po defaultu** (R6, PLAN §10): umjesto feeda je kartica s objašnjenjem i "Odaberi izvore".
- Monogram tile (kad nema favicona) dobiva Aura fill izveden iz hosta, stabilan po stranici. Pravi faviconi dolaze u Fazi 2.
- Ulazak: Rise s staggerom (logo → chipovi → tileovi, najviše 8), polje je interaktivno odmah.

## taste check: New Tab
Generic tells found: none (nema hero gradijenta, glassa, emojija; mesh je jedan i niske zasićenosti).
Scores: distinctiveness 4/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 4/5
Signature moves present: Aura mesh "dawn", Ember samo na CTA i znaku, stvarne Shield brojke, Aura S znak.
Changes made: monogram tileovi su bili identični (tell "jednake kartice") → Aura fill po hostu.
Verdict: pass

## impeccable critique: New Tab
Rendered: shots/light-1440, light-390, dark-1440, dark-390
Machines: contrast 168/168 AA; hard-coded values u CSS-u 0 (samo 0, 1px hairline, %)
| # | P | Finding | Rule | Fix | Status |
|---|---|---|---|---|---|
| 1 | P1 | Na 390 px stranica je bila široka 449 px (red chipova širio grid stupac) | DESIGN §4, mobile | `grid-template-columns: minmax(0,1fr)` u heru | ✅ |
| 2 | P2 | "1.284 Blokirano danas": veliko slovo usred rečenice | §11 tipografija, i18n | novi ključevi `newtab.privacy.*_chip` | ✅ |
| 3 | P2 | Svi monogrami iste boje → monotona mreža | taste, §7 monogram u Aura fillu | fill po hostu | ✅ |
| 4 | P3 | Placeholder se na 390 px reže ("adres…") | čitljivost | kraći mobilni placeholder u Fazi 2 | otvoreno |
| 5 | P3 | Wordmark "SCYDE" je verzalom | §11 "bez VELIKIH SLOVA" | prihvaćeno: to je logotip, ne tekst sučelja | – |
Verdict: ship after P3 (prototip odobriti)

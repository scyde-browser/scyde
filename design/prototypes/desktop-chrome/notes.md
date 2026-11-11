# Desktop chrome prototip (Faza 0.4)

Stanja: `?state=shield` (Shield panel otvoren), `?state=omnibox` (fokus + prijedlozi). Screenshotovi u `shots/` (light/dark, 1440×900).
U pravom buildu popoveri su zasebni, unaprijed kreirani prozori (CLAUDE.md); ovdje su apsolutno pozicionirani.

## Odluke
- Tab strip 44 / tab 34 / toolbar 48 / omnibox 34 / rail 72 (tokeni `size.chrome.*`). Windows caption gumbi (46 px), ne macOS semafor.
- Aktivni tab ima `surface.canvas` i spaja se s toolbarom; naslov se gasi fade maskom umjesto "…".
- Uspavani tab: favicon na 60 % + mjesec (Sleep), tooltip "Spava · ušteđeno 212 MB".
- Omnibox u mirovanju naglašava eTLD+1 (`index.hr`), put je sekundaran (PLAN §7). U fokusu se popover s prijedlozima spaja s poljem (Bloom odozgo), prvi red je inline dovršetak s `Enter`.
- Shield gumb: ikona štita u `status.shield` + brojač (tabular). Panel: veliki prekidač, dvije stvarne brojke, razina, Zen, link na detalje.
- Lijevi rail: web paneli kao krugovi 44, aktivni ima Aura ring, postavke na dnu.

## taste check: desktop chrome
Generic tells found: none (nema Chrome sivih tabova, nema emojija, nema glassa).
Scores: distinctiveness 4/5, restraint 5/5, specificity 4/5, craft 4/5, coherence 5/5
Signature moves: Aura ring na railu, Shield brojke kao junaci panela, mekane sunken površine, hairline u darku.
Verdict: pass

## impeccable critique: desktop chrome
Rendered: shots/shield-light-1440, shield-dark-1440, omnibox-light-1440, omnibox-dark-1440
Machines: contrast 168/168 AA; hard-coded vrijednosti 0 (novi tokeni: `size.chrome.caption-button`, `size.popover.*`, `size.toggle.*`)
| # | P | Finding | Rule | Fix | Status |
|---|---|---|---|---|---|
| 1 | P2 | "Popup blokiran" pill imao je ikonu ✕, čita se kao "zatvori" | §8 jasnoća radnje | ikona prozora/tabova | ✅ |
| 2 | P2 | Opisi ispod Shield brojki različite visine (1 vs 2 reda) | ritam | fiksna visina od 2 reda | ✅ |
| 3 | P3 | Mjesec preko favicona uspavanog taba je sitan na 100 % | §6.3 Sleep | u Fazi 3 provjeriti na 125/150 % skaliranju | otvoreno |
| 4 | P3 | Double ring na aktivnom rail gumbu u lightu je težak | §8 rail | razmotriti 1,5 px ring u Fazi 3 | otvoreno |
Verdict: ship

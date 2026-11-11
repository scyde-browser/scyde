# Redizajn: tri smjera (2026-11-11)

Vlasnik je odbio prvi vizualni smjer ("ne smije ličiti na svaki drugi browser"). Ovo su tri radikalno različita smjera; vlasnik bira jedan, a izabrani postaje novi DESIGN.md, tokeni i puni prototipovi.

| Smjer | Ideja | Što ga čini drugačijim |
|---|---|---|
| **A · Otok** (`a-otok/`) | Nema gornje trake. Sav chrome je jedan plutajući otok na dnu: znak, natrag, adresa, Shield, tabovi kao složeni krugovi. | Stranica ide do ruba ekrana. New Tab je rečenica "Kamo danas?" u koju se tipka, a "nedavno" je popis, ne pločice. |
| **B · Kralježnica** (`b-kraljeznica/`) | Uredničko-tipografski. Tabovi su okomiti naslovi u lijevoj "kralježnici", a adresa je breadcrumb u mono fontu. | New Tab je naslovnica: serifni naslov, numerirano kazalo stranica, Shield kao ledger s velikim brojkama. Papir i tinta umjesto sive. |
| **C · Tipke** (`c-tipke/`) | Sve je tipkovnica. Tabovi su numerirane tipke 1–9, adresa je naredbeni redak `›`. | Prečaci su raspoređeni kao QWERTY tipke: pritisneš Q i otvara se index.hr. |

Parametri: `?state=newtab|browse`, `?theme=light|dark`. Fontovi (OFL) su lokalno u `../_shared/fonts/`: Bricolage Grotesque, Fraunces, Geist, Geist Mono.
Napomena: ovo su istraživački prototipovi s vlastitim tokenima na vrhu datoteke i tekstom inline; izabrani smjer ide kroz `design/tokens` i i18n.

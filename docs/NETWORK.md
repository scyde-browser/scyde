# NETWORK.md: mrežni pozivi samog preglednika

Pravilo (CLAUDE.md, pravilo 2): **svaki** mrežni poziv koji preglednik radi sam od sebe (ne zbog stranice koju je korisnik otvorio) mora biti ovdje upisan prije mergea. Bez telemetrije, analitike i SDK-ova trećih strana.

| # | Poziv | Odredište | Kada | Podaci koji odlaze | Zadano | Isključivo | Platforme | Uveden u |
|---|---|---|---|---|---|---|---|---|
| N1 | Provjera filter bundlea | GitHub Releases `scyde-browser/scyde` (`filters-latest/manifest.json`) | 1× dnevno, `If-None-Match` (ETag) | IP, User-Agent; bez identifikatora | uključeno | da (Postavke → Shield) | sve | planirano, Faza 1 |
| N2 | Provjera ažuriranja aplikacije | GitHub Releases (Tauri updater manifest) | pri startu + 1× dnevno | IP, verzija aplikacije, arhitektura | uključeno | da | Windows | planirano, Faza 3 |
| N3 | Prijedlozi pretrage | odabrana tražilica (default DuckDuckGo `[VLASNIK]`) | dok korisnik tipka u omnibox | upisani tekst | uključeno (opt-out) | da | sve | planirano, Faza 3–5 |
| N4 | Top Stories (RSS) | izvori koje bira `[VLASNIK]` | otvaranje New Taba, keš ≥ 30 min | IP, User-Agent | **isključeno (opt-in)** | – | sve | planirano, Faza 2 |
| N5 | Faviconi za speed dial | stranica tilea (`/favicon.ico`, `<link rel=icon>`) | dodavanje tilea, osvježavanje 1× tjedno | IP, User-Agent | uključeno | da | sve | planirano, Faza 2 |
| N6 | Crash izvješća | self-hosted endpoint `[VLASNIK]` | samo nakon pada, uz potvrdu | stack trace, verzija | **isključeno (opt-in)** | – | sve | planirano, Faza 7 |

Napomene:
- Engine (WebView2, GeckoView, WebKit) ima vlastite pozadinske pozive (npr. WebView2 runtime update, Safe Browsing). Oni se popisuju i, gdje je moguće, gase u fazama 3–5. Politika Safe Browsing / SmartScreen ide u ADR `[VLASNIK]` (PLAN §7).
- Status "planirano" znači da poziv još ne postoji u kodu.

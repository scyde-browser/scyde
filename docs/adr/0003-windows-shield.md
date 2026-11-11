# ADR-0003: Shield na Windowsu (WebView2): opcija A, DNR ekstenzija

- **Status:** Prihvaćeno (Faza 0 zatvorena 2026-11-11)
- **Datum:** 2026-11-11
- **Odlučuje:** Claude Code, na temelju spikea S2; potvrda `[VLASNIK]` pri zatvaranju Faze 0
- **Veza:** PLAN §5.4, gate **G2**, `spikes/s5-tauri`, `spikes/s2-dnr`

## Kontekst
WebView2 nema ugrađeni ad blocker. G2 traži ≤ 3 % (cilj 0) dodatnog vremena na medijan učitavanja i ≥ 95 % blokiranih fixture zahtjeva.

## Opcije
1. **A: MV3 ekstenzija s DNR statičkim rulesetovima** (`browser_extensions_enabled` + `extensions_path` u Tauriju). Blokiranje unutar mrežnog sloja Chromiuma, bez IPC-a prema hostu.
2. **B: `WebResourceRequested` + adblock-rust u hostu.** Svaki zahtjev ide na UI thread hosta (dodatni process hop).
3. **C: CDP `Network.setBlockedURLs`.** Samo kao mjerenje.

## Odluka
**Opcija A za mrežno blokiranje**, uz host za ono što DNR ne može:
- kozmetiku i scriptlete kroz bootstrap skriptu (`AddScriptToExecuteOnDocumentCreated`, PLAN §5.4)
- `NavigationStarting` interstitial za glavni okvir
- per-site iznimke kroz dinamička DNR pravila

**Opcija B ostaje rezerva**, ako DNR ograničenja postanu problem. Kôd za B je iz spikea poznat i radi.

## Posljedice
- `shield-compiler` (Faza 1) mora proizvoditi DNR rulesetove ≤ 30k pravila po rulesetu. Spike je stavio 106k pravila u 4 rulesetova (globalni pool), i to WebView2 prihvaća.
- Brojač blokiranih na Windowsu dolazi iz `declarativeNetRequestFeedback` (`onRuleMatchedDebug` nije za produkciju), pa se u Fazi 3 bira `getMatchedRules` po tabu ili brojanje u hostu kroz B samo za statistiku.
- Svi webviewi (chrome, tabovi, popoveri) moraju imati iste opcije environmenta (`browser_extensions_enabled`), inače nisu na istom environmentu.
- `extensions_path` učitava **podmape** kao ekstenzije.
- Opcija C nije mjerena: A i B su obje prošle G2, a C nema prednosti za produkciju.

## Mjerenja / dokazi
CI run 43019194222 (windows-latest), medijan od 3 učitavanja, PageLoad Started → Finished:

| Stranica | Bez blokiranja | A (DNR) | B (host) |
|---|---|---|---|
| qa/adblock (mrežni dio 6/6) | 28 ms · 2/11 | 26 ms · **8/11** | 25 ms · **8/11** |
| index.hr | 1102 ms | 229 ms | 226 ms |
| 24sata.hr | 1172 ms | 218 ms | 219 ms |
| jutarnji.hr | 1218 ms | 945 ms | 1007 ms |
| youtube.com | 1158 ms | 831 ms | 825 ms |
| edition.cnn.com | 900 ms | 726 ms | 751 ms |

- Fixture: obje opcije blokiraju **6/6 mrežnih** zahtjeva (100 %) i propuštaju iznimku. Ostala 3 boda su kozmetika, koju radi bootstrap skripta (nije dio G2 mrežnog kriterija).
- Dodatno vrijeme: **negativno** na svim stranicama (blokiranje ubrzava učitavanje) → G2 ✅.
- DNR konverzija (spike, ograničena): 139k linija → 106k pravila, 4 ruleseta; odbačeno je kozmetika (24,8k), `$popup` (3k) i ostalo (~0,3k).

> Spike kod je obrisan pri zatvaranju Faze 0. Izvorni kod i CI workflow su u git povijesti na commitu `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

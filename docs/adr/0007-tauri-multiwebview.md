# ADR-0007: Windows ljuska: Tauri 2 multiwebview (feature `unstable`)

- **Status:** Prihvaćeno (Faza 0 zatvorena 2026-11-11)
- **Datum:** 2026-11-11
- **Odlučuje:** Claude Code, na temelju spikea S5
- **Veza:** PLAN §9.1, gate **G5**, `spikes/s5-tauri`

## Kontekst
Desktop ljuska drži chrome webview, po jedan webview za svaki tab i popovere u zasebnim prozorima, sve na jednom WebView2 environmentu. G5 traži da prebacivanje taba i prikaz popovera budu ≤ 16 ms i da radi snap layout.

## Opcije
1. **Tauri 2.12.5 multiwebview** (`unstable`): child webviewi u jednom prozoru, show/hide za tabove.
2. Jedan webview i navigacija između tabova: gubi stanje taba, ne dolazi u obzir.
3. Vlastiti host direktno na webview2-com, bez Taurija: više koda, bez prednosti u mjerenjima.

## Odluka
**Opcija 1**, uz pinane verzije (tauri =2.12.5, tauri-build =2.7.4, webview2-com =0.39.1) i escape hatch `with_webview` za COM API-je (dokazan u spikeu kroz `WebResourceRequested`).

## Posljedice
- Tauri 3 je u alfi; prelazak tek kad bude stabilan (Renovate grupa `tauri` ostaje ručna).
- Svaki webview mora dijeliti iste environment opcije (vidi ADR-0003).
- **Kreiranje taba je skupo:** 20 webviewa = 3,7–4,8 s, oko 185–240 ms po tabu. Lijeno kreiranje tabova (PLAN §8.2) je **obavezno**, a za Ctrl+T treba unaprijed zagrijan (pre-warmed) tab webview da bi New Tab bio ≤ 120 ms.
- **Snap Layouts (otvoreno):** caption gumbi su u chrome webviewu, pa child HWND prima miš i `WM_NCHITTEST → HTMAXBUTTON` na glavnom prozoru tamo ne djeluje. Kandidati za Fazu 3:
  - (a) tanki nativni overlay prozor iznad maximize gumba koji vraća `HTMAXBUTTON`
  - (b) `tauri-plugin-decorum`
  - (c) nativni caption gumbi

  Provjera je ručna na Windows 11.

## Mjerenja / dokazi
CI runovi 43017487417 i 43019194222 (windows-latest):

| Metrika | Run 1 | Run 2 | Cilj |
|---|---|---|---|
| Prebacivanje taba (host: show + hide + focus), p95 / max | 0,16 / 0,86 ms | 0,39 / 1,36 ms | ≤ 16 ms ✅ |
| Popover show (unaprijed kreiran), p95 / max | 0,10 / 0,14 ms | 0,10 / 0,16 ms | ≤ 16 ms ✅ |
| 20 tabova kreirano / učitano | 3,7 s · 20/20 | 4,8 s · 20/20 | – |

Mica i vlastiti titlebar: screenshot `s5.png` u artefaktu `s5-s2-results`. Vizualna frame provjera je ručna.

> Spike kod je obrisan pri zatvaranju Faze 0. Izvorni kod i CI workflow su u git povijesti na commitu `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

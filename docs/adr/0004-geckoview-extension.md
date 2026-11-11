# ADR-0004: Shield na Androidu: GeckoView built-in ekstenzija s WASM engineom

- **Status:** Predloženo (čeka zatvaranje Faze 0)
- **Datum:** 2026-11-11
- **Odlučuje:** Claude Code, na temelju spikea S3
- **Veza:** PLAN §5.4, gate **G3**, `spikes/s3-geckoview`, CLAUDE.md pravilo 11 (GeckoView i adblock-rust odobreni kao biblioteke)

## Kontekst
Na Androidu Shield radi kao built-in WebExtension u GeckoViewu (MV2 `webRequestBlocking`), s adblock-rust engineom kompajliranim u WASM. G3 traži da ekstenzija blokira fixture, da je WASM odluka p99 < 200 µs i da interne stranice razgovaraju s aplikacijom.

## Opcije
1. **Built-in MV2 ekstenzija + WASM engine** (plan).
2. Samo GeckoView ETP (Strict) bez vlastitog enginea: premalo pokrivenosti (nema EasyList / uBO pravila ni kozmetike).
3. Engine u Kotlinu/Rustu kroz native messaging za svaku odluku: IPC po zahtjevu, sporije i složenije.

## Odluka
**Opcija 1**, uz ETP Strict i Total Cookie Protection kao dodatni sloj.

## Posljedice
- **GeckoView 160** traži compileSdk **37.2**, AGP 9.5+ i Kotlin 2.4 (kotlin-stdlib 2.4.21 dolazi s GeckoViewom). Android projekt u Fazi 5 kreće od toga.
- **Gradnja enginea:** iz 139k linija traje 1,7 s pri startu (uklj. dohvat lista). U Fazi 1 učitava se serijalizirani `engine.dat`.
- **Do spremnosti enginea:** zahtjevi prolaze bez filtera, pa aplikacija u spikeu čeka poruku "ready" prije prvog učitavanja. U produkciji to znači: prva navigacija čeka engine, ili se engine učita iz `engine.dat` prije prikaza.
- **Kozmetika:** content script traži selektore od backgrounda (`url_cosmetic_resources` + `hidden_class_id_selectors`). Radi, uklj. generička i atributna pravila.
- **Veličina:** WASM je 2,8 MB (844 KB gz) unutar APK-a, u budgetu (≤ 10 MB našeg koda).

## Mjerenja / dokazi
CI run 43017809149 / 43019194222 (`s3-geckoview-app`, emulator API 34 x86_64, GeckoView 160.0):
- Fixture: **10/11**. Svih 6 mrežnih je blokirano, iznimka je propuštena, a generička i atributna kozmetika rade. Host-specifična kozmetika je ciljala 127.0.0.1, a emulator koristi 10.0.2.2; pravilo je dodano.
- WASM u SpiderMonkeyju: **p99 40 µs**, p90 20 µs, max 240 µs. Mjereno kao 3000 zahtjeva × 50 ponavljanja, jer je rezolucija timera u ekstenziji 1 ms → G3 ✅ (< 200 µs).
- WASM u V8 (CI proxy, 50k odluka): p50 5,5 µs, p99 14,8 µs.
- Interna stranica (`moz-extension://…/internal.html`, prikazuje se kao `scyde://newtab`) → `runtime.sendNativeMessage` → Kotlin `MessageDelegate` → odgovor natrag: ✅.
- UA: `Mozilla/5.0 (Android 14; Mobile; rv:160.0) Gecko/160.0 Firefox/160.0` (relevantno za G1).

# Spike S3: GeckoView → G3

**Pitanje:** radi li built-in ekstenzija u GeckoViewu s `webRequestBlocking`, koliko je brza WASM odluka (p99 < 200 µs) i mogu li interne stranice razgovarati s aplikacijom (native messaging)?

| Dio | Gdje | Status |
|---|---|---|
| WASM engine (adblock-rust → wasm32) + benchmark | `wasm-engine/` | ✅ lokalno i u CI-ju (`spikes.yml` → `s3-wasm`) |
| Android app s GeckoViewom + built-in ekstenzija + native messaging | `app/` | u izradi |

## Rezultat WASM dijela (lokalno, Node 22 / V8, 2026-11-11)
EasyList + EasyPrivacy + uBO filters (139.478 linija pravila), 50.000 odluka, sintetski korpus (30 % ad hostovi):

| Metrika | Vrijednost | Cilj |
|---|---|---|
| p50 | 8,7 µs | – |
| p99 | **28,7 µs** | < 200 µs ✅ |
| p99.9 | 121,2 µs | – |
| Build enginea iz lista | 187 ms | (u Fazi 1 se učitava `engine.dat`) |
| `engine.dat` | ≈ 6,34 MB | – |
| WASM | 2,81 MB (844 KB gz) | ≤ 10 MB našeg koda u APK-u ✅ |
| Fixture (`qa/lists/fixture.txt`) | 8/8 mrežnih odluka, kozmetika točna | 100 % ✅ |

V8 je proxy; konačni broj za G3 mjeri se u SpiderMonkeyju unutar GeckoView aplikacije.

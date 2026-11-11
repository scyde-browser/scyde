# ADR-0006: Shield na iOS-u: WKContentRuleList, više lista, kompajliranje u pozadini

- **Status:** Prihvaćeno (Faza 0 zatvorena 2026-11-11)
- **Datum:** 2026-11-11
- **Odlučuje:** Claude Code, na temelju spikea S4
- **Veza:** PLAN §5.2, §5.4, gate **G4**, `spikes/s4-ios-rulelists`

## Kontekst
WKWebView blokira kroz `WKContentRuleList`, uz limit od 150k pravila po listi. G4 traži da se ≥ 150k pravila podijeljenih u više lista kompajlira, da keširani load bude < 100 ms i da fixture bude blokiran.

## Opcije
1. **adblock-rust `content-blocking` konverzija → više lista ≤ 150k**, kompajlira se jednom i kešira u `WKContentRuleListStore` (plan).
2. Samo jedna lista s prioritetnim pravilima: gubi se pokrivenost.
3. JS presretanje u stranici: ne vidi mrežni sloj, sporo i zaobilazivo.

## Odluka
**Opcija 1**, uz pravila za Fazu 1 i Fazu 4 (vidi Posljedice).

## Posljedice
- **Kompajliranje je sporo:** 83 s za 150k pravila i 133 s za 114k pravila na CI simulatoru. Zato:
  - kompajlira se **samo u pozadini**, nikad na main threadu i nikad na putu do prvog prikaza
  - aplikacija isporučuje **unaprijed kompajliran** snapshot ili kreće s manjom "core" listom
  - nakon updatea bundlea, nova lista se kompajlira u pozadini pa se atomski zamijeni
  - na stvarnom uređaju vrijeme se mjeri u Fazi 4
- `shield-compiler` mora filtrirati regexe koje WebKit ne podržava (`|`, `{}`, `\d`/`\w`/`\s`/`\b`, lookaround, ne-ASCII, `$` u sredini, doslovni `\^`) i prijaviti ih kao odbačene. Spike odbacuje 388 od 132k pravila (0,3 %). Golden test u Fazi 1: sve izlazne liste se kompajliraju na simulatoru.
- adblock-rust 0.13.5 pretvara `||<IP>^*` u regex s doslovnim `\^` → odbacuje se (vidi spike README).
- Brojač blokiranih na iOS-u je procjena "≈" (WebKit ne prijavljuje blokirane zahtjeve).

## Mjerenja / dokazi
CI run na `4e288eb` (`s4-ios-rulelists`, macos-latest, iOS simulator):

| Metrika | Vrijednost | Cilj |
|---|---|---|
| Ukupno pravila | **263.790** u 2 liste (150.000 + 113.790) | ≥ 150k, ≤ 150k po listi ✅ |
| Kompajliranje | 82,5 s + 132,5 s | – (pozadina) |
| Lookup jedne liste | 4,7 ms / 6,2 ms | – |
| Keširani load svih lista | **6,0 ms** | < 100 ms ✅ |
| Fixture (bundlean, `file://`) | 10/11: svih 6 mrežnih, iznimka, generička i atributna kozmetika | ✅ |

Host-specifična kozmetika (`127.0.0.1##`) ne vrijedi za `file://` i nije dio mrežnog G4 kriterija. Stvarne liste daju 131.891 WebKit pravilo; stres test ih duplicira s jedinstvenim markerom.

> Spike kod je obrisan pri zatvaranju Faze 0. Izvorni kod i CI workflow su u git povijesti na commitu `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

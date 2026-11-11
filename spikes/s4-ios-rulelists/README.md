# Spike S4: iOS rule liste → G4

**Pitanje:** može li WebKit kompajlirati ≥ 150k pravila podijeljenih u više `WKContentRuleList`a, koliko traje kompajliranje i keširani load, i blokira li se fixture?

| Dio | Gdje | Kako se pokreće |
|---|---|---|
| Konverter (adblock-rust `content-blocking`) | `converter/` | `cargo run --release -- --out <dir> [--repeat 2] <liste…>` |
| iOS test (XCTest na simulatoru) | `RuleListSpike/` | `.github/workflows/spikes.yml` → job `s4-ios-rulelists` (macOS) |

Lokalno izmjereno (Linux, konverter): EasyList + EasyPrivacy + uBO filters = **132.268** WebKit pravila (14,3 MB JSON). Za stres test G4 koristi se `--repeat 2` (264.536 pravila u 2 liste ≤ 150k).

**G4 kriteriji:** liste se kompajliraju · keširani load svih lista < 100 ms · fixture mrežni zahtjevi blokirani, iznimka propuštena.
Rezultati: CI artefakt `s4-results` (`SCYDE_RESULT` linije iz XCTesta).

Napomena: brojač blokiranih na iOS-u je procjena (WebKit ne prijavljuje blokirane zahtjeve), PLAN §5.4.

## Nalazi
- **Konverzija `||<IP>^*` pravila:** adblock-rust 0.13.5 `content-blocking` pretvara `||127.0.0.1^*/x` u regex s doslovnim `\^`, koji WebKit nikad ne matcha. Fixture zato koristi `/ad-network/frame.html$subdocument`. Za Fazu 1 (`shield-compiler`): golden test koji provjerava da nijedno iOS pravilo ne sadrži `\^`, a takva pravila se prijavljuju kao odbačena.

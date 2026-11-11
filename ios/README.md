# ios

| Path | What |
|---|---|
| `SCYDE/` | the app: SwiftUI + WKWebView, xcodegen (`project.yml`). The CI `ios` → `app` job builds it, takes simulator screenshots and produces an unsigned `.ipa` for Feather |
| `ShieldRuleLists/` | Swift package test: the bundle's WKContentRuleLists compile and block the qa fixture |

The app's generated inputs (pages, filters, tokens, strings, icons, ScydeCore) are listed at the top of `SCYDE/project.yml`.

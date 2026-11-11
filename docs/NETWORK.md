# NETWORK.md: network calls made by the browser itself

Rule (CLAUDE.md, rule 2): **every** network call the browser makes on its own (not because of a page the user opened) must be listed here before merge. No telemetry, analytics or third-party SDKs.

| # | Call | Destination | When | Data sent | Default | Can be disabled | Platforms | Introduced in |
|---|---|---|---|---|---|---|---|---|
| N1 | Filter bundle check | GitHub Releases `scyde-browser/scyde` (`filters-latest/manifest.json`) | 1× daily, `If-None-Match` (ETag) | IP, User-Agent; no identifiers | on | yes (Settings → Shield) | all | planned, Phase 1 |
| N2 | App update check | GitHub Releases (Tauri updater manifest) | at startup + 1× daily | IP, app version, architecture | on | yes | Windows | planned, Phase 3 |
| N3 | Search suggestions | selected search engine (default DuckDuckGo `[OWNER]`) | while the user types in the omnibox | typed text | on (opt-out) | yes | all | planned, Phase 3–5 |
| N4 | Top Stories (RSS) | sources chosen by `[OWNER]` | opening New Tab, cache ≥ 30 min | IP, User-Agent | **off (opt-in)** | – | all | planned, Phase 2 |
| N5 | Favicons for speed dial | the tile's site (`/favicon.ico`, `<link rel=icon>`) | adding a tile, refresh 1× weekly | IP, User-Agent | on | yes | all | planned, Phase 2 |
| N6 | Crash reports | self-hosted endpoint `[OWNER]` | only after a crash, with confirmation | stack trace, version | **off (opt-in)** | – | all | planned, Phase 7 |

Notes:
- The engine (WebView2, GeckoView, WebKit) makes its own background calls (e.g. WebView2 runtime update, Safe Browsing). These are listed and, where possible, disabled in Phases 3–5. The Safe Browsing / SmartScreen policy goes into an ADR `[OWNER]` (PLAN §7).
- Status "planned" means the call does not exist in the code yet.

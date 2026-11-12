# SCYDE privacy policy

*Draft for v0.2.0 (2026-11-11). The final text and the contact address are `[OWNER]`.*

**Short version:** SCYDE has no ads, no telemetry and no account. What you do in the browser stays on your device.

## What SCYDE stores, on your device only
- history, bookmarks and top sites (SQLite in the app's data folder)
- settings (theme, accent, search engine, privacy options)
- Shield statistics (counters only, never a list of the sites you visited)
- the data websites store themselves (cookies, site storage), as in any browser

Nothing of this is sent to us or anyone else. Uninstalling the app deletes it (on Windows, choose "delete data" in the uninstaller).

## Network requests SCYDE makes on its own
Every request the browser makes by itself is listed in [`docs/NETWORK.md`](docs/NETWORK.md), including where it goes, what it sends and how to turn it off. In v0.2.0 the app itself makes **none**: the filter lists ship inside the app, search suggestions come only from your local history, and there is no update check yet.

When you search, your query goes to the search engine you selected (DuckDuckGo by default) under that engine's own privacy policy.

## The render engine
SCYDE uses the platform's engine, and the engine vendor may make its own background requests:
- WebView2 (Microsoft) on Windows, for example for runtime updates and SmartScreen
- WebKit (Apple) on iOS
- GeckoView (Mozilla) on Android

These are listed in `docs/NETWORK.md` and are turned off where the engine allows it.

## Location, camera and microphone
SCYDE asks for these only when a website you are on wants them, and only after you allow that site in SCYDE's own prompt (on iOS, WebKit's prompt). The system then asks once for the app as a whole. SCYDE itself never reads your location, camera or microphone. Downloads are saved from the page's own response into Downloads (Android) or the Files app under SCYDE (iOS); no second request is made.

## What we never do
- no analytics, telemetry or third-party SDKs
- no ads, sponsored tiles or affiliate links
- no selling or sharing of data, because we do not collect any

## Later features (each opt-in and described here before release)
- Crash reports (Phase 7): sent only after a crash, with your confirmation, to a self-hosted endpoint.
- SCYDE Sync (Phase 7): end-to-end encrypted, so the server cannot read your data.

# SCYDE Browser: master plan for Claude Code

> **Working name:** SCYDE Browser · **repo/codename:** `scyde-browser` · **app id:** `dev.scyde.browser`
> The name, icon and colours live in `brand/brand.json` + `design/tokens/`, so a rebrand is a single change.
> **Platforms:** Windows 10/11 (x64, arm64 later) · Android 8+ · iOS 16+ (.ipa for Feather / SideStore / TrollStore)
> **Document status:** v1, the foundation for Phase 0. Decisions that Phase 0 confirms or changes are recorded as ADRs in `docs/adr/`.

---

## 0. How Claude Code uses this document

1. Reading order: **PLAN.md → CLAUDE.md → DESIGN.md → `design/references/`**.
2. Work proceeds strictly by phases (§11). A phase is not closed until all of its **acceptance criteria (AC)** are met.
3. **Phase 0 has gates (G1–G5).** If a gate fails, do not continue with the next phase. Record the finding in an ADR, propose an alternative from §2.3 and mark the decision with `[OWNER]`.
4. Every architectural decision that deviates from this plan gets an ADR (`docs/adr/NNNN-title.md`, template in §13.4).
5. The state of work is always in `docs/STATUS.md`: what is done, what is in progress, blockers and open questions for the owner.
6. Windows and iOS artifacts are built in **GitHub Actions** (Claude Code usually runs on Linux). Iterate via `gh run watch` and `gh run view --log-failed`. For UI changes, CI always attaches screenshots (§13.3).

---

## 1. Vision and non-negotiable requirements

**Vision:** a fast, quiet, premium browser that protects by default. Page content is in the foreground, and the interface feels like one product across all three platforms, not like three apps.

| # | Requirement | What it concretely means |
|---|---|---|
| R1 | **Unique** | Own visual language "Aura" (DESIGN.md), own icon, own Shield system and own features (§11, Phase 6). It must not look like a reskin of Chrome or Brave. |
| R2 | **Zero ads + ad/popup blocker** | (a) Shield blocks ads, trackers and popups on all platforms, enabled by default. (b) **The browser itself never shows ads:** no sponsored tiles, sponsored backgrounds, affiliate redirects or crypto/rewards programmes. |
| R3 | **Dark/Light + themes** | Light, Dark and System. In addition, Aura accent (5 presets + a custom colour with a contrast check), New Tab backgrounds, interface density, reduced motion and an optional adaptive tint based on the page's `theme-color`. |
| R4 | **Premium look** | According to DESIGN.md. Every UI PR goes through an `impeccable` critique. Animations are physics-based, interruptible and within the frame budget. |
| R5 | **Highly optimised** | Hard performance budgets (§8) are measured in CI. A budget regression blocks the merge. |
| R6 | **Privacy** | No telemetry by default. Crash reports are opt-in. Network calls made by the browser itself: update check, filter bundle and (opt-in) search suggestions and news. Every such call is documented in `docs/NETWORK.md`. |

**Out of scope for v1:** crypto wallet, BAT-style rewards (the wallet and gift icons from the reference are **not** implemented), own search index, sync (comes in Phase 7), AI chat (later, opt-in).

---

## 2. Key architectural decisions

### 2.1 Engine strategy

Brave is a fork of Chromium. For a solo developer with Claude Code that is the wrong starting point: the build needs big machines (hundreds of GB, hours of compiling), GitHub hosted runners cannot handle it, and Chromium security patches ship every few weeks, which a fork would have to track by hand. That is why v1 builds **its own shell around platform engines that update their own security**, and keeps all the "brain" (Shield, omnibox, history, themes) in a shared Rust core.

| Platform | Engine | Shell / UI | Why | Security updates |
|---|---|---|---|---|
| **Windows** | **WebView2 (Evergreen)**, Chromium | **Tauri 2** (Rust) + chrome UI in web technology (Svelte 5) | Small installer, shared runtime, the Rust core links directly, full design control through the web UI | Automatic (Microsoft Evergreen runtime) |
| **Android** | **GeckoView** (Mozilla, MPL-2.0) | Kotlin + Jetpack Compose | A real browser engine: WebExtensions with blocking `webRequest`, Enhanced Tracking Protection, Total Cookie Protection. Not affected by Google's sign-in ban in Android WebView (§2.2). | Version bump in Gradle (Renovate PR every 4 weeks + urgent ones) |
| **iOS** | **WKWebView** (WebKit, the only one allowed outside the EU) | Swift: SwiftUI + UIKit for the browser view | The only realistic option. All iOS browsers (Chrome, Brave, Firefox) run on WebKit. | Automatic with the OS |

**Shared Rust core (`core/`)** (the same approach as Mozilla's *application-services*):
- `shield`: a wrapper around **adblock-rust** (Brave's engine, MPL-2.0). Supports EasyList/uBO syntax, cosmetic filtering, scriptlets, a WASM build and conversion to Apple's content-blocking format.
- `shield-compiler`: a CLI that compiles filter lists in CI into per-platform formats (§5).
- `omnibox`: URL fixup, URL or search, suggestion ranking (frecency), secure URL display.
- `places`: history, bookmarks, top sites (SQLite, `rusqlite` bundled).
- `settings`: settings schema, validation, migrations (JSON Schema as the source of truth).
- `theme`: token resolution, generating an accent scale from a user colour, contrast checking.
- `ffi`: UniFFI bindings (Swift, Kotlin). The Windows host uses the crates directly.

### 2.2 Risk no. 1: Google account sign-in in embedded engines

Google blocks OAuth sign-in in "embedded webviews". Android WebView and WKWebView are explicitly listed as non-compliant, and there are also reports of blocking on desktop WebView2. A browser in which you cannot sign in to Gmail or YouTube is unusable. Therefore:
- **Android → GeckoView**, not Android WebView. Gecko is Firefox's engine, not Android WebView.
- **iOS → WKWebView with a full Safari-like User-Agent** (`applicationNameForUserAgent = "Version/<x> Mobile/15E148 Safari/604.1"`), as all iOS browsers do.
- **Windows → WebView2 is conditional.** Gate **G1** in Phase 0 must prove that sign-in works (accounts.google.com → Gmail, YouTube, Drive, and "Sign in with Google" on a third-party site). The UA must be identical to stable Edge. No `navigator.webdriver` and no automation flags.

### 2.3 Fallback if G1 fails on Windows

1. **Gecko desktop fork** (the Zen / Floorp / LibreWolf approach). The same engine as on Android, chrome UI in HTML/CSS/JS, uBO-level blocking. Cost: rebase onto Firefox every 4 weeks and a full build on a self-hosted runner (Proxmox homelab / own server). For UI development, "artifact builds" (frontend only) are used.
2. **CEF**: full Chromium in the package, but Google lists it as problematic in the same policy, so it is not a real solution.
3. **Chromium fork (the Brave route)**: only if the project gets build infrastructure and time to track patches. Not for v1.

The decision goes into ADR-0002 with the spike results.

---

## 3. Architecture

```
┌──────────────────────────── core/  (Rust: single source of truth) ──────────────────┐
│  shield (adblock-rust) · omnibox · places (SQLite) · settings · theme · ffi (UniFFI)  │
│  shield-compiler (CLI, runs in CI)  ──►  signed filter bundle (§5.3)                  │
└──────┬───────────────────────────┬──────────────────────────────┬────────────────────┘
       │ native (crate)            │ WASM                          │ UniFFI
       ▼                           ▼                               ▼
 WINDOWS                     shield-extension/              ANDROID: Kotlin + Compose
 Tauri 2 host (Rust)         · target "chromium-mv3"        · libscyde_core.so (places, omnibox, settings, theme)
  ├ chrome webview (UI)        (DNR rules → WebView2)       · GeckoView + built-in shield extension (MV2, WASM engine)
  ├ tab webviews (WebView2)  · target "gecko-mv2"           IOS: Swift + SwiftUI/UIKit
  └ popover windows            (webRequest → GeckoView)     · ScydeCore.xcframework (incl. shield native)
                             · content scripts (cosmetics,  · WKContentRuleList + WKUserScript (isolated world)
                               scriptlets, counters)
       ▲                           ▲                               ▲
       └────────────── web/ (Svelte 5 + TS + Vite): shared design system and internal pages ─────┘
             newtab · settings (desktop) · shields panel · history/bookmarks · downloads · desktop chrome UI
```

**Internal pages** use the `scyde://` scheme (`newtab`, `settings`, `shields`, `history`, `bookmarks`, `downloads`, `about`). Per platform:
- Windows: Tauri custom protocol / WebView2 virtual host. The omnibox shows `scyde://…`.
- Android: the pages live inside the built-in extension (`moz-extension://…`) and communicate with the app via the built-in extension's native messaging. The omnibox shows `scyde://…`.
- iOS: `WKURLSchemeHandler` for `scyde://`.

---

## 4. Monorepo

```
scyde-browser/
├─ PLAN.md · CLAUDE.md · DESIGN.md · README.md · LICENSE · NOTICE.md
├─ brand/                    brand.json (name, app id, brand colours), icon (SVG master + exports)
├─ design/
│  ├─ references/            reference images (inspiration only, see §16)
│  ├─ prototypes/            quick HTML prototypes (skill: prototype)
│  └─ tokens/                DTCG JSON tokens → Style Dictionary → CSS / Kotlin / Swift
├─ core/                     Cargo workspace
│  ├─ shield/ · shield-compiler/ · omnibox/ · places/ · settings/ · theme/ · ffi/
│  └─ benches/               criterion benchmarks (shield match latency etc.)
├─ web/                      pnpm workspace
│  ├─ packages/tokens/ · packages/ui/ · packages/bridge/   (typed IPC client)
│  └─ apps/newtab/ · apps/settings/ · apps/shields/ · apps/library/ (history+bookmarks+downloads) · apps/desktop-chrome/
├─ shield-extension/         shared code; build targets chromium-mv3 and gecko-mv2
├─ desktop/                  Tauri 2 app (src-tauri/ + link to web/apps/desktop-chrome)
├─ android/                  Gradle project (app/, core-bindings/, baselineprofile/, benchmark/)
├─ ios/                      project.yml (XcodeGen), App/, BrowserKit/, Tests/, UITests/
├─ qa/                       adblock and popup fixture pages, break-ui scripts, site lists for manual QA
├─ spikes/                   Phase 0: one-off engine prototypes (deleted after the ADR)
├─ tools/                    icon export, contrast check, perf scripts, release scripts
├─ docs/                     STATUS.md · NETWORK.md · adr/ · architecture/ · release.md
└─ .github/workflows/        see §13
```

**Tools:** Rust stable (rustup, `cargo-ndk`, `uniffi-bindgen`, `wasm-pack` or `wasm-bindgen-cli`), Node LTS + pnpm, JDK 17+, Android SDK/NDK, latest stable Xcode (on CI), XcodeGen. Versions are pinned in Phase 0 (rust-toolchain.toml, .nvmrc, gradle version catalog).

---

## 5. Shield: ad, tracker and popup protection

### 5.1 Lists (enabled by default)
EasyList, EasyPrivacy, uBlock filters (+ privacy, + unbreak), Peter Lowe's list. Optional: cookie-banner/annoyance lists (the "Zen" switch), regional HR/EU lists (`[OWNER]` confirms the selection). The user can add their own list (URL) and their own rules.

### 5.2 `shield-compiler` outputs (one source → all formats)

| Output | Used by | Note |
|---|---|---|
| `engine.dat` (serialised adblock-rust engine) | iOS native, WASM in the gecko extension, Windows host (cosmetics) | Fast start: deserialisation instead of parsing lists |
| `dnr/*.json` (Chromium declarativeNetRequest static rulesets) | Windows (option A, §5.4) | Chromium guarantees 30,000 static rules per extension plus a global pool. At most 50 enabled rulesets out of 100 declared. The compiler splits lists and reports whatever does not fit. |
| `ios/*.json` (WKContentRuleList) | iOS | At most 150,000 rules per list. The compiler splits them into multiple lists. |
| `cosmetic/` (generic + per-host indexes) | all platforms | For fast lookup in content scripts |
| `resources.json` (scriptlets/redirect resources, uBO-compatible) | all | Licence: GPLv3, see §16 |
| `manifest.json` (version, hashes, signature) | updater | ed25519 / minisign |

### 5.3 List updates
- `filters.yml` (GitHub Actions, daily): download lists → `shield-compiler` → tests (golden fixtures) → sign → publish as release `filters-YYYYMMDD` and as `filters-latest` (GitHub Releases or own CDN/server).
- The client checks once a day (ETag/If-None-Match), verifies the signature, prepares the new engine **off the main thread** and does an atomic swap. If anything breaks, the old bundle stays.
- The installer, APK and IPA ship a snapshot of the bundle, so Shield works offline from the first second.

### 5.4 Per-platform integration

**Windows (WebView2).** The final choice is made by spike S2 against measurable criteria:
- **A, preferred:** our own **MV3 extension** (loaded via `browser_extensions_enabled` + `extensions_path` in Tauri, i.e. `AddBrowserExtension`) with **DNR static rulesets**. Blocking happens inside Chromium's network layer, with no IPC to the host. Per-site exceptions go into dynamic rules.
- **B:** a `WebResourceRequested` filter and the Rust engine in the host. Simple, but every request goes to the host's UI thread (an extra process hop), so latency is possible.
- **C:** CDP `Network.setBlockedURLs` via `CallDevToolsProtocolMethod`, with counters from `Network.loadingFailed`. Only as a measurement.
- **Cosmetics and scriptlets (v1):** the host injects a bootstrap script (`AddScriptToExecuteOnDocumentCreated`) that requests cosmetics from the host via WebMessage (`url_cosmetic_resources`, `hidden_class_id_selectors` batched from a MutationObserver). CSS is applied via `adoptedStyleSheets`. If the spike shows that the extension's isolated world is more stable, cosmetics move into the extension (ADR).
- **Main frame (popunder/redirect):** `NavigationStarting` checks the engine. A blocked domain gets an internal interstitial page "Shield stopped this page" (Continue once / Always allow).

**Android (GeckoView).**
- A built-in extension (`WebExtensionController.ensureBuiltIn`, no signature needed) with **MV2 `webRequest` + `webRequestBlocking`**, and the engine runs as **WASM** in the background script.
- Content scripts handle cosmetics and scriptlets. They send counters to the app via native messaging (`setMessageDelegate`).
- In addition, enable GeckoView **ETP Strict** and **Total Cookie Protection** (ContentBlocking settings).

**iOS (WKWebView).**
- Network and generic cosmetics: **WKContentRuleList** (compiled once, cached in `WKContentRuleListStore`, recompiled in the background after an update).
- Specific cosmetics: `WKUserScript` in its own `WKContentWorld` (isolated from the page). The query goes to the native engine (UniFFI) via `WKScriptMessageHandlerWithReply`.
- Scriptlets: page world, `atDocumentStart`.
- **The counter is an estimate:** WKContentRuleList does not report blocked requests. A content script collects resource URLs from the DOM and asks the engine, and the UI shows "≈".

**Shared:**
- Per-site Shield panel: on/off for the site, level (Standard / Aggressive), cookie banner blocking, number blocked, data and time saved (estimate).
- Global statistics on the New Tab.
- **Expectations:** YouTube and similar services are a cat-and-mouse game. Blocking is best-effort and depends on fresh scriptlets, so the daily bundle is not optional.

### 5.5 Popup blocker

| Platform | Mechanism |
|---|---|
| Windows | WebView2 disables Edge's popup blocker, so we do it ourselves: `NewWindowRequested`. If `IsUserInitiated == false`, set `Handled = true` and show a "Popup blocked" pill in the omnibox (Allow once / Always for this site). A user-initiated popup opens as a new tab in our UI (`NewWindow` = our webview, with a Deferral). |
| Android | Gecko's internal popup blocker + `PromptDelegate.onPopupPrompt` (our pill and decision) + `NavigationDelegate.onNewSession` (new tab). |
| iOS | `WKPreferences.javaScriptCanOpenWindowsAutomatically = false`. `WKUIDelegate.createWebViewWith` opens a new tab only for user gestures. |
| All | Main-frame navigations to blocked domains (popunder, redirect chains) go to the interstitial. Notifications and audio autoplay are "quiet" or disabled by default. |

---

## 6. Themes and design system (summary, details in DESIGN.md)

- **Source of truth:** `design/tokens/*.json` (DTCG format) → Style Dictionary → `web/packages/tokens` (CSS custom properties), `android/…/theme/Tokens.kt`, `ios/BrowserKit/Theme/Tokens.swift`.
- **User options:**
  - mode (Light / Dark / System)
  - Aura accent (Sky, Periwinkle, Lavender, Mint, Ember or a custom colour; `core/theme` generates the scale and "ink" variants and rejects combinations below WCAG AA)
  - New Tab background (Aura gradients, solid colour or own image; the image is scaled and cached locally)
  - density (Comfortable / Compact)
  - text size
  - reduced motion (follows the system, can be overridden manually)
  - adaptive tint (optional)
- **Application:** the desktop chrome UI and internal pages read CSS variables. A theme change is a 200 ms cross-fade without reflow. Native UI (Compose, SwiftUI) reads the generated tokens.

---

## 7. Security (a browser is a security product)

- **Omnibox:** emphasise the registrable domain (eTLD+1). Mixed scripts and confusable IDN domains are displayed as punycode. Long subdomains are truncated from the start, never in a way that hides the domain. The padlock and connection state come exclusively from the engine.
- **Internal pages:** strict CSP, no remote scripts. The native bridge is available **only** to the `scyde://` origin (origin check on every message, method allowlist, typed messages in `web/packages/bridge`). Web content never receives host objects.
- **Permissions** (camera, microphone, location, notifications, clipboard): native prompt with per-site memory and an overview in settings.
- **Private tabs/windows:**
  - Windows: WebView2 InPrivate profile (`incognito`)
  - Android: GeckoSession with `usePrivateMode`
  - iOS: `WKWebsiteDataStore.nonPersistent()`
- **Updates:** Tauri updater with a signature. Filter bundle with a signature. GeckoView bump via Renovate, and a security bump is merged within 48 h.
- Safe Browsing / SmartScreen: the privacy vs. protection policy goes into an ADR `[OWNER]`.

---

## 8. Performance

### 8.1 Budgets (targets; Phase 0 measures the baseline and an ADR may adjust them)

| Metric | Windows (mid-range laptop, SSD) | Android (Pixel 6a class) | iOS (iPhone 12 class) |
|---|---|---|---|
| Cold start to interactive chrome | ≤ 600 ms (target 400) | ≤ 900 ms | ≤ 500 ms |
| New Tab display after Ctrl+T / + | ≤ 120 ms | ≤ 200 ms | ≤ 150 ms |
| Tab switch (live tab) | ≤ 1 frame (16 ms) | ≤ 100 ms | ≤ 100 ms |
| Animations | 60 fps, 0 dropped frames in tab open/close and omnibox focus | 60/90/120 fps depending on the display | 60/120 fps (ProMotion) |
| Shield decision per request | native p99 < 50 µs | WASM p99 < 200 µs | rule list (WebKit) |
| Package size | installer ≤ 15 MB (Evergreen runtime) | GeckoView baseline + ≤ 10 MB of our code, per ABI | ≤ 30 MB |
| Desktop chrome JS bundle | ≤ 80 KB gz | – | – |
| New Tab JS bundle | ≤ 120 KB gz (without images) | same | same |
| Shell memory (without renderers) | ≤ 150 MB | measured in Phase 0 | measured in Phase 0 |

### 8.2 Techniques
- **Windows:**
  - one shared `CoreWebView2Environment` for all tabs
  - the window immediately shows the theme colour and a native skeleton (no white flash), and the chrome webview appears when ready
  - lazy tab restore (a tab is created only when it becomes visible)
  - **sleeping tabs**: a background tab gets `MemoryUsageTargetLevel = Low`, and after N minutes of inactivity `TrySuspendAsync` (the UI shows "sleeping · X MB saved")
  - popover windows are pre-created and hidden
  - no WebView2 for the splash
- **Android:**
  - `GeckoRuntime` as a singleton with early asynchronous initialisation
  - at most N live `GeckoSession`s (default 5), the others save their `SessionState` and are restored on tap
  - Baseline Profiles + Macrobenchmark, R8 full mode, ABI splits, Compose stability (no unnecessary recompositions; checked via Layout Inspector and compiler metrics)
- **iOS:**
  - a limited number of live `WKWebView`s; other tabs are a snapshot + `interactionState` (iOS 15+)
  - content rule lists are compiled once and cached
  - no heavy work on the main thread (Swift 6 concurrency)
- **Web UI:**
  - Svelte 5 (compiled, small runtime)
  - only `transform` and `opacity` are animated; `content-visibility` for lists; history virtualisation
  - fonts as subset variable WOFF2 (Latin + Latin Ext for č, ć, đ, š, ž)
  - images in AVIF/WebP, lazy loading
- **Measurement in CI:** criterion benchmarks (`core/benches`), Lighthouse-CI/perf script for internal pages, Macrobenchmark (Android, trend only on the emulator), XCTest `measure` (iOS), a startup script on the Windows runner (trend). Results are saved as a `perf.json` artifact. A regression greater than 10 % fails the job.

---

## 9. Platforms in detail

### 9.1 Windows (Tauri 2 + WebView2)

**Window:**
- `decorations: false`, Mica/Acrylic background (Tauri window effects), custom titlebar with tabs
- Windows 11 **Snap Layouts** on the maximize button: `WM_NCHITTEST → HTMAXBUTTON` via window subclassing, or evaluate `tauri-plugin-decorum`
- Windows-style caption buttons (not the macOS traffic lights from the reference)

**Layers:**
1. `chrome` webview: tab strip, toolbar and the left rail for web panels
2. `tab-*` webviews (Tauri multiwebview, feature `unstable`, all on the **same environment**), only the active one is visible
3. **popover windows**: transparent borderless windows without a taskbar icon, owned by the main window, for omnibox suggestions, menus, the Shield panel, the command palette and toasts. Child webviews cannot be stacked transparently on top of each other, so popovers live in separate top-level windows.

**Escape hatch:** `Webview::with_webview(|w| w.controller())` gives the raw `ICoreWebView2Controller` for APIs that Tauri does not expose (`NewWindowRequested` with a Deferral, `MemoryUsageTargetLevel`, `TrySuspend`, `AddBrowserExtension`, download events).

**v1 features:**
- tabs (pin, mute, drag reorder, drag-out to a new window later), omnibox with suggestions, back/forward/reload
- find-in-page, zoom, downloads (own panel), print
- history, bookmarks + bookmarks bar, import from Chrome, Edge and Firefox (Phase 7)
- Shield, popup blocker, New Tab, settings, themes
- private window, "Set as default browser" (registry `RegisteredApplications`, `StartMenuInternet`, URL and file associations, then open `ms-settings:defaultapps`)
- Chrome-like shortcuts (Ctrl+T/W/L/Tab, Ctrl+Shift+T, Ctrl+K command palette)

**Package:**
- NSIS per-user installer (no admin) + MSI for IT
- WebView2 bootstrapper if the runtime is missing
- Tauri updater (GitHub Releases, signature)
- code signing (Azure Trusted Signing or OV cert) `[OWNER]`, otherwise SmartScreen warns

### 9.2 Android (Kotlin + Compose + GeckoView)

- **Architecture:** single-activity, `BrowserStore` (StateFlow) for tabs, the active tab, nav state and Shield counters. `EngineSession` wrapper around `GeckoSession` (the interface allows swapping the engine). `TabManager` with a limit on live sessions. Core via UniFFI (`cargo-ndk`, arm64-v8a + armeabi-v7a + x86_64 for the emulator). Settings in DataStore (schema from `core/settings`).
- **UI:**
  - **bottom toolbar** with the omnibox (thumb zone); swiping on the toolbar switches tabs; swiping up opens the tab switcher (grid with a shared-element transition)
  - Shield bottom sheet; context menu for links (long-press, peek preview in Phase 6)
  - **edge-to-edge** (mandatory for targetSdk 35+), **predictive back** (webview history → close sheet → exit)
  - Material 3 is only the technical base; visually everything comes from DESIGN.md (no generic Material look)
- **Internal pages:** the web build is packaged into the built-in extension (assets). The New Tab is a `moz-extension` page displayed as `scyde://newtab`.
- **Package:** signed APK per ABI (GitHub Releases, Obtainium-compatible), later AAB for Play and F-Droid `[OWNER]`. `applicationId dev.scyde.browser`.

### 9.3 iOS (Swift + WKWebView, .ipa for Feather)

- **Architecture:**
  - SwiftUI for the app shell, settings and sheets; UIKit `BrowserViewController` for WKWebView (gestures, performance, `UIScrollView` control)
  - Swift 6 strict concurrency
  - `ScydeCore.xcframework` (Rust, targets `aarch64-apple-ios` + `aarch64-apple-ios-sim`) is built in CI
  - XcodeGen `project.yml` (no hand-edited `.pbxproj`, so no merge conflicts)
- **UI:**
  - **bottom address bar** (like Safari), swiping on the bar switches tabs, tab grid with a zoom transition
  - Shield sheet with detents, haptics
  - system materials (Liquid Glass on iOS 26+ where it fits naturally, otherwise `ultraThinMaterial`); everything per the `apple-design` skill
  - Dynamic Type
- **Engine settings:** Safari-like UA (§2.2). `WKWebpagePreferences.allowsContentJavaScript` per site (option). A private tab uses the `nonPersistent` data store. `isInspectable` only in the debug build.
- **Package:** CI on a macOS runner: `xcodebuild archive CODE_SIGNING_ALLOWED=NO`, then `Payload/<App>.app` zipped into an `.ipa` and a release artifact. Feather signs on the device. Option: an AltStore-compatible JSON source (GitHub Pages) for updates via Feather.
- **Expected sideloading limitations:**
  - the app cannot be the default browser (the `com.apple.developer.web-browser` entitlement is granted by Apple)
  - no push notifications
  - App Groups and the share extension may depend on the certificate in Feather, so the share extension remains optional and must not be required for the app to work

---

## 10. New Tab (based on the reference, but our own)

- **Hero:** SCYDE logo + three "privacy chips" with **live numbers** (e.g. "Blocked today 1,284", "Saved 42 MB", "≈ 3 min").
- **Search:** a large field with a pill CTA button (Ember). Suggestions from history, bookmarks and the search engine (opt-out). Voice input goes through the native speech API via the bridge. Image search (Phase 6) sends the image to the reverse-image page of the selected search engine, with a privacy warning.
- **Speed dial:** a grid of tiles (top sites + pinned), drag reorder, "Add". The favicon is fetched and cached locally. The fallback is a monogram tile in an Aura colour.
- **Top Stories:** an **opt-in** RSS aggregator (categories: News, Tech, Sport, Entertainment, Travel…). Sources are chosen by `[OWNER]` (HR + international). Fetching is direct, with no trackers, with caching. Categories use our own icons (not emoji as in the reference).
- **Desktop left rail:** web panels (Gmail, Discord, Telegram, WhatsApp Web, LinkedIn… that the user adds themselves). Each panel is a separate webview that stays signed in.
- **Background:** Aura gradient or own image. No sponsored images, ever (R2).
- **Empty states and break-ui:** 0 and 100 tiles, no internet, with news turned off.

---

## 11. Phases

> Sizes are relative (S/M/L/XL). Phases 4 and 5 can run in parallel after Phase 3 (separate branches and sessions).

### Phase 0: Foundations, design and spikes (L) 🔒 gate
**Steps:**
1. **0.1 Repo:** structure from §4, pinned toolchains, `CLAUDE.md`, `docs/STATUS.md`, `docs/NETWORK.md`, ADR template, Renovate, basic CI (lint and build of the empty skeleton).
2. **0.2 Design foundation:** the `impeccable` skill finalises **DESIGN.md**. `taste-skill` checks the direction against a generic look. `pick-ui-library` confirms *Svelte 5 + own components (no UI kit)* within the JS budget → ADR-0005.
3. **0.3 Brand and icon:** three original icon concepts (SVG) following the direction in `design/references/03-app-icon-direction.png`, but **without two overlapping circles** (see §16). Export pipeline `tools/icon-export`: Windows `.ico` 16–256, Android adaptive (foreground, background, monochrome for themed icons), iOS 1024 + dark + tinted variants. `[OWNER]` chooses the concept.
4. **0.4 Prototypes** (`prototype` skill, `design/prototypes/`): New Tab light/dark, desktop chrome (tabs, omnibox focus with suggestions, Shield panel), mobile bottom toolbar + tab switcher. `impeccable critique` on each.
5. **0.5 Spikes** (`spikes/`, one-off):
   - **S1 Google sign-in** on all three engines → **G1**
   - **S2 WebView2 Shield:** options A/B/C from §5.4 on 5 heavy sites (e.g. index.hr, 24sata.hr, jutarnji.hr, youtube.com, cnn.com) measuring load time against a baseline without blocking → **G2**
   - **S3 GeckoView:** built-in extension, webRequestBlocking, WASM engine, native messaging from internal pages → **G3**
   - **S4 iOS:** compiling ≥ 150k rules split into multiple lists on the simulator; measuring compilation and cached load → **G4**
   - **S5 Tauri multiwebview:** 20 tabs on one environment, switching, pre-warming popover windows, Mica, custom titlebar + snap → **G5**

**AC / gates:**
- **G1:** Google sign-in (Gmail, YouTube) works on WebView2, GeckoView and WKWebView (screenshots in ADR-0002). If it does not work on WebView2, activate §2.3 and stop.
- **G2:** Shield on Windows adds ≤ 3 % (target 0) to the median load time and blocks ≥ 95 % of fixture requests (`qa/adblock-fixtures`). The chosen option goes into ADR-0003.
- **G3:** The GeckoView extension blocks the fixture, WASM decision p99 < 200 µs, internal pages talk to the app.
- **G4:** Rule lists compile. Cached load is < 100 ms. The fixture is blocked.
- **G5:** Tab switch ≤ 16 ms, popover ≤ 16 ms, snap layout works.
- DESIGN.md is finalised, the icon chosen, prototypes approved by `[OWNER]`.

### Phase 1: Rust core and Shield pipeline (L)
- Crates `shield`, `shield-compiler`, `omnibox`, `places`, `settings`, `theme` and `ffi` (UniFFI → Swift, Kotlin). WASM package for the extension.
- `filters.yml` (daily, signature, publishing). Golden tests (fixture lists → expected DNR / iOS / cosmetic output).
- Benchmarks: match latency, engine deserialisation, bundle size.

**AC:**
- `cargo test` green; clippy without warnings
- native match p50 < 5 µs, p99 < 50 µs; deserialisation < 50 ms (desktop)
- iOS lists ≤ 150k per list; DNR output within Chromium limits (with a report on dropped rules)
- bundle signed and signature verified in a test
- `omnibox` passes tests: URL or search, punycode and confusable names, fixup (`scyde`, `localhost:3000`, `192.168.1.1`, `ftp://`)

### Phase 2: Web design system and internal pages (L)
- `web/packages/tokens` (from Style Dictionary), `packages/ui` (Button, IconButton, Chip, Tile, Card, Menu, Popover, Tooltip, Toast, Toggle, Slider, Tabs, Sheet, SearchField, Kbd, Skeleton), `packages/bridge` (typed, with a mock for tests).
- Apps: `newtab`, `settings`, `shields`, `library`.
- Skills: `emil-design-eng` (component details), `animate` + `choreograph` (New Tab entrance, tile stagger), `animation-performance`, `mobile-native` (internal pages on the phone: safe areas, touch targets, overscroll, `100dvh`, no tap highlight), `awwwards-animations` (guidelines only for the hero and onboarding; implementation via CSS and WAAPI, no GSAP).
- Playwright visual tests in **Chromium, Firefox and WebKit** (all three engines we use!), light and dark, 3 widths.

**AC:**
- bundle budgets (§8)
- Lighthouse a11y ≥ 95
- all contrasts ≥ AA (automated test)
- `prefers-reduced-motion` respected
- screenshots as artifacts

### Phase 3: Windows MVP (XL)
- Tauri shell per §9.1: tabs, omnibox, navigation, Shield (chosen option), popup blocker, internal pages, themes, sleeping tabs, downloads, find, zoom, private window, shortcuts, default browser, installer + updater.
- Skills: `emil-design-eng` (tab strip, omnibox, menus), `choreograph` (tab open/close, omnibox expand), `animation-performance`, `break-ui` at the end.

**AC:**
- §8 budgets (Windows column)
- G1 still green
- 500 tabs without crashing (sleeping)
- `qa/` adblock ≥ 95 %, popup fixture 100 %
- updater tested with two versions
- `break-ui` report with no P1 findings

### Phase 4: iOS MVP (L)
- §9.3: tabs, bottom bar, tab grid, Shield (rule lists + cosmetics + counter estimate), popup, internal pages, themes, private tab, sharing, downloads (Files), find, reader (later).
- Skills: `apple-design` (sheets, springs, materials, typography, gestures), `mobile-native` (internal pages), `break-ui`.
- CI: unsigned `.ipa` + XCUITest screenshots (light/dark, iPhone SE and Pro Max sizes).

**AC:**
- §8 budgets (iOS)
- installation via Feather on a real device confirmed by `[OWNER]`
- Google sign-in works
- adblock/popup fixture

### Phase 5: Android MVP (XL)
- §9.2: GeckoView, Compose UI, bottom toolbar, tab switcher, Shield extension, ETP + TCP, popup, internal pages, themes, private tabs, downloads, predictive back.
- Skills: `apple-design` (spring and gesture principles translated to Compose), `mobile-native`, `animation-performance`, `break-ui`.

**AC:**
- §8 budgets (Android)
- Baseline Profile active (Macrobenchmark report)
- APK per ABI in the release
- Google sign-in works
- fixtures pass

### Phase 6: Premium polish and unique features (L)
- Animation audit: `find-animation-opportunities` → `improve-animations` → `review-animations` → `remove-animations` (remove everything that does not serve a purpose). Then `impeccable audit` + `polish`, then `break-ui` on all platforms.
- **Unique features** (order confirmed by `[OWNER]`):
  1. **Aura adaptive tint** of the chrome based on `theme-color` or the dominant favicon colour (with contrast protection)
  2. **Shield pill** with a morphing counter and "saved" statistics
  3. **Sleeping tabs** with visible RAM savings
  4. **Split view** (2 tabs side by side) + **command palette** (Ctrl+K: tabs, history, actions, settings) on desktop
  5. **Web panels** in the left rail (desktop)
  6. **Peek** (long-press link preview) on mobile
  7. **Zen mode** per site (cookie banners, annoyance lists, reader)
  8. **Spaces** (tab groups with their own Aura colour)
- Onboarding (3 screens: theme, search engine, import), without dark patterns.

**AC:**
- 0 animations outside the budget
- `impeccable audit` with no critical findings
- every feature has an on/off switch and a test

### Phase 7: Data, sync, release (L)
- Import on Windows from Chrome, Edge and Firefox (bookmarks, history; passwords only if it is safe and transparent, otherwise exporting a CSV is recommended).
- Password manager: v1 uses the OS (Windows Hello / Credential Manager, iOS AutoFill, Android Credential Manager), our own only after a security review.
- **SCYDE Sync** (option `[OWNER]`): E2E-encrypted sync of bookmarks, history, settings and tabs. Self-hosted server as a **Docker Compose** app (the same pattern as Mediarr/Streamero), client in `core/sync`.
- Release hardening: code signing, opt-in crash reports (self-hosted), `NOTICE.md` with licences, privacy policy, release rules (`docs/release.md`), Play/F-Droid preparation `[OWNER]`.

---

## 12. Skill map (when each one is used)

| Skill | When | Result |
|---|---|---|
| `impeccable` | Phase 0 (DESIGN.md), **every UI PR** (critique), Phase 6 (audit, polish) | DESIGN.md is a living document; findings go into the PR description |
| `taste-skill` (design-taste-frontend) | prototypes, New Tab, onboarding, marketing page | checks that nothing looks generic or "AI-made" |
| `prototype` | before every new UI feature | HTML prototype in `design/prototypes/` |
| `pick-ui-library` | Phase 0 | ADR-0005 (Svelte 5 + own components or a justified alternative) |
| `emil-design-eng` | components: tabs, omnibox, menus, toasts, tooltips, sheets | micro-details, states, focus, timing |
| `apple-design` | iOS phase; spring and gesture principles for Android and desktop | sheets, materials, typography, gestures |
| `mobile-native` | internal web pages on Android and iOS | they look and behave natively |
| `animate` | a new animation from scratch | implementation according to motion tokens |
| `animation-vocabulary` | naming effects in DESIGN.md and PRs | a shared language |
| `choreograph` | multi-element sequences (New Tab entrance, tab switcher, omnibox) | stagger and ordering |
| `animation-performance` | every PR with an animation | compositor-only, no layout thrashing, within budget |
| `find-animation-opportunities` | Phase 6 | list of places where motion is missing |
| `improve-animations` | Phase 6 | improvement plan |
| `review-animations` | Phase 6, before release | critique |
| `remove-animations` | Phase 6, power-saver mode | removing what is superfluous |
| `break-ui` | end of Phases 3, 4, 5 and 6 | report with scenarios from §14.3 |
| `awwwards-animations` | New Tab hero, onboarding, reveal on theme change | **guidelines only**; no GSAP or its libraries (no licence); implementation via CSS, WAAPI and `linear()` springs |

---

## 13. CI/CD and GitHub

### 13.1 Workflows (`.github/workflows/`)

| Workflow | Runner | Trigger | Does |
|---|---|---|---|
| `core.yml` | ubuntu | changes in `core/**` | fmt, clippy, test, bench smoke, WASM build |
| `web.yml` | ubuntu | `web/**`, `shield-extension/**`, `design/tokens/**` | lint, typecheck, vitest, Playwright (3 engines) + screenshots, bundle-size check |
| `filters.yml` | ubuntu | daily cron + manual | compile, test, sign, publish filter bundle |
| `windows.yml` | windows | `desktop/**` + dependencies; tag `v*` | build NSIS/MSI, smoke start + screenshot; release with updater artifacts on tag |
| `android.yml` | ubuntu | `android/**` + dependencies; tag | assemble, unit tests, (optional) emulator smoke + screenshots; signed APK on tag |
| `ios.yml` | macos | `ios/**` + dependencies; **tag or manual** | xcframework, xcodegen, simulator build + XCUITest screenshots; unsigned .ipa on tag |

- Path filters, `concurrency: cancel-in-progress`, caching (rust-cache, Gradle, pnpm, SPM).
- **Cost:** on a private repo, macOS (and Windows) minutes are billed at a multiple of the Linux rate, so the iOS build runs only on a tag or manually, and PRs build the simulator only when `ios/**` changes.

### 13.2 Secrets
`TAURI_SIGNING_PRIVATE_KEY` (+ `_PASSWORD`), `ANDROID_KEYSTORE_B64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`, `FILTERS_SIGNING_KEY`, later Windows code signing. Public keys for signature verification are in the repo (`brand/keys/`).

### 13.3 Visual verification without devices
Every UI PR attaches artifacts: Playwright screenshots of internal pages (3 engines × light/dark × 3 widths), a Windows smoke screenshot, iOS simulator screenshots and Android emulator screenshots (where the job is active). Claude Code links the artifacts in the PR description.

### 13.4 Conventions
- Branches `phase/N-name`, `feat/…`, `fix/…`. Conventional Commits. Small PRs. Squash merge.
- SemVer versioning `v0.x.y` until the public release. One tag builds all platforms.
- **ADR template:** Context · Options · Decision · Consequences · Measurements/evidence · Status.

---

## 14. Testing and QA

### 14.1 Automated
- **core:** unit, property tests (omnibox parsing), compiler golden tests, benchmarks
- **web:** vitest, Playwright, axe a11y, visual diff
- **Windows:** integration tests via WebView2 CDP in the dev build (remote debugging port **only in debug**)
- **Android:** JUnit, Compose UI tests, Macrobenchmark
- **iOS:** XCTest, XCUITest

### 14.2 `qa/` fixtures (deterministic, served locally)
- `adblock/`: a page with known ad and tracker URLs (network), elements for cosmetics (`##.ad-banner`, generic and specific) and a scriptlet test
- `popup/`: `window.open` without a gesture, with a gesture, `target=_blank`, popunder, redirect chain, `window.open` in `setTimeout` after a click
- `perf/`: a heavy page for measurement
- manual list of real sites: youtube.com, index.hr, 24sata.hr, jutarnji.hr, net.hr, cnn.com, forbes.com, reddit.com, twitch.tv, gmail.com

### 14.3 `break-ui` scenarios (mandatory)
- 500 tabs; a 300-character title; RTL and emoji titles; missing favicon
- a 4,000-character URL; IDN homograph (e.g. Cyrillic "а" in `аpple.com`)
- slow network and offline; a 5 GB download
- 0 and 100 speed-dial tiles
- 200 % scaling (Windows), mobile text at 200 %, a 500 px wide window
- Windows High Contrast; Dark ↔ Light in the middle of an animation
- very long history lists (100k entries)

---

## 15. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Google blocks sign-in in WebView2 | medium | critical | G1 in Phase 0; Edge-identical UA; fallback §2.3 |
| DNR in the WebView2 extension is limited or unstable | medium | high | S2 measures options A/B/C; decision in an ADR |
| Tauri multiwebview (feature `unstable`) changes | medium | medium | version pinning, escape hatch to raw WebView2 COM, tests in S5 |
| GeckoView API changes between versions | medium | medium | Renovate + CI smoke, `EngineSession` abstraction |
| YouTube and anti-adblock scripts | high | medium | daily bundle, scriptlets, fast hotfix channel |
| iOS sideloading limitations | certain | low/medium | documented (§9.3), no dependency on entitlements |
| Scope (3 platforms, 1 developer) | high | high | strict phases, AC, MVP before polish, parallelising Phases 4 and 5 |
| Licences (lists, scriptlets, font) | medium | medium | §16, `NOTICE.md`, open-source decision |

---

## 16. Licences and legal

- **adblock-rust:** MPL-2.0 (file-level copyleft; changes to MPL files are published). **GeckoView:** MPL-2.0.
- **Filter lists:** EasyList and EasyPrivacy are dual-licensed (GPLv3 / CC BY-SA). uBlock filters and **scriptlet resources are GPLv3**. If the app remains closed source, the lists and resources are distributed as a **separate data bundle** with attribution and a link to the source. A legal review is required before the public release `[OWNER]`.
- **Aeonik font** (CoType Foundry) is commercial; embedding it in the app requires an app licence. Until then, an OFL substitute is used (DESIGN.md §3).
- **Icons:** Lucide (ISC) or our own.
- **Reference images** in `design/references/` are inspiration. **Do not copy** the red logo or the exact layout of someone else's Dribbble/Behance work, and do not use their assets.
- **Icon:** two overlapping orange-red circles are visually very close to a well-known card company's trademark. Keep the feel (soft conic gradient, white squircle tile), but make the shape original (Phase 0, step 0.3).
- Before the public release, check the trademark for the name "SCYDE Browser".

---

## 17. Open questions for the owner (`[OWNER]`)

Default values apply until the owner decides.

1. **Name and home screen label.** Default: "SCYDE Browser". Watch out for a clash with an existing app whose label is "SCYDE".
2. **Default search engine.** Default: DuckDuckGo. Alternatives: Brave Search, Startpage, Google, Bing, Ecosia.
3. **Open source or closed source?** Affects §16.
4. **Aeonik licence or OFL substitute?**
5. **Windows code signing** (budget, SmartScreen).
6. **SCYDE Sync** self-hosted on own infrastructure: yes/no.
7. **Sources for Top Stories** (HR + international), opt-in.
8. **Distribution for Android:** GitHub Releases only, or also Play/F-Droid.

# ADR-0012: Windows shell v1: native frame, island band, Shield extension, COM popup blocker

- **Status:** Accepted
- **Date:** 2026-11-11
- **Decided by:** Claude Code (owner: "continue with all phases", 2026-11-11)
- **Related:** PLAN §9.1, §5.4, §5.5, ADR-0003, ADR-0007, DESIGN.md §4–§8

## Context
PLAN §9.1 describes a frameless window with Mica, the island floating over the page, and popovers in separate transparent windows. ADR-0007 left Snap Layouts open, because the caption buttons live in a child webview that swallows `WM_NCHITTEST`. Phase 3 has to ship a working `.exe` that is built and verified only in CI (there is no Windows machine in the loop).

## Options
1. **Full §9.1 now:** frameless window, a floating island window, popover windows and custom caption buttons with a Snap Layouts overlay. Most of it cannot be verified without a person on Windows 11.
2. **v1:**
   - native window frame (Snap Layouts, dragging, caption buttons and accessibility for free)
   - tab webviews above an island band at the bottom
   - the menu inside the island
   - suggestions as inline completion
3. Option 2 now, with option 1 as Phase 6 polish once the owner can check it on a real machine.

## Decision
**Option 3.** In v1:
- **Window:** native frame. The `chrome` webview is a band of 104 px (island 60 + 22 + 22) at the bottom, and tab webviews fill the rest (`relayout` on resize). This deviates from "content to the edge", accepted until Phase 6.
- **Internal pages:** the `scyde` custom protocol (`http://scyde.localhost/<page>/` in WebView2) serves `web/apps/*`. The pages carry a CSP header with `frame-ancestors 'none'`. The `bridge` command answers only a `tab-*` webview whose current URL is on that origin, and only allowlisted methods (CLAUDE.md rule 6). Chrome commands answer only the `chrome` webview.
- **Shield:** option A from ADR-0003. A bundled MV3 extension (`shield-extension`, target `chromium-mv3`) is built in CI from a fresh `shield-compiler` bundle: DNR rulesets for the network, plus cosmetics without a service worker: the generic selectors as one stylesheet that the browser injects (manifest `css`), and host rules as 256 hashed JSON shards that the content script fetches as web-accessible resources. The first version used a worker and messaging, but WebView2 scored only 8/11 in the CI smoke test (cosmetic rules were missing), so the worker was removed. Every webview gets the same `extensions_path` (one environment). Locally in Chromium the extension scores **11/11** on `qa/adblock`.
- **Popup blocker:** a COM `NewWindowRequested` handler. Without `IsUserInitiated` the popup is cancelled and counted (shown in the island's pebble). A user-initiated popup keeps WebView2's default window, so `window.opener` flows (OAuth) keep working.
- **Shortcuts:** a COM `AcceleratorKeyPressed` handler on every webview (Ctrl+T/W/L/Tab, Ctrl+Shift+T/Tab, Ctrl+R/F5, Alt+←/→, Alt+D). Host work runs off the callback thread.
- **History:** visits go to `core/places` (typed vs. link frecency), settings to `core/settings` (JSON in the app's local data directory).

## Consequences
- **Not yet in v1:**
  - blocked-request counters (DNR has no production feedback; ADR-0003): only popups are counted
  - per-site Shield switches (need dynamic DNR allow rules)
  - suggestions popover, session restore, downloads panel, find, zoom, private window, default-browser registration
  - updater (needs `TAURI_SIGNING_PRIVATE_KEY` `[OWNER]`)
- **Snap Layouts** work, because the window frame is native.
- **Phase 6:** frameless window + floating island window (+ popovers), checked by the owner on Windows 11.

## Measurements / evidence
CI `windows` → artifact `scyde-windows`:
- the NSIS installer
- `smoke.json`: tab titles, including the `qa/adblock` score, the popup fixture and the internal pages
- `smoke.png`

# ADR-0013: Phase 6 on phones: island v2, private tabs, per-site Shield, sleeping tabs, Spaces

- Status: accepted (owner, 2026-11-12: "implement all the features and more, better than Arc"; "full screen while reading")

## Decisions
- **Island v2:** mark (menu) · back · address · Shield light (with the page count on Android) · tabs. A control center, the Shield panel and the omnibox panel grow out of it (DESIGN.md §6.3 Swell). Scrolling down hides it (DESIGN v2.1 "Hide").
- **Private tabs:**
  - iOS: one non-persistent `WKWebsiteDataStore`, replaced when the last private tab closes.
  - Android: `GeckoSessionSettings.usePrivateMode`. The Shield extension is allowed in private browsing.
  - No history, no session file, no Space.
- **Shield per site:**
  - The switches are kept in `shield.disabled_sites` (eTLD+1).
  - iOS: each tab has its own content controller, so a site with Shield off simply loads without the rule lists. They are applied in `decidePolicyFor` before each main-frame load.
  - Android: the list goes to the extension over a native port, and the background script skips both blocking and cosmetics for those top hosts.
- **Sleeping tabs:** tabs unused for `tabs.sleep_after_minutes` (default 15, 0 = off) release their page.
  - iOS keeps `interactionState` and swaps in an empty `WKWebView`.
  - Android keeps the `SessionState`, closes the `GeckoSession`, and restores the state into a new session.
  - A sleeping tab wakes, with its history, when it is opened.
- **Session restore:** `session.json` in the app's data folder holds the non-private tabs, pinned flags and Spaces. Restored tabs start asleep; only the active one loads. `startup.open = newtab` restores only the pinned tabs.
- **Pinned tabs:** they come first in the grid and survive "Close all".
- **Spaces:** named tab groups with an accent preset as their colour. The island's edge takes the current Space's colour when there is more than one Space; the tab count, tab swipes and "Close all" stay inside the current Space. Deleting a Space moves its tabs to the first one.

## Consequences
- The session file is local only, like history (`PRIVACY.md`). Nothing about private tabs is ever written.
- iOS cannot count blocked requests (WebKit content rules report nothing), so the iOS Shield panel says so instead of showing a number.

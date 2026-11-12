# ADR-0014: Windows: the island floats in its own transparent window

- Status: accepted (owner, 2026-11-12: "behind the navbar it must be transparent, not a black background")
- Supersedes: the island band in ADR-0012

## Context
In v0.3.0 the island was a child webview in a band at the bottom of the main window. Tab webviews ended above the band, so a black strip framed the island. DESIGN.md §1 wants "content edge to edge, the interface as a single object". WebView2 child webviews cannot be transparently layered over other child webviews (CLAUDE.md: popovers on desktop are separate transparent top-level windows).

## Decision
- **Tabs:** tab webviews fill the whole client area.
- **The island:** a separate top-level `WebviewWindow` with label `chrome`:
  - transparent, borderless, without a shadow or a taskbar entry
  - owned by the main window: it stays above it, hides when the main window is minimized, and closes with it
  - it uses the same WebView2 environment options as the tabs
- **Position:** `place_island` keeps the island centred over the bottom of the client area on every move, resize and DPI change. The window is only as wide as the expanded island plus its shadow (784 px logical), so the page stays clickable beside it.
- **Background:** the island page (`web/apps/desktop-chrome`) has a transparent background.
- **Permissions:** the `chrome` capability now applies to the `chrome` window. Every command still checks the caller's label.

## Consequences
- The page runs under the island, as on phones; the island covers about 104 px at the bottom of the page.
- Clicking the island activates its window, so the main window's title bar shows as inactive until focus returns to a tab.
- Hiding the island while reading on desktop (DESIGN v2.1 "Hide") is still to do.

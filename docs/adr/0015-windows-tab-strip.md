# ADR-0015: Windows: a tab strip at the top

- Status: accepted (owner, 2026-11-13: "I can't see the tabs anywhere … without tabs at the top I don't know how to switch tabs and see what is on which tab")
- Changes: DESIGN.md §1 "no tab strip and no toolbar at the top" and §11 "Don't: a tab strip at the top" (Windows only)

## Context
Since v0.3.0 the tabs on Windows were a stack of overlapping favicon circles inside the island. With more than a few tabs nothing told them apart: no titles, no close buttons, and at most six were shown. The owner could not find or tell apart the tabs.

## Decision
- **Strip:** a child webview `strip` (web/apps/desktop-chrome `strip.html`) across the top of the main window, `size.chrome-tabstrip` (44 px) high. Each tab shows its icon (or a spinner while loading), its title and × (on hover and on the active tab). Click switches, middle click and × close, arrow keys move between tabs, + opens a new tab.
- **Frameless window:** the main window has no native title bar. The strip draws the caption buttons (minimize, maximize/restore, close; close hovers `status.danger`), and its empty area drags the window (double click maximizes). Windows keeps the window shadow and the resize borders (Tauri's undecorated resize handler, attached after the strip is added).
- **Tabs:** tab webviews fill the window below the strip. The island keeps navigation, the address, Shield and +; its favicon stack is removed.
- **Same environment:** the strip uses the same WebView2 environment options as the tabs (ADR-0003, ADR-0007). The `chrome` capability now also covers the `strip` webview; every command still checks the caller's label.

## Consequences
- Every tab is visible with its title, as in other desktop browsers. The page loses 44 px at the top.
- Snap Layouts on hovering the maximize button are not available (the button is ours). Win+arrow keys and dragging to screen edges still snap.
- Phones keep the island with the tab count and the tab grid; whether they also get visible tabs is an open owner question.

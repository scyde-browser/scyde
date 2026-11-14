# ADR-0016: Windows: a sidebar instead of the floating island and the tab strip

- Status: accepted (owner, 2026-11-14: "on Windows I have no navbar … look at how Zen Browser and Arc are built, I want such features too, only keep our UI design")
- Supersedes: ADR-0014 (floating island window) and ADR-0015 (tab strip) on Windows
- Changes: DESIGN.md §1 "the whole interface is an island" for Windows; phones keep the island

## Context
The island on Windows was a separate transparent top-level window kept in place over the main window (ADR-0014). On the owner's PC it did not show at all, which left the browser without navigation and an address. CI could not catch this: the island showed on the CI machine. A window that must be positioned, kept above its owner and hidden by a timer has too many ways to fail that we cannot test.

The owner also asked for the way Arc and Zen are laid out: tabs down the side, pinned tabs, Spaces, split view, a command bar.

## Decision
- **One sidebar webview** (`sidebar`, web/apps/desktop-chrome) on the left of the frameless main window, `size.chrome-sidebar` (280 px) wide. It is a child of the main window, so it cannot be off-screen or behind it. It holds, top to bottom: the mark and the caption buttons (the empty area drags the window), back/forward/reload, Shield and "hide sidebar", the address, pinned tabs as tiles, the current Space's name, "New tab", the Space's tabs, and the Spaces with History and Settings.
- **Command bar:** the address is also the command bar (Ctrl+L, Ctrl+K). While typing, the sidebar lists commands, open tabs, history matches (local, `core/places`) and "search for"; arrows and Enter pick one.
- **Pinned tabs** show in every Space. **Spaces** group the other tabs; each has a colour; a Space always has a tab. Deleting a Space moves its tabs to the first one.
- **Split view:** "open beside" on a tab shows it next to the active tab; both stay side by side while either is active.
- **Hide the sidebar** (Ctrl+S or Ctrl+B): the page fills the window. The mouse at the left edge, or Ctrl+L, brings the sidebar back while it is needed.
- **Session:** tabs, pinned tabs, Spaces and the sidebar state are saved to `session.json` and restored on start. Restored tabs sleep (no webview) until they are opened.
- **Removed on Windows:** the island window, its scroll watcher (`ExecuteScript` polling) and the tab strip.
- **Security:** unchanged: the sidebar is local app content, every command checks that the caller is the `sidebar` webview, tab webviews get no IPC except internal pages' bridge.

## Consequences
- Navigation and the address are always inside the window. Pages are 280 px narrower unless the sidebar is hidden.
- With the sidebar hidden there are no caption buttons until the mouse reaches the left edge.
- Web panels, Peek and Zen mode on Windows are still open (STATUS).

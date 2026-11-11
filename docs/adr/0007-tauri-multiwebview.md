# ADR-0007: Windows shell: Tauri 2 multiwebview (feature `unstable`)

- **Status:** Accepted (Phase 0 closed 2026-11-11)
- **Date:** 2026-11-11
- **Decided by:** Claude Code, based on spike S5
- **Related:** PLAN §9.1, gate **G5**, `spikes/s5-tauri`

## Context
The desktop shell holds the chrome webview, one webview per tab, and popovers in separate windows, all on a single WebView2 environment. G5 requires tab switching and popover display to take ≤ 16 ms, and snap layouts to work.

## Options
1. **Tauri 2.12.5 multiwebview** (`unstable`): child webviews in one window, show/hide for tabs.
2. A single webview with navigation between tabs: loses tab state, out of the question.
3. Our own host directly on webview2-com, without Tauri: more code, no advantage in the measurements.

## Decision
**Option 1**, with pinned versions (tauri =2.12.5, tauri-build =2.7.4, webview2-com =0.39.1) and the `with_webview` escape hatch for COM APIs (proven in the spike via `WebResourceRequested`).

## Consequences
- Tauri 3 is in alpha; we migrate only once it is stable (the Renovate group `tauri` stays manual).
- Every webview must share the same environment options (see ADR-0003).
- **Tab creation is expensive:** 20 webviews = 3.7–4.8 s, about 185–240 ms per tab. Lazy tab creation (PLAN §8.2) is **mandatory**, and Ctrl+T needs a pre-warmed tab webview so that New Tab is ≤ 120 ms.
- **Snap Layouts (open):** the caption buttons are in the chrome webview, so the child HWND receives the mouse and `WM_NCHITTEST → HTMAXBUTTON` on the main window has no effect there. Candidates for Phase 3:
  - (a) a thin native overlay window above the maximize button that returns `HTMAXBUTTON`
  - (b) `tauri-plugin-decorum`
  - (c) native caption buttons

  Verification is manual on Windows 11.

## Measurements / evidence
CI runs 43017487417 and 43019194222 (windows-latest):

| Metric | Run 1 | Run 2 | Target |
|---|---|---|---|
| Tab switch (host: show + hide + focus), p95 / max | 0.16 / 0.86 ms | 0.39 / 1.36 ms | ≤ 16 ms ✅ |
| Popover show (pre-created), p95 / max | 0.10 / 0.14 ms | 0.10 / 0.16 ms | ≤ 16 ms ✅ |
| 20 tabs created / loaded | 3.7 s · 20/20 | 4.8 s · 20/20 | – |

Mica and custom titlebar: screenshot `s5.png` in the artifact `s5-s2-results`. The visual frame check is manual.

> The spike code was deleted at the close of Phase 0. The source code and CI workflow are in git history at commit `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

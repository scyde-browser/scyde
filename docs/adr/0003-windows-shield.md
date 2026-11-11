# ADR-0003: Shield on Windows (WebView2): option A, DNR extension

- **Status:** Accepted (Phase 0 closed 2026-11-11)
- **Date:** 2026-11-11
- **Decided by:** Claude Code, based on spike S2; confirmation by `[OWNER]` at the close of Phase 0
- **Related:** PLAN §5.4, gate **G2**, `spikes/s5-tauri`, `spikes/s2-dnr`

## Context
WebView2 has no built-in ad blocker. G2 requires ≤ 3 % (target 0) extra time on the median page load and ≥ 95 % of fixture requests blocked.

## Options
1. **A: MV3 extension with DNR static rulesets** (`browser_extensions_enabled` + `extensions_path` in Tauri). Blocking inside Chromium's network layer, with no IPC to the host.
2. **B: `WebResourceRequested` + adblock-rust in the host.** Every request goes to the host's UI thread (an extra process hop).
3. **C: CDP `Network.setBlockedURLs`.** For measurement only.

## Decision
**Option A for network blocking**, with the host handling what DNR cannot:
- cosmetics and scriptlets through a bootstrap script (`AddScriptToExecuteOnDocumentCreated`, PLAN §5.4)
- a `NavigationStarting` interstitial for the main frame
- per-site exceptions through dynamic DNR rules

**Option B remains the fallback** in case DNR limitations become a problem. The code for B is known from the spike and works.

## Consequences
- `shield-compiler` (Phase 1) must produce DNR rulesets of ≤ 30k rules per ruleset. The spike put 106k rules into 4 rulesets (global pool), and WebView2 accepts that.
- The blocked counter on Windows comes from `declarativeNetRequestFeedback` (`onRuleMatchedDebug` is not for production), so Phase 3 will choose between `getMatchedRules` per tab or counting in the host via B for statistics only.
- All webviews (chrome, tabs, popovers) must have the same environment options (`browser_extensions_enabled`), otherwise they are not on the same environment.
- `extensions_path` loads **subfolders** as extensions.
- Option C was not measured: A and B both passed G2, and C has no advantage for production.

## Measurements / evidence
CI run 43019194222 (windows-latest), median of 3 loads, PageLoad Started → Finished:

| Page | No blocking | A (DNR) | B (host) |
|---|---|---|---|
| qa/adblock (network part 6/6) | 28 ms · 2/11 | 26 ms · **8/11** | 25 ms · **8/11** |
| index.hr | 1102 ms | 229 ms | 226 ms |
| 24sata.hr | 1172 ms | 218 ms | 219 ms |
| jutarnji.hr | 1218 ms | 945 ms | 1007 ms |
| youtube.com | 1158 ms | 831 ms | 825 ms |
| edition.cnn.com | 900 ms | 726 ms | 751 ms |

- Fixture: both options block **6/6 network** requests (100 %) and let the exception through. The remaining 3 points are cosmetics, handled by the bootstrap script (not part of the G2 network criterion).
- Extra time: **negative** on all pages (blocking speeds up loading) → G2 ✅.
- DNR conversion (spike, limited): 139k lines → 106k rules, 4 rulesets; dropped were cosmetics (24.8k), `$popup` (3k) and other (~0.3k).

> The spike code was deleted at the close of Phase 0. The source code and CI workflow are in git history at commit `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

# ADR-0006: Shield on iOS: WKContentRuleList, multiple lists, background compilation

- **Status:** Accepted (Phase 0 closed 2026-11-11)
- **Date:** 2026-11-11
- **Decided by:** Claude Code, based on spike S4
- **Related:** PLAN §5.2, §5.4, gate **G4**, `spikes/s4-ios-rulelists`

## Context
WKWebView blocks through `WKContentRuleList`, with a limit of 150k rules per list. G4 requires that ≥ 150k rules split across multiple lists compile, that the cached load is < 100 ms, and that the fixture is blocked.

## Options
1. **adblock-rust `content-blocking` conversion → multiple lists ≤ 150k**, compiled once and cached in `WKContentRuleListStore` (the plan).
2. A single list with priority rules only: coverage is lost.
3. JS interception in the page: does not see the network layer, slow and circumventable.

## Decision
**Option 1**, with rules for Phase 1 and Phase 4 (see Consequences).

## Consequences
- **Compilation is slow:** 83 s for 150k rules and 133 s for 114k rules on the CI simulator. Therefore:
  - compilation happens **only in the background**, never on the main thread and never on the path to first paint
  - the app ships a **precompiled** snapshot or starts with a smaller "core" list
  - after a bundle update, the new list is compiled in the background and then swapped atomically
  - the time on a real device is measured in Phase 4
- `shield-compiler` must filter out regexes that WebKit does not support (`|`, `{}`, `\d`/`\w`/`\s`/`\b`, lookaround, non-ASCII, `$` in the middle, literal `\^`) and report them as dropped. The spike drops 388 of 132k rules (0.3 %). Golden test in Phase 1: all output lists compile on the simulator.
- adblock-rust 0.13.5 converts `||<IP>^*` into a regex with a literal `\^` → it is dropped (see the spike README).
- The blocked counter on iOS is an estimate "≈" (WebKit does not report blocked requests).

## Measurements / evidence
CI run at `4e288eb` (`s4-ios-rulelists`, macos-latest, iOS simulator):

| Metric | Value | Target |
|---|---|---|
| Total rules | **263,790** in 2 lists (150,000 + 113,790) | ≥ 150k, ≤ 150k per list ✅ |
| Compilation | 82.5 s + 132.5 s | – (background) |
| Lookup of a single list | 4.7 ms / 6.2 ms | – |
| Cached load of all lists | **6.0 ms** | < 100 ms ✅ |
| Fixture (bundled, `file://`) | 10/11: all 6 network, exception, generic and attribute cosmetics | ✅ |

Host-specific cosmetics (`127.0.0.1##`) do not apply to `file://` and are not part of the network G4 criterion. The real lists produce 131,891 WebKit rules; the stress test duplicates them with a unique marker.

> The spike code was deleted at the close of Phase 0. The source code and CI workflow are in git history at commit `23451d6` (`spikes/`, `.github/workflows/spikes.yml`).

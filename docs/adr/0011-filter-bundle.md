# ADR-0011: Filter bundle format, DNR mapping and signing

- **Status:** Accepted
- **Date:** 2026-11-11
- **Decided by:** Claude Code
- **Related:** PLAN.md §5.2, §5.3, §11 (Phase 1.2), ADR-0003, ADR-0006, ADR-0010

## Context
`shield-compiler` turns one set of filter lists into every platform format. The S2 and S4 spikes proved the formats; Phase 1 needs a reproducible, verifiable bundle that the updater can trust (PLAN §5.3).

## Options
1. **Signing: minisign files.** Needs a minisign implementation or binary in the client.
2. **Signing: a raw ed25519 detached signature** over the exact `manifest.json` bytes, with the manifest listing a SHA-256 and the size of every file. Small (`ed25519-dalek`, `sha2`), and the same code verifies in CI, in the Rust core and through UniFFI.

## Decision
Option 2. The bundle layout:

| Path | Contents |
|---|---|
| `engine.dat` | `shield::Shield::to_dat` (ADR-0010) |
| `dnr/rules-NN.json` | DNR static rulesets, ≤ 30k rules each, ≤ 50 rulesets, 330k total by default |
| `ios/scyde-NN.json` | WKContentRuleList, ≤ 150k rules each |
| `cosmetic/generic.json`, `cosmetic/hosts.json` | cosmetic index (generic selectors keyed by class/id; per host: hide, unhide, scriptlets, procedural) |
| `resources.json` | scriptlet and redirect resources (validated by loading them into the engine) |
| `report.json` | dropped rules by reason, for every converter |
| `manifest.json` | format, version, matcher, sources with hashes, DNR and iOS indexes, `files` {bytes, sha256}, `key_id` |
| `manifest.json.sig` | base64 ed25519 signature of `manifest.json` |

**Verification** checks the signature first and only then trusts the hashes. A path with `..` or a leading `/` is rejected, and a stale signature is deleted when a bundle is rebuilt unsigned.

**Reproducibility:** no timestamps. The version comes from `--version` (`YYYYMMDD` in `filters.yml`), and adblock-rust's hash-set ordering is normalized. The same input gives the same bytes and the same signature (tested).

**DNR mapping** is our own converter, extending the S2 spike:
- **Mapped:** `||`, `|`, `^` and `*` patterns are passed through as `urlFilter`; `@@` becomes `allow` (priority 2); `$important` becomes priority 3; types and `~types`; `third-party`/`first-party`; `domain=`/`from=`; `to=`; `denyallow=`; `method=`; `match-case`; `all`.
- **`$badfilter`** cancels its target. Duplicate rules are removed.
- **Regexes** become `regexFilter` when RE2 can run them (no lookaround or backreferences), up to Chromium's 1,000 regex rules.
- **No type option means "not `main_frame`"**, like the engine; main-frame blocking is the host's interstitial (PLAN §5.4).
- **Dropped and reported:** cosmetics, scriptlets, `$popup`, `$csp`, `$redirect`, `$removeparam`, entity domains (`example.*`), non-ASCII patterns, and unknown options.

**iOS:** adblock-rust's `content-blocking` conversion, minus every `url-filter` WebKit cannot compile or never matches: non-ASCII, a literal `\^`, alternation, counted repetition, `(?`, shorthand classes and backreferences, and a `$` that is not at the end. Each is reported by reason.

## Consequences
- The public key goes to `brand/keys/filters.pub`, and the seed is the `FILTERS_SIGNING_KEY` secret `[OWNER]` (1.4). `shield-compiler keygen` prints both.
- Clients verify with `shield_compiler::bundle::verify` logic. Phase 1.9 (FFI) exposes it.
- The `ios` workflow job `shield-rulelists` compiles the iOS output of real lists and the fixture on the simulator (tags, manual runs, `ios/**` PRs).

## Measurements / evidence
- Fixture: 6 DNR rules (5 block, 1 allow); 12 iOS rules; 5 cosmetic rules indexed. Golden files are in `core/shield-compiler/tests/golden/`.
- 70k host rules → 3 DNR rulesets, with unique ids (test).

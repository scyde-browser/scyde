# ADR-0009: English only

- **Status:** Accepted
- **Date:** 2026-11-11
- **Decided by:** `[OWNER]`
- **Related:** PLAN.md (i18n), CLAUDE.md rule 10, `web/packages/i18n`

## Context
The plan required a Croatian + English UI from day one and Croatian documentation. The owner decided: "everything must be strictly in English".

## Options
1. **English only, keep the i18n infrastructure** so that languages can be added later.
2. Keep Croatian and English as planned: rejected by the owner.

## Decision
**Option 1.** All documentation, ADRs, code comments, commit messages, PR texts and the product UI are in English only. The i18n infrastructure (keys in `web/packages/i18n`, `strings.xml`, `Localizable.xcstrings`) is kept so that languages can be added later, but only `en` ships.

## Consequences
- The Croatian locale is removed from `web/packages/i18n`.
- CLAUDE.md rule 10 is changed accordingly.
- Date and number formats default to en-US (later: follow the OS).
- Croatian example sites in the QA lists stay: they are test targets, not UI.

## Measurements / evidence
n/a

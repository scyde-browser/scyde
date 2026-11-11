# Release rules

## Versions
- `vMAJOR.MINOR.PATCH`, SemVer. Before 1.0 every release is a **pre-release**.
- One version for all platforms. The version lives in `desktop/src-tauri/tauri.conf.json`, `android/app/build.gradle.kts` (`versionName`, and `versionCode` + 1), `ios/SCYDE/project.yml` (`MARKETING_VERSION`, `CURRENT_PROJECT_VERSION`) and `core/Cargo.toml`. Bump them all in one PR.

## Before tagging
- [ ] `main` is green: core, web (Playwright in 3 engines), filters, windows (silent install + smoke, `qa/adblock` 11/11), ios (simulator + .ipa), android (APKs + emulator smoke)
- [ ] budgets (PLAN §8) not exceeded: `pnpm size`, `tools/perf/compare.mjs`
- [ ] `docs/NETWORK.md` matches the code (rule 2)
- [ ] `NOTICE.md` is up to date for new dependencies
- [ ] release notes in `docs/releases/<tag>.md` (what works, what does not yet, how to install, how it was verified)
- [ ] `docs/STATUS.md` updated

## Publishing
1. Actions → `release` → Run workflow: `tag = vX.Y.Z`, `target = main`. Agent sessions cannot push tags, so the workflow creates the tag through the API.
2. The workflow builds Windows, iOS and Android as reusable workflows and publishes a GitHub pre-release with these assets:
   - `SCYDE-<tag>-windows-x64-setup.exe`
   - `SCYDE-<tag>-ios-unsigned.ipa`
   - `scyde-<ver>-<abi>.apk`
   - icons, tokens, fonts and docs
3. A platform that fails does not block the others; the job summary shows the result for each platform. Fix it and re-run with the same tag: assets are replaced (`--clobber`).

## Signing (`[OWNER]`)
| Platform | Today | Needed for v1 |
|---|---|---|
| Windows | unsigned, so SmartScreen shows a warning | code-signing certificate; `TAURI_SIGNING_PRIVATE_KEY` for the updater |
| Android | per-build test key, so updates do not install over earlier versions | release keystore secrets (`ANDROID_KEYSTORE_B64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`) |
| iOS | unsigned `.ipa`, signed by Feather on the device | none for Feather; an Apple Developer account only if TestFlight or the App Store is wanted |
| Filters | unsigned bundle | `FILTERS_SIGNING_KEY` + `brand/keys/filters.pub` (docs/filters-signing.md) |

Never commit keys. Secrets live only in GitHub Actions secrets.

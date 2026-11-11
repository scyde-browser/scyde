# android: SCYDE for Android

Kotlin + Jetpack Compose + GeckoView (PLAN §9.2, ADR-0004).

| Part | Where |
|---|---|
| Browser store (runtime, tabs as GeckoSessions, history, settings, bridge, popup blocking) | `app/src/main/java/dev/scyde/browser/Browser.kt` |
| UI (island, tab grid, predictive back, edge-to-edge) | `MainActivity.kt` |
| Shield + internal pages | the built-in extension `assets/extension` (shield-extension `gecko-mv2`: WASM engine from `engine.dat`, cosmetics, `pages/`) |
| Core | `libscyde_core.so` per ABI + UniFFI Kotlin (`core/ffi/build-android.sh`) |

Generated inputs (tokens, strings, icons, extension, jniLibs, bindings) are created by the CI `android` → `app` job; see its steps to build locally.
APKs are signed with the owner's keystore when `ANDROID_KEYSTORE_B64` and its companion secrets are set (PLAN §13.2) `[OWNER]`. Otherwise CI uses a per-build test key: the APK installs, but a later build cannot update it in place.

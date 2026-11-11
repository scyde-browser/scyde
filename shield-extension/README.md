# shield-extension

Shared code of the Shield extension with two build targets (PLAN §5.4):
- `chromium-mv3`: DNR static rulesets for WebView2 (Windows, option A)
- `gecko-mv2`: `webRequestBlocking` + WASM engine for GeckoView (Android)

The WASM engine is built by `core/shield-wasm/build.sh`: `target/shield-wasm/web` is the ES module for the gecko target. The extension itself is Phase 3/5.

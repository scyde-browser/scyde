# desktop: SCYDE for Windows

Tauri 2 + WebView2 (ADR-0007, ADR-0012). Built only on Windows (CI `windows` workflow).

| Part | Where |
|---|---|
| Host (tabs, omnibox, bridge, popup blocker, shortcuts) | `src-tauri/src/` |
| Island (chrome webview) | `web/apps/desktop-chrome` |
| Internal pages `scyde://newtab|settings|shields|library` | `web/apps/*`, bundled as resources, served by the `scyde` protocol |
| Shield | `shield-extension` (chromium-mv3) built from a `shield-compiler` bundle into `desktop/extensions/scyde-shield` |

Local build on Windows:
```powershell
pnpm install; pnpm tokens; pnpm -r --filter "./web/apps/*" build
cd core; cargo run --release -p shield-compiler -- --lists ../qa/lists --out ../bundle; cd ..
node shield-extension/build.mjs --target chromium-mv3 --bundle bundle --out desktop/extensions/scyde-shield
cd desktop; npx @tauri-apps/cli@2.12.4 build --bundles nsis
```
The installer is per-user (no admin) and downloads the WebView2 runtime if it is missing. It is unsigned until code signing is set up `[OWNER]`, so SmartScreen warns.

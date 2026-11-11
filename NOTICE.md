# NOTICE: third-party software and data in SCYDE

SCYDE is written from scratch (CLAUDE.md, rule 11). It uses the platform render engine and the libraries below. This file ships with every release; the full licence texts are in the linked projects, and the font licences are in `design/fonts/`.

The licence of SCYDE's own code is `[OWNER]`. The Rust workspace currently declares `MPL-2.0` in `core/Cargo.toml`.

## Engines and runtimes (not bundled, provided by the OS or installed separately)
| Component | Used on | Licence |
|---|---|---|
| Microsoft Edge WebView2 Runtime | Windows | Microsoft software licence (installed by the WebView2 bootstrapper) |
| WebKit / WKWebView | iOS | part of iOS |

## Bundled libraries
| Component | Used for | Licence |
|---|---|---|
| GeckoView (Mozilla) | Android render engine (approved exception, rule 11) | MPL-2.0 |
| adblock-rust (Brave) | Shield filter matching (approved exception, rule 11) | MPL-2.0 |
| UniFFI (Mozilla) | Rust ↔ Swift/Kotlin bindings | MPL-2.0 |
| Tauri, tao, wry | Windows shell | MIT OR Apache-2.0 |
| webview2-com | WebView2 COM bindings | MIT |
| rusqlite + SQLite | history and bookmarks | MIT; SQLite is public domain |
| psl (Public Suffix List data) | eTLD+1 in the omnibox | MIT / Apache-2.0; list data MPL-2.0 |
| idna, icu_properties | IDN / punycode display | MIT OR Apache-2.0 / Unicode-3.0 |
| ed25519-dalek | filter bundle signatures | BSD-3-Clause |
| serde, serde_json and other Rust crates | general | MIT and/or Apache-2.0 (full list: `cargo about` in Phase 7, `[OWNER]` to choose the tool) |
| Svelte | internal pages and the desktop island | MIT |
| Jetpack Compose, AndroidX, Kotlin stdlib | Android UI | Apache-2.0 |
| JNA | Android ↔ libscyde_core | Apache-2.0 OR LGPL-2.1 (used under Apache-2.0) |
| wasm-bindgen | Shield WASM engine (Android) | MIT OR Apache-2.0 |

## Fonts (OFL-1.1, substitutes until the Aeonik licence is confirmed `[OWNER]`)
| Font | Licence file |
|---|---|
| Geist, Geist Mono | `design/fonts/LICENSE-Geist.txt` |
| Bricolage Grotesque | `design/fonts/LICENSE-BricolageGrotesque.txt` |
| Fraunces | `design/fonts/LICENSE-Fraunces.txt` |

## Filter lists (compiled into the Shield bundle; `tools/filters/lists.txt`)
| List | Licence |
|---|---|
| EasyList, EasyPrivacy | GPL-3.0 OR CC BY-SA 3.0 |
| uBlock filters (filters, privacy, unbreak) | GPL-3.0 |
| Peter Lowe's ad server list | free use under the terms at pgl.yoyo.org |

The lists are data, compiled into rules and shipped unchanged in meaning. Their attribution travels with every bundle (`report.json` names each source).

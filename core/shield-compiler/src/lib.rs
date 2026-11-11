//! `shield-compiler`: one filter-list source → every platform format (PLAN §5.2).
//!
//! - [`dnr`]: Chromium declarativeNetRequest rulesets (Windows)
//! - [`ios`]: `WKContentRuleList` JSON (iOS)
//! - [`cosmetic`]: generic and per-host cosmetic index (all platforms)
//! - [`bundle`]: `engine.dat`, `resources.json`, manifest with hashes and ed25519 signature

pub mod bundle;
pub mod cosmetic;
pub mod dnr;
pub mod ios;

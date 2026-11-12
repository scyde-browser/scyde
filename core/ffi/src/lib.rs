//! `UniFFI` bindings of the SCYDE core for Swift (iOS) and Kotlin (Android) — `libscyde_core`.
//!
//! A thin layer: every type here maps one-to-one to a core crate. Objects are `Send + Sync`
//! (stores behind a `Mutex`), so the apps can call them from background threads; nothing here
//! should be called on the main thread for large work (engine load, list compile, 100k history).

// UniFFI exports take owned arguments (`String`, `Vec`) by design, and every fallible export
// returns `CoreError`, documented once above instead of per function.
#![allow(clippy::needless_pass_by_value, clippy::missing_errors_doc)]

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

uniffi::setup_scaffolding!("scyde_core");

/// Errors crossing the FFI boundary.
#[derive(Debug, uniffi::Error)]
pub enum CoreError {
    // The field is `detail`, not `message`: Kotlin errors extend Exception, whose `message`
    // would be redeclared by the generated bindings.
    /// Input was rejected (bad URL, invalid setting, unknown bookmark…).
    Invalid { detail: String },
    /// Storage or file problem.
    Io { detail: String },
    /// A filter bundle or `engine.dat` failed verification.
    Verification { detail: String },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid { detail } | Self::Io { detail } | Self::Verification { detail } => {
                f.write_str(detail)
            }
        }
    }
}

impl std::error::Error for CoreError {}

fn invalid(e: impl std::fmt::Display) -> CoreError {
    CoreError::Invalid {
        detail: e.to_string(),
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic while holding the lock aborts (panic = "abort"), so poisoning cannot be observed.
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

// ------------------------------------------------------------------------------------ shield

/// Network decision.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ShieldDecision {
    pub block: bool,
    pub excepted: bool,
    pub redirect: Option<String>,
    pub rewritten_url: Option<String>,
}

/// Page cosmetics.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ShieldCosmetics {
    pub hide_selectors: Vec<String>,
    pub procedural_actions: Vec<String>,
    pub exceptions: Vec<String>,
    pub injected_script: String,
    pub generichide: bool,
}

/// The Shield engine (iOS native cosmetics and counters, Windows host).
#[derive(Debug, uniffi::Object)]
pub struct ShieldEngine {
    inner: shield::Shield,
}

#[uniffi::export]
impl ShieldEngine {
    /// Loads `engine.dat` from a verified filter bundle.
    #[uniffi::constructor]
    pub fn from_dat(bytes: Vec<u8>) -> Result<Arc<Self>, CoreError> {
        shield::Shield::from_dat(&bytes)
            .map(|inner| Arc::new(Self { inner }))
            .map_err(|e| CoreError::Verification {
                detail: e.to_string(),
            })
    }

    /// Builds from list text (fallback when no bundle is available).
    #[uniffi::constructor]
    #[must_use]
    pub fn from_lists(text: String) -> Arc<Self> {
        Arc::new(Self {
            inner: shield::Shield::from_lists(&[text]),
        })
    }

    #[must_use]
    pub fn check(&self, url: String, source_url: String, request_type: String) -> ShieldDecision {
        let d = self.inner.check(
            &url,
            &source_url,
            shield::ResourceType::from_host_name(&request_type),
        );
        ShieldDecision {
            block: d.block,
            excepted: d.excepted,
            redirect: d.redirect,
            rewritten_url: d.rewritten_url,
        }
    }

    #[must_use]
    pub fn cosmetics(&self, page_url: String) -> ShieldCosmetics {
        let c = self.inner.cosmetics(&page_url);
        let mut exceptions: Vec<String> = c.exceptions.into_iter().collect();
        exceptions.sort_unstable();
        ShieldCosmetics {
            hide_selectors: c.hide_selectors,
            procedural_actions: c.procedural_actions,
            exceptions,
            injected_script: c.injected_script,
            generichide: c.generichide,
        }
    }

    #[must_use]
    pub fn hidden_selectors(
        &self,
        classes: Vec<String>,
        ids: Vec<String>,
        exceptions: Vec<String>,
    ) -> Vec<String> {
        let c: Vec<&str> = classes.iter().map(String::as_str).collect();
        let i: Vec<&str> = ids.iter().map(String::as_str).collect();
        self.inner
            .hidden_selectors(&c, &i, &exceptions.into_iter().collect())
    }
}

/// Verifies a downloaded filter bundle directory against the app's built-in public key
/// (base64). Returns the bundle version. The updater swaps engines only after this passes.
///
/// # Errors
/// [`CoreError::Verification`] for a bad key, signature or file hash.
#[uniffi::export]
pub fn verify_filter_bundle(dir: String, public_key_b64: String) -> Result<String, CoreError> {
    let key = shield_compiler::bundle::verifying_key_from_b64(&public_key_b64).map_err(|e| {
        CoreError::Verification {
            detail: e.to_string(),
        }
    })?;
    let m = shield_compiler::bundle::verify(Path::new(&dir), &key).map_err(|e| {
        CoreError::Verification {
            detail: e.to_string(),
        }
    })?;
    Ok(m["version"].as_str().unwrap_or_default().to_owned())
}

// ----------------------------------------------------------------------------------- omnibox

/// What the omnibox should do with the typed text.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum OmniboxInput {
    Url { url: String },
    Search { query: String },
    Empty,
}

#[uniffi::export]
#[must_use]
pub fn omnibox_classify(text: String) -> OmniboxInput {
    match omnibox::classify(&text) {
        omnibox::Input::Url(url) => OmniboxInput::Url { url },
        omnibox::Input::Search(query) => OmniboxInput::Search { query },
        omnibox::Input::Empty => OmniboxInput::Empty,
    }
}

#[uniffi::export]
#[must_use]
pub fn omnibox_search_url(template: String, query: String) -> String {
    omnibox::search_url(&template, &query)
}

/// A URL prepared for the omnibox: render `subdomain` dim, `domain` emphasized, `rest` dim.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DisplayUrl {
    pub scheme: String,
    pub subdomain: String,
    pub domain: String,
    pub rest: String,
    pub punycode: bool,
}

/// `None` for URLs without a host (`about:`, `mailto:`): show them verbatim.
#[uniffi::export]
#[must_use]
pub fn omnibox_display_url(url: String) -> Option<DisplayUrl> {
    omnibox::display_url(&url).map(|d| DisplayUrl {
        scheme: d.scheme,
        subdomain: d.host.subdomain,
        domain: d.host.domain,
        rest: d.rest,
        punycode: d.host.punycode,
    })
}

/// The publisher URL for a Google AMP viewer URL (`google.<tld>/amp/s/…`), `None` otherwise.
#[uniffi::export]
#[must_use]
pub fn omnibox_deamp(url: String) -> Option<String> {
    omnibox::deamp(&url)
}

/// The host shortened to `max_chars` from the start, never cutting the registrable domain.
#[uniffi::export]
#[must_use]
pub fn omnibox_elide_host(host: String, max_chars: u32) -> String {
    omnibox::elide_host(&omnibox::display_host(&host), max_chars as usize)
}

// ------------------------------------------------------------------------------------ places

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum VisitKind {
    Typed,
    Bookmark,
    Link,
    Passive,
}

impl From<VisitKind> for omnibox::frecency::VisitKind {
    fn from(k: VisitKind) -> Self {
        match k {
            VisitKind::Typed => Self::Typed,
            VisitKind::Bookmark => Self::Bookmark,
            VisitKind::Link => Self::Link,
            VisitKind::Passive => Self::Passive,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub last_visit: i64,
    pub visit_count: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Suggestion {
    pub url: String,
    pub title: String,
    pub bookmarked: bool,
    /// Eligible for inline completion.
    pub host_prefix: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Bookmark {
    pub id: i64,
    pub parent: i64,
    pub is_folder: bool,
    pub url: Option<String>,
    pub title: String,
    pub position: i64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct TopSite {
    pub url: String,
    pub title: String,
    pub pinned: bool,
}

fn places_err(e: places::PlacesError) -> CoreError {
    match e {
        places::PlacesError::Sqlite(e) => CoreError::Io {
            detail: e.to_string(),
        },
        other => invalid(other),
    }
}

/// History, bookmarks and top sites of one profile.
#[derive(Debug, uniffi::Object)]
pub struct PlacesStore {
    inner: Mutex<places::Places>,
}

#[uniffi::export]
impl PlacesStore {
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, CoreError> {
        places::Places::open(Path::new(&path))
            .map(|p| {
                Arc::new(Self {
                    inner: Mutex::new(p),
                })
            })
            .map_err(places_err)
    }

    pub fn record_visit(
        &self,
        url: String,
        title: Option<String>,
        kind: VisitKind,
        at: i64,
    ) -> Result<bool, CoreError> {
        lock(&self.inner)
            .record_visit(&url, title.as_deref(), kind.into(), at)
            .map_err(places_err)
    }

    pub fn set_title(&self, url: String, title: String) -> Result<(), CoreError> {
        lock(&self.inner)
            .set_title(&url, &title)
            .map_err(places_err)
    }

    pub fn history(
        &self,
        query: Option<String>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<HistoryEntry>, CoreError> {
        Ok(lock(&self.inner)
            .history(query.as_deref(), limit, offset)
            .map_err(places_err)?
            .into_iter()
            .map(|h| HistoryEntry {
                url: h.url,
                title: h.title,
                last_visit: h.last_visit,
                visit_count: h.visit_count,
            })
            .collect())
    }

    pub fn delete_url(&self, url: String) -> Result<(), CoreError> {
        lock(&self.inner).delete_url(&url).map_err(places_err)
    }

    pub fn delete_range(&self, from: i64, to: i64, now: i64) -> Result<(), CoreError> {
        lock(&self.inner)
            .delete_range(from, to, now)
            .map_err(places_err)
    }

    pub fn clear_history(&self) -> Result<(), CoreError> {
        lock(&self.inner).clear_history().map_err(places_err)
    }

    pub fn suggest(&self, query: String, limit: u32) -> Result<Vec<Suggestion>, CoreError> {
        Ok(lock(&self.inner)
            .suggest(&query, limit as usize)
            .map_err(places_err)?
            .into_iter()
            .map(|s| Suggestion {
                host_prefix: s.quality == omnibox::frecency::MatchQuality::HostPrefix,
                url: s.url,
                title: s.title,
                bookmarked: s.bookmarked,
            })
            .collect())
    }

    pub fn add_bookmark(
        &self,
        parent: i64,
        url: String,
        title: String,
        now: i64,
    ) -> Result<i64, CoreError> {
        lock(&self.inner)
            .add_bookmark(parent, &url, &title, now)
            .map_err(places_err)
    }

    pub fn add_folder(&self, parent: i64, title: String, now: i64) -> Result<i64, CoreError> {
        lock(&self.inner)
            .add_folder(parent, &title, now)
            .map_err(places_err)
    }

    pub fn children(&self, parent: i64) -> Result<Vec<Bookmark>, CoreError> {
        Ok(lock(&self.inner)
            .children(parent)
            .map_err(places_err)?
            .into_iter()
            .map(|b| Bookmark {
                id: b.id,
                parent: b.parent,
                is_folder: b.is_folder,
                url: b.url,
                title: b.title,
                position: b.position,
            })
            .collect())
    }

    pub fn move_bookmark(&self, id: i64, parent: i64, index: u32) -> Result<(), CoreError> {
        lock(&self.inner)
            .move_bookmark(id, parent, index as usize)
            .map_err(places_err)
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<(), CoreError> {
        lock(&self.inner).remove_bookmark(id).map_err(places_err)
    }

    pub fn is_bookmarked(&self, url: String) -> Result<bool, CoreError> {
        lock(&self.inner).is_bookmarked(&url).map_err(places_err)
    }

    /// Removes every bookmark of `url`; returns how many.
    pub fn unbookmark_url(&self, url: String) -> Result<u32, CoreError> {
        lock(&self.inner)
            .unbookmark_url(&url)
            .map(|n| u32::try_from(n).unwrap_or(u32::MAX))
            .map_err(places_err)
    }

    pub fn top_sites(&self, limit: u32) -> Result<Vec<TopSite>, CoreError> {
        Ok(lock(&self.inner)
            .top_sites(limit as usize)
            .map_err(places_err)?
            .into_iter()
            .map(|t| TopSite {
                url: t.url,
                title: t.title,
                pinned: t.pinned,
            })
            .collect())
    }

    pub fn pin_site(&self, position: u32, url: String, title: String) -> Result<(), CoreError> {
        lock(&self.inner)
            .pin_site(position, &url, &title)
            .map_err(places_err)
    }

    pub fn hide_top_site(&self, url: String) -> Result<(), CoreError> {
        lock(&self.inner).hide_top_site(&url).map_err(places_err)
    }
}

/// Built-in bookmark folders.
#[uniffi::export]
#[must_use]
pub fn bookmarks_bar_id() -> i64 {
    places::BOOKMARKS_BAR
}

#[uniffi::export]
#[must_use]
pub fn other_bookmarks_id() -> i64 {
    places::OTHER_BOOKMARKS
}

// ---------------------------------------------------------------------------------- settings

#[derive(Debug, Clone, uniffi::Record)]
pub struct SettingsIssue {
    pub path: String,
    pub message: String,
}

/// Settings of one profile. Values cross the boundary as JSON text (`"dark"`, `true`, `2`).
#[derive(Debug, uniffi::Object)]
pub struct SettingsStore {
    inner: Mutex<settings::Settings>,
    issues: Vec<SettingsIssue>,
}

#[uniffi::export]
impl SettingsStore {
    /// Loads (and repairs) stored settings; pass `None` for defaults.
    #[uniffi::constructor]
    #[must_use]
    pub fn load(json: Option<String>) -> Arc<Self> {
        let (s, issues) = json.map_or_else(
            || (settings::Settings::default(), Vec::new()),
            |j| settings::Settings::load(&j),
        );
        Arc::new(Self {
            inner: Mutex::new(s),
            issues: issues
                .into_iter()
                .map(|i| SettingsIssue {
                    path: i.path,
                    message: i.message,
                })
                .collect(),
        })
    }

    /// What `load` repaired (show once, e.g. "Some settings were reset").
    #[must_use]
    pub fn load_issues(&self) -> Vec<SettingsIssue> {
        self.issues.clone()
    }

    /// The value at a dotted path as JSON, or `None` if unknown.
    #[must_use]
    pub fn get(&self, path: String) -> Option<String> {
        lock(&self.inner).get(&path).map(ToString::to_string)
    }

    /// Sets a value given as JSON after validating it against the schema.
    pub fn set(&self, path: String, json_value: String) -> Result<(), CoreError> {
        let v: serde_json::Value = serde_json::from_str(&json_value).map_err(invalid)?;
        lock(&self.inner).set(&path, v).map_err(invalid)
    }

    pub fn reset(&self, path: String) -> Result<(), CoreError> {
        lock(&self.inner).reset(&path).map_err(invalid)
    }

    /// Everything, for storage.
    #[must_use]
    pub fn to_json(&self) -> String {
        lock(&self.inner).to_json()
    }

    #[must_use]
    pub fn search_template(&self) -> String {
        lock(&self.inner).search_template()
    }
}

/// The settings JSON Schema (for the settings page and native forms).
#[uniffi::export]
#[must_use]
pub fn settings_schema() -> String {
    settings::SCHEMA_JSON.to_owned()
}

// ------------------------------------------------------------------------------------- theme

#[derive(Debug, Clone, uniffi::Record)]
pub struct AccentRoles {
    pub fill: String,
    pub ink: String,
    pub on_fill: String,
    pub ring: String,
    pub island: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CustomAccent {
    pub light: AccentRoles,
    pub dark: AccentRoles,
    /// The fill moved away from the picked colour to pass AA (show the adjusted swatch).
    pub fill_adjusted: bool,
}

fn hex(c: theme::Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

fn roles(m: &theme::contrast::AccentMode) -> AccentRoles {
    AccentRoles {
        fill: hex(m.fill),
        ink: hex(m.ink),
        on_fill: hex(m.on_fill),
        ring: hex(m.ring),
        island: hex(m.island),
    }
}

/// Derives every accent role from a picked `#RRGGBB` (nearest AA-safe shades).
///
/// # Errors
/// [`CoreError::Invalid`] if the colour is not hex.
#[uniffi::export]
pub fn theme_derive_accent(seed_hex: String) -> Result<CustomAccent, CoreError> {
    let seed = theme::Rgb::from_hex(&seed_hex).map_err(invalid)?;
    let a = theme::accent::derive(seed);
    Ok(CustomAccent {
        light: roles(&a.light),
        dark: roles(&a.dark),
        fill_adjusted: a.fill_adjusted,
    })
}

/// Aura tint: the island background with a little of the page's `theme-color`, or `None` when it
/// would cost the island text AA contrast (or the colour is not hex).
#[uniffi::export]
#[must_use]
pub fn theme_island_tint(page_hex: String, dark: bool) -> Option<String> {
    let page = theme::Rgb::from_hex(page_hex.trim()).ok()?;
    theme::aura::island_tint(page, dark).map(hex)
}

/// WCAG contrast ratio of two `#RRGGBB` colours.
///
/// # Errors
/// [`CoreError::Invalid`] if a colour is not hex.
#[uniffi::export]
pub fn theme_contrast_ratio(a: String, b: String) -> Result<f64, CoreError> {
    Ok(theme::contrast_ratio(
        theme::Rgb::from_hex(&a).map_err(invalid)?,
        theme::Rgb::from_hex(&b).map_err(invalid)?,
    ))
}

/// Version of the core library (for about pages and bug reports).
#[uniffi::export]
#[must_use]
pub fn core_version() -> String {
    format!(
        "{} ({})",
        env!("CARGO_PKG_VERSION"),
        shield::MATCHER_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_through_the_ffi_types() {
        let e = ShieldEngine::from_lists(include_str!("../../../qa/lists/fixture.txt").into());
        let page = "http://127.0.0.1:8787/adblock/index.html".to_owned();
        assert!(
            e.check(
                "http://127.0.0.1:8787/adblock/tracker/pixel.gif".into(),
                page.clone(),
                "image".into()
            )
            .block
        );
        assert!(
            e.cosmetics(page)
                .hide_selectors
                .contains(&".scyde-specific-ad".to_owned())
        );
        assert!(matches!(
            ShieldEngine::from_dat(vec![1, 2, 3]),
            Err(CoreError::Verification { .. })
        ));

        assert_eq!(
            omnibox_classify("localhost:3000".into()),
            OmniboxInput::Url {
                url: "http://localhost:3000".into()
            }
        );
        assert!(
            omnibox_display_url("https://xn--pple-43d.com/".into())
                .unwrap()
                .punycode
        );

        let dir = std::env::temp_dir().join(format!("scyde-ffi-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p =
            PlacesStore::open(dir.join("places.sqlite").to_string_lossy().into_owned()).unwrap();
        p.record_visit(
            "https://index.hr/".into(),
            Some("Index".into()),
            VisitKind::Typed,
            1_800_000_000,
        )
        .unwrap();
        assert!(p.suggest("ind".into(), 5).unwrap()[0].host_prefix);
        assert!(
            p.add_bookmark(999, "https://x.hr/".into(), "X".into(), 0)
                .is_err()
        );

        let s = SettingsStore::load(Some(
            r#"{"version":1,"appearance":{"theme":"neon"}}"#.into(),
        ));
        assert_eq!(s.load_issues().len(), 1);
        assert_eq!(
            s.get("appearance.theme".into()).as_deref(),
            Some("\"system\"")
        );
        s.set("appearance.theme".into(), "\"dark\"".into()).unwrap();
        assert!(
            s.set("appearance.theme".into(), "neon".into()).is_err(),
            "not JSON"
        );

        let a = theme_derive_accent("#FF9F7E".into()).unwrap();
        assert_eq!(a.light.fill, "#FF9F7E");
        assert!(theme_contrast_ratio("#000000".into(), "#FFFFFF".into()).unwrap() > 20.9);
        assert!(verify_filter_bundle(dir.to_string_lossy().into_owned(), "bad".into()).is_err());
    }
}

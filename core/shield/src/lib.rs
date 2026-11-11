//! Shield: ad, tracker and popup decisions.
//!
//! A thin SCYDE layer over the approved `adblock` matcher (CLAUDE.md rule 11). It owns:
//! - the request API used by every host ([`Shield::check`]),
//! - cosmetic filtering ([`Shield::cosmetics`], [`Shield::hidden_selectors`]) and scriptlets,
//! - the `engine.dat` container with its own magic and format version ([`Shield::to_dat`]).
//!
//! The engine is `Send + Sync`; a host keeps one instance behind an `Arc`.

use std::collections::HashSet;
use std::fmt;

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::resources::Resource;

/// Magic bytes at the start of every SCYDE `engine.dat`.
pub const DAT_MAGIC: [u8; 4] = *b"SCYS";
/// Version of the SCYDE container. Bump it whenever the header layout changes.
pub const DAT_FORMAT_VERSION: u16 = 1;
/// Version of the matcher that produced the payload. A payload from another version is
/// rejected so hosts rebuild from list text instead of trusting a foreign layout.
pub const MATCHER_VERSION: &str = "adblock-0.13.5";
const _: () = assert!(MATCHER_VERSION.len() <= u8::MAX as usize);

/// What kind of resource a request loads. Mirrors the types every host can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Document,
    Subdocument,
    Script,
    Stylesheet,
    Image,
    Font,
    Media,
    Xhr,
    Websocket,
    Ping,
    Object,
    Other,
}

impl ResourceType {
    fn as_matcher_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Subdocument => "subdocument",
            Self::Script => "script",
            Self::Stylesheet => "stylesheet",
            Self::Image => "image",
            Self::Font => "font",
            Self::Media => "media",
            Self::Xhr => "xmlhttprequest",
            Self::Websocket => "websocket",
            Self::Ping => "ping",
            Self::Object => "object",
            Self::Other => "other",
        }
    }

    /// Parses the type names used by `WebView2`, `webRequest` and `WebKit` (`sub_frame`, `xhr`, …).
    #[must_use]
    pub fn from_host_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "document" | "main_frame" => Self::Document,
            "subdocument" | "sub_frame" | "iframe" => Self::Subdocument,
            "script" => Self::Script,
            "stylesheet" | "css" => Self::Stylesheet,
            "image" | "imageset" | "img" => Self::Image,
            "font" => Self::Font,
            "media" => Self::Media,
            "xhr" | "xmlhttprequest" | "fetch" => Self::Xhr,
            "websocket" => Self::Websocket,
            "ping" | "beacon" => Self::Ping,
            "object" | "object_subrequest" => Self::Object,
            _ => Self::Other,
        }
    }
}

/// The decision for one network request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    /// Cancel the request.
    pub block: bool,
    /// An exception rule (`@@`) matched; useful for the Shield panel's counters.
    pub excepted: bool,
    /// Serve this body instead (`$redirect`), as a data URL.
    pub redirect: Option<String>,
    /// Load this URL instead (`$removeparam`), only if not blocked.
    pub rewritten_url: Option<String>,
}

/// Cosmetic work for one page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cosmetics {
    /// Selectors to hide with `display: none !important`.
    pub hide_selectors: Vec<String>,
    /// JSON-encoded procedural filters / filters with an action.
    pub procedural_actions: Vec<String>,
    /// Class/id selectors excepted from generic hiding; pass back to [`Shield::hidden_selectors`].
    pub exceptions: HashSet<String>,
    /// Scriptlet code to inject at document start (empty if none).
    pub injected_script: String,
    /// `$generichide` matched: skip [`Shield::hidden_selectors`] for this page.
    pub generichide: bool,
}

/// Errors from loading lists, resources or `engine.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldError {
    /// Not a SCYDE `engine.dat` (wrong magic or too short).
    NotEngineDat,
    /// Container written by a newer/older SCYDE build.
    UnsupportedFormat { found: u16 },
    /// Payload written by a different matcher version; rebuild from lists.
    MatcherMismatch { found: String },
    /// The matcher rejected the payload (checksum or layout).
    CorruptPayload(String),
    /// `resources.json` is not valid.
    BadResources(String),
}

impl fmt::Display for ShieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotEngineDat => f.write_str("not a SCYDE engine.dat"),
            Self::UnsupportedFormat { found } => {
                write!(
                    f,
                    "engine.dat format {found} is not supported (expected {DAT_FORMAT_VERSION})"
                )
            }
            Self::MatcherMismatch { found } => {
                write!(
                    f,
                    "engine.dat built by {found}, this build uses {MATCHER_VERSION}"
                )
            }
            Self::CorruptPayload(e) => write!(f, "engine.dat payload rejected: {e}"),
            Self::BadResources(e) => write!(f, "resources.json rejected: {e}"),
        }
    }
}

impl std::error::Error for ShieldError {}

/// One compiled Shield engine.
pub struct Shield {
    engine: Engine,
}

impl fmt::Debug for Shield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shield").finish_non_exhaustive()
    }
}

impl Shield {
    /// Parses filter lists (`EasyList` / uBO syntax) and builds an optimized engine.
    #[must_use]
    pub fn from_lists<S: AsRef<str>>(lists: &[S]) -> Self {
        let mut set = FilterSet::new(false);
        for list in lists {
            set.add_filter_list(list.as_ref().to_owned(), ParseOptions::default());
        }
        Self {
            engine: Engine::new_with_filter_set(set),
        }
    }

    /// Loads scriptlet and redirect resources (`resources.json`, an array of resources).
    ///
    /// # Errors
    /// [`ShieldError::BadResources`] if the JSON does not describe resources.
    pub fn use_resources_json(&mut self, json: &str) -> Result<usize, ShieldError> {
        let resources: Vec<Resource> =
            serde_json::from_str(json).map_err(|e| ShieldError::BadResources(e.to_string()))?;
        let n = resources.len();
        self.engine.use_resources(resources);
        Ok(n)
    }

    /// Decides one network request. `source_url` is the top-level or frame document URL.
    /// Unparsable URLs are never blocked.
    #[must_use]
    pub fn check(&self, url: &str, source_url: &str, ty: ResourceType) -> Decision {
        let Ok(request) = Request::new(url, source_url, ty.as_matcher_str(), "get") else {
            return Decision::default();
        };
        let r = self.engine.check_network_request(&request);
        Decision {
            block: r.should_block(),
            excepted: r.exception.is_some(),
            redirect: r.redirect,
            rewritten_url: r.rewritten_url,
        }
    }

    /// Shorthand for `check(..).block`.
    #[must_use]
    pub fn should_block(&self, url: &str, source_url: &str, ty: ResourceType) -> bool {
        self.check(url, source_url, ty).block
    }

    /// Host-specific cosmetics and scriptlets for a page URL.
    #[must_use]
    pub fn cosmetics(&self, page_url: &str) -> Cosmetics {
        let r = self.engine.url_cosmetic_resources(page_url);
        let mut hide_selectors: Vec<String> = r.hide_selectors.into_iter().collect();
        hide_selectors.sort_unstable();
        let mut procedural_actions: Vec<String> = r.procedural_actions.into_iter().collect();
        procedural_actions.sort_unstable();
        Cosmetics {
            hide_selectors,
            procedural_actions,
            exceptions: r.exceptions,
            injected_script: r.injected_script,
            generichide: r.generichide,
        }
    }

    /// Generic hide selectors for the classes and ids seen on a page.
    #[must_use]
    pub fn hidden_selectors(
        &self,
        classes: &[&str],
        ids: &[&str],
        exceptions: &HashSet<String>,
    ) -> Vec<String> {
        let mut v = self
            .engine
            .hidden_class_id_selectors(classes, ids, exceptions);
        v.sort_unstable();
        v
    }

    /// Serializes to SCYDE `engine.dat`:
    /// `magic[4] | format u16 LE | matcher-len u8 | matcher bytes | payload`.
    #[must_use]
    pub fn to_dat(&self) -> Vec<u8> {
        let payload = self.engine.serialize();
        let mut out = Vec::with_capacity(payload.len() + 8 + MATCHER_VERSION.len());
        out.extend_from_slice(&DAT_MAGIC);
        out.extend_from_slice(&DAT_FORMAT_VERSION.to_le_bytes());
        #[allow(clippy::cast_possible_truncation)] // checked at compile time below
        out.push(MATCHER_VERSION.len() as u8);
        out.extend_from_slice(MATCHER_VERSION.as_bytes());
        out.extend_from_slice(&payload);
        out
    }

    /// Loads an `engine.dat` produced by [`Shield::to_dat`].
    ///
    /// # Errors
    /// See [`ShieldError`]; on any error the host rebuilds from list text.
    pub fn from_dat(bytes: &[u8]) -> Result<Self, ShieldError> {
        let payload = Self::split_dat(bytes)?;
        let mut engine = Engine::default();
        engine
            .deserialize(payload)
            .map_err(|e| ShieldError::CorruptPayload(e.to_string()))?;
        Ok(Self { engine })
    }

    fn split_dat(bytes: &[u8]) -> Result<&[u8], ShieldError> {
        if bytes.len() < 7 || bytes[..4] != DAT_MAGIC {
            return Err(ShieldError::NotEngineDat);
        }
        let format = u16::from_le_bytes([bytes[4], bytes[5]]);
        if format != DAT_FORMAT_VERSION {
            return Err(ShieldError::UnsupportedFormat { found: format });
        }
        let tag_len = usize::from(bytes[6]);
        let tag = bytes.get(7..7 + tag_len).ok_or(ShieldError::NotEngineDat)?;
        if tag != MATCHER_VERSION.as_bytes() {
            return Err(ShieldError::MatcherMismatch {
                found: String::from_utf8_lossy(tag).into_owned(),
            });
        }
        Ok(&bytes[7 + tag_len..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn shield_is_send_sync() {
        assert_send_sync::<Shield>();
    }

    #[test]
    fn host_type_names() {
        assert_eq!(
            ResourceType::from_host_name("sub_frame"),
            ResourceType::Subdocument
        );
        assert_eq!(ResourceType::from_host_name("XHR"), ResourceType::Xhr);
        assert_eq!(ResourceType::from_host_name("beacon"), ResourceType::Ping);
        assert_eq!(ResourceType::from_host_name("weird"), ResourceType::Other);
    }

    #[test]
    fn dat_header_errors() {
        assert_eq!(
            Shield::from_dat(b"nope").unwrap_err(),
            ShieldError::NotEngineDat
        );
        let mut dat = Shield::from_lists(&["||ads.example^"]).to_dat();
        dat[4] = 9;
        assert_eq!(
            Shield::from_dat(&dat).unwrap_err(),
            ShieldError::UnsupportedFormat { found: 9 }
        );
        let mut dat = Shield::from_lists(&["||ads.example^"]).to_dat();
        dat[7] = b'X';
        assert!(matches!(
            Shield::from_dat(&dat),
            Err(ShieldError::MatcherMismatch { .. })
        ));
        let mut dat = Shield::from_lists(&["||ads.example^"]).to_dat();
        let last = dat.len() - 1;
        dat[last] ^= 0xff;
        assert!(matches!(
            Shield::from_dat(&dat),
            Err(ShieldError::CorruptPayload(_))
        ));
    }

    #[test]
    fn unparsable_url_is_allowed() {
        let s = Shield::from_lists(&["*"]);
        assert!(!s.should_block("not a url", "", ResourceType::Script));
    }
}

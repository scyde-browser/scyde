//! Omnibox: URL fixup, URL-or-search, safe URL display and frecency ranking (PLAN §11, §7).
//!
//! - [`input`]: what the user typed → URL or search
//! - [`display`]: eTLD+1 emphasis, punycode for confusable IDNs, elision that keeps the domain
//! - [`frecency`]: suggestion scoring, matching and inline completion
//! - [`deamp`]: Google AMP viewer URLs → the publisher's URL

pub mod deamp;
pub mod display;
pub mod frecency;
pub mod input;

pub use deamp::deamp;
pub use display::{HostDisplay, UrlDisplay, display_host, display_url, elide_host};
pub use input::{DEFAULT_SEARCH_TEMPLATE, Input, classify, search_url};

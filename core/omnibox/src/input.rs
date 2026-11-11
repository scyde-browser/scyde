//! What the user typed → a URL to load or a search (PLAN §11 Phase 1.5).
//!
//! Heuristics, in order:
//! 1. `?query` forces a search.
//! 2. A known scheme (`https:`, `ftp:`, `scyde:`, …) is a URL. `javascript:` and `data:` are
//!    treated as searches so a pasted payload never runs from the omnibox.
//! 3. `host[:port][/path]` is a URL when the host is an IP address, `localhost`, a single label
//!    with an explicit port, or a name whose suffix is on the Public Suffix List.
//! 4. Anything else, or anything with whitespace before the path, is a search.

/// Classification of omnibox input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Navigate to this absolute URL.
    Url(String),
    /// Search for this text with the default engine.
    Search(String),
    /// Nothing to do (empty input).
    Empty,
}

/// Internal pages under `scyde://` (PLAN §3).
pub const INTERNAL_PAGES: &[&str] = &[
    "newtab",
    "settings",
    "shields",
    "history",
    "bookmarks",
    "downloads",
    "about",
];

const URL_SCHEMES: &[&str] = &[
    "http",
    "https",
    "ftp",
    "file",
    "scyde",
    "about",
    "view-source",
    "mailto",
];
const SEARCH_ONLY_SCHEMES: &[&str] = &["javascript", "data", "vbscript"];

/// Classifies and fixes up omnibox input.
#[must_use]
pub fn classify(raw: &str) -> Input {
    let text = raw.trim();
    if text.is_empty() {
        return Input::Empty;
    }
    if let Some(q) = text.strip_prefix('?') {
        let q = q.trim();
        return if q.is_empty() {
            Input::Empty
        } else {
            Input::Search(q.to_owned())
        };
    }
    if let Some(url) = with_scheme(text) {
        return url;
    }
    without_scheme(text).map_or_else(|| Input::Search(text.to_owned()), Input::Url)
}

fn scheme_of(text: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = text.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic() && chars.all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)))
        .then_some((scheme, rest))
}

fn with_scheme(text: &str) -> Option<Input> {
    let (scheme, rest) = scheme_of(text)?;
    let lower = scheme.to_ascii_lowercase();
    if SEARCH_ONLY_SCHEMES.contains(&lower.as_str()) {
        return Some(Input::Search(text.to_owned()));
    }
    if lower == "scyde" {
        // `scyde:settings`, `scyde://settings`, `scyde:///settings` → `scyde://settings`
        let page = rest.trim_start_matches('/');
        if page.is_empty() {
            return Some(Input::Url("scyde://newtab".into()));
        }
        return Some(Input::Url(format!("scyde://{page}")));
    }
    if !URL_SCHEMES.contains(&lower.as_str()) {
        // `localhost:3000`, `example.com:8080/x`: a host and a port, not a scheme.
        return None;
    }
    if matches!(lower.as_str(), "http" | "https" | "ftp") {
        let after = rest.trim_start_matches('/');
        if after.is_empty() {
            return Some(Input::Search(text.to_owned()));
        }
        // Normalise `http:/x.com` and `HTTPS:x.com` to `scheme://host…`, and IDN hosts to ASCII.
        let (host_port, tail) = split_host(after);
        let (host, port) = split_port(host_port)?;
        let ascii = host_to_ascii(host)?;
        let port = port.map(|p| format!(":{p}")).unwrap_or_default();
        return Some(Input::Url(format!("{lower}://{ascii}{port}{tail}")));
    }
    if text.chars().any(char::is_whitespace) && lower != "mailto" {
        return Some(Input::Search(text.to_owned()));
    }
    Some(Input::Url(format!("{lower}:{rest}")))
}

/// `host[:port]` and the rest (`/path?query#frag`).
fn split_host(s: &str) -> (&str, &str) {
    let end = s.find(['/', '?', '#']).unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// Splits `host:port`; `None` if the port is not a valid number. IPv6 literals stay intact.
fn split_port(host_port: &str) -> Option<(&str, Option<u16>)> {
    if host_port.starts_with('[') {
        let close = host_port.find(']')?;
        let (h, rest) = host_port.split_at(close + 1);
        return match rest.strip_prefix(':') {
            Some(p) => Some((h, Some(p.parse().ok()?))),
            None if rest.is_empty() => Some((h, None)),
            None => None,
        };
    }
    match host_port.rsplit_once(':') {
        Some((h, p)) => Some((h, Some(p.parse().ok()?))),
        None => Some((host_port, None)),
    }
}

/// Lower-case ASCII (punycode) host, or `None` if it cannot be a host.
pub(crate) fn host_to_ascii(host: &str) -> Option<String> {
    if host.is_empty() || host.contains(['@', ' ', '\\', '%']) {
        return None;
    }
    if host.starts_with('[') {
        return is_ipv6_literal(host).then(|| host.to_ascii_lowercase());
    }
    let ascii = idna::domain_to_ascii(host).ok()?;
    let ok = !ascii.is_empty()
        && ascii.split('.').enumerate().all(|(i, l)| {
            // a trailing dot (empty last label) is allowed
            (!l.is_empty() || i == ascii.split('.').count() - 1)
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        });
    ok.then_some(ascii)
}

fn is_ipv6_literal(h: &str) -> bool {
    h.strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .is_some_and(|inner| inner.contains(':') && inner.parse::<std::net::Ipv6Addr>().is_ok())
}

pub(crate) fn is_ipv4(h: &str) -> bool {
    h.parse::<std::net::Ipv4Addr>().is_ok()
}

fn without_scheme(text: &str) -> Option<String> {
    let (host_port, tail) = split_host(text);
    if host_port.chars().any(char::is_whitespace) || host_port.is_empty() {
        return None;
    }
    let (host, port) = split_port(host_port)?;
    let ascii = host_to_ascii(host)?;
    let bare = ascii.trim_end_matches('.');
    let port_s = port.map(|p| format!(":{p}")).unwrap_or_default();

    let local = is_ipv4(bare)
        || bare.starts_with('[')
        || bare == "localhost"
        || bare.ends_with(".localhost");
    if local {
        return Some(format!("http://{ascii}{port_s}{tail}"));
    }
    if !bare.contains('.') {
        // `router:8080/admin` is a URL; a lone word (`scyde`, `weather`) is a search.
        return port.map(|_| format!("http://{ascii}{port_s}{tail}"));
    }
    if tail.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let known_suffix = psl::suffix(bare.as_bytes()).is_some_and(|s| s.is_known());
    known_suffix.then(|| format!("https://{ascii}{port_s}{tail}"))
}

/// Builds the search URL from an OpenSearch-style template with `{searchTerms}`.
#[must_use]
pub fn search_url(template: &str, query: &str) -> String {
    let mut enc = String::with_capacity(query.len() * 3);
    for b in query.trim().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                enc.push(char::from(b));
            }
            b' ' => enc.push('+'),
            _ => {
                use std::fmt::Write as _;
                let _ = write!(enc, "%{b:02X}");
            }
        }
    }
    template.replace("{searchTerms}", &enc)
}

/// `DuckDuckGo`, the default engine until the owner decides (STATUS open question 2).
pub const DEFAULT_SEARCH_TEMPLATE: &str = "https://duckduckgo.com/?q={searchTerms}";

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> String {
        match classify(s) {
            Input::Url(u) => u,
            other => panic!("{s:?} → {other:?}, expected URL"),
        }
    }

    fn search(s: &str) {
        assert!(
            matches!(classify(s), Input::Search(_)),
            "{s:?} → {:?}, expected search",
            classify(s)
        );
    }

    #[test]
    fn plan_fixups() {
        // PLAN §11 Phase 1 AC: scyde, localhost:3000, 192.168.1.1, ftp://
        search("scyde");
        assert_eq!(url("scyde://settings"), "scyde://settings");
        assert_eq!(url("scyde:shields"), "scyde://shields");
        assert_eq!(url("scyde:"), "scyde://newtab");
        assert_eq!(url("localhost:3000"), "http://localhost:3000");
        assert_eq!(
            url("localhost:3000/api?x=1"),
            "http://localhost:3000/api?x=1"
        );
        assert_eq!(url("192.168.1.1"), "http://192.168.1.1");
        assert_eq!(
            url("192.168.1.1:8080/admin"),
            "http://192.168.1.1:8080/admin"
        );
        assert_eq!(
            url("ftp://ftp.example.org/pub"),
            "ftp://ftp.example.org/pub"
        );
    }

    #[test]
    fn urls() {
        assert_eq!(url("example.com"), "https://example.com");
        assert_eq!(
            url("Example.COM/Path?Q=1#F"),
            "https://example.com/Path?Q=1#F"
        );
        assert_eq!(url("http://example.com"), "http://example.com");
        assert_eq!(url("HTTPS:example.com"), "https://example.com");
        assert_eq!(url("index.hr"), "https://index.hr");
        assert_eq!(url("router:8080"), "http://router:8080");
        assert_eq!(url("app.localhost"), "http://app.localhost");
        assert_eq!(url("[::1]:8080/x"), "http://[::1]:8080/x");
        assert_eq!(url("žurnal.hr"), "https://xn--urnal-tib.hr");
        assert_eq!(url("https://bücher.de/a"), "https://xn--bcher-kva.de/a");
        assert_eq!(url("mailto:a@b.hr"), "mailto:a@b.hr");
    }

    #[test]
    fn searches() {
        search("weather zagreb");
        search("rust lang");
        search("what is example.com");
        search("foo.notatld");
        search("1.5");
        search("user@example.com");
        search("javascript:alert(1)");
        search("data:text/html,<script>");
        search("http://");
        assert_eq!(
            classify("?example.com"),
            Input::Search("example.com".into())
        );
        assert_eq!(classify("   "), Input::Empty);
        assert_eq!(classify("?"), Input::Empty);
    }

    #[test]
    fn search_templates() {
        assert_eq!(
            search_url(DEFAULT_SEARCH_TEMPLATE, "čaj & kava?"),
            "https://duckduckgo.com/?q=%C4%8Daj+%26+kava%3F"
        );
    }
}

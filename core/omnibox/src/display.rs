//! Safe URL display (PLAN §7, CLAUDE.md rule 6: the omnibox never hides the real domain).
//!
//! - The registrable domain (eTLD+1) is split out so the UI can emphasize it.
//! - Hosts are shown in Unicode only when that cannot be confused with another domain:
//!   one script per label (or an accepted CJK + Latin mix), no lookalike letters that pose
//!   as Latin, and no symbols. Anything else is shown as punycode (`xn--…`).
//! - Elision removes subdomain characters from the start and never touches the domain.

use std::collections::BTreeSet;

use icu_properties::CodePointMapData;
use icu_properties::props::{GeneralCategory, Script};

use crate::input::{host_to_ascii, is_ipv4};

/// A host prepared for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDisplay {
    /// Everything before the registrable domain, including the trailing dot (`www.`), or "".
    pub subdomain: String,
    /// The registrable domain (eTLD+1), or the whole host for IPs, `localhost` and unknown suffixes.
    pub domain: String,
    /// `true` if the host was kept as punycode because Unicode would be unsafe.
    pub punycode: bool,
}

impl HostDisplay {
    /// The full host as displayed.
    #[must_use]
    pub fn full(&self) -> String {
        format!("{}{}", self.subdomain, self.domain)
    }
}

/// Cyrillic and Greek letters that look like Latin letters (UTS #39 whole-script confusables,
/// the subset that matters in hostnames).
const LATIN_LOOKALIKES: &[char] = &[
    // Cyrillic
    'а', 'в', 'е', 'к', 'м', 'н', 'о', 'р', 'с', 'т', 'у', 'х', 'ѕ', 'і', 'ј', 'ԁ', 'ӏ', 'һ', 'ԛ',
    'ԝ', 'ү', 'ɡ', 'ь', 'ц', 'п', 'г', 'д', 'ш', 'з', 'э', 'ъ', // Greek
    'α', 'β', 'γ', 'ε', 'ι', 'κ', 'ν', 'ο', 'ρ', 'τ', 'υ', 'χ', 'ω', 'η', 'μ',
];

/// Latin-script letters that imitate other ASCII letters.
const LATIN_CONFUSABLE: &[char] = &[
    'ı', 'ȷ', 'ɑ', 'ɡ', 'ɩ', 'ɪ', 'ʏ', 'ʟ', 'ʀ', 'ᴀ', 'ᴄ', 'ᴅ', 'ᴇ', 'ᴏ', 'ᴜ', 'ꞵ', 'ſ',
];

fn tld_script(tld_unicode: &str) -> Option<Script> {
    let map = CodePointMapData::<Script>::new();
    tld_unicode
        .chars()
        .map(|c| map.get(c))
        .find(|s| *s != Script::Common && *s != Script::Inherited)
}

/// Is this Unicode label safe to show as Unicode?
fn label_is_safe(label: &str, tld: Option<Script>) -> bool {
    if label.is_ascii() {
        return true;
    }
    let scripts_map = CodePointMapData::<Script>::new();
    let gc = CodePointMapData::<GeneralCategory>::new();
    let mut scripts = BTreeSet::new();
    for c in label.chars() {
        if c == '-' || c.is_ascii_digit() {
            continue;
        }
        let cat = gc.get(c);
        let letter_or_mark = matches!(
            cat,
            GeneralCategory::LowercaseLetter
                | GeneralCategory::UppercaseLetter
                | GeneralCategory::OtherLetter
                | GeneralCategory::ModifierLetter
                | GeneralCategory::NonspacingMark
                | GeneralCategory::SpacingMark
                | GeneralCategory::DecimalNumber
        );
        if !letter_or_mark || LATIN_CONFUSABLE.contains(&c) {
            return false;
        }
        let s = scripts_map.get(c);
        if s != Script::Common && s != Script::Inherited {
            scripts.insert(s);
        }
    }
    let ids = |list: &[Script]| list.iter().copied().collect::<BTreeSet<_>>();
    let allowed_mixes = [
        ids(&[
            Script::Latin,
            Script::Han,
            Script::Hiragana,
            Script::Katakana,
        ]),
        ids(&[Script::Latin, Script::Han, Script::Bopomofo]),
        ids(&[Script::Latin, Script::Han, Script::Hangul]),
    ];
    if scripts.len() > 1 && !allowed_mixes.iter().any(|m| scripts.is_subset(m)) {
        return false; // mixed scripts: `аpple` (Cyrillic а + Latin pple)
    }
    // Whole-script confusable: an all-lookalike Cyrillic/Greek label under a non-matching TLD.
    let single = scripts.iter().next().copied();
    let lookalike_script = single.is_some_and(|s| s == Script::Cyrillic || s == Script::Greek);
    if lookalike_script && tld != single {
        let all_lookalike = label
            .chars()
            .filter(|c| *c != '-' && !c.is_ascii_digit())
            .all(|c| LATIN_LOOKALIKES.contains(&c));
        if all_lookalike {
            return false;
        }
    }
    true
}

/// Prepares a host (ASCII/punycode or Unicode input) for display.
#[must_use]
pub fn display_host(host: &str) -> HostDisplay {
    let Some(ascii) = host_to_ascii(host) else {
        return HostDisplay {
            subdomain: String::new(),
            domain: host.to_owned(),
            punycode: false,
        };
    };
    let bare = ascii.trim_end_matches('.');
    if is_ipv4(bare) || bare.starts_with('[') || !bare.contains('.') {
        return HostDisplay {
            subdomain: String::new(),
            domain: ascii,
            punycode: false,
        };
    }
    let (unicode, ok) = idna::domain_to_unicode(bare);
    let tld = unicode.rsplit('.').next().and_then(tld_script);
    let safe = ok.is_ok() && unicode.split('.').all(|l| label_is_safe(l, tld));
    let shown = if safe { unicode } else { bare.to_owned() };
    let punycode = !safe && bare.contains("xn--");

    // eTLD+1 on the ASCII form, mapped onto the shown form by label count.
    let labels = shown.split('.').count();
    let domain_labels = psl::domain_str(bare).map_or(labels, |d| d.split('.').count());
    let cut = labels.saturating_sub(domain_labels);
    let parts: Vec<&str> = shown.split('.').collect();
    let subdomain = if cut == 0 {
        String::new()
    } else {
        format!("{}.", parts[..cut].join("."))
    };
    HostDisplay {
        subdomain,
        domain: parts[cut..].join("."),
        punycode,
    }
}

/// Fits a host into `max_chars` by removing subdomain characters from the start.
/// The registrable domain is never shortened, even if it alone exceeds `max_chars`.
#[must_use]
pub fn elide_host(h: &HostDisplay, max_chars: usize) -> String {
    let full = h.full();
    if full.chars().count() <= max_chars || h.subdomain.is_empty() {
        return full;
    }
    let domain_len = h.domain.chars().count();
    let keep = max_chars.saturating_sub(domain_len + 1); // 1 for the ellipsis
    let sub: Vec<char> = h.subdomain.chars().collect();
    let tail: String = sub[sub.len().saturating_sub(keep)..].iter().collect();
    format!("…{tail}{}", h.domain)
}

/// A URL split for the omnibox: `scheme`, host parts, and the rest (port, path, query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlDisplay {
    pub scheme: String,
    pub host: HostDisplay,
    /// `:port/path?query#fragment` as given.
    pub rest: String,
}

/// Splits an absolute `scheme://host…` URL for display; `None` for other URLs
/// (`about:`, `mailto:`), which the UI shows verbatim.
#[must_use]
pub fn display_url(url: &str) -> Option<UrlDisplay> {
    let (scheme, after) = url.split_once("://")?;
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let authority = &after[..end];
    // Never display credentials: `https://google.com@evil.example` shows `evil.example`.
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let (host, port) = if host_port.starts_with('[') {
        let close = host_port.find(']').map_or(host_port.len(), |i| i + 1);
        host_port.split_at(close)
    } else {
        host_port
            .rsplit_once(':')
            .map_or((host_port, ""), |(h, _)| (h, &host_port[h.len()..]))
    };
    Some(UrlDisplay {
        scheme: scheme.to_ascii_lowercase(),
        host: display_host(host),
        rest: format!("{port}{}", &after[end..]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_homograph_is_punycode() {
        // PLAN §11 AC: `аpple.com` with Cyrillic "а" is displayed as punycode.
        let h = display_host("аpple.com");
        assert_eq!(h.full(), "xn--pple-43d.com");
        assert!(h.punycode);
        assert_eq!(display_host("xn--pple-43d.com").full(), "xn--pple-43d.com");
    }

    #[test]
    fn whole_script_confusables() {
        // all-Cyrillic lookalike of "epic" under .com
        assert!(display_host("ерус.com").punycode);
        // Cyrillic under a Cyrillic TLD is fine
        assert_eq!(display_host("пример.рф").full(), "пример.рф");
        // Cyrillic with non-lookalike letters is fine
        assert_eq!(display_host("бжд.com").full(), "бжд.com");
        // Latin lookalike letters
        assert!(display_host("gooɡle.com").punycode);
        assert!(display_host("paypaı.com").punycode);
    }

    #[test]
    fn legitimate_unicode() {
        assert_eq!(display_host("žurnal.hr").full(), "žurnal.hr");
        assert_eq!(display_host("xn--urnal-tib.hr").full(), "žurnal.hr");
        assert_eq!(display_host("bücher.de").full(), "bücher.de");
        assert_eq!(display_host("例え.jp").full(), "例え.jp");
        assert_eq!(display_host("ελληνικά.gr").full(), "ελληνικά.gr");
    }

    #[test]
    fn registrable_domain() {
        let h = display_host("www.news.bbc.co.uk");
        assert_eq!(
            (h.subdomain.as_str(), h.domain.as_str()),
            ("www.news.", "bbc.co.uk")
        );
        let h = display_host("192.168.1.1");
        assert_eq!(
            (h.subdomain.as_str(), h.domain.as_str()),
            ("", "192.168.1.1")
        );
        assert_eq!(display_host("localhost").domain, "localhost");
        assert_eq!(display_host("a.b.xn--urnal-tib.hr").subdomain, "a.b.");
    }

    #[test]
    fn elision_keeps_the_domain() {
        let h = display_host("login.secure.account.verify.paypal.com.evil.example");
        let e = elide_host(&h, 24);
        assert!(e.ends_with("evil.example"), "{e}");
        assert!(e.starts_with('…'));
        assert!(e.chars().count() <= 24);
        let h = display_host("www.example.com");
        assert_eq!(elide_host(&h, 3), "…example.com");
        assert_eq!(elide_host(&h, 100), "www.example.com");
    }

    #[test]
    fn urls() {
        let d = display_url("https://google.com@evil.example:8443/login?x").unwrap();
        assert_eq!(d.host.domain, "evil.example");
        assert_eq!(d.rest, ":8443/login?x");
        let d = display_url("https://xn--pple-43d.com/").unwrap();
        assert!(d.host.punycode);
        assert_eq!(display_url("about:blank"), None);
        let d = display_url("http://[::1]:3000/").unwrap();
        assert_eq!(
            (d.host.domain.as_str(), d.rest.as_str()),
            ("[::1]", ":3000/")
        );
    }
}

//! Property tests for omnibox parsing (PLAN §14.1).

use omnibox::{Input, classify, display_host, display_url, elide_host};
use proptest::prelude::*;

fn label() -> impl Strategy<Value = String> {
    // `xn--` labels are punycode and must decode; they get their own tests.
    "[a-z][a-z0-9-]{0,10}[a-z0-9]".prop_filter("not punycode", |l| !l.starts_with("xn--"))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2_000, .. ProptestConfig::default() })]

    /// Any input classifies without panicking, and URLs are absolute and whitespace-free.
    #[test]
    fn classify_never_panics(s in "\\PC{0,80}") {
        match classify(&s) {
            Input::Url(u) => {
                prop_assert!(u.contains(':'));
                prop_assert!(!u.chars().any(char::is_whitespace) || u.starts_with("mailto:"));
                prop_assert!(!u.to_ascii_lowercase().starts_with("javascript:"));
                prop_assert!(!u.to_ascii_lowercase().starts_with("data:"));
            }
            Input::Search(q) => prop_assert!(!q.trim().is_empty()),
            Input::Empty => prop_assert!(s.trim().is_empty() || s.trim() == "?"),
        }
    }

    /// `host.tld[/path]` with a real suffix is always an https URL for that host.
    #[test]
    fn domains_become_https(a in label(), b in label(), tld in prop::sample::select(vec!["com", "hr", "org", "co.uk", "de"]), path in "(/[a-z0-9]{0,8}){0,3}") {
        let input = format!("{a}.{b}.{tld}{path}");
        prop_assert_eq!(classify(&input), Input::Url(format!("https://{input}")));
    }

    /// Text with a space before any slash is a search, never a navigation.
    #[test]
    fn spaced_text_is_search(a in "[a-z]{1,8}", b in "[a-z.]{1,12}") {
        let input = format!("{a} {b}");
        prop_assert!(matches!(classify(&input), Input::Search(_)));
    }

    /// The registrable domain is always kept whole by elision, at any width.
    #[test]
    fn elision_keeps_domain(subs in prop::collection::vec(label(), 0..6), d in label(), width in 0usize..60) {
        let host = format!("{}{d}.com", subs.iter().map(|s| s.clone() + ".").collect::<String>());
        let h = display_host(&host);
        // `{d}.com` itself may be a private PSL suffix (e.g. x0.com), so the domain can be longer.
        let want = format!("{d}.com");
        prop_assert!(h.domain.ends_with(&want));
        let e = elide_host(&h, width);
        prop_assert!(e.ends_with(&h.domain));
        prop_assert!(e.chars().count() <= width.max(h.domain.chars().count() + 1).max(h.full().chars().count().min(width)));
    }

    /// Display is idempotent and never hides a mixed-script label: one Cyrillic letter in a Latin
    /// label always shows as punycode.
    #[test]
    fn mixed_script_is_punycode(prefix in "[a-z]{1,6}", suffix in "[a-z]{0,6}", cyr in prop::sample::select(vec!['а', 'е', 'о', 'р', 'с', 'х'])) {
        let host = format!("{prefix}{cyr}{suffix}.com");
        let h = display_host(&host);
        prop_assert!(h.punycode, "{host} shown as {}", h.full());
        prop_assert!(h.full().is_ascii());
        prop_assert_eq!(display_host(&h.full()), h);
    }

    /// Credentials never change the displayed host.
    #[test]
    fn userinfo_is_ignored(user in "[a-z.]{1,12}", host in label()) {
        let url = format!("https://{user}@{host}.com/x");
        let d = display_url(&url).unwrap();
        prop_assert_eq!(d.host.full(), format!("{host}.com"));
    }
}

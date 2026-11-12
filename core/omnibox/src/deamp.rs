//! De-AMP: Google's AMP viewer (`https://www.google.<tld>/amp/s/<site>/<path>`) shows a publisher's
//! page under a Google URL, so the omnibox would show google.com for someone else's article
//! (CLAUDE.md rule 6: never hide the real domain). The browser loads the publisher's page instead.

/// The publisher URL for a Google AMP viewer URL, `None` for anything else.
#[must_use]
pub fn deamp(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    if !is_google(host) {
        return None;
    }
    let (scheme, target) = if let Some(t) = path.strip_prefix("amp/s/") {
        ("https", t)
    } else {
        ("http", path.strip_prefix("amp/")?)
    };
    let target_host = target.split(['/', '?', '#']).next()?;
    // A real host: has a dot, no credentials or port tricks, not Google again.
    if !target_host.contains('.')
        || target_host.contains(['@', ':', '\\', ' '])
        || target_host.starts_with('.')
        || is_google(target_host)
    {
        return None;
    }
    Some(format!("{scheme}://{target}"))
}

/// `google.com`, `www.google.co.uk`, `google.hr`: the registrable domain is google.<public suffix>.
fn is_google(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    psl::domain_str(&host).is_some_and(|d| d.split('.').next() == Some("google"))
}

#[cfg(test)]
mod tests {
    use super::deamp;

    #[test]
    fn rewrites_google_amp_viewer_urls() {
        assert_eq!(
            deamp("https://www.google.com/amp/s/www.index.hr/vijesti/clanak/x/123.aspx").as_deref(),
            Some("https://www.index.hr/vijesti/clanak/x/123.aspx")
        );
        assert_eq!(
            deamp("https://www.google.co.uk/amp/s/example.co.uk/a?b=1#c").as_deref(),
            Some("https://example.co.uk/a?b=1#c")
        );
        assert_eq!(
            deamp("https://google.hr/amp/example.com/a").as_deref(),
            Some("http://example.com/a")
        );
    }

    #[test]
    fn leaves_everything_else_alone() {
        for url in [
            "https://www.google.com/search?q=amp",
            "https://www.google.com/amp/",
            "https://www.google.com/amp/s/",
            "https://www.google.com/amp/s/localhost/x",
            "https://www.google.com/amp/s/user@evil.com/x",
            "https://www.google.com/amp/s/www.google.com/amp/s/x.com",
            "https://notgoogle.com/amp/s/example.com/",
            "https://google.example.com/amp/s/example.com/",
            "https://example.com/amp/s/example.org/",
            "scyde://settings/",
        ] {
            assert_eq!(deamp(url), None, "{url}");
        }
    }
}

//! Frecency: frequency × recency scoring for history suggestions, and query matching.
//!
//! Our own model (inspired by the general idea, not by any browser's code): each of the
//! most recent visits contributes `age weight × visit-kind weight`; the sum is scaled up by the
//! total visit count, so a page visited often long ago still beats a page seen once yesterday,
//! but recency dominates within the same order of magnitude.

/// How the user arrived at a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitKind {
    /// Typed in the omnibox or picked from suggestions.
    Typed,
    /// Opened from a bookmark.
    Bookmark,
    /// Followed a link.
    Link,
    /// Redirect or reload: does not express intent.
    Passive,
}

impl VisitKind {
    fn weight(self) -> f64 {
        match self {
            Self::Typed => 2.0,
            Self::Bookmark => 1.4,
            Self::Link => 1.0,
            Self::Passive => 0.0,
        }
    }
}

/// One visit: when (seconds since the Unix epoch) and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit {
    pub at: i64,
    pub kind: VisitKind,
}

/// At most this many recent visits are sampled (enough signal, bounded cost).
pub const SAMPLE: usize = 10;

const DAY: i64 = 86_400;

fn age_weight(age_secs: i64) -> f64 {
    match age_secs / DAY {
        ..=3 => 100.0,
        4..=13 => 70.0,
        14..=30 => 50.0,
        31..=90 => 30.0,
        _ => 10.0,
    }
}

/// Frecency score. `recent` should hold the latest visits (any order); `total_visits` is the
/// full count, including visits outside the sample. A bookmark adds a floor so bookmarked pages
/// never vanish from suggestions.
#[must_use]
pub fn frecency(recent: &[Visit], total_visits: u32, bookmarked: bool, now: i64) -> f64 {
    let mut sorted: Vec<&Visit> = recent.iter().collect();
    sorted.sort_by_key(|v| std::cmp::Reverse(v.at));
    let sample = &sorted[..sorted.len().min(SAMPLE)];
    let points: f64 = sample
        .iter()
        .map(|v| age_weight(now - v.at) * v.kind.weight())
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let scaled = if sample.is_empty() {
        0.0
    } else {
        f64::from(total_visits.max(1)) * points / sample.len() as f64
    };
    if bookmarked {
        scaled.max(140.0)
    } else {
        scaled
    }
}

/// How well a candidate matches the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchQuality {
    None,
    /// Query found inside the URL or title.
    Substring,
    /// Query starts a word of the title or a path segment.
    WordStart,
    /// Query is a prefix of the host (after `www.`): eligible for inline autocomplete.
    HostPrefix,
}

impl MatchQuality {
    fn boost(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Substring => 1.0,
            Self::WordStart => 2.0,
            Self::HostPrefix => 4.0,
        }
    }
}

/// A suggestion candidate (from history or bookmarks).
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub url: String,
    pub title: String,
    pub frecency: f64,
}

fn strip_scheme_www(url: &str) -> &str {
    let u = url.split_once("://").map_or(url, |(_, r)| r);
    u.strip_prefix("www.").unwrap_or(u)
}

/// Matches one candidate against the (already lowercased, trimmed) query.
#[must_use]
pub fn match_quality(query: &str, c: &Candidate) -> MatchQuality {
    if query.is_empty() {
        return MatchQuality::None;
    }
    let url = c.url.to_lowercase();
    let host_and_path = strip_scheme_www(&url);
    if host_and_path.starts_with(query) {
        return MatchQuality::HostPrefix;
    }
    let title = c.title.to_lowercase();
    let word_start = |hay: &str| {
        hay.match_indices(query).any(|(i, _)| {
            i == 0
                || hay[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|p| !p.is_alphanumeric())
        })
    };
    if word_start(&title) || word_start(host_and_path) {
        return MatchQuality::WordStart;
    }
    if title.contains(query) || url.contains(query) {
        return MatchQuality::Substring;
    }
    MatchQuality::None
}

/// Ranks candidates for a query: best first, non-matches removed, at most `limit`.
#[must_use]
pub fn rank(query: &str, candidates: &[Candidate], limit: usize) -> Vec<(Candidate, MatchQuality)> {
    let q = query.trim().to_lowercase();
    let mut scored: Vec<(f64, MatchQuality, &Candidate)> = candidates
        .iter()
        .filter_map(|c| {
            let m = match_quality(&q, c);
            (m != MatchQuality::None).then(|| (c.frecency.max(1.0) * m.boost(), m, c))
        })
        .collect();
    // Host-prefix matches come first (they own the inline-completion slot); then by score.
    scored.sort_by(|a, b| {
        let tier = |m: MatchQuality| m == MatchQuality::HostPrefix;
        tier(b.1)
            .cmp(&tier(a.1))
            .then_with(|| b.0.total_cmp(&a.0))
            .then_with(|| a.2.url.len().cmp(&b.2.url.len()))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(_, m, c)| (c.clone(), m))
        .collect()
}

/// Text to complete inline after what the user typed, from the top-ranked host-prefix match.
/// Completes only up to the end of the host, so a typed prefix never jumps into a deep path.
#[must_use]
pub fn inline_completion(query: &str, ranked: &[(Candidate, MatchQuality)]) -> Option<String> {
    let q = query.trim().to_lowercase();
    let (c, m) = ranked.first()?;
    if *m != MatchQuality::HostPrefix || q.is_empty() {
        return None;
    }
    let rest = strip_scheme_www(&c.url);
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..host_end].to_lowercase();
    (host.len() > q.len() && host.starts_with(&q)).then(|| host[q.len()..].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn v(days_ago: i64, kind: VisitKind) -> Visit {
        Visit {
            at: NOW - days_ago * DAY,
            kind,
        }
    }

    #[test]
    fn recency_and_frequency() {
        let yesterday_once = frecency(&[v(1, VisitKind::Link)], 1, false, NOW);
        let old_often = frecency(&[v(200, VisitKind::Link); 10], 50, false, NOW);
        let old_once = frecency(&[v(200, VisitKind::Link)], 1, false, NOW);
        assert!(yesterday_once > old_once);
        assert!(
            old_often > yesterday_once,
            "50 visits outweigh one recent visit"
        );
        let typed = frecency(&[v(1, VisitKind::Typed)], 1, false, NOW);
        assert!(typed > yesterday_once);
        assert!(frecency(&[v(1, VisitKind::Passive)], 1, false, NOW).abs() < f64::EPSILON);
        assert!(frecency(&[], 0, true, NOW) >= 140.0);
    }

    #[test]
    fn ranking_and_inline_completion() {
        let cands = vec![
            Candidate {
                url: "https://www.index.hr/vijesti".into(),
                title: "Index vijesti".into(),
                frecency: 300.0,
            },
            Candidate {
                url: "https://github.com/scyde-browser".into(),
                title: "SCYDE on GitHub".into(),
                frecency: 900.0,
            },
            Candidate {
                url: "https://indeed.com/".into(),
                title: "Jobs".into(),
                frecency: 50.0,
            },
            Candidate {
                url: "https://example.org/".into(),
                title: "Find an index".into(),
                frecency: 5000.0,
            },
        ];
        let r = rank("ind", &cands, 10);
        assert_eq!(r[0].0.url, "https://www.index.hr/vijesti");
        assert_eq!(r[0].1, MatchQuality::HostPrefix);
        assert_eq!(inline_completion("ind", &r).as_deref(), Some("ex.hr"));
        assert!(
            r.iter()
                .any(|(c, m)| c.url == "https://example.org/" && *m == MatchQuality::WordStart)
        );
        assert!(!r.iter().any(|(c, _)| c.url.contains("github")));
        let r = rank("scyde", &cands, 10);
        assert_eq!(r[0].1, MatchQuality::WordStart);
        assert_eq!(
            inline_completion("scyde", &r),
            None,
            "no inline completion into a path"
        );
        assert!(rank("", &cands, 10).is_empty());
    }
}

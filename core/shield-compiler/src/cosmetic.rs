//! Cosmetic index (PLAN §5.2): generic selectors keyed by class/id, and per-host rules, for fast
//! lookup in content scripts on every platform.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// Generic (`##sel`) rules.
#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct Generic {
    /// `.class…` selectors keyed by their first class: query only classes present on the page.
    pub classes: BTreeMap<String, BTreeSet<String>>,
    /// `#id…` selectors keyed by their first id.
    pub ids: BTreeMap<String, BTreeSet<String>>,
    /// Everything else (attribute selectors, tags): applied on every page.
    pub other: BTreeSet<String>,
    /// `#@#sel` without a host: disabled everywhere.
    pub exceptions: BTreeSet<String>,
}

/// Rules for one host (or entity such as `example.*`).
#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct HostRules {
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub hide: BTreeSet<String>,
    /// Generic selectors that must not apply on this host (`#@#`, `~host##`).
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub unhide: BTreeSet<String>,
    /// `##+js(name, args…)` bodies, i.e. `name, args…`.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub scriptlets: BTreeSet<String>,
    /// `#?#` procedural and `#$#` style rules, verbatim after the marker.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub procedural: BTreeSet<String>,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct CosmeticReport {
    pub generic: usize,
    pub host_rules: usize,
    pub hosts: usize,
    pub scriptlets: usize,
    pub dropped: BTreeMap<String, usize>,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct CosmeticIndex {
    pub generic: Generic,
    pub hosts: BTreeMap<String, HostRules>,
}

fn ident(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

const MARKERS: &[(&str, bool, Kind)] = &[
    ("#@?#", true, Kind::Procedural),
    ("#@$#", true, Kind::Procedural),
    ("#@#", true, Kind::Hide),
    ("#?#", false, Kind::Procedural),
    ("#$#", false, Kind::Procedural),
    ("##", false, Kind::Hide),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Hide,
    Procedural,
}

/// Builds the index from list text. Network lines are ignored.
#[must_use]
pub fn build<S: AsRef<str>>(lists: &[S]) -> (CosmeticIndex, CosmeticReport) {
    let mut idx = CosmeticIndex::default();
    let mut rep = CosmeticReport::default();
    for list in lists {
        for line in list.as_ref().lines().map(str::trim) {
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            let Some((pos, marker, exception, kind)) = MARKERS
                .iter()
                .find_map(|(m, e, k)| line.find(m).map(|p| (p, *m, *e, *k)))
            else {
                continue;
            };
            let hosts = &line[..pos];
            let body = line[pos + marker.len()..].trim();
            if body.is_empty() {
                *rep.dropped.entry("empty".into()).or_default() += 1;
                continue;
            }
            if hosts.is_empty() {
                if kind == Kind::Procedural || body.starts_with("+js(") {
                    // Generic procedural/scriptlet rules are too costly to run everywhere.
                    *rep.dropped.entry("generic-procedural".into()).or_default() += 1;
                } else if exception {
                    idx.generic.exceptions.insert(body.to_owned());
                    rep.generic += 1;
                } else {
                    add_generic(&mut idx.generic, body);
                    rep.generic += 1;
                }
                continue;
            }
            for host in hosts.split(',').map(str::trim).filter(|h| !h.is_empty()) {
                let (negated, host) = host.strip_prefix('~').map_or((false, host), |h| (true, h));
                let host = host.to_ascii_lowercase();
                let e = idx.hosts.entry(host).or_default();
                rep.host_rules += 1;
                if let Some(args) = body.strip_prefix("+js(").and_then(|b| b.strip_suffix(')')) {
                    if !(exception || negated) {
                        e.scriptlets.insert(args.trim().to_owned());
                        rep.scriptlets += 1;
                    }
                } else if kind == Kind::Procedural {
                    if !(exception || negated) {
                        e.procedural.insert(body.to_owned());
                    }
                } else if exception || negated {
                    e.unhide.insert(body.to_owned());
                } else {
                    e.hide.insert(body.to_owned());
                }
            }
        }
    }
    idx.hosts.retain(|_, r| *r != HostRules::default());
    rep.hosts = idx.hosts.len();
    (idx, rep)
}

fn add_generic(g: &mut Generic, sel: &str) {
    let simple = |rest: &str| !rest.contains(',');
    if let Some(rest) = sel.strip_prefix('.') {
        let k = ident(rest);
        if !k.is_empty() && simple(rest) {
            g.classes
                .entry(k.to_owned())
                .or_default()
                .insert(sel.to_owned());
            return;
        }
    } else if let Some(rest) = sel.strip_prefix('#') {
        let k = ident(rest);
        if !k.is_empty() && simple(rest) {
            g.ids
                .entry(k.to_owned())
                .or_default()
                .insert(sel.to_owned());
            return;
        }
    }
    g.other.insert(sel.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes() {
        let list = "##.ad-banner\n##.ad-banner > div\n###top-ad\n##div[id^=\"sponsor-\"]\n#@#.ok\n\
                    a.com,~b.a.com##.x\na.com#@#.ad-banner\na.com##+js(set-constant, foo, 1)\n\
                    a.com#?#div:has-text(Ad)\n#?#div:has-text(Ad)\n||net.work^\n";
        let (idx, rep) = build(&[list]);
        assert_eq!(idx.generic.classes["ad-banner"].len(), 2);
        assert!(idx.generic.ids["top-ad"].contains("#top-ad"));
        assert!(idx.generic.other.contains("div[id^=\"sponsor-\"]"));
        assert!(idx.generic.exceptions.contains(".ok"));
        let a = &idx.hosts["a.com"];
        assert!(a.hide.contains(".x"));
        assert!(a.unhide.contains(".ad-banner"));
        assert!(a.scriptlets.contains("set-constant, foo, 1"));
        assert!(a.procedural.contains("div:has-text(Ad)"));
        assert!(idx.hosts["b.a.com"].unhide.contains(".x"));
        assert_eq!(rep.dropped["generic-procedural"], 1);
        assert_eq!(rep.scriptlets, 1);
    }
}

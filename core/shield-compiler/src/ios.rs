//! Filter lists → `WKContentRuleList` JSON (iOS, ADR-0006).
//!
//! Conversion is adblock-rust's `content-blocking` module. We filter out what `WebKit` cannot
//! compile and split the result into lists of at most [`MAX_PER_LIST`] rules.

use std::collections::BTreeMap;

use adblock::lists::{FilterSet, ParseOptions};
use serde::Serialize;
use serde_json::Value;

/// `WebKit`'s limit per `WKContentRuleList`.
pub const MAX_PER_LIST: usize = 150_000;

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct IosReport {
    /// Filters the converter used.
    pub filters_used: usize,
    pub rules: usize,
    pub lists: usize,
    pub dropped: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct IosOutput {
    pub lists: Vec<Vec<Value>>,
    pub report: IosReport,
}

/// Why `WebKit` would reject (or never match) this `url-filter`, if it would.
///
/// `WebKit` accepts a small regex subset: no alternation, counted repetition, shorthand classes,
/// lookaround or backreferences; ASCII only; `$` only at the end; and a literal `\^` (an ABP
/// separator adblock-rust left untranslated for `||<IP>^`) never matches.
#[must_use]
pub fn webkit_regex_problem(f: &str) -> Option<&'static str> {
    if !f.is_ascii() {
        return Some("regex:non-ascii");
    }
    if f.contains("\\^") {
        return Some("regex:literal-caret");
    }
    if f.contains('|') {
        return Some("regex:alternation");
    }
    if f.contains('{') || f.contains('}') {
        return Some("regex:counted-repetition");
    }
    if f.contains("(?") {
        return Some("regex:group-modifier");
    }
    let b = f.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'\\'
            && let Some(&n) = b.get(i + 1)
            && matches!(
                n,
                b'd' | b'D' | b'w' | b'W' | b's' | b'S' | b'b' | b'B' | b'1'..=b'9'
            )
        {
            return Some("regex:shorthand-class");
        }
        if b[i] == b'$' && i + 1 != b.len() && (i == 0 || b[i - 1] != b'\\') {
            return Some("regex:dollar-not-at-end");
        }
    }
    None
}

/// Sorts trigger arrays: adblock-rust builds them from hash sets, and bundles must be
/// byte-for-byte reproducible (the manifest hashes them).
fn normalize(rule: &mut Value) {
    if let Some(trigger) = rule.get_mut("trigger").and_then(Value::as_object_mut) {
        for (_, v) in trigger.iter_mut() {
            if let Some(arr) = v.as_array_mut() {
                arr.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            }
        }
    }
}

/// Converts lists into `WebKit` content-blocker lists of at most `max_per_list` rules.
///
/// # Panics
/// Never in practice: the filter set is created in debug mode, which the conversion requires.
#[must_use]
pub fn convert<S: AsRef<str>>(lists: &[S], max_per_list: usize) -> IosOutput {
    let mut set = FilterSet::new(true);
    for l in lists {
        set.add_filter_list(l.as_ref().to_owned(), ParseOptions::default());
    }
    let (rules, used) = set
        .into_content_blocking()
        .expect("debug filter set converts");
    let mut report = IosReport {
        filters_used: used.len(),
        ..IosReport::default()
    };
    let mut kept = Vec::with_capacity(rules.len());
    for r in &rules {
        let mut v = serde_json::to_value(r).expect("content-blocker rule serializes");
        normalize(&mut v);
        let problem = v
            .pointer("/trigger/url-filter")
            .and_then(Value::as_str)
            .and_then(webkit_regex_problem);
        match problem {
            Some(why) => *report.dropped.entry(why.to_owned()).or_default() += 1,
            None => kept.push(v),
        }
    }
    report.rules = kept.len();
    let lists: Vec<Vec<Value>> = kept
        .chunks(max_per_list.max(1))
        .map(<[Value]>::to_vec)
        .collect();
    report.lists = lists.len();
    IosOutput { lists, report }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_subset() {
        assert_eq!(webkit_regex_problem("^https?://ads\\.example\\.com"), None);
        assert_eq!(webkit_regex_problem("ad\\.js$"), None);
        assert_eq!(webkit_regex_problem("a|b"), Some("regex:alternation"));
        assert_eq!(
            webkit_regex_problem("a{2}"),
            Some("regex:counted-repetition")
        );
        assert_eq!(webkit_regex_problem("\\d+"), Some("regex:shorthand-class"));
        assert_eq!(
            webkit_regex_problem("127\\.0\\.0\\.1\\^"),
            Some("regex:literal-caret")
        );
        assert_eq!(webkit_regex_problem("a$b"), Some("regex:dollar-not-at-end"));
        assert_eq!(webkit_regex_problem("a\\$b"), None);
        assert_eq!(webkit_regex_problem("č"), Some("regex:non-ascii"));
    }

    #[test]
    fn splits_lists() {
        let list = "||a.com^\n||b.com^\n||c.com^\n";
        let out = convert(&[list], 2);
        // adblock-rust adds an `ignore-previous-rules` rule for first-party documents
        assert!(out.report.rules >= 3);
        assert_eq!(out.lists.len(), out.report.rules.div_ceil(2));
        assert!(out.lists.iter().all(|l| l.len() <= 2));
    }
}

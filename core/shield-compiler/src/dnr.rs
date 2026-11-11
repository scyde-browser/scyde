//! Filter lists → Chromium `declarativeNetRequest` static rulesets (Windows, ADR-0003).
//!
//! Our own converter for the network subset DNR can express. Everything else is counted in
//! [`DnrReport::dropped`] by reason, so the bundle report shows exactly what Windows loses
//! (and what the host must cover: cosmetics, scriptlets, `$popup`, main-frame interstitials).

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::{Map, Value, json};

/// Chromium guarantees this many static rules per extension.
pub const GUARANTEED_RULES: usize = 30_000;
/// Chromium's limit on regex rules across enabled static rulesets.
pub const MAX_REGEX_RULES: usize = 1_000;
/// Chromium allows at most this many enabled static rulesets.
pub const MAX_ENABLED_RULESETS: usize = 50;

/// Limits for one conversion.
#[derive(Debug, Clone, Copy)]
pub struct DnrLimits {
    /// Rules per ruleset file.
    pub per_ruleset: usize,
    /// Total rules across rulesets (30k guaranteed + global pool share).
    pub max_rules: usize,
}

impl Default for DnrLimits {
    fn default() -> Self {
        Self {
            per_ruleset: GUARANTEED_RULES,
            max_rules: 330_000,
        }
    }
}

/// What happened to every line.
#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct DnrReport {
    pub filter_lines: usize,
    pub converted: usize,
    pub duplicates: usize,
    pub regex_rules: usize,
    pub over_limit: usize,
    pub rulesets: usize,
    pub dropped: BTreeMap<String, usize>,
}

impl DnrReport {
    fn drop(&mut self, reason: impl Into<String>) {
        *self.dropped.entry(reason.into()).or_default() += 1;
    }
}

/// Output: rulesets (each a JSON array of rules) plus the report.
#[derive(Debug, Clone)]
pub struct DnrOutput {
    pub rulesets: Vec<Vec<Value>>,
    pub report: DnrReport,
}

const PRIORITY_BLOCK: u32 = 1;
const PRIORITY_ALLOW: u32 = 2;
const PRIORITY_IMPORTANT: u32 = 3;

fn resource_type(name: &str) -> Option<&'static str> {
    Some(match name {
        "script" => "script",
        "image" | "img" => "image",
        "stylesheet" | "css" => "stylesheet",
        "xmlhttprequest" | "xhr" => "xmlhttprequest",
        "subdocument" | "frame" => "sub_frame",
        "document" | "doc" => "main_frame",
        "font" => "font",
        "media" => "media",
        "websocket" => "websocket",
        "ping" | "beacon" => "ping",
        "object" | "object-subrequest" => "object",
        "other" => "other",
        _ => return None,
    })
}

/// Options DNR cannot express; the rule is dropped and reported under `option:<name>`.
const UNSUPPORTED: &[&str] = &[
    "popup",
    "popunder",
    "csp",
    "removeparam",
    "redirect",
    "redirect-rule",
    "rewrite",
    "header",
    "permissions",
    "replace",
    "urlskip",
    "uritransform",
    "urltransform",
    "generichide",
    "ghide",
    "elemhide",
    "ehide",
    "specifichide",
    "shide",
    "genericblock",
    "inline-script",
    "inline-font",
    "strict1p",
    "strict3p",
    "strict-first-party",
    "strict-third-party",
    "empty",
    "mp4",
    "cname",
    "webrtc",
    "ipaddress",
    "reason",
];

/// Splits `pattern$options`. A `$` followed by a `/` without `=` belongs to the pattern
/// (`/re$/`, `/path$x/y`), so regexes and paths keep their dollars.
fn split_options(line: &str) -> (&str, &str) {
    match line.rfind('$') {
        Some(i) if !line[i + 1..].contains('/') || line[i + 1..].contains('=') => {
            (&line[..i], &line[i + 1..])
        }
        _ => (line, ""),
    }
}

fn is_cosmetic(line: &str) -> bool {
    ["##", "#@#", "#?#", "#$#", "#%#", "#@$#", "#@?#", "#@%#"]
        .iter()
        .any(|m| line.contains(m))
}

fn valid_domain(d: &str) -> bool {
    !d.is_empty()
        && d.is_ascii()
        && !d.ends_with(".*")
        && d.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
}

/// `a.com|~b.com` → (include, exclude); `None` if a domain cannot be expressed.
fn domain_list(v: &str) -> Option<(Vec<String>, Vec<String>)> {
    let (mut inc, mut exc) = (Vec::new(), Vec::new());
    for d in v.split('|') {
        let (neg, d) = d.strip_prefix('~').map_or((false, d), |d| (true, d));
        let d = d.to_ascii_lowercase();
        if !valid_domain(&d) {
            return None;
        }
        if neg { exc.push(d) } else { inc.push(d) }
    }
    Some((inc, exc))
}

/// RE2 (DNR) has no lookaround or backreferences.
fn re2_ok(re: &str) -> bool {
    re.is_ascii()
        && !re.contains("(?=")
        && !re.contains("(?!")
        && !re.contains("(?<")
        && !(1..=9).any(|n| re.contains(&format!("\\{n}")))
}

/// Chromium compiles DNR regexes with a small RE2 memory budget, and one regex over it makes
/// Chromium reject the whole extension (found with `[%0-9a-zA-Z]{200,1300}` from uBO filters).
/// Large counted repetitions and very long patterns are therefore dropped.
pub const MAX_REGEX_LEN: usize = 300;
pub const MAX_REGEX_REPEAT: u32 = 64;

fn re2_small(re: &str) -> bool {
    if re.len() > MAX_REGEX_LEN {
        return false;
    }
    let b = re.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' && (i == 0 || b[i - 1] != b'\\') {
            let end = re[i..].find('}').map_or(b.len(), |e| i + e);
            for n in re[i + 1..end].split(',') {
                if n.trim().parse::<u32>().is_ok_and(|n| n > MAX_REGEX_REPEAT) {
                    return false;
                }
            }
            i = end;
        }
        i += 1;
    }
    true
}

/// Converts one filter line. `Err(reason)` means dropped; `Ok(None)` means not a filter.
#[allow(clippy::too_many_lines)] // one flat option table reads better than helpers
fn convert_line(raw: &str, report: &mut DnrReport) -> Result<Option<Value>, String> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
        return Ok(None);
    }
    report.filter_lines += 1;
    if is_cosmetic(line) {
        return Err(if line.contains("+js(") {
            "scriptlet"
        } else {
            "cosmetic"
        }
        .into());
    }
    let (allow, line) = line.strip_prefix("@@").map_or((false, line), |l| (true, l));
    let (pattern, options) = split_options(line);

    let mut cond = Map::new();
    let mut types: Vec<&str> = Vec::new();
    let mut not_types: Vec<&str> = Vec::new();
    let mut important = false;
    let mut all_types = false;

    for opt in options.split(',').filter(|o| !o.is_empty()) {
        let (neg, name) = opt.strip_prefix('~').map_or((false, opt), |o| (true, o));
        let (key, value) = name.split_once('=').map_or((name, ""), |(k, v)| (k, v));
        match key {
            "third-party" | "3p" => {
                cond.insert(
                    "domainType".into(),
                    json!(if neg { "firstParty" } else { "thirdParty" }),
                );
            }
            "first-party" | "1p" => {
                cond.insert(
                    "domainType".into(),
                    json!(if neg { "thirdParty" } else { "firstParty" }),
                );
            }
            "domain" | "from" => {
                let (inc, exc) = domain_list(value).ok_or("domain:unsupported")?;
                if !inc.is_empty() {
                    cond.insert("initiatorDomains".into(), json!(inc));
                }
                if !exc.is_empty() {
                    cond.insert("excludedInitiatorDomains".into(), json!(exc));
                }
            }
            "to" => {
                let (inc, exc) = domain_list(value).ok_or("domain:unsupported")?;
                if !inc.is_empty() {
                    cond.insert("requestDomains".into(), json!(inc));
                }
                if !exc.is_empty() {
                    cond.insert("excludedRequestDomains".into(), json!(exc));
                }
            }
            "denyallow" => {
                let (inc, _) = domain_list(value).ok_or("domain:unsupported")?;
                cond.insert("excludedRequestDomains".into(), json!(inc));
            }
            "method" => {
                let (mut inc, mut exc) = (Vec::new(), Vec::new());
                for m in value.split('|') {
                    let (n, m) = m.strip_prefix('~').map_or((false, m), |m| (true, m));
                    if n {
                        exc.push(m.to_ascii_lowercase());
                    } else {
                        inc.push(m.to_ascii_lowercase());
                    }
                }
                if !inc.is_empty() {
                    cond.insert("requestMethods".into(), json!(inc));
                }
                if !exc.is_empty() {
                    cond.insert("excludedRequestMethods".into(), json!(exc));
                }
            }
            "match-case" => {
                cond.insert("isUrlFilterCaseSensitive".into(), json!(true));
            }
            "important" => important = true,
            "all" => all_types = true,
            "badfilter" => return Err("badfilter".into()),
            _ => {
                if let Some(t) = resource_type(key) {
                    if neg {
                        not_types.push(t);
                    } else {
                        types.push(t);
                    }
                } else if UNSUPPORTED.contains(&key) {
                    return Err(format!("option:{key}"));
                } else {
                    return Err(format!("option:unknown:{key}"));
                }
            }
        }
    }

    if pattern.len() > 2 && pattern.starts_with('/') && pattern.ends_with('/') {
        let re = &pattern[1..pattern.len() - 1];
        if !re2_ok(re) {
            return Err("regex:unsupported".into());
        }
        if !re2_small(re) {
            return Err("regex:too-complex".into());
        }
        if report.regex_rules >= MAX_REGEX_RULES {
            return Err("regex:limit".into());
        }
        report.regex_rules += 1;
        cond.insert("regexFilter".into(), json!(re));
    } else {
        let p = pattern
            .strip_prefix("||*")
            .map_or(pattern, |p| p.trim_start_matches('*'));
        if p.is_empty() || p == "*" || p == "|" || p == "||" {
            if cond.contains_key("requestDomains") || cond.contains_key("initiatorDomains") {
                // `$domain=x` / `$to=x` alone is a valid DNR rule without urlFilter
            } else {
                return Err("pattern:empty".into());
            }
        } else if !p.is_ascii() {
            return Err("pattern:non-ascii".into());
        } else {
            cond.insert("urlFilter".into(), json!(p));
        }
    }

    types.sort_unstable();
    types.dedup();
    not_types.sort_unstable();
    not_types.dedup();
    if !types.is_empty() {
        cond.insert("resourceTypes".into(), json!(types));
    }
    if !not_types.is_empty() {
        cond.insert("excludedResourceTypes".into(), json!(not_types));
    } else if types.is_empty() && !all_types {
        // Like the engine: no type option never blocks the top document. Main-frame blocking
        // is the host's interstitial (PLAN §5.4).
        cond.insert("excludedResourceTypes".into(), json!(["main_frame"]));
    }

    let priority = if allow {
        PRIORITY_ALLOW
    } else if important {
        PRIORITY_IMPORTANT
    } else {
        PRIORITY_BLOCK
    };
    let action = if allow { "allow" } else { "block" };
    Ok(Some(
        json!({ "priority": priority, "action": { "type": action }, "condition": cond }),
    ))
}

/// Converts filter lists into DNR rulesets within `limits`.
///
/// # Panics
/// Never: every rule is built as a JSON object.
#[must_use]
pub fn convert<S: AsRef<str>>(lists: &[S], limits: DnrLimits) -> DnrOutput {
    let mut report = DnrReport::default();
    // `$badfilter` cancels the identical filter without that option.
    let mut cancelled = HashSet::new();
    for list in lists {
        for line in list.as_ref().lines() {
            let l = line.trim();
            if let Some((base, opts)) = l.rsplit_once('$')
                && opts.split(',').any(|o| o == "badfilter")
            {
                let rest: Vec<&str> = opts.split(',').filter(|o| *o != "badfilter").collect();
                cancelled.insert(if rest.is_empty() {
                    base.to_owned()
                } else {
                    format!("{base}${}", rest.join(","))
                });
            }
        }
    }

    let mut seen = HashSet::new();
    let mut rules = Vec::new();
    for list in lists {
        for line in list.as_ref().lines() {
            let l = line.trim();
            if cancelled.contains(l) {
                report.filter_lines += 1;
                report.drop("badfilter");
                continue;
            }
            match convert_line(l, &mut report) {
                Ok(Some(rule)) => {
                    if seen.insert(rule.to_string()) {
                        rules.push(rule);
                    } else {
                        report.duplicates += 1;
                    }
                }
                Ok(None) => {}
                Err(reason) => report.drop(reason),
            }
        }
    }

    let max = limits
        .max_rules
        .min(limits.per_ruleset * MAX_ENABLED_RULESETS);
    report.over_limit = rules.len().saturating_sub(max);
    rules.truncate(max);
    report.converted = rules.len();

    let mut rulesets = Vec::new();
    for (i, chunk) in rules.chunks(limits.per_ruleset.max(1)).enumerate() {
        let base = i * limits.per_ruleset;
        rulesets.push(
            chunk
                .iter()
                .enumerate()
                .map(|(j, r)| {
                    let mut r = r.clone();
                    r.as_object_mut()
                        .expect("rule is an object")
                        .insert("id".into(), json!(base + j + 1));
                    r
                })
                .collect(),
        );
    }
    report.rulesets = rulesets.len();
    DnrOutput { rulesets, report }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(line: &str) -> Result<Value, String> {
        let mut r = DnrReport::default();
        convert_line(line, &mut r).map(|v| v.expect("filter"))
    }

    #[test]
    fn host_anchor() {
        let r = one("||ads.example.com^").unwrap();
        assert_eq!(r["condition"]["urlFilter"], "||ads.example.com^");
        assert_eq!(
            r["condition"]["excludedResourceTypes"],
            json!(["main_frame"])
        );
        assert_eq!(r["action"]["type"], "block");
    }

    #[test]
    fn options() {
        let r = one("@@||cdn.example^$script,~third-party,domain=a.com|~b.a.com").unwrap();
        assert_eq!(r["action"]["type"], "allow");
        assert_eq!(r["priority"], PRIORITY_ALLOW);
        assert_eq!(r["condition"]["resourceTypes"], json!(["script"]));
        assert_eq!(r["condition"]["domainType"], "firstParty");
        assert_eq!(r["condition"]["initiatorDomains"], json!(["a.com"]));
        assert_eq!(
            r["condition"]["excludedInitiatorDomains"],
            json!(["b.a.com"])
        );
        let r = one("||x.com^$important,~image,method=post|~get").unwrap();
        assert_eq!(r["priority"], PRIORITY_IMPORTANT);
        assert_eq!(r["condition"]["excludedResourceTypes"], json!(["image"]));
        assert_eq!(r["condition"]["requestMethods"], json!(["post"]));
    }

    #[test]
    fn drops_with_reasons() {
        assert_eq!(one("||x.com^$popup").unwrap_err(), "option:popup");
        assert_eq!(one("example.com##.ad").unwrap_err(), "cosmetic");
        assert_eq!(one("example.com##+js(foo)").unwrap_err(), "scriptlet");
        assert_eq!(
            one("||x.com^$domain=y.*").unwrap_err(),
            "domain:unsupported"
        );
        assert_eq!(one("/ad(?=s)/").unwrap_err(), "regex:unsupported");
        assert_eq!(
            one("/^https?:\\/\\/[a-z]{5,13}\\/[%0-9a-z]{200,1300}/").unwrap_err(),
            "regex:too-complex"
        );
        assert!(one("/banner[0-9]{2,8}\\.js/").is_ok());
        assert_eq!(
            one("||x.com^$frobnicate").unwrap_err(),
            "option:unknown:frobnicate"
        );
    }

    #[test]
    fn regex_and_dollar_in_path() {
        let r = one(r"/banner\d+\.js/$script").unwrap();
        assert_eq!(r["condition"]["regexFilter"], r"banner\d+\.js");
        let r = one("/ad-network/*$script,image").unwrap();
        assert_eq!(r["condition"]["urlFilter"], "/ad-network/*");
        assert_eq!(r["condition"]["resourceTypes"], json!(["image", "script"]));
        let r = one("/ads$/$script").unwrap();
        assert_eq!(r["condition"]["regexFilter"], "ads$");
        let r = one("/path$with/dollar").unwrap();
        assert_eq!(r["condition"]["urlFilter"], "/path$with/dollar");
    }

    #[test]
    fn badfilter_dedupe_and_split() {
        let list = "||a.com^\n||a.com^\n||b.com^\n||b.com^$badfilter\n||c.com^\n||d.com^\n";
        let out = convert(
            &[list],
            DnrLimits {
                per_ruleset: 2,
                max_rules: 3,
            },
        );
        assert_eq!(out.report.duplicates, 1);
        assert_eq!(out.report.dropped["badfilter"], 2);
        assert_eq!(out.report.converted, 3);
        assert_eq!(out.report.over_limit, 0);
        assert_eq!(out.rulesets.len(), 2);
        assert_eq!(out.rulesets[1][0]["id"], 3);
        let out = convert(
            &[list],
            DnrLimits {
                per_ruleset: 1,
                max_rules: 2,
            },
        );
        assert_eq!(out.report.over_limit, 1);
    }
}

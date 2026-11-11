//! Settings: JSON Schema as the source of truth, validation, defaults and migrations
//! (PLAN §11 Phase 1.7).
//!
//! `schema/settings.schema.json` defines every setting. This crate:
//! - builds defaults from the schema,
//! - **repairs** loaded settings: invalid or unknown values fall back to defaults and are
//!   reported as [`Issue`]s, so a broken file never stops the browser from starting,
//! - validates every change ([`Settings::set`]),
//! - migrates older files step by step ([`MIGRATIONS`]).
//!
//! The validator supports exactly the keywords the schema uses (see its `description`); a test
//! fails if the schema starts using anything else.

use std::fmt;
use std::sync::OnceLock;

use serde_json::{Map, Value, json};

/// The schema, compiled in. The settings UI and the native apps read the same file.
pub const SCHEMA_JSON: &str = include_str!("../schema/settings.schema.json");

/// Keywords the validator understands.
pub const SUPPORTED_KEYWORDS: &[&str] = &[
    "$schema",
    "$id",
    "title",
    "description",
    "type",
    "properties",
    "required",
    "additionalProperties",
    "enum",
    "const",
    "default",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "x-scyde-format",
];

/// The parsed schema.
///
/// # Panics
/// Only if the compiled-in schema is not JSON (caught by tests).
#[must_use]
pub fn schema() -> &'static Value {
    static S: OnceLock<Value> = OnceLock::new();
    S.get_or_init(|| serde_json::from_str(SCHEMA_JSON).expect("settings schema is JSON"))
}

/// Current settings format version (`properties.version.const`).
#[must_use]
pub fn current_version() -> u64 {
    schema()["properties"]["version"]["const"]
        .as_u64()
        .unwrap_or(1)
}

/// Upgrades from version `i + 1` to `i + 2`. Empty while the format is at version 1; each
/// future change appends one function and bumps `version.const` in the schema.
pub const MIGRATIONS: &[fn(&mut Map<String, Value>)] = &[];

/// A problem found while loading or changing settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Dotted path, e.g. `search.engine`.
    pub path: String,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for Issue {}

fn issue(path: &str, message: impl Into<String>) -> Issue {
    Issue {
        path: if path.is_empty() {
            "(root)".into()
        } else {
            path.to_owned()
        },
        message: message.into(),
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

/// Default value for a schema node (objects are built from their properties).
fn default_for(schema: &Value) -> Value {
    if schema["type"] == "object" {
        let mut m = Map::new();
        if let Some(props) = schema["properties"].as_object() {
            for (k, s) in props {
                m.insert(k.clone(), default_for(s));
            }
        }
        return Value::Object(m);
    }
    schema.get("default").cloned().unwrap_or(Value::Null)
}

fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#')
        .is_some_and(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// `https://…{searchTerms}…` with no whitespace; empty means "not set".
fn is_search_template(s: &str) -> bool {
    s.is_empty()
        || (s.starts_with("https://")
            && s.contains("{searchTerms}")
            && !s.chars().any(char::is_whitespace)
            && s.len() > "https://".len() + "{searchTerms}".len())
}

/// Validates a leaf (non-object) value against its schema node.
fn check_leaf(schema: &Value, v: &Value) -> Result<(), String> {
    let ty = schema["type"].as_str().unwrap_or("");
    let ok_type = match ty {
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        _ => true,
    };
    if !ok_type {
        return Err(format!("expected {ty}, got {v}"));
    }
    if let Some(c) = schema.get("const")
        && c != v
    {
        return Err(format!("must be {c}"));
    }
    if let Some(e) = schema["enum"].as_array()
        && !e.contains(v)
    {
        return Err(format!("{v} is not one of {}", Value::Array(e.clone())));
    }
    if let (Some(min), Some(n)) = (schema["minimum"].as_f64(), v.as_f64())
        && n < min
    {
        return Err(format!("must be ≥ {min}"));
    }
    if let (Some(max), Some(n)) = (schema["maximum"].as_f64(), v.as_f64())
        && n > max
    {
        return Err(format!("must be ≤ {max}"));
    }
    if let Some(s) = v.as_str() {
        let len = s.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|m| len < m) {
            return Err("too short".into());
        }
        if schema["maxLength"].as_u64().is_some_and(|m| len > m) {
            return Err("too long".into());
        }
        match schema["x-scyde-format"].as_str() {
            Some("hex-color") if !is_hex_color(s) => return Err("expected #RRGGBB".into()),
            Some("search-template") if !is_search_template(s) => {
                return Err("expected an https URL containing {searchTerms}".into());
            }
            _ => {}
        }
    }
    Ok(())
}

/// Builds a valid value from `input`: keeps what validates, defaults the rest, reports issues.
fn repair(schema: &Value, input: Option<&Value>, path: &str, issues: &mut Vec<Issue>) -> Value {
    if schema["type"] == "object" {
        let empty = Map::new();
        let obj = match input {
            Some(Value::Object(o)) => o,
            Some(other) => {
                issues.push(issue(path, format!("expected object, got {other}")));
                &empty
            }
            None => &empty,
        };
        let props = schema["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let mut out = Map::new();
        for (k, s) in &props {
            out.insert(k.clone(), repair(s, obj.get(k), &join(path, k), issues));
        }
        if schema["additionalProperties"] == false {
            for k in obj.keys().filter(|k| !props.contains_key(*k)) {
                issues.push(issue(&join(path, k), "unknown setting, dropped"));
            }
        }
        return Value::Object(out);
    }
    match input {
        None => default_for(schema),
        Some(v) => match check_leaf(schema, v) {
            Ok(()) => v.clone(),
            Err(e) => {
                issues.push(issue(path, format!("{e}; reset to default")));
                default_for(schema)
            }
        },
    }
}

fn node_at<'a>(schema: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(schema, |s, key| s["properties"].get(key))
}

/// Validated settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    value: Value,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            value: default_for(schema()),
        }
    }
}

impl Settings {
    /// Loads settings JSON. Never fails: unreadable JSON, unknown keys and invalid values fall back
    /// to defaults, and every repair is reported.
    #[must_use]
    pub fn load(json: &str) -> (Self, Vec<Issue>) {
        let mut issues = Vec::new();
        let mut input: Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                issues.push(issue("", format!("not valid JSON ({e}); using defaults")));
                return (Self::default(), issues);
            }
        };
        if let Some(obj) = input.as_object_mut() {
            let found = obj.get("version").and_then(Value::as_u64).unwrap_or(1);
            let current = current_version();
            if found > current {
                issues.push(issue(
                    "version",
                    format!("{found} is newer than {current}; unknown settings are dropped"),
                ));
                obj.insert("version".into(), json!(current));
            } else {
                for (i, m) in MIGRATIONS
                    .iter()
                    .enumerate()
                    .skip(usize::try_from(found.saturating_sub(1)).unwrap_or(0))
                {
                    m(obj);
                    obj.insert("version".into(), json!(i as u64 + 2));
                }
            }
        }
        let value = repair(schema(), Some(&input), "", &mut issues);
        (Self { value }, issues)
    }

    /// The setting at a dotted path, e.g. `appearance.theme`.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&Value> {
        path.split('.').try_fold(&self.value, |v, k| v.get(k))
    }

    /// Changes one setting after validating it against the schema.
    ///
    /// # Errors
    /// An [`Issue`] if the path is unknown, not a leaf, or the value is invalid.
    pub fn set(&mut self, path: &str, value: Value) -> Result<(), Issue> {
        let node = node_at(schema(), path).ok_or_else(|| issue(path, "unknown setting"))?;
        if node["type"] == "object" || path == "version" {
            return Err(issue(path, "not a settable value"));
        }
        check_leaf(node, &value).map_err(|e| issue(path, e))?;
        let mut cur = &mut self.value;
        let mut keys = path.split('.').peekable();
        while let Some(k) = keys.next() {
            if keys.peek().is_none() {
                cur[k] = value;
                return Ok(());
            }
            cur = &mut cur[k];
        }
        Ok(())
    }

    /// Restores one setting (or a whole section) to its default.
    ///
    /// # Errors
    /// An [`Issue`] if the path is unknown.
    pub fn reset(&mut self, path: &str) -> Result<(), Issue> {
        let node = node_at(schema(), path).ok_or_else(|| issue(path, "unknown setting"))?;
        let default = default_for(node);
        let mut cur = &mut self.value;
        let mut keys = path.split('.').peekable();
        while let Some(k) = keys.next() {
            if keys.peek().is_none() {
                cur[k] = default;
                return Ok(());
            }
            cur = &mut cur[k];
        }
        Ok(())
    }

    /// Serialized settings (pretty JSON with a version), for storage and the settings page.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.value).unwrap_or_default()
    }

    /// The search URL template for the selected engine. No referral or affiliate parameters
    /// (CLAUDE.md rule 2). Falls back to `DuckDuckGo` if a custom template is missing.
    #[must_use]
    pub fn search_template(&self) -> String {
        let engine = self
            .get("search.engine")
            .and_then(Value::as_str)
            .unwrap_or("duckduckgo");
        let custom = self
            .get("search.custom_template")
            .and_then(Value::as_str)
            .unwrap_or("");
        search_template_for(engine, custom)
    }
}

/// Template for a built-in engine (or the custom one).
#[must_use]
pub fn search_template_for(engine: &str, custom: &str) -> String {
    match engine {
        "startpage" => "https://www.startpage.com/do/search?q={searchTerms}".into(),
        "brave" => "https://search.brave.com/search?q={searchTerms}".into(),
        "ecosia" => "https://www.ecosia.org/search?q={searchTerms}".into(),
        "google" => "https://www.google.com/search?q={searchTerms}".into(),
        "bing" => "https://www.bing.com/search?q={searchTerms}".into(),
        "custom" if !custom.is_empty() && is_search_template(custom) => custom.into(),
        _ => "https://duckduckgo.com/?q={searchTerms}".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keywords(v: &Value, out: &mut Vec<String>) {
        if let Some(o) = v.as_object() {
            for (k, child) in o {
                out.push(k.clone());
                if k == "properties" {
                    for p in child.as_object().into_iter().flat_map(|m| m.values()) {
                        keywords(p, out);
                    }
                }
            }
        }
    }

    #[test]
    fn schema_uses_only_supported_keywords() {
        let mut used = Vec::new();
        keywords(schema(), &mut used);
        let unknown: Vec<_> = used
            .iter()
            .filter(|k| !SUPPORTED_KEYWORDS.contains(&k.as_str()))
            .collect();
        assert!(
            unknown.is_empty(),
            "validator does not implement {unknown:?}"
        );
    }

    #[test]
    fn every_default_is_valid() {
        let (s, issues) = Settings::load(&Settings::default().to_json());
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(s, Settings::default());
        assert_eq!(s.get("search.engine"), Some(&json!("duckduckgo")));
        assert_eq!(s.get("newtab.top_stories"), Some(&json!(false)), "opt-in");
        assert_eq!(s.get("shield.enabled"), Some(&json!(true)));
        assert_eq!(
            s.search_template(),
            "https://duckduckgo.com/?q={searchTerms}"
        );
    }

    #[test]
    fn repairs_instead_of_failing() {
        let (s, issues) = Settings::load(
            r##"{"version":1,"appearance":{"theme":"neon","accent":"sky","custom_accent":"#12"},
               "search":{"engine":"google","suggestions":"yes"},"newtab":{"speed_dial_rows":9},
               "telemetry":{"on":true}}"##,
        );
        let paths: Vec<&str> = issues.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.contains(&"appearance.theme"));
        assert!(paths.contains(&"appearance.custom_accent"));
        assert!(paths.contains(&"search.suggestions"));
        assert!(paths.contains(&"newtab.speed_dial_rows"));
        assert!(paths.contains(&"telemetry"));
        assert_eq!(s.get("appearance.theme"), Some(&json!("system")));
        assert_eq!(
            s.get("appearance.accent"),
            Some(&json!("sky")),
            "valid values are kept"
        );
        assert_eq!(s.get("search.engine"), Some(&json!("google")));
        assert!(s.get("telemetry").is_none());
        let (d, issues) = Settings::load("{not json");
        assert_eq!(d, Settings::default());
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn set_validates() {
        let mut s = Settings::default();
        s.set("appearance.theme", json!("dark")).unwrap();
        assert_eq!(s.get("appearance.theme"), Some(&json!("dark")));
        assert!(s.set("appearance.theme", json!("neon")).is_err());
        assert!(s.set("appearance.nope", json!(1)).is_err());
        assert!(s.set("appearance", json!({})).is_err());
        assert!(s.set("version", json!(2)).is_err());
        assert!(s.set("newtab.speed_dial_rows", json!(0)).is_err());
        assert!(
            s.set(
                "search.custom_template",
                json!("http://x.com/?q={searchTerms}")
            )
            .is_err(),
            "https only"
        );
        assert!(
            s.set("search.custom_template", json!("https://x.com/?q="))
                .is_err()
        );
        s.set(
            "search.custom_template",
            json!("https://kagi.com/search?q={searchTerms}"),
        )
        .unwrap();
        s.set("search.engine", json!("custom")).unwrap();
        assert_eq!(
            s.search_template(),
            "https://kagi.com/search?q={searchTerms}"
        );
        s.reset("search").unwrap();
        assert_eq!(
            s.search_template(),
            "https://duckduckgo.com/?q={searchTerms}"
        );
    }

    #[test]
    fn newer_files_are_downgraded_safely() {
        let (s, issues) =
            Settings::load(r#"{"version":99,"appearance":{"theme":"dark","glass":true}}"#);
        assert_eq!(s.get("version"), Some(&json!(1)));
        assert_eq!(s.get("appearance.theme"), Some(&json!("dark")));
        assert!(issues.iter().any(|i| i.path == "version"));
        assert!(issues.iter().any(|i| i.path == "appearance.glass"));
    }

    #[test]
    fn accent_enum_matches_design_tokens() {
        // Every preset in design/tokens/accent.json, plus "custom".
        let tokens: Value =
            serde_json::from_str(include_str!("../../../design/tokens/accent.json")).unwrap();
        let mut presets: Vec<String> = tokens["accent"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| !k.starts_with('$'))
            .cloned()
            .collect();
        presets.push("custom".into());
        presets.sort();
        let mut schema_enum: Vec<String> =
            schema()["properties"]["appearance"]["properties"]["accent"]["enum"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
        schema_enum.sort();
        assert_eq!(schema_enum, presets);
    }
}

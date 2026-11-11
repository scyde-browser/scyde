//! The design tokens (`design/tokens/*.json`, DTCG) compiled into the crate and resolved at
//! runtime, so `core/theme` checks exactly what the web, Kotlin and Swift outputs ship.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::Value;

use crate::Rgb;

const SOURCES: &[&str] = &[
    include_str!("../../../design/tokens/color.palette.json"),
    include_str!("../../../design/tokens/color.theme.json"),
    include_str!("../../../design/tokens/accent.json"),
];

/// Flat `a.b.c` → resolved string value for every colour token.
#[derive(Debug)]
pub struct Tokens {
    values: BTreeMap<String, String>,
}

fn flatten(prefix: &str, v: &Value, out: &mut BTreeMap<String, String>) {
    let Some(obj) = v.as_object() else { return };
    if let Some(val) = obj.get("$value") {
        if let Some(s) = val.as_str() {
            out.insert(prefix.to_owned(), s.to_owned());
        }
        return;
    }
    for (k, child) in obj {
        if k.starts_with('$') {
            continue;
        }
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        flatten(&key, child, out);
    }
}

impl Tokens {
    /// The tokens built into this binary.
    ///
    /// # Panics
    /// Only if the token sources in the repository are malformed (caught by tests).
    #[must_use]
    pub fn get() -> &'static Self {
        static T: OnceLock<Tokens> = OnceLock::new();
        T.get_or_init(|| {
            let mut raw = BTreeMap::new();
            for src in SOURCES {
                let v: Value = serde_json::from_str(src).expect("token JSON parses");
                flatten("", &v, &mut raw);
            }
            let mut values = BTreeMap::new();
            for key in raw.keys() {
                values.insert(
                    key.clone(),
                    resolve(&raw, key, 0).expect("token reference resolves"),
                );
            }
            Tokens { values }
        })
    }

    /// The resolved value of a token, e.g. `theme.light.surface.canvas` → `#F3F1EC`.
    #[must_use]
    pub fn value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// A colour token as RGB (alpha ignored, like `tools/contrast-check`).
    ///
    /// # Panics
    /// If the token does not exist or is not a colour (programming error).
    #[must_use]
    pub fn color(&self, key: &str) -> Rgb {
        let v = self
            .value(key)
            .unwrap_or_else(|| panic!("unknown token {key}"));
        let hex = v.strip_prefix('#').unwrap_or(v);
        Rgb::from_hex(hex.get(..6).unwrap_or(hex))
            .unwrap_or_else(|_| panic!("{key} is not a colour: {v}"))
    }

    /// Accent presets, as in `tools/contrast-check` (`accent.<name>.light.fill` exists).
    #[must_use]
    pub fn accents(&self) -> Vec<&str> {
        self.values
            .keys()
            .filter_map(|k| k.strip_prefix("accent.")?.strip_suffix(".light.fill"))
            .filter(|n| !n.contains('.'))
            .collect()
    }
}

fn resolve(raw: &BTreeMap<String, String>, key: &str, depth: u8) -> Option<String> {
    if depth > 8 {
        return None;
    }
    let v = raw.get(key)?;
    match v.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        Some(reference) => resolve(raw, reference, depth + 1),
        None => Some(v.clone()),
    }
}

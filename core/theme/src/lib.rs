//! Token resolution, custom accents and WCAG contrast checks (DESIGN.md §2).
//!
//! - [`tokens`]: `design/tokens/*.json` compiled in and resolved
//! - [`contrast`]: the same pairs and thresholds as `tools/contrast-check`
//! - [`accent`]: one picked colour → every accent role, nearest passing shade in OKLCH

use std::fmt;

pub mod accent;
pub mod aura;
pub mod contrast;
pub mod tokens;

/// An sRGB colour with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Error returned when a hex string is not `#RRGGBB` or `#RGB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseHexError(String);

impl fmt::Display for ParseHexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid hex colour: {:?}", self.0)
    }
}

impl std::error::Error for ParseHexError {}

impl Rgb {
    /// Parses `#RRGGBB`, `RRGGBB`, `#RGB` or `RGB`.
    ///
    /// # Errors
    /// Returns [`ParseHexError`] if the string is not a valid hex colour.
    pub fn from_hex(input: &str) -> Result<Self, ParseHexError> {
        let err = || ParseHexError(input.to_owned());
        let hex = input.strip_prefix('#').unwrap_or(input);
        let expanded: String = match hex.len() {
            3 => hex.chars().flat_map(|c| [c, c]).collect(),
            6 => hex.to_owned(),
            _ => return Err(err()),
        };
        let channel = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).map_err(|_| err());
        Ok(Self {
            r: channel(0)?,
            g: channel(2)?,
            b: channel(4)?,
        })
    }

    /// WCAG 2.x relative luminance.
    #[must_use]
    pub fn relative_luminance(self) -> f64 {
        fn lin(c: u8) -> f64 {
            let c = f64::from(c) / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }
}

/// WCAG 2.x contrast ratio between two colours (1.0 ..= 21.0).
#[must_use]
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.relative_luminance(), b.relative_luminance());
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Minimum ratio for normal-size text (WCAG AA).
pub const AA_NORMAL: f64 = 4.5;

#[cfg(test)]
mod tests {
    use super::*;

    fn cr(a: &str, b: &str) -> f64 {
        contrast_ratio(Rgb::from_hex(a).unwrap(), Rgb::from_hex(b).unwrap())
    }

    #[test]
    fn extremes() {
        assert!((cr("#000", "#fff") - 21.0).abs() < 1e-9);
        assert!((cr("#777777", "#777777") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn design_md_claims_hold() {
        // Sky ink on light sunken surface (DESIGN.md §2.2).
        assert!(cr("#236F93", "#E9EBEF") >= AA_NORMAL);
        // Ember CTA fill with white text (DESIGN.md §2.3).
        assert!(cr("#FFFFFF", "#C93A33") >= AA_NORMAL);
        // ember.600 is large-text only.
        assert!(cr("#FFFFFF", "#D9433A") < AA_NORMAL);
    }

    #[test]
    fn gate_mirrors_contrast_check() {
        // Same pair count and result as `tools/contrast-check` (204 pairs, 0 failing).
        let checks = contrast::all_checks(tokens::Tokens::get());
        assert_eq!(checks.len(), 204);
        let failing: Vec<_> = checks
            .iter()
            .filter(|c| !c.passes())
            .map(|c| &c.label)
            .collect();
        assert!(failing.is_empty(), "{failing:?}");
    }

    /// With `THEME_CONTRAST_JSON` pointing at `node tools/contrast-check/index.mjs --json`
    /// output (CI does this), every pair must match label, colours and ratio.
    #[test]
    fn gate_matches_node_output() {
        let Ok(path) = std::env::var("THEME_CONTRAST_JSON") else {
            return;
        };
        let node: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let ours = contrast::all_checks(tokens::Tokens::get());
        let theirs = node["checks"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len());
        for n in theirs {
            let label = n["label"].as_str().unwrap();
            let o = ours
                .iter()
                .find(|c| c.label == label)
                .unwrap_or_else(|| panic!("missing {label}"));
            assert_eq!(
                o.fg,
                Rgb::from_hex(&n["fg"].as_str().unwrap()[..7]).unwrap(),
                "{label}"
            );
            assert_eq!(
                o.bg,
                Rgb::from_hex(&n["bg"].as_str().unwrap()[..7]).unwrap(),
                "{label}"
            );
            assert!(
                (o.ratio - n["ratio"].as_f64().unwrap()).abs() < 1e-9,
                "{label}"
            );
        }
    }

    #[test]
    fn oklch_round_trip() {
        for hex in [
            "#FF9F7E", "#236F93", "#121214", "#FFFFFF", "#3DDC97", "#853BE5",
        ] {
            let c = Rgb::from_hex(hex).unwrap();
            assert_eq!(accent::Oklch::from_rgb(c).to_rgb(), c, "{hex}");
        }
    }

    #[test]
    fn passing_seed_is_kept() {
        // Ember's pastel already passes as a fill.
        let seed = Rgb::from_hex("#FF9F7E").unwrap();
        let a = accent::derive(seed);
        assert_eq!(a.light.fill, seed);
        assert!(!a.fill_adjusted);
        assert!(accent::is_valid(&a));
    }

    #[test]
    fn failing_seed_moves_to_nearest_shade() {
        // A saturated mid blue fails as a fill (dark text on it) and as light ink? → adjusted.
        let seed = Rgb::from_hex("#2F6BFF").unwrap();
        let a = accent::derive(seed);
        assert!(a.fill_adjusted);
        assert!(accent::is_valid(&a));
        let (s, f) = (
            accent::Oklch::from_rgb(seed),
            accent::Oklch::from_rgb(a.light.fill),
        );
        assert!((s.h - f.h).abs() < 3.0, "hue kept: {} vs {}", s.h, f.h);
        assert!(f.l > s.l, "the fill got lighter");
    }

    #[test]
    fn every_hue_derives_a_valid_accent() {
        for h in (0..360).step_by(10) {
            for l in [0.25, 0.5, 0.7, 0.9] {
                for c in [0.0, 0.08, 0.2, 0.35] {
                    #[allow(clippy::cast_lossless)]
                    let seed = accent::Oklch {
                        l,
                        c,
                        h: f64::from(h),
                    }
                    .to_rgb();
                    let a = accent::derive(seed);
                    assert!(accent::is_valid(&a), "seed {seed:?}");
                }
            }
        }
    }

    #[test]
    fn explicit_roles_below_aa_are_rejected() {
        let t = tokens::Tokens::get();
        let good_l = contrast::preset(t, "ember", "light");
        let good_d = contrast::preset(t, "ember", "dark");
        assert!(accent::validate(&good_l, &good_d).is_ok());
        let bad = contrast::AccentMode {
            ink: Rgb::from_hex("#FFB27A").unwrap(),
            ..good_l
        };
        let err = accent::validate(&bad, &good_d).unwrap_err();
        assert!(err.iter().any(|c| c.label.contains("ink on surface")));
    }

    #[test]
    fn rejects_garbage() {
        assert!(Rgb::from_hex("#12345").is_err());
        assert!(Rgb::from_hex("zzzzzz").is_err());
    }
}

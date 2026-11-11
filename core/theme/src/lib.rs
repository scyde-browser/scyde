//! Token resolution, accent scale generation and WCAG contrast checks.
//!
//! Phase 0 ships the contrast primitives that `tools/contrast-check` mirrors,
//! so the browser can reject custom accents below AA at runtime (DESIGN.md §2.2).

use std::fmt;

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
    fn rejects_garbage() {
        assert!(Rgb::from_hex("#12345").is_err());
        assert!(Rgb::from_hex("zzzzzz").is_err());
    }
}

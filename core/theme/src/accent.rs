//! Custom accents (DESIGN.md §2.3): one colour the user picks → every role in both modes,
//! adjusted in OKLCH to the nearest lightness that passes the contrast gate.
//!
//! Roles follow the presets: light mode uses a dark `ink` (and `ring = ink`) on paper and the
//! pastel `fill` on the island; dark mode uses the pastel `fill` for ink, ring and island.
//! `on-fill` is always `neutral.light.900`.

#![allow(clippy::many_single_char_names)] // colour-space maths uses the standard names

use crate::contrast::{AccentMode, Check, accent_checks};
use crate::tokens::Tokens;
use crate::{Rgb, contrast_ratio};

/// OKLCH: perceptual lightness `l` (0..1), chroma `c`, hue `h` in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

fn to_linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

impl Oklch {
    /// sRGB → OKLCH (Ottosson's `OKLab`).
    #[must_use]
    pub fn from_rgb(c: Rgb) -> Self {
        let (r, g, b) = (to_linear(c.r), to_linear(c.g), to_linear(c.b));
        let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
        let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
        let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
        let ll = 0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s;
        let a = 1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s;
        let bb = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s;
        Self {
            l: ll,
            c: a.hypot(bb),
            h: bb.atan2(a).to_degrees().rem_euclid(360.0),
        }
    }

    /// Linear sRGB channels; may fall outside 0..1 for out-of-gamut colours.
    fn to_linear_rgb(self) -> [f64; 3] {
        let (a, b) = (
            self.c * self.h.to_radians().cos(),
            self.c * self.h.to_radians().sin(),
        );
        let l = (self.l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
        let m = (self.l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
        let s = (self.l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
        [
            4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
            -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
            -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
        ]
    }

    fn in_gamut(self) -> bool {
        self.to_linear_rgb()
            .iter()
            .all(|c| (-1e-6..=1.0 + 1e-6).contains(c))
    }

    /// The closest sRGB colour: chroma is reduced (binary search) until it fits the gamut.
    #[must_use]
    pub fn to_rgb(self) -> Rgb {
        let l = self.l.clamp(0.0, 1.0);
        let mut p = Self { l, ..self };
        if !p.in_gamut() {
            let (mut lo, mut hi) = (0.0, self.c);
            for _ in 0..24 {
                let mid = f64::midpoint(lo, hi);
                if (Self { c: mid, ..p }).in_gamut() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            p.c = lo;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ch = |v: f64| (from_linear(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
        let [r, g, b] = p.to_linear_rgb();
        Rgb {
            r: ch(r),
            g: ch(g),
            b: ch(b),
        }
    }
}

/// The nearest colour (by OKLCH lightness, same hue and chroma where the gamut allows) that
/// satisfies `ok`. Searches outward from the seed in 0.005 steps; `None` if nothing in 0..1 does.
pub fn nearest_shade(seed: Rgb, ok: impl Fn(Rgb) -> bool) -> Option<Rgb> {
    if ok(seed) {
        return Some(seed);
    }
    let o = Oklch::from_rgb(seed);
    for k in 1..=200 {
        let d = f64::from(k) * 0.005;
        for l in [o.l - d, o.l + d] {
            if (0.0..=1.0).contains(&l) {
                let c = Oklch { l, ..o }.to_rgb();
                if ok(c) {
                    return Some(c);
                }
            }
        }
    }
    None
}

/// A derived custom accent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomAccent {
    pub seed: Rgb,
    pub light: AccentMode,
    pub dark: AccentMode,
    /// `true` if the fill had to move away from the seed to pass.
    pub fill_adjusted: bool,
}

fn passes_all(c: &[Check]) -> bool {
    c.iter().all(Check::passes)
}

/// Derives every role from one seed colour. Always succeeds: the nearest passing shade is used
/// when the seed itself would fail (the UI shows the adjusted swatch).
///
/// # Panics
/// Never for real tokens: very light and very dark shades always pass (tested over all hues).
#[must_use]
pub fn derive(seed: Rgb) -> CustomAccent {
    let t = Tokens::get();
    let on_fill = t.color("neutral.light.900");
    let surfaces = |mode: &str| -> Vec<Rgb> {
        crate::contrast::SURFACES
            .iter()
            .map(|s| t.color(&format!("theme.{mode}.surface.{s}")))
            .collect()
    };
    let (light_s, dark_s) = (surfaces("light"), surfaces("dark"));
    let islands: Vec<Rgb> = ["light", "dark"]
        .iter()
        .flat_map(|m| ["bg", "raised"].map(|b| t.color(&format!("theme.{m}.island.{b}"))))
        .collect();

    // The fill is the pastel: on-fill text on it, dark-mode ink on dark paper, and the island colour.
    let fill = nearest_shade(seed, |c| {
        contrast_ratio(on_fill, c) >= crate::AA_NORMAL
            && dark_s
                .iter()
                .all(|s| contrast_ratio(c, *s) >= crate::AA_NORMAL)
            && islands
                .iter()
                .all(|i| contrast_ratio(c, *i) >= crate::contrast::AA_LARGE)
    })
    .expect("a light enough shade always passes");
    // Light-mode ink: the darker variant that reads on paper.
    let ink = nearest_shade(seed, |c| {
        light_s
            .iter()
            .all(|s| contrast_ratio(c, *s) >= crate::AA_NORMAL)
    })
    .expect("a dark enough shade always passes");

    CustomAccent {
        seed,
        light: AccentMode {
            fill,
            ink,
            on_fill,
            ring: ink,
            island: fill,
        },
        dark: AccentMode {
            fill,
            ink: fill,
            on_fill,
            ring: fill,
            island: fill,
        },
        fill_adjusted: fill != seed,
    }
}

/// Validates explicit roles (e.g. imported from a theme file): every failing pair, or `Ok`.
///
/// # Errors
/// The checks below their threshold; the caller rejects the accent (DESIGN.md §2.3).
pub fn validate(light: &AccentMode, dark: &AccentMode) -> Result<(), Vec<Check>> {
    let t = Tokens::get();
    let mut failing: Vec<Check> = accent_checks(t, "custom", "light", light);
    failing.extend(accent_checks(t, "custom", "dark", dark));
    failing.retain(|c| !c.passes());
    if failing.is_empty() {
        Ok(())
    } else {
        Err(failing)
    }
}

/// Convenience: does this derived accent pass every check?
#[must_use]
pub fn is_valid(a: &CustomAccent) -> bool {
    let t = Tokens::get();
    passes_all(&accent_checks(t, "custom", "light", &a.light))
        && passes_all(&accent_checks(t, "custom", "dark", &a.dark))
}

//! The contrast gate, mirrored from `tools/contrast-check/index.mjs` pair for pair, so the
//! runtime (custom accents) and CI (presets) apply the same rules (DESIGN.md §2).

use crate::tokens::Tokens;
use crate::{AA_NORMAL, Rgb, contrast_ratio};

/// WCAG 1.4.11: non-text UI (focus ring) and large text.
pub const AA_LARGE: f64 = 3.0;

/// One checked pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub label: String,
    pub fg: Rgb,
    pub bg: Rgb,
    pub ratio: f64,
    pub min: f64,
}

impl Check {
    #[must_use]
    pub fn passes(&self) -> bool {
        self.ratio >= self.min
    }
}

pub const MODES: [&str; 2] = ["light", "dark"];
pub const SURFACES: [&str; 4] = ["canvas", "chrome", "sunken", "raised"];

/// The roles of one accent in one mode (DESIGN.md §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentMode {
    pub fill: Rgb,
    pub ink: Rgb,
    pub on_fill: Rgb,
    pub ring: Rgb,
    pub island: Rgb,
}

/// Checks for one accent in one mode: ink ≥ 4.5 and ring ≥ 3 on every surface, on-fill on fill
/// ≥ 4.5, island colour ≥ 3 on both island surfaces.
#[must_use]
pub fn accent_checks(t: &Tokens, name: &str, mode: &str, a: &AccentMode) -> Vec<Check> {
    let mut out = Vec::new();
    for s in SURFACES {
        let bg = t.color(&format!("theme.{mode}.surface.{s}"));
        push(
            &mut out,
            format!("{mode}: accent.{name}.ink on surface.{s}"),
            a.ink,
            bg,
            AA_NORMAL,
        );
        push(
            &mut out,
            format!("{mode}: accent.{name}.ring on surface.{s}"),
            a.ring,
            bg,
            AA_LARGE,
        );
    }
    push(
        &mut out,
        format!("{mode}: on-fill on accent.{name}.fill"),
        a.on_fill,
        a.fill,
        AA_NORMAL,
    );
    for b in ["bg", "raised"] {
        let isl = t.color(&format!("theme.{mode}.island.{b}"));
        push(
            &mut out,
            format!("{mode}: accent.{name}.island on island.{b}"),
            a.island,
            isl,
            AA_LARGE,
        );
    }
    out
}

/// A preset accent from the tokens.
#[must_use]
pub fn preset(t: &Tokens, name: &str, mode: &str) -> AccentMode {
    let c = |role: &str| t.color(&format!("accent.{name}.{mode}.{role}"));
    AccentMode {
        fill: c("fill"),
        ink: c("ink"),
        on_fill: c("on-fill"),
        ring: c("ring"),
        island: c("island"),
    }
}

fn push(out: &mut Vec<Check>, label: String, fg: Rgb, bg: Rgb, min: f64) {
    out.push(Check {
        label,
        fg,
        bg,
        ratio: contrast_ratio(fg, bg),
        min,
    });
}

/// Every pair `tools/contrast-check` checks, with the same labels and thresholds.
#[must_use]
pub fn all_checks(t: &Tokens) -> Vec<Check> {
    let mut out = Vec::new();
    let accents = t.accents();
    for mode in MODES {
        let tok = |k: &str| t.color(&format!("theme.{mode}.{k}"));
        for s in SURFACES {
            let bg = tok(&format!("surface.{s}"));
            for x in ["primary", "secondary", "tertiary"] {
                push(
                    &mut out,
                    format!("{mode}: text.{x} on surface.{s}"),
                    tok(&format!("text.{x}")),
                    bg,
                    AA_NORMAL,
                );
            }
            for x in ["shield", "warning", "danger", "info"] {
                push(
                    &mut out,
                    format!("{mode}: status.{x} on surface.{s}"),
                    tok(&format!("status.{x}")),
                    bg,
                    AA_NORMAL,
                );
            }
        }
        for name in &accents {
            out.extend(accent_checks(t, name, mode, &preset(t, name, mode)));
        }
        for b in ["bg", "raised"] {
            let isl = tok(&format!("island.{b}"));
            for x in ["text", "text-secondary", "shield"] {
                push(
                    &mut out,
                    format!("{mode}: island.{x} on island.{b}"),
                    tok(&format!("island.{x}")),
                    isl,
                    AA_NORMAL,
                );
            }
        }
        let cta = tok("cta.text");
        push(
            &mut out,
            format!("{mode}: cta.text on cta.fill"),
            cta,
            tok("cta.fill"),
            AA_NORMAL,
        );
        push(
            &mut out,
            format!("{mode}: cta.text on cta.fill-hover"),
            cta,
            tok("cta.fill-hover"),
            AA_NORMAL,
        );
    }
    out
}

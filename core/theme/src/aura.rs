//! Aura adaptive tint (DESIGN.md §9, PLAN Phase 6 #1): the island takes a little of the page's
//! `theme-color`, only while the island text keeps AA contrast on the result.

use crate::{AA_NORMAL, Rgb, contrast_ratio, tokens::Tokens};

/// Share of the page colour mixed into the island background (DESIGN.md §9: 8–12 %).
pub const ISLAND_TINT: f64 = 0.12;

/// `base` mixed towards `page` by `amount` (0..=1) in sRGB.
#[must_use]
pub fn mix(base: Rgb, page: Rgb, amount: f64) -> Rgb {
    let a = amount.clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ch = |b: u8, p: u8| (f64::from(b) + (f64::from(p) - f64::from(b)) * a).round() as u8;
    Rgb {
        r: ch(base.r, page.r),
        g: ch(base.g, page.g),
        b: ch(base.b, page.b),
    }
}

/// The tinted island background for `page` in the light or dark theme, or `None` when the island
/// text or its secondary text would fall below AA on it (then the island stays untinted).
#[must_use]
pub fn island_tint(page: Rgb, dark: bool) -> Option<Rgb> {
    let t = Tokens::get();
    let mode = if dark { "dark" } else { "light" };
    let bg = t.color(&format!("theme.{mode}.island.bg"));
    let text = t.color(&format!("theme.{mode}.island.text"));
    let tinted = mix(bg, page, ISLAND_TINT);
    (contrast_ratio(text, tinted) >= AA_NORMAL).then_some(tinted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes_a_little_of_the_page_colour() {
        let black = Rgb { r: 0, g: 0, b: 0 };
        let white = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        assert_eq!(
            mix(black, white, 0.5),
            Rgb {
                r: 128,
                g: 128,
                b: 128
            }
        );
        assert_eq!(mix(black, white, 0.0), black);
        assert_eq!(mix(black, white, 2.0), white);
    }

    #[test]
    fn tints_with_dark_and_bright_pages_and_keeps_aa() {
        for dark in [false, true] {
            for hex in ["#1DA1F2", "#FF0000", "#FFFFFF", "#000000", "#FFD400"] {
                let page = Rgb::from_hex(hex).unwrap();
                if let Some(c) = island_tint(page, dark) {
                    let text = Tokens::get().color(&format!(
                        "theme.{}.island.text",
                        if dark { "dark" } else { "light" }
                    ));
                    assert!(contrast_ratio(text, c) >= AA_NORMAL, "{hex} dark={dark}");
                }
            }
        }
        // The island is dark, so a saturated brand colour must still produce a tint.
        assert!(island_tint(Rgb::from_hex("#1DA1F2").unwrap(), true).is_some());
    }
}

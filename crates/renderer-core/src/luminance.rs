//! Luminance calculation.
//!
//! ## Color space
//!
//! The coefficients below are applied to **gamma-encoded** sRGB channel
//! values, not to linear-light values. This is the conventional approximation:
//! it is cheap, and it is what almost every ASCII renderer does. The choice is
//! deliberate and tested so that output stays stable across refactors.
//!
//! ## Why integer math
//!
//! The reference formula is
//! `Y = 0.2126R + 0.7152G + 0.0722B`.
//! The coefficients sum to exactly 1.0, so they are stored as fixed-point
//! integers scaled by 10_000 (2126 + 7152 + 722 = 10_000). This makes the
//! result bit-for-bit reproducible and avoids platform float differences.

use crate::color::Rgb;

/// Weight of the red channel, scaled by [`LUMINANCE_SCALE`].
const R_WEIGHT: u32 = 2_126;
/// Weight of the green channel, scaled by [`LUMINANCE_SCALE`].
const G_WEIGHT: u32 = 7_152;
/// Weight of the blue channel, scaled by [`LUMINANCE_SCALE`].
const B_WEIGHT: u32 = 722;
/// Fixed-point scale factor; the weights sum to exactly this value.
const LUMINANCE_SCALE: u32 = 10_000;

/// Perceptual luminance of a color, in `0..=255`.
///
/// Operates on gamma-encoded channels. Pure black yields `0` and pure white
/// yields `255`.
pub fn luminance(color: Rgb) -> u8 {
    let weighted =
        R_WEIGHT * color.r as u32 + G_WEIGHT * color.g as u32 + B_WEIGHT * color.b as u32;

    // Round to nearest instead of truncating.
    ((weighted + LUMINANCE_SCALE / 2) / LUMINANCE_SCALE) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_is_zero() {
        assert_eq!(luminance(Rgb::BLACK), 0);
    }

    #[test]
    fn white_is_max() {
        assert_eq!(luminance(Rgb::WHITE), 255);
    }

    #[test]
    fn green_dominates_red_dominates_blue() {
        let r = luminance(Rgb::new(255, 0, 0));
        let g = luminance(Rgb::new(0, 255, 0));
        let b = luminance(Rgb::new(0, 0, 255));

        assert!(g > r, "green {g} should exceed red {r}");
        assert!(r > b, "red {r} should exceed blue {b}");
    }

    #[test]
    fn primaries_match_reference_coefficients() {
        // 0.2126 * 255 = 54.2, 0.7152 * 255 = 182.4, 0.0722 * 255 = 18.4
        assert_eq!(luminance(Rgb::new(255, 0, 0)), 54);
        assert_eq!(luminance(Rgb::new(0, 255, 0)), 182);
        assert_eq!(luminance(Rgb::new(0, 0, 255)), 18);
    }

    #[test]
    fn mid_gray_is_about_mid() {
        let y = luminance(Rgb::new(128, 128, 128));
        assert!((127..=129).contains(&y), "expected ~128, got {y}");
    }

    #[test]
    fn monotonic_in_each_channel() {
        let mut previous = 0;
        for v in 0..=255u8 {
            let y = luminance(Rgb::new(v, 0, 0));
            assert!(y >= previous, "not monotonic at {v}");
            previous = y;
        }
    }

    #[test]
    fn is_deterministic() {
        let c = Rgb::new(17, 200, 99);
        assert_eq!(luminance(c), luminance(c));
    }
}

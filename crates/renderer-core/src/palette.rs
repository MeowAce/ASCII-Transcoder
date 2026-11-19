//! Ordered character palettes and luminance-to-character mapping.

/// Default palette, ordered from lowest to highest visual density.
pub const DEFAULT_PALETTE: &str = " .:-=+*#%@";

/// Why a palette could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteError {
    /// A palette needs at least one character to map into.
    Empty,
}

impl core::fmt::Display for PaletteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(f, "palette must contain at least one character"),
        }
    }
}

impl core::error::Error for PaletteError {}

/// An ordered set of glyphs, from least to most visually dense.
///
/// The ordering is a hard requirement: mapping assumes index 0 is the darkest
/// glyph and the last index is the brightest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    chars: Vec<char>,
}

impl Palette {
    /// Builds a palette from a string. Fails if the string has no characters.
    pub fn new(chars: &str) -> Result<Self, PaletteError> {
        let chars: Vec<char> = chars.chars().collect();
        if chars.is_empty() {
            return Err(PaletteError::Empty);
        }
        Ok(Self { chars })
    }

    /// The default ` .:-=+*#%@` palette.
    pub fn default_palette() -> Self {
        Self::new(DEFAULT_PALETTE).expect("default palette is non-empty")
    }

    /// Number of glyphs.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Always `false`; a constructed palette is never empty.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// The glyphs, least dense first.
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Maps a luminance value to a glyph.
    ///
    /// Always returns a valid glyph: the index is clamped into range, so
    /// luminance `0` maps to the first entry and `255` to the last.
    pub fn map(&self, luminance: u8) -> char {
        let last = self.chars.len() - 1;
        // Round to nearest index rather than truncating, so the midpoint
        // between two glyphs is not biased toward the darker one.
        let index = (luminance as usize * last + 127) / 255;
        self.chars[index.min(last)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_is_the_specified_one() {
        assert_eq!(
            Palette::default_palette().chars(),
            &DEFAULT_PALETTE.chars().collect::<Vec<_>>()[..]
        );
        assert_eq!(Palette::default_palette().len(), 10);
    }

    #[test]
    fn darkest_luminance_maps_to_first_glyph() {
        let p = Palette::default_palette();
        assert_eq!(p.map(0), ' ');
    }

    #[test]
    fn brightest_luminance_maps_to_last_glyph() {
        let p = Palette::default_palette();
        assert_eq!(p.map(255), '@');
    }

    #[test]
    fn every_luminance_maps_into_the_palette() {
        let p = Palette::default_palette();
        for lum in 0..=255u8 {
            let c = p.map(lum);
            assert!(p.chars().contains(&c), "glyph {c:?} not in palette");
        }
    }

    #[test]
    fn mapping_is_monotonic_non_decreasing() {
        let p = Palette::default_palette();
        let mut previous = 0usize;
        for lum in 0..=255u8 {
            let c = p.map(lum);
            let idx = p.chars().iter().position(|&x| x == c).unwrap();
            assert!(idx >= previous, "index went backwards at {lum}");
            previous = idx;
        }
    }

    #[test]
    fn mapping_is_deterministic() {
        let p = Palette::default_palette();
        assert_eq!(p.map(123), p.map(123));
    }

    #[test]
    fn single_character_palette_always_maps_to_it() {
        let p = Palette::new("#").unwrap();
        assert_eq!(p.map(0), '#');
        assert_eq!(p.map(255), '#');
    }

    #[test]
    fn empty_palette_is_rejected() {
        assert_eq!(Palette::new(""), Err(PaletteError::Empty));
    }

    #[test]
    fn custom_palette_order_is_respected() {
        let p = Palette::new("ab").unwrap();
        assert_eq!(p.map(0), 'a');
        assert_eq!(p.map(255), 'b');
    }
}

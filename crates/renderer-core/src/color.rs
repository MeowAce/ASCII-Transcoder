//! Raw color types shared by every frontend.

/// A raw 24-bit RGB color.
///
/// This is an unencoded, frontend-independent value. Terminal escape
/// sequences and palette indices are deliberately *not* represented here;
/// mapping to ANSI True Color or xterm-256 belongs to `terminal-renderer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_preserved_verbatim() {
        let c = Rgb::new(1, 2, 3);
        assert_eq!((c.r, c.g, c.b), (1, 2, 3));
    }

    #[test]
    fn endpoints_are_correct() {
        assert_eq!(Rgb::BLACK, Rgb::new(0, 0, 0));
        assert_eq!(Rgb::WHITE, Rgb::new(255, 255, 255));
    }
}

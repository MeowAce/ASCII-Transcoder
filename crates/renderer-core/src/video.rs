//! Decoder-facing contracts shared between `video-decoder` and the pipeline.
//!
//! These types live in `renderer-core` so that `video-decoder` can depend on
//! the core for *shared frame/metadata contracts only* (see
//! `docs/ARCHITECTURE.md` section 4) without the core ever knowing how frames
//! are produced.

/// An exact rational number, used for time bases and frame rates.
///
/// Deliberately not `f64`: timing conversions must be exact so that
/// timestamps are reproducible and testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub const fn new(num: i64, den: i64) -> Self {
        Self { num, den }
    }
}

/// Raw pixel layouts the decoder may emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// Packed 8-bit RGB, 3 bytes per pixel.
    Rgb24,
}

impl PixelFormat {
    /// Bytes required for one pixel.
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb24 => 3,
        }
    }

    /// Bytes required for a full frame, or `None` on overflow.
    pub fn frame_len(self, width: u32, height: u32) -> Option<usize> {
        (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(self.bytes_per_pixel())
    }
}

/// Source media metadata, captured once at decoder startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    /// Source time base. Required for correct PTS conversion.
    pub time_base: Rational,
    /// Nominal frame rate, for display only.
    ///
    /// **Never use this for scheduling.** Sources may be variable frame rate;
    /// presentation timing must come from each frame's PTS.
    pub nominal_frame_rate: Option<Rational>,
    pub duration_us: Option<i64>,
    pub pixel_format: PixelFormat,
}

/// A decoded frame, still in source pixel order.
///
/// `pts` and `time_base` are retained alongside `timestamp_us` so that timing
/// mistakes remain auditable rather than being hidden behind one derived value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    /// Presentation timestamp in source time-base units.
    pub pts: i64,
    /// Presentation timestamp converted to microseconds.
    pub timestamp_us: i64,
    pub pixel_format: PixelFormat,
    /// Raw pixel bytes; length must equal `pixel_format.frame_len(w, h)`.
    pub data: Vec<u8>,
}

impl DecodedFrame {
    /// Expected byte length of `data`, or `None` on overflow.
    pub fn expected_len(&self) -> Option<usize> {
        self.pixel_format.frame_len(self.width, self.height)
    }

    /// Whether `data` is exactly one frame of the declared layout.
    pub fn has_valid_len(&self) -> bool {
        self.expected_len() == Some(self.data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_len_matches_layout() {
        assert_eq!(PixelFormat::Rgb24.frame_len(4, 3), Some(36));
    }

    #[test]
    fn frame_len_detects_overflow() {
        assert_eq!(PixelFormat::Rgb24.frame_len(u32::MAX, u32::MAX), None);
    }

    #[test]
    fn truncated_payload_is_invalid() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            pts: 0,
            timestamp_us: 0,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0; 11], // one byte short of 12
        };
        assert!(!frame.has_valid_len());
    }

    #[test]
    fn exact_payload_is_valid() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            pts: 0,
            timestamp_us: 0,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0; 12],
        };
        assert!(frame.has_valid_len());
    }
}

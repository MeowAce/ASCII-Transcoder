//! Decoded RGB images and the output-grid geometry used for sampling.

use crate::color::Rgb;
use crate::video::PixelFormat;

/// A decoded frame already converted to packed [`Rgb`] pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbImage {
    pub width: u32,
    pub height: u32,
    /// Row-major pixels; `len() == width * height`.
    pub pixels: Vec<Rgb>,
}

/// Why an image could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    /// The byte slice length does not match `width * height * bytes_per_pixel`.
    LengthMismatch { expected: usize, actual: usize },
    /// `width * height` overflowed.
    DimensionsTooLarge { width: u32, height: u32 },
    /// The pixel format has no conversion to RGB yet.
    UnsupportedFormat(PixelFormat),
}

impl core::fmt::Display for ImageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthMismatch { expected, actual } => {
                write!(f, "expected {expected} bytes, got {actual}")
            }
            Self::DimensionsTooLarge { width, height } => {
                write!(f, "dimensions {width}x{height} are too large")
            }
            Self::UnsupportedFormat(fmt) => write!(f, "unsupported pixel format {fmt:?}"),
        }
    }
}

impl core::error::Error for ImageError {}

impl RgbImage {
    /// Wraps already-packed RGB pixels.
    pub fn new(width: u32, height: u32, pixels: Vec<Rgb>) -> Result<Self, ImageError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or(ImageError::DimensionsTooLarge { width, height })?;
        if pixels.len() != expected {
            return Err(ImageError::LengthMismatch {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Converts raw decoder output into packed RGB pixels.
    ///
    /// This is the single conversion point from source pixel format to the
    /// core's RGB representation; frontends must never do this themselves.
    pub fn from_bytes(
        width: u32,
        height: u32,
        format: PixelFormat,
        bytes: &[u8],
    ) -> Result<Self, ImageError> {
        let expected = format
            .frame_len(width, height)
            .ok_or(ImageError::DimensionsTooLarge { width, height })?;
        if bytes.len() != expected {
            return Err(ImageError::LengthMismatch {
                expected,
                actual: bytes.len(),
            });
        }

        let pixels = match format {
            PixelFormat::Rgb24 => bytes
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| Rgb::new(c[0], c[1], c[2]))
                .collect(),
        };

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Pixel at `(x, y)`, or `None` when out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Option<Rgb> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels
            .get(y as usize * self.width as usize + x as usize)
            .copied()
    }
}

/// The output grid, measured in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSpec {
    pub width: u32,
    pub height: u32,
}

/// Why a grid could not be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridError {
    /// Source or target dimensions were zero.
    ZeroDimension,
    /// A cell aspect ratio had a zero denominator or numerator.
    InvalidCellAspect,
    /// The computed height rounded to zero; the grid must be at least 1 cell.
    GridTooSmall,
    /// Intermediate arithmetic overflowed.
    Overflow,
}

impl core::fmt::Display for GridError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroDimension => write!(f, "source and target dimensions must be non-zero"),
            Self::InvalidCellAspect => write!(f, "cell aspect ratio must be non-zero"),
            Self::GridTooSmall => write!(f, "grid height rounded to zero cells"),
            Self::Overflow => write!(f, "grid dimensions overflowed"),
        }
    }
}

impl core::error::Error for GridError {}

/// Default terminal cell aspect ratio: a cell is about twice as tall as wide.
pub const DEFAULT_CELL_ASPECT_NUM: u32 = 1;
/// Denominator of [`DEFAULT_CELL_ASPECT_NUM`].
pub const DEFAULT_CELL_ASPECT_DEN: u32 = 2;

impl GridSpec {
    /// Computes a grid that preserves the source aspect ratio in character cells.
    ///
    /// `cell_aspect_num / cell_aspect_den` is the width/height ratio of one
    /// terminal cell. Because cells are taller than they are wide, a naive
    /// width-derived height would stretch the image vertically; this applies
    /// the correction.
    pub fn for_image(
        image_width: u32,
        image_height: u32,
        target_width: u32,
        cell_aspect_num: u32,
        cell_aspect_den: u32,
    ) -> Result<Self, GridError> {
        if image_width == 0 || image_height == 0 || target_width == 0 {
            return Err(GridError::ZeroDimension);
        }
        if cell_aspect_num == 0 || cell_aspect_den == 0 {
            return Err(GridError::InvalidCellAspect);
        }

        // height = target_width * (image_height / image_width) * (cell_w / cell_h)
        let numerator = target_width as u128 * image_height as u128 * cell_aspect_num as u128;
        let denominator = image_width as u128 * cell_aspect_den as u128;

        let height = (numerator + denominator / 2) / denominator;
        if height == 0 {
            return Err(GridError::GridTooSmall);
        }
        if height > u32::MAX as u128 {
            return Err(GridError::Overflow);
        }

        Ok(Self {
            width: target_width,
            height: height as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_rejects_wrong_length() {
        let err = RgbImage::from_bytes(2, 2, PixelFormat::Rgb24, &[0; 11]).unwrap_err();
        assert_eq!(
            err,
            ImageError::LengthMismatch {
                expected: 12,
                actual: 11
            }
        );
    }

    #[test]
    fn from_bytes_preserves_channel_order() {
        let bytes = [1, 2, 3, 4, 5, 6];
        let img = RgbImage::from_bytes(2, 1, PixelFormat::Rgb24, &bytes).unwrap();
        assert_eq!(img.pixel(0, 0), Some(Rgb::new(1, 2, 3)));
        assert_eq!(img.pixel(1, 0), Some(Rgb::new(4, 5, 6)));
    }

    #[test]
    fn pixel_access_is_bounds_checked() {
        let img = RgbImage::new(2, 2, vec![Rgb::BLACK; 4]).unwrap();
        assert!(img.pixel(2, 0).is_none());
        assert!(img.pixel(0, 2).is_none());
    }

    #[test]
    fn square_image_halves_height_for_tall_cells() {
        // 100x100 source, 40 columns, cells twice as tall as wide:
        // height = 40 * 1 * (1/2) = 20
        let grid = GridSpec::for_image(100, 100, 40, 1, 2).unwrap();
        assert_eq!(
            grid,
            GridSpec {
                width: 40,
                height: 20
            }
        );
    }

    #[test]
    fn wide_image_keeps_wide_ratio() {
        // 1920x1080 -> 160 columns, 2:1 cells: 160*1080/1920/2 = 45
        let grid = GridSpec::for_image(1920, 1080, 160, 1, 2).unwrap();
        assert_eq!(
            grid,
            GridSpec {
                width: 160,
                height: 45
            }
        );
    }

    #[test]
    fn square_cells_do_not_apply_correction() {
        let grid = GridSpec::for_image(100, 100, 40, 1, 1).unwrap();
        assert_eq!(
            grid,
            GridSpec {
                width: 40,
                height: 40
            }
        );
    }

    #[test]
    fn zero_dimensions_are_rejected() {
        assert_eq!(
            GridSpec::for_image(0, 100, 40, 1, 2),
            Err(GridError::ZeroDimension)
        );
        assert_eq!(
            GridSpec::for_image(100, 0, 40, 1, 2),
            Err(GridError::ZeroDimension)
        );
        assert_eq!(
            GridSpec::for_image(100, 100, 0, 1, 2),
            Err(GridError::ZeroDimension)
        );
    }

    #[test]
    fn invalid_cell_aspect_is_rejected() {
        assert_eq!(
            GridSpec::for_image(100, 100, 40, 0, 2),
            Err(GridError::InvalidCellAspect)
        );
        assert_eq!(
            GridSpec::for_image(100, 100, 40, 1, 0),
            Err(GridError::InvalidCellAspect)
        );
    }

    #[test]
    fn extreme_aspect_does_not_overflow() {
        let grid = GridSpec::for_image(u32::MAX, u32::MAX, u32::MAX, 1, 2).unwrap();
        assert_eq!(grid.width, u32::MAX);
        assert!(grid.height > 0);
    }

    #[test]
    fn very_short_source_yields_grid_too_small() {
        // 1000x1 source into 10 columns with 1:2 cells rounds to 0.
        assert_eq!(
            GridSpec::for_image(1000, 1, 10, 1, 2),
            Err(GridError::GridTooSmall)
        );
    }
}

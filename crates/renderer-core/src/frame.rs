//! The shared, frontend-independent presentation model.
//!
//! Both the terminal and desktop frontends consume [`RenderFrame`] and must
//! never recompute characters from pixels themselves.

use crate::color::Rgb;

/// One character cell of a rendered frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCell {
    /// The glyph to display. Must come from the configured palette.
    pub character: char,
    pub foreground: Rgb,
    pub background: Option<Rgb>,
}

/// A fully processed frame, ready for presentation.
///
/// Invariants (enforced by [`RenderFrame::new`]):
/// - `cells.len() == width * height`
/// - cells are stored in row-major order
/// - `timestamp_us` is in microseconds, converted from the source stream
///   time base; the unit is part of the contract and must not change silently
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFrame {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<RenderCell>,
    pub timestamp_us: i64,
}

/// Why a [`RenderFrame`] could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// `width * height` overflowed `usize`, or exceeded `u32::MAX`.
    DimensionsTooLarge { width: u32, height: u32 },
    /// `cells.len()` did not equal `width * height`.
    CellCountMismatch { expected: usize, actual: usize },
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DimensionsTooLarge { width, height } => {
                write!(f, "frame dimensions {width}x{height} are too large")
            }
            Self::CellCountMismatch { expected, actual } => {
                write!(f, "expected {expected} cells, got {actual}")
            }
        }
    }
}

impl core::error::Error for FrameError {}

impl RenderFrame {
    /// Builds a frame, enforcing the cell-count invariant.
    pub fn new(
        width: u32,
        height: u32,
        cells: Vec<RenderCell>,
        timestamp_us: i64,
    ) -> Result<Self, FrameError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or(FrameError::DimensionsTooLarge { width, height })?;

        if cells.len() != expected {
            return Err(FrameError::CellCountMismatch {
                expected,
                actual: cells.len(),
            });
        }

        Ok(Self {
            width,
            height,
            cells,
            timestamp_us,
        })
    }

    /// Returns the cell at `(x, y)` in row-major order.
    pub fn cell(&self, x: u32, y: u32) -> Option<&RenderCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.cells
            .get(y as usize * self.width as usize + x as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell() -> RenderCell {
        RenderCell {
            character: ' ',
            foreground: Rgb::BLACK,
            background: None,
        }
    }

    #[test]
    fn accepts_matching_cell_count() {
        let frame = RenderFrame::new(3, 2, vec![cell(); 6], 0).unwrap();
        assert_eq!(frame.cells.len(), 6);
    }

    #[test]
    fn rejects_mismatched_cell_count() {
        let err = RenderFrame::new(3, 2, vec![cell(); 5], 0).unwrap_err();
        assert_eq!(
            err,
            FrameError::CellCountMismatch {
                expected: 6,
                actual: 5
            }
        );
    }

    #[test]
    fn indexing_is_row_major_and_bounds_checked() {
        let mut cells = vec![cell(); 6];
        cells[4].character = 'X'; // (x=1, y=1) in a 3-wide frame
        let frame = RenderFrame::new(3, 2, cells, 0).unwrap();

        assert_eq!(frame.cell(1, 1).unwrap().character, 'X');
        assert!(frame.cell(3, 0).is_none());
        assert!(frame.cell(0, 2).is_none());
    }
}

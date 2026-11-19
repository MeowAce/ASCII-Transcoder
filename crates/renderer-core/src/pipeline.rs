//! The core conversion pipeline: RGB image -> [`RenderFrame`].
//!
//! This is the *only* place luminance, palette mapping, and sampling live.
//! Frontends consume the result and must not reimplement any of it.

use crate::color::Rgb;
use crate::frame::{RenderCell, RenderFrame};
use crate::image::{GridSpec, RgbImage};
use crate::luminance::luminance;
use crate::palette::Palette;

/// How source pixels are reduced to one sample per output cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamplingMode {
    /// Nearest-neighbour: cheap, but aliases badly on detailed footage.
    Point,
    /// Average all source pixels covered by a cell. Slower, much more stable
    /// for video and photographs. This is the default.
    #[default]
    Box,
}

/// Tunables for the conversion pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingConfig {
    pub palette: Palette,
    pub sampling: SamplingMode,
    /// Emit a background color per cell in addition to the foreground.
    ///
    /// Off by default: a background costs an extra escape sequence per cell
    /// and gains nothing when the glyph already carries the color.
    pub background: bool,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            palette: Palette::default_palette(),
            sampling: SamplingMode::default(),
            background: false,
        }
    }
}

/// Converts an image into a presentation-ready frame.
///
/// Pure and deterministic: the same image, grid, and config always produce the
/// same cells.
pub fn render_frame(
    image: &RgbImage,
    grid: GridSpec,
    config: &ProcessingConfig,
    timestamp_us: i64,
) -> RenderFrame {
    let mut cells = Vec::with_capacity(grid.width as usize * grid.height as usize);

    for gy in 0..grid.height {
        for gx in 0..grid.width {
            let sample = match config.sampling {
                SamplingMode::Point => sample_point(image, grid, gx, gy),
                SamplingMode::Box => sample_box(image, grid, gx, gy),
            };

            let character = config.palette.map(luminance(sample));
            cells.push(RenderCell {
                character,
                foreground: sample,
                background: if config.background {
                    Some(Rgb::BLACK)
                } else {
                    None
                },
            });
        }
    }

    // Dimensions and cell count are consistent by construction.
    RenderFrame::new(grid.width, grid.height, cells, timestamp_us)
        .expect("cells were built to match grid dimensions")
}

/// Maps an output cell index range onto a source pixel range.
///
/// Returns `(start, end)` in source pixels, end-exclusive, guaranteed
/// non-empty and in bounds.
fn cell_span(index: u32, cells: u32, pixels: u32) -> (u32, u32) {
    let start = (index as u64 * pixels as u64 / cells as u64) as u32;
    let mut end = ((index as u64 + 1) * pixels as u64 / cells as u64) as u32;

    // Very large downscales can collapse a span to zero pixels; take one so
    // every cell samples something.
    if end <= start {
        end = (start + 1).min(pixels);
    }
    (start.min(pixels.saturating_sub(1)), end)
}

fn sample_point(image: &RgbImage, grid: GridSpec, gx: u32, gy: u32) -> Rgb {
    let (x0, _) = cell_span(gx, grid.width, image.width);
    let (y0, _) = cell_span(gy, grid.height, image.height);
    image.pixel(x0, y0).unwrap_or(Rgb::BLACK)
}

fn sample_box(image: &RgbImage, grid: GridSpec, gx: u32, gy: u32) -> Rgb {
    let (x0, x1) = cell_span(gx, grid.width, image.width);
    let (y0, y1) = cell_span(gy, grid.height, image.height);

    let mut r: u64 = 0;
    let mut g: u64 = 0;
    let mut b: u64 = 0;
    let mut count: u64 = 0;

    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(p) = image.pixel(x, y) {
                r += p.r as u64;
                g += p.g as u64;
                b += p.b as u64;
                count += 1;
            }
        }
    }

    if count == 0 {
        return Rgb::BLACK;
    }

    Rgb::new((r / count) as u8, (g / count) as u8, (b / count) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, color: Rgb) -> RgbImage {
        RgbImage::new(width, height, vec![color; (width * height) as usize]).unwrap()
    }

    fn render(image: &RgbImage, width: u32, height: u32) -> RenderFrame {
        render_frame(
            image,
            GridSpec { width, height },
            &ProcessingConfig::default(),
            0,
        )
    }

    #[test]
    fn black_image_maps_to_darkest_glyph() {
        let frame = render(&solid(4, 4, Rgb::BLACK), 2, 2);
        assert!(frame.cells.iter().all(|c| c.character == ' '));
    }

    #[test]
    fn white_image_maps_to_brightest_glyph() {
        let frame = render(&solid(4, 4, Rgb::WHITE), 2, 2);
        assert!(frame.cells.iter().all(|c| c.character == '@'));
    }

    #[test]
    fn rgb_is_preserved_in_cells() {
        let color = Rgb::new(10, 20, 30);
        let frame = render(&solid(2, 2, color), 1, 1);
        assert_eq!(frame.cells[0].foreground, color);
    }

    #[test]
    fn cell_count_matches_dimensions() {
        let frame = render(&solid(9, 7, Rgb::WHITE), 3, 2);
        assert_eq!(frame.cells.len(), 6);
        assert_eq!((frame.width, frame.height), (3, 2));
    }

    #[test]
    fn box_sampling_averages_a_split_image() {
        // Left half black, right half white; a single cell must average them.
        let mut pixels = Vec::new();
        for _y in 0..2 {
            pixels.push(Rgb::BLACK);
            pixels.push(Rgb::WHITE);
        }
        let image = RgbImage::new(2, 2, pixels).unwrap();

        let frame = render_frame(
            &image,
            GridSpec {
                width: 1,
                height: 1,
            },
            &ProcessingConfig::default(),
            0,
        );
        let avg = frame.cells[0].foreground;
        assert!((127..=128).contains(&avg.r), "expected ~127, got {}", avg.r);
    }

    #[test]
    fn point_sampling_picks_a_source_pixel() {
        let image = RgbImage::new(2, 1, vec![Rgb::BLACK, Rgb::WHITE]).unwrap();
        let frame = render_frame(
            &image,
            GridSpec {
                width: 1,
                height: 1,
            },
            &ProcessingConfig {
                sampling: SamplingMode::Point,
                ..Default::default()
            },
            0,
        );
        // Nearest-neighbour at x=0 yields the black pixel.
        assert_eq!(frame.cells[0].foreground, Rgb::BLACK);
    }

    #[test]
    fn upscaling_does_not_panic() {
        let frame = render(&solid(1, 1, Rgb::WHITE), 20, 10);
        assert_eq!(frame.cells.len(), 200);
    }

    #[test]
    fn extreme_downscale_does_not_panic() {
        let frame = render(&solid(2000, 2000, Rgb::new(9, 9, 9)), 1, 1);
        assert_eq!(frame.cells[0].foreground, Rgb::new(9, 9, 9));
    }

    #[test]
    fn background_is_opt_in() {
        let image = solid(2, 2, Rgb::WHITE);
        let grid = GridSpec {
            width: 1,
            height: 1,
        };
        let without = render_frame(&image, grid, &ProcessingConfig::default(), 0);
        assert_eq!(without.cells[0].background, None);

        let with = render_frame(
            &image,
            grid,
            &ProcessingConfig {
                background: true,
                ..Default::default()
            },
            0,
        );
        assert_eq!(with.cells[0].background, Some(Rgb::BLACK));
    }

    #[test]
    fn timestamp_is_carried_through() {
        let frame = render_frame(
            &solid(2, 2, Rgb::WHITE),
            GridSpec {
                width: 1,
                height: 1,
            },
            &ProcessingConfig::default(),
            123_456,
        );
        assert_eq!(frame.timestamp_us, 123_456);
    }

    #[test]
    fn output_is_deterministic() {
        let image = solid(8, 8, Rgb::new(33, 66, 99));
        let a = render(&image, 4, 2);
        let b = render(&image, 4, 2);
        assert_eq!(a, b);
    }
}

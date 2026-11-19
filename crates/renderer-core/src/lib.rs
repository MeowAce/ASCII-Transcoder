//! Frontend-independent rendering core.
//!
//! This crate is the lowest-level project crate. It must not depend on
//! terminal or GUI toolkits, and it must not contain ANSI escape sequences.
//! See `docs/ARCHITECTURE.md` sections 3-6.
//!
//! The conversion pipeline lives here exactly once: decode produces
//! [`RgbImage`], [`pipeline::render_frame`] turns it into a [`RenderFrame`],
//! and both frontends only present the result.

pub mod color;
pub mod frame;
pub mod image;
pub mod luminance;
pub mod palette;
pub mod pipeline;
pub mod video;

pub use color::Rgb;
pub use frame::{FrameError, RenderCell, RenderFrame};
pub use image::{
    DEFAULT_CELL_ASPECT_DEN, DEFAULT_CELL_ASPECT_NUM, GridError, GridSpec, ImageError, RgbImage,
};
pub use luminance::luminance;
pub use palette::{DEFAULT_PALETTE, Palette, PaletteError};
pub use pipeline::{ProcessingConfig, SamplingMode, render_frame};
pub use video::{DecodedFrame, PixelFormat, Rational, VideoMetadata};

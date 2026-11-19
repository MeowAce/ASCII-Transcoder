//! Video input and decoder adapters.
//!
//! May depend on `renderer-core` for shared frame/metadata contracts only.
//! Must not depend on terminal or GUI crates, and must not leak subprocess
//! details across its boundary.

mod ffmpeg;

pub use ffmpeg::FfmpegDecoder;

use renderer_core::{DecodedFrame, VideoMetadata};

/// A sequential frame source.
///
/// Blocking by design: callers own this on a dedicated worker thread so that
/// decoding never stalls a presentation loop.
pub trait VideoDecoder {
    fn metadata(&self) -> &VideoMetadata;

    /// Returns the next frame, or `Ok(None)` at end of stream.
    fn next_frame(&mut self) -> Result<Option<DecodedFrame>, DecoderError>;
}

/// Failures surfaced by a decoder implementation.
#[derive(Debug)]
pub enum DecoderError {
    /// The decoder backend (e.g. the FFmpeg binary) is missing or unusable.
    BackendUnavailable(String),
    /// The media could not be opened or its format is unsupported.
    Unsupported(String),
    /// A read returned malformed or truncated data.
    Corrupt(String),
    /// The decoder was cancelled; the worker should stop.
    Cancelled,
}

impl core::fmt::Display for DecoderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BackendUnavailable(msg) => write!(f, "decoder backend unavailable: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported media: {msg}"),
            Self::Corrupt(msg) => write!(f, "corrupt frame data: {msg}"),
            Self::Cancelled => write!(f, "decoder cancelled"),
        }
    }
}

impl core::error::Error for DecoderError {}

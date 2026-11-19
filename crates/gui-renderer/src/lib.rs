//! Desktop presentation layer.
//!
//! **Stub only.** The eframe/egui shell, file picker, preview surface, and CPU
//! glyph renderer are roadmap phase P4.
//!
//! Constraints that must survive implementation (see `docs/ARCHITECTURE.md`
//! section 11):
//! - Never create one egui widget per ASCII character.
//! - Never duplicate ASCII conversion; consume [`renderer_core::RenderFrame`].
//! - Never block the UI thread with decode or processing work.
//! - `wgpu` stays out until the CPU path is validated.
//!
//! This crate must not depend on `terminal-renderer`.

#![forbid(unsafe_code)]

use renderer_core::RenderFrame;

/// Receives frames produced off the UI thread.
///
/// Placeholder for the P4 presentation surface. It exists now so the
/// `gui-renderer -> renderer-core` edge is a real, compiled dependency and the
/// shared-model contract is fixed before any GUI code is written.
#[derive(Debug, Default)]
pub struct PreviewSurface {
    latest: Option<RenderFrame>,
}

impl PreviewSurface {
    /// Stores the newest frame. Must be called with a frame received from a
    /// worker, never by decoding inline on the UI thread.
    pub fn submit(&mut self, frame: RenderFrame) {
        self.latest = Some(frame);
    }

    /// The most recent frame, if any.
    pub fn latest(&self) -> Option<&RenderFrame> {
        self.latest.as_ref()
    }
}

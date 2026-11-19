//! Desktop frontend entrypoint.
//!
//! Owns desktop application startup and wiring of GUI, decoder, core pipeline,
//! and playback services. Not implemented yet (roadmap phase P4).

use gui_renderer::PreviewSurface;
use video_decoder::VideoDecoder;

fn main() {
    // Wires the P4 crate boundaries without pretending playback works.
    let _surface = PreviewSurface::default();
    let _decoder: Option<Box<dyn VideoDecoder>> = None;
    eprintln!("rgb-desktop: not implemented yet (roadmap phase P4)");
    std::process::exit(2);
}

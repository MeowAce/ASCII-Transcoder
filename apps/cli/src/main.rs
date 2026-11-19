//! `rgb-render` — terminal frontend entrypoint.
//!
//! Owns CLI parsing, configuration assembly, and orchestration of decoder,
//! core pipeline, playback, and terminal backend.
//!
//! Phase P1 scope: render a **static image** to the terminal. Video playback,
//! interactive controls, and terminal lifecycle handling are P2.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use renderer_core::{
    DEFAULT_CELL_ASPECT_DEN, DEFAULT_CELL_ASPECT_NUM, GridSpec, Palette, ProcessingConfig, Rgb,
    RgbImage, render_frame,
};
use terminal_renderer::{ColorMode, serialize_frame};
use video_decoder::{FfmpegDecoder, VideoDecoder};

const USAGE: &str = "\
rgb-render — render a static image as colored ASCII art

USAGE:
    rgb-render <IMAGE> [OPTIONS]

OPTIONS:
    --width <COLS>        Output width in character cells [default: 80]
    --color-mode <MODE>   truecolor | 256          [default: truecolor]
    --palette <CHARS>     Ordered glyphs, dark to light [default: \" .:-=+*#%@\"]
    --no-cell-aspect      Treat terminal cells as square (no 1:2 correction)
    --sampling <MODE>     box | point              [default: box]
    --background          Emit a background color per cell
    -h, --help            Print this help

INPUT:
    A still image (png/jpeg/bmp/gif/webp/tiff), or a video
    (mp4/mkv/avi/mov/webm) of which only the FIRST frame is rendered.

NOTE: continuous video playback, pause/resume, and seeking are not
implemented yet (roadmap phase P2, remaining work).
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("rgb-render: {err}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
struct Options {
    input: PathBuf,
    width: u32,
    color_mode: ColorMode,
    cell_aspect: (u32, u32),
    config: ProcessingConfig,
}

fn parse_args() -> Result<Option<Options>, String> {
    parse_args_from(std::env::args().skip(1))
}

/// Parses arguments from any iterator, so tests can drive the CLI contract
/// without spawning a process.
fn parse_args_from<I>(args: I) -> Result<Option<Options>, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();

    let mut input: Option<PathBuf> = None;
    let mut width: u32 = 80;
    let mut color_mode = ColorMode::TrueColor;
    let mut cell_aspect = (DEFAULT_CELL_ASPECT_NUM, DEFAULT_CELL_ASPECT_DEN);
    let mut config = ProcessingConfig::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "--width" => {
                let value = args.next().ok_or("--width requires a value")?;
                width = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --width value: {value}"))?;
                if width == 0 {
                    return Err("--width must be greater than zero".into());
                }
            }
            "--color-mode" => {
                let value = args.next().ok_or("--color-mode requires a value")?;
                color_mode = match value.as_str() {
                    "truecolor" => ColorMode::TrueColor,
                    "256" => ColorMode::Ansi256,
                    other => {
                        return Err(format!(
                            "invalid --color-mode: {other} (expected truecolor or 256)"
                        ));
                    }
                };
            }
            "--palette" => {
                let value = args.next().ok_or("--palette requires a value")?;
                config.palette =
                    Palette::new(&value).map_err(|err| format!("invalid --palette: {err}"))?;
            }
            "--sampling" => {
                let value = args.next().ok_or("--sampling requires a value")?;
                config.sampling = match value.as_str() {
                    "box" => renderer_core::SamplingMode::Box,
                    "point" => renderer_core::SamplingMode::Point,
                    other => {
                        return Err(format!(
                            "invalid --sampling: {other} (expected box or point)"
                        ));
                    }
                };
            }
            "--no-cell-aspect" => cell_aspect = (1, 1),
            "--background" => config.background = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            path => {
                if input.is_some() {
                    return Err("only one input image is supported".into());
                }
                input = Some(PathBuf::from(path));
            }
        }
    }

    let input = match input {
        Some(path) => path,
        None => {
            eprint!("{USAGE}");
            return Err("no input image given".into());
        }
    };

    Ok(Some(Options {
        input,
        width,
        color_mode,
        cell_aspect,
        config,
    }))
}

fn run() -> Result<(), String> {
    let Some(options) = parse_args()? else {
        return Ok(());
    };

    let image = load_input(&options.input)?;

    let grid = GridSpec::for_image(
        image.width,
        image.height,
        options.width,
        options.cell_aspect.0,
        options.cell_aspect.1,
    )
    .map_err(|err| format!("cannot compute output grid: {err}"))?;

    let frame = render_frame(&image, grid, &options.config, 0);
    print!("{}", serialize_frame(&frame, options.color_mode));

    Ok(())
}

/// Extensions treated as video rather than still images.
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "webm"];

/// Loads either a still image or the **first frame** of a video.
///
/// Video playback is P2's remaining work; this path proves the decoder,
/// core pipeline, and terminal serializer compose correctly end to end.
fn load_input(path: &Path) -> Result<RgbImage, String> {
    let is_video = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false);

    if is_video {
        load_first_video_frame(path)
    } else {
        load_image(path)
    }
}

/// Decodes the first frame of a video file.
fn load_first_video_frame(path: &Path) -> Result<RgbImage, String> {
    let mut decoder = FfmpegDecoder::open(&path.to_string_lossy())
        .map_err(|err| format!("cannot open video {}: {err}", path.display()))?;

    let frame = decoder
        .next_frame()
        .map_err(|err| format!("cannot decode {}: {err}", path.display()))?
        .ok_or_else(|| format!("video {} contains no frames", path.display()))?;

    RgbImage::from_bytes(frame.width, frame.height, frame.pixel_format, &frame.data)
        .map_err(|err| format!("invalid decoded frame: {err}"))
}

/// Loads an image file into the core's RGB representation.
///
/// This is the only place `image` is used; the core stays dependency-free.
fn load_image(path: &Path) -> Result<RgbImage, String> {
    let decoded =
        image::open(path).map_err(|err| format!("cannot open {}: {err}", path.display()))?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();

    let pixels: Vec<Rgb> = rgba.pixels().map(|p| Rgb::new(p[0], p[1], p[2])).collect();

    RgbImage::new(width, height, pixels).map_err(|err| format!("invalid image data: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Options>, String> {
        parse_args_from(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn usage_text_is_honest_about_missing_playback() {
        // The CLI renders only the first video frame; the help must say so
        // rather than implying working playback.
        assert!(USAGE.contains("FIRST frame"));
        assert!(USAGE.contains("not\nimplemented") || USAGE.contains("not implemented"));
    }

    #[test]
    fn color_mode_values_are_stable() {
        // Guards the CLI contract documented in README/ROADMAP.
        assert_eq!(ColorMode::default(), ColorMode::TrueColor);
    }

    #[test]
    fn defaults_are_applied() {
        let options = parse(&["image.png"]).unwrap().unwrap();
        assert_eq!(options.input, PathBuf::from("image.png"));
        assert_eq!(options.width, 80);
        assert_eq!(options.color_mode, ColorMode::TrueColor);
        assert_eq!(options.cell_aspect, (1, 2));
        assert_eq!(options.config.sampling, renderer_core::SamplingMode::Box);
        assert!(!options.config.background);
    }

    #[test]
    fn help_returns_none() {
        assert!(parse(&["--help"]).unwrap().is_none());
        assert!(parse(&["-h"]).unwrap().is_none());
    }

    #[test]
    fn width_and_color_mode_are_parsed() {
        let options = parse(&["a.png", "--width", "100", "--color-mode", "256"])
            .unwrap()
            .unwrap();
        assert_eq!(options.width, 100);
        assert_eq!(options.color_mode, ColorMode::Ansi256);
    }

    #[test]
    fn palette_is_parsed() {
        let options = parse(&["a.png", "--palette", "01"]).unwrap().unwrap();
        assert_eq!(options.config.palette.chars(), &['0', '1']);
    }

    #[test]
    fn custom_palette_changes_glyphs() {
        let options = parse(&["a.png", "--palette", "01"]).unwrap().unwrap();
        assert_eq!(options.config.palette.map(0), '0');
        assert_eq!(options.config.palette.map(255), '1');
    }

    #[test]
    fn no_cell_aspect_disables_correction() {
        let options = parse(&["a.png", "--no-cell-aspect"]).unwrap().unwrap();
        assert_eq!(options.cell_aspect, (1, 1));
    }

    #[test]
    fn background_flag_is_opt_in() {
        let options = parse(&["a.png", "--background"]).unwrap().unwrap();
        assert!(options.config.background);
    }

    #[test]
    fn zero_width_is_rejected() {
        assert!(parse(&["a.png", "--width", "0"]).is_err());
    }

    #[test]
    fn non_numeric_width_is_rejected() {
        assert!(parse(&["a.png", "--width", "abc"]).is_err());
    }

    #[test]
    fn unknown_color_mode_is_rejected() {
        let err = parse(&["a.png", "--color-mode", "rainbow"]).unwrap_err();
        assert!(err.contains("truecolor"));
    }

    #[test]
    fn empty_palette_is_rejected() {
        assert!(parse(&["a.png", "--palette", ""]).is_err());
    }

    #[test]
    fn missing_value_is_rejected() {
        assert!(parse(&["a.png", "--width"]).is_err());
    }

    #[test]
    fn unknown_option_is_rejected() {
        assert!(parse(&["a.png", "--nope"]).is_err());
    }

    #[test]
    fn missing_input_is_rejected() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn two_inputs_are_rejected() {
        assert!(parse(&["a.png", "b.png"]).is_err());
    }
}

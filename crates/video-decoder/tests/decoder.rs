//! Integration tests for the FFmpeg subprocess decoder.
//!
//! These require `ffmpeg` and `ffprobe` on `PATH`. Fixtures are generated into
//! a temp directory at test time rather than committed, because video files
//! are large and binary. The generated clips are deterministic.
//!
//! The VFR test is the important one: it guards against FFmpeg silently
//! forcing constant frame rate, which would drop frames and desynchronise
//! PTS/frame pairing without any error.

use std::path::{Path, PathBuf};
use std::process::Command;

use renderer_core::PixelFormat;
use video_decoder::{FfmpegDecoder, VideoDecoder};

/// Generates a clip and returns its path. Panics if ffmpeg is unavailable.
fn generate(dir: &Path, name: &str, args: &[&str]) -> PathBuf {
    let out = dir.join(name);

    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y"])
        .args(args)
        .arg(&out)
        .status()
        .expect("ffmpeg must be installed to run decoder integration tests");

    assert!(status.success(), "ffmpeg failed generating {name}");
    out
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rgb-ascii-decoder-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn decode_all(path: &Path) -> (Vec<i64>, u32, u32) {
    let mut decoder = FfmpegDecoder::open(path.to_str().unwrap()).expect("decoder should open");

    let (width, height) = (decoder.metadata().width, decoder.metadata().height);
    assert_eq!(decoder.metadata().pixel_format, PixelFormat::Rgb24);

    let mut timestamps = Vec::new();
    while let Some(frame) = decoder.next_frame().expect("decode should succeed") {
        assert!(frame.has_valid_len(), "frame payload must match layout");
        assert_eq!((frame.width, frame.height), (width, height));
        timestamps.push(frame.timestamp_us);
    }

    (timestamps, width, height)
}

#[test]
fn decodes_a_cfr_clip_with_evenly_spaced_timestamps() {
    let dir = temp_dir("cfr");
    let clip = generate(
        &dir,
        "cfr.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=160x120:rate=10:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let (timestamps, width, height) = decode_all(&clip);

    assert_eq!((width, height), (160, 120));
    assert_eq!(timestamps.len(), 10, "expected 10 frames");
    assert_eq!(timestamps[0], 0);

    // 10 fps -> 100 ms between frames.
    for pair in timestamps.windows(2) {
        assert_eq!(pair[1] - pair[0], 100_000, "CFR spacing must be uniform");
    }
}

#[test]
fn preserves_variable_frame_rate_without_dropping_frames() {
    let dir = temp_dir("vfr");

    // Two segments at different rates, concatenated without re-encoding.
    let a = generate(
        &dir,
        "a.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=160x120:rate=10:duration=0.5",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let b = generate(
        &dir,
        "b.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=160x120:rate=30:duration=0.5",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let list = dir.join("list.txt");
    std::fs::write(
        &list,
        format!("file '{}'\nfile '{}'\n", a.display(), b.display()),
    )
    .unwrap();

    let vfr = generate(
        &dir,
        "vfr.mp4",
        &[
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            list.to_str().unwrap(),
            "-c",
            "copy",
        ],
    );

    let (timestamps, _, _) = decode_all(&vfr);

    // The decisive assertion: FFmpeg's default rawvideo output would drop
    // frames here and yield 17, breaking PTS/frame pairing.
    assert_eq!(
        timestamps.len(),
        20,
        "frames were dropped; -fps_mode passthrough is missing or broken"
    );

    let deltas: Vec<i64> = timestamps.windows(2).map(|p| p[1] - p[0]).collect();
    let has_uniform = deltas.iter().all(|d| *d == deltas[0]);
    assert!(
        !has_uniform,
        "timestamps are uniform; VFR was not preserved: {deltas:?}"
    );

    // The 10 fps segment ends at 0.4 s and the 30 fps segment starts at 0.75 s.
    assert!(
        deltas.iter().any(|d| *d > 300_000),
        "expected a large gap at the segment boundary: {deltas:?}"
    );
}

#[test]
fn reports_unsupported_input_clearly() {
    let dir = temp_dir("bad");
    let bad = dir.join("not-a-video.mp4");
    std::fs::write(&bad, b"this is definitely not a video").unwrap();

    let result = FfmpegDecoder::open(bad.to_str().unwrap());
    assert!(result.is_err(), "corrupt input must not open successfully");
}

#[test]
fn reports_missing_file_clearly() {
    let result = FfmpegDecoder::open("/nonexistent/video.mp4");
    assert!(result.is_err());
}

#[test]
fn decoder_is_usable_after_reaching_eof() {
    let dir = temp_dir("eof");
    let clip = generate(
        &dir,
        "short.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=32x32:rate=5:duration=0.4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let mut decoder = FfmpegDecoder::open(clip.to_str().unwrap()).unwrap();

    let mut count = 0;
    while decoder.next_frame().unwrap().is_some() {
        count += 1;
    }
    assert_eq!(count, 2);

    // Calling past EOF must keep returning None, not panic or error.
    assert!(decoder.next_frame().unwrap().is_none());
    assert!(decoder.next_frame().unwrap().is_none());
}

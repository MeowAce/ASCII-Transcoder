//! Integration tests for the `rgb-render` binary.
//!
//! These exercise the real CLI end to end: argument parsing, image loading,
//! grid computation, and ANSI serialization. Fixtures are committed under
//! `tests/fixtures/` so the suite does not depend on generated files.

use std::path::PathBuf;
use std::process::Command;

/// Path to a committed fixture.
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

/// Runs the binary and returns `(exit_code, stdout, stderr)`.
fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_rgb-render"))
        .args(args)
        .output()
        .expect("failed to execute rgb-render");

    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Strips ANSI SGR sequences, leaving only glyphs and newlines.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until the terminating 'm' of the SGR sequence.
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }

    out
}

#[test]
fn renders_a_black_image_as_spaces() {
    let (code, stdout, _) = run(&[
        fixture("black.png").to_str().unwrap(),
        "--width",
        "8",
        "--no-cell-aspect",
    ]);

    assert_eq!(code, 0);
    let plain = strip_ansi(&stdout);
    let glyphs: Vec<char> = plain.chars().filter(|c| *c != '\n').collect();

    // Guard against a vacuous pass: there must actually be glyph cells.
    assert_eq!(glyphs.len(), 64, "expected 8x8 cells, got {plain:?}");
    assert!(
        glyphs.iter().all(|c| *c == ' '),
        "expected only spaces, got: {plain:?}"
    );
}

#[test]
fn renders_a_white_image_as_the_brightest_glyph() {
    let (code, stdout, _) = run(&[
        fixture("white.png").to_str().unwrap(),
        "--width",
        "8",
        "--no-cell-aspect",
    ]);

    assert_eq!(code, 0);
    let plain = strip_ansi(&stdout);
    let glyphs: Vec<char> = plain.chars().filter(|c| *c != '\n').collect();

    assert_eq!(glyphs.len(), 64, "expected 8x8 cells, got {plain:?}");
    assert!(
        glyphs.iter().all(|c| *c == '@'),
        "expected only '@', got: {plain:?}"
    );
}

#[test]
fn output_geometry_matches_the_requested_width() {
    let (code, stdout, _) = run(&[
        fixture("gradient.png").to_str().unwrap(),
        "--width",
        "20",
        "--no-cell-aspect",
    ]);

    assert_eq!(code, 0);
    let plain = strip_ansi(&stdout);
    let lines: Vec<&str> = plain.lines().collect();

    // 64x32 source, square cells, 20 columns -> 10 rows.
    assert_eq!(lines.len(), 10, "unexpected row count");
    for line in &lines {
        assert_eq!(line.chars().count(), 20, "unexpected column count");
    }
}

#[test]
fn cell_aspect_correction_halves_the_row_count() {
    let path = fixture("gradient.png");
    let path = path.to_str().unwrap();

    let (_, corrected, _) = run(&[path, "--width", "20"]);
    let (_, uncorrected, _) = run(&[path, "--width", "20", "--no-cell-aspect"]);

    let corrected_rows = strip_ansi(&corrected).lines().count();
    let uncorrected_rows = strip_ansi(&uncorrected).lines().count();

    assert_eq!(uncorrected_rows, 10);
    assert_eq!(corrected_rows, 5, "1:2 cells should halve the height");
}

#[test]
fn truecolor_mode_emits_24bit_sequences() {
    let (code, stdout, _) = run(&[
        fixture("gradient.png").to_str().unwrap(),
        "--width",
        "10",
        "--color-mode",
        "truecolor",
    ]);

    assert_eq!(code, 0);
    assert!(stdout.contains("\x1b[38;2;"), "expected 24-bit SGR");
}

#[test]
fn ansi256_mode_emits_index_sequences() {
    let (code, stdout, _) = run(&[
        fixture("gradient.png").to_str().unwrap(),
        "--width",
        "10",
        "--color-mode",
        "256",
    ]);

    assert_eq!(code, 0);
    assert!(stdout.contains("\x1b[38;5;"), "expected 256-color SGR");
    assert!(!stdout.contains("38;2;"), "must not emit 24-bit SGR");
}

#[test]
fn custom_palette_is_honoured() {
    let (code, stdout, _) = run(&[
        fixture("gradient.png").to_str().unwrap(),
        "--width",
        "10",
        "--palette",
        "01",
        "--no-cell-aspect",
    ]);

    assert_eq!(code, 0);
    let plain = strip_ansi(&stdout);
    let glyphs: String = plain.chars().filter(|c| !c.is_whitespace()).collect();

    assert!(
        glyphs.chars().all(|c| c == '0' || c == '1'),
        "expected only palette glyphs, got: {glyphs:?}"
    );
}

#[test]
fn output_ends_with_a_reset() {
    let (code, stdout, _) = run(&[fixture("gradient.png").to_str().unwrap(), "--width", "10"]);
    assert_eq!(code, 0);
    assert!(stdout.ends_with("\x1b[0m"), "terminal state must be reset");
}

#[test]
fn missing_file_exits_nonzero_with_actionable_message() {
    let (code, _, stderr) = run(&["/nonexistent/image.png"]);

    assert_eq!(code, 1);
    assert!(stderr.contains("cannot open"), "stderr was: {stderr}");
}

#[test]
fn unsupported_input_exits_nonzero() {
    let (code, _, stderr) = run(&["Cargo.toml"]);
    assert_eq!(code, 1);
    assert!(!stderr.is_empty());
}

#[test]
fn no_arguments_exits_nonzero() {
    let (code, _, stderr) = run(&[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("no input image"), "stderr was: {stderr}");
}

#[test]
fn help_exits_zero_and_documents_options() {
    let (code, stdout, _) = run(&["--help"]);

    assert_eq!(code, 0);
    for option in ["--width", "--color-mode", "--palette", "--sampling"] {
        assert!(stdout.contains(option), "help is missing {option}");
    }
}

/// Generates a short video clip into a temp dir.
fn generate_video(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rgb-ascii-cli-video");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let out = dir.join(name);

    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("testsrc=size=160x120:rate=10:duration=1")
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p"])
        .arg(&out)
        .status()
        .expect("ffmpeg must be installed for video integration tests");

    assert!(status.success(), "ffmpeg failed to generate {name}");
    out
}

#[test]
fn renders_the_first_frame_of_a_video() {
    let clip = generate_video("first-frame.mp4");

    let (code, stdout, stderr) = run(&[clip.to_str().unwrap(), "--width", "20"]);
    assert_eq!(code, 0, "stderr was: {stderr}");

    let plain = strip_ansi(&stdout);
    let rows = plain.lines().count();
    assert!(rows > 0, "video frame produced no output");

    // 160x120 source at 20 columns with 1:2 cells -> 8 rows.
    assert_eq!(rows, 8, "unexpected geometry: {plain:?}");

    for line in plain.lines() {
        assert_eq!(line.chars().count(), 20);
    }
}

#[test]
fn video_and_image_paths_agree_on_geometry() {
    // Guards against the video branch silently using a different grid policy.
    let clip = generate_video("geometry.mp4");
    let (code, stdout, _) = run(&[clip.to_str().unwrap(), "--width", "16"]);
    assert_eq!(code, 0);

    let plain = strip_ansi(&stdout);
    assert_eq!(plain.lines().count(), 6, "160x120 at 16 cols -> 6 rows");
}

#[test]
fn corrupt_video_exits_nonzero_with_message() {
    let dir = std::env::temp_dir().join("rgb-ascii-cli-video");
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("corrupt.mp4");
    std::fs::write(&bad, b"not a video at all").unwrap();

    let (code, _, stderr) = run(&[bad.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(!stderr.is_empty());
}

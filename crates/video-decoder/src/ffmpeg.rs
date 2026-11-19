//! FFmpeg subprocess decoder.
//!
//! ## Why a subprocess
//!
//! The MVP decodes via the `ffmpeg` binary instead of FFI, to avoid native
//! library linking on Linux and Windows (see `docs/ARCHITECTURE.md` §7). The
//! [`VideoDecoder`] trait keeps the choice reversible.
//!
//! ## Frame boundaries
//!
//! Raw `rawvideo` output is a byte stream, not a message stream. Pipes return
//! partial reads, so every read is accumulated into an exactly-sized frame
//! buffer. Never assume one read equals one frame.
//!
//! ## Timestamps
//!
//! `rawvideo` carries no timestamps. They are obtained from a separate
//! `showinfo` filter on stderr, which logs one `pts_time:` per frame, and are
//! paired with frames in decode order.
//!
//! **This pairing is only valid if FFmpeg emits exactly one frame per logged
//! PTS.** FFmpeg's default behaviour silently converts output to constant
//! frame rate and *drops* frames, which would desynchronise the pairing. The
//! decoder therefore always passes `-fps_mode passthrough`; removing it is a
//! correctness bug, not a tuning choice.
//!
//! ## Deadlock avoidance
//!
//! `showinfo` writes to stderr continuously. An unread stderr pipe fills and
//! blocks the child, so stderr is drained on its own thread for the lifetime
//! of the process.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread::JoinHandle;

use renderer_core::{DecodedFrame, PixelFormat, Rational, VideoMetadata};

use crate::{DecoderError, VideoDecoder};

/// Parsed `ffprobe` stream information.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamInfo {
    width: u32,
    height: u32,
    time_base: Rational,
    nominal_frame_rate: Option<Rational>,
    duration_us: Option<i64>,
}

/// Decodes a video file by spawning one `ffmpeg` process.
pub struct FfmpegDecoder {
    metadata: VideoMetadata,
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
    /// PTS values in microseconds, in decode order, from `showinfo`.
    timestamps: Receiver<i64>,
    stderr_handle: Option<JoinHandle<()>>,
    frame_bytes: usize,
    finished: bool,
}

impl FfmpegDecoder {
    /// Opens `path`, verifies it with `ffprobe`, and starts the decoder.
    pub fn open(path: &str) -> Result<Self, DecoderError> {
        let info = probe(path)?;
        let frame_bytes = PixelFormat::Rgb24
            .frame_len(info.width, info.height)
            .ok_or_else(|| DecoderError::Corrupt("frame dimensions overflow".into()))?;

        let metadata = VideoMetadata {
            width: info.width,
            height: info.height,
            time_base: info.time_base,
            nominal_frame_rate: info.nominal_frame_rate,
            duration_us: info.duration_us,
            pixel_format: PixelFormat::Rgb24,
        };

        let mut child = Command::new("ffmpeg")
            .args([
                "-v",
                "info",
                "-nostdin",
                "-i",
                path,
                // Mandatory: without this FFmpeg silently forces CFR and drops
                // frames, which breaks PTS/frame pairing.
                "-fps_mode",
                "passthrough",
                "-vf",
                "showinfo",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| DecoderError::BackendUnavailable(format!("cannot run ffmpeg: {err}")))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DecoderError::BackendUnavailable("ffmpeg stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| DecoderError::BackendUnavailable("ffmpeg stderr unavailable".into()))?;

        let (tx, timestamps) = channel();
        let stderr_handle = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(pts_us) = parse_showinfo_pts(&line) {
                    // Receiver dropped means the decoder is gone; stop parsing.
                    if tx.send(pts_us).is_err() {
                        return;
                    }
                }
            }
        });

        Ok(Self {
            metadata,
            child,
            stdout: BufReader::with_capacity(frame_bytes * 2, stdout),
            timestamps,
            stderr_handle: Some(stderr_handle),
            frame_bytes,
            finished: false,
        })
    }
}

impl VideoDecoder for FfmpegDecoder {
    fn metadata(&self) -> &VideoMetadata {
        &self.metadata
    }

    fn next_frame(&mut self) -> Result<Option<DecodedFrame>, DecoderError> {
        if self.finished {
            return Ok(None);
        }

        let mut data = vec![0u8; self.frame_bytes];
        match read_exact_or_eof(&mut self.stdout, &mut data) {
            Ok(true) => {}
            Ok(false) => {
                self.finished = true;
                return Ok(None);
            }
            Err(err) => {
                self.finished = true;
                return Err(DecoderError::Corrupt(format!("frame read failed: {err}")));
            }
        }

        // The PTS for this frame comes from the parallel stderr stream. If it
        // is missing, the pairing contract is broken and we must not guess.
        let timestamp_us = self.timestamps.recv().map_err(|_| {
            self.finished = true;
            DecoderError::Corrupt(
                "no timestamp for decoded frame; PTS/frame pairing is invalid".into(),
            )
        })?;

        let pts = timestamp_us;

        Ok(Some(DecodedFrame {
            width: self.metadata.width,
            height: self.metadata.height,
            pts,
            timestamp_us,
            pixel_format: self.metadata.pixel_format,
            data,
        }))
    }
}

impl Drop for FfmpegDecoder {
    fn drop(&mut self) {
        // Ensure the child never outlives the decoder.
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(handle) = self.stderr_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Reads exactly `buf.len()` bytes, returning `Ok(false)` on a clean EOF.
///
/// A short read is not an error by itself: pipes deliver partial data, so this
/// loops until the buffer is full. A partial frame at EOF is corruption.
fn read_exact_or_eof(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<bool> {
    let mut filled = 0;

    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => {
                return if filled == 0 {
                    Ok(false)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("truncated frame: got {filled} of {} bytes", buf.len()),
                    ))
                };
            }
            Ok(n) => filled += n,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }

    Ok(true)
}

/// Extracts `pts_time` from a `showinfo` line, converted to microseconds.
fn parse_showinfo_pts(line: &str) -> Option<i64> {
    let idx = line.find("pts_time:")?;
    let rest = &line[idx + "pts_time:".len()..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .unwrap_or(rest.len());
    let value: f64 = rest[..end].parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some((value * 1_000_000.0).round() as i64)
}

/// Runs `ffprobe` to obtain dimensions, time base, and duration.
fn probe(path: &str) -> Result<StreamInfo, DecoderError> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,time_base,avg_frame_rate:format=duration",
            "-of",
            "default=noprint_wrappers=1",
            path,
        ])
        .output()
        .map_err(|err| DecoderError::BackendUnavailable(format!("cannot run ffprobe: {err}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DecoderError::Unsupported(format!(
            "ffprobe failed for {path}: {}",
            stderr.trim()
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut width = None;
    let mut height = None;
    let mut time_base = None;
    let mut frame_rate = None;
    let mut duration = None;

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "width" => width = value.parse::<u32>().ok(),
            "height" => height = value.parse::<u32>().ok(),
            "time_base" => time_base = parse_rational(value),
            "avg_frame_rate" => frame_rate = parse_rational(value),
            "duration" => {
                duration = value
                    .parse::<f64>()
                    .ok()
                    .map(|secs| (secs * 1_000_000.0).round() as i64)
            }
            _ => {}
        }
    }

    let (width, height) = match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
        _ => {
            return Err(DecoderError::Unsupported(format!(
                "no usable video stream in {path}"
            )));
        }
    };

    Ok(StreamInfo {
        width,
        height,
        time_base: time_base.unwrap_or(Rational::new(1, 1_000_000)),
        nominal_frame_rate: frame_rate,
        duration_us: duration,
    })
}

/// Parses `num/den`, rejecting zero or negative denominators.
fn parse_rational(value: &str) -> Option<Rational> {
    let (num, den) = value.split_once('/')?;
    let num = num.trim().parse::<i64>().ok()?;
    let den = den.trim().parse::<i64>().ok()?;
    if den <= 0 || num <= 0 {
        return None;
    }
    Some(Rational::new(num, den))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_showinfo_pts_in_microseconds() {
        let line = "[Parsed_showinfo_0 @ 0x55] n:   3 pts:    3072 pts_time:0.3 \
                    duration:  512 duration_time:0.05 pos: 1234 fmt:rgb24";
        assert_eq!(parse_showinfo_pts(line), Some(300_000));
    }

    #[test]
    fn parses_zero_and_fractional_pts() {
        assert_eq!(parse_showinfo_pts("pts_time:0"), Some(0));
        assert_eq!(parse_showinfo_pts("pts_time:1.45"), Some(1_450_000));
    }

    #[test]
    fn ignores_lines_without_pts() {
        assert_eq!(parse_showinfo_pts("Stream mapping:"), None);
    }

    #[test]
    fn rejects_non_finite_pts() {
        assert_eq!(parse_showinfo_pts("pts_time:inf"), None);
    }

    #[test]
    fn parses_rational_values() {
        assert_eq!(parse_rational("1/1000"), Some(Rational::new(1, 1000)));
        assert_eq!(parse_rational("25/2"), Some(Rational::new(25, 2)));
    }

    #[test]
    fn rejects_invalid_rationals() {
        assert_eq!(parse_rational("0/0"), None);
        assert_eq!(parse_rational("1/0"), None);
        assert_eq!(parse_rational("garbage"), None);
        assert_eq!(parse_rational("0/1"), None);
    }

    #[test]
    fn read_exact_fills_across_partial_reads() {
        // A cursor yields all bytes at once, but the loop must still be exact.
        let mut cursor = Cursor::new(vec![7u8; 10]);
        let mut buf = [0u8; 10];
        assert!(read_exact_or_eof(&mut cursor, &mut buf).unwrap());
        assert_eq!(buf, [7u8; 10]);
    }

    #[test]
    fn read_exact_reports_clean_eof() {
        let mut cursor = Cursor::new(Vec::new());
        let mut buf = [0u8; 4];
        assert!(!read_exact_or_eof(&mut cursor, &mut buf).unwrap());
    }

    #[test]
    fn read_exact_rejects_truncated_frame() {
        let mut cursor = Cursor::new(vec![1u8; 3]);
        let mut buf = [0u8; 4];
        let err = read_exact_or_eof(&mut cursor, &mut buf).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }
}

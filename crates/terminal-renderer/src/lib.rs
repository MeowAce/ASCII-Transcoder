//! Terminal presentation layer.
//!
//! Owns every terminal-specific concern: escape sequence serialization,
//! RGB-to-xterm-256 mapping, buffered output, cursor control, and terminal
//! state restoration.
//!
//! Consumes [`renderer_core::RenderFrame`] and must never decode video or
//! recompute ASCII characters.
//!
//! Crossterm integration and buffered interactive output are not implemented
//! yet (roadmap phase P2).

use renderer_core::{RenderFrame, Rgb};

/// How colors are encoded for the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// 24-bit ANSI foreground color.
    #[default]
    TrueColor,
    /// xterm-256 palette index.
    Ansi256,
}

/// Serializes one cell's foreground as an ANSI SGR sequence.
///
/// Kept here, never in `renderer-core`: the core model must stay free of
/// escape sequences so that both frontends can share it.
pub fn foreground_sequence(color: Rgb, mode: ColorMode) -> String {
    match mode {
        ColorMode::TrueColor => format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b),
        ColorMode::Ansi256 => format!("\x1b[38;5;{}m", rgb_to_xterm256(color)),
    }
}

/// Maps an RGB color to the nearest xterm-256 palette index.
///
/// Uses the 6x6x6 color cube (indices 16..=231) with simple, documented
/// nearest-value quantization in gamma-encoded RGB space. Deterministic and
/// always within `16..=231`.
pub fn rgb_to_xterm256(color: Rgb) -> u8 {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

    fn nearest_level(channel: u8) -> u8 {
        let mut best = 0u8;
        let mut best_dist = u16::MAX;
        for (index, &level) in LEVELS.iter().enumerate() {
            let dist = (channel as i16 - level as i16).unsigned_abs();
            if dist < best_dist {
                best_dist = dist;
                best = index as u8;
            }
        }
        best
    }

    let r = nearest_level(color.r);
    let g = nearest_level(color.g);
    let b = nearest_level(color.b);

    16 + 36 * r + 6 * g + b
}

/// ANSI reset, emitted once at the end of a frame.
const RESET: &str = "\x1b[0m";

/// Serializes a whole frame into one `String`, ready for a single write.
///
/// Batching matters: flushing per cell or per glyph is the usual reason a
/// terminal renderer cannot hit 30 FPS. Redundant foreground sequences are
/// also skipped when consecutive cells share a color.
///
/// No newline is appended after the final row, so callers control cursor
/// placement.
pub fn serialize_frame(frame: &RenderFrame, mode: ColorMode) -> String {
    let cell_count = frame.cells.len();
    // Roughly 20 bytes per cell: escape sequence plus glyph.
    let mut out = String::with_capacity(cell_count * 20 + RESET.len());

    let mut current_fg: Option<Rgb> = None;

    for (index, cell) in frame.cells.iter().enumerate() {
        if current_fg != Some(cell.foreground) {
            out.push_str(&foreground_sequence(cell.foreground, mode));
            current_fg = Some(cell.foreground);
        }

        if let Some(bg) = cell.background {
            out.push_str(&background_sequence(bg, mode));
        }

        out.push(cell.character);

        if frame.width > 0 && (index + 1) % frame.width as usize == 0 {
            if index + 1 != cell_count {
                out.push('\n');
            }
            // A newline does not reset colors, but tracking it keeps the
            // next row honest if the terminal is configured to reset on wrap.
            current_fg = None;
        }
    }

    out.push_str(RESET);
    out
}

/// Serializes one cell's background as an ANSI SGR sequence.
pub fn background_sequence(color: Rgb, mode: ColorMode) -> String {
    match mode {
        ColorMode::TrueColor => format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b),
        ColorMode::Ansi256 => format!("\x1b[48;5;{}m", rgb_to_xterm256(color)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_map_to_cube_corners() {
        assert_eq!(rgb_to_xterm256(Rgb::BLACK), 16);
        assert_eq!(rgb_to_xterm256(Rgb::WHITE), 231);
    }

    #[test]
    fn mapping_is_always_in_range() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(17) {
                    let idx = rgb_to_xterm256(Rgb::new(r, g, b));
                    assert!((16..=231).contains(&idx), "out of range: {idx}");
                }
            }
        }
    }

    #[test]
    fn mapping_is_deterministic() {
        let c = Rgb::new(12, 200, 77);
        assert_eq!(rgb_to_xterm256(c), rgb_to_xterm256(c));
    }

    #[test]
    fn truecolor_sequence_uses_24bit_form() {
        assert_eq!(
            foreground_sequence(Rgb::new(1, 2, 3), ColorMode::TrueColor),
            "\x1b[38;2;1;2;3m"
        );
    }

    #[test]
    fn ansi256_sequence_uses_index_form() {
        let seq = foreground_sequence(Rgb::WHITE, ColorMode::Ansi256);
        assert_eq!(seq, "\x1b[38;5;231m");
    }

    fn frame(width: u32, height: u32, fg: Rgb) -> RenderFrame {
        use renderer_core::RenderCell;
        let cells = vec![
            RenderCell {
                character: '#',
                foreground: fg,
                background: None,
            };
            (width * height) as usize
        ];
        RenderFrame::new(width, height, cells, 0).unwrap()
    }

    #[test]
    fn serialized_frame_has_one_newline_per_row_boundary() {
        let out = serialize_frame(&frame(3, 2, Rgb::WHITE), ColorMode::TrueColor);
        // Two rows: exactly one newline between them, none at the end.
        assert_eq!(out.matches('\n').count(), 1);
        assert!(!out.trim_end_matches(RESET).ends_with('\n'));
    }

    #[test]
    fn serialized_frame_contains_every_glyph() {
        let out = serialize_frame(&frame(4, 3, Rgb::WHITE), ColorMode::TrueColor);
        assert_eq!(out.matches('#').count(), 12);
    }

    #[test]
    fn identical_colors_emit_one_sequence_per_row() {
        // One SGR per row (the row boundary resets tracking), not per cell.
        let out = serialize_frame(&frame(5, 2, Rgb::WHITE), ColorMode::TrueColor);
        assert_eq!(out.matches("\x1b[38;2;255;255;255m").count(), 2);
    }

    #[test]
    fn serialized_frame_ends_with_reset() {
        let out = serialize_frame(&frame(2, 1, Rgb::BLACK), ColorMode::TrueColor);
        assert!(out.ends_with(RESET));
    }

    #[test]
    fn ansi256_frame_uses_index_sequences() {
        let out = serialize_frame(&frame(1, 1, Rgb::WHITE), ColorMode::Ansi256);
        assert!(out.contains("\x1b[38;5;231m"));
        assert!(!out.contains("38;2;"));
    }

    #[test]
    fn empty_frame_does_not_panic() {
        let out = serialize_frame(&frame(0, 0, Rgb::BLACK), ColorMode::TrueColor);
        assert_eq!(out, RESET);
    }
}

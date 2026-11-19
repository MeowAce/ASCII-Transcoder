# Current Progress

- Active phase: P2 — Video Decoder (partial)
- Status: Decoder implemented and VFR-verified; terminal playback loop not yet built
- Last verified commit: none (repository has no commits yet)
- Last verification date: 2026-11-19

## Completed

### P0 — Foundation

- Rust stable toolchain pinned via `rust-toolchain.toml`.
- Cargo workspace `rgb-ascii-renderer` with all six crates.
- Shared contracts in `renderer-core`: `Rgb`, `RenderCell`, `RenderFrame`,
  `DecodedFrame`, `VideoMetadata`, `PixelFormat`, `Rational`.
- `README.md` with prerequisites and workspace commands.

### P1 — Static ASCII Engine

- `luminance()` — fixed-point `0.2126R + 0.7152G + 0.0722B` on gamma-encoded
  channels, rounded, bit-for-bit reproducible.
- `Palette` — ordered glyphs, default ` .:-=+*#%@`, bounded and deterministic.
- `GridSpec::for_image()` — aspect-preserving grid with terminal cell
  correction (default 1:2), overflow-safe.
- `RgbImage` — packed RGB plus the single `PixelFormat` → RGB conversion point.
- `render_frame()` — the one pipeline entrypoint; box (default) and point
  sampling; opt-in background color.
- `terminal-renderer` — `ColorMode`, True Color and xterm-256 serialization,
  whole-frame serialization with escape-sequence elision.
- `rgb-render` CLI — static images, `--palette`, `--sampling`, `--background`.
- Criterion benchmarks for luminance, palette mapping, and `render_frame`.
- Committed image fixtures (`tests/fixtures/`) plus binary-level integration
  tests that assert geometry, palette, color modes, and exit codes.

### P2 — Video decoder (subprocess half)

- `VideoDecoder` trait + typed `DecoderError` with `Display`/`Error` impls.
- `FfmpegDecoder`: `ffprobe` metadata probe, `ffmpeg` subprocess, raw
  `rgb24` pipe.
- **VFR correctness proven**: `-fps_mode passthrough` is mandatory; a VFR
  fixture decodes 20/20 frames with non-uniform PTS. Without the flag FFmpeg
  drops 3 of 20 frames silently.
- Frame reads accumulate partial pipe reads into an exactly-sized buffer;
  a truncated frame at EOF is reported as corruption, not silently accepted.
- stderr is drained on a dedicated thread to avoid the classic
  unread-pipe deadlock.
- Child process is killed and reaped in `Drop`.
- CLI can render the **first frame** of a video (mp4/mkv/avi/mov/webm).

## In progress

- P2 remaining: playback scheduler (PTS-based), bounded frame queues,
  Crossterm terminal lifecycle, pause/resume/quit, terminal resize.

## Blocked

- Nothing currently.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Build | `cargo build --workspace` | pass |
| Tests | `cargo test --workspace` | pass, **103 tests** |
| Formatting | `cargo fmt --check` | pass |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass, 0 warnings |
| Dependency direction | `cargo tree -p <crate> -e normal` | matches ARCHITECTURE §4 |
| Rayon absent | `cargo tree --workspace -e normal \| grep -c rayon` | `0` |

### Decoder verification (real media)

| Fixture | Expected | Result |
| --- | --- | --- |
| CFR 10 fps, 1 s | 10 frames, uniform 100 ms spacing | pass |
| VFR (10 fps + 30 fps concat) | 20 frames, non-uniform PTS | pass |
| VFR without `-fps_mode passthrough` | 17 frames (silent drop) | confirmed defect |
| Corrupt input | clean error, no panic | pass |
| Past EOF | repeated `None`, no panic | pass |

### Benchmark baseline

Environment: WSL, Intel Core i3-10110U target class, Rust 1.100.0, source image
1920×1080 (synthetic gradient), Criterion 0.8.2.

| Benchmark | Time |
| --- | --- |
| `luminance/256_samples` | ~391 ns |
| `palette/map_256_luminances` | ~398 ns |
| `render_frame/1080p/box/60x20` | ~3.08 ms |
| `render_frame/1080p/box/100x30` | ~3.10 ms |
| `render_frame/1080p/box/160x45` | ~3.16 ms |
| `render_frame/1080p/point/60x20` | ~16.8 µs |
| `render_frame/1080p/point/100x30` | ~41.1 µs |
| `render_frame/1080p/point/160x45` | ~100.5 µs |

Box sampling is dominated by touching all 2.07M source pixels and is
essentially independent of grid size. At 30 FPS (33 ms budget) it consumes
~10% before decoding. P3 optimization target, not a correctness bug.

## Known limitations

- **No continuous playback.** The CLI renders one frame (still image, or the
  first frame of a video) and exits. There is no scheduler, no bounded queue,
  and no terminal lifecycle management yet.
- `rgb-desktop` remains a stub.
- Timestamps come from parsing `showinfo` stderr text. This is
  version-fragile: it is isolated in `video-decoder` and guarded by tests, but
  a future FFmpeg that changes the log format would break it. A `-progress`
  side channel or a single-pass `ffprobe` timestamp list is the likely
  replacement.
- `DecodedFrame.pts` currently holds microseconds rather than raw stream
  time-base ticks; the field name is therefore misleading. Either convert
  properly or rename before scheduling work depends on it.
- Box sampling averages gamma-encoded values without dithering, slightly
  darkening fine detail.
- `RgbImage` supports only `PixelFormat::Rgb24`.
- True Color output cannot be visually verified in this WSL session
  (`COLORTERM` unset, 256-color terminal).
- Decoder integration tests require `ffmpeg`/`ffprobe` on `PATH` and generate
  clips at runtime; they are not hermetic.
- `docs/SPECIFICATION.md` §11.1 still says `SPECIFICATION.md` while
  `docs/ROADMAP.md` says `docs/SPECIFICATION.md`; unify on `docs/`.

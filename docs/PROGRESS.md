# Current Progress

- Active phase: P1 — Static ASCII Rendering Engine
- Status: Implemented; algorithm exit criteria met, visual verification outstanding
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
- `rgb-render` CLI — renders a static image to the terminal.
- Criterion benchmarks for luminance, palette mapping, and `render_frame`.

## In progress

- P1 visual sign-off in a real True Color terminal (not possible in this
  WSL session; see limitations).

## Blocked

- Nothing currently.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Build | `cargo build --workspace` | pass |
| Tests | `cargo test --workspace` | pass, **59 tests** |
| Formatting | `cargo fmt --check` | pass |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | pass, 0 warnings |
| Dependency direction | `cargo tree -p <crate> -e normal` | matches ARCHITECTURE §4 |
| Rayon absent | `cargo tree --workspace -e normal \| grep -c rayon` | `0` |
| Benchmarks | `cargo bench -p benches` | recorded below |

### Benchmark baseline

Environment: WSL, Intel Core i3-10110U target class, Rust 1.100.0, source image
1920×1080 (synthetic gradient), Criterion 0.8.2. Debug-profile release of the
bench target; treat as relative numbers, not absolute guarantees.

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

**Interpretation:** box sampling cost is dominated by touching all 2.07M source
pixels (~3 ms regardless of grid size); point sampling is ~30× cheaper. At
30 FPS the frame budget is 33 ms, so box sampling currently consumes ~10% of
it before decoding. This is a P3 optimization target, not a correctness bug.

## Known limitations

- `rgb-render` renders **static images only**; `rgb-desktop` is still a stub.
  Video playback is P2.
- True Color output cannot be visually verified in this WSL session
  (`COLORTERM` unset, 256-color terminal). The escape sequences are unit-tested,
  but no human has confirmed the rendering.
- Box sampling does no linear-light averaging and no dithering; it averages
  gamma-encoded values, which slightly darkens fine detail.
- `RgbImage` supports only `PixelFormat::Rgb24`; other formats are rejected
  rather than converted.
- Palette is configurable in the core but **not exposed on the CLI** yet.
- No integration test against a committed image fixture; verification used
  generated images in `/tmp`.
- `docs/SPECIFICATION.md` §11.1 still says `SPECIFICATION.md` while
  `docs/ROADMAP.md` says `docs/SPECIFICATION.md`; unify on `docs/`.

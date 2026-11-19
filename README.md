# rgb-ascii-renderer

Real-time video-to-ASCII renderer in Rust, with a terminal frontend and a
desktop frontend that share one frontend-independent rendering core.

**Status: P1 (static ASCII engine).** A static image can be rendered to the
terminal with True Color or 256-color output. **Video playback is not
implemented** (P2), and `rgb-desktop` is a stub.

## Quick start

```bash
cargo run -p cli -- path/to/image.png
cargo run -p cli -- image.png --width 100 --color-mode 256
```

```text
rgb-render <IMAGE> [OPTIONS]

  --width <COLS>        Output width in character cells [default: 80]
  --color-mode <MODE>   truecolor | 256              [default: truecolor]
  --no-cell-aspect      Treat terminal cells as square (no 1:2 correction)
  --sampling <MODE>     box | point                  [default: box]
  --background          Emit a background color per cell
```

Supported image formats: PNG, JPEG, BMP, GIF, WebP, TIFF.

## Prerequisites

- Rust **stable** (pinned via `rust-toolchain.toml`; edition 2024 needs ≥ 1.85).
  Components `rustfmt` and `clippy` are required.
- FFmpeg on `PATH` (needed from phase P2 onward). Verified with
  `ffmpeg -version`; `ffprobe` is required too.

Install the toolchain on Arch:

```bash
pacman -S rustup && rustup default stable
rustup component add rustfmt clippy
```

## Workspace layout

```text
crates/renderer-core      # shared model + algorithms (no frontend deps)
crates/video-decoder      # decoder adapters (FFmpeg subprocess in P2)
crates/terminal-renderer  # ANSI serialization, palette, terminal lifecycle
crates/gui-renderer       # eframe/egui shell (P4)
apps/cli                  # bin: rgb-render
apps/desktop              # bin: rgb-desktop
```

Dependency direction is enforced by `docs/ARCHITECTURE.md` §4:
`renderer-core` depends on nothing; frontends depend on it and never on each
other; `apps/*` are the only composition roots.

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

cargo test -p renderer-core <name>   # single test
cargo run -p cli -- <file>           # terminal frontend (P2)
```

## Documentation

- `docs/SPECIFICATION.md` — scope, non-goals, locked decisions, acceptance criteria
- `docs/ARCHITECTURE.md` — crate boundaries, data flow, concurrency rules
- `docs/ROADMAP.md` — phases P0–P5 and their exit gates
- `docs/PROGRESS.md` — current phase status

Read all three specs before changing anything; they are a contract, not
background reading.

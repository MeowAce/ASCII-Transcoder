# RGB ASCII Video Renderer — Specification

> Project specification for OpenCode. This document defines product scope, requirements, constraints, and acceptance criteria. Treat decisions marked **LOCKED** as authoritative unless the user explicitly approves a change.

## 1. Project Summary

**Project name:** RGB ASCII Video Renderer  
**Language:** Rust (stable toolchain)  
**Product type:** Real-time video-to-ASCII renderer with terminal and desktop frontends  
**Primary development environment:** Arch Linux / Arch Linux on WSL  
**Initial target hardware:** Intel Core i3-10110U, 16 GB RAM, Intel UHD 620  
**Primary priority:** Real-time playback  
**Color modes:** ANSI 24-bit True Color and ANSI 256-color  
**Frontends:** Terminal CLI/TUI-style playback and desktop GUI

The application decodes video, transforms frames into character cells with color information, and displays the result in either a terminal or desktop window. Both frontends must consume the same renderer-core output model.

## 2. Product Goals

1. Convert supported video files into ASCII frames in real time.
2. Preserve the source video's presentation timing as closely as practical.
3. Support RGB True Color and 256-color terminal output.
4. Provide both terminal and desktop GUI frontends.
5. Keep decoding, frame processing, playback scheduling, and presentation modular.
6. Bound memory use and prevent unbounded frame queues.
7. Provide CPU fallback; GPU acceleration is an optional later optimization.
8. Make performance measurable through repeatable benchmarks and runtime statistics.

## 3. Non-Goals for the Initial Release

The following are explicitly out of scope for the MVP:

- Audio playback or audio/video synchronization.
- Video editing, filters, or transcoding/export.
- Streaming from network URLs or live capture.
- Hardware-accelerated decoding as a mandatory feature.
- GPU rendering as a prerequisite for the first working release.
- Plugin systems or scripting APIs.
- Advanced color grading.
- Guaranteed 30 FPS at every resolution and on every terminal.

Do not implement these features unless the user changes scope.

## 4. Target Platforms

### Required targets

- Linux desktop.
- Arch Linux.
- Arch Linux on WSL for development and terminal testing.

### Planned targets

- Windows 10/11 CLI and GUI.
- Other Linux distributions.

Platform support must be verified by actual builds and tests. Do not claim compatibility solely because Rust supports a target.

WSL GUI and GPU capabilities vary by host configuration. The application must remain usable through CPU rendering when GPU support is unavailable.

## 5. Input and Media Support

The application accepts a local video file path.

Initial container targets:

- MP4
- MKV
- AVI
- MOV
- WebM

Actual playback support depends on the codecs and FFmpeg build available on the host. The application must report unsupported containers/codecs and decoder failures clearly.

The decoder must expose, where available:

- Frame dimensions.
- Frame rate metadata.
- Time base.
- Presentation timestamp (PTS).
- Duration.
- Pixel format information.

Do not assume constant frame rate. Playback scheduling must use decoded frame timestamps when available.

## 6. Functional Requirements

### FR-01: Video input

- Accept a video path from the CLI.
- Validate that the file exists and is readable.
- Report actionable errors for invalid paths and unsupported media.
- GUI must provide a file-open workflow.

### FR-02: Frame decoding

- Decode frames sequentially.
- Preserve frame presentation timestamps.
- Handle end-of-file and decoder errors.
- Avoid loading the entire video into memory.
- The MVP may use an FFmpeg subprocess that emits a documented raw pixel format.
- A later native FFmpeg integration may replace the subprocess behind the decoder interface.

### FR-03: Frame preprocessing

- Resize frames to the configured output grid.
- Preserve aspect ratio.
- Account for terminal cell aspect ratio in terminal mode.
- Convert source pixels into a stable internal RGB representation.
- Avoid unnecessary allocations in the per-frame hot path where practical.

### FR-04: ASCII conversion

- Convert pixel luminance to a character from a configurable ordered palette.
- Provide a default palette: ` .:-=+*#%@`
- Support palette configuration in a later compatible extension.
- Define and test luminance calculation.
- The initial luminance formula may use:
  `Y = 0.2126R + 0.7152G + 0.0722B`
- Keep the character mapping deterministic.

### FR-05: Color modes

Terminal output must support:

1. **True Color:** ANSI 24-bit foreground RGB.
2. **256-color:** map RGB values to the xterm 256-color palette.

The internal render model must retain RGB values and must not store ANSI escape sequences. Color conversion belongs in the terminal backend.

Grayscale output may be offered as an additional mode, but it must not replace the two required color modes.

### FR-06: Shared render model

Both frontends must consume a frontend-independent frame model.

Minimum conceptual data:

- Frame width and height in character cells.
- A sequence of cells.
- Each cell contains a character and foreground RGB.
- Optional background RGB.
- Presentation timestamp in a documented unit.

The core model must not depend on `crossterm`, `egui`, or `wgpu`.

### FR-07: Terminal frontend

- Render frames in the terminal.
- Use Crossterm for terminal control and input handling.
- Use buffered output; avoid flushing after every individual character.
- Minimize cursor movement and redundant escape sequences.
- Restore terminal state on normal exit and handled errors.
- Support pause/resume and quit.
- Support configurable output width/height and color mode.

Planned controls:

| Key | Action |
|---|---|
| Space | Pause / resume |
| Q | Quit |
| Left / Right | Seek backward / forward, if seek is supported by the decoder |
| + / - | Increase / decrease playback speed |
| C | Cycle color mode |

If seeking is not supported in the MVP decoder, do not expose a non-functional seek control. Document the limitation.

### FR-08: Desktop GUI frontend

- Use `eframe` / `egui` for the desktop application shell and controls.
- Provide file selection, preview, play/pause/stop, timeline where supported, output size, color mode, and playback speed controls.
- The GUI event loop must not be blocked by decoding or frame processing.
- Reuse the same core frame model and playback logic where feasible.
- Initial GUI rendering may use a CPU path.
- GPU rendering through `wgpu`, glyph atlas, and instancing is a later optimization.
- Do not create one egui widget per ASCII character.

### FR-09: Playback synchronization

- Schedule presentation using frame PTS/time base when available.
- Support pause/resume and playback speed.
- Handle late frames using an explicit policy.
- Use a bounded queue between processing and presentation.
- Prevent latency and memory growth from an unbounded backlog.
- Track dropped frames and timing statistics.

The initial real-time policy should prefer low latency: if a frame is too late to be useful, it may be dropped in favor of a newer frame. This policy must be documented and tested.

### FR-10: Runtime statistics

Expose or log:

- Current presentation FPS.
- Average processing time.
- Average render time, where measurable.
- Dropped frame count/ratio.
- Queue occupancy.
- Approximate memory usage if reliably available.

Do not label an estimated metric as an exact measurement.

## 7. Non-Functional Requirements

### Performance

Initial engineering targets, not guarantees:

- 30 FPS playback target for suitable clips and output sizes.
- Test terminal grids at 60×20, 100×30, and 160×45.
- Test input clips up to 1080p.
- Keep queue capacity bounded; initial default may be 2–3 frames.
- Avoid excessive thread creation on a dual-core CPU.
- Profile before adding Rayon, SIMD, or GPU complexity.

### Reliability

- No unbounded frame accumulation.
- No panic for ordinary user errors such as missing files or unsupported media.
- Terminal state must be restored on exit.
- Decoder shutdown and worker shutdown must be coordinated.
- Handle window close and cancellation cleanly.

### Maintainability

- Use modular crates with clear dependency direction.
- Keep the core independent from frontends.
- Use typed errors for library boundaries.
- Add tests for algorithms and timing behavior.
- Document external system dependencies and build steps.

## 8. Architecture and Workspace Expectations

Planned Cargo workspace:

```text
rgb-ascii-renderer/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── crates/
│   ├── renderer-core/
│   ├── video-decoder/
│   ├── terminal-renderer/
│   └── gui-renderer/
├── apps/
│   ├── cli/
│   └── desktop/
├── tests/
├── benches/
└── docs/
    ├── SPECIFICATION.md
    ├── ARCHITECTURE.md
    └── ROADMAP.md
```

This is a target structure, not a mandate to create every crate immediately. OpenCode should create only what is needed for the current roadmap phase.

## 9. Technology Decisions

| Concern | Preferred technology | Notes |
|---|---|---|
| Language | Rust stable | Pin minimum supported Rust version only after dependency verification |
| CLI | `clap` | Argument parsing |
| Terminal | `crossterm` | Terminal control and input |
| GUI | `eframe` + `egui` | Desktop shell and controls |
| Image utilities | `image` | Static image utilities and supported conversions |
| Video decode MVP | FFmpeg subprocess | Keep behind a decoder abstraction |
| Native video decode later | Evaluate `ffmpeg-next` or another maintained binding | Verify current compatibility and build requirements before adoption |
| Parallel CPU work | `rayon` | Add only after profiling |
| Serialization | `serde` + `toml` | Configuration |
| Errors | `thiserror` + `anyhow` | Library and application boundaries |
| Logging | `tracing` | Structured diagnostics |
| Benchmarking | `criterion` | Microbenchmarks |

Do not assume a crate version, FFmpeg API, or system package name. Verify current documentation and the actual target environment before changing dependencies.

## 10. Acceptance Criteria

### MVP acceptance

- [ ] CLI accepts a valid local video path.
- [ ] Decoder produces frames incrementally.
- [ ] PTS or equivalent timing information is retained.
- [ ] Static frame conversion produces deterministic ASCII output.
- [ ] True Color terminal mode works.
- [ ] 256-color terminal mode works.
- [ ] Terminal playback supports pause, resume, and quit.
- [ ] Frame queue is bounded.
- [ ] EOF and common decoder errors are handled.
- [ ] Terminal state is restored after exit.
- [ ] Unit and integration tests pass.
- [ ] README documents build dependencies and usage.

### Desktop acceptance

- [ ] GUI opens a local video.
- [ ] GUI preview uses the shared render model.
- [ ] GUI controls do not block the event loop.
- [ ] Play, pause, stop, and supported seeking work.
- [ ] CPU fallback works without GPU initialization.

### Release acceptance

- [ ] Linux builds and tests pass.
- [ ] Windows builds are verified before claiming Windows support.
- [ ] Benchmarks are recorded.
- [ ] No known unbounded queue or memory growth.
- [ ] User-facing errors are actionable.
- [ ] Documentation matches implemented behavior.

## 11. OpenCode Operating Instructions

OpenCode must follow these rules when working in this repository:

1. Read `SPECIFICATION.md`, `ARCHITECTURE.md`, and `ROADMAP.md` before making changes.
2. Identify the current roadmap phase and implement only that phase unless explicitly instructed otherwise.
3. Inspect the existing repository before creating or replacing files.
4. Do not overwrite user work or perform destructive Git operations.
5. Do not introduce speculative dependencies. Verify crate maintenance, API compatibility, platform support, and build requirements.
6. Prefer small, reviewable changes.
7. Run relevant formatting, tests, and checks after each implementation unit.
8. Never claim a test, benchmark, build, or platform verification was performed unless it actually ran.
9. Record blockers and assumptions explicitly.
10. Update documentation when implementation decisions materially change.
11. Do not add audio, streaming, GPU-only requirements, or other non-goals without approval.
12. Do not start GUI or GPU work before the terminal MVP and shared core contracts are validated.

## 12. Definition of Done

A task is complete only when:

- The requested behavior is implemented.
- Relevant tests are added or updated.
- Formatting and applicable checks pass.
- Errors and edge cases are considered.
- Documentation is updated where needed.
- The final report lists changed files, commands executed, test results, and known limitations.

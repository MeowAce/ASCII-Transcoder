# RGB ASCII Video Renderer — Roadmap

> Execution plan for OpenCode. Work sequentially, keep changes small, and do not advance phases by assumption. A phase is complete only when its exit criteria are demonstrated by commands, tests, or documented evidence.

## 1. Execution Principles

1. Read all three project documents before implementation.
2. Inspect the repository and report its current state before modifying files.
3. Implement only the active phase.
4. Prefer small, reviewable commits or patches; do not commit unless requested.
5. Do not overwrite existing user work.
6. Verify current crate APIs and platform requirements before adding dependencies.
7. Run formatting, tests, and relevant checks after each coherent task.
8. Report actual results; never fabricate test, build, or benchmark outcomes.
9. Update documentation when behavior or architecture changes.
10. Ask before expanding scope or changing locked decisions.

## 2. Phase Overview

| Phase | Name | Main outcome | Exit gate |
|---|---|---|---|
| P0 | Foundation | Workspace and engineering baseline | Workspace builds and checks |
| P1 | Static ASCII Engine | Image/frame to colored ASCII | Deterministic algorithm tests pass |
| P2 | Video + Terminal MVP | Real video playback in terminal | End-to-end playback works |
| P3 | Real-time Optimization | Measured low-latency playback | Benchmarks and profiling report |
| P4 | Desktop GUI | Shared engine in desktop app | GUI works without blocking UI |
| P5 | GPU + Release | Optional GPU glyph renderer and release readiness | CPU fallback and platform verification |

## 3. P0 — Project Foundation

### Tasks

- Inspect repository, Git status, existing files, and toolchain.
- Confirm Rust stable and Cargo availability.
- Verify FFmpeg availability on the development host.
- Create Cargo workspace.
- Establish crate boundaries:
  - `renderer-core`
  - `video-decoder`
  - `terminal-renderer`
  - `gui-renderer`
  - `apps/cli`
  - `apps/desktop`
- Create only minimal crate manifests and library/application entry points needed to validate workspace structure.
- Configure formatting and linting conventions.
- Add basic logging and error-handling setup.
- Add README build/development notes.
- Add CI only if repository context and target environment are understood.

### Constraints

- Do not implement full video decoding in P0.
- Do not add wgpu.
- Do not add Rayon without a measured need.
- Do not introduce unnecessary abstractions.

### Exit criteria

- [ ] Workspace metadata resolves.
- [ ] Workspace builds.
- [ ] `cargo fmt --check` passes.
- [ ] Applicable `cargo clippy` check passes.
- [ ] Crate dependency direction matches ARCHITECTURE.md.
- [ ] README describes prerequisites and workspace commands.

## 4. P1 — Static ASCII Rendering Engine

### Tasks

1. Define and test `Rgb`, `RenderCell`, and `RenderFrame`.
2. Define frame dimensions and row-major cell layout.
3. Implement luminance calculation.
4. Implement default ASCII palette mapping.
5. Implement deterministic RGB-to-256-color mapping.
6. Implement True Color serialization helpers only in the terminal crate, not in core.
7. Implement image loading for test fixtures.
8. Implement output-grid resizing and aspect-ratio correction.
9. Add a small CLI path that renders a static image to terminal, if useful for end-to-end validation.
10. Add unit tests and Criterion benchmarks for hot algorithms.

### Test cases

- Black pixel maps to darkest palette entry.
- White pixel maps to brightest palette entry.
- Palette mapping is bounded and deterministic.
- RGB channels are preserved in the core model.
- 256-color mapping always returns an index in 0..=255.
- RenderFrame cell count matches dimensions.
- Aspect ratio calculations handle invalid/zero dimensions safely.

### Exit criteria

- [ ] Static image converts to ASCII.
- [ ] True Color output is visually verified in a compatible terminal.
- [ ] 256-color output is visually verified.
- [ ] Unit tests pass.
- [ ] Initial algorithm benchmarks are recorded.
- [ ] Core has no frontend dependencies.

## 5. P2 — Video Decoder and Terminal MVP

### Tasks

1. Define decoder abstraction and metadata types.
2. Implement FFmpeg subprocess adapter.
3. Verify raw frame protocol, including pixel format and exact frame byte length.
4. Correctly handle partial reads from stdout.
5. Drain and retain useful stderr diagnostics without deadlock.
6. Implement cancellation and child-process cleanup.
7. Verify timestamp strategy. Do not assume CFR if source timestamps are unavailable.
8. Connect decoded frames to the core processor.
9. Implement bounded queues and backpressure.
10. Implement PTS-based playback scheduling.
11. Implement Crossterm terminal setup, rendering, input, and cleanup.
12. Add pause/resume/quit controls.
13. Add output size and color mode CLI options.
14. Add integration fixtures for a small known video.

### MVP command examples

```bash
rgb-render video.mp4
rgb-render video.mp4 --color-mode truecolor
rgb-render video.mp4 --color-mode 256
rgb-render video.mp4 --width 100 --height 30
rgb-render video.mp4 --fps 30
```

These are target interface examples. Keep CLI names consistent once implemented.

### Important technical gate

Before claiming variable-frame-rate correctness, demonstrate that the decoder path preserves usable per-frame PTS. If the chosen FFmpeg raw pipe cannot provide reliable timestamps, stop and propose a timestamp side channel or alternate decoder approach.

### Exit criteria

- [ ] A supported local video plays in terminal.
- [ ] True Color and 256-color modes work.
- [ ] Playback uses timestamps rather than a fixed sleep-only loop.
- [ ] Pause/resume/quit work.
- [ ] Queue capacity is bounded.
- [ ] EOF and decoder failures are handled.
- [ ] Terminal state is restored after normal exit and handled errors.
- [ ] Integration tests pass.
- [ ] README contains real build and run instructions.

## 6. P3 — Real-time Optimization

### Baseline measurement

Measure before optimizing. Use these terminal grids:

- 60×20
- 100×30
- 160×45

Use representative clips and record:

- Average presented FPS.
- 1% low FPS where the measurement method supports it.
- Decode time.
- Processing time.
- Terminal serialization/write time.
- Dropped frames.
- Queue occupancy.
- CPU usage.
- Peak memory or a clearly described memory estimate.

### Optimization tasks

- Reuse frame and output buffers where practical.
- Reduce per-cell allocations.
- Batch terminal writes.
- Avoid redundant ANSI color changes.
- Evaluate full-frame redraw versus diff rendering.
- Tune bounded queue capacity.
- Tune worker count for low-core-count hardware.
- Evaluate Rayon only if CPU profiling shows parallelizable processing is a bottleneck.
- Evaluate SIMD only after profiling and correctness tests.
- Add performance regression benchmarks.

### Constraints

- Do not optimize based on intuition alone.
- Do not trade correctness or timestamp handling for an unreported FPS increase.
- Do not add GPU work in this phase.
- Keep the CPU path fully functional.

### Exit criteria

- [ ] Baseline and post-optimization results are documented.
- [ ] Bottleneck is identified with evidence.
- [ ] Memory remains bounded during long playback.
- [ ] No severe latency growth from queued stale frames.
- [ ] Tests still pass.
- [ ] Performance changes are justified by measurements.

## 7. P4 — Desktop GUI

### Tasks

1. Build the eframe/egui application shell.
2. Add file picker and media metadata display.
3. Add preview surface using the shared RenderFrame model.
4. Add play/pause/stop.
5. Add timeline and seeking only if decoder seek support is implemented correctly.
6. Add output dimensions, color mode, palette, and playback speed controls.
7. Add FPS, processing time, and dropped-frame statistics.
8. Move decode and processing off the UI thread.
9. Implement clean cancellation and application shutdown.
10. Provide a CPU rendering path.
11. Add GUI integration/manual test checklist.

### Constraints

- Do not create one egui widget per character.
- Do not duplicate ASCII conversion logic in GUI.
- Do not block the egui event loop.
- Do not require wgpu-specific functionality for basic GUI operation.
- Do not expose controls that are not implemented.

### Exit criteria

- [ ] GUI opens a supported video.
- [ ] Preview displays the shared core render output.
- [ ] UI remains responsive during playback.
- [ ] Play/pause/stop work.
- [ ] Seeking works only if genuinely supported.
- [ ] CPU fallback works.
- [ ] GUI shutdown stops workers and decoder cleanly.

## 8. P5 — GPU Rendering and Release

### GPU tasks

- Profile GUI CPU renderer first.
- Implement glyph atlas.
- Design compact per-cell instance data.
- Batch character rendering with wgpu.
- Avoid one draw call per character.
- Handle device/adapter initialization failures.
- Preserve CPU fallback.
- Compare CPU and GPU visual output.
- Benchmark on available hardware; do not assume Intel UHD 620 or WSL exposes a suitable adapter.

### Release tasks

- Verify Linux build and runtime dependencies.
- Verify Windows build before claiming Windows support.
- Document FFmpeg installation and codec limitations.
- Add release profile configuration where justified.
- Add version/help output.
- Review error messages.
- Run unit, integration, and regression tests.
- Record final benchmark environment and results.
- Document known limitations.

### Exit criteria

- [ ] GPU renderer works where a compatible adapter exists.
- [ ] CPU fallback works when GPU is unavailable.
- [ ] Linux build and runtime are verified.
- [ ] Windows support is claimed only after actual verification.
- [ ] Documentation is complete and accurate.
- [ ] Release artifacts are reproducible.

## 9. Suggested OpenCode Task Workflow

For every task, OpenCode should:

1. State the active phase and task objective.
2. Inspect relevant files and current Git status.
3. List a short implementation plan.
4. Implement the smallest coherent change.
5. Run formatting and targeted tests.
6. Run broader checks when appropriate.
7. Inspect the diff for unrelated changes.
8. Report:
   - Files changed.
   - Behavior implemented.
   - Commands executed.
   - Test/build results.
   - Known limitations.
   - Recommended next task.

Do not silently skip failed checks. Explain failures and distinguish pre-existing failures from regressions where evidence permits.

## 10. Initial OpenCode Prompt

Use this prompt when starting work in the repository:

```text
You are working on the RGB ASCII Video Renderer Rust project.

First read:
- docs/SPECIFICATION.md
- docs/ARCHITECTURE.md
- docs/ROADMAP.md

Do not modify files yet.

Inspect the repository, Rust toolchain, FFmpeg availability, current Git status, and existing project structure. Identify the current roadmap phase based on actual repository evidence.

Then report:
1. Current project state.
2. Which roadmap phase is active.
3. Missing prerequisites or blockers.
4. The smallest next implementation task.
5. Files likely to be created or changed.
6. Commands and tests that will verify the task.

Respect all locked decisions and non-goals in the documents. Do not invent completed work, benchmark results, or platform support. Do not overwrite user changes. Wait for approval before beginning implementation.
```

## 11. First Implementation Prompt (After Review)

```text
Implement P0 only, following docs/SPECIFICATION.md, docs/ARCHITECTURE.md, and docs/ROADMAP.md.

Before editing, inspect Git status and preserve all existing user work. Create the minimal Cargo workspace and crate skeleton needed to validate the intended dependency boundaries. Do not implement video decoding, ASCII conversion, GUI features, GPU rendering, or speculative optimizations in this phase.

Verify workspace metadata, build, formatting, and applicable lint checks. Report exact commands and results, changed files, blockers, and the next recommended task. Do not commit unless explicitly requested.
```

## 12. Progress Tracking Template

Maintain a concise progress section in the project README or a dedicated `docs/PROGRESS.md`:

```markdown
## Current Progress

- Active phase: P0
- Status: Not started
- Last verified commit: <commit or N/A>
- Last verification date: <date>

### Completed
- None

### In progress
- None

### Blocked
- None

### Verification
- Build:
- Tests:
- Formatting:
- Clippy:
- Benchmarks:
```

Update status only with evidence from the repository and actual command output.

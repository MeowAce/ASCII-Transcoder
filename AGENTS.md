# AGENTS.md

## Current state (verify before assuming)

Cargo workspace `rgb-ascii-renderer` (root `Cargo.toml`), edition 2024, Rust stable pinned via `rust-toolchain.toml`. Active roadmap phase: **P1 (Static ASCII Engine), implemented** — the pipeline and terminal serializer work and are tested, but P1 visual sign-off is outstanding and no video playback exists.

`rgb-render` renders **static images only**. `rgb-desktop` is a stub that exits with status 2. Do not treat a green build as evidence that video works end to end.

Git is initialized on branch `master` with **zero commits**. Anything requiring `HEAD`, a `main` branch, or a base ref (e.g. `detect-changes --scope compare --base-ref main`) cannot run until a first commit exists.

Environment: WSL. `ffmpeg` n9.0.2 and `ffprobe` at `/usr/sbin/ffmpeg`. Rust `1.100.0` with rustfmt + clippy installed. Do not report build, test, benchmark, or platform results you did not actually run.

## Read first

Read all three docs in `docs/` before editing. They are the authoritative contract, not background reading:
- `docs/SPECIFICATION.md` — scope, non-goals, **LOCKED** decisions, acceptance criteria.
- `docs/ARCHITECTURE.md` — crate boundaries, dependency direction, pipeline/data model.
- `docs/ROADMAP.md` — phases P0–P5; implement only the active phase.
- `docs/PROGRESS.md` — current phase status and verified check results.

## The rules agents most often break

These are enforced by the docs and are easy to violate accidentally:

1. **Cleanup is mandatory.** Restore terminal to normal before exit; kill FFmpeg child processes on cancel/exit.
2. **Queues must be bounded.** Never use unbounded channels for full frame payloads.
3. **Don't fabricate provenance.** No claimed verification (build/test/bench/codec/platform support) unless you actually ran it.
4. **No scope creep.** Audio is out of scope. Don't add wgpu/Rayon/SIMD/abstraction without profiling or measured need. Don't start GUI/GPU work before the terminal MVP and shared core contracts exist.
5. **Don't add dependencies speculatively.** Verify API compatibility, maintenance status, and build requirements first.
6. **Don't overwrite user work.** Inspect Git status and files first. Don't commit unless asked.

## Architecture invariants (the highest-risk ones)

- `renderer-core` is the lowest crate: **decoder → core transform → single shared `RenderFrame`; both frontends present, neither copies the algorithm.** Don't implement luminance/palette conversion twice.
- Shared contracts live in `renderer-core` (`Rgb`, `RenderCell`, `RenderFrame`, `DecodedFrame`, `VideoMetadata`, `PixelFormat`, `Rational`). `video-decoder` depends on the core *for these types only*.
- `renderer-core` must not depend on `crossterm`, `egui`/`eframe`, or `wgpu`, and must not contain ANSI escape sequences.
- ANSI/text serialization belongs **only** in `terminal-renderer`. Color mode conversion lives there too — never in core.
- Crates use typed errors (`thiserror`); `anyhow` only at application entrypoints.
- Correctness over speed: don't assume CFR — always use real decoded PTS + time base as the timing source, and never claim correctness you haven't tested against real media. `VideoMetadata::nominal_frame_rate` is display-only, never a scheduling input.

## Commands

```bash
cargo build --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p renderer-core <name>      # single test
cargo run -p cli -- <file>              # terminal frontend (bin: rgb-render)
cargo run -p desktop                    # desktop frontend (bin: rgb-desktop)
```

Binary name is `rgb-render`, package name is `cli`. Don't pass `--bin rgb-render` to `cargo run` without `-p`; use `-p cli`.

Check dependency direction with `cargo tree -p <crate> -e normal` — `--no-dev` is not a valid flag on this Cargo.

FFmpeg integration tests require a local video fixture; none is committed. Generate one with `ffmpeg -f lavfi -i testsrc=...` and document it. **Always include `-fps_mode passthrough`** in decoder invocations: FFmpeg's default `rawvideo` output silently converts to CFR and drops frames (measured: 20-frame VFR clip → 17 frames), which corrupts PTS-to-frame pairing without any error.

## P1 gotchas

- The pipeline lives in `renderer-core/src/pipeline.rs` and is the **only** place luminance/palette/sampling may exist. Add algorithms there, not in a frontend.
- `image` is a dependency of `apps/cli` **only**, and must keep `default-features = false`: the default feature set pulls in `rayon`, which the spec forbids before profiling. Verify with `cargo tree --workspace -e normal | grep -c rayon` → must be `0`.
- Luminance uses fixed-point integer math (weights scaled by 10_000) on **gamma-encoded** channels. Don't switch to floats; the exact primaries are asserted in tests (red=54, green=182, blue=18).
- Box sampling costs ~3 ms/frame on a 1080p source regardless of grid size (touches every source pixel); point sampling is ~30× cheaper. Don't "optimize" this without a benchmark run — `cargo bench -p benches` is the recorded baseline.
- CLI binary is `rgb-render`, package `cli`. Usage: `rgb-render <IMAGE> [--width N] [--color-mode truecolor|256] [--sampling box|point] [--no-cell-aspect] [--background]`.

## Response format

Report: changed files, what changed, exact commands run, actual results, known limitations, recommended next task. If a check fails, say what failed and why.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **ASCII** (117 symbols, 113 relationships, 0 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/ASCII/context` | Codebase overview, check index freshness |
| `gitnexus://repo/ASCII/clusters` | All functional areas |
| `gitnexus://repo/ASCII/processes` | All execution flows |
| `gitnexus://repo/ASCII/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->

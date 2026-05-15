# AGENTS.md — guidance for AI coding agents working in this repo

This file targets agents other than Claude Code (Codex, Cursor, Aider, etc.). Claude Code has its own file at [`CLAUDE.md`](CLAUDE.md); the substance is the same.

## Repo at a glance

- Rust 2021 workspace.
- `crates/meme-garden-core` — simulation engine (library, no I/O beyond config parsing).
- `crates/meme-garden-cli` — `clap` + `ratatui` TUI and a headless CSV runner.
- Current state: boilerplate + a minimal POC (agents move/eat on a grid + one transmissible meme).
- Long-term vision: [`docs/design.md`](docs/design.md). The MVP research question is whether cooperative memes survive against selfish ones under scarcity.

## Commands

- `cargo check --workspace`
- `cargo test --workspace` (includes a determinism test)
- `cargo run -p meme-garden-cli -- run --seed 42`
- `cargo run -p meme-garden-cli -- headless --seed 42 --ticks 500`

## Hard rules

1. **Determinism.** All randomness in `meme-garden-core` MUST use `rng::SimRng`. No `thread_rng`, no `SystemTime`, no `Instant` in core. Same seed → identical metrics stream.
2. **No I/O in core** except config parsing. TUI / stdout / env vars belong in the CLI crate.
3. **Stable agent iteration order** (by `AgentId`). Don't introduce hash-based iteration in the tick loop.
4. **No real AI SDK in core.** `core::ai` ships traits and a `NoopProvider`; real providers go in a separate crate.

## Conventions

- Errors: `thiserror` at library boundaries, `anyhow` in the binary.
- Comments only for non-obvious *why* — never restate what the code does.
- Match `enum`s exhaustively (no wildcard arms on internal enums) so the compiler flags every site when the meme grammar expands.
- One concept per change; no incidental refactors.

## Where to extend

- Symbolic meme grammar (`Trigger` / `Effect` / `Target`): `meme-garden-core/src/meme.rs`.
- Transmission/mutation/recombination helpers: also `meme.rs`; the tick loop in `world.rs` should call into them, not inline the logic.
- AI providers: a new crate that depends on `meme-garden-core` (never the reverse), implementing `ai::MemeNamer` / `ai::RunAnalyst`.
- Visualization additions: `meme-garden-cli/src/tui.rs`.

See [`CLAUDE.md`](CLAUDE.md) for the same content with a few extra notes specific to Claude Code's tooling.

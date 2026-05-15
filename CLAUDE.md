# CLAUDE.md — guidance for Claude Code in the Meme Garden repo

This file orients Claude Code (and any compatible agent) before it edits code here. Read [`docs/design.md`](docs/design.md) for the long-term vision; this file is about *how* to work in the repo today.

## Snapshot

- Rust workspace, edition 2021, stable toolchain (`rust-toolchain.toml`).
- Two crates:
  - `crates/meme-garden-core` — pure simulation engine, no I/O beyond config parsing.
  - `crates/meme-garden-cli` — `clap` CLI with a `ratatui` TUI and a headless CSV runner.
- Current iteration is a **boilerplate + tiny POC**: agents move/eat on a grid, one transmissible meme (`SharerNorm`). The rest of the design lives in docs and is intentionally not implemented yet.

## Commands

```sh
cargo check --workspace
cargo test --workspace
cargo run -p meme-garden-cli -- run --seed 42
cargo run -p meme-garden-cli -- headless --seed 42 --ticks 500
```

## Invariants (do not violate)

1. **Determinism is sacred.** All randomness in `meme-garden-core` flows through `rng::SimRng`. Never call `rand::thread_rng()`, `std::time::*`, or any nondeterministic source inside the core. Same seed must produce a bit-identical metrics stream — the test in `world.rs::tests::same_seed_same_metrics` enforces this.
2. **Core has no terminal/IO concerns.** TUI, stdout, env vars, file system writes belong in `meme-garden-cli`. Config parsing is the only file I/O allowed in core.
3. **Stable iteration order.** Agents are processed in `AgentId` order. Don't introduce `HashMap` iteration over agents in the hot path without sorting.
4. **No real AI deps in core.** The `ai` module ships traits + a `NoopProvider` only. When a real provider is added later, it lives in a separate crate (`meme-garden-ai` or similar) that depends on `meme-garden-core` — never the other way around.

## Where things will grow

- **Symbolic meme grammar** lands in `meme-garden-core/src/meme.rs`. Today `MemeKind` is a one-variant enum; it'll become a `{ trigger, effect, target, strength, transmissibility, mutation_rate }` struct. Match exhaustively so the compiler flags every site that needs updating.
- **Mutation + recombination** belong next to `Meme` itself, not scattered across `world.rs`. The meme phase in `world.rs` should call into `meme::transmit(&mut rng, source, recipient)`.
- **AI providers** plug in behind `ai::MemeNamer` / `ai::RunAnalyst`. Add new providers in a new crate; `core` stays HTTP-free.
- **Visualization** lives in `meme-garden-cli/src/tui.rs`. Adding a meme-phylogenetic-tree pane is a `Layout::vertical` split on the right sidebar.

## Style

- Prefer small, focused PRs (one concept per change).
- No comments narrating *what* well-named code does. Comments only for *why* something is non-obvious — surprising invariants, performance traps, deliberate simplifications for this iteration.
- Don't add backwards-compat shims for code that doesn't have external consumers yet.
- Errors at lib boundary use `thiserror`; the binary uses `anyhow`.

## North star

Read [`docs/design.md`](docs/design.md). The MVP question we want to be able to ask first:

> Can a cooperative meme survive against a selfish meme under different levels of scarcity, mutation, and social copying?

Today's POC proves the *transmission pipeline*. Everything else is scaffolding for that question.

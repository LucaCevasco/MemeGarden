# Implementation Plan: Meme Garden MVP — Memetic Petri Dish

**Branch**: `001-meme-garden-mvp` | **Date**: 2026-05-19 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-meme-garden-mvp/spec.md`

## Summary

Grow today's transmission-pipeline POC into a measurable petri dish: bounded symbolic
memes with `{trigger, target, effect, strength, transmissibility, mutation_rate, cognitive_cost,
lineage}`, agent inventories of multiple memes, mutation + lineage tracking, six starter
memes, a richer per-tick + per-event metrics surface emitted as JSON-Lines (plus a CSV
summary), a TOML config that drives the whole sweep surface, a ratatui TUI with a
meme-dynamics side pane, a headless mode that runs in the same simulator with the same
seed, and AI seams behind traits — kept symbolic in the hot loop, deterministic everywhere.

The technical approach is to **extend the existing two-crate workspace** (`meme-garden-core`
+ `meme-garden-cli`) rather than rebuild. Each MVP capability slots into a file that
already exists or one new file alongside it. AI provider implementations stay out of the
workspace for now — only the traits + a `NoopProvider` ship in `core::ai`, per constitution
Principle IV.

## Technical Context

**Language/Version**: Rust 2021 edition, stable toolchain pinned via `rust-toolchain.toml`
(MSRV 1.75 per existing `Cargo.toml`).

**Primary Dependencies**: `serde` + `toml` (config), `rand` + `rand_pcg` (deterministic
PRNG funneled through `core::rng::SimRng`), `thiserror` (lib boundaries), `anyhow` (binary),
`tracing`. CLI: `clap`, `ratatui`, `crossterm`. Metrics serialization uses `serde_json` for
JSON-Lines output (new dep) plus the existing CSV row formatter for human-readable
summaries.

**Storage**: Plain files. TOML for inputs (`configs/*.toml`); JSON-Lines for the per-tick
+ per-event metrics stream (`runs/<timestamp>-<short-name>/events.jsonl`); CSV for a flat
per-tick summary suitable for spreadsheet inspection
(`runs/<timestamp>-<short-name>/summary.csv`); the run's resolved config is copied alongside
as `config.toml` so the artifact is self-describing (FR-020). No database.

**Testing**: `cargo test --workspace`. Unit tests live next to source (`#[cfg(test)]`).
Integration tests in `crates/meme-garden-core/tests/`. Two regression suites are
load-bearing: (1) the existing `world.rs::tests::same_seed_same_metrics` extended to cover
the new metric fields; (2) a new milestone integration test that runs the
cooperative-vs-selfish preset under three scarcity levels with fixed seeds and asserts the
recorded direction of survival is stable from run to run.

**Target Platform**: macOS / Linux developer laptops. Headless mode also runnable in CI.
No native dependencies beyond what `crossterm` already requires for the TUI; the headless
binary path MUST link without `crossterm`/`ratatui` runtime use.

**Project Type**: Rust workspace, multiple crates. Single-project layout per the
`plan-template.md` Option 1, with crates substituting for the template's bare `src/`.

**Performance Goals**: 1,000-tick headless run on the default-sized world completes in
under 30 s on a typical developer laptop (SC-004). Allocations in the per-tick hot path
are kept bounded — no per-tick `HashMap` construction over agents, no per-tick re-parsing
of config.

**Constraints**: Constitution principles bind. Specifically: (I) every stochastic decision
flows through `SimRng`; (II) `meme-garden-core` performs no terminal/network/filesystem-write
I/O — only config read; (III) iteration over agents and memes is in stable id order;
(IV) memes stay symbolic and bounded — no LLM in the tick path; (V) every new behavioral
claim ships with a metric.

**Scale/Scope**: Default world ~60×30 cells, ~120 agents, ~1,000 ticks. MVP is not
optimizing for thousands of agents; structural choices (e.g. `Vec<Agent>` with stable
ordering) hold up to a few thousand entities, which is far above MVP scale.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Each principle is evaluated against the design that follows. All gates **PASS**; no
entries are needed in Complexity Tracking.

| # | Principle | Verdict | Evidence in this plan |
|---|---|---|---|
| I | Determinism Is Sacred | PASS | All new randomness (mutation, recombination, transmission roll, copying bias roll, target tie-break) goes through `SimRng`. New regression test in Phase 1 asserts bit-identical metrics under fixed seed. |
| II | Pure Core, Impure Edges | PASS | JSONL/CSV writes, run-directory creation, and TUI rendering all live in `meme-garden-cli`. `core` keeps emitting `Metrics` and `Event` values; serialization helpers live in `core` but the file I/O does not. |
| III | Stable Iteration Order | PASS | Agents continue to live in `Vec<Agent>` indexed by `AgentId`. The new meme inventory is `Vec<Meme>` ordered by insertion / `MemeId`. No `HashMap` iterates the per-tick hot path. |
| IV | Symbolic Memes, Not Black Boxes | PASS | `Meme` becomes `{trigger, target, effect, strength, transmissibility, mutation_rate, cognitive_cost, lineage}` with each symbolic field drawn from a small Rust enum. Mutation operates on these fields; no string interpretation. `core::ai` keeps trait + `NoopProvider` only. |
| V | Metrics-First Experimentation | PASS | The new symbolic-meme behaviors each ship with a metric (mutation events, transmission events, per-meme carrier fitness, per-cluster group fitness, dominance, extinction events). Milestone regression test reads these metrics — not the UI — to answer the north-star question. |

**Re-check note**: After Phase 1 artifacts are written, re-evaluate this table. The
post-design verdicts are recorded in the appendix at the bottom of this file.

## Project Structure

### Documentation (this feature)

```text
specs/001-meme-garden-mvp/
├── plan.md                  # This file (/speckit-plan output)
├── research.md              # Phase 0 — decision log for non-obvious choices
├── data-model.md            # Phase 1 — entities, fields, state transitions
├── quickstart.md            # Phase 1 — how to run a baseline + milestone experiment
├── contracts/
│   ├── config.schema.md     # TOML config schema
│   ├── metrics.schema.md    # JSONL event + per-tick metric schema
│   ├── ai-seams.md          # Trait signatures for MemeNamer / ExperimentDesigner / RunAnalyst
│   └── cli.md               # CLI subcommand surface
├── checklists/
│   └── requirements.md      # Spec quality checklist (from /speckit-specify)
└── tasks.md                 # Phase 2 — produced by /speckit-tasks (not here)
```

### Source Code (repository root)

```text
crates/
├── meme-garden-core/
│   ├── src/
│   │   ├── lib.rs               # Re-exports public surface
│   │   ├── rng.rs               # SimRng (existing); only source of randomness
│   │   ├── config.rs            # SimConfig + sub-configs (extended for new params)
│   │   ├── agent.rs             # Agent struct (extended: inventory, traits, trust, memory)
│   │   ├── meme.rs              # Meme struct + Trigger/Target/Effect/MemeKind enums
│   │   ├── policy.rs            # NEW: agent default policy + meme-driven modifiers
│   │   ├── mutation.rs          # NEW: mutate / recombine; lineage helpers
│   │   ├── lineage.rs           # NEW: MemeLineage graph (parent links, ids)
│   │   ├── starters.rs          # NEW: six starter memes as constructors
│   │   ├── world.rs             # Simulation tick (extended: new phases & action set)
│   │   ├── action.rs            # NEW: enum Action { Move, Eat, Share, Attack, Imitate, Transmit, Reproduce }
│   │   ├── metrics.rs           # Per-tick Metrics (extended) + Event enum
│   │   ├── presets.rs           # NEW: cooperation-vs-selfishness + others
│   │   └── ai.rs                # MemeNamer / ExperimentDesigner / RunAnalyst traits + NoopProvider
│   └── tests/
│       ├── determinism.rs       # NEW: extended same-seed regression
│       ├── milestone.rs         # NEW: cooperation-vs-selfish under 3 scarcity levels
│       ├── mutation.rs          # NEW: bounded mutation invariants
│       └── transmission.rs      # NEW: starter-meme transmission behaviors
└── meme-garden-cli/
    └── src/
        ├── main.rs              # clap subcommands (run | headless | export | list-presets)
        ├── app.rs               # Shared state for TUI runs
        ├── tui.rs               # Ratatui TUI (extended: meme dynamics pane)
        ├── runner.rs            # NEW: shared run loop used by TUI + headless
        └── export.rs            # NEW: JSONL + CSV writers, run-dir layout

configs/
├── default.toml                 # baseline config (existing; extended for new fields)
└── presets/
    ├── cooperation-vs-selfish-low.toml     # NEW
    ├── cooperation-vs-selfish-mid.toml     # NEW
    └── cooperation-vs-selfish-high.toml    # NEW

runs/                            # NEW: gitignored output directory for run artifacts
└── <YYYYMMDD-HHMMSS>-<short>/   # one directory per run
    ├── config.toml              # resolved config (FR-020)
    ├── events.jsonl             # per-tick + per-event records (FR-021, FR-022)
    └── summary.csv              # flat per-tick summary
```

**Structure Decision**: Single Rust workspace, two crates today, designed so a future
third crate (`meme-garden-ai`) can be added later for live LLM providers without touching
`meme-garden-core`. The `runs/` directory is added to `.gitignore` so artifact volume
doesn't bloat the repo. AI provider implementations are explicitly **not** introduced in
this MVP — only the trait surfaces in `core::ai`.

## Complexity Tracking

*No violations of the constitution. No entries required.*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| _(none)_  | _(n/a)_    | _(n/a)_                              |

## Post-Design Constitution Re-check

After drafting `research.md`, `data-model.md`, `contracts/*`, and `quickstart.md`, the
constitution gate is re-evaluated:

| # | Principle | Verdict | Notes after design |
|---|---|---|---|
| I | Determinism Is Sacred | PASS | `data-model.md` confirms every new stochastic site (mutation roll, transmission roll, copying bias, recombination, target tie-break, extinction trigger) routes through `SimRng`. |
| II | Pure Core, Impure Edges | PASS | `contracts/cli.md` keeps all file-I/O in `meme-garden-cli::export`. Core surfaces an `Event` stream returned from `Simulation::step` and an `events_drain()` accessor; serialization is `serde`-driven but file writes are CLI-side. |
| III | Stable Iteration Order | PASS | `data-model.md` specifies `Vec<Agent>` (indexed by `AgentId`) and a per-agent `Vec<Meme>` inventory ordered by insertion. No `HashMap` is iterated inside `Simulation::step`. |
| IV | Symbolic Memes, Not Black Boxes | PASS | `data-model.md` defines `Trigger`, `Target`, `Effect`, `MemeKind`, `AgentTrait` as small enums. `mutation.rs` only mutates within these enums and bounded numeric ranges. `contracts/ai-seams.md` confirms AI traits are called only from CLI commands and post-run paths — not from `Simulation::step`. |
| V | Metrics-First Experimentation | PASS | `contracts/metrics.schema.md` enumerates every metric / event required by FR-021. The milestone integration test in `tests/milestone.rs` consumes these metrics — not the UI — to answer the cooperative-vs-selfish question. |

Re-check verdict: **all gates still pass after design**. No Complexity Tracking entries
added. Ready for `/speckit-tasks`.

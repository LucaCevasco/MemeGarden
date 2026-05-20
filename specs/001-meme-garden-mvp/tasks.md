---
description: "Task list for Meme Garden MVP — Memetic Petri Dish"
---

# Tasks: Meme Garden MVP — Memetic Petri Dish

**Input**: Design documents from `/specs/001-meme-garden-mvp/`

**Prerequisites**: `plan.md` (required), `spec.md` (required), `research.md`, `data-model.md`,
`contracts/*`, `quickstart.md`.

**Tests**: Included throughout. The user request and `spec.md` explicitly require unit
tests, deterministic seed tests, integration tests, and a milestone regression suite.

**Organization**: Tasks are grouped by user story so each story can be implemented,
tested, and demoed independently. US1 alone is the MVP increment.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no incomplete-task dependencies).
- **[Story]**: `[US1]`, `[US2]`, `[US3]` — only on user-story phase tasks.
- Exact file paths are included in every task.

## Path Conventions

- Rust workspace at repo root.
- Core simulator: `crates/meme-garden-core/src/`.
- Core integration tests: `crates/meme-garden-core/tests/`.
- CLI / TUI: `crates/meme-garden-cli/src/`.
- Configs: `configs/`. Presets: `configs/presets/`.
- Spec docs: `specs/001-meme-garden-mvp/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Workspace prep so every subsequent task has the deps and the artifact
layout it expects.

- [ ] T001 Add `serde_json = "1"` to `[workspace.dependencies]` in `Cargo.toml` and declare it under `[dependencies]` in `crates/meme-garden-core/Cargo.toml` and `crates/meme-garden-cli/Cargo.toml`.
- [ ] T002 [P] Add `runs/` to `.gitignore` so per-run artifact directories don't end up tracked.
- [ ] T003 [P] Add `smallvec = "1"` to `[workspace.dependencies]` in `Cargo.toml` and to `crates/meme-garden-core/Cargo.toml` for the bounded `TrustMap` and bounded `LineageNode::parents`.
- [ ] T004 [P] Add a top-of-file `# description:` comment line to `configs/default.toml` so `meme-garden list-presets` has something to surface.
- [ ] T005 Verify `cargo check --workspace` and `cargo test --workspace` still pass after the dep additions — this is the green baseline Phase 2 work refactors against.

**Checkpoint**: Workspace builds with the new deps; tests green.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Land the new core data model from `data-model.md`. Every user story depends
on these shapes existing and the existing determinism test still being green afterwards.

**CRITICAL**: No user-story work begins until Phase 2 is complete. The existing
`world.rs::tests::same_seed_same_metrics` MUST be green at the end of this phase even
if its asserted fields change shape.

### Symbolic enums and bounded types

- [ ] T006 [P] Add `Trigger`, `TargetSelector`, and `Effect` enums per `data-model.md §3b–3d` to `crates/meme-garden-core/src/meme.rs`; derive `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`.
- [ ] T007 [P] Add a `MemeKind` enum with variants `Cooperative, Defensive, Imitative, Aggressive, Punitive, Conformist, Mutant` in `crates/meme-garden-core/src/meme.rs` (replace the POC's single-variant version; downstream `match` sites must be made exhaustive).
- [ ] T008 [P] Create `crates/meme-garden-core/src/action.rs` defining `enum Action { Move(Direction), Eat, Share(AgentId), Attack(AgentId), Imitate(AgentId), Transmit(AgentId, MemeId), Reproduce(AgentId), Idle }` and a small `Direction` enum; register the module in `crates/meme-garden-core/src/lib.rs`.
- [ ] T009 [P] Create `crates/meme-garden-core/src/lineage.rs` with `LineageId`, `LineageOrigin`, `LineageNode`, and `LineageGraph` (Vec-backed, append-only) per `data-model.md §4`; register the module in `crates/meme-garden-core/src/lib.rs`.

### Entity extensions

- [ ] T010 Extend `Meme` in `crates/meme-garden-core/src/meme.rs` to the full struct: `id`, `lineage_id`, `kind`, `trigger`, `target`, `effect`, `strength`, `transmissibility`, `mutation_rate`, `cognitive_cost`; remove the `sharer_norm` constructor (starters move in T021).
- [ ] T011 Extend `Agent` in `crates/meme-garden-core/src/agent.rs`: replace `meme: Option<Meme>` with `inventory: Vec<Meme>`; add `traits: Vec<AgentTrait>`, `memory: AgentMemory`, `trust: TrustMap` per `data-model.md §2`; define `AgentTrait`, `AgentMemory`, and the `TrustMap = SmallVec<[(AgentId, f32); 8]>` alias in the same file.
- [ ] T012 [P] Extend `Metrics` in `crates/meme-garden-core/src/metrics.rs` to include all fields in `data-model.md §6` (`population_by_trait`, `meme_count`, `meme_prevalence_by_kind`, `diversity_shannon`, `dominance_top1_fraction`, `mean_age`, `transmissions_this_tick`, `mutations_this_tick`, `deaths_this_tick`, `births_this_tick`); update `csv_header()` and `to_csv_row()` to the schema in `contracts/metrics.schema.md`.
- [ ] T013 [P] Add an `Event` enum (variants: `Header, Tick, Birth, Death, Transmission, Mutation, Recombination, MemeForgotten, Extinction, ClusterSnapshot`) and the supporting sub-enums (`DeathCause`, `MutatedField`, `ExtinctionScope`) in `crates/meme-garden-core/src/metrics.rs` per `contracts/metrics.schema.md`; derive `Serialize` with a `#[serde(tag = "kind", rename_all = "snake_case")]` envelope.

### Config extensions

- [ ] T014 Extend `SimConfig` in `crates/meme-garden-core/src/config.rs` with the full sub-config tables in `contracts/config.schema.md`: add `ScarcityConfig`, `CognitionConfig`, `TransmissionConfig`, `MutationConfig`, `ReproductionConfig`, `AttackConfig`, `SharingConfig`, `MemePoolConfig`; extend `AgentConfig`, `FoodConfig`, `RunConfig` with the new fields.
- [ ] T015 Add typed validation in `crates/meme-garden-core/src/config.rs::SimConfig::validate(&self) -> Result<(), ConfigError>` covering: probabilities ∈ [0,1], `agents.count > 0`, `initial_traits_dist` sums to 1.0 ± 1e-6, `memes.seed[*].name` resolves to a known starter (deferred name set wired by T021).
- [ ] T016 Add a legacy-config adapter in `crates/meme-garden-core/src/config.rs` (separate `LegacySimConfig` struct + `From` impl into the new `SimConfig`) emitting a `tracing::warn!` line when used; document the adapter in a one-line `// why:` comment.

### Simulation skeleton

- [ ] T017 Extend `Simulation` in `crates/meme-garden-core/src/world.rs`: add `lineage: LineageGraph`, `next_agent_id`, `next_meme_id`, `next_lineage_id`, `pending_events: Vec<Event>` fields per `data-model.md §8`; keep existing fields. Update `Simulation::new` accordingly.
- [ ] T018 Add `pub fn events_drain(&mut self) -> Vec<Event>` to `crates/meme-garden-core/src/world.rs` that swaps `pending_events` with an empty Vec and returns the buffer.
- [ ] T019 Update the existing `world.rs::tests::same_seed_same_metrics` in `crates/meme-garden-core/src/world.rs` to also assert `meme_prevalence_by_kind`, `diversity_shannon`, and `transmissions_this_tick` are bit-identical between paired runs (extended fields, same invariant).
- [ ] T020 Confirm `cargo check --workspace` and `cargo test --workspace` are green. The existing POC behavior may produce different *values* than before (because the shape of `Meme`/`Agent`/`Metrics` changed), but the determinism gate must still hold.

**Checkpoint**: Core types match `data-model.md`. The POC simulation is now wired in
terms of those types, the determinism gate still holds, and US1 work can begin.

---

## Phase 3: User Story 1 — Run the cooperative-vs-selfish milestone experiment (Priority: P1) 🎯 MVP

**Goal**: A researcher can launch a headless run with the cooperation-vs-selfish preset
at three scarcity levels, get `events.jsonl` + `summary.csv` + `config.toml` per run, and
read "did the cooperative meme survive?" directly from the metrics.

**Independent Test**: `cargo run -p meme-garden-cli -- headless --preset
cooperation-vs-selfish-low --seed 42`. The resulting `runs/<id>/summary.csv` last row
shows the cooperative prevalence; the milestone integration test asserts the *direction*
of survival under each scarcity level is stable across re-runs.

### Starter memes & symbolic meme behavior

- [ ] T021 [P] [US1] Create `crates/meme-garden-core/src/starters.rs` defining six `pub fn` constructors for `Meme` — `share_with_allies`, `avoid_strangers`, `copy_high_energy`, `attack_low_energy_outsiders`, `punish_non_sharers`, `prefer_same_meme` — each producing a fully-populated `Meme` with the correct `MemeKind`, `Trigger`, `Target`, `Effect`, and starting `strength`/`transmissibility`/`mutation_rate`/`cognitive_cost`; register the module in `crates/meme-garden-core/src/lib.rs`; export a `StarterMemeName` enum and a `lookup(name: &str) -> Option<fn(&MemeConfig) -> Meme>` resolver used by config validation.
- [ ] T022 [US1] Wire the resolver from T021 into `SimConfig::validate` in `crates/meme-garden-core/src/config.rs` so unknown `memes.seed[*].name` values return `ConfigError::UnknownStarterMeme`.
- [ ] T023 [P] [US1] Create `crates/meme-garden-core/src/policy.rs` implementing `compute_action(agent: &Agent, perception: &Perception, rng: &mut SimRng, cfg: &SimConfig) -> Action`: start from the default-policy action distribution, then for each meme in inventory order whose `trigger` matches the agent's perception, multiplicatively bias the distribution toward the meme's `effect` by `strength`; sample the action from the resulting distribution via `SimRng`. Add the `Perception` struct in this file. Register the module in `crates/meme-garden-core/src/lib.rs`.

### Tick phases

- [ ] T024 [US1] In `crates/meme-garden-core/src/world.rs`, replace the existing two-phase tick with the eight phases from `data-model.md §State transitions per tick` — perception → policy → action execution → transmission → reproduction → aging/death → world maintenance → metrics emission. Each phase is its own private method on `Simulation`. Phase order is part of the determinism contract; do not reorder.
- [ ] T025 [US1] Implement `Simulation::perception_phase` in `crates/meme-garden-core/src/world.rs` building a per-agent transient `Vec<Perception>` (length = `agents.len()`, indexed by `AgentId`). No state changes in this phase.
- [ ] T026 [US1] Implement `Simulation::policy_phase` in `crates/meme-garden-core/src/world.rs` calling `policy::compute_action` for each living agent in `AgentId` order; collect chosen actions into a `Vec<Action>` for the action phase to execute.
- [ ] T027 [US1] Implement `Simulation::action_phase` in `crates/meme-garden-core/src/world.rs` dispatching each action; conflicts (two agents targeting the same victim same tick) resolve in lower-`AgentId`-first order; emit `Event::Death { cause: Combat }` on kill, `Event::Transmission` when an `Action::Transmit` succeeds via the transmission helper from T029.
- [ ] T028 [US1] Implement food consumption and movement inside `Simulation::action_phase` (`crates/meme-garden-core/src/world.rs`) preserving the existing food-seeking bias; the previous `move_and_feed_phase` logic is folded in here.
- [ ] T029 [US1] Implement `Simulation::transmission_phase` in `crates/meme-garden-core/src/world.rs`: for each living agent in `AgentId` order, for each meme in inventory order, roll transmission against eligible neighbors using `meme.transmissibility * recipient_social_copying_bias` (plus `prestige_boost` when applicable); on a successful transmission, call into the mutation operator from T030 with `meme.mutation_rate`; emit `Event::Transmission` and (if applicable) `Event::Mutation`.

### Mutation & lineage

- [ ] T030 [P] [US1] Create `crates/meme-garden-core/src/mutation.rs` implementing `mutate(parent: &Meme, rng: &mut SimRng, cfg: &MutationConfig, lineage: &mut LineageGraph, tick: u64) -> Meme`: enum-swap for `trigger`/`target`/`effect` with probability `cfg.enum_swap_probability` per field; strength jitter in ±`cfg.strength_jitter_max` clamped to `[0, 1]`; allocate a new `LineageId` linked to the parent. Hold `transmissibility` and `mutation_rate` fixed per `research.md D-006`. Register the module in `crates/meme-garden-core/src/lib.rs`.
- [ ] T031 [P] [US1] Add `recombine(a: &Meme, b: &Meme, rng: &mut SimRng, lineage: &mut LineageGraph, tick: u64) -> Meme` to `crates/meme-garden-core/src/mutation.rs` — picks each field independently from one of the two parents and links the new lineage node to both. Recombination is invoked from the reproduction phase only.

### Reproduction, death, world maintenance

- [ ] T032 [US1] Implement `Simulation::reproduction_phase` in `crates/meme-garden-core/src/world.rs`: agents with `energy >= reproduction.energy_threshold` and `age >= reproduction.min_age` adjacent to a compatible partner reproduce; deduct `reproduction.offspring_energy_cost` from each parent; inherit memes per `reproduction.inherit_meme_prob`; roll trait mutation per `agents.trait_mutation_rate`; optionally recombine two random parental memes into one offspring meme; emit `Event::Birth`.
- [ ] T033 [US1] Implement `Simulation::death_phase` in `crates/meme-garden-core/src/world.rs`: subtract metabolism + per-meme cognitive cost; mark `alive=false` and emit `Event::Death { cause }` when energy reaches 0 (`Starvation`) or age exceeds `agents.max_age` (`Aging`).
- [ ] T034 [US1] Implement `Simulation::world_maintenance_phase` in `crates/meme-garden-core/src/world.rs` re-using the existing food regrowth logic; gate it on `food.regrowth_rate > 0` as today.

### Metrics emission

- [ ] T035 [US1] Replace `Simulation::snapshot` with `Simulation::emit_metrics_phase` in `crates/meme-garden-core/src/world.rs`: compute `Metrics` per `data-model.md §6`; emit `Event::Tick` carrying the same data (so JSONL consumers don't need a separate `Metrics` channel); also detect and emit `Event::Extinction` once when population or all memes first reach zero (per `research.md D-008`).
- [ ] T036 [US1] Implement Shannon diversity and top-1 dominance helpers in `crates/meme-garden-core/src/metrics.rs` (`shannon_diversity(prevalence_by_kind: &[f32]) -> f32`, `top1_fraction(prevalence_by_kind: &[f32]) -> f32`); call them from `emit_metrics_phase`.
- [ ] T037 [US1] Add `Simulation::cluster_snapshot` in `crates/meme-garden-core/src/world.rs` computing Jaccard-based cultural clusters per `research.md D-011`; call it from `emit_metrics_phase` every `run.cluster_snapshot_every` ticks and emit `Event::ClusterSnapshot`.

### CLI / IO surface for US1

- [ ] T038 [US1] Create `crates/meme-garden-cli/src/export.rs` with `RunWriter` that owns the per-run output handles: opens `runs/<run-id>/events.jsonl`, `runs/<run-id>/summary.csv`, and writes a copy of the resolved config to `runs/<run-id>/config.toml`; the first JSONL record is the `Event::Header { schema_version: 1, run_id, core_version }` line per `contracts/metrics.schema.md`.
- [ ] T039 [US1] Add `pub fn write_event(&mut self, event: &Event)` and `pub fn write_summary_row(&mut self, m: &Metrics)` to `RunWriter` in `crates/meme-garden-cli/src/export.rs`. Use `serde_json::to_writer` + `writeln!` for JSONL; reuse `Metrics::to_csv_row` for CSV.
- [ ] T040 [US1] Create `crates/meme-garden-cli/src/runner.rs` with a single `pub fn run_to_horizon(config: SimConfig, seed: u64, writer: &mut RunWriter, horizon: u32, observer: Option<&mut dyn Observer>) -> anyhow::Result<()>` used by both headless and TUI; loop calls `sim.step()`, drains events, writes them, calls the optional observer hook. The TUI passes a non-None `Observer`; headless passes `None`.
- [ ] T041 [US1] Refactor `crates/meme-garden-cli/src/main.rs` to use `clap`'s derive API with subcommands `Run` and `Headless` per `contracts/cli.md`. The `Headless` subcommand parses `--config | --preset`, `--seed`, `--ticks`, `--run-id`, instantiates `RunWriter`, and calls `runner::run_to_horizon`. The `Run` subcommand will be wired in Phase 5 (US3); for US1 it can return `unimplemented!()`.
- [ ] T042 [US1] Generate a default `--run-id` of the form `<YYYYMMDD-HHMMSS>-<short-name>` in `crates/meme-garden-cli/src/main.rs` (NOT in core, per `research.md D-009`); error with exit 1 if the resolved run directory already exists.

### Presets

- [ ] T043 [P] [US1] Create `configs/presets/cooperation-vs-selfish-low.toml` per `contracts/config.schema.md`: 50% `share_with_allies` carriers, 50% `attack_low_energy_outsiders` carriers, `scarcity.level = "low"`. Add a `# description:` line at the top.
- [ ] T044 [P] [US1] Create `configs/presets/cooperation-vs-selfish-mid.toml` — identical to T043 except `scarcity.level = "mid"`.
- [ ] T045 [P] [US1] Create `configs/presets/cooperation-vs-selfish-high.toml` — identical to T043 except `scarcity.level = "high"`.
- [ ] T046 [US1] Implement the scarcity-level → food-rate transform per `contracts/config.schema.md` in `crates/meme-garden-core/src/config.rs::SimConfig::apply_scarcity(&mut self)`: `low = 1.0x`, `mid = 0.5x`, `high = 0.2x` against `food.initial_density` and `food.regrowth_rate`; called by the loader; after application, `scarcity.level` is preserved for traceability in the resolved `config.toml` (downstream rules don't read it).

### Tests for User Story 1

- [ ] T047 [P] [US1] Add `crates/meme-garden-core/tests/determinism.rs` — full-simulation determinism: spin two `Simulation` instances with `seed=42`, run for 500 ticks, assert each `Vec<Event>` returned per tick is byte-identical when serialized via `serde_json` (this is the JSONL-equivalence check).
- [ ] T048 [P] [US1] Add `crates/meme-garden-core/tests/transmission.rs` — for each of the six starter memes, seed a single carrier in a deterministic world, run 200 ticks, assert prevalence rises above the initial 1-carrier baseline at least once.
- [ ] T049 [P] [US1] Add `crates/meme-garden-core/tests/mutation.rs` — run 500 ticks with `mutation_rate = 1.0` and assert every emitted `Meme` has `trigger`/`target`/`effect` in their enum ranges, `strength ∈ [0,1]`, and a `lineage_id` resolvable in `LineageGraph` back to a `LineageOrigin::Starter`.
- [ ] T050 [P] [US1] Add `crates/meme-garden-core/tests/lineage.rs` — assert every live meme at horizon traces to a starter; assert `parents.len() ≤ 2`; assert the `LineageGraph` is append-only (no node ever has `parents` mutated after creation).
- [ ] T051 [P] [US1] Add `crates/meme-garden-core/tests/extinction.rs` — force a high-metabolism / no-regrowth config; assert the simulation runs to horizon, emits exactly one `Event::Extinction { scope: Population }`, and subsequent ticks emit `alive=0` records cleanly.
- [ ] T052 [P] [US1] Add `crates/meme-garden-core/tests/cognitive_cost.rs` — assert agents with a meme that has nonzero `cognitive_cost` lose energy strictly faster than identical agents without that meme over a fixed window.
- [ ] T053 [P] [US1] Add `crates/meme-garden-core/tests/milestone.rs` — the load-bearing regression: run the cooperation-vs-selfish preset under `scarcity = low/mid/high` with `seed=42`, assert (a) each run produces a bit-identical `Vec<Event>` stream across a re-run, (b) the *direction* of cooperative-meme survival under each scarcity level matches a recorded baseline (encode the expected boolean per scarcity level in the test; if the direction changes, the test fails and we discuss).
- [ ] T054 [US1] Smoke-test the headless CLI end-to-end: a test (or shell script under `crates/meme-garden-cli/tests/headless.rs`) running `cargo run -p meme-garden-cli -- headless --preset cooperation-vs-selfish-low --seed 42 --ticks 200` produces a `runs/<id>/` directory containing `config.toml`, `events.jsonl` whose first record has `kind: "header"`, and a `summary.csv` whose first column matches the JSONL `tick` field for every row.

### US1 wrap

- [ ] T055 [US1] Update `configs/default.toml` to the new schema so the legacy adapter from T016 is no longer needed in the default path (the adapter stays in code for one release, then is removed in polish).

**Checkpoint**: US1 is fully functional. The milestone question is answerable from
`runs/<id>/summary.csv` and `events.jsonl` for any of the three preset scarcity levels.
**This is the MVP.**

---

## Phase 4: User Story 2 — Sweep parameters to compare memetic strategies (Priority: P2)

**Goal**: A researcher can vary scarcity / mutation rate / transmission probability /
social copying bias / initial meme distribution across multiple runs and compare them
side-by-side using only the emitted artifacts.

**Independent Test**: Run two `headless` invocations differing only in mutation rate
with the same seed; confirm each run's `runs/<id>/config.toml` is byte-identical to the
config the simulator consumed, the two `events.jsonl` files differ, and a third run
with the original config + seed produces a `events.jsonl` byte-identical to the first.

### Config & validation completeness

- [ ] T056 [P] [US2] Wire `SimConfig::validate` (T015) into the CLI entry path in `crates/meme-garden-cli/src/main.rs` so any invalid config aborts before `Simulation::new`; map `ConfigError` to a `clap`-friendly error message.
- [ ] T057 [P] [US2] Add `crates/meme-garden-core/tests/config_validation.rs` covering: out-of-range probabilities are rejected; `initial_traits_dist` that doesn't sum to 1.0 is rejected; unknown starter meme names are rejected with `ConfigError::UnknownStarterMeme`; legacy POC config adapts cleanly with a `tracing::warn!` log.

### Preset / override / id surface

- [ ] T058 [US2] Implement `--preset <name>` resolution in `crates/meme-garden-cli/src/main.rs` (loads `configs/presets/<name>.toml`); `--preset` and `--config` are mutually exclusive (use `clap::ArgGroup`).
- [ ] T059 [US2] Implement `--seed`, `--ticks`, `--run-id` overrides in `crates/meme-garden-cli/src/main.rs`. `--seed` overrides `run.seed` in the resolved config copy (so the resolved `config.toml` written to the run directory matches what actually executed).
- [ ] T060 [P] [US2] Implement `meme-garden list-presets` in `crates/meme-garden-cli/src/main.rs`: lists `configs/presets/*.toml` with the first `# description:` line of each file. Add `crates/meme-garden-cli/tests/list_presets.rs` asserting all three cooperation-vs-selfish presets are listed.

### Export & analyze

- [ ] T061 [P] [US2] Implement `meme-garden export <run-dir> --to <csv|jsonl|summary-md>` in `crates/meme-garden-cli/src/main.rs`: reads `events.jsonl`, rebuilds the `Vec<Metrics>` history, regenerates `summary.csv` (for `--to csv`), or invokes `NoopProvider::summarize` (for `--to summary-md`).
- [ ] T062 [P] [US2] Implement `meme-garden analyze <run-dir>` in `crates/meme-garden-cli/src/main.rs`: same machinery as `export --to summary-md` but writes to stdout. Document the exit code: 0 on success, 1 on missing/corrupt `events.jsonl`.

### Self-describing artifact

- [ ] T063 [US2] In `crates/meme-garden-cli/src/export.rs`, ensure `RunWriter::finalize()` calls `fsync` on `events.jsonl` so a `kill -9`d run leaves a recoverable (if truncated) file. Document the trade-off in a one-line `// why:` comment.
- [ ] T064 [US2] Add a `core_version: &str` constant in `crates/meme-garden-core/src/lib.rs` populated from `env!("CARGO_PKG_VERSION")` and consumed by `RunWriter::write_header()` so the JSONL header line embeds the core version per `contracts/metrics.schema.md`.

### Tests for User Story 2

- [ ] T065 [P] [US2] Add `crates/meme-garden-cli/tests/sweep.rs` — programmatically run three headless simulations with the same seed but `mutation.strength_jitter_max ∈ {0.0, 0.10, 0.30}`; assert (a) all three `events.jsonl` files are well-formed JSONL, (b) the resolved `config.toml`s differ on `mutation.strength_jitter_max` only, (c) running each twice yields byte-identical `events.jsonl`.
- [ ] T066 [P] [US2] Add `crates/meme-garden-cli/tests/transmission_zero.rs` — set `transmission.base_rate = 0.0`, run 200 ticks, assert no `Event::Transmission` records appear in `events.jsonl`.
- [ ] T067 [P] [US2] Add `crates/meme-garden-cli/tests/export_roundtrip.rs` — run a 100-tick headless sim, then `meme-garden export <run-dir> --to csv` re-emits a `summary.csv` byte-identical to the one originally written by the run.

**Checkpoint**: US1 and US2 work independently. A researcher can run, sweep, compare,
and re-emit summaries entirely from CLI artifacts.

---

## Phase 5: User Story 3 — Observe simulation dynamics live (Priority: P3)

**Goal**: A researcher can launch the TUI, watch agents and food move on the grid, see
meme prevalence shift in a side pane, and quit without losing the run artifacts.

**Independent Test**: `cargo run -p meme-garden-cli -- run --seed 42 --preset
cooperation-vs-selfish-mid` opens the TUI; observe ≥100 ticks; quit with `q`; verify
the `runs/<id>/events.jsonl` is byte-identical to the one produced by `headless` with
the same flags.

- [ ] T068 [US3] Define the `Observer` trait in `crates/meme-garden-cli/src/runner.rs` consumed by `run_to_horizon`: `fn on_tick(&mut self, sim: &Simulation, events: &[Event]) -> ObserverDirective` where `ObserverDirective` is `{ Continue, Quit }`. Headless passes `None`; the TUI wires its own implementation.
- [ ] T069 [US3] Refactor `crates/meme-garden-cli/src/app.rs` to be the TUI's `Observer`. It mirrors the latest `Metrics` and a bounded ring buffer of recent per-kind prevalence; processes user input (`q` → `Quit`).
- [ ] T070 [US3] Refactor `crates/meme-garden-cli/src/tui.rs` to render two panes per `quickstart.md §4`: left = world grid (existing rendering, extended for the new agent traits); right = `meme_prevalence_by_kind` as a multi-line sparkline (or stacked bar chart) with one line per `MemeKind`. Use `ratatui::widgets::Sparkline` or `Chart` — whichever ratatui 0.28 supports cleanly.
- [ ] T071 [US3] Wire the TUI through the `Run` subcommand in `crates/meme-garden-cli/src/main.rs` (T041 left it as `unimplemented!()`). The `Run` subcommand instantiates the same `RunWriter` as `headless`, the TUI `Observer`, and calls `runner::run_to_horizon`. On `Quit`, `RunWriter::finalize` runs before the terminal is restored so the artifact is intact.
- [ ] T072 [US3] Add `crates/meme-garden-cli/tests/tui_headless_equivalence.rs` — programmatically construct the TUI `Observer` (without rendering), run `run_to_horizon` for 100 ticks, do the same headlessly, assert the two resulting `events.jsonl` byte streams are identical. This is the FR-026 regression test.

**Checkpoint**: All three user stories complete. The headless and interactive modes
produce identical artifacts for matched seeds, and the TUI visualization sits cleanly
on top of the shared runner.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: AI seam contracts, documentation, lint cleanup, and a perf sanity check.

### AI seams

- [ ] T073 [P] Update `crates/meme-garden-core/src/ai.rs` to the trait signatures in `contracts/ai-seams.md`: extend `MemeNamer::name` with the new `Meme` shape; add the `ExperimentDesigner` trait + `AiError` enum; extend `RunAnalyst::summarize` to take `&LineageGraph` alongside `&[Metrics]`; update `NoopProvider` impls to match.
- [ ] T074 Implement `meme-garden experiment design` and `meme-garden analyze` subcommands in `crates/meme-garden-cli/src/main.rs` wired to `NoopProvider`; `experiment design` exits 2 with `Error: ai provider not configured` per `contracts/cli.md`.

### Documentation

- [ ] T075 [P] Create `docs/meme-grammar.md` describing the symbolic meme grammar: enums (`Trigger`, `TargetSelector`, `Effect`, `MemeKind`), how `Meme.strength` biases policy, why mutation is bounded, and worked examples for each of the six starter memes. Reference `data-model.md` for normative shapes.
- [ ] T076 [P] Update `docs/design.md` "MVP scope" section to point at `specs/001-meme-garden-mvp/` as the executable plan for the MVP; keep the north-star framing intact.
- [ ] T077 [P] Update `CLAUDE.md` "Where things will grow" section to reflect that the symbolic grammar, mutation/lineage, and AI seams now have concrete code paths (replace forward-looking language with present-tense references to the now-existing modules and files).
- [ ] T078 [P] Add a `runs/.gitkeep` (and a one-line `runs/README.md` if desired) so contributors don't get confused by the empty directory after T002 ignores its contents.

### Cleanup & verification

- [ ] T079 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` from repo root; fix every lint surfaced. Lints disabled with `#[allow(...)]` MUST include a one-line `// why:` justification.
- [ ] T080 Remove the legacy-config adapter introduced in T016 from `crates/meme-garden-core/src/config.rs` if `configs/default.toml` (T055) and every preset (T043–T045) parse under the strict schema. Confirm with `cargo test --workspace`.
- [ ] T081 Perf sanity: time a 1,000-tick headless run on the default config (`time cargo run --release -p meme-garden-cli -- headless --ticks 1000 --seed 42`). If wall time exceeds 30 s on a representative developer laptop (SC-004), open a follow-up issue documenting where the budget is going (do not block merge — note the regression).
- [ ] T082 Validate `quickstart.md` end-to-end by walking through every command block in a clean checkout; fix any divergence between the doc and the implemented CLI surface.

**Checkpoint**: Spec quality checklist items all pass against the implemented system;
both the constitution Principle V "metrics-first" check and the headless / interactive
equivalence check have load-bearing tests guarding them.

---

## Dependencies & Execution Order

### Phase dependencies

- **Phase 1 (Setup)**: no dependencies — start immediately.
- **Phase 2 (Foundational)**: depends on Phase 1 completion — **blocks** all user stories.
- **Phase 3 (US1)**: depends on Phase 2 completion. **This phase alone is the MVP.**
- **Phase 4 (US2)**: depends on Phase 3 (uses the same `runner` + `RunWriter`).
- **Phase 5 (US3)**: depends on Phase 3 (uses the same `runner` + `RunWriter`).
- **Phase 6 (Polish)**: depends on US1; some sub-tasks (docs) can land in parallel with
  US2 / US3 once US1 is green.

### User story dependencies

- **US1 (P1)**: depends on Phase 2 only. No other-story dependencies.
- **US2 (P2)**: depends on US1's `runner` and `RunWriter`. The CLI subcommand surface
  added in US2 does not modify simulation logic.
- **US3 (P3)**: depends on US1's `runner`. US2 and US3 are independent of each other
  and could ship in either order, though product priority puts US2 first.

### Within each user story

- Tests in `tests/` directories MAY be written ahead of the matching implementation
  task in the same phase, but at least one matching implementation task MUST land
  before the test is expected to pass.
- Symbolic enums and bounded types (Phase 2) before any logic that consumes them
  (Phase 3).
- Single-meme behavior (T023, T021) before transmission/mutation (T029, T030).
- Mutation operator (T030) before lineage tests (T050) and milestone (T053).
- Headless CLI surface (T038–T042) before US2 export/analyze (T061–T062) before US3 TUI
  wiring (T068–T072).

### Parallel opportunities

- All Phase 1 tasks except T001 + T005 can run in parallel.
- T006, T007, T008, T009, T012, T013 in Phase 2 touch independent files and can run
  in parallel.
- All starter-meme tests T048 and all preset files T043–T045 can run in parallel.
- The four cleanup-and-docs tasks T075, T076, T077, T078 are independent.

---

## Parallel Example: Phase 2 Foundational

```bash
# Independent foundational files — different files, no incomplete-task dependencies:
Task: "T006 Add Trigger, TargetSelector, Effect enums in crates/meme-garden-core/src/meme.rs"
Task: "T007 Add MemeKind variants in crates/meme-garden-core/src/meme.rs"   # serialized with T006 (same file)
Task: "T008 Create crates/meme-garden-core/src/action.rs"
Task: "T009 Create crates/meme-garden-core/src/lineage.rs"
Task: "T012 Extend Metrics in crates/meme-garden-core/src/metrics.rs"
Task: "T013 Add Event enum in crates/meme-garden-core/src/metrics.rs"   # serialized with T012 (same file)
```

T006 and T007 share `meme.rs`, so they cannot truly parallelize despite both being
"foundational enum" work; the same applies to T012/T013 sharing `metrics.rs`. Treat the
[P] markers as guidance, not authority.

## Parallel Example: Phase 3 US1 starter memes + mutation

```bash
# These touch independent files:
Task: "T021 Create crates/meme-garden-core/src/starters.rs"
Task: "T023 Create crates/meme-garden-core/src/policy.rs"
Task: "T030 Create crates/meme-garden-core/src/mutation.rs"

# Once T021/T023/T030 land, the tests parallelize:
Task: "T048 crates/meme-garden-core/tests/transmission.rs"
Task: "T049 crates/meme-garden-core/tests/mutation.rs"
Task: "T050 crates/meme-garden-core/tests/lineage.rs"
Task: "T052 crates/meme-garden-core/tests/cognitive_cost.rs"
```

---

## Implementation Strategy

### MVP first (US1 only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational — keep the existing determinism test green
   throughout.
3. Complete Phase 3: US1, ending at T055.
4. **Stop and validate**: run the milestone regression test (`cargo test -p
   meme-garden-core --test milestone`). The cooperation-vs-selfish question is
   answerable. Demo-ready.

### Incremental delivery

1. Setup + Foundational → foundation ready.
2. US1 → MVP shipped, milestone answerable, demo-ready.
3. US2 → sweep + export ergonomics for sustained research use.
4. US3 → live TUI for intuition-building.
5. Polish → AI seam contracts, docs, perf sanity.

### Parallel team strategy

With multiple contributors, once Phase 2 is green:

- Developer A: Phase 3 (US1) — owns the simulation loop and milestone test.
- Developer B: prepares Phase 4 (US2) CLI surface against US1's `runner` API once
  the API is stabilized.
- Developer C: prepares Phase 5 (US3) TUI integration against the same `runner` API.

---

## Notes

- `[P]` tasks have no incomplete-task dependencies and touch independent files.
- `[US1] / [US2] / [US3]` map each user-story task back to its `spec.md` user story
  for traceability.
- Tests are part of the deliverable for this MVP per the user's explicit request and
  per the constitution's Principle V — "every behavioral claim must be answerable from
  the metrics stream."
- Constitution Principle I (determinism) is treated as a binding gate, not advisory:
  T019, T047, T053, T065, and T072 all assert byte- or event-stream-identical behavior
  under fixed seeds. Any divergence is a critical defect.
- Phase 2's refactor will temporarily reshape `Metrics` / `Agent` / `Meme`; existing
  POC values WILL change, but the bit-identical-under-same-seed gate continues to hold.
- The legacy-config adapter (T016, removed in T080) exists only to keep the workspace
  green during the Phase 2 transition; do not depend on it from new code.

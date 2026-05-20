# Phase 0 — Research & Decision Log

**Feature**: Meme Garden MVP — Memetic Petri Dish
**Branch**: `001-meme-garden-mvp`
**Date**: 2026-05-19

No `[NEEDS CLARIFICATION]` markers entered Phase 0 from `spec.md`. This file records
the non-obvious decisions made during planning and the alternatives considered, so a
later reviewer can see *why* each choice landed where it did.

---

## D-001 — Metrics output format: JSON-Lines (primary) + CSV summary

**Decision**: Emit per-tick and per-event records as JSON-Lines
(`runs/<run-id>/events.jsonl`), one record per line, with a discriminated
`{"kind": "...", ...}` envelope. In addition, write a flat `summary.csv` of the per-tick
aggregate metrics for spreadsheet inspection.

**Rationale**:

- Events carry heterogeneous shapes (per-tick aggregate vs mutation event vs transmission
  event vs extinction event vs cluster snapshot). JSON-Lines handles a mixed-shape
  stream natively; CSV would require either parallel files per shape, or a flat schema
  with many always-null columns.
- JSON-Lines stays line-oriented, so `wc -l`, `jq -c`, `grep`, and standard log tooling
  all work. It loads cleanly into pandas / polars / DuckDB for offline analysis.
- A flat CSV summary still exists for the common "open it in a spreadsheet" reflex.
- Adds exactly one workspace dep: `serde_json`.

**Alternatives considered**:

- *CSV only*: Cheap to write, but mutation/transmission/extinction events do not fit
  a single column schema without wide null bloat. Rejected.
- *Binary format (bincode / Parquet)*: Faster + smaller, but harder to inspect with
  unix tools and adds non-trivial deps. Premature for MVP scale.
- *SQLite database*: Powerful for analysis but adds a real runtime dep, conflicts with
  the "plain files" assumption in `spec.md`. Out of MVP scope.

---

## D-002 — Visualization: ratatui TUI (already present), no web UI in MVP

**Decision**: Keep the existing ratatui-based TUI as the interactive view. Add a
right-side pane for meme prevalence over time. Do not introduce a web UI.

**Rationale**:

- The TUI already exists and the constitution's "Pure Core, Impure Edges" principle
  has been validated against it.
- A web UI would require an HTTP server, a JS bundle, and a transport for live state —
  all out of scope for an MVP whose primary deliverable is *the metrics stream*, not the
  visualization.
- The TUI can grow a lineage-tree pane later without changing the simulation contract.

**Alternatives considered**:

- *Web UI (Axum + WS + small SPA)*: Better visuals, but a multi-week side quest. Out
  of scope.
- *Static PNG dashboards generated post-run*: Useful as a separate post-run tool; not a
  substitute for live observation during a run.

---

## D-003 — Crate layout: extend existing two crates, defer `meme-garden-ai`

**Decision**: Keep the workspace at two crates (`meme-garden-core`, `meme-garden-cli`)
for the MVP. AI trait surfaces and `NoopProvider` stay in `core::ai`. The future
`meme-garden-ai` crate that will host live LLM providers is sketched in the plan but not
created yet.

**Rationale**:

- The constitution already mandates that real AI deps live outside `core`, but for an
  MVP that ships only `NoopProvider`, creating a third crate is structure for its own
  sake.
- Adding a third crate later is a clean, single-commit refactor once a real provider
  appears.

**Alternatives considered**:

- *Create `meme-garden-ai` now with only `NoopProvider`*: All cost, no benefit until a
  real provider arrives.
- *Inline `NoopProvider` into `meme-garden-cli`*: Wrong direction; bindings then point
  the wrong way (`cli → core::ai` becomes `core ← cli::ai`).

---

## D-004 — Config schema: extend the existing TOML rather than introduce JSON

**Decision**: Stay on TOML. Extend `SimConfig` with new sub-configs for symbolic-meme
parameters, scarcity, mutation, transmission, social copying bias, reproduction, attack,
and initial meme population. Presets ship under `configs/presets/*.toml`.

**Rationale**:

- TOML is already the format. Switching now would invalidate `configs/default.toml`
  with no upside.
- TOML's section-based structure maps cleanly to grouped sub-configs (world, agents,
  food, scarcity, mutation, transmission, reproduction, memes, run).
- The user input lists "JSON or TOML" — TOML matches the existing investment.

**Alternatives considered**:

- *JSON*: More universal, but harder to hand-edit (no comments).
- *YAML*: Familiar but adds a non-trivial parser dep and indentation-sensitivity bugs.

---

## D-005 — Meme inventory data structure: `Vec<Meme>` ordered by `MemeId`

**Decision**: Each `Agent` carries `inventory: Vec<Meme>` where memes are appended on
acquisition. Iteration over an agent's inventory is in insertion order. Cognitive cost
caps the inventory at a per-config maximum; on overflow, the oldest meme is dropped
(deterministically) and an `Event::MemeForgotten` is emitted.

**Rationale**:

- `Vec` is allocation-bounded by the per-config inventory cap; iteration is stable and
  deterministic. No `HashMap` iteration, satisfying Principle III.
- Insertion order is a stable input to the policy resolution algorithm. The same agent
  in the same state under the same RNG MUST resolve to the same action.

**Alternatives considered**:

- *`BTreeMap<MemeId, Meme>`*: Ordered iteration but the map's overhead is unjustified
  at MVP inventory size (≤ ~16 memes per agent).
- *Indexed slab*: Performance gain at large inventory size; over-engineered for MVP.

---

## D-006 — Mutation operator surface: enum-bounded edits + bounded numeric jitter

**Decision**: Mutation operates on exactly four fields — `trigger`, `target`, `effect`,
`strength`. Enum-valued fields swap to another enum variant drawn uniformly at random
(via `SimRng`). The scalar `strength` jitters within `[-0.1, +0.1]` and is clamped to
`[0.0, 1.0]`. `transmissibility` and `mutation_rate` do not themselves mutate in the
MVP (they could in a later iteration; deferring keeps the search space smaller and the
milestone interpretable).

**Rationale**:

- Bounded mutation guarantees mutated memes remain valid symbolic structures
  (Principle IV / FR-016).
- Holding `transmissibility` and `mutation_rate` fixed makes the milestone experiment
  easier to read: changes to prevalence come from policy effects, not from meme-level
  drift.
- A clean expansion path: a later iteration can opt these fields into the mutation
  surface without changing call sites.

**Alternatives considered**:

- *Mutate every field*: Bigger search space, harder to attribute outcomes to anything.
- *Discrete strength steps only*: Equivalent in expressivity at MVP scale; jitter is
  simpler to reason about.

---

## D-007 — Initial AgentTrait surface: minimal MVP set

**Decision**: `AgentTrait` is an enum starting with `{Generous, Cautious, Aggressive,
Conformist}`. Traits are inherited at birth with a small mutation rate, independent of
meme mutation. Traits gate certain default-policy probabilities but do not directly
implement memes. Adding new traits is a single enum-variant extension.

**Rationale**:

- Spec lists "traits" as a key field but does not constrain the set. A minimal set
  keeps the policy resolution algorithm tractable to spec and test.
- Keeping traits separate from memes preserves the distinction that the project cares
  about: memes are transmissible, traits are inherited.

**Alternatives considered**:

- *No traits in MVP*: Spec explicitly names them; dropping them would violate FR-002.
- *Many traits*: Larger combinatorial surface, harder to interpret experiments.

---

## D-008 — Determinism boundary on extinction & extinction-tail behavior

**Decision**: When agent or meme extinction occurs, the simulation continues ticking
through the configured horizon. Extinction events are recorded once with their tick
number; subsequent ticks still emit per-tick aggregate metrics (with zeroed
prevalence). The simulation only short-circuits if explicitly configured with
`run.stop_on_extinction = true` (default `false`).

**Rationale**:

- Spec edge cases require both "total extinction" and "extinction tail with living
  agents" to be observable. Continuing the simulation lets metrics readers see the
  post-extinction tail consistently.
- The `stop_on_extinction` switch keeps CI / sweep jobs short when the tail isn't
  interesting.

**Alternatives considered**:

- *Always stop on extinction*: Hides the post-extinction tail; bad for analysis.
- *Always run to horizon*: Wastes CPU on sweeps that don't care about the tail.

---

## D-009 — Run identifier scheme: `<YYYYMMDD-HHMMSS>-<short-name>`

**Decision**: Each run writes to `runs/<YYYYMMDD-HHMMSS>-<short-name>/`. The short-name
defaults to the config file stem (e.g. `cooperation-vs-selfish-low`). The timestamp is
generated by **the CLI** (not core) at run start.

**Rationale**:

- Time-prefixed directories sort chronologically and avoid collisions across sweep
  runs.
- Generating the timestamp in the CLI (not core) keeps core nondeterministic-source-free
  per Principle I — the run-dir name does not enter the metrics stream itself.

**Alternatives considered**:

- *Hash of (config, seed)*: Reproducible but unfriendly to humans.
- *Sequential N*: Requires scanning the directory; conflicts under parallel runs.

---

## D-010 — Headless determinism vs interactive mode

**Decision**: Both modes call into the same `runner` module in
`meme-garden-cli::runner`, which owns the `Simulation` instance. The TUI is a
visualization-only observer — it reads `Simulation::tick`/`Metrics`/`Event` but does
not advance the simulation independently. Pausing the TUI pauses time-in-simulation,
which means TUI tempo cannot affect the metrics stream.

**Rationale**:

- Required by FR-026 (interactive and headless produce equivalent metrics for the same
  seed).
- Single source of truth for the tick loop minimizes the risk of drift between modes.

**Alternatives considered**:

- *Separate tick loops in TUI and headless*: Maintenance burden + divergence risk.

---

## D-011 — Cluster (cultural cluster) definition

**Decision**: A "cultural cluster" is a set of agents whose pairwise meme-inventory
Jaccard similarity exceeds a configured threshold (default `0.6`). Cluster snapshots
are emitted every N ticks (default 50), not every tick — to bound metrics volume.

**Rationale**:

- Spec calls for "cultural clusters" without defining them. Jaccard over inventories is
  the simplest definition that respects the symbolic-meme model. Threshold is exposed
  in config so the experiment can probe it.
- Emitting every 50 ticks keeps the JSONL file from being dominated by cluster snapshots
  on long runs.

**Alternatives considered**:

- *Connected components on a meme-similarity graph*: Richer but expensive at large
  populations; can ship later.
- *Per-tick emission*: Cheap but bloats the metrics file.

---

## D-012 — Test taxonomy

**Decision**: Three categories of tests:

1. **Unit tests** alongside source in `#[cfg(test)]` modules — cover individual
   functions (mutation operator stays bounded, transmission roll is deterministic,
   etc.).
2. **Integration tests** in `crates/meme-garden-core/tests/` — exercise full
   `Simulation` runs and assert behavioral invariants (cooperative meme can transmit
   under default conditions; mutation rate of 0 yields flat lineage; etc.).
3. **Milestone regression test** (`tests/milestone.rs`) — runs the
   cooperation-vs-selfish preset under low/mid/high scarcity with fixed seeds; asserts
   the *direction* of survival (cooperative meme survives at low scarcity, fails at
   high scarcity, etc.) is stable across runs. This test is the project's canary for
   "did we accidentally change the simulation's behavior?"

**Rationale**:

- The milestone test is the test that the constitution's Principle V points at:
  metrics-first means the regression bar is "the experiment still produces the same
  answer," not "every function does what it did before."

**Alternatives considered**:

- *Property-based tests with `proptest`*: Valuable later (especially for mutation
  operator invariants); not blocking MVP.

---

## Open follow-ups (post-MVP)

These are explicitly **out of scope** for the MVP but logged so they aren't lost:

- LLM-backed implementations of the three AI seams (live in a future `meme-garden-ai`
  crate per constitution).
- Lineage-tree visualization pane in the TUI.
- Web UI replacement for the TUI.
- Mutation of `transmissibility` and `mutation_rate` themselves.
- Connected-component cultural clusters.
- Property-based fuzzing of the mutation operator.
- Persistent run history with indexing for multi-run comparison.

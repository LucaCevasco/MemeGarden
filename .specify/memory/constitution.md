<!--
Sync Impact Report
==================
Version change: (template, unfilled) → 1.0.0
Bump rationale: Initial ratification. The prior file was the unfilled template; this is the first concrete constitution, so MAJOR=1.

Principles defined (initial set, 5 total):
- I. Determinism Is Sacred (NON-NEGOTIABLE)
- II. Pure Core, Impure Edges
- III. Stable Iteration Order
- IV. Symbolic Memes, Not Black Boxes
- V. Metrics-First Experimentation

Added sections:
- Technical Constraints
- Development Workflow
- Governance

Removed sections: none (template placeholders replaced wholesale).

Templates / docs reviewed for consistency:
- ✅ .specify/templates/plan-template.md — "Constitution Check" gate present; principles above map cleanly to gates (determinism, no core I/O, symbolic-meme bounds, metric coverage). No edits required.
- ✅ .specify/templates/spec-template.md — Generic; no principle-specific edits required.
- ✅ .specify/templates/tasks-template.md — Generic phase structure; no edits required (project-specific tasks will be inserted per feature).
- ✅ .specify/templates/checklist-template.md — Not inspected line-by-line; generic and principle-agnostic.
- ✅ CLAUDE.md — Already encodes the same invariants; this constitution makes them governing rather than advisory. No edit required, but CLAUDE.md remains the runtime guidance file referenced from §Governance.
- ✅ docs/design.md — Compatible; north-star question drives §V.

Deferred / TODO:
- None. Ratification date set to today (2026-05-19) per current-date context.
-->

# Meme Garden Constitution

## Core Principles

### I. Determinism Is Sacred (NON-NEGOTIABLE)

Every simulation run MUST be bit-identically reproducible from `(seed, config)` alone.
All randomness inside `meme-garden-core` MUST flow through `rng::SimRng`. The core MUST
NOT call `rand::thread_rng()`, `std::time::*`, `SystemTime::now`, process IDs,
environment variables, or any other ambient nondeterministic source. The regression
test `world.rs::tests::same_seed_same_metrics` is the canonical gate; any change that
breaks it is by default rejected, and may only land with an explicit constitution
amendment recording the new determinism contract.

**Rationale**: The whole research value of Meme Garden — comparing meme dynamics across
parameter sweeps — collapses the moment a run cannot be replayed. Determinism is the
substrate that makes every other claim falsifiable.

### II. Pure Core, Impure Edges

`meme-garden-core` MUST be free of terminal, network, filesystem-write, and stdout
concerns. Config parsing is the only file I/O permitted in core. TUI, CLI argument
handling, logging sinks, and any future HTTP or vendor SDK live in
`meme-garden-cli` or in dedicated downstream crates (e.g. a future
`meme-garden-ai`). Downstream crates MAY depend on core; core MUST NOT depend on
them. Errors at the core library boundary MUST use `thiserror`; binaries MAY use
`anyhow`.

**Rationale**: Keeping the simulator embeddable, testable, and free of side effects
is what lets us swap visualizations, run headless sweeps, and later attach LLM-based
analysts without polluting the hot loop.

### III. Stable Iteration Order

Agents MUST be processed in `AgentId` order. Any per-tick traversal of agents, memes,
or other simulation state MUST use a deterministically ordered container, OR sort
before iterating. `HashMap`-style iteration over simulation state in the hot path is
prohibited. New collections introduced into the hot path MUST document their ordering
guarantee.

**Rationale**: Iteration order is a hidden input to a deterministic simulation;
violating it silently breaks Principle I in ways that only surface when seeds, host
architectures, or hash seeds change.

### IV. Symbolic Memes, Not Black Boxes

Memes MUST remain bounded symbolic structures of the form
`{ trigger, effect, target, strength, transmissibility, mutation_rate }`. The hot
loop MUST NOT execute arbitrary code, evaluate natural-language strings, or call
LLM/HTTP services. AI providers (e.g. `MemeNamer`, `RunAnalyst`) are seams that
operate outside the tick loop — typically pre-config or post-run — and MUST be plugged
in via traits with a deterministic default (`NoopProvider` or equivalent). When
`MemeKind`-like enums are extended, callers MUST be made exhaustive so the compiler
flags every transmission/mutation site.

**Rationale**: A bounded grammar is what makes the system measurable, debuggable,
and cheap to sweep. Free-form beliefs would destroy reproducibility and turn the
sim into a model-evaluation exercise rather than a memetics experiment.

### V. Metrics-First Experimentation

Every behavioral or mechanical change MUST be justifiable by a metric the simulation
already emits, OR ship the new metric alongside it. Claims of the form "cooperation
won," "the punishment meme stabilized the group," or "scarcity favored selfish
memes" MUST be answerable from the metrics stream — not from eyeballing the TUI.
Features whose only motivation is visual polish or unmeasured hypothesis are
out of scope until a measurable question demands them. The current north-star
question — *can a cooperative meme survive against a selfish meme under different
levels of scarcity, mutation, and social copying?* — is the default tiebreaker
for prioritization.

**Rationale**: Meme Garden is a research tool. Features that cannot be evaluated
by metrics accumulate as decoration; metrics-first keeps the project honest about
what it is actually learning.

## Technical Constraints

- **Language & toolchain**: Rust 2021 edition, stable toolchain pinned via
  `rust-toolchain.toml`. No nightly-only features without an amendment.
- **Workspace shape**: Two crates today (`meme-garden-core`, `meme-garden-cli`).
  New concerns get new crates rather than expanding the core. AI integrations,
  alternative visualizations, and persistence layers MUST land as separate crates.
- **Error surfaces**: `thiserror` at library boundaries; `anyhow` permitted in
  binaries only.
- **Hot-loop budget**: The per-tick path MUST avoid allocations in tight loops
  where reasonable, MUST NOT perform I/O, and MUST NOT spawn threads with
  nondeterministic join order.
- **Comments**: Code comments are reserved for *why* — surprising invariants,
  performance traps, deliberate simplifications. Comments narrating *what*
  well-named code already does MUST be removed on sight.
- **No premature compat shims**: Until a feature has external consumers, deprecate
  by deletion. No `// removed` placeholders, no re-exports kept "just in case."

## Development Workflow

- **Local gates**: `cargo check --workspace` and `cargo test --workspace` MUST pass
  before any PR is opened. The determinism regression test is part of
  `cargo test --workspace` and is not optional.
- **Spec-Kit flow**: Non-trivial work follows the Spec-Kit flow
  (`/speckit-specify` → `/speckit-clarify` → `/speckit-plan` → `/speckit-tasks` →
  `/speckit-implement`). Trivial fixes (typos, comment removal, single-call-site
  refactors) MAY skip the flow.
- **PR scope**: Small, single-concept PRs are the default. PRs that touch the
  simulation core AND the CLI AND a new crate should be split unless the split
  would break determinism guarantees.
- **Constitution check in plans**: The `Constitution Check` section of
  `.specify/templates/plan-template.md` MUST evaluate each principle above as
  a pass/justify gate before Phase 0 and after Phase 1.
- **Complexity justification**: Any violation of these principles is allowed only
  via the Complexity Tracking table of the relevant plan, with an explicit
  "simpler alternative rejected because…" entry.

## Governance

This constitution supersedes ad-hoc conventions, PR descriptions, and informal
preferences. When `CLAUDE.md`, `docs/design.md`, or any template conflicts with
this document, this document wins, and the conflicting file MUST be updated in
the same change.

**Amendment procedure**:

1. Open a PR that edits `.specify/memory/constitution.md`.
2. Update the Sync Impact Report at the top of this file.
3. Bump the version per semantic rules below.
4. Update or annotate any dependent templates and runtime guidance files
   (`CLAUDE.md`, plan/spec/tasks templates) in the same PR.
5. Merge only after a human reviewer has confirmed the bump rationale and the
   propagation list.

**Versioning policy**:

- **MAJOR** — Removal or backward-incompatible redefinition of a principle, or
  governance changes that alter the amendment procedure itself.
- **MINOR** — Addition of a new principle or section, or material expansion of
  an existing principle's scope.
- **PATCH** — Clarifications, wording, typo fixes, non-semantic refinements.

**Compliance review**: Every PR description SHOULD note which principles the
change touches, even if only to assert "no constitutional impact." Reviewers
MUST reject changes that silently weaken Principles I–IV.

**Runtime guidance**: `CLAUDE.md` is the authoritative runtime guidance file for
agents and contributors working day-to-day in the repo; this constitution is the
authoritative source of *binding rules*. CLAUDE.md is expected to mirror the
spirit of the principles here, but if the two disagree, this file governs.

**Version**: 1.0.0 | **Ratified**: 2026-05-19 | **Last Amended**: 2026-05-19

# Feature Specification: Meme Garden MVP — Memetic Petri Dish

**Feature Branch**: `001-meme-garden-mvp`

**Created**: 2026-05-19

**Status**: Draft

**Input**: User description: "Build Meme Garden: a controlled memetic petri dish where simple agents
live in a grid world and symbolic memes spread, mutate, compete, and affect behavior… First
milestone: can a cooperative meme survive against a selfish meme under different levels of
scarcity, mutation, and social copying? MVP must support grid world, agent lifecycle, local
social interaction, symbolic meme transmission with mutation and lineage, starter memes,
the core action set, a rich metrics surface, and a visualization. Memes must be real
simulation objects with bounded symbolic grammar — no free-form natural language in the
hot loop. AI seams (experiment designer / meme namer / post-run analyst) live around the
simulation, never inside it. Prioritize observability, reproducibility, experimentation."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run the cooperative-vs-selfish milestone experiment (Priority: P1)

A researcher seeds a world with both a cooperative meme (e.g., "share with allies") and a
selfish meme (e.g., "hoard food") at equal initial prevalence, picks a scarcity level, runs the
simulation, and reads off from the emitted metrics whether the cooperative meme survived, was
outcompeted, or co-existed.

**Why this priority**: This is the north-star question the project exists to answer. Delivering
just this story is a viable MVP — every other story expands its reach. Without it, the rest of
the system has no destination.

**Independent Test**: Run a single simulation with a fixed seed, two starter memes (one
cooperative, one selfish), and default parameters; inspect the metrics stream and determine
"did the cooperative meme survive?" using only the data emitted by the run.

**Acceptance Scenarios**:

1. **Given** a world seeded with the cooperative meme at 50% and the selfish meme at 50% under
   low-scarcity defaults, **When** the simulation runs to its configured horizon, **Then** the
   metrics stream reports an unambiguous end-state prevalence for both memes and the
   "survived / did not survive" determination can be read directly from the metrics without
   manual interpretation.
2. **Given** the same seed and configuration, **When** the run is repeated, **Then** the
   emitted metrics stream is bit-identical to the previous run.
3. **Given** scarcity is increased to a high level with all other inputs held constant, **When**
   the run completes, **Then** the metrics show whether the cooperative meme's end-state
   survival changed direction relative to the low-scarcity baseline.

---

### User Story 2 - Sweep parameters to compare memetic strategies (Priority: P2)

A researcher varies scarcity, mutation rate, transmission probability, social copying bias, and
the initial meme population across multiple runs and compares each run's outcome to learn which
conditions favor cooperative vs selfish memes.

**Why this priority**: A single run answers a single question; parameter sweeps are where
insight lives. Not viable without US1, so P2.

**Independent Test**: Run two simulations differing only in one parameter (e.g., scarcity low
vs high) with all other inputs and the seed fixed; confirm each run's metrics stream is
self-describing (carries its own config) and the two outcomes are comparable side-by-side
without re-deriving any configuration from memory.

**Acceptance Scenarios**:

1. **Given** a baseline configuration, **When** the user changes one parameter (e.g., mutation
   rate from 0.01 to 0.10) and re-runs, **Then** each run's metrics stream is labeled with its
   exact configuration and the two are independently inspectable.
2. **Given** mutation rate is non-zero, **When** the run completes, **Then** the metrics show
   at least one mutation event during the run and the lineage of any mutated meme can be traced
   back to a starter meme through the recorded lineage data.
3. **Given** transmission probability is set to 0, **When** the run completes, **Then** no
   transmission events are recorded and no agent ends with a meme it did not start with (except
   via mutation of an inherited meme).

---

### User Story 3 - Observe simulation dynamics live (Priority: P3)

A researcher watches a simulation tick-by-tick to build intuition about agent behavior and meme
dynamics — seeing the world grid, agents moving and interacting, food appearing and being
consumed, and meme prevalence shifting over time.

**Why this priority**: Visualization builds intuition but does not produce the research result.
The headless metrics path is the source of truth; visualization is a tool for the researcher's
brain. Useful, but not the deliverable.

**Independent Test**: Launch the interactive run, observe at least 100 ticks elapse on screen,
and confirm that the displayed agent count, world state, and at least one meme-prevalence
indicator update in step with the underlying simulation.

**Acceptance Scenarios**:

1. **Given** an interactive run, **When** the simulation is ticking, **Then** the user sees a
   2D representation of the grid showing agent and food positions updating each tick.
2. **Given** an interactive run with multiple memes active, **When** prevalence shifts during
   the run, **Then** the visualization conveys the relative prevalence of each meme clearly
   enough that the user can perceive dominance, co-existence, or near-extinction trends.
3. **Given** the user prefers throughput over visualization, **When** the user launches the
   headless mode, **Then** the simulation runs without any visualization and emits the metrics
   stream to a file or stdout suitable for offline analysis.

---

### Edge Cases

- **Total population extinction**: All agents die before the configured horizon. The system MUST
  emit final metrics, mark the run as "population extinct at tick N," and stop cleanly rather
  than hang or crash.
- **Total meme extinction with living agents**: All memes go extinct while agents remain alive.
  The system MUST continue ticking until the horizon, recording the extinction event, and emit
  metrics covering the post-extinction tail.
- **Mutation rate = 0**: No mutations occur; lineage trees remain flat. Run still completes and
  the lineage record still emits a (degenerate) structure.
- **Mutation rate = 1**: Every transmission mutates. The system MUST not crash; lineage storage
  MUST remain bounded as the number of distinct meme variants grows.
- **Single meme reaches 100% dominance**: Recorded as a dominance event in metrics; simulation
  continues through to the horizon rather than terminating early.
- **Zero starting memes**: Agents survive on their default policy; the system MUST still emit
  metrics, MUST not divide by zero in prevalence calculations, and MUST record "no memes at any
  tick" cleanly.
- **Carrier dies in the same tick it would have transmitted a meme**: The transmission MUST
  resolve deterministically with one well-defined outcome (no race-dependent behavior).
- **Determinism breach**: If two runs with the same seed + configuration ever diverge, the
  system MUST treat that as a critical defect — not as variance to be averaged away.

## Requirements *(mandatory)*

### Functional Requirements

**World and agent lifecycle**

- **FR-001**: The system MUST simulate a 2D grid world populated by agents and food, with grid
  dimensions, food spawn rate, and initial agent population configurable per run.
- **FR-002**: Each agent MUST carry at minimum: energy, age, position, traits, memory, a trust
  map keyed by other agent identity, a meme inventory, and a current policy derived from its
  memes.
- **FR-003**: Agents MUST be able to perform the following actions and no others in the MVP:
  move, eat, share, attack, imitate, transmit (a meme), and reproduce.
- **FR-004**: Agents MUST lose energy through aging and action costs, and MUST die when energy
  reaches zero or when age exceeds a configured maximum.
- **FR-005**: Agents MUST be able to reproduce when energy and proximity preconditions are met;
  offspring MUST inherit some subset of the parent's memes, with inherited memes subject to the
  configured mutation rate at inheritance time.

**Memes as bounded behavioral rules**

- **FR-006**: Each meme MUST be a bounded symbolic structure containing trigger, target, effect,
  strength, transmissibility, mutation rate, cognitive cost, and lineage identifier.
- **FR-007**: Triggers MUST be drawn from a fixed, enumerated set defined at config time
  (e.g., Hungry, NearFood, NearAlly, NearStranger, AttackedRecently, SawAgentGainEnergy). No
  triggers outside this set may appear in any meme during the run.
- **FR-008**: Effects MUST be drawn from a fixed, enumerated set defined at config time (e.g.,
  MoveToward, MoveAway, Share, Attack, Imitate, RefuseInteraction, TransmitMeme, IncreaseTrust,
  DecreaseTrust). No effects outside this set may appear in any meme.
- **FR-009**: Targets MUST be drawn from a fixed, enumerated set defined at config time (e.g.,
  Self, Kin, Ally, Stranger, HighEnergyAgent, LowEnergyAgent). No targets outside this set may
  appear in any meme.
- **FR-010**: Memes MUST influence agent decisions in observable, measurable ways. A "share
  with allies" meme MUST measurably raise the probability of food-sharing actions toward
  low-energy allied agents relative to an identical agent without that meme; analogous claims
  MUST hold for every starter meme.
- **FR-011**: The system MUST ship at least these six starter memes by default: share with
  allies, avoid strangers, copy high-energy agents, attack low-energy outsiders, punish
  non-sharers, prefer agents with the same meme.
- **FR-012**: Memes MUST be the only mechanism by which an agent's decision policy differs from
  the default policy. There MUST be no free-form natural-language belief, no user-supplied
  script, and no arbitrary code path in the per-tick decision logic.

**Transmission, mutation, recombination, lineage**

- **FR-013**: Memes MUST transmit between locally-interacting agents with a probability driven
  by the meme's transmissibility, the receiving agent's social copying bias, and contextual
  factors (e.g., prestige of the carrier as measured by energy or kills).
- **FR-014**: On transmission, a meme MAY mutate per its configured mutation rate, producing a
  variant with a related-but-modified trigger, target, effect, or strength while preserving a
  lineage link to its parent.
- **FR-015**: The system MUST track meme lineage such that any meme alive at any tick can be
  traced back through its ancestors to a founding starter meme.
- **FR-016**: Mutation MUST be bounded: any mutated meme MUST remain a valid symbolic structure
  drawn entirely from the fixed enumerations of triggers, targets, and effects. There MUST be
  no unbounded growth in the representation size of an individual meme.
- **FR-017**: Recombination of two memes carried by the same agent MAY occur per configured
  rules; any recombined meme MUST also be a valid bounded meme with lineage links to both
  parents.

**Configuration and reproducibility**

- **FR-018**: Every run MUST be parameterized by an explicit configuration object including at
  minimum: random seed, grid size, initial population, food spawn rate, scarcity level,
  mutation rate, transmission probability, social copying bias, run horizon (max ticks), and
  initial meme population.
- **FR-019**: Two runs with the same seed and configuration MUST produce a bit-identical
  metrics stream.
- **FR-020**: The configuration MUST be readable from a human-editable file and MUST also be
  embedded into every metrics stream output so that any metrics artifact is self-describing.

**Metrics and observability**

- **FR-021**: The system MUST emit a metrics stream including at minimum: meme prevalence over
  time, mutation events with parent/child lineage links, transmission rate per meme, host
  fitness per meme (mean carrier lifespan, reproduction count), group fitness per meme cluster,
  meme diversity, meme dominance, extinction events with tick number, and cultural clusters.
- **FR-022**: The metrics stream MUST be readable in a form suitable for offline analysis
  (per-tick or per-event records) independently of any visualization layer, and independently
  of whether the run was launched interactively or headlessly.
- **FR-023**: The metrics stream MUST be sufficient on its own to answer the milestone
  question: did the cooperative meme survive to horizon, and what was its end-state prevalence
  relative to the selfish meme? No additional manual instrumentation may be required.

**Execution modes**

- **FR-024**: The system MUST provide an interactive mode that visualizes the world grid and
  at least one meme dynamics view during a run.
- **FR-025**: The system MUST provide a headless mode that runs without visualization and emits
  metrics suitable for batch analysis.
- **FR-026**: For the same seed and configuration, interactive and headless modes MUST produce
  equivalent metric streams. Visualization MUST NOT influence simulation outcomes.

**AI seams (around the core, not inside)**

- **FR-027**: The system MUST expose seams for AI-assisted tooling — an experiment designer
  that maps natural language to a configuration object, a meme namer that produces
  human-readable labels for symbolic memes, and a post-run analyst that summarizes a metrics
  history — such that any of these tools can be added later without changes to the per-tick
  simulation logic.
- **FR-028**: AI-assisted tools MUST NOT execute inside the per-tick decision path of the
  simulation. The hot loop remains symbolic, bounded, and deterministic.

### Key Entities

- **World**: The 2D grid environment containing agents and food. Carries tick number, food
  distribution, and overall population.
- **Agent**: A simulated entity with energy, age, position, traits, memory, trust relationships,
  meme inventory, and a current policy derived from its memes.
- **Meme**: A bounded symbolic behavioral rule — trigger, target, effect, strength,
  transmissibility, mutation rate, cognitive cost, and lineage identifier.
- **Lineage**: The ancestral graph of memes. Every meme links to its parent(s), making
  historical descent fully reconstructible.
- **Run Configuration**: The set of inputs (seed, grid, population, scarcity, mutation rate,
  transmission probability, social copying bias, horizon, initial meme population) that defines
  an experiment.
- **Metrics Record**: A per-tick or per-event entry in the metrics stream capturing observable
  state of memes, agents, and the population.
- **AI Seam**: A pluggable, deterministic-by-default interface used outside the per-tick loop
  for naming, designing, or analyzing experiments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A researcher can answer the cooperative-vs-selfish milestone question — "did the
  cooperative meme survive?" — from the metrics output of a single run, without inspecting
  source code or visualization.
- **SC-002**: Two runs launched with the same seed and configuration produce bit-identical
  metrics streams 100% of the time across consecutive runs on the same machine.
- **SC-003**: A researcher can configure and execute at least 10 distinct parameter
  combinations and compare their end-state meme survival outcomes side-by-side using only
  emitted metrics artifacts.
- **SC-004**: A simulation horizon of at least 1,000 ticks on a default-sized world completes
  on a typical developer laptop in under 30 seconds in headless mode.
- **SC-005**: 100% of memes alive at horizon in any run are traceable back through the lineage
  record to a founding starter meme.
- **SC-006**: The full set of at least six starter memes (share with allies, avoid strangers,
  copy high-energy agents, attack low-energy outsiders, punish non-sharers, prefer agents with
  the same meme) ships in shipped defaults, and each one can be observed influencing at least
  one agent action in a baseline run.
- **SC-007**: A run that reaches agent or meme extinction terminates cleanly, emits final
  metrics, and records an explicit extinction event marker.
- **SC-008**: At least one AI seam (experiment designer, meme namer, or post-run analyst) is
  callable via a stable interface from outside the per-tick loop, even if the only shipped
  implementation is a deterministic no-op stub.

## Assumptions

- **Definition of "meme survives"**: For milestone reporting, a meme is considered to have
  survived if its end-of-run prevalence is ≥ 5% of the live agent population AND at least one
  living carrier exists. Below that threshold the meme is recorded but treated as "did not
  survive." This threshold is itself configurable but defaults to 5%.
- **MVP scope of AI seams**: The MVP ships the seam interfaces and a deterministic no-op
  default. Real LLM-backed implementations (experiment designer, meme namer, post-run analyst)
  are explicitly out of scope for this feature and will land in a follow-up.
- **Visualization shape**: The interactive visualization is a terminal-based view of the grid
  plus at least one meme dynamics pane (e.g., prevalence over time). A graphical / web UI is
  out of scope for the MVP.
- **Default scale**: Default-sized world is on the order of dozens to a few hundred agents on a
  small grid. Larger-scale runs are deferred.
- **Persistence**: Run inputs (configs) and outputs (metrics streams) are written to plain
  files. A database-backed run history and multi-run comparison UI are out of scope for the
  MVP.
- **Social copying bias model**: Modeled as a per-agent or per-population scalar multiplier on
  the meme's transmissibility. Richer network-effect or homophily models are out of scope for
  the MVP.
- **Cognitive cost mechanics**: Cognitive cost is recorded per meme and applied as a per-tick
  energy drain on the carrier. Attention budgets, forgetting curves, and active belief revision
  are out of scope for the MVP.
- **Metrics output format**: A line-oriented, tabular format suitable for offline analysis. The
  exact format (e.g., CSV vs JSON-lines) is a planning-stage decision.
- **Determinism is a hard constraint**: This spec inherits Principle I of the project
  constitution. Any reading of these requirements that would allow nondeterministic behavior
  inside the per-tick loop is incorrect.

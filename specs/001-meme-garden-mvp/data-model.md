# Phase 1 — Data Model

**Feature**: Meme Garden MVP — Memetic Petri Dish
**Branch**: `001-meme-garden-mvp`
**Date**: 2026-05-19

This document captures the in-memory entities, their fields, their relationships, and
the per-tick state transitions. It is a contract between `spec.md` (what) and
`tasks.md` (how). Any deviation in implementation MUST come back here first.

The model is described in Rust terms because the project is a Rust workspace; the same
shapes are realizable in any language.

---

## Conventions

- Identifiers are newtype `u32` wrappers (`AgentId`, `MemeId`, `LineageId`,
  `ClusterId`).
- Ordering of entity iteration follows id assignment order. Ids are issued by
  `Simulation` from monotonic counters; new ids never collide with retired ones.
- All probabilities are `f32` in `[0.0, 1.0]`.
- All scalar mutations clamp to documented ranges.
- Every stochastic decision routes through `core::rng::SimRng`.
- Serde derive is on every entity except `Simulation` itself.

---

## Entities

### 1. `World` (grid environment)

```text
World {
    width: u32,
    height: u32,
    food: Vec<bool>,   // length = width * height; row-major (y * width + x)
}
```

- Indexed by `(x, y)` with `(0, 0)` at top-left.
- `food[idx] == true` means a food unit is present at that cell.
- Mutated by: food spawn at init, regrowth phase, eat action.

### 2. `Agent`

```text
Agent {
    id: AgentId,
    position: Position { x: i32, y: i32 },
    energy: f32,                 // in [0.0, agent_config.max_energy]
    age: u32,
    alive: bool,
    traits: Vec<AgentTrait>,     // small (≤ 4); inherited at birth
    memory: AgentMemory,         // see below
    trust: TrustMap,             // see below
    inventory: Vec<Meme>,        // ordered by insertion; bounded by config.cognition.inventory_cap
}
```

- `traits` are inherited from parent at reproduction with low per-trait mutation
  probability.
- `inventory` cannot exceed `cognition.inventory_cap`. On overflow at acquisition time,
  the oldest meme is removed (FIFO) and an `Event::MemeForgotten` is emitted.
- An agent's *current policy* is **not stored**; it is computed each tick from
  `traits` + `inventory` + perception of neighbors.

#### 2a. `AgentTrait`

```text
enum AgentTrait { Generous, Cautious, Aggressive, Conformist }
```

Bounded enumerated set. Trait inheritance and mutation lives in `mutation.rs`.

#### 2b. `AgentMemory`

```text
AgentMemory {
    last_attacker: Option<AgentId>,
    last_attacked_tick: Option<u64>,
    saw_agent_gain_energy: Option<AgentId>,
}
```

This is the **bounded** memory needed by the symbolic triggers. No general associative
memory; no string storage.

#### 2c. `TrustMap`

```text
TrustMap = SmallVec<[(AgentId, f32); 8]>   // values in [-1.0, 1.0]
```

Bounded per-agent map of other-agent trust. SmallVec keeps it stack-local at MVP scale.
Iteration is in insertion order (stable). Entries fall off when their absolute value
decays below ε (default 0.05) at a per-tick decay rate.

---

### 3. `Meme`

```text
Meme {
    id: MemeId,                   // unique within a run
    lineage_id: LineageId,        // id of the LineageNode for this meme
    kind: MemeKind,               // categorical label, see below
    trigger: Trigger,
    target: TargetSelector,
    effect: Effect,
    strength: f32,                // in [0.0, 1.0]
    transmissibility: f32,        // in [0.0, 1.0]
    mutation_rate: f32,           // in [0.0, 1.0]
    cognitive_cost: f32,          // per-tick energy drain on the carrier (≥ 0.0)
}
```

#### 3a. `MemeKind`

```text
enum MemeKind {
    Cooperative,  // "share with allies" family
    Defensive,    // "avoid strangers" family
    Imitative,    // "copy high-energy agents" family
    Aggressive,   // "attack low-energy outsiders" family
    Punitive,     // "punish non-sharers" family
    Conformist,   // "prefer agents with the same meme" family
    Mutant,       // produced by mutation/recombination; carries no semantic hint
}
```

The kind is a **categorical label only**. Behavior comes from `{trigger, target,
effect, strength}`, never from the kind. The kind exists so metrics aggregate cleanly
("cooperative prevalence" vs "selfish prevalence") and so the AI namer can suggest
labels.

#### 3b. `Trigger`

```text
enum Trigger {
    Hungry,
    NearFood,
    NearAlly,
    NearStranger,
    AttackedRecently,
    SawAgentGainEnergy,
}
```

#### 3c. `TargetSelector`

```text
enum TargetSelector {
    Self_,
    Kin,
    Ally,
    Stranger,
    HighEnergyAgent,
    LowEnergyAgent,
}
```

#### 3d. `Effect`

```text
enum Effect {
    MoveToward,
    MoveAway,
    Share,
    Attack,
    Imitate,
    RefuseInteraction,
    TransmitMeme,
    IncreaseTrust,
    DecreaseTrust,
}
```

These enums are **closed for MVP**. Extending them is a constitutional event (more
variants → more sites to match → re-check exhaustiveness).

---

### 4. `Lineage`

```text
LineageNode {
    id: LineageId,
    parents: SmallVec<[LineageId; 2]>,   // 0 for starters, 1 for mutations, 2 for recombinations
    birth_tick: u64,
    origin: LineageOrigin,
}

enum LineageOrigin { Starter, Mutation, Recombination, Inheritance }

LineageGraph = Vec<LineageNode>          // indexed by LineageId
```

- Append-only. A node's id equals its index in the `LineageGraph` vector for O(1)
  lookup.
- `parents` is bounded by 2 → memory cost is O(memes_ever_created), not exponential.

---

### 5. `RunConfig` (drives an experiment)

```text
SimConfig {
    world: WorldConfig,
    agents: AgentConfig,
    food: FoodConfig,
    scarcity: ScarcityConfig,
    cognition: CognitionConfig,
    transmission: TransmissionConfig,
    mutation: MutationConfig,
    reproduction: ReproductionConfig,
    attack: AttackConfig,
    sharing: SharingConfig,
    memes: MemePoolConfig,
    run: RunConfig,
}
```

Each sub-config carries the knobs the spec calls out. See
`contracts/config.schema.md` for the field-by-field schema.

---

### 6. `Metrics` (per-tick aggregate)

```text
Metrics {
    tick: u64,
    alive: u32,
    food_count: u32,
    population_by_trait: [u32; 4],
    meme_count: u32,
    meme_prevalence_by_kind: [f32; 7],   // indexed by MemeKind enum order
    diversity_shannon: f32,
    dominance_top1_fraction: f32,        // largest single-meme prevalence
    mean_energy: f32,
    mean_age: f32,
    transmissions_this_tick: u32,
    mutations_this_tick: u32,
    deaths_this_tick: u32,
    births_this_tick: u32,
}
```

- Emitted every tick.
- Backwards-compatible with the existing single CSV row format (same first columns).
- The CSV summary writes a flattened subset of these fields; the JSONL stream writes
  the full record under `{"kind": "tick", ...}`.

---

### 7. `Event` (per-event records, between tick records in the stream)

```text
enum Event {
    Birth        { tick: u64, child: AgentId, parent: AgentId, inherited: Vec<MemeId> },
    Death        { tick: u64, agent: AgentId, cause: DeathCause },
    Transmission { tick: u64, from: AgentId, to: AgentId, meme: MemeId },
    Mutation     { tick: u64, parent: MemeId, child: MemeId, field: MutatedField },
    Recombination{ tick: u64, parents: (MemeId, MemeId), child: MemeId },
    MemeForgotten{ tick: u64, agent: AgentId, meme: MemeId },
    Extinction   { tick: u64, scope: ExtinctionScope },        // Population | Memes | OneMeme(MemeId)
    ClusterSnapshot { tick: u64, clusters: Vec<ClusterId, Vec<AgentId>> },
}

enum DeathCause     { Starvation, Aging, Combat }
enum MutatedField   { Trigger, Target, Effect, Strength }
enum ExtinctionScope { Population, AllMemes, SingleMeme(MemeId) }
```

Events are buffered per tick on `Simulation` and drained by the caller via
`Simulation::events_drain() -> Vec<Event>`. The CLI runner is responsible for
serializing each one to the JSONL file.

---

### 8. `Simulation` (root object)

```text
Simulation {
    config: SimConfig,
    grid: Grid,                 // existing
    agents: Vec<Agent>,         // indexed by AgentId
    lineage: LineageGraph,
    next_agent_id: AgentId,
    next_meme_id: MemeId,
    next_lineage_id: LineageId,
    tick: u64,
    pending_events: Vec<Event>, // drained each tick
    rng: SimRng,
}
```

`Simulation` owns the only `SimRng` instance. No other module instantiates one. Borrow
discipline (passing `&mut SimRng` into helpers) is the agreed pattern — never returning
a fresh RNG.

---

## State transitions per tick

A tick executes phases in this **fixed** order. Order is itself a determinism
contract — reordering phases changes outputs and counts as a breaking change.

1. **Perception phase**: For each `Agent` in `AgentId` order, compute the (read-only)
   perception of its neighborhood. No state changes. Populates a per-agent transient
   `Perception` struct held only for this tick.
2. **Policy resolution phase**: For each `Agent` in `AgentId` order, compute the
   `Action` to take this tick from `traits` + `inventory` + `Perception`. This is the
   only place where meme effects compose. The composition rule:
   - Start with the default policy's action distribution.
   - For each meme in inventory order whose trigger matches: multiplicatively bias the
     distribution toward that meme's `effect` by `strength`.
   - Sample the action from the resulting distribution via `SimRng`.
3. **Action execution phase**: For each `Agent` in `AgentId` order, execute its
   chosen `Action`. Conflicts (two agents trying to attack the same victim same tick)
   resolve by lower-`AgentId` first.
4. **Meme transmission phase**: For each `Agent` in `AgentId` order, for each meme in
   inventory order, roll transmission against eligible neighbors per the meme's
   `transmissibility * recipient.social_copying_bias`. On success, mutation may apply
   per the meme's `mutation_rate`.
5. **Reproduction phase**: Agents above `reproduction.energy_threshold` adjacent to a
   compatible partner reproduce; offspring inherit a subset of parent memes with
   per-meme inheritance probability; trait mutation rolls happen here.
6. **Aging / death phase**: Apply metabolism and cognitive cost; agents whose energy
   reaches 0 or whose age exceeds `agents.max_age` die.
7. **World maintenance phase**: Food regrowth (existing).
8. **Metrics / event emission phase**: Compute the tick aggregate; emit the cluster
   snapshot if `tick % cluster_snapshot_every == 0`; package buffered events.

Phases 1–8 form one tick. `Simulation::step()` returns the per-tick `Metrics` and the
caller drains `events_drain()` afterwards.

---

## Invariants enforced by tests

1. **Determinism**: For any fixed `(config, seed)`, the sequence of `(Metrics, [Event])`
   pairs emitted across N ticks is bit-identical between runs.
2. **Lineage closure**: Every live meme has a `lineage_id` that points to a node in
   `LineageGraph`, and that node's `parents` chain terminates at `LineageOrigin::Starter`.
3. **Bounded mutation**: A mutated `Meme` has `trigger`, `target`, `effect` in their
   declared enum ranges and `strength` in `[0.0, 1.0]`.
4. **Inventory cap**: For every agent at every tick,
   `inventory.len() ≤ config.cognition.inventory_cap`.
5. **Extinction tail**: If `Event::Extinction { scope: Population, tick: t }` is emitted,
   metrics continue to be emitted until `tick = config.run.horizon` with `alive = 0`.
6. **No HashMap iteration in step**: Static check via test that `Simulation::step` does
   not allocate a `HashMap` (this is documented; enforced at code review).

---

## Sizing & memory estimates (MVP defaults)

| Item | Default | Memory at default scale |
|---|---|---|
| Agents | 120 | ~50 KB |
| Inventory cap | 8 memes/agent | bounded |
| Lineage nodes per 1000 ticks | ~10² (most mutations + starters) | ~5 KB |
| Per-tick metrics record | ~200 B | 200 KB / 1000 ticks |
| Per-tick events (avg) | ~2 events × ~120 B | ~250 KB / 1000 ticks |

A 1000-tick run is comfortably under 1 MB of metrics output.

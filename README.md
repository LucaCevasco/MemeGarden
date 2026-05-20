# Meme Garden

A controlled **memetic petri dish** in Rust: tiny agents live on a grid, eat,
move, share, attack, reproduce, and pass around symbolic memes that mutate,
compete, and influence behavior. The long-term goal is a research toy for
cultural evolution — does cooperation survive against selfishness under
scarcity? do punishment norms stabilize cooperation? do honest signals beat
deceptive ones? — but the project answers one question first.

## North-star question (the MVP milestone)

> **Can a cooperative meme survive against a selfish meme under different
> levels of scarcity, mutation, and social copying?**

The MVP answers this end-to-end. Three shipped presets run the experiment at
`low`, `mid`, and `high` scarcity; the answer is read straight from the emitted
metrics.

## What this iteration ships (MVP)

- **Deterministic, seedable tick loop.** Same `(config, seed)` → bit-identical
  metrics stream. Verified by a regression test plus a milestone-direction test.
- **Bounded symbolic memes** with `{trigger, target, effect, strength,
  transmissibility, mutation_rate, cognitive_cost, lineage}`. No strings or LLM
  calls anywhere in the tick loop. See [`docs/meme-grammar.md`](docs/meme-grammar.md).
- **Six starter memes**: `share_with_allies`, `avoid_strangers`,
  `copy_high_energy`, `attack_low_energy_outsiders`, `punish_non_sharers`,
  `prefer_same_meme`.
- **Bounded mutation + recombination** with full lineage tracking — every meme
  alive at any tick traces back to a founding starter.
- **Full agent life-cycle**: move, eat, share, attack, imitate, transmit,
  reproduce, die (starvation, combat, aging).
- **Traits + memory + trust map** per agent.
- **Per-tick + per-event metrics** emitted as JSON-Lines, plus a flat per-tick
  CSV summary. Tracks population, per-kind prevalence, transmission events,
  mutation events, extinction, diversity (Shannon), dominance (top-1), and
  Jaccard-based cultural clusters.
- **Ratatui TUI** with world grid + meme-prevalence chart pane.
- **Headless mode** producing identical artifacts to the TUI for the same seed.
- **AI seams** (`MemeNamer`, `ExperimentDesigner`, `RunAnalyst`) behind traits
  with a deterministic `NoopProvider`. Live LLM providers ship in a later
  iteration.
- **47 tests** pass: 13 core unit, 22 core integration, 12 CLI integration.
- **Performance**: 1000-tick run on the default config completes in **~0.6s**
  in release mode (vs the 30s budget).

The full spec lives in [`specs/001-meme-garden-mvp/`](specs/001-meme-garden-mvp/).
The project constitution (binding rules) is at
[`.specify/memory/constitution.md`](.specify/memory/constitution.md).

## Workspace layout

```
crates/
  meme-garden-core/         simulation engine — pure, deterministic, no I/O
    src/
      lib.rs                public surface
      rng.rs                SimRng — the ONLY randomness source
      config.rs             SimConfig + sub-configs + validation
      world.rs              Simulation + 8-phase tick loop + Grid
      agent.rs              Agent, AgentId, AgentTrait, AgentMemory, TrustMap
      meme.rs               Meme + Trigger/TargetSelector/Effect/MemeKind enums
      action.rs             Action enum (Move/Eat/Share/Attack/...)
      policy.rs             per-tick action resolution
      mutation.rs           mutate_in_place + recombine
      lineage.rs            LineageGraph (append-only)
      starters.rs           six starter meme constructors
      metrics.rs            Metrics + Event enum + diversity/dominance helpers
      ai.rs                 MemeNamer / ExperimentDesigner / RunAnalyst + NoopProvider
    tests/                  9 integration test files
  meme-garden-cli/          binary: TUI + headless runner
    src/
      main.rs               clap subcommands: run | headless | list-presets | export | analyze | experiment
      runner.rs             shared tick loop + default_run_id timestamp
      export.rs             RunWriter (JSONL + CSV + config.toml)
      app.rs                TUI state
      tui.rs                ratatui rendering (grid + sparkline)
    tests/                  4 CLI integration test files
configs/
  default.toml              baseline parameters
  presets/                  shipped milestone presets
    cooperation-vs-selfish-low.toml
    cooperation-vs-selfish-mid.toml
    cooperation-vs-selfish-high.toml
docs/
  design.md                 long-term vision (north star, not spec)
  meme-grammar.md           the symbolic grammar in detail
runs/                       per-run artifacts (gitignored)
specs/001-meme-garden-mvp/  executable spec: spec.md, plan.md, tasks.md, contracts/
.specify/memory/constitution.md   project principles
```

## The 8-phase tick loop

Each `Simulation::step()` runs phases in this **fixed** order — reordering them
breaks the determinism contract:

1. **Perception** — for every living agent, build a read-only snapshot of
   its 4-neighborhood (adjacent food, nearby agents with trust/energy
   classifications, hunger flag, attacked-recently flag). No state changes.
2. **Policy resolution** — pick one `Action` per agent. Start from a baseline
   distribution biased slightly by traits; for each meme whose `trigger`
   matches, multiply the weight of the meme's `effect` category by
   `(1 + strength)`; zero out impossible categories; sample via `SimRng`.
3. **Action execution** — apply the chosen action. Movement, eating,
   sharing, attacking, imitating. Death emits `Event::Death`.
4. **Transmission** — for each agent in `AgentId` order, for each meme in
   inventory order, roll transmission against eligible neighbors. On success,
   maybe mutate. Emit `Event::Transmission` and (if applicable)
   `Event::Mutation`.
5. **Reproduction** — adjacent eligible pairs produce offspring; offspring
   inherit a subset of parent traits + memes; recombination may fuse two
   parental memes. Emit `Event::Birth`.
6. **Death** — apply metabolism + cognitive cost; agents past `max_age` or
   below 0 energy die. Trust map decays.
7. **World maintenance** — food regrowth.
8. **Metrics emission** — compute per-tick aggregate, emit `Event::Tick`,
   detect first-time extinction events, emit cluster snapshots on cadence.

## Run it

### One-line milestone

```sh
cargo run --release -p meme-garden-cli -- headless \
  --preset cooperation-vs-selfish-low --seed 42
```

Writes `runs/<YYYYMMDD-HHMMSS>-cooperation-vs-selfish-low/{config.toml,events.jsonl,summary.csv}`.

### Interactive TUI

```sh
cargo run -p meme-garden-cli -- run --preset cooperation-vs-selfish-mid --seed 42
```

Left pane shows the world grid (`C` = cooperative carrier, `S` = selfish
carrier, `X` = both, `a` = no relevant meme, `.` = food). Right side shows the
live metric panel plus a prevalence-over-time chart.

### Sweep three scarcity levels

```sh
for level in low mid high; do
  cargo run --release -q -p meme-garden-cli -- headless \
    --preset cooperation-vs-selfish-$level --seed 42 --ticks 1000 \
    --run-id milestone-$level
done

# Compare final prevalence:
for level in low mid high; do
  printf "%-5s " $level
  tail -1 runs/milestone-$level/summary.csv \
    | awk -F, '{printf "alive=%-3s coop=%.3f aggr=%.3f\n", $2, $5, $8}'
done
```

### Inspect a run

```sh
# Markdown summary via the RunAnalyst seam (NoopProvider for now):
cargo run -p meme-garden-cli -- analyze runs/milestone-low/

# Cooperative vs aggressive prevalence over time, via jq:
jq -r 'select(.kind=="tick") | "\(.tick),\(.meme_prevalence_by_kind.cooperative),\(.meme_prevalence_by_kind.aggressive)"' \
   runs/milestone-low/events.jsonl | tail -20

# Regenerate summary.csv from events.jsonl:
cargo run -p meme-garden-cli -- export runs/milestone-low --to csv

# Validate the JSONL stream:
cargo run -p meme-garden-cli -- export runs/milestone-low --to jsonl
```

### Other commands

```sh
cargo run -p meme-garden-cli -- list-presets
cargo run -p meme-garden-cli -- experiment design "study famine cooperation"
#   → exits 2 with "Error: ai provider not configured" (NoopProvider stub)
```

## TUI keys

| key | action |
|---|---|
| `space` | pause / resume |
| `s` | single step (while paused) |
| `+` / `-` | speed up / slow down |
| `q`, esc | quit |

## Test

```sh
cargo test --workspace
```

47 tests; load-bearing among them:

- `world::tests::same_seed_same_metrics` — Principle I determinism gate.
- `tests/milestone.rs::milestone_direction_is_recorded` — the cooperative-vs-selfish
  experiment's *direction* of survival is stable across re-runs. Changing the
  simulator changes which scarcity buckets the cooperative meme survives in;
  that's a milestone outcome, not a defect to be hidden by relaxing the test.
- `tests/tui_headless_equivalence.rs` — TUI and headless produce identical event
  streams for the same `(config, seed)`.

## Config reference

Every run is parameterized by a TOML file deserialized into `SimConfig`. Below is
the full surface. Defaults shown are the values in
`configs/presets/cooperation-vs-selfish-low.toml`.

### `[world]` — the grid

| Param | Type | Meaning |
|---|---|---|
| `width` | `u32 > 0` | Grid columns. |
| `height` | `u32 > 0` | Grid rows. Agents can stack on the same cell. |

Bigger grid → agents spread out, transmission slows. Smaller grid → forced contact, faster spread, more fights.

### `[agents]` — population & life-cycle

| Param | Type | Meaning |
|---|---|---|
| `count` | `u32 > 0` | Initial agent population. |
| `starting_energy` | `f32 > 0` | Energy each agent spawns with. Also the baseline for the `Hungry` trigger (`< 0.5 × starting_energy`) and the `LowEnergyAgent` target. |
| `metabolism` | `f32 ≥ 0` | Energy lost per tick by every living agent, before action costs. |
| `max_energy` | `f32 ≥ starting_energy` | Hard cap. Threshold for `HighEnergyAgent` is `0.75 × max_energy`. |
| `max_age` | `u32 > 0` | Ticks before death by `Aging`. |
| `initial_traits_dist` | `[f32; 4]` summing to 1.0 | Initial trait distribution: `[Generous, Cautious, Aggressive, Conformist]`. Traits bias per-tick action weights (e.g. `Generous` × 1.4 on `Share`). |
| `trait_mutation_rate` | `f32 ∈ [0, 1]` | Per-trait probability at reproduction that an inherited trait re-rolls. |

### `[food]` — the resource

| Param | Type | Meaning |
|---|---|---|
| `initial_density` | `f32 ∈ [0, 1]` | Fraction of cells seeded with food at tick 0. |
| `regrowth_rate` | `f32 ∈ [0, 1]` | Per-empty-cell, per-tick probability of food growing. |
| `energy_per_food` | `f32 > 0` | Energy gained when eating one food unit. |

### `[scarcity]` — convenience knob

| Param | Type | Meaning |
|---|---|---|
| `level` | `"low" \| "mid" \| "high" \| "custom"` | Multiplier applied to `food.initial_density` and `food.regrowth_rate` at load: `low` = 1.0×, `mid` = 0.5×, `high` = 0.2×. `custom` leaves `food.*` untouched. The resolved (post-multiplier) values are what get written to `runs/<id>/config.toml`. |

This is why the three milestone presets are identical except for one line.

### `[cognition]` — bounded inventory

| Param | Type | Meaning |
|---|---|---|
| `inventory_cap` | `u32 > 0` | Max memes per agent. On overflow the **oldest** meme is dropped FIFO and a `MemeForgotten` event fires. |

### `[transmission]` — how memes spread

| Param | Type | Meaning |
|---|---|---|
| `base_rate` | `f32 ∈ [0, 1]` | Multiplied into every transmission roll. `0` disables all transmission. |
| `social_copying_bias_mean` | `f32 ∈ [0, 1]` | Mean of the per-agent "how willing am I to adopt new memes" trait, sampled at birth. |
| `social_copying_bias_std` | `f32 ≥ 0` | Standard deviation of that per-agent draw. |
| `prestige_boost` | `f32 ∈ [0, 1]` | Additive bonus when the transmitter is top-quartile energy. |

Roll for "does meme M move from A to B this tick":

```
p = base_rate * meme.transmissibility * B.social_copying_bias
if A is top-quartile energy: p += prestige_boost
sample = SimRng.gen_bool(p.clamp(0, 1))
```

### `[mutation]` — how memes change on transmission

| Param | Type | Meaning |
|---|---|---|
| `strength_jitter_max` | `f32 ≥ 0` | Max ± delta on `strength` (clamped to `[0, 1]`). |
| `enum_swap_probability` | `f32 ∈ [0, 1]` | Probability a mutation event swaps one of `trigger`/`target`/`effect` to a different enum variant. |

The per-meme `mutation_rate` (in `starters.rs`) gates *whether* mutation fires on
a transmission; these knobs control *how big* the change is. Mutation preserves
`MemeKind`.

### `[reproduction]` — making new agents

| Param | Type | Meaning |
|---|---|---|
| `energy_threshold` | `f32` | Both parents must be ≥ this energy. |
| `offspring_energy_cost` | `f32` | Energy each parent pays. |
| `inherit_meme_prob` | `f32 ∈ [0, 1]` | Per-parent-meme probability of inheritance. |
| `min_age` | `u32` | Minimum age to reproduce. |

Recombination of two parental memes fires with a fixed 20% chance when both
parents have non-empty inventories.

### `[attack]` — combat mechanics

| Param | Type | Meaning |
|---|---|---|
| `energy_cost_attacker` | `f32 ≥ 0` | Energy the attacker spends. |
| `energy_steal` | `f32 ≥ 0` | Energy transferred from victim to attacker (capped at victim's energy). |
| `retaliation_chance` | `f32 ∈ [0, 1]` | _Reserved._ Currently parsed but not yet wired into victim counter-attack. |

Attacks drop the victim's trust in the attacker by 0.30, set
`last_attacker` in memory (feeding the `AttackedRecently` trigger for 10 ticks),
and emit `Death { cause: Combat }` on kill.

### `[sharing]` — the cooperative action

| Param | Type | Meaning |
|---|---|---|
| `share_threshold` | `f32` | Donor only shares if its own energy is above this. |
| `share_amount` | `f32 > 0` | Energy transferred per share. Donor's trust in recipient bumps `+0.10`. |

### `[[memes.seed]]` — initial meme pool

Repeated table — one entry per seeded meme.

| Param | Type | Meaning |
|---|---|---|
| `name` | string | Must be one of the six starters: `share_with_allies`, `avoid_strangers`, `copy_high_energy`, `attack_low_energy_outsiders`, `punish_non_sharers`, `prefer_same_meme`. |
| `carrier_fraction` | `f32 ∈ [0, 1]` | Independent per-agent probability of getting this starter. Two entries at `0.5` give ~25% with both, ~25% with neither, ~50% with exactly one. |

### `[run]` — execution control

| Param | Type | Meaning |
|---|---|---|
| `seed` | `u64` | The **only** randomness source. Same seed + same config = bit-identical metrics. Overridable via `--seed`. |
| `horizon` | `u32 > 0` | Max ticks. Overridable via `--ticks`. |
| `stop_on_extinction` | `bool` | If `true`, terminate at the first population extinction. Default `false` keeps emitting per-tick metrics so the post-extinction tail is visible. |
| `cluster_snapshot_every` | `u32` | Cadence (ticks) for Jaccard cultural-cluster snapshots. `0` disables. |
| `metrics_emit_every` | `u32 ≥ 1` | Cadence for `Event::Tick`. Raise for shorter `events.jsonl`. |
| `survival_threshold` | `f32 ∈ [0, 1]` | Prevalence bar a meme must clear at horizon to be reported as "survived." |

## Quick recipes

- **Make cooperation fail.** Raise `agents.metabolism`, lower
  `food.regrowth_rate`, raise `sharing.share_amount`. Donors bleed energy
  faster than they can recover it.
- **Maximize mutation drift.** Raise `mutation.strength_jitter_max` and
  `mutation.enum_swap_probability` toward 1.0. (Per-meme `mutation_rate` is in
  `starters.rs` for now.)
- **Pure deterministic baseline (no mutation, no trait drift).** Set
  `mutation.strength_jitter_max = 0`, `mutation.enum_swap_probability = 0`,
  `agents.trait_mutation_rate = 0`. Memes still spread but never mutate.
- **Fast-forward sweep.** Set `metrics_emit_every = 10`,
  `cluster_snapshot_every = 0`, raise `--ticks`. Same simulation, ~10× smaller
  `events.jsonl`.
- **Force a single-meme world.** Drop one `[[memes.seed]]` entry and bump the
  remaining one's `carrier_fraction` to `0.9+`. Useful for "does this meme
  spread on its own merits" tests.
- **Test a meme's solo viability.** Same as above, but seed only one starter at
  `0.05`. Check whether it still reaches `≥ run.survival_threshold` by horizon.
- **Reproducibility check.** Run twice with the same seed, then
  `tail -n +2 runs/<a>/events.jsonl | shasum -a 256` against the same for run
  `<b>`. Hashes must match.

## Determinism contract

Every stochastic decision in `meme-garden-core` flows through
`rng::SimRng`. The core forbids `std::time::*`, `rand::thread_rng`,
environment variables, and any other ambient nondeterministic source. The CLI
generates the run-id timestamp before constructing `Simulation`, so wall-clock
never leaks into the metrics. Two runs with the same seed produce
byte-identical `events.jsonl` (after stripping the run-id-bearing header).

## What's next

- LLM-backed implementations of the three AI seams (a future
  `meme-garden-ai` crate; `meme-garden-core` stays HTTP-free).
- Lineage-tree visualization pane in the TUI.
- A `sweep` subcommand that runs a parameter grid in one invocation.
- A `replay` subcommand that re-renders a finished run in the TUI without
  re-running the simulator.
- Mutation of `transmissibility` and `mutation_rate` themselves (currently
  fixed per-meme, by design — keeps the milestone interpretable).
- Connected-component cultural clusters (currently Jaccard threshold-based).
- Property-based fuzzing of the mutation operator.

The complete tasks list is in
[`specs/001-meme-garden-mvp/tasks.md`](specs/001-meme-garden-mvp/tasks.md).

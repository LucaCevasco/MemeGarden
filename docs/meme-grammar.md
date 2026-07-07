# The Symbolic Meme Grammar

Memes in Meme Garden are **bounded symbolic structures**, not natural-language
beliefs or arbitrary code. Every meme is a single tuple of small enum-valued
fields plus four `f32` scalars. The simulator never reads strings to decide
behavior; it never invokes an LLM in the tick loop. Constitution Principle IV
is exactly this rule.

## The struct

```rust
struct Meme {
    id: MemeId,                   // unique within a run
    lineage_id: LineageId,        // pointer into LineageGraph
    kind: MemeKind,               // categorical label
    trigger: Trigger,             // when does this meme activate?
    target: TargetSelector,       // who does it target?
    effect: Effect,               // what does it bias toward?
    strength: f32,                // in [0.0, 1.0]
    transmissibility: f32,        // in [0.0, 1.0]
    mutation_rate: f32,           // in [0.0, 1.0]
    cognitive_cost: f32,          // per-tick energy drain
}
```

## The four enums

### `Trigger` — when does this meme fire?

- `Hungry` — the carrier's energy is below half its starting energy.
- `NearFood` — at least one adjacent grid cell carries food.
- `NearAlly` — at least one adjacent agent has non-negative trust.
- `NearStranger` — at least one adjacent agent has negative trust.
- `AttackedRecently` — the carrier was attacked within the last 10 ticks.
- `SawAgentGainEnergy` — a high-energy agent is within perception range.

### `TargetSelector` — who does it target?

- `Self_` — the carrier itself.
- `Kin` — descendants of the carrier (post-MVP; currently always false).
- `Ally` — adjacent agents with non-negative trust.
- `Stranger` — adjacent agents with negative trust.
- `HighEnergyAgent` — perception-range agents whose energy is in the top quartile.
- `LowEnergyAgent` — perception-range agents whose energy is below half-baseline.

### `Effect` — what behavior does it bias?

- `MoveToward` / `MoveAway` — directional movement.
- `Share` — donate energy.
- `Attack` — combat action that steals energy.
- `Imitate` — copy a meme from the target.
- `RefuseInteraction` — no-op; the meme acts as a brake.
- `TransmitMeme` — boost transmission rolls toward the target.
- `IncreaseTrust` / `DecreaseTrust` — adjust the trust map entry.

### `MemeKind` — categorical label for analytics

`Cooperative, Defensive, Imitative, Aggressive, Punitive, Conformist, Mutant`.
Behavior comes from `{trigger, target, effect, strength}` — the kind is for
metric bucketing, naming via `MemeNamer`, and milestone reporting.

## How policy resolution composes memes

For each tick, each agent picks one `Action` via a categorical sample
weighted by:

1. A baseline distribution biased slightly by the agent's `traits`.
2. Hunger / proximity / reproduction eligibility bonuses.
3. **For each meme in the inventory whose `trigger` matches the current
   perception**: multiply the weight of the meme's `effect` category by
   `(1 + strength)`.
4. Zero out categories with no valid target.

The sample is drawn from `SimRng` (the only randomness source).

## Why memes are bounded

- **Determinism**: same seed + same config ⇒ bit-identical metrics.
- **Debuggability**: a meme is a struct you can `dbg!()`. No string parsing.
- **Measurability**: prevalence, transmission rate, mutation events, lineage
  trees — all trivially computed from the struct fields.
- **Cost**: no model calls in the hot loop means a 1000-tick run completes
  in well under a second on a developer laptop.

## How mutation works (and what it does NOT do)

Mutation operates on exactly four fields: `trigger`, `target`, `effect`,
`strength`.

- Enum-valued fields swap to another enum variant chosen uniformly at random.
  The probability per field is `mutation.enum_swap_probability`.
- `strength` jitters within `±mutation.strength_jitter_max`, clamped to
  `[0.0, 1.0]`.
- `transmissibility`, `mutation_rate`, and `cognitive_cost` are held fixed.

`kind` tracks *behaviour*, not ancestry: it is derived from the meme's `effect`
(`share → Cooperative`, `attack → Aggressive`, `move_away → Defensive`,
`imitate → Imitative`, `increase_trust → Conformist`; effects with no clear
valence fall back to `Mutant`). A mutation that swaps the `effect` field
re-derives `kind`; trigger/target/strength changes leave it alone. A recombinant
takes the `kind` implied by whichever parent's `effect` it inherits — so hybrids
re-enter conflict resolution as cooperators or aggressors rather than hiding in a
conflict-exempt `Mutant` bucket. `Mutant` is now only the rare no-valence fallback.

## The six starter memes

| Constructor | Kind | Trigger | Target | Effect |
|---|---|---|---|---|
| `share_with_allies` | Cooperative | NearAlly | LowEnergyAgent | Share |
| `avoid_strangers` | Defensive | NearStranger | Stranger | MoveAway |
| `copy_high_energy` | Imitative | SawAgentGainEnergy | HighEnergyAgent | Imitate |
| `attack_low_energy_outsiders` | Aggressive | NearStranger | LowEnergyAgent | Attack |
| `punish_non_sharers` | Punitive | SawAgentGainEnergy | Stranger | Attack |
| `prefer_same_meme` | Conformist | NearAlly | Ally | IncreaseTrust |

Their initial `transmissibility` is `0.40–0.50`, initial `strength` is
`0.4–0.7`, initial `mutation_rate` is `0.05`, and initial `cognitive_cost` is
small (`0.01–0.03` energy/tick). All numerics are tuned in `crates/meme-garden-core/src/starters.rs`.

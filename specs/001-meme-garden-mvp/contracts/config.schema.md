# Contract — Run Configuration Schema (TOML)

Every run is parameterized by a TOML file deserialized into `SimConfig`. The schema
below is the public contract: any field rename, removal, or type change is a breaking
change.

Defaults shown are the values shipped in `configs/default.toml` (post-MVP-extension).

```toml
[world]
width        = 60       # u32, > 0
height       = 30       # u32, > 0

[agents]
count            = 120  # u32, > 0
starting_energy  = 25.0 # f32, > 0
metabolism       = 0.4  # f32, energy lost per tick per living agent, ≥ 0
max_energy       = 60.0 # f32, > starting_energy
max_age          = 800  # u32, ticks; > 0
initial_traits_dist = [0.35, 0.20, 0.20, 0.25]
                        # probabilities summing to 1.0, in MemeKind/AgentTrait enum order:
                        # [Generous, Cautious, Aggressive, Conformist]
trait_mutation_rate = 0.01    # f32 in [0,1]; per-trait reroll prob at reproduction

[food]
initial_density   = 0.18    # f32 in [0,1]
regrowth_rate     = 0.002   # f32 in [0,1]; per-empty-cell per-tick
energy_per_food   = 9.0     # f32, > 0

[scarcity]
# A scalar "preset" knob. The runner reads this and applies a transform to
# food.initial_density and food.regrowth_rate at load time, then drops scarcity
# from the resolved config copy. low = 1.0 baseline; mid = 0.5x; high = 0.2x.
level = "low"               # "low" | "mid" | "high" | "custom"

[cognition]
inventory_cap = 8           # u32, max memes per agent; > 0

[transmission]
base_rate                = 0.45  # f32 in [0,1]; multiplied by meme.transmissibility
social_copying_bias_mean = 0.5   # f32 in [0,1]; per-agent gaussian draw mean
social_copying_bias_std  = 0.15  # f32, ≥ 0
prestige_boost           = 0.10  # f32, additive bonus when carrier energy is top-quartile

[mutation]
strength_jitter_max   = 0.10  # f32, ≥ 0; max single-step Δ on strength
enum_swap_probability = 0.20  # f32 in [0,1]; per-mutation chance to swap an enum field

[reproduction]
energy_threshold       = 40.0  # f32; both parents must be ≥ this
offspring_energy_cost  = 15.0  # f32; deducted from each parent
inherit_meme_prob      = 0.5   # f32 in [0,1]; per-meme inheritance probability
min_age                = 50    # u32, ticks; agent must be at least this old

[attack]
energy_cost_attacker = 3.0     # f32, ≥ 0
energy_steal         = 5.0     # f32, ≥ 0
retaliation_chance   = 0.5     # f32 in [0,1]

[sharing]
share_threshold        = 22.0  # f32; default policy only shares if energy ≥ this
share_amount           = 2.0   # f32, > 0
share_target           = "low_energy_ally"   # "low_energy_ally" | "kin"

[memes]
# Initial meme population. Each entry instantiates a starter meme with the given
# carrier fraction. Memes named here must be drawn from the starter set:
#   share_with_allies | avoid_strangers | copy_high_energy | attack_low_energy_outsiders
#   | punish_non_sharers | prefer_same_meme
seed = [
    { name = "share_with_allies",          carrier_fraction = 0.5 },
    { name = "attack_low_energy_outsiders", carrier_fraction = 0.5 },
]

[run]
seed                      = 42      # u64
horizon                   = 1000    # u32, tick count
stop_on_extinction        = false   # bool
cluster_snapshot_every    = 50      # u32, ticks; 0 disables cluster snapshots
metrics_emit_every        = 1       # u32, ticks; ≥ 1
survival_threshold        = 0.05    # f32 in [0,1]; meme "survives" if end prevalence ≥ this
```

## Validation rules

- All probabilities are in `[0.0, 1.0]`. Validation rejects values outside that range
  with a typed `ConfigError::OutOfRange`.
- `agents.count` must be > 0 and `≤ world.width * world.height` (each agent starts on a
  cell, but multiple agents per cell are allowed; the check guards against absurd
  values).
- `initial_traits_dist` must sum to 1.0 ± 1e-6.
- `memes.seed[*].name` must be one of the documented starter names; unknown names
  produce `ConfigError::UnknownStarterMeme`.
- `scarcity.level == "custom"` means `food.*` is used as-is; otherwise the runner
  applies the scarcity transform and the resolved config is written to the run dir.

## Backwards compatibility with the POC config

The existing `configs/default.toml` (5 sections, no scarcity / cognition / mutation /
reproduction / attack / sharing tables) is **not** valid under this schema. The
config loader recognizes the legacy shape — exactly the existing fields — and adapts
it with documented defaults for the new sections, emitting a `tracing::warn!` line.
This adaptation is removed once the default config is updated.

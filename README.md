# Meme Garden

A controlled **memetic petri dish** in Rust: tiny agents live on a grid, eat, share, and pass around symbolic memes that mutate, compete, and influence behavior. The long-term goal is a research toy for cultural evolution — does cooperation survive against selfishness? do honest signals beat deceptive ones? — but the codebase grows from a very small core.

This iteration is the **a minimal proof of concept**:

- Deterministic, seedable tick loop.
- Grid world with agents that move, eat food, and metabolize energy.
- **One** hardcoded transmissible meme — `SharerNorm` — that propagates when a carrier shares energy with an adjacent low-energy ally.
- Terminal UI (ratatui) for live observation.
- Headless CSV dump for smoke testing and future analysis pipelines.
- Trait stubs (no SDK wired yet) marking where AI providers — meme namer, run analyst — will plug in.

See [`docs/design.md`](docs/design.md) for the full project vision.

## Workspace layout

```
crates/
  meme-garden-core/    simulation engine (library)
  meme-garden-cli/     binary: TUI + headless runner
configs/default.toml   sim parameters
docs/design.md         memetic-evolution kickoff notes (north star, not spec)
```

## Simulation Phases

1. move_and_feed_phase (world.rs:74)

For each living agent, in stable AgentId order:

1. age += 1, energy -= metabolism.
2. If energy ≤ 0 → agent dies (alive = false), skip to the next agent.
3. Pick a direction:
  - Scan the 4 cardinal neighbors. If any contain food, pick one of the food-bearing neighbors at random (food-seeking bias).
  - Otherwise, pick a random direction from N/S/E/W.
4. Move if the target cell is in bounds (off-grid attempts are no-ops — they just don't move).
5. Eat if standing on a food cell: clear the food, gain energy_per_food, clamped at max_energy.

So this phase is the entire "biological layer" — metabolism, movement, eating, death.

2. meme_phase (world.rs:135)

This is the only place the meme actually does anything. For each living agent in stable order:

1. Skip if it carries no meme, or if its own energy is at or below share_threshold (won't share when hungry itself).
2. Scan all other agents to find the first adjacent ally with energy below share_threshold (4-neighbor adjacency, is_adjacent = Manhattan distance 1).
3. If a recipient is found:
  - Transfer share_amount energy (capped at the sharer's current energy, recipient capped at max_energy).
  - Roll transmissibility: if it hits and the recipient carries no meme, copy the meme over.

Two important POC simplifications baked in:

- First match wins (stable id order). No closest-neighbor heuristic, no fairness.
- No re-infection. A recipient that already has the meme just receives energy, no re-roll. Fine for one variant; will matter when multiple memes compete.

3. regrowth_phase (world.rs:184)

For every cell on the grid, if it's empty, roll regrowth_rate — on success, food sprouts. This is what keeps the world from starving to death after the initial food is eaten. It's
O(W·H) per tick, which is fine for POC sizes.

Then snapshot() + tick += 1

Walks all agents once to count alive, count carriers, sum energy, count food, and returns a Metrics row. Tick counter increments after the snapshot, so the first call to step() returns
metrics labeled tick = 0.

## Run it

Interactive TUI:

```sh
cargo run -p meme-garden-cli -- run --seed 42
```

Headless CSV dump (per-tick metrics to stdout):

```sh
cargo run -p meme-garden-cli -- headless --seed 42 --ticks 500 > run.csv
```

Optional flags: `--config <path>`, `--tps <float>`.

## TUI keys

| key       | action            |
|-----------|-------------------|
| `space`   | pause / resume    |
| `s`       | single step (while paused) |
| `+` / `-` | speed up / slow down       |
| `q`, esc  | quit              |

## Test

```sh
cargo test --workspace
```

Includes a determinism test: same seed → identical metrics stream over 100 ticks.

## What's next

- Symbolic meme grammar: `Trigger` / `Effect` / `Target` enums + mutation + recombination.
- Multiple memes per agent (`meme_inventory`), cognitive cost.
- Reproduction, kinship, trust maps, signals, territory.
- Real AI provider wiring behind the `MemeNamer` / `RunAnalyst` traits.
- Phylogenetic meme tree + post-run analyst summaries.

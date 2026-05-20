# Quickstart — Meme Garden MVP

Hands-on path through the MVP, intended for a contributor who has cloned the repo and
just wants the simulation answering its north-star question.

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml` (stable, MSRV 1.75).
- A terminal that supports ANSI cursor positioning (any modern Mac/Linux terminal).

## 1. Confirm the workspace builds and tests pass

```sh
cargo check --workspace
cargo test --workspace
```

Both must pass on `main` before you start. The load-bearing test is
`crates/meme-garden-core/src/world.rs::tests::same_seed_same_metrics` — that's the
determinism gate from constitution Principle I.

## 2. Run the default headless simulation

```sh
cargo run -p meme-garden-cli -- headless --seed 42 --ticks 500
```

This writes a run directory under `runs/<timestamp>-default/` containing:

- `config.toml` — exactly what the simulator consumed.
- `events.jsonl` — per-tick + per-event records.
- `summary.csv` — flat per-tick summary.

Inspect the final tick's prevalence:

```sh
tail -n 1 runs/*-default/summary.csv
```

## 3. Run the milestone experiment

The MVP ships three scarcity presets for the cooperative-vs-selfish milestone:

```sh
cargo run -p meme-garden-cli -- headless --preset cooperation-vs-selfish-low  --seed 42
cargo run -p meme-garden-cli -- headless --preset cooperation-vs-selfish-mid  --seed 42
cargo run -p meme-garden-cli -- headless --preset cooperation-vs-selfish-high --seed 42
```

Each preset seeds the world with the cooperative meme at 50% and the selfish meme at
50%, varying only `scarcity.level`. The milestone question — *did the cooperative meme
survive under this scarcity?* — is answered by:

```sh
jq -r 'select(.kind == "tick") | "\(.tick),\(.meme_prevalence_by_kind.Cooperative),\(.meme_prevalence_by_kind.Aggressive)"' \
   runs/*-cooperation-vs-selfish-low/events.jsonl | tail -n 5
```

A meme is reported as "surviving" if its end-state prevalence is ≥
`run.survival_threshold` (default `0.05`) with at least one living carrier.

## 4. Run the interactive TUI

```sh
cargo run -p meme-garden-cli -- run --seed 42 --preset cooperation-vs-selfish-mid
```

- Left pane: world grid (`.` = empty, `*` = food, lowercase letters = agents by trait).
- Right pane: meme prevalence over time, one line per `MemeKind`.
- `q` quits cleanly and finalizes the run directory.

The TUI produces an identical `events.jsonl` byte stream to the headless command for
the same `(config, seed)` (FR-026). If you find a divergence, that's a determinism
bug — file it.

## 5. Reproduce a result

```sh
cargo run -p meme-garden-cli -- headless --seed 42 --ticks 500
sha256sum runs/<latest>/events.jsonl
# clear it out, run again with the same seed:
cargo run -p meme-garden-cli -- headless --seed 42 --ticks 500
sha256sum runs/<latest>/events.jsonl
```

The two hashes MUST be identical. If they're not, you've found a constitution-Principle-I
breach.

## 6. Sweep a single parameter

Trivial shell loop until a `sweep` subcommand exists:

```sh
for s in low mid high; do
  cargo run -q -p meme-garden-cli -- headless \
    --preset cooperation-vs-selfish-$s \
    --seed 42 \
    --run-id "milestone-$s"
done

# Compare end-state prevalences
for s in low mid high; do
  printf "%-5s " "$s"
  tail -n 1 runs/milestone-$s/summary.csv
done
```

## 7. Wire up an AI seam (optional, post-MVP)

The CLI's `experiment design` and `analyze` subcommands are wired to `NoopProvider`
out of the box:

```sh
cargo run -p meme-garden-cli -- experiment design "study cooperation under famine"
# → Error: ai provider not configured

cargo run -p meme-garden-cli -- analyze runs/milestone-low/
# → "ran 1000 ticks; final alive=37, top1_meme_share=0.78, diversity=0.41"
```

Replacing `NoopProvider` with a live LLM provider is a future feature and lives in a
new `meme-garden-ai` crate. The simulation core does not change.

## 8. Where the contracts live

If you're touching code, read these before opening a PR:

- `specs/001-meme-garden-mvp/data-model.md` — entities & tick phases.
- `specs/001-meme-garden-mvp/contracts/config.schema.md` — config fields & validation.
- `specs/001-meme-garden-mvp/contracts/metrics.schema.md` — JSONL & CSV output.
- `specs/001-meme-garden-mvp/contracts/cli.md` — user-facing command surface.
- `specs/001-meme-garden-mvp/contracts/ai-seams.md` — trait surfaces for AI integrations.
- `.specify/memory/constitution.md` — the binding principles every PR must respect.

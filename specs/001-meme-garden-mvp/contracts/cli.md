# Contract — CLI Command Surface

The `meme-garden` binary lives in `crates/meme-garden-cli`. Subcommand surface for the
MVP:

```text
meme-garden <subcommand> [args]
```

## Subcommands

### `meme-garden run`

Interactive TUI run. Opens the ratatui dashboard, advances the simulation each frame,
and writes the same artifacts as the headless mode under `runs/`.

```text
meme-garden run [--config <path>] [--seed <u64>] [--preset <name>] [--run-id <name>]
```

- `--config <path>`: Path to a TOML config. Defaults to `configs/default.toml`.
- `--seed <u64>`: Overrides `run.seed` in the config.
- `--preset <name>`: Loads `configs/presets/<name>.toml` instead of `--config`.
- `--run-id <name>`: Overrides the default `<YYYYMMDD-HHMMSS>-<short-name>` run id.

Exit code: 0 on clean shutdown, 1 on any error.

### `meme-garden headless`

Non-interactive run. Same simulation behavior as `run`, no TUI. Used for sweeps,
regression tests, and CI.

```text
meme-garden headless [--config <path>] [--seed <u64>] [--preset <name>] [--ticks <u32>] [--run-id <name>]
```

- `--ticks <u32>`: Overrides `run.horizon`. Useful for short smoke runs.
- Other flags as `run`.

Exit code: 0 on successful run-to-horizon. 1 on configuration or I/O error. **Never**
non-zero on simulation outcomes (extinction, dominance, etc.) — those are data, not
errors.

### `meme-garden list-presets`

Lists every TOML file under `configs/presets/` with its description (read from a
top-of-file `# description:` line if present).

### `meme-garden export`

Re-emits a finished run's metrics into alternative shapes for tooling.

```text
meme-garden export <run-dir> --to <csv|jsonl|summary-md>
```

- `csv`: Re-emit `summary.csv` from `events.jsonl` (used to regenerate a corrupted CSV).
- `jsonl`: Pass-through (used to validate JSONL records against the schema).
- `summary-md`: Human-readable Markdown summary; calls `RunAnalyst::summarize`.

### `meme-garden experiment design`

Calls `ExperimentDesigner::design(prompt)` and writes the result to stdout (or to a
`--out <path>` file).

```text
meme-garden experiment design "<natural language prompt>" [--out <path>]
```

With `NoopProvider`, this returns `Error: ai provider not configured` and exits 2.
Documented so that adding a live provider later does not change the user-visible
surface.

### `meme-garden analyze`

```text
meme-garden analyze <run-dir>
```

Loads the run's `events.jsonl`, reconstructs the metrics history + lineage graph,
calls `RunAnalyst::summarize`, prints the result.

## Global behaviors

- **Logging**: `tracing_subscriber` initialized once in `main`. Default level `info`;
  overridden via `RUST_LOG=...`.
- **Run-dir creation**: `run` and `headless` create `runs/<run-id>/` and write
  `config.toml`, `events.jsonl`, `summary.csv`. If the directory exists the command
  errors with `Error: run id <id> already exists` (exit 1). `--run-id` allows the
  user to disambiguate.
- **Deterministic mode**: Both `run` and `headless` MUST produce identical
  `events.jsonl` byte streams for the same `(config, seed)` pair. The TUI presence
  MUST NOT affect simulation state.

## Out of scope for MVP

- A `compare` subcommand that diffs two runs (deferred — handled offline with `jq` /
  pandas for now).
- A `replay` subcommand that re-renders a finished run in the TUI without re-running
  the simulator.
- A `sweep` subcommand that runs a parameter grid (the user can script this with shell
  for now; baking it in is post-MVP).

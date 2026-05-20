# Contract — Metrics Output Schema

A run writes three artifacts under `runs/<YYYYMMDD-HHMMSS>-<short-name>/`:

1. `config.toml` — the resolved configuration after scarcity transform and any
   adaptation; byte-identical to what the simulator actually used.
2. `events.jsonl` — newline-delimited JSON records (per-tick aggregate + events).
3. `summary.csv` — flat per-tick summary, suitable for spreadsheet inspection.

## `events.jsonl`

Each line is one record. Records are discriminated by a `"kind"` field.

### Per-tick aggregate record (`kind: "tick"`)

```json
{
  "kind": "tick",
  "tick": 0,
  "alive": 120,
  "food_count": 324,
  "population_by_trait": { "Generous": 42, "Cautious": 24, "Aggressive": 24, "Conformist": 30 },
  "meme_count": 60,
  "meme_prevalence_by_kind": {
    "Cooperative": 0.50,
    "Defensive": 0.0,
    "Imitative": 0.0,
    "Aggressive": 0.50,
    "Punitive": 0.0,
    "Conformist": 0.0,
    "Mutant": 0.0
  },
  "diversity_shannon": 1.0,
  "dominance_top1_fraction": 0.50,
  "mean_energy": 25.0,
  "mean_age": 0.0,
  "transmissions_this_tick": 0,
  "mutations_this_tick": 0,
  "deaths_this_tick": 0,
  "births_this_tick": 0
}
```

Emitted every `run.metrics_emit_every` ticks (default every tick).

### Event records (between tick records)

Birth:

```json
{ "kind": "birth", "tick": 73, "child": 121, "parent": 17, "inherited": [4, 12] }
```

Death:

```json
{ "kind": "death", "tick": 88, "agent": 17, "cause": "starvation" }
```

Transmission:

```json
{ "kind": "transmission", "tick": 12, "from": 4, "to": 21, "meme": 4 }
```

Mutation:

```json
{ "kind": "mutation", "tick": 12, "parent_meme": 4, "child_meme": 91, "field": "strength" }
```

Recombination:

```json
{ "kind": "recombination", "tick": 200, "parents": [4, 17], "child_meme": 145 }
```

Meme forgotten (inventory overflow):

```json
{ "kind": "meme_forgotten", "tick": 305, "agent": 21, "meme": 4 }
```

Extinction:

```json
{ "kind": "extinction", "tick": 612, "scope": "all_memes" }
```

Valid `scope` values: `"population"`, `"all_memes"`, `{"single_meme": <MemeId>}`.

Cluster snapshot (every `cluster_snapshot_every` ticks):

```json
{
  "kind": "cluster_snapshot",
  "tick": 50,
  "clusters": [
    { "id": 0, "members": [4, 7, 12, 21] },
    { "id": 1, "members": [17, 33] }
  ]
}
```

## Ordering guarantees

Within a single tick boundary the order of records is:

1. All event records produced during that tick (in the order they were appended).
2. The per-tick aggregate record for that tick.

So a reader that wants to attribute events to ticks can either read the `tick` field on
each event record or use the "next `kind: tick`" record as the closing marker.

## `summary.csv`

Header:

```csv
tick,alive,food_count,meme_count,prevalence_cooperative,prevalence_aggressive,prevalence_defensive,prevalence_imitative,prevalence_punitive,prevalence_conformist,prevalence_mutant,diversity_shannon,dominance_top1,mean_energy,mean_age,transmissions,mutations,births,deaths
```

One row per emitted tick. Floats formatted with 4 decimal places for prevalence /
fractions, 3 for energy, integers for counts. Backwards-compatible note: the first
columns (`tick,alive,food_count,...`) include the POC's existing summary fields so
existing tooling keeps working.

## Self-describing requirement (FR-020)

The run directory MUST contain a `config.toml` byte-identical to what the simulator
consumed for that run. Tools that read `events.jsonl` can locate `config.toml` by
walking up from the events file. The `events.jsonl` itself does NOT embed the config —
embedding would make the file large and prone to drift if hand-edited.

## Versioning

The first line of `events.jsonl` MUST be a header record:

```json
{ "kind": "header", "schema_version": 1, "run_id": "20260519-103045-cooperation-vs-selfish-low", "core_version": "0.1.0" }
```

A reader that sees `schema_version > N` (where N is its supported version) MUST refuse
to load the file with a clear error. `schema_version` bumps follow the same MAJOR/MINOR
discipline as the constitution: additive fields → MINOR; removed/renamed/typed-changed
fields → MAJOR.

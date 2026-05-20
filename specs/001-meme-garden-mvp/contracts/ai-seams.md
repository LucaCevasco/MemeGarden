# Contract — AI Seams

AI integrations are **not** in the per-tick decision path (Principle IV). They live
behind traits in `meme-garden-core::ai`, with a deterministic `NoopProvider` as the
only shipped implementation in the MVP. Live LLM-backed providers ship in a future
`meme-garden-ai` crate that depends on `meme-garden-core` (never the reverse).

## Trait signatures

```rust
// crates/meme-garden-core/src/ai.rs

use crate::config::SimConfig;
use crate::meme::Meme;
use crate::metrics::Metrics;
use crate::lineage::LineageGraph;

/// Produces human-readable names for symbolic memes.
/// Called by the CLI when rendering / exporting. Never called from Simulation::step.
pub trait MemeNamer: Send + Sync {
    /// Return a stable short label for `meme`.
    fn name(&self, meme: &Meme) -> String;
}

/// Maps natural-language experiment requests into validated configs.
/// Called by `cli experiment design` only. Not used during a run.
pub trait ExperimentDesigner: Send + Sync {
    /// Translate a free-form natural-language prompt into a complete SimConfig.
    /// Implementations MAY return a richer error type via `Box<dyn Error>` —
    /// the trait signature stays small.
    fn design(&self, prompt: &str) -> Result<SimConfig, AiError>;
}

/// Summarizes a completed run in prose, optionally with the lineage graph as
/// additional context. Called by `cli analyze` post-run only.
pub trait RunAnalyst: Send + Sync {
    fn summarize(&self, history: &[Metrics], lineage: &LineageGraph) -> String;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("ai provider not configured")]
    NotConfigured,
    #[error("ai provider returned invalid config: {0}")]
    InvalidConfig(String),
    #[error("ai provider failed: {0}")]
    Provider(String),
}

/// Deterministic no-op default. Used in tests and as the MVP-shipped impl.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProvider;

impl MemeNamer for NoopProvider {
    fn name(&self, meme: &Meme) -> String {
        format!("meme:{:?}:{:?}/{:?}", meme.kind, meme.trigger, meme.effect)
    }
}

impl ExperimentDesigner for NoopProvider {
    fn design(&self, _prompt: &str) -> Result<SimConfig, AiError> {
        Err(AiError::NotConfigured)
    }
}

impl RunAnalyst for NoopProvider {
    fn summarize(&self, history: &[Metrics], _lineage: &LineageGraph) -> String {
        let Some(last) = history.last() else {
            return "no ticks recorded".to_string();
        };
        format!(
            "ran {} ticks; final alive={}, top1_meme_share={:.2}, diversity={:.2}",
            last.tick + 1,
            last.alive,
            last.dominance_top1_fraction,
            last.diversity_shannon,
        )
    }
}
```

## Determinism rules

- **`MemeNamer`**: MUST be a pure function of the meme argument; calling it twice on
  the same meme MUST return the same string. Real implementations that call an LLM
  MUST cache results per meme identity for the duration of a run.
- **`ExperimentDesigner`**: MAY be nondeterministic between calls — it only produces
  configs, never modifies a running simulation.
- **`RunAnalyst`**: MAY be nondeterministic; runs only after the simulation has
  finished. Output is human-facing text, not consumed by the simulator.

## Where these are called

| Trait | Caller | Hot loop? |
|---|---|---|
| `MemeNamer` | TUI render code (`meme-garden-cli::tui`); CSV/JSONL export for human-readable labels | No |
| `ExperimentDesigner` | CLI subcommand `meme-garden experiment design` | No |
| `RunAnalyst` | CLI subcommand `meme-garden analyze <run-dir>`; optional run-finalize hook | No |

The `Simulation` type does **not** hold a reference to any AI trait. There is no
compile-time path from `Simulation::step` to any of these traits.

## Default wiring

In the MVP, all three trait slots in the CLI are filled with `NoopProvider`. The CLI
exposes future swap points but does not ship live LLM providers in this feature.
